use std::ptr;
use std::io::Write;
use std::fs;

fn main() {
    println!("🔬 Complete HVNC Pipeline Debug");
    println!("===============================");
    
    // Step 1: Find notepad window
    let window = unsafe {
        winapi::um::winuser::FindWindowA(
            ptr::null(),
            "Untitled - Notepad\0".as_ptr() as *const i8
        )
    };
    
    if window.is_null() {
        println!("❌ Start notepad first! Run: Start-Process notepad");
        return;
    }
    
    println!("✅ Found notepad window: {:?}", window);
    
    // Step 2: Test basic window info
    unsafe {
        let is_valid = winapi::um::winuser::IsWindow(window) != 0;
        let is_visible = winapi::um::winuser::IsWindowVisible(window) != 0;
        let is_iconic = winapi::um::winuser::IsIconic(window) != 0;
        
        println!("📋 Window validation:");
        println!("   - IsWindow: {}", is_valid);
        println!("   - IsVisible: {}", is_visible);
        println!("   - IsIconic: {}", is_iconic);
        
        if !is_valid {
            println!("❌ Window handle is invalid!");
            return;
        }
    }
    
    // Step 3: Test window dimensions
    let mut rect = unsafe { std::mem::zeroed::<winapi::shared::windef::RECT>() };
    if unsafe { winapi::um::winuser::GetWindowRect(window, &mut rect) } == 0 {
        println!("❌ Failed to get window rectangle");
        return;
    }
    
    let width = rect.right - rect.left;
    let height = rect.bottom - rect.top;
    
    println!("📐 Window dimensions: {}x{}", width, height);
    
    if width <= 0 || height <= 0 {
        println!("❌ Invalid dimensions!");
        return;
    }
    
    // Step 4: Test DC creation
    let window_dc = unsafe { winapi::um::winuser::GetDC(window) };
    if window_dc.is_null() {
        println!("❌ Failed to get window DC");
        return;
    }
    
    println!("✅ Got window DC: {:?}", window_dc);
    
    let mem_dc = unsafe { winapi::um::wingdi::CreateCompatibleDC(window_dc) };
    if mem_dc.is_null() {
        println!("❌ Failed to create compatible DC");
        unsafe { winapi::um::winuser::ReleaseDC(window, window_dc) };
        return;
    }
    
    println!("✅ Created compatible DC: {:?}", mem_dc);
    
    // Step 5: Test bitmap creation
    let bitmap = unsafe { winapi::um::wingdi::CreateCompatibleBitmap(window_dc, width, height) };
    if bitmap.is_null() {
        println!("❌ Failed to create bitmap");
        unsafe {
            winapi::um::wingdi::DeleteDC(mem_dc);
            winapi::um::winuser::ReleaseDC(window, window_dc);
        }
        return;
    }
    
    println!("✅ Created bitmap: {:?}", bitmap);
    
    let old_bitmap = unsafe { winapi::um::wingdi::SelectObject(mem_dc, bitmap as *mut _) };
    
    // Step 6: Test BitBlt
    let blt_result = unsafe {
        winapi::um::wingdi::BitBlt(
            mem_dc,
            0, 0,
            width, height,
            window_dc,
            0, 0,
            winapi::um::wingdi::SRCCOPY,
        )
    };
    
    if blt_result == 0 {
        let error = unsafe { winapi::um::errhandlingapi::GetLastError() };
        println!("❌ BitBlt failed! Error: {}", error);
    } else {
        println!("✅ BitBlt successful!");
        
        // Step 7: Test pixel sampling
        let mut sample_pixels = Vec::new();
        for y in (0..height).step_by(height as usize / 10) {
            for x in (0..width).step_by(width as usize / 10) {
                let pixel = unsafe { winapi::um::wingdi::GetPixel(mem_dc, x, y) };
                sample_pixels.push(pixel);
            }
        }
        
        println!("🎨 Sample pixels: {:?}", sample_pixels);
        
        let black_pixels = sample_pixels.iter().filter(|&&p| p == 0).count();
        let total_pixels = sample_pixels.len();
        
        println!("📊 Pixel analysis: {}/{} black pixels ({:.1}%)", 
            black_pixels, total_pixels, 
            (black_pixels as f32 / total_pixels as f32) * 100.0);
            
        if black_pixels == total_pixels {
            println!("❌ ALL PIXELS ARE BLACK! This is the source of your black screen!");
        } else {
            println!("✅ Some pixels have color - capture should work");
        }
        
        // Step 8: Test RGB conversion
        println!("\n🔄 Testing RGB conversion...");
        
        let mut bmi = winapi::um::wingdi::BITMAPINFO {
            bmiHeader: winapi::um::wingdi::BITMAPINFOHEADER {
                biSize: std::mem::size_of::<winapi::um::wingdi::BITMAPINFOHEADER>() as u32,
                biWidth: width,
                biHeight: -height, // Top-down
                biPlanes: 1,
                biBitCount: 24, // RGB
                biCompression: winapi::um::wingdi::BI_RGB,
                biSizeImage: 0,
                biXPelsPerMeter: 0,
                biYPelsPerMeter: 0,
                biClrUsed: 0,
                biClrImportant: 0,
            },
            bmiColors: [winapi::um::wingdi::RGBQUAD { 
                rgbBlue: 0, rgbGreen: 0, rgbRed: 0, rgbReserved: 0 
            }; 1],
        };
        
        let bytes_per_line = ((width * 3 + 3) / 4) * 4; // DWORD aligned
        let buffer_size = (bytes_per_line * height) as usize;
        let mut buffer = vec![0u8; buffer_size];
        
        let screen_dc = unsafe { winapi::um::winuser::GetDC(ptr::null_mut()) };
        let lines_copied = unsafe {
            winapi::um::wingdi::GetDIBits(
                screen_dc,
                bitmap,
                0,
                height as u32,
                buffer.as_mut_ptr() as *mut _,
                &mut bmi,
                winapi::um::wingdi::DIB_RGB_COLORS,
            )
        };
        
        unsafe { winapi::um::winuser::ReleaseDC(ptr::null_mut(), screen_dc) };
        
        if lines_copied == 0 {
            println!("❌ Failed to get DIB bits!");
        } else {
            println!("✅ Got {} lines of RGB data", lines_copied);
            
            // Sample the RGB buffer
            let non_zero_bytes = buffer.iter().filter(|&&b| b != 0).count();
            println!("📊 RGB buffer: {}/{} non-zero bytes ({:.1}%)", 
                non_zero_bytes, buffer.len(),
                (non_zero_bytes as f32 / buffer.len() as f32) * 100.0);
                
            if non_zero_bytes == 0 {
                println!("❌ RGB BUFFER IS ALL ZEROS! This will cause black screen!");
                
                // Try saving raw bitmap to debug
                println!("💾 Saving debug info...");
                fs::write("debug_buffer.bin", &buffer).ok();
                println!("   Saved raw buffer to debug_buffer.bin");
                
            } else {
                println!("✅ RGB buffer has content - should work!");
                
                // Test JPEG compression
                println!("\n📸 Testing JPEG compression...");
                
                // Convert BGR to RGB
                let mut rgb_buffer = Vec::new();
                for y in 0..height {
                    let line_start = (y * bytes_per_line) as usize;
                    for x in 0..width {
                        let pixel_start = line_start + (x * 3) as usize;
                        if pixel_start + 2 < buffer.len() {
                            let b = buffer[pixel_start];
                            let g = buffer[pixel_start + 1];
                            let r = buffer[pixel_start + 2];
                            rgb_buffer.extend_from_slice(&[r, g, b]);
                        }
                    }
                }
                
                println!("🔄 Converted to RGB: {} bytes", rgb_buffer.len());
                
                let expected_size = (width * height * 3) as usize;
                if rgb_buffer.len() != expected_size {
                    println!("⚠️  RGB size mismatch: got {}, expected {}", rgb_buffer.len(), expected_size);
                }
                
                // Sample final RGB
                let rgb_non_zero = rgb_buffer.iter().filter(|&&b| b != 0).count();
                println!("📊 Final RGB: {}/{} non-zero bytes ({:.1}%)", 
                    rgb_non_zero, rgb_buffer.len(),
                    (rgb_non_zero as f32 / rgb_buffer.len() as f32) * 100.0);
                    
                if rgb_non_zero > 0 {
                    fs::write("debug_rgb.bin", &rgb_buffer).ok();
                    println!("💾 Saved RGB data to debug_rgb.bin");
                }
            }
        }
    }
    
    // Cleanup
    unsafe {
        winapi::um::wingdi::SelectObject(mem_dc, old_bitmap);
        winapi::um::wingdi::DeleteObject(bitmap as *mut _);
        winapi::um::wingdi::DeleteDC(mem_dc);
        winapi::um::winuser::ReleaseDC(window, window_dc);
    }
    
    println!("\n🎯 SUMMARY:");
    println!("If you see '❌ ALL PIXELS ARE BLACK!' → Window capture is not working");
    println!("If you see '❌ RGB BUFFER IS ALL ZEROS!' → Bitmap conversion failing");
    println!("If you see '✅ RGB buffer has content' → Problem is in JPEG compression or client display");
    println!("\nThis diagnostic will pinpoint exactly where the black screen issue occurs!");
}