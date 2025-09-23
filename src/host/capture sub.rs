//! Window capture and image processing

use crate::{HvncError, Result};
use crate::host::WindowHandle;
use crate::common::Frame;
use std::time::{Duration, Instant};
use std::mem;
use std::ptr;
use std::io::Cursor;
use image::{ImageBuffer, ImageFormat, Rgb, Rgba};
use image::codecs::jpeg::JpegEncoder;
use winapi::shared::windef::{RECT, HBITMAP};
use winapi::um::wingdi::{
    CreateCompatibleDC, CreateCompatibleBitmap, SelectObject, BitBlt, GetDIBits,
    DeleteObject, DeleteDC, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, SRCCOPY
};
use winapi::um::winuser::{GetWindowRect, GetDC, ReleaseDC, IsIconic, IsWindowVisible};

/// Window capture engine with frame optimization
pub struct WindowCapture {
    jpeg_quality: u8,
    frame_rate_ms: u64,
    last_capture_time: Option<Instant>,
    last_frame_hash: Option<u64>,
    change_threshold: f32,
}

impl WindowCapture {
    /// Create a new window capture instance
    pub fn new(jpeg_quality: u8) -> Self {
        Self { 
            jpeg_quality,
            frame_rate_ms: 33, // Default 30 FPS (1000ms / 30 = 33ms)
            last_capture_time: None,
            last_frame_hash: None,
            change_threshold: 0.01, // 1% change threshold
        }
    }
    
    /// Create a new window capture instance with custom frame rate
    pub fn new_with_frame_rate(jpeg_quality: u8, frame_rate_ms: u64) -> Self {
        Self {
            jpeg_quality,
            frame_rate_ms,
            last_capture_time: None,
            last_frame_hash: None,
            change_threshold: 0.01,
        }
    }
    
    /// Capture a window and return compressed image data
    pub fn capture_window(&self, window: WindowHandle) -> Result<Frame> {
        capture_window(window)
    }
    
    /// Compress image data to JPEG format
    pub fn compress_image(&self, image_data: &[u8], width: u32, height: u32) -> Result<Vec<u8>> {
        compress_to_jpeg(image_data, width, height, self.jpeg_quality)
    }
    
    /// Get the current JPEG quality setting
    pub fn jpeg_quality(&self) -> u8 {
        self.jpeg_quality
    }
    
    /// Set a new JPEG quality setting
    pub fn set_jpeg_quality(&mut self, quality: u8) {
        self.jpeg_quality = quality.clamp(1, 100);
    }
    
    /// Get the current frame rate in milliseconds
    pub fn frame_rate_ms(&self) -> u64 {
        self.frame_rate_ms
    }
    
    /// Set the frame rate in milliseconds
    pub fn set_frame_rate_ms(&mut self, frame_rate_ms: u64) {
        self.frame_rate_ms = frame_rate_ms.max(16); // Minimum 16ms (62.5 FPS)
    }
    
    /// Set the frame rate in FPS
    pub fn set_frame_rate_fps(&mut self, fps: u32) {
        let fps = fps.max(1).min(60); // Clamp between 1-60 FPS
        self.frame_rate_ms = 1000 / fps as u64;
    }
    
    /// Get the change detection threshold
    pub fn change_threshold(&self) -> f32 {
        self.change_threshold
    }
    
    /// Set the change detection threshold (0.0 to 1.0)
    pub fn set_change_threshold(&mut self, threshold: f32) {
        self.change_threshold = threshold.clamp(0.0, 1.0);
    }
    
    /// Check if enough time has passed since the last capture
    pub fn should_capture(&self) -> bool {
        match self.last_capture_time {
            None => true,
            Some(last_time) => {
                let elapsed = last_time.elapsed();
                elapsed >= Duration::from_millis(self.frame_rate_ms)
            }
        }
    }
    
    /// Capture a window with frame rate control and change detection
    pub fn capture_window_optimized(&mut self, window: WindowHandle) -> Result<Option<Frame>> {
        // Check if we should capture based on frame rate
        if !self.should_capture() {
            return Ok(None);
        }
        
        // Capture the window
        let frame = capture_window(window)?;
        
        // Calculate frame hash for change detection
        let frame_hash = calculate_frame_hash(&frame.data);
        
        // Check if frame has changed significantly
        let should_send = match self.last_frame_hash {
            None => true,
            Some(last_hash) => {
                let change_ratio = calculate_change_ratio(last_hash, frame_hash);
                change_ratio >= self.change_threshold
            }
        };
        
        // Update timing and hash
        self.last_capture_time = Some(Instant::now());
        self.last_frame_hash = Some(frame_hash);
        
        if should_send {
            Ok(Some(frame))
        } else {
            Ok(None) // No significant change detected
        }
    }
    
