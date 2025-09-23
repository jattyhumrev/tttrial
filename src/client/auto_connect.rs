//! Automatic SSH tunnel and connection management for HVNC client
//! 
//! This module provides automatic SSH tunnel establishment and connection
//! management, reducing manual configuration for end users.

use crate::error::{HvncError, Result};
use log::{info, warn, error, debug};
use std::process::{Command, Child, Stdio};
use std::time::{Duration, Instant};
use std::thread;
use std::net::TcpStream;
use std::io::{BufRead, BufReader};

/// Automatic connection manager for HVNC client
#[derive(Debug)]
pub struct AutoConnect {
    ssh_tunnel: Option<SshTunnel>,
    connection_info: ConnectionInfo,
}

/// SSH tunnel process manager
#[derive(Debug)]
struct SshTunnel {
    process: Child,
    local_port: u16,
    tunnel_established: bool,
}

/// Connection information for automatic setup
#[derive(Debug, Clone)]
pub struct ConnectionInfo {
    pub host: String,
    pub ssh_port: u16,
    pub username: String,
    pub password: Option<String>,
    pub local_tunnel_port: u16,
    pub remote_hvnc_port: u16,
}

impl Default for ConnectionInfo {
    fn default() -> Self {
        // Use centralized default configuration
        crate::common::get_default_connection_info()
    }
}

impl AutoConnect {
    /// Create a new auto-connect manager
    pub fn new(connection_info: ConnectionInfo) -> Self {
        Self {
            ssh_tunnel: None,
            connection_info,
        }
    }

    /// Automatically establish SSH tunnel and connect to HVNC server
    pub fn auto_connect(&mut self) -> Result<String> {
        info!("Starting automatic connection to HVNC server...");

        // Check if SSH client is available
        self.check_ssh_client()?;

        // Establish SSH tunnel
        self.establish_ssh_tunnel()?;

        // Wait for tunnel to be ready
        self.wait_for_tunnel()?;

        // Return connection endpoint
        let endpoint = format!("127.0.0.1:{}", self.connection_info.local_tunnel_port);
        info!("Auto-connection established successfully: {}", endpoint);
        Ok(endpoint)
    }

    /// Check if SSH client is available on the system
    fn check_ssh_client(&self) -> Result<()> {
        debug!("Checking for SSH client availability...");

        let output = Command::new("ssh")
            .arg("-V")
            .output();

        match output {
            Ok(output) => {
                let version = String::from_utf8_lossy(&output.stderr);
                info!("SSH client found: {}", version.trim());
                Ok(())
            }
            Err(_) => {
                error!("SSH client not found. Please install OpenSSH client.");
                
                // Provide installation instructions based on platform
                #[cfg(target_os = "windows")]
                {
                    println!("\nTo install SSH client on Windows:");
                    println!("1. Open Settings > Apps > Optional Features");
                    println!("2. Click 'Add a feature'");
                    println!("3. Find and install 'OpenSSH Client'");
                    println!("4. Restart your terminal and try again");
                }
                
                #[cfg(target_os = "linux")]
                {
                    println!("\nTo install SSH client on Linux:");
                    println!("Ubuntu/Debian: sudo apt-get install openssh-client");
                    println!("CentOS/RHEL: sudo yum install openssh-clients");
                    println!("Arch: sudo pacman -S openssh");
                }
                
                #[cfg(target_os = "macos")]
                {
                    println!("\nSSH client should be pre-installed on macOS.");
                    println!("If not available, install via Homebrew: brew install openssh");
                }

                Err(HvncError::OperationFailed("SSH client not available".to_string()))
            }
        }
    }

