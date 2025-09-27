//! Display management for the client

use crate::{HvncError, Result};
use crate::common::ClientConfig;
use minifb::{Window, WindowOptions, Key, Scale, ScaleMode};
use image::ImageFormat;

use log::{info, warn, error, debug};

/// Display manager for the client
pub struct DisplayManager {
    window: Option<Window>,
    config: ClientConfig,
    current_size: (usize, usize),
    buffer: Vec<u32>,
    scale_factor: f32,
    maintain_aspect_ratio: bool,
}

impl DisplayManager {
    /// Create a new display manager
    pub fn new(config: ClientConfig) -> Self {
        Self {
            window: None,
            config,
            current_size: (800, 600), // Default size
            buffer: Vec::new(),
            scale_factor: 1.0,
            maintain_aspect_ratio: true,
        }
    }
    
    /// Create a new display manager with custom settings
    pub fn new_with_settings(
        config: ClientConfig,
        scale_factor: f32,
        maintain_aspect_ratio: bool,
    ) -> Self {
        Self {
            window: None,
            config,
            current_size: (800, 600),
            buffer: Vec::new(),
            scale_factor: scale_factor.max(0.1).min(4.0), // Clamp between 0.1x and 4x
            maintain_aspect_ratio,
        }
    }
    
    /// Create and show the display window
    pub fn create_window(&mut self, width: usize, height: usize) -> Result<()> {
        info!("Creating display window: {}x{}", width, height);
        
        // Validate dimensions
        if width == 0 || height == 0 {
            return Err(HvncError::Connection(
                "Invalid window dimensions: width and height must be greater than 0".to_string()
            ));
        }
        
        if width > 4096 || height > 4096 {
            return Err(HvncError::Connection(
                "Window dimensions too large: maximum 4096x4096".to_string()
            ));
        }
        
        // Calculate scaled dimensions
        let scaled_width = (width as f32 * self.scale_factor) as usize;
        let scaled_height = (height as f32 * self.scale_factor) as usize;
        
        self.current_size = (width, height);
        
        // Configure window options
        let mut options = WindowOptions::default();
        options.resize = self.config.auto_resize;
        options.scale = Scale::X1; // We handle scaling manually
        options.scale_mode = if self.maintain_aspect_ratio {
            ScaleMode::AspectRatioStretch
        } else {
            ScaleMode::Stretch
        };
        
        // Create the window
        let window = Window::new(
            &self.config.window_title,
            scaled_width,
            scaled_height,
            options,
        ).map_err(|e| {
            error!("Failed to create window: {}", e);
            HvncError::Connection(format!("Failed to create display window: {}", e))
        })?;
        
        info!("Display window created successfully: {}x{} (scaled to {}x{})", 
              width, height, scaled_width, scaled_height);
        
        // Initialize buffer
        self.buffer = vec![0; width * height];
        self.window = Some(window);
        
        Ok(())
    }
    
