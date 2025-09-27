//! Input event handling for the host

use crate::{HvncError, Result};
use crate::common::{InputEvent, MouseButton};
use crate::host::WindowHandle;
use std::mem;
use winapi::shared::windef::{POINT, RECT};
use winapi::um::winuser::{
    PostMessageA, GetWindowRect, SetThreadDesktop, GetThreadDesktop,
    WM_LBUTTONDOWN, WM_LBUTTONUP, WM_RBUTTONDOWN, WM_RBUTTONUP, 
    WM_MBUTTONDOWN, WM_MBUTTONUP, WM_MOUSEMOVE, WM_KEYDOWN, WM_KEYUP,
    GetCursorPos, SetCursorPos, GetForegroundWindow, SendInput, 
    MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP, MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP,
    MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_MIDDLEUP, MOUSEEVENTF_MOVE, KEYEVENTF_KEYUP, MOUSEEVENTF_ABSOLUTE,
    GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN, OpenDesktopA, CloseDesktop,
    SendMessageA, SetForegroundWindow, AttachThreadInput, GetWindowThreadProcessId
};
use winapi::um::winuser::{INPUT, INPUT_MOUSE, INPUT_KEYBOARD, MOUSEINPUT, KEYBDINPUT};
use winapi::um::winnt::GENERIC_ALL;
use winapi::um::processthreadsapi::GetCurrentThreadId;
use log::{debug, warn, info};
use std::time::{Duration, Instant};

use std::collections::HashMap;
pub struct InputHandler {
    last_input_time: Option<Instant>,
    input_rate_limit_ms: u64,
    coordinate_offset: (i32, i32),
    security_config: SecurityConfig,
    input_statistics: InputStatistics,
    target_desktop: Option<crate::host::DesktopHandle>,
    original_desktop: Option<crate::host::DesktopHandle>,
}

/// Security configuration for input handling
#[derive(Debug, Clone)]
pub struct SecurityConfig {
    /// Maximum input events per second
    pub max_events_per_second: u32,
    /// Maximum coordinate values (absolute)
    pub max_coordinate_value: i32,
    /// Minimum coordinate values (absolute)
    pub min_coordinate_value: i32,
    /// Blocked keycodes (e.g., system keys)
    pub blocked_keycodes: Vec<u32>,
    /// Enable coordinate bounds checking
    pub enable_coordinate_validation: bool,
    /// Enable keycode validation
    pub enable_keycode_validation: bool,
    /// Maximum consecutive identical events
    pub max_consecutive_identical: u32,
}

/// Input statistics for monitoring and security
#[derive(Debug)]
pub struct InputStatistics {
    /// Total events processed
    pub total_events: u64,
    /// Events rejected due to rate limiting
    pub rate_limited_events: u64,
    /// Events rejected due to validation
    pub validation_rejected_events: u64,
    /// Events by type
    pub events_by_type: HashMap<String, u64>,
    /// Last event for duplicate detection
    pub last_event: Option<InputEvent>,
    /// Consecutive identical event count
    pub consecutive_identical_count: u32,
    /// Start time for rate calculation
    pub start_time: Instant,
}

impl Default for InputStatistics {
    fn default() -> Self {
        Self {
            total_events: 0,
            rate_limited_events: 0,
            validation_rejected_events: 0,
            events_by_type: HashMap::new(),
            last_event: None,
            consecutive_identical_count: 0,
            start_time: Instant::now(),
        }
    }
}

impl SecurityConfig {
    /// Create default security configuration
    pub fn default() -> Self {
        Self {
            max_events_per_second: 1000, // Allow up to 1000 events per second
            max_coordinate_value: 10000,
            min_coordinate_value: -10000,
            blocked_keycodes: vec![
                // Block some system keys for security
                91,  // Left Windows key
                92,  // Right Windows key
                93,  // Menu key
                // Add more as needed
            ],
            enable_coordinate_validation: true,
            enable_keycode_validation: true,
            max_consecutive_identical: 100, // Prevent spam attacks
        }
    }
    
    /// Create permissive security configuration for testing
    pub fn permissive() -> Self {
        Self {
            max_events_per_second: 10000,
            max_coordinate_value: 100000,
            min_coordinate_value: -100000,
            blocked_keycodes: vec![],
            enable_coordinate_validation: false,
            enable_keycode_validation: false,
            max_consecutive_identical: 1000,
        }
    }
}

impl InputHandler {
    /// Create a new input handler with default security
    pub fn new() -> Self {
        Self {
            last_input_time: None,
            input_rate_limit_ms: 1, // Minimum 1ms between inputs
            coordinate_offset: (0, 0),
            security_config: SecurityConfig::default(),
            input_statistics: InputStatistics {
                start_time: Instant::now(),
                ..Default::default()
            },
            target_desktop: None,
            original_desktop: None,
        }
    }
    
    /// Set the target desktop for input events (typically the hidden desktop)
    pub fn set_target_desktop(&mut self, desktop: crate::host::DesktopHandle) {
        self.target_desktop = Some(desktop);
        debug!("Input handler target desktop set");
    }
    
    /// Switch to the target desktop context for input operations
    fn switch_to_target_desktop(&self) -> Result<()> {
        if let Some(target_desktop) = self.target_desktop {
            // Get current desktop to store as original
            let original_desktop = unsafe { GetThreadDesktop(winapi::um::processthreadsapi::GetCurrentThreadId()) };
            if original_desktop.is_null() {
                warn!("Failed to get current desktop for input switching");
                return Ok(()); // Don't fail, just continue without switching
            }
            
            // Switch to target desktop
            let success = unsafe { SetThreadDesktop(target_desktop) };
            if success == 0 {
                let error_code = unsafe { winapi::um::errhandlingapi::GetLastError() };
                warn!("Failed to switch to target desktop for input: error {}", error_code);
                return Ok(()); // Don't fail, just continue without switching
            }
            
            debug!("Successfully switched to target desktop for input");
        }
        Ok(())
    }
    
    /// Restore the original desktop context after input operations
    fn restore_original_desktop(&self) -> Result<()> {
        if self.target_desktop.is_some() {
            // Get the original (main) desktop
            let original_desktop = unsafe { GetThreadDesktop(winapi::um::processthreadsapi::GetCurrentThreadId()) };
            if !original_desktop.is_null() {
                // We need to get the main desktop handle - for simplicity, we'll get it fresh
                let main_desktop = unsafe {
                    winapi::um::winuser::OpenDesktopA(
                        std::ffi::CString::new("Default").unwrap().as_ptr(),
                        0,
                        0,
                        GENERIC_ALL,
                    )
                };
                
                if !main_desktop.is_null() {
                    let success = unsafe { SetThreadDesktop(main_desktop) };
                    if success == 0 {
                        let error_code = unsafe { winapi::um::errhandlingapi::GetLastError() };
                        warn!("Failed to restore original desktop after input: error {}", error_code);
                    } else {
                        debug!("Successfully restored original desktop after input");
                    }
                    unsafe { winapi::um::winuser::CloseDesktop(main_desktop) };
                } else {
                    warn!("Failed to open default desktop for restoration");
                }
            }
        }
        Ok(())
    }
    
