# Pingora IP Rotator

Forward proxy with automatic IP rotation built on Cloudflare's Pingora framework.

## What It Does

Routes your HTTP/HTTPS traffic through multiple IP addresses using round-robin rotation. Each request automatically uses a different source IP.

**This is a forward proxy** - clients explicitly configure it to route outbound traffic.

```
Your App → [Proxy: IP Rotator] → Target Website
              ↓ Rotating IPs
        [IP1, IP2, IP3, IP4...]
```

**Use Cases:**
- Web scraping without IP bans
- API testing from multiple IPs
- Bypass rate limiting
- Geo-distribution testing

## Features

- Round-robin IP rotation (atomic, thread-safe)
- HTTP Basic Auth
- HTTP & HTTPS support
- Environment-based configuration
- Single file implementation (269 lines)


## Quick Start

### Installation

```bash
# Clone and deploy
git clone <repo> /opt/pingora-proxy
cd /opt/pingora-proxy
sudo bash deploy.sh
```

### Configuration

Edit `/opt/pingora-proxy/.env`:

```bash
IP_POOL=172.105.123.45,172.105.123.46,172.105.123.47
PROXY_USER=your_username
PROXY_PASS=your_password
LISTEN_ADDR=0.0.0.0:7777
```

Add IPs to `/etc/netplan/01-netcfg.yaml`:

```yaml
network:
  version: 2
  ethernets:
    eth0:
      dhcp4: true
      addresses:
        - 172.105.123.46/24
        - 172.105.123.47/24
```

```bash
sudo netplan apply
sudo systemctl start pingora-proxy
```

## How It Works

1. Client sends request with Basic Auth
2. Proxy validates credentials
3. Selects next IP using atomic counter (round-robin)
4. Forwards request through selected IP
5. Returns response to client

## Usage

```bash
# Single request
curl -x http://user:pass@proxy-ip:7777 https://httpbin.org/ip

# Test rotation (each request uses different IP)
for i in {1..10}; do
  curl -x http://user:pass@proxy-ip:7777 https://httpbin.org/ip
done
```

**Python:**
```python
import requests

proxies = {
    'http': 'http://user:pass@proxy-ip:7777',
    'https': 'http://user:pass@proxy-ip:7777'
}

response = requests.get('https://httpbin.org/ip', proxies=proxies)
print(response.json())
```

## Management

```bash
sudo systemctl start|stop|restart pingora-proxy
sudo systemctl status pingora-proxy
sudo journalctl -u pingora-proxy -f
```

## Configuration

| Variable | Default | Description |
|----------|---------|-------------|
| `IP_POOL` | `127.0.0.1` | Comma-separated IPs |
| `PROXY_USER` | `proxy_user` | Username |
| `PROXY_PASS` | `proxy_pass` | Password |
| `LISTEN_ADDR` | `0.0.0.0:7777` | Listen address |
| `RUST_LOG` | `info` | Log level (error/warn/info/debug/trace) |

## Project Structure

```
src/main.rs              # Single file (269 lines)
deploy.sh                # Auto deployment
test-proxy.py            # Test IP rotation
.env.example             # Config template
```


## Troubleshooting

**Check logs:**
```bash
sudo journalctl -u pingora-proxy -n 50
```

**Common issues:**
```bash
# Missing .env file
sudo cp .env.example .env

# Verify IP pool
sudo grep IP_POOL .env

# Check IPs are assigned
ip addr show eth0

# Test IP rotation
for i in {1..5}; do curl -s -x http://user:pass@proxy-ip:7777 https://httpbin.org/ip; done
```

## Development

```bash
# Build
cargo build --release

# Run locally
./run-local.sh

# Test
python3 test-proxy.py localhost 7777 test test123 20
```

## Technical Details

- Built on Pingora (Cloudflare's proxy framework)
- Lock-free IP rotation using atomic counter
- Single-file implementation (269 lines)
- Memory: ~10MB base
- Async I/O with Tokio

## License

MIT