    /// Update the display with new JPEG image data
    pub fn update_display(&mut self, jpeg_data: &[u8]) -> Result<()> {
        debug!("Updating display with {} bytes of JPEG data", jpeg_data.len());
        
        // 🔍 DIAGNOSTIC: Validate JPEG data
        if jpeg_data.len() < 10 {
            error!("❌ JPEG data too small: {} bytes", jpeg_data.len());
            return Err(HvncError::Connection("JPEG data too small".to_string()));
        }
        
        if jpeg_data[0] != 0xFF || jpeg_data[1] != 0xD8 {
            error!("❌ Invalid JPEG header: {:02X} {:02X} (expected FF D8)", jpeg_data[0], jpeg_data[1]);
            return Err(HvncError::Connection("Invalid JPEG header".to_string()));
        }
        
        info!("📷 Processing JPEG: {} bytes, header: FF D8 ✅", jpeg_data.len());
        
        // Decode JPEG data first
        let (rgb_data, img_width, img_height) = self.decode_jpeg(jpeg_data)?;
        
        // 🔍 DIAGNOSTIC: Check if decoded image is mostly black
        let non_zero_pixels = rgb_data.iter().filter(|&&pixel| pixel > 10).count(); // Allow for slight compression artifacts
        let total_pixels = rgb_data.len();
        let content_percentage = (non_zero_pixels as f32 / total_pixels as f32) * 100.0;
        
        info!("📊 Decoded image: {}x{}, content: {:.1}% non-black pixels", 
            img_width, img_height, content_percentage);
            
        if content_percentage < 5.0 {
            warn!("⚠️ Decoded image appears mostly black! Client will show black screen.");
            warn!("   This indicates the server capture is not working properly.");
        }
        
        // Check for valid image size before resizing
        if img_width <= 0 || img_height <= 0 {
            error!("Invalid decoded image size: {}x{}", img_width, img_height);
            return Err(HvncError::WindowResizeFailed(format!("Invalid size: {}x{}", img_width, img_height)));
        }
        if self.config.auto_resize && (img_width != self.current_size.0 || img_height != self.current_size.1) {
            info!("Auto-resizing window from {}x{} to {}x{}", 
                  self.current_size.0, self.current_size.1, img_width, img_height);
            self.resize_window(img_width, img_height)?;
        }
        
        // Convert RGB to ARGB format for minifb
        self.convert_rgb_to_buffer(&rgb_data, img_width, img_height)?;
        
        // Update the window
        let window = self.window.as_mut()
            .ok_or_else(|| HvncError::Connection("Display window not created".to_string()))?;
        
        info!("📺 Updating minifb window with buffer size: {}, dimensions: {}x{}", 
            self.buffer.len(), self.current_size.0, self.current_size.1);
        
        // 🔧 CRITICAL DEBUG: Show buffer statistics before update WITH COLOR FORMAT INFO
        let non_zero_buffer = self.buffer.iter().filter(|&&pixel| pixel != 0).count();
        let white_buffer = self.buffer.iter().filter(|&&pixel| pixel == 0xFFFFFF).count(); // WHITE in fixed format
        let black_buffer = self.buffer.iter().filter(|&&pixel| pixel == 0x000000).count();  // BLACK in fixed format
        
        info!("📊 Buffer stats (FIXED FORMAT): non-zero={}, white={}, black={}, total={}",
            non_zero_buffer, white_buffer, black_buffer, self.buffer.len());
            
        // 🔧 VERBOSE: Show what colors we're actually sending to minifb
        let sample_colors: Vec<u32> = self.buffer.iter().take(10).cloned().collect();
        info!("🎨 Sample colors being sent to minifb: {:?}", sample_colors);
        
        window.update_with_buffer(&self.buffer, self.current_size.0, self.current_size.1)
            .map_err(|e| {
                error!("Failed to update window buffer: {}", e);
                HvncError::Connection(format!("Failed to update display: {}", e))
            })?;
        
        // 🔧 FORCE WINDOW REFRESH: Ensure the window actually displays the new content
        window.update();
        
        info!("✅ Display update completed successfully - buffer sent to minifb");
        debug!("Display updated successfully");
        Ok(())
    }
    
    /// Decode JPEG data to RGB
    fn decode_jpeg(&self, jpeg_data: &[u8]) -> Result<(Vec<u8>, usize, usize)> {
        if jpeg_data.len() < 2 || jpeg_data[0] != 0xFF || jpeg_data[1] != 0xD8 {
            return Err(HvncError::Connection(
                "Invalid JPEG data: missing JPEG magic bytes".to_string()
            ));
        }
        
        let img = image::load_from_memory_with_format(jpeg_data, ImageFormat::Jpeg)
            .map_err(|e| {
                error!("Failed to decode JPEG: {}", e);
                HvncError::Image(e)
            })?;
        
        let rgb_img = img.to_rgb8();
        let (width, height) = rgb_img.dimensions();
        let rgb_data = rgb_img.into_raw();
        
        debug!("JPEG decoded: {}x{} pixels, {} bytes", width, height, rgb_data.len());
        Ok((rgb_data, width as usize, height as usize))
    }
    
