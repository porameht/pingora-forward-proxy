#!/bin/bash

# Pingora Multi-IP Proxy Deployment Script
# This script automates the deployment on a Linode VPS

set -e

echo "🚀 Pingora Multi-IP Proxy Deployment Script"
echo "============================================"
echo ""

# Check if running as root
if [ "$EUID" -ne 0 ]; then
    echo "❌ Please run as root (use: sudo bash deploy.sh)"
    exit 1
fi

# Update system
echo "📦 Step 1: Updating system packages..."
apt update && apt upgrade -y

# Install dependencies
echo "🔧 Step 2: Installing build dependencies..."
apt install -y build-essential pkg-config libssl-dev git curl ufw

# Install Rust
if ! command -v cargo &> /dev/null; then
    echo "🦀 Step 3: Installing Rust..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
else
    echo "✅ Rust already installed"
fi

# Verify Rust installation
rustc --version
cargo --version

# Create project directory
echo "📁 Step 4: Setting up project directory..."
mkdir -p /opt/pingora-proxy
cd /opt/pingora-proxy

# Copy files (assuming they're in current directory)
if [ -f "Cargo.toml" ]; then
    echo "✅ Cargo.toml found"
else
    echo "❌ Cargo.toml not found. Please ensure project files are in the current directory."
    exit 1
fi

# Build the project
echo "🔨 Step 5: Building project (this may take a while)..."
cargo build --release

# Verify binary
if [ -f "target/release/pingora-proxy" ]; then
    echo "✅ Binary built successfully"
else
    echo "❌ Build failed"
    exit 1
fi

# Install systemd service
echo "⚙️  Step 6: Installing systemd service..."
if [ -f "pingora-proxy.service" ]; then
    cp pingora-proxy.service /etc/systemd/system/
    systemctl daemon-reload
    systemctl enable pingora-proxy
    echo "✅ Systemd service installed"
else
    echo "⚠️  pingora-proxy.service not found, skipping..."
fi

# Setup firewall
echo "🔥 Step 7: Configuring firewall..."
ufw --force enable
ufw allow 22/tcp      # SSH
ufw allow 7777/tcp    # Proxy
echo "✅ Firewall configured"

# Display network configuration reminder
# Create environment file from example
if [ ! -f ".env" ]; then
    echo "📝 Step 8: Creating .env configuration file..."
    cp .env.example .env
    echo "⚠️  IMPORTANT: Edit .env with your configuration:"
    echo "   nano /opt/pingora-proxy/.env"
else
    echo "✅ .env file already exists"
fi

echo ""
echo "⚠️  NEXT STEPS"
echo "=============================================="
echo "1. Configure network (add additional IPs):"
echo "   sudo nano /etc/netplan/01-netcfg.yaml"
echo "   sudo netplan apply"
echo ""
echo "2. Edit configuration:"
echo "   sudo nano /opt/pingora-proxy/.env"
echo "   (Set IP_POOL, PROXY_USER, PROXY_PASS)"
echo ""
echo "3. Start service:"
echo "   sudo systemctl start pingora-proxy"
echo ""
echo "4. Check status:"
echo "   sudo systemctl status pingora-proxy"
echo ""
echo "5. View logs:"
echo "   sudo journalctl -u pingora-proxy -f"
echo ""
echo "✅ Deployment completed!"
