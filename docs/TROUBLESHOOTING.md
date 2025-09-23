# Hidden VNC Troubleshooting Guide

## Quick Diagnostic Commands

```cmd
# Check HVNC server status
netstat -an | findstr :5900

# Check SSH tunnel
telnet 127.0.0.1 3390

# Test SSH connection (default server)
ssh -v tunneluser@146.190.74.86

# Check Windows services
Get-Service sshd
```

## Common Issues and Solutions

### 1. Server Startup Issues

#### Issue: "Access Denied" when starting server
```
Error: Failed to create hidden desktop (Access Denied)
```

**Cause**: Insufficient privileges

**Solutions**:
```cmd
# Solution 1: Run as Administrator
# Right-click Command Prompt → "Run as administrator"

# Solution 2: Check User Account Control (UAC)
# Temporarily disable UAC or run with elevated privileges

# Solution 3: Verify user permissions
net localgroup administrators
```

#### Issue: "Port already in use"
```
Error: Failed to bind to 127.0.0.1:5900 (Address already in use)
```

**Cause**: Another service is using port 5900

**Solutions**:
```cmd
# Check what's using the port
netstat -ano | findstr :5900

# Kill the process (replace PID with actual process ID)
taskkill /PID 1234 /F

# Use a different port
hvnc-server.exe --app "notepad.exe" --port 5901
```

#### Issue: Application fails to launch
```
Error: Failed to launch application 'notepad.exe'
```

**Cause**: Application not found or insufficient permissions

**Solutions**:
```cmd
# Use full path
hvnc-server.exe --app "C:\Windows\System32\notepad.exe"

# Check if application exists
where notepad.exe

# Try with a different application
hvnc-server.exe --app "calc.exe"

# Check application permissions
icacls "C:\Windows\System32\notepad.exe"
```

### 2. SSH Connection Issues

#### Issue: SSH connection refused
```
ssh: connect to host 146.190.74.86 port 22: Connection refused
```

**Cause**: SSH server not running or firewall blocking

**Solutions**:
```powershell
# Check SSH service status
Get-Service sshd

# Start SSH service
Start-Service sshd

# Check firewall rules
Get-NetFirewallRule -DisplayName "*SSH*"

# Add firewall rule
New-NetFirewallRule -DisplayName "SSH" -Direction Inbound -Protocol TCP -LocalPort 22 -Action Allow
```

#### Issue: SSH authentication failed
```
Permission denied (publickey,password)
```

**Cause**: Authentication credentials incorrect

**Solutions**:
```bash
# Test with verbose output (default server)
ssh -v tunneluser@146.190.74.86

# Test with password authentication
ssh -o PasswordAuthentication=yes tunneluser@146.190.74.86
# Password: EuroFw19@a03ee

# Check SSH configuration (if you have server access)
cat /etc/ssh/sshd_config
```

#### Issue: SSH tunnel not working
```
channel 2: open failed: connect failed: Connection refused
```

**Cause**: HVNC server not running or wrong port

**Solutions**:
```bash
# Verify HVNC server is running
ssh tunneluser@146.190.74.86 "netstat -an | findstr :5900"

# Test tunnel with different port
ssh -L 3391:127.0.0.1:5901 tunneluser@146.190.74.86

# Check tunnel is established
netstat -an | findstr :3390
```

### 3. Client Connection Issues

#### Issue: Client cannot connect to tunnel
```
Error: Connection refused to 127.0.0.1:3390
```

**Cause**: SSH tunnel not established or HVNC server not running

**Solutions**:
```bash
# Verify SSH tunnel is active
ps aux | grep ssh
netstat -an | grep :3390

# Re-establish tunnel
ssh -L 3390:127.0.0.1:5900 username@host-ip-address

# Test tunnel connectivity
telnet 127.0.0.1 3390
```

#### Issue: Client connects but no display
```
Client connected but window shows black screen
```

**Cause**: Application not visible or capture failed

