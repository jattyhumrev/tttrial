//! Network protocol definitions for Hidden VNC

use serde::{Deserialize, Serialize};

/// Input events that can be sent from client to server
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum InputEvent {
    /// Mouse click event
    MouseClick { x: i32, y: i32, button: MouseButton },
    /// Mouse move event
    MouseMove { x: i32, y: i32 },
    /// Keyboard event
    KeyPress { keycode: u32, pressed: bool },
}

/// Mouse button types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

/// Frame data structure for image streaming
#[derive(Debug)]
pub struct Frame {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}

/// Protocol constants
pub mod constants {
    /// Default server port
    pub const DEFAULT_PORT: u16 = 5900;
    /// Maximum frame size (10MB)
    pub const MAX_FRAME_SIZE: usize = 10 * 1024 * 1024;
    /// Default JPEG quality
    pub const DEFAULT_JPEG_QUALITY: u8 = 80;
    /// Default frame rate (30 FPS)
    pub const DEFAULT_FRAME_RATE_MS: u64 = 33;
}