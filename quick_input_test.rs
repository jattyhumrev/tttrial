//! Simple HVNC input test for port 5901
use std::net::TcpStream;
use std::io::{Write, Read};
use std::time::Duration;
use std::thread;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 Testing HVNC Input System on port 5901...");
    
    // Connect to server on port 5901
    println!("📡 Connecting to server at 127.0.0.1:5901...");
    let mut stream = TcpStream::connect("127.0.0.1:5901")?;
    println!("✅ Connected successfully!");
    
    // Test mouse move event
    let mouse_move = r#"{"MouseMove":{"x":200,"y":150}}"#;
    let mouse_move_len = (mouse_move.len() as u32).to_le_bytes();
    stream.write_all(&mouse_move_len)?;
    stream.write_all(mouse_move.as_bytes())?;
    stream.flush()?;
    println!("🖱️ Sent mouse move event: x=200, y=150");
    
    thread::sleep(Duration::from_millis(500));
    
    // Test mouse click event  
    let mouse_click = r#"{"MouseClick":{"x":200,"y":150,"button":"Left"}}"#;
    let mouse_click_len = (mouse_click.len() as u32).to_le_bytes();
    stream.write_all(&mouse_click_len)?;
    stream.write_all(mouse_click.as_bytes())?;
    stream.flush()?;
    println!("🖱️ Sent mouse click event: Left button at (200, 150)");
    
    thread::sleep(Duration::from_millis(500));
    
    // Test keyboard event
    let key_press = r#"{"KeyPress":{"keycode":72,"pressed":true}}"#; // H key down
    let key_press_len = (key_press.len() as u32).to_le_bytes();
    stream.write_all(&key_press_len)?;
    stream.write_all(key_press.as_bytes())?;
    stream.flush()?;
    println!("⌨️ Sent key press event: H key down");
    
    thread::sleep(Duration::from_millis(100));
    
    let key_release = r#"{"KeyPress":{"keycode":72,"pressed":false}}"#; // H key up
    let key_release_len = (key_release.len() as u32).to_le_bytes();
    stream.write_all(&key_release_len)?;
    stream.write_all(key_release.as_bytes())?;
    stream.flush()?;
    println!("⌨️ Sent key release event: H key up");
    
    // Keep connection alive for a few seconds
    thread::sleep(Duration::from_secs(3));
    
    println!("🎉 All input events sent successfully!");
    println!("Check the server logs to see if input events were received and processed.");
    
    Ok(())
}