**Solutions**:
```cmd
# Check if application is running
tasklist | findstr notepad

# Restart server with debug logging
hvnc-server.exe --app "notepad.exe" --log-level debug

# Try different application
hvnc-server.exe --app "calc.exe"

# Check desktop creation
hvnc-server.exe --app "notepad.exe" --desktop "test-desktop"
```

### 4. Performance Issues

#### Issue: Slow frame rate or high latency
```
Client display is laggy or updates slowly
```

**Cause**: Network bandwidth, compression settings, or system resources

**Solutions**:
```cmd
# Reduce JPEG quality for better performance
hvnc-server.exe --app "notepad.exe" --quality 50

# Lower frame rate
hvnc-server.exe --app "notepad.exe" --fps 15

# Check network latency
ping host-ip-address

# Monitor system resources
taskmgr
```

#### Issue: High CPU usage
```
HVNC server consuming excessive CPU
```

**Cause**: High frame rate or inefficient capture

**Solutions**:
```cmd
# Reduce frame rate
hvnc-server.exe --app "notepad.exe" --fps 10

# Increase capture interval
hvnc-server.exe --app "notepad.exe" --quality 60

# Check for other processes
tasklist /svc
```

### 5. Input Issues

#### Issue: Mouse clicks not working
```
Mouse clicks in client don't affect remote application
```

**Cause**: Input coordinate translation or application focus issues

**Solutions**:
```cmd
# Restart server with debug logging
hvnc-server.exe --app "notepad.exe" --log-level debug

# Check application window state
# Ensure application window is visible and active

# Try different application
hvnc-server.exe --app "calc.exe"
```

#### Issue: Keyboard input not working
```
Typing in client doesn't appear in remote application
```

**Cause**: Keyboard focus or input handling issues

**Solutions**:
```cmd
# Check application has focus
# Click in the application window first

# Restart with different application
hvnc-server.exe --app "notepad.exe"

# Check for keyboard layout issues
# Ensure both systems use compatible layouts
```

### 6. Network Issues

#### Issue: Connection drops frequently
```
Client disconnects after short periods
```

**Cause**: Network instability or timeout settings

**Solutions**:
```bash
# Use SSH keep-alive (default server)
ssh -o ServerAliveInterval=60 -o ServerAliveCountMax=3 -L 3390:127.0.0.1:5900 tunneluser@146.190.74.86

# Increase client timeout
hvnc-client.exe --server 127.0.0.1:3390 --timeout 60

# Enable client reconnection
hvnc-client.exe --server 127.0.0.1:3390 --reconnect
```

#### Issue: Data corruption or artifacts
```
Display shows corrupted images or artifacts
```

**Cause**: Network packet loss or compression issues

**Solutions**:
```cmd
# Increase JPEG quality
hvnc-server.exe --app "notepad.exe" --quality 90

# Reduce frame rate to allow better compression
hvnc-server.exe --app "notepad.exe" --fps 20

# Check network stability
ping -t host-ip-address
```

## Advanced Troubleshooting

### Debug Logging

#### Enable Detailed Logging
```cmd
# Server debug logging
hvnc-server.exe --app "notepad.exe" --log-level debug > server.log 2>&1

# Client debug logging
hvnc-client.exe --server 127.0.0.1:3390 --log-level debug > client.log 2>&1
```

#### Log Analysis
```cmd
# Search for errors in logs
findstr /i "error" server.log
findstr /i "failed" server.log
findstr /i "timeout" client.log

# Monitor logs in real-time
Get-Content server.log -Wait
```

### Network Diagnostics

#### Test Network Connectivity
```bash
# Basic connectivity
ping host-ip-address

# Trace route
tracert host-ip-address

# Port connectivity
telnet host-ip-address 22
telnet 127.0.0.1 3390

# Bandwidth test
iperf3 -c host-ip-address
```

#### SSH Tunnel Diagnostics
```bash
# Verbose SSH connection (default server)
ssh -vvv -L 3390:127.0.0.1:5900 tunneluser@146.190.74.86

# Monitor SSH tunnel
ssh -o LogLevel=DEBUG3 -L 3390:127.0.0.1:5900 tunneluser@146.190.74.86

# Test tunnel forwarding
nc -zv 127.0.0.1 3390
```