    /// Convert RGB data to ARGB buffer for minifb
    fn convert_rgb_to_buffer(&mut self, rgb_data: &[u8], width: usize, height: usize) -> Result<()> {
        if rgb_data.len() != width * height * 3 {
            error!("❌ RGB buffer size mismatch: expected {} bytes ({}x{}x3), got {}",
                width * height * 3, width, height, rgb_data.len());
            return Err(HvncError::Connection(format!(
                "RGB data size mismatch: expected {} bytes, got {}",
                width * height * 3, rgb_data.len()
            )));
        }
        
        // Resize buffer if needed
        let required_size = width * height;
        if self.buffer.len() != required_size {
            self.buffer.resize(required_size, 0);
            info!("📊 Resized buffer to {} pixels ({}x{})", required_size, width, height);
        }
        
        // 🔍 DIAGNOSTIC: Check RGB data content before conversion
        let rgb_non_zero = rgb_data.iter().filter(|&&b| b > 10).count(); // Allow for compression artifacts
        let rgb_content_pct = (rgb_non_zero as f32 / rgb_data.len() as f32) * 100.0;
        
        info!("🎨 Converting RGB to display buffer: {:.1}% content", rgb_content_pct);
        
        if rgb_content_pct < 5.0 {
            error!("❌ RGB data appears to be mostly black/empty!");
            error!("   This will result in a black display.");
            
            // Sample first few pixels for debugging
            for i in 0..std::cmp::min(5, rgb_data.len() / 3) {
                let r = rgb_data[i * 3];
                let g = rgb_data[i * 3 + 1];
                let b = rgb_data[i * 3 + 2];
                error!("   Pixel {}: R={}, G={}, B={}", i, r, g, b);
            }
        } else {
            // 🔧 CRITICAL DEBUG: Sample pixels even when content looks good
            info!("🔍 Sampling first 5 pixels for verification:");
            for i in 0..std::cmp::min(5, rgb_data.len() / 3) {
                let r = rgb_data[i * 3];
                let g = rgb_data[i * 3 + 1];
                let b = rgb_data[i * 3 + 2];
                info!("   Pixel {}: R={}, G={}, B={}", i, r, g, b);
            }
        }
        
        // Convert RGB to proper format for minifb
        // 🔧 CRITICAL FIX: minifb expects 0x00RRGGBB format (NOT ARGB!)
        let mut converted_pixels = 0;
        for i in 0..required_size {
            let rgb_index = i * 3;
            let r = rgb_data[rgb_index] as u32;
            let g = rgb_data[rgb_index + 1] as u32;
            let b = rgb_data[rgb_index + 2] as u32;
            
            // 🔧 CRITICAL FIX: Use (r << 16) | (g << 8) | b format for minifb (RGB, not ARGB!)
            // minifb expects 0x00RRGGBB format without alpha channel
            self.buffer[i] = (r << 16) | (g << 8) | b;
            
            if r > 10 || g > 10 || b > 10 {
                converted_pixels += 1;
            }
        }
        
        let converted_pct = (converted_pixels as f32 / required_size as f32) * 100.0;
        info!("✅ Converted {} pixels, {:.1}% non-black", required_size, converted_pct);
        
        // 🔧 CRITICAL DEBUG: Show first few converted buffer values WITH VERBOSE DETAILS
        info!("🔍 First 5 converted buffer values (FIXED FORMAT):");
        for i in 0..std::cmp::min(5, self.buffer.len()) {
            let original_rgb_idx = i * 3;
            if original_rgb_idx + 2 < rgb_data.len() {
                let r = rgb_data[original_rgb_idx] as u32;
                let g = rgb_data[original_rgb_idx + 1] as u32;
                let b = rgb_data[original_rgb_idx + 2] as u32;
                info!("   Buffer[{}]: 0x{:08X} (R={}, G={}, B={}) - FIXED FORMAT", i, self.buffer[i], r, g, b);
            }
        }
        
        if converted_pct < 5.0 {
            error!("⚠️ CRITICAL: Converted buffer appears mostly black! Check server capture.");
        }
        
        Ok(())
    }
    