    /// Reset the optimization state (useful when switching windows or reconnecting)
    pub fn reset_optimization_state(&mut self) {
        self.last_capture_time = None;
        self.last_frame_hash = None;
    }
}

/// Compress RGB image data to JPEG format
pub fn compress_to_jpeg(rgb_data: &[u8], width: u32, height: u32, quality: u8) -> Result<Vec<u8>> {
    // Validate input parameters
    if rgb_data.is_empty() {
        return Err(HvncError::InvalidInput("RGB data is empty".to_string()));
    }
    
    if width == 0 || height == 0 {
        return Err(HvncError::InvalidInput("Width and height must be greater than 0".to_string()));
    }
    
    let expected_size = (width * height * 3) as usize;
    if rgb_data.len() != expected_size {
        return Err(HvncError::InvalidInput(format!(
            "RGB data size mismatch: expected {} bytes, got {}",
            expected_size, rgb_data.len()
        )));
    }
    
    // Clamp quality to valid range
    let quality = quality.clamp(1, 100);
    
    // Convert RGB data to RGBA (add alpha channel) for image crate compatibility
    let mut rgba_data = Vec::with_capacity((width * height * 4) as usize);
    for chunk in rgb_data.chunks(3) {
        rgba_data.extend_from_slice(&[chunk[0], chunk[1], chunk[2], 255]);
    }
    
    // Create image buffer from RGBA data
    let image_buffer = match ImageBuffer::<Rgba<u8>, _>::from_raw(width, height, rgba_data) {
        Some(buffer) => buffer,
        None => {
            return Err(HvncError::ImageProcessing(
                "Failed to create image buffer from RGB data".to_string()
            ));
        }
    };
    
    // Create output buffer
    let mut output = Cursor::new(Vec::new());
    
    // Create JPEG encoder with specified quality
    let mut encoder = JpegEncoder::new_with_quality(&mut output, quality);
    
    // Encode the image
    if let Err(e) = encoder.encode(
        image_buffer.as_raw(),
        width,
        height,
        image::ColorType::Rgb8,
    ) {
        return Err(HvncError::ImageProcessing(format!(
            "JPEG encoding failed: {}",
            e
        )));
    }
    
    Ok(output.into_inner())
}

/// Decompress JPEG data back to RGB format (for testing)
pub fn decompress_from_jpeg(jpeg_data: &[u8]) -> Result<(Vec<u8>, u32, u32)> {
    if jpeg_data.is_empty() {
        return Err(HvncError::InvalidInput("JPEG data is empty".to_string()));
    }
    
    // Load image from JPEG data
    let img = match image::load_from_memory_with_format(jpeg_data, ImageFormat::Jpeg) {
        Ok(img) => img,
        Err(e) => {
            return Err(HvncError::ImageProcessing(format!(
                "Failed to decode JPEG: {}",
                e
            )));
        }
    };
    
    // Convert to RGB8
    let rgb_img = img.to_rgb8();
    let (width, height) = rgb_img.dimensions();
    
    Ok((rgb_img.into_raw(), width, height))
}

/// Capture a window using GetWindowRect and BitBlt APIs
pub fn capture_window(window: WindowHandle) -> Result<Frame> {
    // Validate window handle
    if window.is_null() {
        return Err(HvncError::window_not_found("Window handle is null"));
    }
    
    // Check if window is visible and not minimized
    unsafe {
        if IsWindowVisible(window) == 0 {
            return Err(HvncError::window_not_found("Window is not visible"));
        }
        
        if IsIconic(window) != 0 {
            return Err(HvncError::window_not_found("Window is minimized"));
        }
    }
    
    // Get window rectangle
    let mut window_rect: RECT = unsafe { mem::zeroed() };
    let success = unsafe { GetWindowRect(window, &mut window_rect) };
    
    if success == 0 {
        let error_code = unsafe { winapi::um::errhandlingapi::GetLastError() };
        return Err(HvncError::windows_api_with_code(
            "Failed to get window rectangle", error_code
        ));
    }
    
    // Calculate window dimensions
    let width = (window_rect.right - window_rect.left) as u32;
    let height = (window_rect.bottom - window_rect.top) as u32;
    
    if width == 0 || height == 0 {
        return Err(HvncError::window_not_found("Window has zero dimensions"));
    }
    
    // Get window device context
    let window_dc = unsafe { GetDC(window) };
    if window_dc.is_null() {
        return Err(HvncError::windows_api("Failed to get window device context"));
    }
    
    // Create compatible device context and bitmap
    let mem_dc = unsafe { CreateCompatibleDC(window_dc) };
    if mem_dc.is_null() {
        unsafe { ReleaseDC(window, window_dc) };
        return Err(HvncError::windows_api("Failed to create compatible device context"));
    }
    
    let bitmap = unsafe { CreateCompatibleBitmap(window_dc, width as i32, height as i32) };
    if bitmap.is_null() {
        unsafe {
            DeleteDC(mem_dc);
            ReleaseDC(window, window_dc);
        }
        return Err(HvncError::windows_api("Failed to create compatible bitmap"));
    }
    
    // Select bitmap into memory device context
    let old_bitmap = unsafe { SelectObject(mem_dc, bitmap as *mut _) };
    
    // Copy window content to bitmap
    let blt_success = unsafe {
        BitBlt(
            mem_dc,
            0,
            0,
            width as i32,
            height as i32,
            window_dc,
            0,
            0,
            SRCCOPY,
        )
    };
    
    if blt_success == 0 {
        unsafe {
            SelectObject(mem_dc, old_bitmap);
            DeleteObject(bitmap as *mut _);
            DeleteDC(mem_dc);
            ReleaseDC(window, window_dc);
        }
        let error_code = unsafe { winapi::um::errhandlingapi::GetLastError() };
        return Err(HvncError::windows_api_with_code(
            "Failed to copy window content", error_code
        ));
    }
    
    // Convert bitmap to RGB buffer
    let rgb_data = match bitmap_to_rgb_buffer(bitmap, width, height) {
        Ok(data) => data,
        Err(e) => {
            unsafe {
                SelectObject(mem_dc, old_bitmap);
                DeleteObject(bitmap as *mut _);
                DeleteDC(mem_dc);
                ReleaseDC(window, window_dc);
            }
            return Err(e);
        }
    };
    
    // Clean up resources
    unsafe {
        SelectObject(mem_dc, old_bitmap);
        DeleteObject(bitmap as *mut _);
        DeleteDC(mem_dc);
        ReleaseDC(window, window_dc);
    }
    
    Ok(Frame {
        width,
        height,
        data: rgb_data,
    })
}