    /// Establish SSH tunnel to the HVNC server
    fn establish_ssh_tunnel(&mut self) -> Result<()> {
        info!("Establishing SSH tunnel to {}:{}...", self.connection_info.host, self.connection_info.ssh_port);

        // Build SSH command
        let mut ssh_cmd = Command::new("ssh");
        ssh_cmd
            .arg("-L")
            .arg(format!("{}:127.0.0.1:{}", 
                self.connection_info.local_tunnel_port, 
                self.connection_info.remote_hvnc_port))
            .arg("-N") // Don't execute remote command
            .arg("-T") // Disable pseudo-terminal allocation
            .arg("-o").arg("StrictHostKeyChecking=no") // Auto-accept host keys
            .arg("-o").arg("UserKnownHostsFile=/dev/null") // Don't save host keys
            .arg("-o").arg("ServerAliveInterval=60") // Keep connection alive
            .arg("-o").arg("ServerAliveCountMax=3") // Retry count
            .arg("-o").arg("ConnectTimeout=10") // Connection timeout
            .arg(format!("{}@{}", self.connection_info.username, self.connection_info.host))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        // Add password authentication if available
        if self.connection_info.password.is_some() {
            ssh_cmd.arg("-o").arg("PasswordAuthentication=yes");
            ssh_cmd.arg("-o").arg("PubkeyAuthentication=no");
        }

        // Start SSH process
        let mut process = ssh_cmd.spawn()
            .map_err(|e| HvncError::OperationFailed(format!("Failed to start SSH tunnel: {}", e)))?;

        // Handle password input if needed
        if let Some(ref password) = self.connection_info.password {
            if let Some(mut stdin) = process.stdin.take() {
                use std::io::Write;
                let _ = writeln!(stdin, "{}", password);
            }
        }

        self.ssh_tunnel = Some(SshTunnel {
            process,
            local_port: self.connection_info.local_tunnel_port,
            tunnel_established: false,
        });

        info!("SSH tunnel process started");
        Ok(())
    }

    /// Wait for SSH tunnel to be established and ready
    fn wait_for_tunnel(&mut self) -> Result<()> {
        info!("Waiting for SSH tunnel to be ready...");

        let start_time = Instant::now();
        let timeout = Duration::from_secs(30);
        let check_interval = Duration::from_millis(500);

        while start_time.elapsed() < timeout {
            // Check if tunnel port is accepting connections
            if self.test_tunnel_connection() {
                if let Some(ref mut tunnel) = self.ssh_tunnel {
                    tunnel.tunnel_established = true;
                }
                info!("SSH tunnel is ready and accepting connections");
                return Ok(());
            }

            // Check if SSH process is still running
            if let Some(ref mut tunnel) = self.ssh_tunnel {
                match tunnel.process.try_wait() {
                    Ok(Some(status)) => {
                        let error_msg = if let Some(stderr) = tunnel.process.stderr.take() {
                            let mut reader = BufReader::new(stderr);
                            let mut error_output = String::new();
                            let _ = reader.read_line(&mut error_output);
                            error_output
                        } else {
                            "Unknown SSH error".to_string()
                        };

                        return Err(HvncError::OperationFailed(
                            format!("SSH tunnel process exited with status {}: {}", status, error_msg)
                        ));
                    }
                    Ok(None) => {
                        // Process is still running, continue waiting
                    }
                    Err(e) => {
                        return Err(HvncError::OperationFailed(
                            format!("Failed to check SSH process status: {}", e)
                        ));
                    }
                }
            }

            thread::sleep(check_interval);
        }

        Err(HvncError::ConnectionTimeout("SSH tunnel establishment timed out".to_string()))
    }

    /// Test if the SSH tunnel is accepting connections
    fn test_tunnel_connection(&self) -> bool {
        let address = format!("127.0.0.1:{}", self.connection_info.local_tunnel_port);
        
        match TcpStream::connect_timeout(
            &address.parse().unwrap(), 
            Duration::from_millis(1000)
        ) {
            Ok(_) => {
                debug!("Tunnel connection test successful");
                true
            }
            Err(_) => {
                debug!("Tunnel connection test failed, retrying...");
                false
            }
        }
    }

    /// Get the local tunnel endpoint for HVNC client connection
    pub fn get_tunnel_endpoint(&self) -> Option<String> {
        if let Some(ref tunnel) = self.ssh_tunnel {
            if tunnel.tunnel_established {
                return Some(format!("127.0.0.1:{}", tunnel.local_port));
            }
        }
        None
    }

    /// Check if the SSH tunnel is still active
    pub fn is_tunnel_active(&mut self) -> bool {
        if let Some(ref mut tunnel) = self.ssh_tunnel {
            match tunnel.process.try_wait() {
                Ok(Some(_)) => {
                    warn!("SSH tunnel process has exited");
                    false
                }
                Ok(None) => {
                    // Process is still running, test connection
                    self.test_tunnel_connection()
                }
                Err(_) => false,
            }
        } else {
            false
        }
    }