### System Diagnostics

#### Windows System Checks
```powershell
# Check system resources
Get-Counter "\Processor(_Total)\% Processor Time"
Get-Counter "\Memory\Available MBytes"

# Check Windows services
Get-Service | Where-Object {$_.Name -like "*ssh*"}

# Check Windows event logs
Get-EventLog -LogName System -Newest 50 | Where-Object {$_.Source -like "*SSH*"}
```

#### Process Monitoring
```cmd
# Monitor HVNC processes
tasklist | findstr hvnc

# Check process details
wmic process where "name='hvnc-server.exe'" get ProcessId,CommandLine,WorkingSetSize

# Monitor network connections
netstat -ano | findstr hvnc
```

## Error Code Reference

### Server Error Codes

| Code | Description | Common Causes | Solutions |
|------|-------------|---------------|-----------|
| 1001 | Desktop creation failed | Insufficient privileges | Run as Administrator |
| 1002 | Application launch failed | App not found, permissions | Check app path, run as admin |
| 1003 | Window capture failed | App minimized, hidden | Ensure app is visible |
| 1004 | Desktop switch failed | System restrictions | Check desktop permissions |
| 2001 | Network bind failed | Port in use, permissions | Check port availability |
| 2002 | Client accept failed | Network issues | Check firewall, network |
| 2003 | Frame send failed | Connection lost | Check client connection |
| 3001 | Input processing failed | Invalid coordinates | Check input validation |
| 3002 | Input injection failed | System restrictions | Check input permissions |

### Client Error Codes

| Code | Description | Common Causes | Solutions |
|------|-------------|---------------|-----------|
| 4001 | Connection failed | Server not running, network | Check server, SSH tunnel |
| 4002 | Authentication failed | Wrong credentials | Check SSH credentials |
| 4003 | Protocol error | Version mismatch | Update client/server |
| 4004 | Display initialization failed | Graphics issues | Check display drivers |
| 4005 | Input capture failed | Permission issues | Check input permissions |

## Recovery Procedures

### Complete System Reset

#### Host Reset
```cmd
# Stop all HVNC processes
taskkill /f /im hvnc-server.exe

# Restart SSH service
net stop sshd
net start sshd

# Clear temporary files
del /q C:\temp\hvnc-*

# Restart with clean state
hvnc-server.exe --app "notepad.exe" --log-level info
```

#### Client Reset
```bash
# Kill existing SSH tunnels
pkill -f "ssh.*3390"

# Clear SSH connection cache
ssh-keygen -R host-ip-address

# Re-establish tunnel
ssh -L 3390:127.0.0.1:5900 username@host-ip-address

# Restart client
./hvnc-client --server 127.0.0.1:3390
```

### Emergency Procedures

#### If Host Becomes Unresponsive
```cmd
# Remote restart via SSH
ssh username@host-ip-address "shutdown /r /t 0"

# Or use remote desktop if available
mstsc /v:host-ip-address
```

#### If Client Hangs
```bash
# Force kill client
pkill -9 hvnc-client

# Kill SSH tunnel
pkill -f "ssh.*3390"

# Restart connection
ssh -L 3390:127.0.0.1:5900 username@host-ip-address &
./hvnc-client --server 127.0.0.1:3390
```

## Getting Additional Help

### Information to Collect

When seeking help, please provide:

1. **System Information**
   ```cmd
   systeminfo | findstr /B /C:"OS Name" /C:"OS Version"
   ```

2. **Error Messages**
   - Complete error text
   - Error codes if available
   - Log file contents

3. **Configuration Details**
   - Command line arguments used
   - Network configuration
   - SSH tunnel setup

4. **Reproduction Steps**
   - Exact steps to reproduce the issue
   - Expected vs actual behavior
   - Frequency of occurrence

### Support Channels

- **GitHub Issues**: Report bugs and feature requests
- **Documentation**: Check latest documentation updates
- **Community Forums**: Ask questions and share solutions

---

*This troubleshooting guide covers the most common issues. For complex problems, enable debug logging and analyze the output for specific error patterns.*