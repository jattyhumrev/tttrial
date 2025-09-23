// Quick test to validate the color format fix
use minifb::{Window, WindowOptions};

fn main() {
    let width = 200;
    let height = 200;
    
    let mut window = Window::new("Color Test - Fixed Format", width, height, WindowOptions::default())
        .expect("Failed to create window");
    
    // Create test pattern with the FIXED format (0x00RRGGBB)
    let mut buffer = vec![0; width * height];
    
    for y in 0..height {
        for x in 0..width {
            let index = y * width + x;
            
            // Create a test pattern
            if x < width / 2 && y < height / 2 {
                // Red quadrant - FIXED FORMAT: (r << 16) | (g << 8) | b
                buffer[index] = (255 << 16) | (0 << 8) | 0;  // 0x00FF0000
            } else if x >= width / 2 && y < height / 2 {
                // Green quadrant
                buffer[index] = (0 << 16) | (255 << 8) | 0;  // 0x0000FF00
            } else if x < width / 2 && y >= height / 2 {
                // Blue quadrant  
                buffer[index] = (0 << 16) | (0 << 8) | 255;  // 0x000000FF
            } else {
                // White quadrant
                buffer[index] = (255 << 16) | (255 << 8) | 255;  // 0x00FFFFFF
            }
        }
    }
    
    println!("✅ Fixed Color Format Test:");
    println!("   Red:   0x{:08X} (should show as red)", (255 << 16) | (0 << 8) | 0);
    println!("   Green: 0x{:08X} (should show as green)", (0 << 16) | (255 << 8) | 0);
    println!("   Blue:  0x{:08X} (should show as blue)", (0 << 16) | (0 << 8) | 255);
    println!("   White: 0x{:08X} (should show as white)", (255 << 16) | (255 << 8) | 255);
    println!("Press ESC to close");
    
    while window.is_open() && !window.is_key_down(minifb::Key::Escape) {
        window.update_with_buffer(&buffer, width, height).unwrap();
    }
}