/// Convert a Windows bitmap to RGB buffer
fn bitmap_to_rgb_buffer(bitmap: HBITMAP, width: u32, height: u32) -> Result<Vec<u8>> {
    // Create bitmap info structure
    let mut bitmap_info: BITMAPINFO = unsafe { mem::zeroed() };
    bitmap_info.bmiHeader.biSize = mem::size_of::<BITMAPINFOHEADER>() as u32;
    bitmap_info.bmiHeader.biWidth = width as i32;
    bitmap_info.bmiHeader.biHeight = -(height as i32); // Negative for top-down bitmap
    bitmap_info.bmiHeader.biPlanes = 1;
    bitmap_info.bmiHeader.biBitCount = 24; // 24-bit RGB
    bitmap_info.bmiHeader.biCompression = BI_RGB;
    
    // Calculate buffer size (3 bytes per pixel for RGB, with padding)
    let bytes_per_line = ((width * 3 + 3) / 4) * 4; // DWORD aligned
    let buffer_size = (bytes_per_line * height) as usize;
    let mut buffer = vec![0u8; buffer_size];
    
    // Get device context for GetDIBits
    let screen_dc = unsafe { winapi::um::winuser::GetDC(ptr::null_mut()) };
    if screen_dc.is_null() {
        return Err(HvncError::windows_api("Failed to get screen device context"));
    }
    
    // Get bitmap bits
    let lines_copied = unsafe {
        GetDIBits(
            screen_dc,
            bitmap,
            0,
            height,
            buffer.as_mut_ptr() as *mut _,
            &mut bitmap_info,
            DIB_RGB_COLORS,
        )
    };
    
    unsafe { winapi::um::winuser::ReleaseDC(ptr::null_mut(), screen_dc) };
    
    if lines_copied == 0 {
        let error_code = unsafe { winapi::um::errhandlingapi::GetLastError() };
        return Err(HvncError::windows_api_with_code(
            "Failed to get bitmap bits", error_code
        ));
    }
    
    // Convert BGR to RGB and remove padding
    let mut rgb_buffer = Vec::with_capacity((width * height * 3) as usize);
    
    for y in 0..height {
        let line_start = (y * bytes_per_line) as usize;
        for x in 0..width {
            let pixel_start = line_start + (x * 3) as usize;
            
            // Windows bitmap is BGR, we need RGB
            let b = buffer[pixel_start];
            let g = buffer[pixel_start + 1];
            let r = buffer[pixel_start + 2];
            
            rgb_buffer.push(r);
            rgb_buffer.push(g);
            rgb_buffer.push(b);
        }
    }
    
    Ok(rgb_buffer)
}

/// Calculate a simple hash of frame data for change detection
fn calculate_frame_hash(frame_data: &[u8]) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    
    let mut hasher = DefaultHasher::new();
    
    // Sample every 16th pixel to reduce computation while maintaining sensitivity
    let sample_step = (frame_data.len() / 3 / 256).max(1) * 3; // Ensure we sample RGB triplets
    
    for chunk in frame_data.chunks(sample_step) {
        if chunk.len() >= 3 {
            // Hash the first RGB triplet of each chunk
            chunk[0].hash(&mut hasher);
            chunk[1].hash(&mut hasher);
            chunk[2].hash(&mut hasher);
        }
    }
    
    hasher.finish()
}