    /// Reconnect the SSH tunnel if it has failed
    pub fn reconnect(&mut self) -> Result<()> {
        info!("Attempting to reconnect SSH tunnel...");

        // Clean up existing tunnel
        self.cleanup();

        // Re-establish tunnel
        self.establish_ssh_tunnel()?;
        self.wait_for_tunnel()?;

        info!("SSH tunnel reconnected successfully");
        Ok(())
    }

    /// Clean up SSH tunnel resources
    pub fn cleanup(&mut self) {
        if let Some(mut tunnel) = self.ssh_tunnel.take() {
            info!("Cleaning up SSH tunnel...");
            
            // Terminate SSH process
            let _ = tunnel.process.kill();
            let _ = tunnel.process.wait();
            
            debug!("SSH tunnel cleanup completed");
        }
    }
}

impl Drop for AutoConnect {
    fn drop(&mut self) {
        self.cleanup();
    }
}

/// Helper function to parse connection info from file
pub fn load_connection_info_from_file(file_path: &str) -> Result<ConnectionInfo> {
    use std::fs;
    use std::collections::HashMap;

    let content = fs::read_to_string(file_path)
        .map_err(|e| HvncError::OperationFailed(format!("Failed to read connection file: {}", e)))?;

    let mut info = ConnectionInfo::default();
    let mut values: HashMap<String, String> = HashMap::new();

    // Parse key-value pairs from the file
    for line in content.lines() {
        if let Some(colon_pos) = line.find(':') {
            let key = line[..colon_pos].trim().to_lowercase();
            let value = line[colon_pos + 1..].trim().to_string();
            values.insert(key, value);
        }
    }

    // Extract connection information
    if let Some(host) = values.get("host") {
        info.host = host.clone();
    }

    if let Some(username) = values.get("username") {
        info.username = username.clone();
    }

    if let Some(password) = values.get("password") {
        info.password = Some(password.clone());
    }

    if let Some(port_str) = values.get("ssh port") {
        if let Ok(port) = port_str.parse::<u16>() {
            info.ssh_port = port;
        }
    }

    Ok(info)
}

/// Interactive connection setup for manual configuration
pub fn interactive_connection_setup() -> Result<ConnectionInfo> {
    use std::io::{self, Write};

    println!("\n=== HVNC Client Connection Setup ===");
    
    let mut info = ConnectionInfo::default();

    // Get host address
    print!("Enter HVNC server host address [{}]: ", info.host);
    io::stdout().flush().unwrap();
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    let host_input = input.trim();
    if !host_input.is_empty() {
        info.host = host_input.to_string();
    }

    // Get username
    print!("Enter SSH username [{}]: ", info.username);
    io::stdout().flush().unwrap();
    input.clear();
    io::stdin().read_line(&mut input).unwrap();
    let username_input = input.trim();
    if !username_input.is_empty() {
        info.username = username_input.to_string();
    }

    // Get password (optional)
    print!("Enter SSH password (leave empty for key-based auth): ");
    io::stdout().flush().unwrap();
    input.clear();
    io::stdin().read_line(&mut input).unwrap();
    let password_input = input.trim();
    if !password_input.is_empty() {
        info.password = Some(password_input.to_string());
    }

    // Get SSH port
    print!("Enter SSH port [{}]: ", info.ssh_port);
    io::stdout().flush().unwrap();
    input.clear();
    io::stdin().read_line(&mut input).unwrap();
    let port_input = input.trim();
    if !port_input.is_empty() {
        if let Ok(port) = port_input.parse::<u16>() {
            info.ssh_port = port;
        }
    }

    println!("=====================================\n");

    Ok(info)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connection_info_default() {
        let info = ConnectionInfo::default();
        assert_eq!(info.host, "127.0.0.1");
        assert_eq!(info.ssh_port, 22);
        assert_eq!(info.username, "hvnc-user");
        assert_eq!(info.local_tunnel_port, 3390);
        assert_eq!(info.remote_hvnc_port, 5900);
    }

    #[test]
    fn test_auto_connect_creation() {
        let info = ConnectionInfo::default();
        let auto_connect = AutoConnect::new(info);
        assert!(auto_connect.ssh_tunnel.is_none());
    }

    #[test]
    fn test_tunnel_endpoint_none_when_not_established() {
        let info = ConnectionInfo::default();
        let auto_connect = AutoConnect::new(info);
        assert!(auto_connect.get_tunnel_endpoint().is_none());
    }
}