//! GUI client for Hidden VNC with host discovery and application selection
//! 
//! This module provides a user-friendly GUI interface that allows users to:
//! - Discover and connect to available hosts
//! - Select applications (browsers, custom apps)
//! - Configure connection quality and options
//! - Manage connections with proper close options

use crate::error::{HvncError, Result};
use crate::client::{ConnectionInfo, AutoConnect};
use crate::common::get_default_connection_info;
use log::{info, warn, error, debug};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use serde::{Deserialize, Serialize};

/// Available application types for remote execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ApplicationType {
    Chrome { incognito: Option<bool>, url: Option<String> },
    Firefox { private: Option<bool>, url: Option<String> },
    Edge { inprivate: Option<bool>, url: Option<String> },
    Custom { path: String, args: Vec<String> },
    System(SystemApp),
}

/// System applications
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SystemApp {
    Notepad,
    Calculator,
    Paint,
    WordPad,
    Explorer,
}

/// Connection quality settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualitySettings {
    pub jpeg_quality: u8,    // 1-100
    pub frame_rate: u32,     // FPS
    pub scale_factor: f32,   // Display scaling
    pub bandwidth_limit: Option<u32>, // KB/s
}

impl Default for QualitySettings {
    fn default() -> Self {
        Self {
            jpeg_quality: 80,
            frame_rate: 30,
            scale_factor: 1.0,
            bandwidth_limit: None,
        }
    }
}

/// Host information for discovery
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostInfo {
    pub name: String,
    pub address: String,
    pub port: u16,
    pub username: String,
    pub password: Option<String>,
    pub status: HostStatus,
    pub last_seen: std::time::SystemTime,
    pub applications: Vec<ApplicationType>,
}

/// Host connection status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HostStatus {
    Online,
    Offline,
    Connecting,
    Connected,
    Error(String),
}

/// GUI client manager
pub struct GuiClient {
    discovered_hosts: Arc<Mutex<HashMap<String, HostInfo>>>,
    active_connections: Arc<Mutex<HashMap<String, ActiveConnection>>>,
    quality_presets: HashMap<String, QualitySettings>,
}

/// Active connection information
#[derive(Debug)]
pub struct ActiveConnection {
    pub host_id: String,
    pub auto_connect: AutoConnect,
    pub application: ApplicationType,
    pub quality: QualitySettings,
    pub connected_at: std::time::SystemTime,
}

impl GuiClient {
    /// Create a new GUI client
    pub fn new() -> Self {
        let mut quality_presets = HashMap::new();
        
        // Define quality presets
        quality_presets.insert("High Quality".to_string(), QualitySettings {
            jpeg_quality: 95,
            frame_rate: 60,
            scale_factor: 1.0,
            bandwidth_limit: None,
        });
        
        quality_presets.insert("Balanced".to_string(), QualitySettings {
            jpeg_quality: 80,
            frame_rate: 30,
            scale_factor: 1.0,
            bandwidth_limit: Some(1000), // 1MB/s
        });
        
        quality_presets.insert("Low Bandwidth".to_string(), QualitySettings {
            jpeg_quality: 50,
            frame_rate: 15,
            scale_factor: 0.8,
            bandwidth_limit: Some(500), // 500KB/s
        });
        
        quality_presets.insert("Mobile".to_string(), QualitySettings {
            jpeg_quality: 40,
            frame_rate: 10,
            scale_factor: 0.6,
            bandwidth_limit: Some(200), // 200KB/s
        });

        Self {
            discovered_hosts: Arc::new(Mutex::new(HashMap::new())),
            active_connections: Arc::new(Mutex::new(HashMap::new())),
            quality_presets,
        }
    }

    /// Start host discovery
    pub fn start_host_discovery(&self) -> Result<()> {
        info!("Starting host discovery...");
        
        let hosts = self.discovered_hosts.clone();
        
        thread::spawn(move || {
            loop {
                // Discover hosts on network
                if let Ok(discovered) = discover_hosts() {
                    let mut hosts_map = hosts.lock().unwrap();
                    for host in discovered {
                        hosts_map.insert(host.address.clone(), host);
                    }
                }
                
                // Update host status
                update_host_status(&hosts);
                
                thread::sleep(Duration::from_secs(30)); // Discovery every 30 seconds
            }
        });
        
        Ok(())
    }

    /// Get list of discovered hosts
    pub fn get_discovered_hosts(&self) -> Vec<HostInfo> {
        let hosts = self.discovered_hosts.lock().unwrap();
        hosts.values().cloned().collect()
    }