    /// Focus the target window for input injection (HVNC technique)
    fn focus_target_window(&self, window: WindowHandle) -> Result<()> {
        if window.is_null() || window as usize == 1 {
            return Ok(()); // Skip for desktop capture mode
        }
        
        // Get current thread ID
        let current_thread_id = unsafe { GetCurrentThreadId() };
        
        // Get target window's thread ID
        let mut process_id = 0;
        let target_thread_id = unsafe { GetWindowThreadProcessId(window, &mut process_id) };
        
        if target_thread_id == 0 {
            warn!("Failed to get target window thread ID");
            return Ok(()); // Don't fail, just continue without focus
        }
        
        debug!("🎯 HVNC Input Focus: Current thread: {}, Target thread: {}, Window: {:?}", 
               current_thread_id, target_thread_id, window);
        
        // Attach input to target thread if different
        let mut attached = false;
        if current_thread_id != target_thread_id {
            let attach_result = unsafe { AttachThreadInput(current_thread_id, target_thread_id, 1) };
            if attach_result != 0 {
                attached = true;
                debug!("✅ Successfully attached input to target thread");
            } else {
                let error_code = unsafe { winapi::um::errhandlingapi::GetLastError() };
                warn!("❌ Failed to attach input to target thread: error {}", error_code);
            }
        }
        
        // Set foreground window for proper input focus
        let focus_result = unsafe { SetForegroundWindow(window) };
        if focus_result == 0 {
            let error_code = unsafe { winapi::um::errhandlingapi::GetLastError() };
            warn!("❌ SetForegroundWindow failed for window {:?}: error {}", window, error_code);
        } else {
            debug!("✅ Successfully focused target window {:?}", window);
        }
        
        // Small delay for focus to take effect
        std::thread::sleep(Duration::from_millis(10));
        
        Ok(())
    }
    
    /// Cleanup after input injection
    fn cleanup_input_focus(&self, window: WindowHandle) -> Result<()> {
        if window.is_null() || window as usize == 1 {
            return Ok(()); // Skip for desktop capture mode
        }
        
        // Get current thread ID
        let current_thread_id = unsafe { GetCurrentThreadId() };
        
        // Get target window's thread ID
        let mut process_id = 0;
        let target_thread_id = unsafe { GetWindowThreadProcessId(window, &mut process_id) };
        
        if target_thread_id != 0 && current_thread_id != target_thread_id {
            // Detach input thread
            unsafe { AttachThreadInput(current_thread_id, target_thread_id, 0) };
            debug!("🔄 Input thread detached from target");
        }
        
        Ok(())
    }
    
    /// Create a new input handler with custom rate limiting
    pub fn new_with_rate_limit(rate_limit_ms: u64) -> Self {
        Self {
            last_input_time: None,
            input_rate_limit_ms: rate_limit_ms,
            coordinate_offset: (0, 0),
            security_config: SecurityConfig::default(),
            input_statistics: InputStatistics {
                start_time: Instant::now(),
                ..Default::default()
            },
            target_desktop: None,
            original_desktop: None,
        }
    }
    
    /// Create a new input handler with custom security configuration
    pub fn new_with_security(security_config: SecurityConfig) -> Self {
        Self {
            last_input_time: None,
            input_rate_limit_ms: 1,
            coordinate_offset: (0, 0),
            security_config,
            input_statistics: InputStatistics {
                start_time: Instant::now(),
                ..Default::default()
            },
            target_desktop: None,
            original_desktop: None,
        }
    }
    
    /// Set coordinate offset for window-relative positioning
    pub fn set_coordinate_offset(&mut self, offset_x: i32, offset_y: i32) {
        self.coordinate_offset = (offset_x, offset_y);
        debug!("Coordinate offset set to: ({}, {})", offset_x, offset_y);
    }
    
    /// Check if input should be rate limited
    fn should_rate_limit(&mut self) -> bool {
        // Check basic rate limiting
        let basic_rate_limit = match self.last_input_time {
            None => {
                self.last_input_time = Some(Instant::now());
                false
            }
            Some(last_time) => {
                let elapsed = last_time.elapsed();
                if elapsed < Duration::from_millis(self.input_rate_limit_ms) {
                    true // Rate limit
                } else {
                    self.last_input_time = Some(Instant::now());
                    false
                }
            }
        };
        
        if basic_rate_limit {
            self.input_statistics.rate_limited_events += 1;
            return true;
        }
        
        // Check events per second rate limiting
        let elapsed_seconds = self.input_statistics.start_time.elapsed().as_secs_f64();
        if elapsed_seconds > 0.0 {
            let events_per_second = self.input_statistics.total_events as f64 / elapsed_seconds;
            if events_per_second > self.security_config.max_events_per_second as f64 {
                self.input_statistics.rate_limited_events += 1;
                warn!("Rate limiting: {} events/second exceeds limit of {}", 
                      events_per_second, self.security_config.max_events_per_second);
                return true;
            }
        }
        
        false
    }
    
    /// Validate input event for security
    fn validate_input_event_security(&mut self, event: &InputEvent) -> Result<()> {
        // Check for consecutive identical events (spam protection)
        if let Some(ref last_event) = self.input_statistics.last_event {
            if last_event == event {
                self.input_statistics.consecutive_identical_count += 1;
                if self.input_statistics.consecutive_identical_count > self.security_config.max_consecutive_identical {
                    self.input_statistics.validation_rejected_events += 1;
                    return Err(HvncError::Connection(format!(
                        "Too many consecutive identical events: {}", 
                        self.input_statistics.consecutive_identical_count
                    )));
                }
            } else {
                self.input_statistics.consecutive_identical_count = 1;
            }
        } else {
            self.input_statistics.consecutive_identical_count = 1;
        }
        
        // Validate based on event type
        match event {
            InputEvent::MouseClick { x, y, button: _ } | InputEvent::MouseMove { x, y } => {
                if self.security_config.enable_coordinate_validation {
                    if *x < self.security_config.min_coordinate_value || 
                       *x > self.security_config.max_coordinate_value ||
                       *y < self.security_config.min_coordinate_value || 
                       *y > self.security_config.max_coordinate_value {
                        self.input_statistics.validation_rejected_events += 1;
                        return Err(HvncError::Connection(format!(
                            "Coordinates ({}, {}) outside allowed range ({} to {})", 
                            x, y, self.security_config.min_coordinate_value, 
                            self.security_config.max_coordinate_value
                        )));
                    }
                }
            }
            InputEvent::KeyPress { keycode, pressed: _ } => {
                if self.security_config.enable_keycode_validation {
                    // Check if keycode is blocked
                    if self.security_config.blocked_keycodes.contains(keycode) {
                        self.input_statistics.validation_rejected_events += 1;
                        return Err(HvncError::Connection(format!(
                            "Keycode {} is blocked for security reasons", keycode
                        )));
                    }
                    
                    // Validate keycode range
                    if *keycode > 255 {
                        self.input_statistics.validation_rejected_events += 1;
                        return Err(HvncError::Connection(format!(
                            "Invalid keycode: {}", keycode
                        )));
                    }
                }
            }
        }
        
        Ok(())
    }
    
    /// Update input statistics
    fn update_statistics(&mut self, event: &InputEvent) {
        self.input_statistics.total_events += 1;
        
        // Update event type statistics
        let event_type = match event {
            InputEvent::MouseClick { .. } => "mouse_click",
            InputEvent::MouseMove { .. } => "mouse_move",
            InputEvent::KeyPress { .. } => "key_press",
        };
        
        *self.input_statistics.events_by_type.entry(event_type.to_string()).or_insert(0) += 1;
        
        // Store last event for duplicate detection
        self.input_statistics.last_event = Some(event.clone());
    }
    
