use std::fs;
use image::GenericImageView;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Comparing JPEG data to find the difference...");
    
    // Read our working test JPEG (from test_output.jpg)
    if let Ok(working_jpeg) = fs::read("test_output.jpg") {
        println!("✅ Working JPEG: {} bytes", working_jpeg.len());
        
        // Check header
        if working_jpeg.len() >= 2 {
            println!("   Header: {:02X} {:02X}", working_jpeg[0], working_jpeg[1]);
        }
        
        // Check if we can decode it
        match image::load_from_memory(&working_jpeg) {
            Ok(img) => {
                let (width, height) = img.dimensions();
                println!("   Decoded: {}x{}", width, height);
                
                // Convert to RGB and check content
                let rgb_img = img.to_rgb8();
                let rgb_data = rgb_img.into_raw();
                let non_zero = rgb_data.iter().filter(|&&b| b != 0).count();
                let content_pct = (non_zero as f32 / rgb_data.len() as f32) * 100.0;
                println!("   Content: {:.1}% non-black pixels", content_pct);
                
                // Save a test pattern to see what the client should receive
                create_test_pattern_jpeg(width, height)?;
            }
            Err(e) => {
                println!("❌ Failed to decode working JPEG: {}", e);
            }
        }
    } else {
        println!("❌ Working JPEG (test_output.jpg) not found");
        println!("   Run the JPEG compression test first");
    }
    
    // Create a simple test pattern that should be clearly visible
    create_simple_test_pattern()?;
    
    println!("\n🎯 Test patterns created:");
    println!("   - simple_test.jpg: High contrast pattern");
    println!("   - Use these to test if client can display known content");
    
    Ok(())
}

fn create_test_pattern_jpeg(width: u32, height: u32) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🎨 Creating test pattern JPEG...");
    
    let mut rgb_data = Vec::with_capacity((width * height * 3) as usize);
    
    // Create a very obvious test pattern
    for y in 0..height {
        for x in 0..width {
            if (x / 50 + y / 50) % 2 == 0 {
                // Red squares
                rgb_data.extend_from_slice(&[255, 0, 0]);
            } else {
                // White squares  
                rgb_data.extend_from_slice(&[255, 255, 255]);
            }
        }
    }
    
    // Compress to JPEG with high quality
    let img = image::ImageBuffer::<image::Rgb<u8>, _>::from_raw(width, height, rgb_data)
        .ok_or("Failed to create image buffer")?;
    
    img.save("test_pattern.jpg")?;
    println!("✅ Saved test_pattern.jpg");
    
    Ok(())
}

fn create_simple_test_pattern() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎨 Creating simple test pattern...");
    
    let width = 400;
    let height = 300;
    let mut rgb_data = Vec::with_capacity((width * height * 3) as usize);
    
    // Create a very simple and obvious pattern
    for y in 0..height {
        for x in 0..width {
            if x < width / 2 {
                if y < height / 2 {
                    // Top-left: Red
                    rgb_data.extend_from_slice(&[255, 0, 0]);
                } else {
                    // Bottom-left: Green
                    rgb_data.extend_from_slice(&[0, 255, 0]);
                }
            } else {
                if y < height / 2 {
                    // Top-right: Blue
                    rgb_data.extend_from_slice(&[0, 0, 255]);
                } else {
                    // Bottom-right: White
                    rgb_data.extend_from_slice(&[255, 255, 255]);
                }
            }
        }
    }
    
    // Save as JPEG
    let img = image::ImageBuffer::<image::Rgb<u8>, _>::from_raw(width, height, rgb_data)
        .ok_or("Failed to create image buffer")?;
    
    img.save("simple_test.jpg")?;
    
    // Also test our compression function
    if let Ok(jpeg_data) = hidden_vnc::host::capture::compress_to_jpeg(&img.clone().into_raw(), width, height, 75) {
        fs::write("simple_test_compressed.jpg", &jpeg_data)?;
        println!("✅ Created simple_test.jpg ({} bytes)", jpeg_data.len());
        
        // Test decompression
        match hidden_vnc::host::capture::decompress_from_jpeg(&jpeg_data) {
            Ok((rgb, w, h)) => {
                let non_zero = rgb.iter().filter(|&&b| b != 0).count();
                let content_pct = (non_zero as f32 / rgb.len() as f32) * 100.0;
                println!("   Compression test: {}x{}, {:.1}% content", w, h, content_pct);
            }
            Err(e) => {
                println!("❌ Compression test failed: {}", e);
            }
        }
    } else {
        println!("❌ Failed to compress simple test pattern");
    }
    
    Ok(())
}