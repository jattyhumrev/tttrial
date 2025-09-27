//! Configuration file loader for Hidden VNC
//! 
//! This module provides functionality to load configuration from TOML files
//! and override default settings.

use crate::error::{HvncError, Result};
use crate::client::ConnectionInfo;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// Server configuration from TOML file
#[derive(Debug, Deserialize, Serialize)]
pub struct ServerConfigToml {
    pub server: ServerSection,
    pub client: Option<ClientSection>,
    pub security: Option<SecuritySection>,
    pub advanced: Option<AdvancedSection>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ServerSection {
    pub host: String,
    pub ssh_port: u16,
    pub username: String,
    pub password: String,
    pub hvnc_port: u16,
    pub tunnel_port: u16,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ClientSection {
    pub auto_connect: Option<bool>,
    pub fullscreen: Option<bool>,
    pub scale_factor: Option<f32>,
    pub reconnect_attempts: Option<u32>,
    pub connection_timeout: Option<u64>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SecuritySection {
    pub strict_host_key_checking: Option<bool>,
    pub server_alive_interval: Option<u64>,
    pub server_alive_count_max: Option<u32>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AdvancedSection {
    pub log_level: Option<String>,
    pub frame_rate: Option<u32>,
    pub jpeg_quality: Option<u8>,
}

/// Load configuration from TOML file
pub fn load_config_from_file(file_path: &str) -> Result<ServerConfigToml> {
    if !Path::new(file_path).exists() {
        log::error!("Configuration file not found: {}", file_path);
        return Err(HvncError::OperationFailed(format!("Configuration file not found: {}", file_path)));
    }

    let content = fs::read_to_string(file_path)
        .map_err(|e| {
            log::error!("Failed to read config file {}: {}", file_path, e);
            HvncError::OperationFailed(format!("Failed to read config file: {}", e))
        })?;

    let config: ServerConfigToml = toml::from_str(&content)
        .map_err(|e| {
            log::error!("Failed to parse config file {}: {}", file_path, e);
            HvncError::OperationFailed(format!("Failed to parse config file: {}", e))
        })?;

    Ok(config)
}

/// Convert TOML config to ConnectionInfo
pub fn config_to_connection_info(config: &ServerConfigToml) -> ConnectionInfo {
    ConnectionInfo {
        host: config.server.host.clone(),
        ssh_port: config.server.ssh_port,
        username: config.server.username.clone(),
        password: Some(config.server.password.clone()),
        local_tunnel_port: config.server.tunnel_port,
        remote_hvnc_port: config.server.hvnc_port,
    }
}

/// Load connection info from config file or use defaults
pub fn load_connection_info_with_config(config_path: Option<&str>) -> ConnectionInfo {
    if let Some(path) = config_path {
        if let Ok(config) = load_config_from_file(path) {
            return config_to_connection_info(&config);
        }
    }

    // Try default config locations
    let default_paths = vec![
        "config/server_config.toml",
        "server_config.toml",
        "./config/server_config.toml",
    ];

    for path in default_paths {
        if let Ok(config) = load_config_from_file(path) {
            return config_to_connection_info(&config);
        }
    }

    // Fall back to hardcoded defaults
    crate::common::get_default_connection_info()
}

/// Save current configuration to file
pub fn save_config_to_file(config: &ServerConfigToml, file_path: &str) -> Result<()> {
    let toml_content = toml::to_string_pretty(config)
        .map_err(|e| HvncError::OperationFailed(format!("Failed to serialize config: {}", e)))?;

    fs::write(file_path, toml_content)
        .map_err(|e| HvncError::OperationFailed(format!("Failed to write config file: {}", e)))?;

    Ok(())
}

/// Create default configuration file
pub fn create_default_config_file(file_path: &str) -> Result<()> {
    let default_config = ServerConfigToml {
        server: ServerSection {
            host: crate::common::DEFAULT_SERVER.host.to_string(),
            ssh_port: crate::common::DEFAULT_SERVER.ssh_port,
            username: crate::common::DEFAULT_SERVER.username.to_string(),
            password: crate::common::DEFAULT_SERVER.password.to_string(),
            hvnc_port: crate::common::DEFAULT_SERVER.hvnc_port,
            tunnel_port: crate::common::DEFAULT_SERVER.tunnel_port,
        },
        client: Some(ClientSection {
            auto_connect: Some(true),
            fullscreen: Some(false),
            scale_factor: Some(1.0),
            reconnect_attempts: Some(3),
            connection_timeout: Some(30),
        }),
        security: Some(SecuritySection {
            strict_host_key_checking: Some(false),
            server_alive_interval: Some(60),
            server_alive_count_max: Some(3),
        }),
        advanced: Some(AdvancedSection {
            log_level: Some("info".to_string()),
            frame_rate: Some(30),
            jpeg_quality: Some(80),
        }),
    };

    save_config_to_file(&default_config, file_path)?;
    println!("Created default configuration file: {}", file_path);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use std::io::Write;

    #[test]
    fn test_config_loading() {
        let config_content = r#"
[server]
host = "test.example.com"
ssh_port = 2222
username = "testuser"
password = "testpass"
hvnc_port = 5901
tunnel_port = 3391
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(config_content.as_bytes()).unwrap();
        
        let config = load_config_from_file(temp_file.path().to_str().unwrap()).unwrap();
        assert_eq!(config.server.host, "test.example.com");
        assert_eq!(config.server.ssh_port, 2222);
        assert_eq!(config.server.username, "testuser");
    }

    #[test]
    fn test_config_to_connection_info() {
        let config = ServerConfigToml {
            server: ServerSection {
                host: "test.com".to_string(),
                ssh_port: 22,
                username: "user".to_string(),
                password: "pass".to_string(),
                hvnc_port: 5900,
                tunnel_port: 3390,
            },
            client: None,
            security: None,
            advanced: None,
        };

        let conn_info = config_to_connection_info(&config);
        assert_eq!(conn_info.host, "test.com");
        assert_eq!(conn_info.username, "user");
        assert_eq!(conn_info.password, Some("pass".to_string()));
    }
}