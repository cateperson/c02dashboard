# c02dashboard

A CO2 monitoring dashboard for Raspberry Pi.

## Installation (Raspberry Pi / ARM)

Install script

```bash
curl -sSL https://raw.githubusercontent.com/cateperson/c02dashboard/main/install.sh | sudo bash
```

This script will:
1. Create a `c02dash` system user.
2. Install the binary to `/opt/c02dash/`.
3. Download necessary static assets.
4. Setup and start a `c02dash` systemd service that persists across reboots.

The service runs on port `3000` by default. Data is stored in `/opt/c02dash/data/co2.db`.

To check the service status:
```bash
sudo systemctl status c02dash
```

To view logs:
```bash
sudo journalctl -u c02dash -f
```