    /// Connect to a host with specified application
    pub fn connect_to_host(
        &self,
        host_id: &str,
        application: ApplicationType,
        quality: QualitySettings,
    ) -> Result<String> {
        info!("Connecting to host: {} with app: {:?}", host_id, application);

        let hosts = self.discovered_hosts.lock().unwrap();
        let host = hosts.get(host_id)
            .ok_or_else(|| HvncError::OperationFailed("Host not found".to_string()))?;

        // Create connection info
        let connection_info = ConnectionInfo {
            host: host.address.clone(),
            ssh_port: host.port,
            username: host.username.clone(),
            password: host.password.clone(),
            local_tunnel_port: 3390,
            remote_hvnc_port: 5900,
        };

        // Establish connection
        let mut auto_connect = AutoConnect::new(connection_info);
        let endpoint = auto_connect.auto_connect()?;

        // Send application launch command to host
        self.send_application_command(host_id, &application)?;

        // Store active connection
        let connection_id = format!("{}_{}", host_id, chrono::Utc::now().timestamp());
        let active_connection = ActiveConnection {
            host_id: host_id.to_string(),
            auto_connect,
            application,
            quality,
            connected_at: std::time::SystemTime::now(),
        };

        let mut connections = self.active_connections.lock().unwrap();
        connections.insert(connection_id.clone(), active_connection);

        info!("Successfully connected to host: {}", host_id);
        Ok(connection_id)
    }

    /// Send application launch command to host
    fn send_application_command(&self, host_id: &str, application: &ApplicationType) -> Result<()> {
        let command = match application {
            ApplicationType::Chrome { incognito, url } => {
                let mut cmd = "C:\\Program Files\\Google\\Chrome\\chrome.exe".to_string();
                if let Some(true) = incognito {
                    cmd.push_str(" --incognito");
                }
                if let Some(url) = url {
                    cmd.push_str(&format!(" {}", url));
                }
                cmd
            }
            ApplicationType::Firefox { private, url } => {
                let mut cmd = "C:\\Program Files\\Mozilla Firefox\\firefox.exe".to_string();
                if let Some(true) = private {
                    cmd.push_str(" -private-window");
                }
                if let Some(url) = url {
                    cmd.push_str(&format!(" {}", url));
                }
                cmd
            }
            ApplicationType::Edge { inprivate, url } => {
                let mut cmd = "C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe".to_string();
                if let Some(true) = inprivate {
                    cmd.push_str(" --inprivate");
                }
                if let Some(url) = url {
                    cmd.push_str(&format!(" {}", url));
                }
                cmd
            }
            ApplicationType::Custom { path, args } => {
                let mut cmd = path.clone();
                for arg in args {
                    cmd.push_str(&format!(" {}", arg));
                }
                cmd
            }
            ApplicationType::System(app) => {
                match app {
                    SystemApp::Notepad => "notepad.exe".to_string(),
                    SystemApp::Calculator => "calc.exe".to_string(),
                    SystemApp::Paint => "mspaint.exe".to_string(),
                    SystemApp::WordPad => "wordpad.exe".to_string(),
                    SystemApp::Explorer => "explorer.exe".to_string(),
                }
            }
        };

        // TODO: Send command to host via SSH or API
        info!("Would launch application: {}", command);
        Ok(())
    }

    /// Disconnect from host
    pub fn disconnect_from_host(&self, connection_id: &str) -> Result<()> {
        info!("Disconnecting from host: {}", connection_id);

        let mut connections = self.active_connections.lock().unwrap();
        if let Some(mut connection) = connections.remove(connection_id) {
            connection.auto_connect.cleanup();
            info!("Successfully disconnected from host");
        }

        Ok(())
    }

    /// Get active connections
    pub fn get_active_connections(&self) -> Vec<String> {
        let connections = self.active_connections.lock().unwrap();
        connections.keys().cloned().collect()
    }

    /// Get quality presets
    pub fn get_quality_presets(&self) -> &HashMap<String, QualitySettings> {
        &self.quality_presets
    }

    /// Get available applications for a host
    pub fn get_available_applications(&self, host_id: &str) -> Vec<ApplicationType> {
        let hosts = self.discovered_hosts.lock().unwrap();
        if let Some(host) = hosts.get(host_id) {
            host.applications.clone()
        } else {
            // Default applications (private/incognito modes are optional)
            vec![
                ApplicationType::Chrome { incognito: None, url: None },
                ApplicationType::Firefox { private: None, url: None },
                ApplicationType::Edge { inprivate: None, url: None },
                ApplicationType::System(SystemApp::Notepad),
                ApplicationType::System(SystemApp::Calculator),
                ApplicationType::System(SystemApp::Paint),
                ApplicationType::System(SystemApp::WordPad),
                ApplicationType::System(SystemApp::Explorer),
                ApplicationType::Custom { 
                    path: "".to_string(), 
                    args: vec![] 
                },
            ]
        }
    }
}