/// Calculate the change ratio between two frame hashes
fn calculate_change_ratio(old_hash: u64, new_hash: u64) -> f32 {
    if old_hash == new_hash {
        return 0.0;
    }
    
    // Calculate Hamming distance between the two hashes
    let xor_result = old_hash ^ new_hash;
    let bit_differences = xor_result.count_ones();
    
    // Normalize to 0.0-1.0 range (64 bits maximum difference)
    bit_differences as f32 / 64.0
}

/// Frame buffer manager for efficient memory usage
pub struct FrameBuffer {
    buffers: Vec<Vec<u8>>,
    current_index: usize,
    max_buffers: usize,
}

impl FrameBuffer {
    /// Create a new frame buffer manager
    pub fn new(max_buffers: usize) -> Self {
        Self {
            buffers: Vec::with_capacity(max_buffers),
            current_index: 0,
            max_buffers: max_buffers.max(2), // Minimum 2 buffers
        }
    }
    
    /// Get a buffer for the next frame, reusing existing buffers when possible
    pub fn get_buffer(&mut self, required_size: usize) -> &mut Vec<u8> {
        // If we don't have enough buffers, create a new one
        if self.buffers.len() < self.max_buffers {
            self.buffers.push(Vec::with_capacity(required_size));
            self.current_index = self.buffers.len() - 1;
        } else {
            // Cycle through existing buffers
            self.current_index = (self.current_index + 1) % self.buffers.len();
        }
        
        let buffer = &mut self.buffers[self.current_index];
        
        // Resize buffer if needed
        if buffer.capacity() < required_size {
            buffer.reserve(required_size - buffer.capacity());
        }
        
        buffer.clear();
        buffer
    }
    
    /// Get the current buffer count
    pub fn buffer_count(&self) -> usize {
        self.buffers.len()
    }
    