    /// Resize the window
    pub fn resize_window(&mut self, width: usize, height: usize) -> Result<()> {
        info!("Resizing window to {}x{}", width, height);
        
        // Validate dimensions
        if width == 0 || height == 0 {
            return Err(HvncError::Connection(
                "Invalid resize dimensions: width and height must be greater than 0".to_string()
            ));
        }
        
        if width > 4096 || height > 4096 {
            return Err(HvncError::Connection(
                "Resize dimensions too large: maximum 4096x4096".to_string()
            ));
        }
        
        self.current_size = (width, height);
        
        // Resize buffer
        self.buffer.resize(width * height, 0);
        
        // If window exists, we'll need to recreate it for proper resizing
        if let Some(ref mut window) = self.window {
            // minifb doesn't support runtime resizing, so we log the change
            // The actual resize will happen on the next frame update
            debug!("Window resize requested: {}x{}", width, height);
        }
        
        Ok(())
    }
    
    /// Check if the window is open
    pub fn is_open(&self) -> bool {
        match &self.window {
            Some(window) => window.is_open(),
            None => false,
        }
    }
    
    /// Process window events and return if window should close
    pub fn process_events(&mut self) -> bool {
        match &mut self.window {
            Some(window) => {
                window.update();
                
                // Check for close request
                if !window.is_open() || window.is_key_down(Key::Escape) {
                    info!("Window close requested");
                    return true;
                }
                
                false
            }
            None => true, // No window means we should close
        }
    }
    
    /// Get current window size
    pub fn get_size(&self) -> (usize, usize) {
        self.current_size
    }
    
    /// Get window reference for input handling
    pub fn get_window(&self) -> Option<&Window> {
        self.window.as_ref()
    }
    
    /// Get mutable window reference for input handling
    pub fn get_window_mut(&mut self) -> Option<&mut Window> {
        self.window.as_mut()
    }
    
    /// Set window title
    pub fn set_title(&mut self, title: &str) {
        self.config.window_title = title.to_string();
        // Note: minifb doesn't support runtime title changes
        info!("Window title set to: {}", title);
    }
    
    /// Get current scale factor
    pub fn get_scale_factor(&self) -> f32 {
        self.scale_factor
    }
    
    /// Set scale factor
    pub fn set_scale_factor(&mut self, scale_factor: f32) {
        let clamped_scale = scale_factor.max(0.1).min(4.0);
        if clamped_scale != scale_factor {
            warn!("Scale factor {} clamped to {}", scale_factor, clamped_scale);
        }
        
        self.scale_factor = clamped_scale;
        info!("Scale factor set to: {}", self.scale_factor);
    }
    
    /// Toggle aspect ratio maintenance
    pub fn set_maintain_aspect_ratio(&mut self, maintain: bool) {
        self.maintain_aspect_ratio = maintain;
        info!("Maintain aspect ratio: {}", maintain);
    }
    
    /// Get display statistics
    pub fn get_display_stats(&self) -> DisplayStats {
        DisplayStats {
            is_open: self.is_open(),
            current_size: self.current_size,
            buffer_size: self.buffer.len(),
            scale_factor: self.scale_factor,
            maintain_aspect_ratio: self.maintain_aspect_ratio,
            window_title: self.config.window_title.clone(),
            auto_resize: self.config.auto_resize,
        }
    }
    
    /// Close the window
    pub fn close(&mut self) {
        if self.window.is_some() {
            info!("Closing display window");
            self.window = None;
        }
    }
}

impl Drop for DisplayManager {
    fn drop(&mut self) {
        self.close();
    }
}

/// Display statistics
#[derive(Debug, Clone)]
pub struct DisplayStats {
    pub is_open: bool,
    pub current_size: (usize, usize),
    pub buffer_size: usize,
    pub scale_factor: f32,
    pub maintain_aspect_ratio: bool,
    pub window_title: String,
    pub auto_resize: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::ClientConfig;
    
    fn create_test_config() -> ClientConfig {
        ClientConfig {
            server_address: "127.0.0.1:5900".to_string(),
            window_title: "Test Window".to_string(),
            auto_resize: true,
        }
    }
    