/// Discover hosts on the network
fn discover_hosts() -> Result<Vec<HostInfo>> {
    let mut hosts = Vec::new();

    // Add default configured host
    let default_info = get_default_connection_info();
    hosts.push(HostInfo {
        name: "Default Server".to_string(),
        address: default_info.host,
        port: default_info.ssh_port,
        username: default_info.username,
        password: default_info.password,
        status: HostStatus::Online,
        last_seen: std::time::SystemTime::now(),
        applications: get_default_applications(),
    });

    // TODO: Add network discovery logic
    // - Scan local network for SSH servers
    // - Check for HVNC service announcements
    // - Load from configuration files

    Ok(hosts)
}

/// Update host status by checking connectivity
fn update_host_status(hosts: &Arc<Mutex<HashMap<String, HostInfo>>>) {
    let hosts_clone = {
        let hosts_map = hosts.lock().unwrap();
        hosts_map.clone()
    };

    for (address, mut host) in hosts_clone {
        // Test SSH connectivity
        let status = match test_ssh_connection(&host.address, host.port, &host.username) {
            Ok(_) => HostStatus::Online,
            Err(e) => HostStatus::Error(e.to_string()),
        };

        host.status = status;
        host.last_seen = std::time::SystemTime::now();

        let mut hosts_map = hosts.lock().unwrap();
        hosts_map.insert(address, host);
    }
}

/// Test SSH connection to a host
fn test_ssh_connection(host: &str, port: u16, username: &str) -> Result<()> {
    use std::process::Command;

    let output = Command::new("ssh")
        .args(&[
            "-o", "ConnectTimeout=5",
            "-o", "BatchMode=yes",
            "-o", "StrictHostKeyChecking=no",
            &format!("{}@{}", username, host),
            "echo", "test"
        ])
        .output()
        .map_err(|e| HvncError::OperationFailed(format!("SSH test failed: {}", e)))?;

    if output.status.success() {
        Ok(())
    } else {
        Err(HvncError::ConnectionRefused("SSH connection failed".to_string()))
    }
}

/// Get default available applications
fn get_default_applications() -> Vec<ApplicationType> {
    vec![
        ApplicationType::Chrome { incognito: None, url: None },
        ApplicationType::Firefox { private: None, url: None },
        ApplicationType::Edge { inprivate: None, url: None },
        ApplicationType::System(SystemApp::Notepad),
        ApplicationType::System(SystemApp::Calculator),
        ApplicationType::System(SystemApp::Paint),
        ApplicationType::System(SystemApp::WordPad),
        ApplicationType::System(SystemApp::Explorer),
    ]
}

/// GUI interface trait for different GUI frameworks
pub trait GuiInterface {
    fn show_host_list(&self, hosts: &[HostInfo]);
    fn show_application_selector(&self, applications: &[ApplicationType]) -> Option<ApplicationType>;
    fn show_quality_settings(&self, presets: &HashMap<String, QualitySettings>) -> QualitySettings;
    fn show_connection_status(&self, status: &str);
    fn show_error(&self, error: &str);
    fn show_custom_app_dialog(&self) -> Option<ApplicationType>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gui_client_creation() {
        let client = GuiClient::new();
        assert_eq!(client.quality_presets.len(), 4);
        assert!(client.quality_presets.contains_key("High Quality"));
        assert!(client.quality_presets.contains_key("Low Bandwidth"));
    }

    #[test]
    fn test_application_types() {
        let chrome = ApplicationType::Chrome { incognito: Some(true), url: Some("https://google.com".to_string()) };
        let custom = ApplicationType::Custom { 
            path: "C:\\MyApp\\app.exe".to_string(), 
            args: vec!["--debug".to_string()] 
        };
        
        // Should serialize/deserialize properly
        let chrome_json = serde_json::to_string(&chrome).unwrap();
        let chrome_back: ApplicationType = serde_json::from_str(&chrome_json).unwrap();
        
        match chrome_back {
            ApplicationType::Chrome { incognito, url } => {
                assert_eq!(incognito, Some(true));
                assert_eq!(url, Some("https://google.com".to_string()));
            }
            _ => panic!("Wrong application type"),
        }
    }

    #[test]
    fn test_quality_settings() {
        let high_quality = QualitySettings {
            jpeg_quality: 95,
            frame_rate: 60,
            scale_factor: 1.0,
            bandwidth_limit: None,
        };
        
        let low_bandwidth = QualitySettings {
            jpeg_quality: 40,
            frame_rate: 10,
            scale_factor: 0.6,
            bandwidth_limit: Some(200),
        };
        
        assert!(high_quality.jpeg_quality > low_bandwidth.jpeg_quality);
        assert!(high_quality.frame_rate > low_bandwidth.frame_rate);
    }
}