    /// Handle a mouse click event
    pub fn handle_mouse_click(
        &mut self,
        window: WindowHandle,
        x: i32,
        y: i32,
        button: MouseButton,
        pressed: bool,
    ) -> Result<()> {
        if self.should_rate_limit() {
            debug!("Rate limiting mouse click event");
            return Ok(());
        }
        
        debug!("Handling mouse click: ({}, {}) button: {:?} pressed: {}", x, y, button, pressed);
        
        // Switch to hidden desktop context for input
        self.switch_to_target_desktop()?;
        
        // 🎯 HVNC Input Focus: Focus the target window first
        self.focus_target_window(window)?;
        
        // Validate window handle (except for desktop capture mode)
        if window.is_null() {
            self.cleanup_input_focus(window)?;
            self.restore_original_desktop()?;
            return Err(HvncError::window_not_found("Window handle is null"));
        }
        
        // Apply coordinate offset
        let adjusted_x = x + self.coordinate_offset.0;
        let adjusted_y = y + self.coordinate_offset.1;
        
        // Validate coordinates
        if let Err(e) = self.validate_coordinates(window, adjusted_x, adjusted_y) {
            self.cleanup_input_focus(window)?;
            self.restore_original_desktop()?;
            return Err(e);
        }
        
        // For desktop capture mode (window handle is 1), use SendInput API for better compatibility
        if window as usize == 1 {
            debug!("Using SendInput API for desktop capture mode");
            let result = self.handle_desktop_mouse_click(adjusted_x, adjusted_y, button, pressed);
            self.cleanup_input_focus(window)?;
            self.restore_original_desktop()?;
            return result;
        }
        
        // Determine the appropriate Windows message
        let message = match (&button, pressed) {
            (MouseButton::Left, true) => WM_LBUTTONDOWN,
            (MouseButton::Left, false) => WM_LBUTTONUP,
            (MouseButton::Right, true) => WM_RBUTTONDOWN,
            (MouseButton::Right, false) => WM_RBUTTONUP,
            (MouseButton::Middle, true) => WM_MBUTTONDOWN,
            (MouseButton::Middle, false) => WM_MBUTTONUP,
        };
        
        // Create lParam with coordinates
        let lparam = ((adjusted_y as u32) << 16) | (adjusted_x as u32 & 0xFFFF);
        
        info!("🖱️ HVNC Mouse Input: Sending {:?} {} to window {:?} at ({}, {})", 
              &button, if pressed { "DOWN" } else { "UP" }, window, adjusted_x, adjusted_y);
        
        // 🚀 HVNC Technique: Use SendMessage instead of PostMessage for synchronous delivery
        let result = unsafe {
            SendMessageA(window, message, 0, lparam as isize)
        };
        
        // Cleanup input focus
        self.cleanup_input_focus(window)?;
        
        // Restore original desktop context
        self.restore_original_desktop()?;
        
        // SendMessage returns the result directly (not just success/failure)
        debug!("✅ HVNC Mouse input sent successfully: result = {}", result);
        Ok(())
    }
    
    /// Handle a mouse move event
    pub fn handle_mouse_move(
        &mut self,
        window: WindowHandle,
        x: i32,
        y: i32,
    ) -> Result<()> {
        if self.should_rate_limit() {
            debug!("Rate limiting mouse move event");
            return Ok(());
        }
        
        debug!("Handling mouse move: ({}, {})", x, y);
        
        // Switch to hidden desktop context for input
        self.switch_to_target_desktop()?;
        
        // Validate window handle (except for desktop capture mode)
        if window.is_null() {
            self.restore_original_desktop()?;
            return Err(HvncError::window_not_found("Window handle is null"));
        }
        
        // Apply coordinate offset
        let adjusted_x = x + self.coordinate_offset.0;
        let adjusted_y = y + self.coordinate_offset.1;
        
        // Validate coordinates
        if let Err(e) = self.validate_coordinates(window, adjusted_x, adjusted_y) {
            self.restore_original_desktop()?;
            return Err(e);
        }
        
        // For desktop capture mode (window handle is 1), use SendInput API for better compatibility
        if window as usize == 1 {
            debug!("Using SendInput API for desktop capture mouse move");
            let result = self.handle_desktop_mouse_move(adjusted_x, adjusted_y);
            self.restore_original_desktop()?;
            return result;
        }
        
        // Create lParam with coordinates
        let lparam = ((adjusted_y as u32) << 16) | (adjusted_x as u32 & 0xFFFF);
        
        // Send mouse move message
        let result = unsafe {
            PostMessageA(window, WM_MOUSEMOVE, 0, lparam as isize)
        };
        
        // Restore original desktop context
        self.restore_original_desktop()?;
        
        if result == 0 {
            let error_code = unsafe { winapi::um::errhandlingapi::GetLastError() };
            return Err(HvncError::windows_api_with_code(
                "Failed to post mouse move message", error_code
            ));
        }
        
        debug!("Mouse move event sent successfully");
        Ok(())
    }
    
    /// Handle a keyboard event using SendInput API
    pub fn handle_keyboard_event(
        &mut self,
        _window: WindowHandle, // Not needed for SendInput
        keycode: u32,
        pressed: bool,
    ) -> Result<()> {
        if self.should_rate_limit() {
            debug!("Rate limiting keyboard event");
            return Ok(());
        }
        
        debug!("Handling keyboard event: keycode {} pressed: {}", keycode, pressed);
        
        // 🔍 HVNC Enhancement: For target desktop context, find the focused window
        let target_window = if self.target_desktop.is_some() {
            // Get the currently focused window in the target desktop
            unsafe { GetForegroundWindow() }
        } else {
            std::ptr::null_mut()
        };
        
        // Switch to hidden desktop context for input
        self.switch_to_target_desktop()?;
        
        // Focus the target window if we have one
        if !target_window.is_null() {
            self.focus_target_window(target_window)?;
        }
        
        // Validate keycode (Windows virtual key codes are 0-255)
        if keycode > 255 {
            if !target_window.is_null() {
                self.cleanup_input_focus(target_window)?;
            }
            self.restore_original_desktop()?;
            return Err(HvncError::Connection(
                format!("Invalid keycode: {}", keycode)
            ));
        }
        
        info!("⌨️ HVNC Keyboard Input: Sending keycode {} {} to window {:?}", 
              keycode, if pressed { "DOWN" } else { "UP" }, target_window);
        
        // 🚀 HVNC Technique: Use both SendInput (global) and SendMessage (window-specific)
        if !target_window.is_null() {
            // Send to specific window using SendMessage
            let message = if pressed { WM_KEYDOWN } else { WM_KEYUP };
            let result = unsafe {
                SendMessageA(target_window, message, keycode as usize, 0)
            };
            debug!("✅ HVNC Keyboard message sent to window: result = {}", result);
        }
        
        // Also use SendInput for global keyboard state
        // Create INPUT structure for keyboard event
        let mut input: INPUT = unsafe { mem::zeroed() };
        input.type_ = INPUT_KEYBOARD;
        
        unsafe {
            let keyboard_input = input.u.ki_mut();
            keyboard_input.wVk = keycode as u16;
            keyboard_input.wScan = 0;
            keyboard_input.dwFlags = if pressed { 0 } else { KEYEVENTF_KEYUP };
            keyboard_input.time = 0;
            keyboard_input.dwExtraInfo = 0;
        }
        
        // Send the input
        let result = unsafe { SendInput(1, &mut input, mem::size_of::<INPUT>() as i32) };
        
        // Cleanup input focus
        if !target_window.is_null() {
            self.cleanup_input_focus(target_window)?;
        }
        
        // Restore original desktop context
        self.restore_original_desktop()?;
        
        if result == 0 {
            let error_code = unsafe { winapi::um::errhandlingapi::GetLastError() };
            return Err(HvncError::windows_api_with_code(
                "Failed to send keyboard input", error_code
            ));
        }
        
        debug!("✅ HVNC Keyboard event sent successfully using hybrid approach");
        Ok(())
    }
    
