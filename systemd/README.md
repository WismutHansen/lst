# Systemd Service Files

This directory contains systemd service files for running lst-server and lst-syncd as system services.

## Installation

1. **Create system user and directories:**
   ```bash
   sudo useradd --system --home /var/lib/lst --create-home lst
   sudo mkdir -p /etc/lst
   sudo mkdir -p /var/lib/lst
   sudo chown lst:lst /var/lib/lst
   ```

2. **Install binaries:**
   ```bash
   # Build and install binaries
   cargo build --release
   sudo cp target/release/lst-server /usr/local/bin/
   sudo cp target/release/lst-syncd /usr/local/bin/
   sudo chmod +x /usr/local/bin/lst-server /usr/local/bin/lst-syncd
   ```

3. **Install service files:**
   ```bash
   sudo cp systemd/lst-server.service /etc/systemd/system/
   sudo cp systemd/lst-syncd.service /etc/systemd/system/
   sudo systemctl daemon-reload
   ```

4. **Create configuration:**
   ```bash
   sudo cp examples/config.toml /etc/lst/config.toml
   sudo chown root:lst /etc/lst/config.toml
   sudo chmod 640 /etc/lst/config.toml
   ```

5. **Enable and start services:**
   ```bash
   sudo systemctl enable lst-server
   sudo systemctl enable lst-syncd
   sudo systemctl start lst-server
   sudo systemctl start lst-syncd
   ```

## Service Management

- **Check status:** `sudo systemctl status lst-server lst-syncd`
- **View logs:** `sudo journalctl -u lst-server -f`
- **Restart:** `sudo systemctl restart lst-server`
- **Stop:** `sudo systemctl stop lst-syncd`

## Configuration

Edit `/etc/lst/config.toml` and restart services to apply changes.

The services run as the `lst` user with restricted permissions for security.