use hidden_vnc::host::capture::compress_to_jpeg;
use hidden_vnc::common::Frame;
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 Testing JPEG compression with debug RGB data...");
    
    // Read the RGB data from our debug pipeline
    let rgb_data = fs::read("debug_rgb.bin")?;
    println!("📖 Read {} bytes of RGB data", rgb_data.len());
    
    // Expected dimensions from our debug: 1439x654
    let width = 1439;
    let height = 654;
    let expected_size = (width * height * 3) as usize;
    
    if rgb_data.len() != expected_size {
        println!("❌ Size mismatch! Expected {}, got {}", expected_size, rgb_data.len());
        return Ok(());
    }
    
    println!("✅ RGB data size is correct");
    
    // Test JPEG compression
    println!("🔄 Compressing to JPEG...");
    let jpeg_data = compress_to_jpeg(&rgb_data, width, height, 75)?;
    
    println!("📊 JPEG compression results:");
    println!("  Input RGB: {} bytes", rgb_data.len());
    println!("  Output JPEG: {} bytes", jpeg_data.len());
    println!("  Compression ratio: {:.1}x", rgb_data.len() as f32 / jpeg_data.len() as f32);
    
    // Check JPEG header
    if jpeg_data.len() >= 2 {
        println!("  JPEG header: {:02X} {:02X}", jpeg_data[0], jpeg_data[1]);
        if jpeg_data[0] == 0xFF && jpeg_data[1] == 0xD8 {
            println!("✅ Valid JPEG header");
        } else {
            println!("❌ Invalid JPEG header!");
        }
    }
    
    // Save JPEG for inspection
    fs::write("test_output.jpg", &jpeg_data)?;
    println!("💾 Saved JPEG to test_output.jpg");
    
    // Test decompression
    println!("🔄 Testing JPEG decompression...");
    match hidden_vnc::host::capture::decompress_from_jpeg(&jpeg_data) {
        Ok((decompressed_rgb, dec_width, dec_height)) => {
            println!("✅ JPEG decompression successful");
            println!("  Decompressed: {} bytes ({}x{})", decompressed_rgb.len(), dec_width, dec_height);
            
            // Check for non-zero content
            let non_zero_bytes = decompressed_rgb.iter().filter(|&&b| b != 0).count();
            let content_percentage = (non_zero_bytes as f32 / decompressed_rgb.len() as f32) * 100.0;
            
            println!("  Content: {:.1}% non-black pixels", content_percentage);
            
            if content_percentage > 0.0 {
                println!("✅ Decompressed JPEG contains visible content!");
            } else {
                println!("❌ Decompressed JPEG is all black!");
            }
            
            // Save decompressed RGB for comparison
            fs::write("test_decompressed.bin", &decompressed_rgb)?;
            println!("💾 Saved decompressed RGB to test_decompressed.bin");
        }
        Err(e) => {
            println!("❌ JPEG decompression failed: {}", e);
        }
    }
    
    println!("\n🎯 SUMMARY:");
    println!("If JPEG compression/decompression works here but client shows black screen,");
    println!("the issue is in the client display pipeline, not the compression.");
    
    Ok(())
}