    /// Process a generic input event with security validation
    pub fn process_input_event(
        &mut self,
        window: WindowHandle,
        event: InputEvent,
    ) -> Result<()> {
        debug!("Processing input event: {:?}", event);

        // Validate window handle
        if window.is_null() {
            log::error!("Input event failed: null window handle");
            return Err(HvncError::WindowNotFound { title: "null".to_string() });
        }

        // Check rate limiting first
        if self.should_rate_limit() {
            debug!("Input event rate limited");
            return Ok(()); // Silently drop rate-limited events
        }

        // Validate input event for security
        self.validate_input_event_security(&event)?;

        // Additional fail-proof validation
        match &event {
            InputEvent::MouseClick { x, y, .. } | InputEvent::MouseMove { x, y } => {
                if *x < 0 || *y < 0 || *x > MAX_SCREEN_WIDTH || *y > MAX_SCREEN_HEIGHT {
                    log::warn!("Input event out of bounds: x={}, y={}", x, y);
                    return Err(HvncError::InputCoordinateOutOfBounds { x: *x, y: *y });
                }
            }
            InputEvent::KeyPress { keycode, .. } => {
                if *keycode > 255 {
                    log::warn!("Invalid keycode: {}", keycode);
                    return Err(HvncError::InvalidKeycode { keycode: *keycode });
                }
            }
        }

        // Update statistics
        self.update_statistics(&event);

        // Process the event with error recovery
        let result = match event {
            InputEvent::MouseClick { x, y, button } => {
                self.handle_mouse_click(window, x, y, button.clone(), true)
                    .and_then(|_| {
                        std::thread::sleep(Duration::from_millis(10));
                        self.handle_mouse_click(window, x, y, button, false)
                    })
            }
            InputEvent::MouseMove { x, y } => {
                self.handle_mouse_move(window, x, y)
            }
            InputEvent::KeyPress { keycode, pressed } => {
                self.handle_keyboard_event(window, keycode, pressed)
            }
        };
        if let Err(e) = &result {
            log::error!("Input event processing failed: {:?}", e);
        }
        result
    }
    
    /// Process input event without security validation (for testing)
    pub fn process_input_event_unsafe(
        &mut self,
        window: WindowHandle,
        event: InputEvent,
    ) -> Result<()> {
        debug!("Processing input event (unsafe): {:?}", event);
        
        match event {
            InputEvent::MouseClick { x, y, button } => {
                self.handle_mouse_click(window, x, y, button.clone(), true)?;
                std::thread::sleep(Duration::from_millis(10));
                self.handle_mouse_click(window, x, y, button, false)?;
            }
            InputEvent::MouseMove { x, y } => {
                self.handle_mouse_move(window, x, y)?;
            }
            InputEvent::KeyPress { keycode, pressed } => {
                self.handle_keyboard_event(window, keycode, pressed)?;
            }
        }
        
        Ok(())
    }
    
    /// Validate that coordinates are within window bounds
    fn validate_coordinates(&self, window: WindowHandle, x: i32, y: i32) -> Result<()> {
        // Special handling for desktop capture mode (window handle is 1)
        if window as usize == 1 {
            // For desktop capture, use more permissive validation
            // Allow coordinates within typical screen bounds
            if x >= -1000 && x <= 5000 && y >= -1000 && y <= 5000 {
                return Ok(());
            } else {
                return Err(HvncError::Connection(format!(
                    "Desktop capture coordinates ({}, {}) outside reasonable bounds", 
                    x, y
                )));
            }
        }
        
        let mut window_rect: RECT = unsafe { mem::zeroed() };
        let success = unsafe { GetWindowRect(window, &mut window_rect) };
        
        if success == 0 {
            // If we can't get window rect, allow the coordinates (window might be special)
            warn!("Could not get window rectangle for coordinate validation");
            return Ok(());
        }
        
        let window_width = window_rect.right - window_rect.left;
        let window_height = window_rect.bottom - window_rect.top;
        
        // Allow some tolerance for coordinates slightly outside window bounds
        let tolerance = 10;
        
        if x < -tolerance || x > window_width + tolerance || 
           y < -tolerance || y > window_height + tolerance {
            return Err(HvncError::Connection(format!(
                "Coordinates ({}, {}) outside window bounds ({}x{})", 
                x, y, window_width, window_height
            )));
        }
        
        Ok(())
    }
    
    /// Get current mouse position (for debugging/testing)
    pub fn get_cursor_position() -> Result<(i32, i32)> {
        let mut point: POINT = unsafe { mem::zeroed() };
        let success = unsafe { GetCursorPos(&mut point) };
        
        if success == 0 {
            let error_code = unsafe { winapi::um::errhandlingapi::GetLastError() };
            return Err(HvncError::windows_api_with_code(
                "Failed to get cursor position", error_code
            ));
        }
        
        Ok((point.x, point.y))
    }
    
    /// Set cursor position (for testing)
    pub fn set_cursor_position(x: i32, y: i32) -> Result<()> {
        let success = unsafe { SetCursorPos(x, y) };
        
        if success == 0 {
            let error_code = unsafe { winapi::um::errhandlingapi::GetLastError() };
            return Err(HvncError::windows_api_with_code(
                "Failed to set cursor position", error_code
            ));
        }
        
        Ok(())
    }
    
    /// Get the currently focused window
    pub fn get_foreground_window() -> WindowHandle {
        unsafe { GetForegroundWindow() }
    }
    
    /// Reset rate limiting state
    pub fn reset_rate_limiting(&mut self) {
        self.last_input_time = None;
        debug!("Rate limiting state reset");
    }
    
    /// Get current rate limit setting
    pub fn get_rate_limit_ms(&self) -> u64 {
        self.input_rate_limit_ms
    }
    
    /// Set rate limit
    pub fn set_rate_limit_ms(&mut self, rate_limit_ms: u64) {
        self.input_rate_limit_ms = rate_limit_ms;
        debug!("Rate limit set to: {}ms", rate_limit_ms);
    }
    
    /// Get security configuration
    pub fn get_security_config(&self) -> &SecurityConfig {
        &self.security_config
    }
    
    /// Update security configuration
    pub fn set_security_config(&mut self, config: SecurityConfig) {
        self.security_config = config;
        info!("Security configuration updated");
    }
    
    /// Get input statistics
    pub fn get_statistics(&self) -> &InputStatistics {
        &self.input_statistics
    }
    
    /// Reset input statistics
    pub fn reset_statistics(&mut self) {
        self.input_statistics = InputStatistics {
            start_time: Instant::now(),
            ..Default::default()
        };
        info!("Input statistics reset");
    }
    
