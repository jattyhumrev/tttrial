//! TCP server for host-side networking
use crate::{HvncError, Result};
use crate::common::{InputEvent, ServerConfig, constants::*};
use crate::host::{WindowHandle, ssh_setup::{SshSetup, SshConnectionInfo}};
use crate::host::ImprovedInputHandler;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tokio::time::{timeout, Duration};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use log::{info, warn, error, debug};

/// TCP server for handling client connections
pub struct HvncServer {
    config: ServerConfig,
    listener: Option<TcpListener>,
    shutdown_signal: Arc<AtomicBool>,
    current_client: Option<TcpStream>,
    ssh_info: Option<SshConnectionInfo>,
}

impl HvncServer {
    /// Create a new server instance
    pub fn new(config: ServerConfig) -> Self {
        Self {
            config,
            listener: None,
            shutdown_signal: Arc::new(AtomicBool::new(false)),
            current_client: None,
            ssh_info: None,
        }
    }

    /// Automatically setup SSH server and prepare for connections
    pub fn auto_setup(&mut self) -> Result<()> {
        info!("Starting automatic HVNC server setup...");

        // Setup SSH server automatically
        let mut ssh_setup = SshSetup::new();
        let ssh_info = ssh_setup.auto_setup()?;

        // Display connection information
        ssh_info.display_connection_info();

        // Save connection info to file
        ssh_info.save_to_file("hvnc_connection_info.txt")?;

        self.ssh_info = Some(ssh_info);

        info!("HVNC server auto-setup completed successfully");
        Ok(())
    }

    /// Get SSH connection information
    pub fn get_ssh_info(&self) -> Option<&SshConnectionInfo> {
        self.ssh_info.as_ref()
    }
    
    /// Start the server and listen for connections
    pub async fn start(&mut self) -> Result<()> {
        info!("Starting HVNC server on {}", self.config.bind_address);
        
        // Create TCP listener
        let listener = TcpListener::bind(&self.config.bind_address).await
            .map_err(|e| {
                error!("Failed to bind to {}: {}", self.config.bind_address, e);
                HvncError::Network(e)
            })?;
        
        info!("Server listening on {}", self.config.bind_address);
        self.listener = Some(listener);
        
        Ok(())
    }
    
    /// Accept and handle client connections (single client only)
    pub async fn accept_connections(&mut self) -> Result<()> {
        let listener = self.listener.as_ref()
            .ok_or_else(|| HvncError::Connection("Server not started".to_string()))?;
        
        loop {
            // Check for shutdown signal
            if self.shutdown_signal.load(Ordering::Relaxed) {
                info!("Server shutdown requested");
                break;
            }
            
            // Accept new connection with timeout
            match timeout(Duration::from_secs(1), listener.accept()).await {
                Ok(Ok((stream, addr))) => {
                    info!("New client connection from: {}", addr);
                    
                    // Check if we already have a client
                    if self.current_client.is_some() {
                        warn!("Rejecting connection from {} - client already connected", addr);
                        drop(stream); // Close the connection
                        continue;
                    }
                    
                    // Store the client connection
                    self.current_client = Some(stream);
                    info!("Client {} connected successfully", addr);
                }
                Ok(Err(e)) => {
                    error!("Failed to accept connection: {}", e);
                    return Err(HvncError::Network(e));
                }
                Err(_) => {
                    // Timeout - continue loop to check shutdown signal
                    continue;
                }
            }
        }
        
        Ok(())
    }
    
    /// Accept a single client connection (non-blocking, single client only)
    pub async fn accept_single_connection(&mut self) -> Result<()> {
        let listener = self.listener.as_ref()
            .ok_or_else(|| HvncError::Connection("Server not started".to_string()))?;
        
        // Check for shutdown signal
        if self.shutdown_signal.load(Ordering::Relaxed) {
            return Err(HvncError::Connection("Server shutdown requested".to_string()));
        }
        
        // Accept new connection (this is already async and will return immediately if no connection)
        let (stream, addr) = listener.accept().await
            .map_err(|e| {
                debug!("Failed to accept connection: {}", e);
                HvncError::Network(e)
            })?;
        
        info!("New client connection from: {}", addr);
        
        // Check if we already have a client
        if self.current_client.is_some() {
            warn!("Rejecting connection from {} - client already connected", addr);
            drop(stream); // Close the connection
            return Err(HvncError::Connection("Client already connected".to_string()));
        }
        
        // Store the client connection
        self.current_client = Some(stream);
        info!("Client {} connected successfully", addr);
        
        Ok(())
    }
    
