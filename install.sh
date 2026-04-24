#!/bin/bash
# install.sh - Installer for c02dashboard on Raspberry Pi (ARM)

set -e

# Colors
G='\033[0;32m'
NC='\033[0m'

log() {
    echo -e "[${G}+${NC}] $1"
}

INSTALL_DIR="/opt/c02dash"
BINARY_URL="https://github.com/cateperson/c02dashboard/releases/download/release/c02dashboard"
CSS_URL="https://raw.githubusercontent.com/cateperson/c02dashboard/main/static/output.css"
SERVICE_NAME="c02dash"
USER_NAME="c02dash"

if [[ $EUID -ne 0 ]]; then
   echo "This script must be run as root (use sudo)"
   exit 1
fi

log "Creating system user: $USER_NAME"
if ! id "$USER_NAME" &>/dev/null; then
    useradd --system --user-group --shell /usr/sbin/nologin "$USER_NAME"
fi

log "Creating directories in $INSTALL_DIR"
mkdir -p "$INSTALL_DIR/data"
mkdir -p "$INSTALL_DIR/static"

log "Downloading binary from GitHub"
curl -L "$BINARY_URL" -o "$INSTALL_DIR/c02dashboard"
chmod +x "$INSTALL_DIR/c02dashboard"

log "Downloading static CSS"
curl -L "$CSS_URL" -o "$INSTALL_DIR/static/output.css"

log "Applying permissions"
chown -R "$USER_NAME:$USER_NAME" "$INSTALL_DIR"

log "Configuring systemd service"
cat <<EOF > /etc/systemd/system/$SERVICE_NAME.service
[Unit]
Description=CO2 Dashboard Service
After=network.target

[Service]
Type=simple
User=$USER_NAME
Group=$USER_NAME
WorkingDirectory=$INSTALL_DIR
ExecStart=$INSTALL_DIR/c02dashboard
Restart=always
RestartSec=5
StandardOutput=journal
StandardError=journal
ReadWritePaths=$INSTALL_DIR/data

[Install]
WantedBy=multi-user.target
EOF

log "Starting $SERVICE_NAME service"
systemctl daemon-reload
systemctl enable $SERVICE_NAME
systemctl restart $SERVICE_NAME

log "Installation complete! Dashboard is running."
systemctl status $SERVICE_NAME --no-pager
