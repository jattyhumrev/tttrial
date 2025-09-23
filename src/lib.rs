//! Hidden VNC Library
//! 
//! A custom Hidden Virtual Network Computing (HVNC) solution that allows
//! remote access to specific applications running in isolated desktop environments.

pub mod common;
pub mod host;
pub mod client;
pub mod error;
pub mod monitoring;

pub use error::{HvncError, Result};