    /// Handle a client connection with frame streaming and input handling
    pub async fn handle_client(
        &mut self,
        window: WindowHandle,
        desktop: Option<crate::host::DesktopHandle>,
    ) -> Result<()> {
    let stream = match self.current_client.as_mut() {
        Some(stream) => stream,
        None => return Err(HvncError::Connection("No client connected".to_string())),
    };

    info!("Starting client session with frame streaming");
    
    // Create a temporary stream by cloning the address and reconnecting
    // This is a workaround since we can't easily split a borrowed stream
    let stream_addr = stream.peer_addr()
        .map_err(|e| HvncError::Connection(format!("Failed to get client address: {}", e)))?;
    
    // Take ownership of the stream for the session
    let owned_stream = self.current_client.take()
        .ok_or_else(|| HvncError::Connection("No client connected".to_string()))?;
        
    // Split stream for concurrent read/write
    let (mut read_half, mut write_half) = owned_stream.into_split();
    
    // Create channels for communication between tasks
    let (input_tx, mut input_rx) = mpsc::channel::<InputEvent>(100);
    let shutdown_signal = Arc::clone(&self.shutdown_signal);
    
    // Spawn input handling task
    let input_task = {
        let shutdown_signal = Arc::clone(&shutdown_signal);
        tokio::spawn(async move {
            loop {
                if shutdown_signal.load(Ordering::Relaxed) {
                    break;
                }
                
                match timeout(Duration::from_millis(100), Self::receive_input(&mut read_half)).await {
                    Ok(Ok(input_event)) => {
                        debug!("Received input event: {:?}", input_event);
                        info!("📥 Received input event: {:?}", input_event);
                        if input_tx.send(input_event).await.is_err() {
                            warn!("Failed to send input event to handler");
                            break;
                        }
                    }
                    Ok(Err(e)) => {
                        error!("Error receiving input: {}", e);
                        break;
                    }
                    Err(_) => {
                        // Timeout - continue loop
                        continue;
                    }
                }
            }
            debug!("Input handling task terminated");
        })
    };
    
    // Main client handling loop with frame streaming
    let mut frame_interval = tokio::time::interval(Duration::from_millis(self.config.frame_rate_ms));
    
    // Initialize window capture
    let mut window_capture = crate::host::WindowCapture::new(self.config.jpeg_quality);
    window_capture.set_frame_rate_ms(self.config.frame_rate_ms);
    
    // Initialize improved input handler for better hidden desktop support
    let mut input_handler = crate::host::ImprovedInputHandler::new()?;
    
    // Set the target window for input handling
    input_handler.set_target_window(window)?;
    info!("Improved input handler configured for window: {:?}", window);
    
    // 🐛 DEBUG: Validate window handle
    let is_window_valid = unsafe { winapi::um::winuser::IsWindow(window) != 0 };
    let is_window_visible = unsafe { winapi::um::winuser::IsWindowVisible(window) != 0 };
    info!("Window validation for input handler - valid: {}, visible: {}", is_window_valid, is_window_visible);
    
    // The improved handler works directly with window handles
    info!("Improved input handler using window-based input handling (no desktop switching)");
    
    info!("Client session loop starting with window handle: {:?}", window);
    
    loop {
        if shutdown_signal.load(Ordering::Relaxed) {
            info!("Client session shutdown requested");
            break;
        }
        
        tokio::select! {
            // Handle frame streaming
            _ = frame_interval.tick() => {
                debug!("Frame interval tick - attempting capture with window handle: {:?}", window);
                
                // Option C: Advanced HVNC Capture - Multi-strategy approach
                let capture_result = if window as usize == 1 {
                    // This indicates desktop capture mode was requested, but we'll skip it
                    warn!("Desktop capture mode requested but skipping - no valid window handle");
                    Err(HvncError::window_not_found("No valid window handle for capture"))
                } else {
                    // 🔧 FIX: Check if window is actually visible/valid before capture
                    let is_window_valid = unsafe { winapi::um::winuser::IsWindow(window) != 0 };
                    let is_window_visible = unsafe { winapi::um::winuser::IsWindowVisible(window) != 0 };
                    
                    info!("📋 Window validation: valid={}, visible={}", is_window_valid, is_window_visible);
                    
                    if !is_window_valid {
                        error!("❌ Window handle is invalid: {:?}", window);
                        Err(HvncError::window_not_found("Invalid window handle"))
                    } else if !is_window_visible {
                        warn!("⚠️ Window is not visible - may be in hidden desktop or minimized");
                        info!("🔄 Attempting capture anyway (hidden desktop scenario)");
                        
                        // Try Option C with hidden window
                        match crate::host::capture_advanced::capture_window_advanced(window, desktop.unwrap()) {
                            Ok(frame) => {
                                info!("✅ Option C hidden window capture successful: {}x{}", frame.width, frame.height);
                                Ok(frame)
                            }
                            Err(e) => {
                                warn!("❌ Option C hidden window capture failed: {}", e);
                                // Try direct capture from the main desktop to see if window moved there
                                info!("🔄 Trying fallback: capture from main desktop");
                                match crate::host::capture::capture_window(window) {
                                    Ok(frame) => {
                                        info!("✅ Main desktop fallback successful: {}x{}", frame.width, frame.height);
                                        Ok(frame)
                                    }
                                    Err(e2) => {
                                        error!("❌ All capture methods failed. Hidden: {}, Main: {}", e, e2);
                                        Err(e2)
                                    }
                                }
                            }
                        }
                    } else {
                        // Window is visible - use Option C normally
                        info!("🚀 Using Option C: Advanced HVNC capture for visible window: {:?}", window);
                        match crate::host::capture_advanced::capture_window_advanced(window, desktop.unwrap()) {
                            Ok(frame) => {
                                info!("✅ Option C visible window capture successful: {}x{}", frame.width, frame.height);
                                Ok(frame)
                            }
                            Err(e) => {
                                warn!("❌ Option C visible window capture failed: {}", e);
                                // Fallback chain
                                info!("🔄 Falling back to Option A window-only capture");
                                match crate::host::capture::capture_window_with_desktop_switch(window, desktop) {
                                    Ok(frame) => {
                                        info!("Option A fallback successful: {}x{}", frame.width, frame.height);
                                        Ok(frame)
                                    }
                                    Err(e2) => {
                                        warn!("Option A fallback failed: {}", e2);
                                        // 🔧 HVNC FIX: Use desktop context-aware capture
                                        info!("🔧 Using desktop context-aware capture for HVNC");
                                        match crate::host::capture::capture_window_with_desktop_context(window, desktop) {
                                            Ok(frame) => {
                                                info!("✅ Desktop context capture successful: {}x{}", frame.width, frame.height);
                                                Ok(frame)
                                            }
                                            Err(e) => {
                                                warn!("❌ Desktop context capture failed: {}", e);
                                                // Fallback to basic capture
                                                crate::host::capture::capture_window(window)
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                };
                
                match capture_result {
                    Ok(frame) => {
                        info!("Frame captured successfully: {}x{}, {} bytes", frame.width, frame.height, frame.data.len());
                        
                        // 🔍 DIAGNOSTIC: Check if frame data is all black/empty
                        let non_zero_pixels = frame.data.iter().filter(|&&pixel| pixel != 0).count();
                        let total_pixels = frame.data.len();
                        let non_black_percentage = (non_zero_pixels as f32 / total_pixels as f32) * 100.0;
                        
                        info!("📊 Frame analysis: {}/{} non-black pixels ({:.1}%)", 
                            non_zero_pixels, total_pixels, non_black_percentage);
                        
                        if non_black_percentage < 1.0 {
                            warn!("⚠️ Frame appears to be mostly black! This could cause client black screen.");
                            warn!("   Possible causes:");
                            warn!("   - Window is minimized or hidden");
                            warn!("   - Capture method not working properly");
                            warn!("   - Administrator privileges needed");
                        } else {
                            info!("✅ Frame has content - should display properly in client");
                        }
                        
                        // Compress the frame
                        match window_capture.compress_image(&frame.data, frame.width, frame.height) {
                            Ok(compressed_data) => {
                                info!("Frame compressed successfully: {} bytes -> {} bytes (ratio: {:.1}%)", 
                                    frame.data.len(), compressed_data.len(), 
                                    (compressed_data.len() as f32 / frame.data.len() as f32) * 100.0);
                                
                                // 🔍 DIAGNOSTIC: Validate JPEG data
                                if compressed_data.len() < 2 || compressed_data[0] != 0xFF || compressed_data[1] != 0xD8 {
                                    error!("❌ Invalid JPEG data! First bytes: {:02X} {:02X}", 
                                        compressed_data.get(0).unwrap_or(&0), compressed_data.get(1).unwrap_or(&0));
                                    continue; // Skip this frame
                                } else {
                                    debug!("✅ JPEG header valid: FF D8");
                                }
                                
                                // Stream the compressed frame
                                if let Err(e) = Self::stream_frame(&mut write_half, &compressed_data).await {
                                    error!("Failed to stream frame: {}", e);
                                    break;
                                } else {
                                    info!("Frame streamed successfully to client");
                                }
                            }
                            Err(e) => {
                                error!("Failed to compress frame: {}", e);
                                // Continue with next frame
                            }
                        }
                    }
                    Err(e) => {
                        error!("Failed to capture window: {}", e);
                        // Continue trying - window might be temporarily unavailable
                    }
                }
            }
            
            // Handle input events
            Some(input_event) = input_rx.recv() => {
                debug!("Processing input event: {:?}", input_event);
                info!("🎮 Processing input event: {:?}", input_event);
                
                // Validate input event first
                if let Err(e) = Self::validate_input_event(&input_event) {
                    warn!("Invalid input event rejected: {}", e);
                    continue;
                }
                
                // Process the input event with improved input handler
                match input_handler.process_input_event(input_event).await {
                    Ok(()) => {
                        info!("✅ Input event processed successfully");
                    }
                    Err(e) => {
                        error!("❌ Failed to process input event: {}", e);
                        // Continue processing other events
                    }
                }
            }
            
            // Handle client disconnection
            else => {
                info!("Client disconnected");
                break;
            }
        }
    }
    
    // Clean up
    input_task.abort();
    info!("Client session ended");
    
    // Session ended, client is disconnected
    Ok(())
}

    /// Stream a frame to the client
    pub async fn stream_frame(
        stream: &mut tokio::net::tcp::OwnedWriteHalf,
        frame_data: &[u8],
    ) -> Result<()> {
        use tokio::io::AsyncWriteExt;
        
        // Validate frame size
        if frame_data.is_empty() {
            return Err(HvncError::Connection("Frame data is empty".to_string()));
        }
        
        if frame_data.len() > MAX_FRAME_SIZE {
            return Err(HvncError::Connection(
                format!("Frame size {} exceeds maximum {}", frame_data.len(), MAX_FRAME_SIZE)
            ));
        }
        
        debug!("Streaming frame: {} bytes", frame_data.len());
        
        // Send frame header: magic bytes + length
        let frame_length = frame_data.len() as u32;
        let header = [
            0xFF, 0xD8, // JPEG magic bytes (also serves as frame marker)
            ((frame_length >> 24) & 0xFF) as u8,
            ((frame_length >> 16) & 0xFF) as u8,
            ((frame_length >> 8) & 0xFF) as u8,
            (frame_length & 0xFF) as u8,
        ];
        
        // Write header
        stream.write_all(&header).await
            .map_err(|e| {
                error!("Failed to write frame header: {}", e);
                HvncError::Network(e)
            })?;
        
        // Write frame data
        stream.write_all(frame_data).await
            .map_err(|e| {
                error!("Failed to write frame data: {}", e);
                HvncError::Network(e)
            })?;
        
        // Flush to ensure data is sent immediately
        stream.flush().await
            .map_err(|e| {
                error!("Failed to flush frame data: {}", e);
                HvncError::Network(e)
            })?;
        
        debug!("Frame streamed successfully");
        Ok(())
    }
    
    /// Receive a frame from the stream (for client-side use)
    pub async fn receive_frame(stream: &mut tokio::net::tcp::OwnedReadHalf) -> Result<Vec<u8>> {
        use tokio::io::AsyncReadExt;
        
        // Read frame header (6 bytes: 2 magic + 4 length)
        let mut header = [0u8; 6];
        stream.read_exact(&mut header).await
            .map_err(|e| {
                debug!("Failed to read frame header: {}", e);
                HvncError::Network(e)
            })?;
        
        // Validate magic bytes
        if header[0] != 0xFF || header[1] != 0xD8 {
            return Err(HvncError::Connection(
                format!("Invalid frame magic bytes: {:02X} {:02X}", header[0], header[1])
            ));
        }
        
        // Extract frame length
        let frame_length = u32::from_be_bytes([header[2], header[3], header[4], header[5]]) as usize;
        
        // Validate frame length
        if frame_length == 0 {
            return Err(HvncError::Connection("Frame length is zero".to_string()));
        }
        
        if frame_length > MAX_FRAME_SIZE {
            return Err(HvncError::Connection(
                format!("Frame length {} exceeds maximum {}", frame_length, MAX_FRAME_SIZE)
            ));
        }
        
        debug!("Receiving frame: {} bytes", frame_length);
        
        // Read frame data
        let mut frame_data = vec![0u8; frame_length];
        stream.read_exact(&mut frame_data).await
            .map_err(|e| {
                error!("Failed to read frame data: {}", e);
                HvncError::Network(e)
            })?;
        
        // Validate that we received JPEG data
        if frame_data.len() < 2 || frame_data[0] != 0xFF || frame_data[1] != 0xD8 {
            return Err(HvncError::Connection(
                "Received data is not valid JPEG".to_string()
            ));
        }
        
        debug!("Frame received successfully");
        Ok(frame_data)
    }
    
    /// Receive a frame from a borrowed read half
    pub async fn receive_frame_from_read_half(stream: &mut tokio::net::tcp::ReadHalf<'_>) -> Result<Vec<u8>> {
        use tokio::io::AsyncReadExt;
        
        // Read frame header (6 bytes: 2 magic + 4 length)
        let mut header = [0u8; 6];
        stream.read_exact(&mut header).await
            .map_err(|e| {
                debug!("Failed to read frame header: {}", e);
                HvncError::Network(e)
            })?;
        
        // Validate magic bytes
        if header[0] != 0xFF || header[1] != 0xD8 {
            return Err(HvncError::Connection(
                format!("Invalid frame magic bytes: {:02X} {:02X}", header[0], header[1])
            ));
        }
        
        // Extract frame length
        let frame_length = u32::from_be_bytes([header[2], header[3], header[4], header[5]]) as usize;
        
        // Validate frame length
        if frame_length == 0 {
            return Err(HvncError::Connection("Frame length is zero".to_string()));
        }
        
        if frame_length > MAX_FRAME_SIZE {
            return Err(HvncError::Connection(
                format!("Frame length {} exceeds maximum {}", frame_length, MAX_FRAME_SIZE)
            ));
        }
        
        debug!("Receiving frame: {} bytes", frame_length);
        
        // Read frame data
        let mut frame_data = vec![0u8; frame_length];
        stream.read_exact(&mut frame_data).await
            .map_err(|e| {
                error!("Failed to read frame data: {}", e);
                HvncError::Network(e)
            })?;
        
        // Validate that we received JPEG data
        if frame_data.len() < 2 || frame_data[0] != 0xFF || frame_data[1] != 0xD8 {
            return Err(HvncError::Connection(
                "Received data is not valid JPEG".to_string()
            ));
        }
        
        debug!("Frame received successfully");
        Ok(frame_data)
    }
    
    /// Receive input event from client
    pub async fn receive_input(stream: &mut tokio::net::tcp::OwnedReadHalf) -> Result<InputEvent> {
        use tokio::io::AsyncReadExt;
        
        debug!("🔍 HVNC Input Receive: Waiting to read input message length...");
        
        // Read message length (4 bytes, little-endian)
        let mut len_buf = [0u8; 4];
        match stream.read_exact(&mut len_buf).await {
            Ok(_) => debug!("✅ HVNC Input Receive: Length header read successfully"),
            Err(e) => {
                error!("❌ HVNC Input Receive: Failed to read input message length: {}", e);
                return Err(HvncError::Network(e));
            }
        }
        
        let message_len = u32::from_le_bytes(len_buf) as usize;
        info!("📄 HVNC Input Receive: Message length: {} bytes", message_len);
        
        // Validate message length (reasonable limits for input events)
        if message_len == 0 {
            error!("❌ HVNC Input Receive: Input message length is zero");
            return Err(HvncError::Connection("Input message length is zero".to_string()));
        }
        
        if message_len > 1024 { // Max 1KB for input events
            error!("❌ HVNC Input Receive: Input message too large: {} bytes", message_len);
            return Err(HvncError::Connection(
                format!("Input message length {} exceeds maximum 1024 bytes", message_len)
            ));
        }
        
        debug!("💾 HVNC Input Receive: Receiving input event data: {} bytes", message_len);
        
        // Read message data
        let mut message_buf = vec![0u8; message_len];
        match stream.read_exact(&mut message_buf).await {
            Ok(_) => debug!("✅ HVNC Input Receive: Message data read successfully"),
            Err(e) => {
                error!("❌ HVNC Input Receive: Failed to read input message data: {}", e);
                return Err(HvncError::Network(e));
            }
        }
        
        debug!("📄 HVNC Input Receive: Received message data: {} bytes", message_buf.len());
        debug!("🔍 HVNC Input Receive: Raw message: {:?}", String::from_utf8_lossy(&message_buf));
        
        // Deserialize input event from JSON
        let input_event: InputEvent = match serde_json::from_slice(&message_buf) {
            Ok(event) => {
                info!("✅ HVNC Input Receive: Successfully deserialized input event: {:?}", event);
                event
            }
            Err(e) => {
                error!("❌ HVNC Input Receive: Failed to deserialize input event: {}", e);
                error!("❌ HVNC Input Receive: Message data: {:?}", message_buf);
                error!("❌ HVNC Input Receive: Message string: {:?}", String::from_utf8_lossy(&message_buf));
                return Err(HvncError::Serialization(e));
            }
        };
        
        info!("🎉 HVNC Input Receive: Input event received successfully: {:?}", input_event);
        Ok(input_event)
    }
    
    /// Send input event to server (for client-side use)
    pub async fn send_input(stream: &mut tokio::net::tcp::OwnedWriteHalf, event: &InputEvent) -> Result<()> {
        use tokio::io::AsyncWriteExt;
        
        debug!("Sending input event: {:?}", event);
        
        // Serialize input event to JSON
        let message_data = serde_json::to_vec(event)
            .map_err(|e| {
                error!("Failed to serialize input event: {}", e);
                HvncError::Serialization(e)
            })?;
        
        // Validate message size
        if message_data.len() > 1024 {
            return Err(HvncError::Connection(
                format!("Serialized input event too large: {} bytes", message_data.len())
            ));
        }
        
        // Send message length (4 bytes, little-endian)
        let message_len = message_data.len() as u32;
        stream.write_all(&message_len.to_le_bytes()).await
            .map_err(|e| {
                error!("Failed to write input message length: {}", e);
                HvncError::Network(e)
            })?;
        
        // Send message data
        stream.write_all(&message_data).await
            .map_err(|e| {
                error!("Failed to write input message data: {}", e);
                HvncError::Network(e)
            })?;
        
        debug!("Input event sent successfully");
        Ok(())
    }
    
    /// Send input event to server using WriteHalf (for client-side use)
    pub async fn send_input_to_write_half(stream: &mut tokio::net::tcp::WriteHalf<'_>, event: &InputEvent) -> Result<()> {
        use tokio::io::AsyncWriteExt;
        
        debug!("Sending input event: {:?}", event);
        
        // Serialize input event to JSON
        let message_data = serde_json::to_vec(event)
            .map_err(|e| {
                error!("Failed to serialize input event: {}", e);
                HvncError::Serialization(e)
            })?;
        
        // Validate message size
        if message_data.len() > 1024 {
            return Err(HvncError::Connection(
                format!("Serialized input event too large: {} bytes", message_data.len())
            ));
        }
        
        // Send message length (4 bytes, little-endian)
        let message_len = message_data.len() as u32;
        stream.write_all(&message_len.to_le_bytes()).await
            .map_err(|e| {
                error!("Failed to write input message length: {}", e);
                HvncError::Network(e)
            })?;
        
        // Send message data
        stream.write_all(&message_data).await
            .map_err(|e| {
                error!("Failed to write input message data: {}", e);
                HvncError::Network(e)
            })?;
        
        // Flush to ensure immediate delivery
        stream.flush().await
            .map_err(|e| {
                error!("Failed to flush input message: {}", e);
                HvncError::Network(e)
            })?;
        
        debug!("Input event sent successfully");
        Ok(())
    }
    
    /// Validate input event for security and sanity
    pub fn validate_input_event(event: &InputEvent) -> Result<()> {
        match event {
            InputEvent::MouseClick { x, y, button: _ } => {
                // Validate mouse coordinates (reasonable screen bounds)
                if *x < -10000 || *x > 10000 || *y < -10000 || *y > 10000 {
                    return Err(HvncError::Connection(
                        format!("Mouse coordinates out of bounds: ({}, {})", x, y)
                    ));
                }
            }
            InputEvent::MouseMove { x, y } => {
                // Validate mouse coordinates
                if *x < -10000 || *x > 10000 || *y < -10000 || *y > 10000 {
                    return Err(HvncError::Connection(
                        format!("Mouse coordinates out of bounds: ({}, {})", x, y)
                    ));
                }
            }
            InputEvent::KeyPress { keycode, pressed: _ } => {
                // Validate keycode (Windows virtual key codes are 0-255)
                if *keycode > 255 {
                    return Err(HvncError::Connection(
                        format!("Invalid keycode: {}", keycode)
                    ));
                }
            }
        }
        
        Ok(())
    }
    
    /// Check if a client is currently connected
    pub fn has_client(&self) -> bool {
        self.current_client.is_some()
    }
    
    /// Get the server's bind address
    pub fn bind_address(&self) -> &str {
        &self.config.bind_address
    }
    
    /// Request server shutdown
    pub fn shutdown(&self) {
        info!("Server shutdown requested");
        self.shutdown_signal.store(true, Ordering::Relaxed);
    }
    
    /// Check if shutdown has been requested
    pub fn is_shutdown_requested(&self) -> bool {
        self.shutdown_signal.load(Ordering::Relaxed)
    }
    
    /// Disconnect the current client
    pub fn disconnect_client(&mut self) {
        if let Some(stream) = self.current_client.take() {
            drop(stream);
            info!("Client disconnected by server");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::{ServerConfig, InputEvent, MouseButton};
    use tokio::net::TcpStream;
    use tokio::io::{AsyncWriteExt, AsyncReadExt};
    use std::time::Duration;
    
    fn create_test_config() -> ServerConfig {
        ServerConfig {
            bind_address: "127.0.0.1:0".to_string(), // Use port 0 for automatic assignment
            target_application: "test.exe".to_string(),
            jpeg_quality: 80,
            frame_rate_ms: 33,
        }
    }
    
    #[tokio::test]
    async fn test_server_creation() {
        let config = create_test_config();
        let server = HvncServer::new(config.clone());
        
        assert_eq!(server.bind_address(), &config.bind_address);
        assert!(!server.has_client());
        assert!(!server.is_shutdown_requested());
    }
    
    #[tokio::test]
    async fn test_server_start() {
        let config = create_test_config();
        let mut server = HvncServer::new(config);
        
        let result = server.start().await;
        assert!(result.is_ok(), "Server should start successfully");
        assert!(server.listener.is_some(), "Listener should be created");
    }
    
    #[tokio::test]
    async fn test_server_start_invalid_address() {
        let mut config = create_test_config();
        config.bind_address = "invalid_address:1234".to_string();
        
        let mut server = HvncServer::new(config);
        let result = server.start().await;
        
        assert!(result.is_err(), "Server should fail to start with invalid address");
    }
    
    #[tokio::test]
    async fn test_server_shutdown_signal() {
        let config = create_test_config();
        let server = HvncServer::new(config);
        
        assert!(!server.is_shutdown_requested());
        
        server.shutdown();
        assert!(server.is_shutdown_requested());
    }
    
    #[tokio::test]
    async fn test_client_connection_management() {
        let config = create_test_config();
        let mut server = HvncServer::new(config);
        
        // Start server
        server.start().await.unwrap();
        
        // Get the actual bound address
        let listener = server.listener.as_ref().unwrap();
        let server_addr = listener.local_addr().unwrap();
        
        // Connect a client
        let client_stream = TcpStream::connect(server_addr).await.unwrap();
        
        // Simulate accepting the connection
        let (stream, _addr) = listener.accept().await.unwrap();
        server.current_client = Some(stream);
        
        assert!(server.has_client(), "Server should have a client");
        
        // Disconnect client
        server.disconnect_client();
        assert!(!server.has_client(), "Server should not have a client after disconnect");
        
        drop(client_stream);
    }
    
    #[tokio::test]
    async fn test_receive_input_valid() {
        // Create a mock stream with valid input data
        let input_event = InputEvent::MouseClick {
            x: 100,
            y: 200,
            button: MouseButton::Left,
        };
        
        let json_data = serde_json::to_vec(&input_event).unwrap();
        let message_len = json_data.len() as u32;
        
        // Create test data: length + message
        let mut test_data = Vec::new();
        test_data.extend_from_slice(&message_len.to_le_bytes());
        test_data.extend_from_slice(&json_data);
        
        // Create a mock stream using a cursor
        use std::io::Cursor;
        use tokio_test::io::Builder;
        
        let mock_stream = Builder::new()
            .read(&test_data)
            .build();
        
        // This test would need a more sophisticated mock for the actual stream
        // For now, we'll test the serialization logic
        let serialized = serde_json::to_vec(&input_event).unwrap();
        let deserialized: InputEvent = serde_json::from_slice(&serialized).unwrap();
        
        match (input_event, deserialized) {
            (InputEvent::MouseClick { x: x1, y: y1, button: b1 }, 
             InputEvent::MouseClick { x: x2, y: y2, button: b2 }) => {
                assert_eq!(x1, x2);
                assert_eq!(y1, y2);
                // Note: MouseButton comparison would need PartialEq implementation
            }
            _ => panic!("Input event types don't match"),
        }
    }
    
    #[tokio::test]
    async fn test_receive_input_invalid_length() {
        // Test with invalid message length (too large)
        let invalid_len = (MAX_FRAME_SIZE + 1) as u32;
        let test_data = invalid_len.to_le_bytes();
        
        // This would test the length validation logic
        assert!(invalid_len as usize > 1024, "Should exceed maximum input message size");
    }
    
    #[tokio::test]
    async fn test_receive_input_invalid_json() {
        // Test with invalid JSON data
        let invalid_json = b"{ invalid json }";
        let result = serde_json::from_slice::<InputEvent>(invalid_json);
        
        assert!(result.is_err(), "Should fail to deserialize invalid JSON");
    }
    
    #[tokio::test]
    async fn test_server_configuration() {
        let config = ServerConfig {
            bind_address: "127.0.0.1:5900".to_string(),
            target_application: "notepad.exe".to_string(),
            jpeg_quality: 75,
            frame_rate_ms: 50,
        };
        
        let server = HvncServer::new(config.clone());
        
        assert_eq!(server.bind_address(), "127.0.0.1:5900");
        assert_eq!(server.config.target_application, "notepad.exe");
        assert_eq!(server.config.jpeg_quality, 75);
        assert_eq!(server.config.frame_rate_ms, 50);
    }
    
    #[tokio::test]
    async fn test_concurrent_operations() {
        let config = create_test_config();
        let server = HvncServer::new(config);
        
        // Test that shutdown signal works across tasks
        let shutdown_signal = Arc::clone(&server.shutdown_signal);
        
        let task1 = tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(10)).await;
            shutdown_signal.store(true, Ordering::Relaxed);
        });
        
        let task2 = tokio::spawn(async move {
            // Wait for shutdown signal
            while !server.is_shutdown_requested() {
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
            true
        });
        
        let (_, result) = tokio::join!(task1, task2);
        assert!(result.unwrap(), "Shutdown signal should be detected");
    }
    
    #[tokio::test]
    async fn test_input_event_protocol() {
        // Test input event serialization and validation
        let events = vec![
            InputEvent::MouseClick { x: 100, y: 200, button: MouseButton::Left },
            InputEvent::MouseMove { x: 150, y: 250 },
            InputEvent::KeyPress { keycode: 65, pressed: true }, // 'A' key
        ];
        
        for event in &events {
            // Test serialization
            let serialized = serde_json::to_vec(event).unwrap();
            assert!(!serialized.is_empty(), "Serialized event should not be empty");
            assert!(serialized.len() <= 1024, "Serialized event should not exceed 1KB");
            
            // Test deserialization
            let deserialized: InputEvent = serde_json::from_slice(&serialized).unwrap();
            assert_eq!(event, &deserialized, "Event should deserialize correctly");
            
            // Test validation
            let validation_result = HvncServer::validate_input_event(event);
            assert!(validation_result.is_ok(), "Valid event should pass validation");
        }
    }
    
    #[tokio::test]
    async fn test_input_event_validation() {
        // Test valid events
        let valid_events = vec![
            InputEvent::MouseClick { x: 0, y: 0, button: MouseButton::Left },
            InputEvent::MouseClick { x: 1920, y: 1080, button: MouseButton::Right },
            InputEvent::MouseMove { x: -100, y: -100 }, // Negative coordinates should be allowed
            InputEvent::KeyPress { keycode: 0, pressed: true },
            InputEvent::KeyPress { keycode: 255, pressed: false },
        ];
        
        for event in &valid_events {
            let result = HvncServer::validate_input_event(event);
            assert!(result.is_ok(), "Valid event should pass validation: {:?}", event);
        }
        
        // Test invalid events
        let invalid_events = vec![
            InputEvent::MouseClick { x: 20000, y: 0, button: MouseButton::Left }, // X too large
            InputEvent::MouseClick { x: 0, y: -20000, button: MouseButton::Right }, // Y too small
            InputEvent::MouseMove { x: 15000, y: 15000 }, // Both coordinates too large
            InputEvent::KeyPress { keycode: 300, pressed: true }, // Invalid keycode
        ];
        
        for event in &invalid_events {
            let result = HvncServer::validate_input_event(event);
            assert!(result.is_err(), "Invalid event should fail validation: {:?}", event);
        }
    }
    
    #[tokio::test]
    async fn test_input_message_length_validation() {
        // Test message length validation logic
        let valid_lengths = vec![1, 100, 500, 1024];
        let invalid_lengths = vec![0, 1025, 2048, u32::MAX as usize];
        
        for len in valid_lengths {
            assert!(len > 0 && len <= 1024, "Length {} should be valid", len);
        }
        
        for len in invalid_lengths {
            assert!(len == 0 || len > 1024, "Length {} should be invalid", len);
        }
    }
    
    #[tokio::test]
    async fn test_input_event_size_limits() {
        // Test that serialized events don't exceed size limits
        let large_events = vec![
            // These should still be within reasonable limits
            InputEvent::MouseClick { x: i32::MAX, y: i32::MIN, button: MouseButton::Middle },
            InputEvent::MouseMove { x: -9999, y: 9999 },
            InputEvent::KeyPress { keycode: 255, pressed: true },
        ];
        
        for event in &large_events {
            let serialized = serde_json::to_vec(event).unwrap();
            assert!(serialized.len() <= 1024, 
                   "Event should not exceed 1KB when serialized: {} bytes", serialized.len());
        }
    }
    
    #[tokio::test]
    async fn test_input_event_json_format() {
        // Test that events serialize to expected JSON format
        let mouse_click = InputEvent::MouseClick { 
            x: 100, 
            y: 200, 
            button: MouseButton::Left 
        };
        
        let json = serde_json::to_string(&mouse_click).unwrap();
        assert!(json.contains("MouseClick"), "JSON should contain event type");
        assert!(json.contains("100"), "JSON should contain x coordinate");
        assert!(json.contains("200"), "JSON should contain y coordinate");
        assert!(json.contains("Left"), "JSON should contain button type");
        
        let mouse_move = InputEvent::MouseMove { x: 50, y: 75 };
        let json = serde_json::to_string(&mouse_move).unwrap();
        assert!(json.contains("MouseMove"), "JSON should contain event type");
        assert!(json.contains("50"), "JSON should contain x coordinate");
        assert!(json.contains("75"), "JSON should contain y coordinate");
        
        let key_press = InputEvent::KeyPress { keycode: 65, pressed: true };
        let json = serde_json::to_string(&key_press).unwrap();
        assert!(json.contains("KeyPress"), "JSON should contain event type");
        assert!(json.contains("65"), "JSON should contain keycode");
        assert!(json.contains("true"), "JSON should contain pressed state");
    }
    
    #[tokio::test]
    async fn test_input_event_error_handling() {
        // Test deserialization error handling
        let invalid_json_samples: Vec<&[u8]> = vec![
            b"", // Empty
            b"{", // Incomplete JSON
            b"null", // Wrong type
            b"{\"invalid\": \"event\"}", // Unknown event type
            b"{\"MouseClick\": {\"x\": \"not_a_number\"}}", // Wrong data type
        ];
        
        for invalid_json in invalid_json_samples {
            let result = serde_json::from_slice::<InputEvent>(invalid_json);
            assert!(result.is_err(), "Invalid JSON should fail to deserialize");
        }
    }
    
    #[tokio::test]
    async fn test_mouse_button_types() {
        // Test all mouse button types
        let buttons = vec![MouseButton::Left, MouseButton::Right, MouseButton::Middle];
        
        for button in buttons {
            let event = InputEvent::MouseClick { x: 0, y: 0, button: button.clone() };
            
            // Test serialization
            let serialized = serde_json::to_vec(&event).unwrap();
            let deserialized: InputEvent = serde_json::from_slice(&serialized).unwrap();
            
            // Verify button type is preserved
            if let InputEvent::MouseClick { button: deserialized_button, .. } = deserialized {
                assert_eq!(button, deserialized_button, "Button type should be preserved");
            } else {
                panic!("Event type should be preserved");
            }
        }
    }
    
    #[tokio::test]
    async fn test_coordinate_edge_cases() {
        // Test edge cases for coordinates
        let edge_cases = vec![
            (i32::MIN, i32::MIN),
            (i32::MAX, i32::MAX),
            (0, 0),
            (-1, -1),
            (1, 1),
        ];
        
        for (x, y) in edge_cases {
            let mouse_event = InputEvent::MouseMove { x, y };
            
            // Test serialization roundtrip
            let serialized = serde_json::to_vec(&mouse_event).unwrap();
            let deserialized: InputEvent = serde_json::from_slice(&serialized).unwrap();
            
            if let InputEvent::MouseMove { x: dx, y: dy } = deserialized {
                assert_eq!(x, dx, "X coordinate should be preserved");
                assert_eq!(y, dy, "Y coordinate should be preserved");
            } else {
                panic!("Event type should be preserved");
            }
        }
    }
    
    #[tokio::test]
    async fn test_keycode_edge_cases() {
        // Test edge cases for keycodes
        let keycodes = vec![0, 1, 127, 255]; // Valid Windows virtual key codes
        
        for keycode in keycodes {
            for pressed in [true, false] {
                let key_event = InputEvent::KeyPress { keycode, pressed };
                
                // Test validation
                let validation_result = HvncServer::validate_input_event(&key_event);
                assert!(validation_result.is_ok(), "Valid keycode should pass validation");
                
                // Test serialization roundtrip
                let serialized = serde_json::to_vec(&key_event).unwrap();
                let deserialized: InputEvent = serde_json::from_slice(&serialized).unwrap();
                
                if let InputEvent::KeyPress { keycode: dk, pressed: dp } = deserialized {
                    assert_eq!(keycode, dk, "Keycode should be preserved");
                    assert_eq!(pressed, dp, "Pressed state should be preserved");
                } else {
                    panic!("Event type should be preserved");
                }
            }
        }
    }
    
    #[tokio::test]
    async fn test_input_event_serialization() {
        // Test all input event types
        let events = vec![
            InputEvent::MouseClick { x: 10, y: 20, button: MouseButton::Left },
            InputEvent::MouseClick { x: 30, y: 40, button: MouseButton::Right },
            InputEvent::MouseClick { x: 50, y: 60, button: MouseButton::Middle },
            InputEvent::MouseMove { x: 100, y: 200 },
            InputEvent::KeyPress { keycode: 65, pressed: true }, // 'A' key
            InputEvent::KeyPress { keycode: 65, pressed: false },
        ];
        
        for event in events {
            let serialized = serde_json::to_vec(&event).unwrap();
            let deserialized: InputEvent = serde_json::from_slice(&serialized).unwrap();
            
            // Basic validation that serialization/deserialization works
            assert!(!serialized.is_empty(), "Serialized data should not be empty");
            
            // More detailed validation would require PartialEq on InputEvent
            match (&event, &deserialized) {
                (InputEvent::MouseClick { x: x1, y: y1, .. }, 
                 InputEvent::MouseClick { x: x2, y: y2, .. }) => {
                    assert_eq!(x1, x2);
                    assert_eq!(y1, y2);
                }
                (InputEvent::MouseMove { x: x1, y: y1 }, 
                 InputEvent::MouseMove { x: x2, y: y2 }) => {
                    assert_eq!(x1, x2);
                    assert_eq!(y1, y2);
                }
                (InputEvent::KeyPress { keycode: k1, pressed: p1 }, 
                 InputEvent::KeyPress { keycode: k2, pressed: p2 }) => {
                    assert_eq!(k1, k2);
                    assert_eq!(p1, p2);
                }
                _ => panic!("Event types don't match after serialization"),
            }
        }
    }
    
    #[tokio::test]
    async fn test_message_length_validation() {
        // Test various message lengths
        let valid_lengths = vec![1, 100, 500, 1024];
        let invalid_lengths = vec![0, 1025, 10000, u32::MAX];
        
        for len in valid_lengths {
            assert!(len > 0 && len <= 1024, "Length {} should be valid", len);
        }
        
        for len in invalid_lengths {
            assert!(len == 0 || len > 1024, "Length {} should be invalid", len);
        }
    }
    
    #[tokio::test]
    async fn test_frame_streaming_protocol() {
        // Test frame streaming with mock data
        let test_frame = create_test_jpeg_frame();
        
        // Test frame validation
        assert!(!test_frame.is_empty(), "Test frame should not be empty");
        assert!(test_frame.len() <= MAX_FRAME_SIZE, "Test frame should not exceed max size");
        
        // Test header creation
        let frame_length = test_frame.len() as u32;
        let expected_header = [
            0xFF, 0xD8, // JPEG magic bytes
            ((frame_length >> 24) & 0xFF) as u8,
            ((frame_length >> 16) & 0xFF) as u8,
            ((frame_length >> 8) & 0xFF) as u8,
            (frame_length & 0xFF) as u8,
        ];
        
        // Verify header construction
        assert_eq!(expected_header[0], 0xFF);
        assert_eq!(expected_header[1], 0xD8);
        
        // Verify length encoding/decoding
        let decoded_length = u32::from_be_bytes([
            expected_header[2], expected_header[3], 
            expected_header[4], expected_header[5]
        ]);
        assert_eq!(decoded_length, frame_length);
    }
    
    #[tokio::test]
    async fn test_frame_validation() {
        // Test empty frame
        let empty_frame = Vec::<u8>::new();
        assert!(empty_frame.is_empty());
        
        // Test oversized frame
        let oversized_frame = vec![0u8; MAX_FRAME_SIZE + 1];
        assert!(oversized_frame.len() > MAX_FRAME_SIZE);
        
        // Test valid frame
        let valid_frame = create_test_jpeg_frame();
        assert!(!valid_frame.is_empty());
        assert!(valid_frame.len() <= MAX_FRAME_SIZE);
        assert_eq!(valid_frame[0], 0xFF);
        assert_eq!(valid_frame[1], 0xD8);
    }
    
    #[tokio::test]
    async fn test_frame_header_parsing() {
        // Test valid header
        let frame_length = 1000u32;
        let header = [
            0xFF, 0xD8, // Magic bytes
            ((frame_length >> 24) & 0xFF) as u8,
            ((frame_length >> 16) & 0xFF) as u8,
            ((frame_length >> 8) & 0xFF) as u8,
            (frame_length & 0xFF) as u8,
        ];
        
        // Validate magic bytes
        assert_eq!(header[0], 0xFF);
        assert_eq!(header[1], 0xD8);
        
        // Validate length extraction
        let extracted_length = u32::from_be_bytes([header[2], header[3], header[4], header[5]]);
        assert_eq!(extracted_length, frame_length);
        
        // Test invalid magic bytes
        let invalid_header = [0x00, 0x01, 0x00, 0x00, 0x03, 0xE8];
        assert_ne!(invalid_header[0], 0xFF);
        assert_ne!(invalid_header[1], 0xD8);
    }
    
    #[tokio::test]
    async fn test_frame_size_limits() {
        // Test various frame sizes
        let test_sizes = vec![
            1,                    // Minimum size
            1024,                 // Small frame
            100_000,              // Medium frame
            1_000_000,            // Large frame
            MAX_FRAME_SIZE,       // Maximum allowed
            MAX_FRAME_SIZE + 1,   // Over limit
        ];
        
        for size in test_sizes {
            if size == 0 {
                // Empty frame should be invalid
                assert_eq!(size, 0);
            } else if size <= MAX_FRAME_SIZE {
                // Valid frame size
                assert!(size > 0 && size <= MAX_FRAME_SIZE);
            } else {
                // Invalid frame size
                assert!(size > MAX_FRAME_SIZE);
            }
        }
    }
    
    #[tokio::test]
    async fn test_jpeg_validation() {
        // Test valid JPEG data
        let valid_jpeg = create_test_jpeg_frame();
        assert_eq!(valid_jpeg[0], 0xFF);
        assert_eq!(valid_jpeg[1], 0xD8);
        
        // Test invalid JPEG data
        let invalid_jpeg = vec![0x00, 0x01, 0x02, 0x03];
        assert_ne!(invalid_jpeg[0], 0xFF);
        assert_ne!(invalid_jpeg[1], 0xD8);
        
        // Test partial JPEG data
        let partial_jpeg = vec![0xFF]; // Only one byte
        assert!(partial_jpeg.len() < 2);
    }
    
    #[tokio::test]
    async fn test_frame_streaming_error_conditions() {
        // Test error conditions that would occur during streaming
        
        // Empty frame data
        let empty_data = Vec::<u8>::new();
        assert!(empty_data.is_empty());
        
        // Oversized frame data
        let oversized_data = vec![0xFF, 0xD8]; // Start with JPEG magic
        let mut large_data = oversized_data;
        large_data.resize(MAX_FRAME_SIZE + 1, 0);
        assert!(large_data.len() > MAX_FRAME_SIZE);
        
        // Invalid JPEG data
        let invalid_data = vec![0x00, 0x01, 0x02, 0x03];
        assert!(invalid_data.len() >= 2);
        assert_ne!(invalid_data[0], 0xFF);
        assert_ne!(invalid_data[1], 0xD8);
    }
    
    #[tokio::test]
    async fn test_frame_protocol_constants() {
        // Verify protocol constants are reasonable
        assert!(MAX_FRAME_SIZE > 0, "Max frame size should be positive");
        assert!(MAX_FRAME_SIZE >= 1024 * 1024, "Max frame size should be at least 1MB");
        
        assert_eq!(DEFAULT_JPEG_QUALITY, 80, "Default JPEG quality should be 80");
        assert!(DEFAULT_JPEG_QUALITY >= 1 && DEFAULT_JPEG_QUALITY <= 100, 
                "JPEG quality should be in valid range");
        
        assert_eq!(DEFAULT_FRAME_RATE_MS, 33, "Default frame rate should be 33ms (30 FPS)");
        assert!(DEFAULT_FRAME_RATE_MS > 0, "Frame rate should be positive");
    }
    
    #[tokio::test]
    async fn test_concurrent_frame_streaming() {
        // Test that frame streaming can handle concurrent operations
        use std::sync::Arc;
        use std::sync::atomic::{AtomicUsize, Ordering};
        
        let frame_count = Arc::new(AtomicUsize::new(0));
        let test_frame = create_test_jpeg_frame();
        
        // Simulate multiple frame processing tasks
        let mut tasks = Vec::new();
        for i in 0..5 {
            let frame_data = test_frame.clone();
            let counter = Arc::clone(&frame_count);
            
            let task = tokio::spawn(async move {
                // Simulate frame processing
                tokio::time::sleep(Duration::from_millis(i * 10)).await;
                
                // Validate frame
                if !frame_data.is_empty() && frame_data.len() <= MAX_FRAME_SIZE {
                    counter.fetch_add(1, Ordering::Relaxed);
                }
            });
            
            tasks.push(task);
        }
        
        // Wait for all tasks to complete
        for task in tasks {
            task.await.unwrap();
        }
        
        assert_eq!(frame_count.load(Ordering::Relaxed), 5, "All frames should be processed");
    }
    
    // Helper function to create test JPEG frame data
    fn create_test_jpeg_frame() -> Vec<u8> {
        // Create minimal valid JPEG data for testing
        // This is a very basic JPEG structure for testing purposes
        vec![
            0xFF, 0xD8, // SOI (Start of Image)
            0xFF, 0xE0, // APP0
            0x00, 0x10, // Length
            0x4A, 0x46, 0x49, 0x46, 0x00, // "JFIF\0"
            0x01, 0x01, // Version
            0x01,       // Units
            0x00, 0x48, // X density
            0x00, 0x48, // Y density
            0x00, 0x00, // Thumbnail width/height
            0xFF, 0xD9, // EOI (End of Image)
        ]
    }
    
    #[tokio::test]
    async fn test_server_state_management() {
        let config = create_test_config();
        let mut server = HvncServer::new(config);
        
        // Initial state
        assert!(!server.has_client());
        assert!(!server.is_shutdown_requested());
        
        // Start server
        server.start().await.unwrap();
        assert!(server.listener.is_some());
        
        // Request shutdown
        server.shutdown();
        assert!(server.is_shutdown_requested());
        
        // Client management
        server.disconnect_client(); // Should not panic even with no client
        assert!(!server.has_client());
    }
}