    /// Get events per second rate
    pub fn get_events_per_second(&self) -> f64 {
        let elapsed_seconds = self.input_statistics.start_time.elapsed().as_secs_f64();
        if elapsed_seconds > 0.0 {
            self.input_statistics.total_events as f64 / elapsed_seconds
        } else {
            0.0
        }
    }
    
    /// Check if input handler is under attack (high rate of rejected events)
    pub fn is_under_attack(&self) -> bool {
        let total_events = self.input_statistics.total_events;
        if total_events < 100 {
            return false; // Not enough data
        }
        
        let rejection_rate = (self.input_statistics.validation_rejected_events + 
                             self.input_statistics.rate_limited_events) as f64 / total_events as f64;
        
        rejection_rate > 0.5 // More than 50% rejection rate indicates potential attack
    }
    
    /// Add a keycode to the blocked list
    pub fn block_keycode(&mut self, keycode: u32) {
        if !self.security_config.blocked_keycodes.contains(&keycode) {
            self.security_config.blocked_keycodes.push(keycode);
            info!("Keycode {} added to blocked list", keycode);
        }
    }
    
    /// Remove a keycode from the blocked list
    pub fn unblock_keycode(&mut self, keycode: u32) {
        self.security_config.blocked_keycodes.retain(|&k| k != keycode);
        info!("Keycode {} removed from blocked list", keycode);
    }
    
    /// Check if a keycode is blocked
    pub fn is_keycode_blocked(&self, keycode: u32) -> bool {
        self.security_config.blocked_keycodes.contains(&keycode)
    }
    
    /// Handle mouse click for desktop capture mode using SendInput API
    fn handle_desktop_mouse_click(&self, x: i32, y: i32, button: MouseButton, pressed: bool) -> Result<()> {
        debug!("Desktop mouse click: ({}, {}) button: {:?} pressed: {}", x, y, button, pressed);
        
        // Get screen dimensions for coordinate conversion
        let screen_width = unsafe { winapi::um::winuser::GetSystemMetrics(winapi::um::winuser::SM_CXSCREEN) };
        let screen_height = unsafe { winapi::um::winuser::GetSystemMetrics(winapi::um::winuser::SM_CYSCREEN) };
        
        if screen_width <= 0 || screen_height <= 0 {
            warn!("Invalid screen dimensions: {}x{}", screen_width, screen_height);
            return Ok(()); // Don't fail, just skip the input
        }
        
        // Convert coordinates to absolute coordinates (0-65535 range for SendInput)
        let absolute_x = ((x * 65535) / screen_width) as u32;
        let absolute_y = ((y * 65535) / screen_height) as u32;
        
        // Determine mouse event flags
        let event_flags = match (button, pressed) {
            (MouseButton::Left, true) => MOUSEEVENTF_LEFTDOWN | MOUSEEVENTF_ABSOLUTE,
            (MouseButton::Left, false) => MOUSEEVENTF_LEFTUP | MOUSEEVENTF_ABSOLUTE,
            (MouseButton::Right, true) => MOUSEEVENTF_RIGHTDOWN | MOUSEEVENTF_ABSOLUTE,
            (MouseButton::Right, false) => MOUSEEVENTF_RIGHTUP | MOUSEEVENTF_ABSOLUTE,
            (MouseButton::Middle, true) => MOUSEEVENTF_MIDDLEDOWN | MOUSEEVENTF_ABSOLUTE,
            (MouseButton::Middle, false) => MOUSEEVENTF_MIDDLEUP | MOUSEEVENTF_ABSOLUTE,
        };
        
        // Create INPUT structure for mouse event
        let mut input = INPUT {
            type_: INPUT_MOUSE,
            u: unsafe { mem::zeroed() },
        };
        
        unsafe {
            let mouse_input = input.u.mi_mut();
            mouse_input.dx = absolute_x as i32;
            mouse_input.dy = absolute_y as i32;
            mouse_input.dwFlags = event_flags;
            mouse_input.mouseData = 0;
            mouse_input.dwExtraInfo = 0;
            mouse_input.time = 0;
        }
        
        // Send the input
        let result = unsafe { SendInput(1, &mut input, mem::size_of::<INPUT>() as i32) };
        
        if result == 0 {
            let error_code = unsafe { winapi::um::errhandlingapi::GetLastError() };
            return Err(HvncError::windows_api_with_code(
                "Failed to send desktop mouse input", error_code
            ));
        }
        
        debug!("Desktop mouse click sent successfully");
        Ok(())
    }
    
    /// Handle mouse move for desktop capture mode using SendInput API
    fn handle_desktop_mouse_move(&self, x: i32, y: i32) -> Result<()> {
        debug!("Desktop mouse move: ({}, {})", x, y);
        
        // Get screen dimensions for coordinate conversion
        let screen_width = unsafe { winapi::um::winuser::GetSystemMetrics(winapi::um::winuser::SM_CXSCREEN) };
        let screen_height = unsafe { winapi::um::winuser::GetSystemMetrics(winapi::um::winuser::SM_CYSCREEN) };
        
        if screen_width <= 0 || screen_height <= 0 {
            warn!("Invalid screen dimensions: {}x{}", screen_width, screen_height);
            return Ok(()); // Don't fail, just skip the input
        }
        
        // Convert coordinates to absolute coordinates (0-65535 range for SendInput)
        let absolute_x = ((x * 65535) / screen_width) as u32;
        let absolute_y = ((y * 65535) / screen_height) as u32;
        
        // Create INPUT structure for mouse move
        let mut input = INPUT {
            type_: INPUT_MOUSE,
            u: unsafe { mem::zeroed() },
        };
        
        unsafe {
            let mouse_input = input.u.mi_mut();
            mouse_input.dx = absolute_x as i32;
            mouse_input.dy = absolute_y as i32;
            mouse_input.dwFlags = MOUSEEVENTF_MOVE | MOUSEEVENTF_ABSOLUTE;
            mouse_input.mouseData = 0;
            mouse_input.dwExtraInfo = 0;
            mouse_input.time = 0;
        }
        
        // Send the input
        let result = unsafe { SendInput(1, &mut input, mem::size_of::<INPUT>() as i32) };
        
        if result == 0 {
            let error_code = unsafe { winapi::um::errhandlingapi::GetLastError() };
            return Err(HvncError::windows_api_with_code(
                "Failed to send desktop mouse move", error_code
            ));
        }
        
        debug!("Desktop mouse move sent successfully");
        Ok(())
    }
}

