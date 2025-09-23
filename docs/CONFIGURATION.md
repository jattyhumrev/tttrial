# Hidden VNC Configuration Guide

## Centralized Configuration System

Hidden VNC uses a centralized configuration system that allows you to update server credentials and connection details in one place. This eliminates the need to update multiple files when server details change.

## Configuration Hierarchy

The system loads configuration in the following order (first found wins):

1. **Command line arguments** (highest priority)
2. **Environment variables**
3. **Configuration files** (`config/server_config.toml`)
4. **Default hardcoded values** (lowest priority)

## Default Server Configuration

### Current Default Server
- **Host**: `146.190.74.86`
- **Username**: `tunneluser`
- **Password**: `EuroFw19@a03ee`
- **SSH Port**: `22`
- **HVNC Port**: `5900`
- **Tunnel Port**: `3390`

## Configuration Files

### Main Configuration File: `config/server_config.toml`

```toml
# Hidden VNC Server Configuration
# 
# This file contains the default server configuration.
# Modify these values to change the default connection settings.

[server]
# Default server host address
host = "146.190.74.86"

# SSH port (usually 22)
ssh_port = 22

# SSH username for tunnel connection
username = "tunneluser"

# SSH password (use with caution - consider SSH keys for production)
password = "EuroFw19@a03ee"

# HVNC server port (on remote host)
hvnc_port = 5900

# Local tunnel port (on client machine)
tunnel_port = 3390

[client]
# Default client settings
auto_connect = true
fullscreen = false
scale_factor = 1.0
reconnect_attempts = 3
connection_timeout = 30

[security]
# Security settings
strict_host_key_checking = false
server_alive_interval = 60
server_alive_count_max = 3

[advanced]
# Advanced settings
log_level = "info"
frame_rate = 30
jpeg_quality = 80
```

### Code-Level Configuration: `src/common/default_config.rs`

For developers who want to change the hardcoded defaults:

```rust
/// Centralized server credentials - SINGLE POINT OF CONFIGURATION
pub const DEFAULT_SERVER: DefaultServerConfig = DefaultServerConfig {
    host: "146.190.74.86",
    ssh_port: 22,
    username: "tunneluser",
    password: "EuroFw19@a03ee",
    hvnc_port: 5900,
    tunnel_port: 3390,
};
```

## Updating Server Configuration

### Method 1: Edit Configuration File (Recommended)

1. **Edit** `config/server_config.toml`:
   ```toml
   [server]
   host = "your-new-server.com"
   username = "your-username"
   password = "your-password"
   ```

2. **Restart** the client - it will automatically use the new settings

### Method 2: Edit Source Code

1. **Edit** `src/common/default_config.rs`:
   ```rust
   pub const DEFAULT_SERVER: DefaultServerConfig = DefaultServerConfig {
       host: "your-new-server.com",
       username: "your-username",
       password: "your-password",
       // ... other settings
   };
   ```

2. **Rebuild** the application:
   ```bash
   cargo build --release
   ```

### Method 3: Environment Variables

Set environment variables to override defaults:

```bash
export HVNC_HOST="your-server.com"
export HVNC_USERNAME="your-username"
export HVNC_PASSWORD="your-password"
export HVNC_SSH_PORT="22"
```

### Method 4: Command Line Arguments

Override settings via command line:

```bash
# Client with custom server
hvnc-client.exe your-server.com:3390

# Client with interactive setup
hvnc-client.exe --interactive
```

## Configuration Examples

### Example 1: Different Server
```toml
[server]
host = "192.168.1.100"
username = "admin"
password = "secure123"
ssh_port = 2222
```

### Example 2: High Performance Setup
```toml
[server]
host = "146.190.74.86"
username = "tunneluser"
password = "EuroFw19@a03ee"

[advanced]
frame_rate = 60
jpeg_quality = 95

[client]
scale_factor = 1.0
reconnect_attempts = 5
```

### Example 3: Low Bandwidth Setup
```toml
[server]
host = "146.190.74.86"
username = "tunneluser"
password = "EuroFw19@a03ee"

[advanced]
frame_rate = 15
jpeg_quality = 50

[client]
scale_factor = 0.8
```

## Security Considerations

### Password Storage
- Configuration files contain passwords in plain text
- Consider using SSH key authentication instead
- Restrict file permissions: `chmod 600 config/server_config.toml`

### SSH Key Authentication
To use SSH keys instead of passwords:

1. **Generate SSH key pair**:
   ```bash
   ssh-keygen -t rsa -b 4096 -f ~/.ssh/hvnc_key
   ```

2. **Copy public key to server**:
   ```bash
   ssh-copy-id -i ~/.ssh/hvnc_key.pub tunneluser@146.190.74.86
   ```

3. **Update configuration**:
   ```toml
   [server]
   host = "146.190.74.86"
   username = "tunneluser"
   # Remove password line
   ssh_key_path = "~/.ssh/hvnc_key"
   
   [security]
   strict_host_key_checking = true
   ```

## Configuration Validation

The system validates configuration on startup:

- **Host reachability**: Checks if the server is accessible
- **SSH connectivity**: Verifies SSH connection works
- **Port availability**: Ensures tunnel ports are free
- **Credential validation**: Tests SSH authentication

## Troubleshooting Configuration

### Common Issues

1. **Configuration file not found**
   ```
   Error: Configuration file not found: config/server_config.toml
   ```
   **Solution**: Create the config directory and file, or use defaults

2. **Invalid TOML syntax**
   ```
   Error: Failed to parse config file: expected an equals
   ```
   **Solution**: Check TOML syntax, ensure proper key = value format

3. **Connection refused**
   ```
   Error: SSH connection refused
   ```
   **Solution**: Verify host, port, and credentials in configuration

### Debug Configuration Loading

Enable debug logging to see configuration loading:

```bash
RUST_LOG=debug hvnc-client.exe
```

Output will show:
```
[DEBUG] Loading configuration from: config/server_config.toml
[DEBUG] Using server: 146.190.74.86:22
[DEBUG] SSH username: tunneluser
```

## Configuration Management Scripts

### Generate Default Configuration
```bash
# Create default config file
hvnc-client.exe --create-config config/server_config.toml
```

### Validate Configuration
```bash
# Test configuration without connecting
hvnc-client.exe --validate-config config/server_config.toml
```

### Export Current Configuration
```bash
# Export current settings to file
hvnc-client.exe --export-config current_config.toml
```

## Integration with CI/CD

For automated deployments, use environment variables:

```yaml
# GitHub Actions example
env:
  HVNC_HOST: ${{ secrets.HVNC_HOST }}
  HVNC_USERNAME: ${{ secrets.HVNC_USERNAME }}
  HVNC_PASSWORD: ${{ secrets.HVNC_PASSWORD }}
```

```dockerfile
# Docker example
ENV HVNC_HOST=146.190.74.86
ENV HVNC_USERNAME=tunneluser
ENV HVNC_PASSWORD=EuroFw19@a03ee
```

This centralized configuration system ensures that server credentials and connection details can be updated in one place, making maintenance and deployment much easier.