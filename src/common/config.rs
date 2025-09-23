//! Configuration structures for Hidden VNC

use crate::common::constants::*;

/// Server configuration
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// Address to bind the server to
    pub bind_address: String,
    /// Target application to launch
    pub target_application: String,
    /// JPEG compression quality (1-100)
    pub jpeg_quality: u8,
    /// Frame rate in milliseconds
    pub frame_rate_ms: u64,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind_address: format!("127.0.0.1:{}", DEFAULT_PORT),
            target_application: String::new(),
            jpeg_quality: DEFAULT_JPEG_QUALITY,
            frame_rate_ms: DEFAULT_FRAME_RATE_MS,
        }
    }
}

/// Client configuration
#[derive(Debug, Clone)]
pub struct ClientConfig {
    /// Server address to connect to
    pub server_address: String,
    /// Window title for the client display
    pub window_title: String,
    /// Whether to auto-resize the window
    pub auto_resize: bool,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            server_address: format!("127.0.0.1:{}", DEFAULT_PORT),
            window_title: "Hidden VNC Client".to_string(),
            auto_resize: true,
        }
    }
}