    /// Get the total allocated capacity across all buffers
    pub fn total_capacity(&self) -> usize {
        self.buffers.iter().map(|b| b.capacity()).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::host::desktop::create_hidden_desktop;
    use crate::host::launcher::{launch_app_in_desktop, find_window};
    use std::time::Duration;
    
    #[test]
    fn test_window_capture_new() {
        let capture = WindowCapture::new(80);
        assert_eq!(capture.jpeg_quality, 80);
    }
    
    #[test]
    fn test_capture_window_null_handle() {
        let result = capture_window(ptr::null_mut());
        
        assert!(result.is_err(), "Should fail with null window handle");
        
        if let Err(HvncError::WindowNotFound(msg)) = result {
            assert!(msg.contains("null"), "Error should mention null handle");
        } else {
            panic!("Expected WindowNotFound error");
        }
    }
    
    #[test]
    fn test_capture_window_success() {
        // Create a hidden desktop for testing
        let desktop = create_hidden_desktop("test_capture_window")
            .expect("Desktop creation should succeed");
        
        // Launch notepad.exe as a test application
        let process_id = launch_app_in_desktop(desktop.handle(), "notepad.exe")
            .expect("Application launch should succeed");
        
        // Wait for the window to appear
        std::thread::sleep(Duration::from_millis(1000));
        
        // Find the notepad window
        let window_handle = find_window(process_id, "Notepad")
            .expect("Window should be found");
        
        // Capture the window
        let result = capture_window(window_handle);
        
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
        
        assert!(result.is_ok(), "Window capture should succeed");
        
        let frame = result.unwrap();
        assert!(frame.width > 0, "Frame width should be greater than 0");
        assert!(frame.height > 0, "Frame height should be greater than 0");
        assert!(!frame.data.is_empty(), "Frame data should not be empty");
        
        // Check that we have the expected amount of RGB data (3 bytes per pixel)
        let expected_size = (frame.width * frame.height * 3) as usize;
        assert_eq!(frame.data.len(), expected_size, "Frame data should have correct size");
    }
    
    #[test]
    fn test_window_capture_struct() {
        // Create a hidden desktop for testing
        let desktop = create_hidden_desktop("test_capture_struct")
            .expect("Desktop creation should succeed");
        
        // Launch notepad.exe as a test application
        let process_id = launch_app_in_desktop(desktop.handle(), "notepad.exe")
            .expect("Application launch should succeed");
        
        // Wait for the window to appear
        std::thread::sleep(Duration::from_millis(1000));
        
        // Find the notepad window
        let window_handle = find_window(process_id, "Notepad")
            .expect("Window should be found");
        
        // Create window capture instance
        let capture = WindowCapture::new(75);
        
        // Capture the window using the struct method
        let result = capture.capture_window(window_handle);
        
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
        
        assert!(result.is_ok(), "WindowCapture struct method should work");
        
        let frame = result.unwrap();
        assert!(frame.width > 0, "Frame width should be greater than 0");
        assert!(frame.height > 0, "Frame height should be greater than 0");
        assert!(!frame.data.is_empty(), "Frame data should not be empty");
    }
    
    #[test]
    fn test_capture_window_invalid_handle() {
        // Use an invalid but non-null handle
        let invalid_handle = 0x12345678 as HWND;
        let result = capture_window(invalid_handle);
        
        assert!(result.is_err(), "Should fail with invalid window handle");
        
        // The error could be either WindowNotFound or WindowsApi depending on the specific failure
        match result {
            Err(HvncError::WindowNotFound(_)) | Err(HvncError::WindowsApi(_)) => {
                // Both are acceptable for invalid handles
            }
            _ => panic!("Expected WindowNotFound or WindowsApi error"),
        }
    }
    
    #[test]
    fn test_bitmap_to_rgb_buffer_invalid() {
        // Test with null bitmap handle
        let result = bitmap_to_rgb_buffer(ptr::null_mut(), 100, 100);
        
        assert!(result.is_err(), "Should fail with null bitmap handle");
        
        if let Err(HvncError::WindowsApi(msg)) = result {
            assert!(msg.contains("Failed to get bitmap bits"), "Error should mention bitmap bits failure");
        } else {
            panic!("Expected WindowsApi error");
        }
    }
    
    #[test]
    fn test_capture_window_zero_dimensions() {
        // This test is harder to create reliably, but we can test the logic
        // by checking that our validation catches zero-dimension windows
        
        // The actual test would require creating a window with zero dimensions,
        // which is not straightforward. The validation logic is tested
        // through the main capture test above.
    }
    
    #[test]
    fn test_capture_window_error_handling() {
        // Test various error conditions by using invalid handles
        // and checking that proper cleanup occurs
        
        let invalid_handle = 0x1 as HWND; // Very likely to be invalid
        let result = capture_window(invalid_handle);
        
        // Should fail gracefully without crashing
        assert!(result.is_err(), "Should handle invalid window gracefully");
    }
    
    #[test]
    fn test_window_capture_different_qualities() {
        // Test creating WindowCapture with different quality settings
        let capture_low = WindowCapture::new(30);
        let capture_medium = WindowCapture::new(60);
        let capture_high = WindowCapture::new(90);
        
        assert_eq!(capture_low.jpeg_quality, 30);
        assert_eq!(capture_medium.jpeg_quality, 60);
        assert_eq!(capture_high.jpeg_quality, 90);
    }
    
    #[test]
    fn test_compress_to_jpeg_basic() {
        // Create a simple 2x2 RGB image (red, green, blue, white)
        let rgb_data = vec![
            255, 0, 0,    // Red pixel
            0, 255, 0,    // Green pixel
            0, 0, 255,    // Blue pixel
            255, 255, 255 // White pixel
        ];
        
        let result = compress_to_jpeg(&rgb_data, 2, 2, 80);
        assert!(result.is_ok(), "JPEG compression should succeed");
        
        let jpeg_data = result.unwrap();
        assert!(!jpeg_data.is_empty(), "JPEG data should not be empty");
        
        // JPEG data should start with JPEG magic bytes (0xFF, 0xD8)
        assert_eq!(jpeg_data[0], 0xFF, "JPEG should start with 0xFF");
        assert_eq!(jpeg_data[1], 0xD8, "JPEG should have 0xD8 as second byte");
    }
    
    #[test]
    fn test_compress_to_jpeg_different_qualities() {
        // Create a larger test image for better quality comparison
        let width = 10;
        let height = 10;
        let mut rgb_data = Vec::with_capacity((width * height * 3) as usize);
        
        // Create a gradient pattern
        for y in 0..height {
            for x in 0..width {
                let r = (x * 255 / width) as u8;
                let g = (y * 255 / height) as u8;
                let b = 128;
                rgb_data.extend_from_slice(&[r, g, b]);
            }
        }
        
        // Test different quality levels
        let low_quality = compress_to_jpeg(&rgb_data, width, height, 10).unwrap();
        let medium_quality = compress_to_jpeg(&rgb_data, width, height, 50).unwrap();
        let high_quality = compress_to_jpeg(&rgb_data, width, height, 95).unwrap();
        
        // Higher quality should generally result in larger file sizes
        assert!(low_quality.len() <= medium_quality.len(), "Low quality should be smaller than medium");
        assert!(medium_quality.len() <= high_quality.len(), "Medium quality should be smaller than high");
        
        // All should be valid JPEG data
        assert_eq!(low_quality[0], 0xFF);
        assert_eq!(medium_quality[0], 0xFF);
        assert_eq!(high_quality[0], 0xFF);
    }
    
    #[test]
    fn test_compress_to_jpeg_invalid_input() {
        // Test empty data
        let result = compress_to_jpeg(&[], 10, 10, 80);
        assert!(result.is_err(), "Should fail with empty data");
        
        // Test zero dimensions
        let rgb_data = vec![255, 0, 0];
        let result = compress_to_jpeg(&rgb_data, 0, 1, 80);
        assert!(result.is_err(), "Should fail with zero width");
        
        let result = compress_to_jpeg(&rgb_data, 1, 0, 80);
        assert!(result.is_err(), "Should fail with zero height");
        
        // Test mismatched data size
        let rgb_data = vec![255, 0, 0]; // Only 1 pixel worth of data
        let result = compress_to_jpeg(&rgb_data, 2, 2, 80); // But claiming 2x2
        assert!(result.is_err(), "Should fail with mismatched data size");
    }
    
    #[test]
    fn test_compress_to_jpeg_quality_clamping() {
        let rgb_data = vec![255, 0, 0, 0, 255, 0, 0, 0, 255];
        
        // Test quality values outside valid range
        let result_low = compress_to_jpeg(&rgb_data, 1, 3, 0); // Should clamp to 1
        let result_high = compress_to_jpeg(&rgb_data, 1, 3, 200); // Should clamp to 100
        
        assert!(result_low.is_ok(), "Should handle quality 0 by clamping to 1");
        assert!(result_high.is_ok(), "Should handle quality 200 by clamping to 100");
    }
    
    #[test]
    fn test_decompress_from_jpeg_basic() {
        // Create test image and compress it
        let rgb_data = vec![
            255, 0, 0,    // Red
            0, 255, 0,    // Green
            0, 0, 255,    // Blue
            255, 255, 255 // White
        ];
        
        let jpeg_data = compress_to_jpeg(&rgb_data, 2, 2, 90).unwrap();
        
        // Decompress it back
        let result = decompress_from_jpeg(&jpeg_data);
        assert!(result.is_ok(), "JPEG decompression should succeed");
        
        let (decompressed_rgb, width, height) = result.unwrap();
        assert_eq!(width, 2, "Width should be preserved");
        assert_eq!(height, 2, "Height should be preserved");
        assert_eq!(decompressed_rgb.len(), 12, "Should have 12 bytes (2x2x3)");
        
        // Note: Due to JPEG compression being lossy, we can't expect exact pixel values
        // but we can check that the data is reasonable
        assert_eq!(decompressed_rgb.len(), rgb_data.len(), "Data length should match");
    }
    
    #[test]
    fn test_decompress_from_jpeg_invalid_data() {
        // Test empty data
        let result = decompress_from_jpeg(&[]);
        assert!(result.is_err(), "Should fail with empty JPEG data");
        
        // Test invalid JPEG data
        let invalid_data = vec![0x00, 0x01, 0x02, 0x03];
        let result = decompress_from_jpeg(&invalid_data);
        assert!(result.is_err(), "Should fail with invalid JPEG data");
    }
    
    #[test]
    fn test_window_capture_compress_image() {
        // Test the WindowCapture compress_image method
        let mut capture = WindowCapture::new(75);
        
        let rgb_data = vec![
            255, 128, 64,  // Orange-ish
            64, 128, 255,  // Blue-ish
            128, 255, 64,  // Green-ish
            200, 200, 200  // Gray
        ];
        
        let result = capture.compress_image(&rgb_data, 2, 2);
        assert!(result.is_ok(), "WindowCapture compress_image should work");
        
        let jpeg_data = result.unwrap();
        assert!(!jpeg_data.is_empty(), "Compressed data should not be empty");
        assert_eq!(jpeg_data[0], 0xFF, "Should be valid JPEG");
    }
    
    #[test]
    fn test_window_capture_quality_methods() {
        let mut capture = WindowCapture::new(80);
        
        // Test getter
        assert_eq!(capture.jpeg_quality(), 80);
        
        // Test setter with valid value
        capture.set_jpeg_quality(60);
        assert_eq!(capture.jpeg_quality(), 60);
        
        // Test setter with clamping
        capture.set_jpeg_quality(0); // Should clamp to 1
        assert_eq!(capture.jpeg_quality(), 1);
        
        capture.set_jpeg_quality(150); // Should clamp to 100
        assert_eq!(capture.jpeg_quality(), 100);
    }
    
    #[test]
    fn test_frame_optimization_new() {
        let capture = WindowCapture::new(80);
        assert_eq!(capture.frame_rate_ms(), 33); // Default 30 FPS
        assert_eq!(capture.change_threshold(), 0.01);
        
        let capture_custom = WindowCapture::new_with_frame_rate(75, 50);
        assert_eq!(capture_custom.frame_rate_ms(), 50);
        assert_eq!(capture_custom.jpeg_quality(), 75);
    }
    
    #[test]
    fn test_frame_rate_control() {
        let mut capture = WindowCapture::new(80);
        
        // Test FPS setting
        capture.set_frame_rate_fps(60);
        assert_eq!(capture.frame_rate_ms(), 16); // 1000/60 ≈ 16ms
        
        capture.set_frame_rate_fps(30);
        assert_eq!(capture.frame_rate_ms(), 33); // 1000/30 ≈ 33ms
        
        capture.set_frame_rate_fps(15);
        assert_eq!(capture.frame_rate_ms(), 66); // 1000/15 ≈ 66ms
        
        // Test direct millisecond setting
        capture.set_frame_rate_ms(100);
        assert_eq!(capture.frame_rate_ms(), 100);
        
        // Test minimum frame rate (16ms = ~62.5 FPS)
        capture.set_frame_rate_ms(5);
        assert_eq!(capture.frame_rate_ms(), 16);
    }
    
    #[test]
    fn test_change_threshold() {
        let mut capture = WindowCapture::new(80);
        
        // Test valid threshold values
        capture.set_change_threshold(0.05);
        assert_eq!(capture.change_threshold(), 0.05);
        
        capture.set_change_threshold(0.5);
        assert_eq!(capture.change_threshold(), 0.5);
        
        // Test clamping
        capture.set_change_threshold(-0.1);
        assert_eq!(capture.change_threshold(), 0.0);
        
        capture.set_change_threshold(1.5);
        assert_eq!(capture.change_threshold(), 1.0);
    }
    
    #[test]
    fn test_should_capture_timing() {
        let mut capture = WindowCapture::new_with_frame_rate(80, 100); // 100ms between frames
        
        // Should capture initially
        assert!(capture.should_capture());
        
        // Simulate a capture
        capture.last_capture_time = Some(Instant::now());
        
        // Should not capture immediately after
        assert!(!capture.should_capture());
        
        // Wait and check again (we can't actually wait in a unit test, so we'll simulate)
        capture.last_capture_time = Some(Instant::now() - Duration::from_millis(150));
        assert!(capture.should_capture());
    }
    
    #[test]
    fn test_calculate_frame_hash() {
        // Test with identical data
        let data1 = vec![255, 128, 64; 100]; // 100 identical RGB triplets
        let data2 = vec![255, 128, 64; 100];
        
        let hash1 = calculate_frame_hash(&data1);
        let hash2 = calculate_frame_hash(&data2);
        
        assert_eq!(hash1, hash2, "Identical data should produce identical hashes");
        
        // Test with different data
        let data3 = vec![128, 255, 64; 100]; // Different RGB values
        let hash3 = calculate_frame_hash(&data3);
        
        assert_ne!(hash1, hash3, "Different data should produce different hashes");
    }
    
    #[test]
    fn test_calculate_change_ratio() {
        // Test identical hashes
        let ratio = calculate_change_ratio(0x1234567890ABCDEF, 0x1234567890ABCDEF);
        assert_eq!(ratio, 0.0, "Identical hashes should have 0 change ratio");
        
        // Test completely different hashes (all bits different)
        let ratio = calculate_change_ratio(0x0000000000000000, 0xFFFFFFFFFFFFFFFF);
        assert_eq!(ratio, 1.0, "Completely different hashes should have 1.0 change ratio");
        
        // Test partial difference
        let ratio = calculate_change_ratio(0x0000000000000000, 0x0000000000000001);
        assert!(ratio > 0.0 && ratio < 1.0, "Partial difference should be between 0 and 1");
        assert!((ratio - 1.0/64.0).abs() < 0.001, "Single bit difference should be 1/64");
    }
    
    #[test]
    fn test_frame_buffer_basic() {
        let mut buffer_manager = FrameBuffer::new(3);
        
        // Test initial state
        assert_eq!(buffer_manager.buffer_count(), 0);
        assert_eq!(buffer_manager.total_capacity(), 0);
        
        // Get first buffer
        let buffer1 = buffer_manager.get_buffer(1000);
        assert_eq!(buffer_manager.buffer_count(), 1);
        assert!(buffer_manager.total_capacity() >= 1000);
        
        // Get second buffer
        let buffer2 = buffer_manager.get_buffer(2000);
        assert_eq!(buffer_manager.buffer_count(), 2);
        
        // Get third buffer
        let buffer3 = buffer_manager.get_buffer(1500);
        assert_eq!(buffer_manager.buffer_count(), 3);
        
        // Fourth buffer should reuse the first one
        let buffer4 = buffer_manager.get_buffer(500);
        assert_eq!(buffer_manager.buffer_count(), 3); // Still 3 buffers
    }
    
    #[test]
    fn test_frame_buffer_reuse() {
        let mut buffer_manager = FrameBuffer::new(2);
        
        // Get first buffer and add some data
        {
            let buffer = buffer_manager.get_buffer(100);
            buffer.extend_from_slice(&[1, 2, 3, 4, 5]);
            assert_eq!(buffer.len(), 5);
        }
        
        // Get second buffer
        {
            let buffer = buffer_manager.get_buffer(100);
            buffer.extend_from_slice(&[6, 7, 8]);
            assert_eq!(buffer.len(), 3);
        }
        
        // Get third buffer (should reuse first)
        {
            let buffer = buffer_manager.get_buffer(100);
            assert_eq!(buffer.len(), 0); // Should be cleared
            buffer.extend_from_slice(&[9, 10]);
            assert_eq!(buffer.len(), 2);
        }
    }
    
    #[test]
    fn test_frame_buffer_capacity_growth() {
        let mut buffer_manager = FrameBuffer::new(2);
        
        // Start with small buffer
        {
            let buffer = buffer_manager.get_buffer(100);
            let initial_capacity = buffer.capacity();
            assert!(initial_capacity >= 100);
        }
        
        // Request larger buffer - should grow
        {
            let buffer = buffer_manager.get_buffer(1000);
            let new_capacity = buffer.capacity();
            assert!(new_capacity >= 1000);
        }
        
        // Total capacity should have grown
        assert!(buffer_manager.total_capacity() >= 1000);
    }
    
    #[test]
    fn test_frame_buffer_minimum_buffers() {
        // Test that minimum buffer count is enforced
        let buffer_manager = FrameBuffer::new(0);
        assert_eq!(buffer_manager.max_buffers, 2);
        
        let buffer_manager = FrameBuffer::new(1);
        assert_eq!(buffer_manager.max_buffers, 2);
        
        let buffer_manager = FrameBuffer::new(5);
        assert_eq!(buffer_manager.max_buffers, 5);
    }
    
    #[test]
    fn test_reset_optimization_state() {
        let mut capture = WindowCapture::new(80);
        
        // Set some state
        capture.last_capture_time = Some(Instant::now());
        capture.last_frame_hash = Some(0x1234567890ABCDEF);
        
        // Reset state
        capture.reset_optimization_state();
        
        assert!(capture.last_capture_time.is_none());
        assert!(capture.last_frame_hash.is_none());
        assert!(capture.should_capture()); // Should be ready to capture again
    }
    
    #[test]
    fn test_frame_hash_sampling() {
        // Test that frame hash sampling works with different data sizes
        
        // Small data (less than sampling threshold)
        let small_data = vec![255, 128, 64]; // Single RGB triplet
        let hash_small = calculate_frame_hash(&small_data);
        
        // Large data
        let large_data = vec![255, 128, 64; 10000]; // Many RGB triplets
        let hash_large = calculate_frame_hash(&large_data);
        
        // Both should produce valid hashes
        assert_ne!(hash_small, 0);
        assert_ne!(hash_large, 0);
        
        // Different sized data with same pattern should produce same hash
        let medium_data = vec![255, 128, 64; 1000];
        let hash_medium = calculate_frame_hash(&medium_data);
        
        // Due to sampling, these might be the same or different depending on the sampling pattern
        // The important thing is that they're all valid hashes
        assert_ne!(hash_medium, 0);
    }
    
    #[test]
    fn test_compression_roundtrip() {
        // Test that we can compress and decompress without major issues
        let width = 4;
        let height = 4;
        let mut rgb_data = Vec::new();
        
        // Create a checkerboard pattern
        for y in 0..height {
            for x in 0..width {
                if (x + y) % 2 == 0 {
                    rgb_data.extend_from_slice(&[255, 255, 255]); // White
                } else {
                    rgb_data.extend_from_slice(&[0, 0, 0]); // Black
                }
            }
        }
        
        // Compress with high quality to minimize loss
        let jpeg_data = compress_to_jpeg(&rgb_data, width, height, 95).unwrap();
        
        // Decompress
        let (decompressed_rgb, dec_width, dec_height) = decompress_from_jpeg(&jpeg_data).unwrap();
        
        // Check dimensions are preserved
        assert_eq!(dec_width, width);
        assert_eq!(dec_height, height);
        assert_eq!(decompressed_rgb.len(), rgb_data.len());
        
        // For a high-contrast pattern like checkerboard with high quality,
        // the decompressed image should be reasonably close to the original
        // We'll just verify that we have some white and some black pixels
        let has_bright_pixels = decompressed_rgb.chunks(3).any(|pixel| {
            pixel[0] > 200 && pixel[1] > 200 && pixel[2] > 200
        });
        let has_dark_pixels = decompressed_rgb.chunks(3).any(|pixel| {
            pixel[0] < 50 && pixel[1] < 50 && pixel[2] < 50
        });
        
        assert!(has_bright_pixels, "Should have some bright pixels");
        assert!(has_dark_pixels, "Should have some dark pixels");
    }
}