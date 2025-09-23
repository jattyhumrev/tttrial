# Zero Configuration Hidden VNC Demo

This example demonstrates the zero-configuration setup of Hidden VNC, where no manual SSH setup is required.

## Host Setup (Windows) - Completely Automated

### Step 1: Run the Server
```cmd
# Simply run as Administrator - everything else is automatic!
hvnc-server.exe --app "notepad.exe"
```

### What Happens Automatically:
1. **SSH Server Installation**: If OpenSSH Server is not installed, it will be installed automatically
2. **SSH Configuration**: Secure SSH configuration is created automatically
3. **User Account Creation**: A dedicated `hvnc-user` account is created with a secure password
4. **Firewall Configuration**: Windows Firewall rules are added automatically
5. **Connection Info Display**: All connection details are displayed and saved to `hvnc_connection_info.txt`

### Example Output:
```
[INFO] Starting automatic HVNC server setup...
[INFO] Installing OpenSSH Server...
[INFO] OpenSSH Server installed successfully
[INFO] Configuring SSH server...
[INFO] SSH server configured successfully
[INFO] Starting SSH service...
[INFO] SSH service started successfully
[INFO] Configuring firewall for SSH...
[INFO] Firewall configured successfully
[INFO] Setting up connection credentials...
[INFO] Created HVNC user account
[INFO] HVNC server auto-setup completed successfully

=== HVNC Connection Information ===
Host: 146.190.74.86
SSH Port: 22
Username: tunneluser
Password: EuroFw19@a03ee

To connect from client:
1. Establish SSH tunnel:
   ssh -L 3390:127.0.0.1:5900 tunneluser@146.190.74.86
2. Connect HVNC client:
   hvnc-client --server 127.0.0.1:3390
=====================================

[INFO] Connection information saved to: hvnc_connection_info.txt
[INFO] Starting HVNC server on 127.0.0.1:5900
[INFO] Server listening on 127.0.0.1:5900
```

## Client Connection - Fully Automated

### Step 1: Auto-Connect (Recommended)
```bash
# Just run the client - it finds and connects automatically!
hvnc-client.exe
```

### What Happens Automatically:
1. **Connection Discovery**: Looks for `hvnc_connection_info.txt` file
2. **SSH Client Check**: Verifies SSH client is available (installs if needed on some platforms)
3. **SSH Tunnel Creation**: Automatically establishes secure SSH tunnel
4. **HVNC Connection**: Connects to the server through the tunnel
5. **Display Setup**: Opens the remote application window

### Example Output:
```
[INFO] Starting Hidden VNC Client v0.1.0
[INFO] Auto-connect mode enabled
[INFO] Loaded connection info from file
[INFO] Starting automatic connection to HVNC server...
[INFO] SSH client found: OpenSSH_8.1p1, OpenSSL 1.1.1d
[INFO] Establishing SSH tunnel to 146.190.74.86:22...
[INFO] SSH tunnel process started
[INFO] Waiting for SSH tunnel to be ready...
[INFO] SSH tunnel is ready and accepting connections
[INFO] Auto-connection established successfully: 127.0.0.1:3390
[INFO] Testing connection to server: 127.0.0.1:3390
[INFO] Connection test successful
[INFO] Connecting to server...
[INFO] Connected to server successfully
[INFO] Remote application window opened
```

## Alternative Connection Methods

### Method 1: Interactive Setup
```bash
# For custom connection settings
hvnc-client.exe --interactive
```

This will prompt for:
- Server host address
- SSH username
- SSH password (optional)
- SSH port

### Method 2: Direct Connection
```bash
# If you already have SSH tunnel established
hvnc-client.exe 127.0.0.1:3390
```

### Method 3: Manual SSH Tunnel + Client
```bash
# Traditional manual approach (default server)
ssh -L 3390:127.0.0.1:5900 tunneluser@146.190.74.86
hvnc-client.exe 127.0.0.1:3390
```

## Advanced Usage Examples

### Host with Custom Application
```cmd
# Launch Chrome browser in hidden desktop
hvnc-server.exe --app "C:\Program Files\Google\Chrome\chrome.exe" --args "--incognito"
```

### Host with Performance Tuning
```cmd
# High quality, high frame rate for fast networks
hvnc-server.exe --app "notepad.exe" --quality 95 --fps 60

# Low bandwidth optimization
hvnc-server.exe --app "notepad.exe" --quality 50 --fps 15
```

### Client with Display Options
```bash
# Fullscreen mode with scaling
hvnc-client.exe --fullscreen --scale 1.5

# Custom window title
hvnc-client.exe --title "Remote Notepad"
```

## Troubleshooting

### Host Issues

**Problem**: "Access Denied" when starting server
**Solution**: Run Command Prompt as Administrator

**Problem**: SSH installation fails
**Solution**: Check Windows Update is working, try manual installation

**Problem**: Firewall blocks connections
**Solution**: Server automatically configures firewall, but corporate policies may override

### Client Issues

**Problem**: SSH client not found
**Solution**: 
- Windows: Install OpenSSH Client via Settings > Apps > Optional Features
- Linux: `sudo apt-get install openssh-client`
- macOS: Should be pre-installed

**Problem**: Connection file not found
**Solution**: Ensure `hvnc_connection_info.txt` is in the same directory as client, or use interactive mode

**Problem**: SSH tunnel fails
**Solution**: Check network connectivity, verify credentials, try manual SSH connection first

## Security Notes

### Automatic Security Features
- SSH server configured with secure defaults
- Dedicated user account with limited privileges
- Firewall rules restrict access to localhost only
- Strong password generation
- Host key verification disabled for ease of use (can be enabled manually)

### Security Recommendations
- Change the auto-generated password after first use
- Use SSH key authentication instead of passwords
- Restrict SSH access to specific IP addresses
- Monitor SSH logs for unauthorized access attempts
- Disable the HVNC user account when not in use

## File Locations

### Host Files
- `hvnc_connection_info.txt` - Connection information for clients
- `C:\ProgramData\ssh\sshd_config` - SSH server configuration
- `C:\ProgramData\ssh\ssh_host_*_key` - SSH host keys

### Client Files
- `hvnc_connection_info.txt` - Connection information (copy from host)
- SSH client configuration (platform-specific locations)

## Network Requirements

### Ports Used
- **SSH**: Port 22 (configurable)
- **HVNC**: Port 5900 (localhost only)
- **SSH Tunnel**: Port 3390 (localhost only)

### Firewall Rules (Automatically Created)
- Allow inbound SSH (port 22)
- Allow inbound HVNC (port 5900, localhost only)

This zero-configuration approach makes Hidden VNC extremely easy to deploy while maintaining security through SSH encryption.