    fn create_test_jpeg() -> Vec<u8> {
        // Create a minimal valid JPEG for testing
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
    
    #[test]
    fn test_display_manager_creation() {
        let config = create_test_config();
        let display = DisplayManager::new(config.clone());
        
        assert!(!display.is_open());
        assert_eq!(display.current_size, (800, 600));
        assert_eq!(display.scale_factor, 1.0);
        assert!(display.maintain_aspect_ratio);
        assert_eq!(display.config.window_title, "Test Window");
    }
    
    #[test]
    fn test_display_manager_with_settings() {
        let config = create_test_config();
        let display = DisplayManager::new_with_settings(config, 2.0, false);
        
        assert_eq!(display.scale_factor, 2.0);
        assert!(!display.maintain_aspect_ratio);
    }
    
    #[test]
    fn test_scale_factor_clamping() {
        let config = create_test_config();
        let display = DisplayManager::new_with_settings(config, 10.0, true);
        
        // Should be clamped to maximum 4.0
        assert_eq!(display.scale_factor, 4.0);
        
        let config = create_test_config();
        let display = DisplayManager::new_with_settings(config, 0.01, true);
        
        // Should be clamped to minimum 0.1
        assert_eq!(display.scale_factor, 0.1);
    }
    
    #[test]
    fn test_invalid_window_dimensions() {
        let config = create_test_config();
        let mut display = DisplayManager::new(config);
        
        // Test zero dimensions
        let result = display.create_window(0, 600);
        assert!(result.is_err());
        
        let result = display.create_window(800, 0);
        assert!(result.is_err());
        
        // Test oversized dimensions
        let result = display.create_window(5000, 600);
        assert!(result.is_err());
        
        let result = display.create_window(800, 5000);
        assert!(result.is_err());
    }
    
    #[test]
    fn test_resize_validation() {
        let config = create_test_config();
        let mut display = DisplayManager::new(config);
        
        // Test invalid resize dimensions
        let result = display.resize_window(0, 600);
        assert!(result.is_err());
        
        let result = display.resize_window(800, 0);
        assert!(result.is_err());
        
        let result = display.resize_window(5000, 600);
        assert!(result.is_err());
        
        // Test valid resize
        let result = display.resize_window(1024, 768);
        assert!(result.is_ok());
        assert_eq!(display.get_size(), (1024, 768));
    }
    
    #[test]
    fn test_jpeg_validation() {
        let config = create_test_config();
        let display = DisplayManager::new(config);
        
        // Test invalid JPEG data (no magic bytes)
        let invalid_jpeg = vec![0x00, 0x01, 0x02, 0x03];
        let result = display.decode_jpeg(&invalid_jpeg);
        assert!(result.is_err());
        
        // Test empty data
        let empty_data = vec![];
        let result = display.decode_jpeg(&empty_data);
        assert!(result.is_err());
        
        // Test partial magic bytes
        let partial_magic = vec![0xFF];
        let result = display.decode_jpeg(&partial_magic);
        assert!(result.is_err());
    }
    
    #[test]
    fn test_rgb_to_buffer_conversion() {
        let config = create_test_config();
        let mut display = DisplayManager::new(config);
        
        // Test valid RGB data
        let rgb_data = vec![
            255, 0, 0,    // Red pixel
            0, 255, 0,    // Green pixel
            0, 0, 255,    // Blue pixel
            255, 255, 255 // White pixel
        ];
        
        let result = display.convert_rgb_to_buffer(&rgb_data, 2, 2);
        assert!(result.is_ok());
        
        // Check buffer contents
        assert_eq!(display.buffer.len(), 4);
        assert_eq!(display.buffer[0], 0x0000FF); // Red in RGB format
        assert_eq!(display.buffer[1], 0x00FF00); // Green in RGB format
        assert_eq!(display.buffer[2], 0xFF0000); // Blue in RGB format
        assert_eq!(display.buffer[3], 0xFFFFFF); // White in RGB format
        
        // Test mismatched data size
        let wrong_size_data = vec![255, 0, 0]; // Only 1 pixel worth of data
        let result = display.convert_rgb_to_buffer(&wrong_size_data, 2, 2); // But claiming 2x2
        assert!(result.is_err());
    }
    
    #[test]
    fn test_display_stats() {
        let config = create_test_config();
        let display = DisplayManager::new_with_settings(config, 1.5, false);
        
        let stats = display.get_display_stats();
        assert!(!stats.is_open);
        assert_eq!(stats.current_size, (800, 600));
        assert_eq!(stats.scale_factor, 1.5);
        assert!(!stats.maintain_aspect_ratio);
        assert_eq!(stats.window_title, "Test Window");
        assert!(stats.auto_resize);
    }
    
    #[test]
    fn test_title_setting() {
        let config = create_test_config();
        let mut display = DisplayManager::new(config);
        
        display.set_title("New Title");
        assert_eq!(display.config.window_title, "New Title");
        
        let stats = display.get_display_stats();
        assert_eq!(stats.window_title, "New Title");
    }
    
    #[test]
    fn test_scale_factor_setting() {
        let config = create_test_config();
        let mut display = DisplayManager::new(config);
        
        display.set_scale_factor(2.5);
        assert_eq!(display.get_scale_factor(), 2.5);
        
        // Test clamping
        display.set_scale_factor(10.0);
        assert_eq!(display.get_scale_factor(), 4.0);
        
        display.set_scale_factor(0.01);
        assert_eq!(display.get_scale_factor(), 0.1);
    }
    
    #[test]
    fn test_aspect_ratio_setting() {
        let config = create_test_config();
        let mut display = DisplayManager::new(config);
        
        assert!(display.maintain_aspect_ratio);
        
        display.set_maintain_aspect_ratio(false);
        assert!(!display.maintain_aspect_ratio);
        
        display.set_maintain_aspect_ratio(true);
        assert!(display.maintain_aspect_ratio);
    }
    
    #[test]
    fn test_buffer_resizing() {
        let config = create_test_config();
        let mut display = DisplayManager::new(config);
        
        // Initial buffer should be empty
        assert_eq!(display.buffer.len(), 0);
        
        // Resize should update buffer
        display.resize_window(100, 50).unwrap();
        assert_eq!(display.buffer.len(), 5000); // 100 * 50
        
        // Another resize
        display.resize_window(200, 100).unwrap();
        assert_eq!(display.buffer.len(), 20000); // 200 * 100
    }
    
    #[test]
    fn test_window_state_without_creation() {
        let config = create_test_config();
        let display = DisplayManager::new(config);
        
        // Without creating window, should not be open
        assert!(!display.is_open());
        assert!(display.get_window().is_none());
    }
    
    #[test]
    fn test_close_functionality() {
        let config = create_test_config();
        let mut display = DisplayManager::new(config);
        
        // Close should work even without window
        display.close();
        assert!(!display.is_open());
    }
    
    #[test]
    fn test_color_conversion_edge_cases() {
        let config = create_test_config();
        let mut display = DisplayManager::new(config);
        
        // Test with extreme color values
        let rgb_data = vec![
            0, 0, 0,       // Black
            255, 255, 255, // White
            128, 128, 128, // Gray
            255, 0, 128,   // Custom color
        ];
        
        let result = display.convert_rgb_to_buffer(&rgb_data, 2, 2);
        assert!(result.is_ok());
        
        assert_eq!(display.buffer[0], 0x000000); // Black in RGB format
        assert_eq!(display.buffer[1], 0xFFFFFF); // White in RGB format
        assert_eq!(display.buffer[2], 0x808080); // Gray in RGB format
        assert_eq!(display.buffer[3], 0x800080); // Custom color in RGB format
    }
    
    #[test]
    fn test_display_manager_drop() {
        let config = create_test_config();
        let display = DisplayManager::new(config);
        
        // Drop should not panic
        drop(display);
    }
    
    // Note: Tests that require actual window creation are commented out
    // as they would require a display environment which may not be available
    // in CI/CD environments
    
    /*
    #[test]
    fn test_window_creation() {
        let config = create_test_config();
        let mut display = DisplayManager::new(config);
        
        let result = display.create_window(800, 600);
        // This test would require a display environment
        // assert!(result.is_ok());
        // assert!(display.is_open());
    }
    
    #[test]
    fn test_display_update() {
        let config = create_test_config();
        let mut display = DisplayManager::new(config);
        
        display.create_window(800, 600).unwrap();
        
        let test_jpeg = create_test_jpeg();
        let result = display.update_display(&test_jpeg);
        // This would require a valid JPEG and display environment
        // assert!(result.is_ok());
    }
    */
}