impl Default for InputHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::host::desktop::create_hidden_desktop;
    use crate::host::launcher::{launch_app_in_desktop, find_window};
    use std::time::Duration;
    use std::ptr;
    
    #[test]
    fn test_input_handler_creation() {
        let handler = InputHandler::new();
        assert_eq!(handler.input_rate_limit_ms, 1);
        assert_eq!(handler.coordinate_offset, (0, 0));
        assert!(handler.last_input_time.is_none());
        
        let handler_custom = InputHandler::new_with_rate_limit(50);
        assert_eq!(handler_custom.input_rate_limit_ms, 50);
    }
    
    #[test]
    fn test_coordinate_offset() {
        let mut handler = InputHandler::new();
        
        // Test initial offset
        assert_eq!(handler.coordinate_offset, (0, 0));
        
        // Test setting offset
        handler.set_coordinate_offset(100, 200);
        assert_eq!(handler.coordinate_offset, (100, 200));
        
        handler.set_coordinate_offset(-50, -75);
        assert_eq!(handler.coordinate_offset, (-50, -75));
    }
    
    #[test]
    fn test_rate_limiting() {
        let mut handler = InputHandler::new_with_rate_limit(100); // 100ms rate limit
        
        // First call should not be rate limited
        assert!(!handler.should_rate_limit());
        
        // Immediate second call should be rate limited
        assert!(handler.should_rate_limit());
        
        // Reset and test again
        handler.reset_rate_limiting();
        assert!(!handler.should_rate_limit());
    }
    
    #[test]
    fn test_rate_limit_configuration() {
        let mut handler = InputHandler::new();
        
        // Test default rate limit
        assert_eq!(handler.get_rate_limit_ms(), 1);
        
        // Test setting rate limit
        handler.set_rate_limit_ms(25);
        assert_eq!(handler.get_rate_limit_ms(), 25);
        
        handler.set_rate_limit_ms(0);
        assert_eq!(handler.get_rate_limit_ms(), 0);
    }
    
    #[test]
    fn test_input_handler_null_window() {
        let mut handler = InputHandler::new();
        let null_window = ptr::null_mut();
        
        // Test mouse click with null window
        let result = handler.handle_mouse_click(null_window, 100, 200, MouseButton::Left, true);
        assert!(result.is_err());
        if let Err(HvncError::WindowNotFound { title }) = result {
            assert!(title.contains("null"));
        } else {
            panic!("Expected WindowNotFound error");
        }
        
        // Test mouse move with null window
        let result = handler.handle_mouse_move(null_window, 100, 200);
        assert!(result.is_err());
        
        // Test keyboard event with null window
        let result = handler.handle_keyboard_event(null_window, 65, true);
        // Note: keyboard events don't validate window handle in the same way
        // but should still fail with Windows API error
    }
    
    #[test]
    fn test_keyboard_keycode_validation() {
        let mut handler = InputHandler::new();
        let test_window = 0x12345678 as WindowHandle; // Mock window handle
        
        // Test valid keycodes
        let valid_keycodes = vec![0, 1, 65, 127, 255]; // A-Z, etc.
        
        for keycode in valid_keycodes {
            // Note: This will fail with Windows API error since it's a mock handle,
            // but it should pass the keycode validation
            let result = handler.handle_keyboard_event(test_window, keycode, true);
            
            // Should fail with Windows API error, not keycode validation error
            if let Err(HvncError::WindowsApi { .. }) = result {
                // Expected - mock window handle causes Windows API failure
            } else if let Err(HvncError::Connection(msg)) = result {
                if msg.contains("Invalid keycode") {
                    panic!("Valid keycode {} should not fail validation", keycode);
                }
            }
        }
        
        // Test invalid keycodes
        let invalid_keycodes = vec![256, 300, 1000, u32::MAX];
        
        for keycode in invalid_keycodes {
            let result = handler.handle_keyboard_event(test_window, keycode, true);
            assert!(result.is_err());
            
            if let Err(HvncError::Connection(msg)) = result {
                assert!(msg.contains("Invalid keycode"));
            } else {
                panic!("Expected Connection error for invalid keycode {}", keycode);
            }
        }
    }
    
    #[test]
    fn test_process_input_event_types() {
        let mut handler = InputHandler::new();
        let test_window = 0x12345678 as WindowHandle; // Mock window handle
        
        // Test different input event types
        let events = vec![
            InputEvent::MouseClick { x: 100, y: 200, button: MouseButton::Left },
            InputEvent::MouseMove { x: 150, y: 250 },
            InputEvent::KeyPress { keycode: 65, pressed: true },
            InputEvent::KeyPress { keycode: 65, pressed: false },
        ];
        
        for event in events {
            let result = handler.process_input_event(test_window, event);
            
            // Should fail with Windows API error due to mock handle, but not with validation errors
            if let Err(HvncError::WindowsApi { .. }) = result {
                // Expected for mock window handle
            } else if let Err(HvncError::WindowNotFound { .. }) = result {
                // Also expected for mock window handle
            } else if result.is_ok() {
                // Unexpected success with mock handle, but not necessarily wrong
                // (might happen in some test environments)
            } else {
                panic!("Unexpected error type: {:?}", result);
            }
        }
    }
    
    #[test]
    fn test_mouse_button_message_mapping() {
        // Test that mouse button events map to correct Windows messages
        // This is a logic test, not a Windows API test
        
        let button_mappings = vec![
            (MouseButton::Left, true, WM_LBUTTONDOWN),
            (MouseButton::Left, false, WM_LBUTTONUP),
            (MouseButton::Right, true, WM_RBUTTONDOWN),
            (MouseButton::Right, false, WM_RBUTTONUP),
            (MouseButton::Middle, true, WM_MBUTTONDOWN),
            (MouseButton::Middle, false, WM_MBUTTONUP),
        ];
        
        for (button, pressed, expected_message) in button_mappings {
            // Test the mapping logic (this is what would be used in handle_mouse_click)
            let actual_message = match (&button, pressed) {
                (MouseButton::Left, true) => WM_LBUTTONDOWN,
                (MouseButton::Left, false) => WM_LBUTTONUP,
                (MouseButton::Right, true) => WM_RBUTTONDOWN,
                (MouseButton::Right, false) => WM_RBUTTONUP,
                (MouseButton::Middle, true) => WM_MBUTTONDOWN,
                (MouseButton::Middle, false) => WM_MBUTTONUP,
            };
            
            assert_eq!(actual_message, expected_message, 
                      "Button {:?} pressed {} should map to message {}", 
                      button, pressed, expected_message);
        }
    }
    
    #[test]
    fn test_coordinate_validation_logic() {
        let handler = InputHandler::new();
        
        // Test coordinate bounds checking logic
        // Note: This tests the validation logic, not the Windows API calls
        
        let test_cases = vec![
            // (x, y, window_width, window_height, should_be_valid)
            (0, 0, 800, 600, true),           // Top-left corner
            (799, 599, 800, 600, true),      // Bottom-right corner (within bounds)
            (400, 300, 800, 600, true),      // Center
            (-5, 0, 800, 600, true),         // Slightly outside (within tolerance)
            (805, 600, 800, 600, true),     // Slightly outside (within tolerance)
            (-20, 0, 800, 600, false),      // Too far outside
            (820, 0, 800, 600, false),      // Too far outside
            (0, -20, 800, 600, false),      // Too far outside
            (0, 620, 800, 600, false),      // Too far outside
        ];
        
        for (x, y, width, height, should_be_valid) in test_cases {
            let tolerance = 10;
            let is_valid = x >= -tolerance && x <= width + tolerance && 
                          y >= -tolerance && y <= height + tolerance;
            
            assert_eq!(is_valid, should_be_valid, 
                      "Coordinate ({}, {}) in {}x{} window should be {}", 
                      x, y, width, height, if should_be_valid { "valid" } else { "invalid" });
        }
    }
    
    #[test]
    fn test_lparam_creation() {
        // Test MAKELPARAM macro usage for coordinate encoding
        let test_coordinates = vec![
            (0, 0),
            (100, 200),
            (1920, 1080),
            (32767, 32767), // Max positive values for 16-bit
        ];
        
        for (x, y) in test_coordinates {
            let lparam = ((y as u32) << 16) | (x as u32 & 0xFFFF);
            
            // Extract coordinates back from lparam
            let extracted_x = (lparam & 0xFFFF) as i16 as i32;
            let extracted_y = ((lparam >> 16) & 0xFFFF) as i16 as i32;
            
            // For positive coordinates within 16-bit range, they should match
            if x >= 0 && x <= 32767 && y >= 0 && y <= 32767 {
                assert_eq!(extracted_x, x, "X coordinate should be preserved");
                assert_eq!(extracted_y, y, "Y coordinate should be preserved");
            }
        }
    }
    
    #[test]
    fn test_cursor_position_functions() {
        // Test cursor position getter/setter functions
        // Note: These might fail in headless test environments
        
        match InputHandler::get_cursor_position() {
            Ok((x, y)) => {
                // If we can get cursor position, test setting it
                let new_x = x + 10;
                let new_y = y + 10;
                
                match InputHandler::set_cursor_position(new_x, new_y) {
                    Ok(()) => {
                        // Verify the position was set (with some tolerance for timing)
                        if let Ok((actual_x, actual_y)) = InputHandler::get_cursor_position() {
                            let tolerance = 5; // Allow some tolerance
                            assert!((actual_x - new_x).abs() <= tolerance, 
                                   "X position should be close to set value");
                            assert!((actual_y - new_y).abs() <= tolerance, 
                                   "Y position should be close to set value");
                        }
                        
                        // Restore original position
                        let _ = InputHandler::set_cursor_position(x, y);
                    }
                    Err(_) => {
                        // Setting cursor position might fail in test environment
                        println!("Warning: Could not set cursor position in test environment");
                    }
                }
            }
            Err(_) => {
                // Getting cursor position might fail in headless test environment
                println!("Warning: Could not get cursor position in test environment");
            }
        }
    }
    
    #[test]
    fn test_foreground_window() {
        // Test getting foreground window
        let foreground_window = InputHandler::get_foreground_window();
        
        // In a test environment, this might be null, but the function should not crash
        // We just verify that the function can be called without panicking
        println!("Foreground window handle: {:?}", foreground_window);
    }
    
    #[test]
    fn test_input_handler_with_real_window() {
        // This test attempts to use a real window if available
        // It will be skipped if no suitable window is found
        
        // Try to create a hidden desktop and launch an application
        match create_hidden_desktop("test_input_handler") {
            Ok(desktop) => {
                match launch_app_in_desktop(desktop.handle(), "notepad.exe") {
                    Ok(process_id) => {
                        // Wait for window to appear
                        std::thread::sleep(Duration::from_millis(1000));
                        
                        match find_window(process_id, "Notepad") {
                            Ok(window_handle) => {
                                let mut handler = InputHandler::new();
                                
                                // Test mouse move (should succeed)
                                let result = handler.handle_mouse_move(window_handle, 100, 100);
                                assert!(result.is_ok(), "Mouse move should succeed with real window");
                                
                                // Test keyboard event (should succeed)
                                let result = handler.handle_keyboard_event(window_handle, 65, true);
                                assert!(result.is_ok(), "Keyboard event should succeed with real window");
                                
                                // Test mouse click (should succeed)
                                let result = handler.handle_mouse_click(
                                    window_handle, 100, 100, MouseButton::Left, true
                                );
                                assert!(result.is_ok(), "Mouse click should succeed with real window");
                                
                                // Clean up: terminate the process
                                unsafe {
                                    let handle = winapi::um::processthreadsapi::OpenProcess(
                                        winapi::um::winnt::PROCESS_TERMINATE,
                                        0,
                                        process_id,
                                    );
                                    if !handle.is_null() {
                                        winapi::um::processthreadsapi::TerminateProcess(handle, 0);
                                        winapi::um::handleapi::CloseHandle(handle);
                                    }
                                }
                            }
                            Err(_) => {
                                println!("Warning: Could not find test window, skipping real window test");
                            }
                        }
                    }
                    Err(_) => {
                        println!("Warning: Could not launch test application, skipping real window test");
                    }
                }
            }
            Err(_) => {
                println!("Warning: Could not create test desktop, skipping real window test");
            }
        }
    }
    
    #[test]
    fn test_security_config() {
        let default_config = SecurityConfig::default();
        assert_eq!(default_config.max_events_per_second, 1000);
        assert_eq!(default_config.max_coordinate_value, 10000);
        assert_eq!(default_config.min_coordinate_value, -10000);
        assert!(!default_config.blocked_keycodes.is_empty());
        assert!(default_config.enable_coordinate_validation);
        assert!(default_config.enable_keycode_validation);
        
        let permissive_config = SecurityConfig::permissive();
        assert_eq!(permissive_config.max_events_per_second, 10000);
        assert!(permissive_config.blocked_keycodes.is_empty());
        assert!(!permissive_config.enable_coordinate_validation);
        assert!(!permissive_config.enable_keycode_validation);
    }
    
    #[test]
    fn test_input_handler_with_security() {
        let security_config = SecurityConfig {
            max_events_per_second: 10,
            max_coordinate_value: 1000,
            min_coordinate_value: -1000,
            blocked_keycodes: vec![91, 92], // Windows keys
            enable_coordinate_validation: true,
            enable_keycode_validation: true,
            max_consecutive_identical: 5,
        };
        
        let handler = InputHandler::new_with_security(security_config.clone());
        assert_eq!(handler.security_config.max_events_per_second, 10);
        assert_eq!(handler.security_config.blocked_keycodes, vec![91, 92]);
    }
    
    #[test]
    fn test_coordinate_validation_security() {
        let mut handler = InputHandler::new();
        
        // Test valid coordinates
        let valid_event = InputEvent::MouseMove { x: 100, y: 200 };
        let result = handler.validate_input_event_security(&valid_event);
        assert!(result.is_ok(), "Valid coordinates should pass validation");
        
        // Test invalid coordinates (too large)
        let invalid_event = InputEvent::MouseMove { x: 20000, y: 200 };
        let result = handler.validate_input_event_security(&invalid_event);
        assert!(result.is_err(), "Invalid coordinates should fail validation");
        
        // Test invalid coordinates (too small)
        let invalid_event = InputEvent::MouseMove { x: 100, y: -20000 };
        let result = handler.validate_input_event_security(&invalid_event);
        assert!(result.is_err(), "Invalid coordinates should fail validation");
    }
    
    #[test]
    fn test_keycode_blocking() {
        let mut handler = InputHandler::new();
        
        // Test that Windows keys are blocked by default
        let windows_key_event = InputEvent::KeyPress { keycode: 91, pressed: true };
        let result = handler.validate_input_event_security(&windows_key_event);
        assert!(result.is_err(), "Windows key should be blocked by default");
        
        // Test normal key
        let normal_key_event = InputEvent::KeyPress { keycode: 65, pressed: true }; // 'A'
        let result = handler.validate_input_event_security(&normal_key_event);
        assert!(result.is_ok(), "Normal key should not be blocked");
        
        // Test blocking/unblocking
        handler.block_keycode(65); // Block 'A'
        assert!(handler.is_keycode_blocked(65));
        
        let blocked_event = InputEvent::KeyPress { keycode: 65, pressed: true };
        let result = handler.validate_input_event_security(&blocked_event);
        assert!(result.is_err(), "Blocked key should fail validation");
        
        handler.unblock_keycode(65); // Unblock 'A'
        assert!(!handler.is_keycode_blocked(65));
        
        let result = handler.validate_input_event_security(&blocked_event);
        assert!(result.is_ok(), "Unblocked key should pass validation");
    }
    
    #[test]
    fn test_consecutive_event_limiting() {
        let mut handler = InputHandler::new();
        handler.security_config.max_consecutive_identical = 3;
        
        let event = InputEvent::MouseMove { x: 100, y: 100 };
        
        // First few events should pass
        for i in 0..3 {
            let result = handler.validate_input_event_security(&event);
            assert!(result.is_ok(), "Event {} should pass validation", i + 1);
            handler.update_statistics(&event);
        }
        
        // Fourth identical event should fail
        let result = handler.validate_input_event_security(&event);
        assert!(result.is_err(), "Fourth consecutive identical event should fail");
        
        // Different event should reset counter
        let different_event = InputEvent::MouseMove { x: 200, y: 200 };
        let result = handler.validate_input_event_security(&different_event);
        assert!(result.is_ok(), "Different event should pass validation");
    }
    
    #[test]
    fn test_input_statistics() {
        let mut handler = InputHandler::new();
        
        // Initial statistics
        let stats = handler.get_statistics();
        assert_eq!(stats.total_events, 0);
        assert_eq!(stats.rate_limited_events, 0);
        assert_eq!(stats.validation_rejected_events, 0);
        
        // Process some events
        let events = vec![
            InputEvent::MouseMove { x: 100, y: 100 },
            InputEvent::MouseClick { x: 200, y: 200, button: MouseButton::Left },
            InputEvent::KeyPress { keycode: 65, pressed: true },
        ];
        
        for event in events {
            handler.update_statistics(&event);
        }
        
        let stats = handler.get_statistics();
        assert_eq!(stats.total_events, 3);
        assert_eq!(stats.events_by_type.get("mouse_move").unwrap_or(&0), &1);
        assert_eq!(stats.events_by_type.get("mouse_click").unwrap_or(&0), &1);
        assert_eq!(stats.events_by_type.get("key_press").unwrap_or(&0), &1);
        
        // Test statistics reset
        handler.reset_statistics();
        let stats = handler.get_statistics();
        assert_eq!(stats.total_events, 0);
        assert!(stats.events_by_type.is_empty());
    }
    
    #[test]
    fn test_events_per_second_calculation() {
        let mut handler = InputHandler::new();
        
        // Initially should be 0
        assert_eq!(handler.get_events_per_second(), 0.0);
        
        // Add some events
        for _ in 0..10 {
            handler.update_statistics(&InputEvent::MouseMove { x: 100, y: 100 });
        }
        
        // Should have some rate (exact value depends on timing)
        let rate = handler.get_events_per_second();
        assert!(rate > 0.0, "Events per second should be positive");
    }
    
    #[test]
    fn test_attack_detection() {
        let mut handler = InputHandler::new();
        
        // Initially not under attack
        assert!(!handler.is_under_attack());
        
        // Simulate many events with high rejection rate
        for _ in 0..200 {
            handler.input_statistics.total_events += 1;
            handler.input_statistics.validation_rejected_events += 1;
        }
        
        // Should detect attack
        assert!(handler.is_under_attack(), "Should detect attack with high rejection rate");
        
        // Reset and test with normal traffic
        handler.reset_statistics();
        for _ in 0..200 {
            handler.input_statistics.total_events += 1;
            // Only 10% rejection rate
            if handler.input_statistics.total_events % 10 == 0 {
                handler.input_statistics.validation_rejected_events += 1;
            }
        }
        
        assert!(!handler.is_under_attack(), "Should not detect attack with low rejection rate");
    }
    
    #[test]
    fn test_security_configuration_updates() {
        let mut handler = InputHandler::new();
        
        // Test initial configuration
        let initial_config = handler.get_security_config();
        assert_eq!(initial_config.max_events_per_second, 1000);
        
        // Update configuration
        let new_config = SecurityConfig {
            max_events_per_second: 500,
            max_coordinate_value: 5000,
            min_coordinate_value: -5000,
            blocked_keycodes: vec![1, 2, 3],
            enable_coordinate_validation: false,
            enable_keycode_validation: false,
            max_consecutive_identical: 50,
        };
        
        handler.set_security_config(new_config.clone());
        
        let updated_config = handler.get_security_config();
        assert_eq!(updated_config.max_events_per_second, 500);
        assert_eq!(updated_config.blocked_keycodes, vec![1, 2, 3]);
        assert!(!updated_config.enable_coordinate_validation);
    }
    
    #[test]
    fn test_permissive_security_mode() {
        let permissive_config = SecurityConfig::permissive();
        let mut handler = InputHandler::new_with_security(permissive_config);
        
        // Test that extreme coordinates pass in permissive mode
        let extreme_event = InputEvent::MouseMove { x: 50000, y: -50000 };
        let result = handler.validate_input_event_security(&extreme_event);
        assert!(result.is_ok(), "Extreme coordinates should pass in permissive mode");
        
        // Test that high keycodes pass in permissive mode
        let high_keycode_event = InputEvent::KeyPress { keycode: 300, pressed: true };
        let result = handler.validate_input_event_security(&high_keycode_event);
        assert!(result.is_ok(), "High keycodes should pass in permissive mode");
    }
    
    #[test]
    fn test_input_validation_with_disabled_features() {
        let mut config = SecurityConfig::default();
        config.enable_coordinate_validation = false;
        config.enable_keycode_validation = false;
        
        let mut handler = InputHandler::new_with_security(config);
        
        // Test that validation is bypassed when disabled
        let extreme_event = InputEvent::MouseMove { x: 100000, y: -100000 };
        let result = handler.validate_input_event_security(&extreme_event);
        assert!(result.is_ok(), "Coordinate validation should be bypassed when disabled");
        
        let blocked_key_event = InputEvent::KeyPress { keycode: 91, pressed: true };
        let result = handler.validate_input_event_security(&blocked_key_event);
        assert!(result.is_ok(), "Keycode validation should be bypassed when disabled");
    }
    
    #[test]
    fn test_input_event_processing_sequence() {
        let mut handler = InputHandler::new();
        let test_window = 0x12345678 as WindowHandle; // Mock window handle
        
        // Test a sequence of input events
        let event_sequence = vec![
            InputEvent::MouseMove { x: 50, y: 50 },
            InputEvent::MouseClick { x: 100, y: 100, button: MouseButton::Left },
            InputEvent::KeyPress { keycode: 72, pressed: true },  // 'H'
            InputEvent::KeyPress { keycode: 72, pressed: false }, // 'H' release
            InputEvent::KeyPress { keycode: 73, pressed: true },  // 'I'
            InputEvent::KeyPress { keycode: 73, pressed: false }, // 'I' release
            InputEvent::MouseMove { x: 200, y: 200 },
        ];
        
        for (i, event) in event_sequence.iter().enumerate() {
            let result = handler.process_input_event(test_window, event.clone());
            
            // With mock window handle, we expect Windows API errors, not validation errors
            match result {
                Ok(()) => {
                    // Unexpected success, but not necessarily wrong
                }
                Err(HvncError::WindowsApi { .. }) | Err(HvncError::WindowNotFound { .. }) => {
                    // Expected for mock window handle
                }
                Err(e) => {
                    panic!("Unexpected error for event {}: {:?}", i, e);
                }
            }
        }
    }
}