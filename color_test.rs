use minifb::{Window, WindowOptions};

fn main() {
    let width = 400;
    let height = 400;
    let mut buffer: Vec<u32> = vec![0; width * height];
    
    // Test different color formats
    println!("Testing color formats...");
    
    // Try 0xRRGGBBAA format
    buffer[0] = 0xFF0000FF; // Red in RRGGBBAA format
    println!("0xFF0000FF should be red in RRGGBBAA format");
    
    // Try 0xAARRGGBB format
    buffer[1] = 0xFFFF0000; // Red in AARRGGBB format
    println!("0xFFFF0000 should be red in AARRGGBB format");
    
    // Try 0xRRGGBB format with implicit alpha
    buffer[2] = 0x00FF00FF; // Green in RRGGBBAA format
    println!("0x00FF00FF should be green in RRGGBBAA format");
    
    let mut window = Window::new(
        "Color Format Test",
        width,
        height,
        WindowOptions::default(),
    ).unwrap();
    
    window.update_with_buffer(&buffer, width, height).unwrap();
    
    println!("Check the window to see which format displays correctly.");
    println!("Press any key to exit...");
    
    while window.is_open() {
        window.update();
    }
}