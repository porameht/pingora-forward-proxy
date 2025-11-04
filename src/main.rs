use async_trait::async_trait;
use base64::Engine;
use log::{debug, info, warn};
use pingora_core::prelude::*;
use pingora_core::server::configuration::Opt;
use pingora_core::server::Server;
use pingora_core::upstreams::peer::HttpPeer;
use pingora_http::ResponseHeader;
use pingora_proxy::{http_proxy_service, ProxyHttp, Session};
use std::env;
use std::sync::atomic::{AtomicUsize, Ordering};

// ============================================================================
// Configuration
// ============================================================================

struct ProxyConfig {
    ip_addresses: Vec<String>,
    username: String,
    password: String,
    listen_address: String,
}

impl ProxyConfig {
    fn load_from_environment() -> Self {
        let ip_addresses = Self::parse_ip_pool();
        let username = env::var("PROXY_USER").unwrap_or_else(|_| "proxy_user".into());
        let password = env::var("PROXY_PASS").unwrap_or_else(|_| "proxy_pass".into());
        let listen_address = env::var("LISTEN_ADDR").unwrap_or_else(|_| "0.0.0.0:7777".into());

        Self {
            ip_addresses,
            username,
            password,
            listen_address,
        }
    }

    fn parse_ip_pool() -> Vec<String> {
        env::var("IP_POOL")
            .unwrap_or_else(|_| "127.0.0.1".into())
            .split(',')
            .map(|ip| ip.trim().to_string())
            .filter(|ip| !ip.is_empty())
            .collect()
    }

    fn validate(&self) -> Result<(), String> {
        if self.ip_addresses.is_empty() {
            return Err("IP_POOL is empty. Please set IP_POOL environment variable.".into());
        }

        if self.username.is_empty() {
            return Err("PROXY_USER cannot be empty.".into());
        }

        if self.password.is_empty() {
            return Err("PROXY_PASS cannot be empty.".into());
        }

        Ok(())
    }
}

// ============================================================================
// Proxy Implementation
// ============================================================================

pub struct MultiIPProxy {
    ip_addresses: Vec<String>,
    request_counter: AtomicUsize,
    expected_auth_header: String,
}

impl MultiIPProxy {
    fn new(ip_addresses: Vec<String>, username: &str, password: &str) -> Self {
        let expected_auth_header = Self::create_basic_auth_header(username, password);

        info!("Proxy initialized with {} IP addresses", ip_addresses.len());
        debug!("Available IPs: {:?}", ip_addresses);

        Self {
            ip_addresses,
            request_counter: AtomicUsize::new(0),
            expected_auth_header,
        }
    }

    fn create_basic_auth_header(username: &str, password: &str) -> String {
        let credentials = format!("{}:{}", username, password);
        let encoded = base64::engine::general_purpose::STANDARD.encode(credentials);
        format!("Basic {}", encoded)
    }

    fn select_next_ip(&self) -> &str {
        let request_number = self.request_counter.fetch_add(1, Ordering::Relaxed);
        let ip_index = request_number % self.ip_addresses.len();
        &self.ip_addresses[ip_index]
    }

    fn verify_authentication(&self, auth_header: Option<&str>) -> bool {
        match auth_header {
            Some(header) => header == self.expected_auth_header,
            None => false,
        }
    }
}

// ============================================================================
// HTTP Proxy Implementation
// ============================================================================

#[async_trait]
impl ProxyHttp for MultiIPProxy {
    type CTX = ();

    fn new_ctx(&self) -> Self::CTX {}

    async fn upstream_peer(
        &self,
        session: &mut Session,
        _ctx: &mut Self::CTX,
    ) -> Result<Box<HttpPeer>> {
        let source_ip = self.select_next_ip();
        let target_info = extract_target_info(session);

        debug!(
            "Routing request to {}:{} via IP {}",
            target_info.host, target_info.port, source_ip
        );

        let peer = create_http_peer(&target_info);
        Ok(Box::new(peer))
    }

    async fn request_filter(&self, session: &mut Session, _ctx: &mut Self::CTX) -> Result<bool> {
        let auth_header = extract_auth_header(session);

        if self.verify_authentication(auth_header) {
            return Ok(false); // Allow request to proceed
        }

        warn!("Unauthorized access attempt");
        send_auth_required_response(session).await?;

        Ok(true) // Stop request processing
    }

    async fn logging(&self, session: &mut Session, _error: Option<&Error>, _ctx: &mut Self::CTX) {
        let status_code = get_response_status(session);
        let method = &session.req_header().method;
        let uri = &session.req_header().uri;

        info!("{} {} -> {}", method, uri, status_code);
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

struct TargetInfo {
    host: String,
    port: u16,
    use_tls: bool,
}

fn extract_target_info(session: &Session) -> TargetInfo {
    let uri = &session.req_header().uri;
    let host = uri
        .authority()
        .map(|a| a.as_str())
        .unwrap_or("localhost")
        .to_string();

    let use_tls = uri.scheme_str() == Some("https");
    let default_port = if use_tls { 443 } else { 80 };
    let port = uri.port_u16().unwrap_or(default_port);

    TargetInfo {
        host,
        port,
        use_tls,
    }
}

fn create_http_peer(target: &TargetInfo) -> HttpPeer {
    let address = format!("{}:{}", target.host, target.port);
    HttpPeer::new(&address, target.use_tls, target.host.clone())
}

fn extract_auth_header(session: &Session) -> Option<&str> {
    session
        .req_header()
        .headers
        .get("Proxy-Authorization")
        .and_then(|value| value.to_str().ok())
}

async fn send_auth_required_response(session: &mut Session) -> Result<()> {
    let mut response = ResponseHeader::build(407, None)?;
    response.insert_header("Proxy-Authenticate", "Basic realm=\"Proxy\"")?;
    response.insert_header("Content-Type", "text/plain")?;

    session
        .write_response_header(Box::new(response), false)
        .await?;
    session
        .write_response_body(Some(b"Proxy Authentication Required".as_ref().into()), true)
        .await?;

    Ok(())
}

fn get_response_status(session: &Session) -> u16 {
    session
        .response_written()
        .map(|response| response.status.as_u16())
        .unwrap_or(0)
}

// ============================================================================
// Application Entry Point
// ============================================================================

fn main() {
    initialize_logger();

    let config = ProxyConfig::load_from_environment();

    if let Err(error_message) = config.validate() {
        eprintln!("Configuration Error: {}", error_message);
        std::process::exit(1);
    }

    log_startup_info(&config);

    let server = start_proxy_server(config);
    server.run_forever();
}

fn initialize_logger() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
}

fn log_startup_info(config: &ProxyConfig) {
    info!("Starting Pingora Multi-IP Proxy");
    info!("Listen address: {}", config.listen_address);
    info!("IP pool size: {}", config.ip_addresses.len());
    info!("Authentication: enabled");
}

fn start_proxy_server(config: ProxyConfig) -> Server {
    let mut server = Server::new(Some(Opt::default())).expect("Failed to create server");

    server.bootstrap();

    let proxy = MultiIPProxy::new(config.ip_addresses, &config.username, &config.password);

    let mut proxy_service = http_proxy_service(&server.configuration, proxy);
    proxy_service.add_tcp(&config.listen_address);

    server.add_service(proxy_service);

    info!("Server ready to accept connections");

    server
}
