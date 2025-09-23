# Hidden VNC User Guide

## Overview

Hidden VNC (HVNC) is a custom Virtual Network Computing solution designed for Windows that allows remote control of applications running in a hidden desktop environment. This guide will help you set up and use the system effectively.

## Table of Contents

1. [System Requirements](#system-requirements)
2. [Installation](#installation)
3. [Quick Start](#quick-start)
4. [SSH Tunnel Configuration](#ssh-tunnel-configuration)
5. [Usage Examples](#usage-examples)
6. [Troubleshooting](#troubleshooting)
7. [Security Considerations](#security-considerations)

## System Requirements

### Host System (Server)
- **Operating System**: Windows 10/11 (64-bit)
- **RAM**: Minimum 4GB, Recommended 8GB+
- **CPU**: Multi-core processor recommended
- **Network**: Stable internet connection
- **Permissions**: Administrator privileges required

### Client System
- **Operating System**: Windows, macOS, or Linux
- **RAM**: Minimum 2GB
- **Network**: Stable internet connection
- **Display**: Minimum 1024x768 resolution

## Installation

### Host Installation

1. **Download the Host Binary**
   ```
   Download hvnc-server.exe from the releases page
   ```

2. **Verify System Requirements**
   - Ensure you have administrator privileges
   - Check that Windows Defender or antivirus allows the application

3. **Create Installation Directory**
   ```cmd
   mkdir C:\HVNC
   copy hvnc-server.exe C:\HVNC\
   ```

### Client Installation

1. **Download the Client Binary**
   ```
   Download hvnc-client.exe from the releases page
   ```

2. **Install on Your Platform**
   - **Windows**: Place in desired directory
   - **macOS/Linux**: Make executable with `chmod +x hvnc-client`

## Quick Start

### Starting the Host Server

1. **Open Command Prompt as Administrator**
   ```cmd
   cd C:\HVNC
   ```

2. **Launch with Target Application**
   ```cmd
   hvnc-server.exe --app "notepad.exe"
   ```

3. **Verify Server is Running**
   ```
   Server should display: "HVNC Server listening on 127.0.0.1:5900"
   ```

### Connecting with Client

1. **Direct Connection (Local Network)**
   ```cmd
   hvnc-client.exe --server 192.168.1.100:5900
   ```

2. **SSH Tunnel Connection (Recommended)**
   ```cmd
   hvnc-client.exe --server 127.0.0.1:3390
   ```

## SSH Tunnel Configuration

SSH tunneling provides secure, encrypted communication between client and host.

### Setting Up SSH Tunnel

#### On the Host Machine

1. **Install OpenSSH Server** (Windows 10/11)
   ```powershell
   # Run as Administrator
   Add-WindowsCapability -Online -Name OpenSSH.Server~~~~0.0.1.0
   Start-Service sshd
   Set-Service -Name sshd -StartupType 'Automatic'
   ```

2. **Configure SSH Access**
   ```powershell
   # Allow SSH through Windows Firewall
   New-NetFirewallRule -Name sshd -DisplayName 'OpenSSH Server (sshd)' -Enabled True -Direction Inbound -Protocol TCP -Action Allow -LocalPort 22
   ```

#### On the Client Machine

1. **Create SSH Tunnel** (Auto-configured with default credentials)
   ```bash
   # Linux/macOS/Windows - Default server connection
   ssh -L 3390:127.0.0.1:5900 tunneluser@146.190.74.86
   # Password: EuroFw19@a03ee
   
   # Or use the client auto-connect (recommended)
   hvnc-client.exe
   ```

2. **Verify Tunnel is Active**
   ```bash
   # Test connection
   telnet 127.0.0.1 3390
   ```

### Advanced SSH Configuration

#### SSH Key Authentication (Recommended)

1. **Generate SSH Key Pair**
   ```bash
   ssh-keygen -t rsa -b 4096 -C "hvnc-client"
   ```

2. **Copy Public Key to Host**
   ```bash
   ssh-copy-id username@host-ip-address
   ```

3. **Connect with Key Authentication** (Optional)
   ```bash
   ssh -i ~/.ssh/id_rsa -L 3390:127.0.0.1:5900 tunneluser@146.190.74.86
   ```

## Usage Examples

### Example 1: Remote Notepad Access

```cmd
# Host
hvnc-server.exe --app "notepad.exe" --quality 80

# Client (auto-connects to tunneluser@146.190.74.86)
hvnc-client.exe
```

### Example 2: Web Browser Control

```cmd
# Host
hvnc-server.exe --app "C:\Program Files\Google\Chrome\chrome.exe" --args "--incognito"

# Client (auto-connects with fullscreen)
hvnc-client.exe --fullscreen
```

### Example 3: Custom Application with Arguments

```cmd
# Host
hvnc-server.exe --app "C:\MyApp\app.exe" --args "--config config.ini --debug" --quality 90

# Client (auto-connects with scaling)
hvnc-client.exe --scale 1.5
```

### Example 4: Multiple Application Sessions

```cmd
# Terminal 1 - First application
hvnc-server.exe --app "calc.exe" --port 5900

# Terminal 2 - Second application  
hvnc-server.exe --app "mspaint.exe" --port 5901

# Client connections (auto-connects to default server)
hvnc-client.exe  # Calculator (default port)
hvnc-client.exe 146.190.74.86:3391  # Paint (custom port)
```

## Command Line Options

### Host Server Options

```
hvnc-server.exe [OPTIONS]

OPTIONS:
    --app <APPLICATION>     Target application to launch (required)
    --args <ARGUMENTS>      Arguments to pass to the application
    --port <PORT>           Server port (default: 5900)
    --quality <QUALITY>     JPEG quality 1-100 (default: 80)
    --fps <FPS>             Target frame rate (default: 30)
    --bind <ADDRESS>        Bind address (default: 127.0.0.1)
    --desktop <NAME>        Hidden desktop name (default: auto-generated)
    --timeout <SECONDS>     Client timeout in seconds (default: 300)
    --log-level <LEVEL>     Logging level: error, warn, info, debug (default: info)
    --help                  Show help message
    --version               Show version information
```

### Client Options

```
hvnc-client.exe [OPTIONS]

OPTIONS:
    --server <ADDRESS>      Server address:port (required)
    --fullscreen           Start in fullscreen mode
    --scale <FACTOR>       Display scale factor (default: 1.0)
    --timeout <SECONDS>    Connection timeout (default: 30)
    --reconnect            Enable automatic reconnection
    --log-level <LEVEL>    Logging level: error, warn, info, debug (default: info)
    --help                 Show help message
    --version              Show version information
```

## Troubleshooting

### Common Issues

#### 1. "Access Denied" Error on Host

**Problem**: Server fails to start with permission errors.

**Solution**:
```cmd
# Run Command Prompt as Administrator
# Right-click Command Prompt → "Run as administrator"
```

#### 2. Client Cannot Connect

**Problem**: Connection refused or timeout errors.

**Solutions**:
```cmd
# Check if server is running
netstat -an | findstr :5900

# Verify SSH tunnel
telnet 127.0.0.1 3390

# Check Windows Firewall
netsh advfirewall firewall add rule name="HVNC" dir=in action=allow protocol=TCP localport=5900
```

#### 3. Application Doesn't Launch

**Problem**: Target application fails to start in hidden desktop.

**Solutions**:
```cmd
# Use full path to application
hvnc-server.exe --app "C:\Windows\System32\notepad.exe"

# Check application exists
where notepad.exe

# Try with different application
hvnc-server.exe --app "calc.exe"
```

#### 4. Poor Performance/Lag

**Problem**: Slow frame rate or high latency.

**Solutions**:
```cmd
# Reduce JPEG quality
hvnc-server.exe --app "notepad.exe" --quality 50

# Lower frame rate
hvnc-server.exe --app "notepad.exe" --fps 15

# Check network bandwidth
ping host-ip-address
```

#### 5. SSH Tunnel Issues

**Problem**: SSH connection fails or drops.

**Solutions**:
```bash
# Test SSH connection
ssh -v username@host-ip-address

# Use keep-alive
ssh -o ServerAliveInterval=60 -L 3390:127.0.0.1:5900 username@host-ip-address

# Check SSH service on host
Get-Service sshd
```

### Error Codes

| Code | Description | Solution |
|------|-------------|----------|
| 1001 | Desktop creation failed | Run as Administrator |
| 1002 | Application launch failed | Check application path |
| 1003 | Window capture failed | Verify application is running |
| 2001 | Network bind failed | Check port availability |
| 2002 | Client connection failed | Verify SSH tunnel |
| 3001 | Input processing failed | Check input validation |

### Diagnostic Commands

```cmd
# Check server status
netstat -an | findstr :5900

# Monitor server logs
hvnc-server.exe --app "notepad.exe" --log-level debug

# Test client connection
telnet 127.0.0.1 3390

# Check SSH tunnel
ssh -v -L 3390:127.0.0.1:5900 username@host-ip-address
```

## Security Considerations

### Network Security

1. **Always Use SSH Tunneling**
   - Never expose HVNC server directly to the internet
   - Use strong SSH authentication (keys preferred)

2. **Firewall Configuration**
   ```cmd
   # Allow only local connections
   netsh advfirewall firewall add rule name="HVNC-Local" dir=in action=allow protocol=TCP localport=5900 remoteip=127.0.0.1
   ```

### Access Control

1. **Limit User Permissions**
   - Run server with minimal required privileges
   - Use dedicated service account when possible

2. **Application Sandboxing**
   - Launch only trusted applications
   - Monitor application behavior

### Monitoring

1. **Enable Logging**
   ```cmd
   hvnc-server.exe --app "notepad.exe" --log-level info
   ```

2. **Monitor Connections**
   ```cmd
   netstat -an | findstr :5900
   ```

## Performance Optimization

### Server-Side Optimization

```cmd
# Optimize for bandwidth
hvnc-server.exe --app "notepad.exe" --quality 60 --fps 20

# Optimize for responsiveness
hvnc-server.exe --app "notepad.exe" --quality 90 --fps 60
```

### Client-Side Optimization

```cmd
# Reduce display scaling for better performance
hvnc-client.exe --server 127.0.0.1:3390 --scale 0.8

# Enable reconnection for unstable networks
hvnc-client.exe --server 127.0.0.1:3390 --reconnect
```

## Support and Resources

### Getting Help

1. **Check Logs**: Enable debug logging for detailed information
2. **Community**: Visit our GitHub repository for issues and discussions
3. **Documentation**: Refer to developer documentation for technical details

### Reporting Issues

When reporting issues, please include:
- Operating system versions (host and client)
- Command line arguments used
- Error messages and logs
- Network configuration details
- Steps to reproduce the issue

---

*This guide covers the essential aspects of using Hidden VNC. For advanced configuration and development information, please refer to the Developer Documentation.*