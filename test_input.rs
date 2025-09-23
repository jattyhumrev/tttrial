//! Simple test program to verify input events are working

use hidden_vnc::common::{InputEvent, MouseButton};
use hidden_vnc::client::network::NetworkClient;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing input event sending to HVNC server...");
    
    // Create a network client
    let mut client = NetworkClient::new_with_config(
        Duration::from_secs(10),
        3,
        Duration::from_secs(2),
    );
    
    // Connect to the server
    println!("Connecting to server at 127.0.0.1:5900...");
    match client.connect("127.0.0.1:5900").await {
        Ok(_) => {
            println!("Successfully connected to server!");
            
            // Send a simple mouse move event
            let mouse_move_event = InputEvent::MouseMove { x: 100, y: 100 };
            println!("Sending mouse move event: {:?}", mouse_move_event);
            
            match client.send_input(mouse_move_event).await {
                Ok(_) => println!("Mouse move event sent successfully!"),
                Err(e) => println!("Failed to send mouse move event: {}", e),
            }
            
            // Small delay between events
            sleep(Duration::from_millis(100)).await;
            
            // Send a mouse click event
            let mouse_click_event = InputEvent::MouseClick { 
                x: 100, 
                y: 100, 
                button: MouseButton::Left 
            };
            println!("Sending mouse click event: {:?}", mouse_click_event);
            
            match client.send_input(mouse_click_event).await {
                Ok(_) => println!("Mouse click event sent successfully!"),
                Err(e) => println!("Failed to send mouse click event: {}", e),
            }
            
            // Small delay between events
            sleep(Duration::from_millis(100)).await;
            
            // Send a key press event (e.g., pressing the 'A' key)
            let key_press_event = InputEvent::KeyPress { 
                keycode: 0x41, // 'A' key virtual keycode
                pressed: true 
            };
            println!("Sending key press event: {:?}", key_press_event);
            
            match client.send_input(key_press_event).await {
                Ok(_) => println!("Key press event sent successfully!"),
                Err(e) => println!("Failed to send key press event: {}", e),
            }
            
            // Small delay between events
            sleep(Duration::from_millis(100)).await;
            
            // Send a key release event
            let key_release_event = InputEvent::KeyPress { 
                keycode: 0x41, // 'A' key virtual keycode
                pressed: false 
            };
            println!("Sending key release event: {:?}", key_release_event);
            
            match client.send_input(key_release_event).await {
                Ok(_) => println!("Key release event sent successfully!"),
                Err(e) => println!("Failed to send key release event: {}", e),
            }
            
            // Keep the connection open for a bit to allow server to process events
            println!("Keeping connection open for 10 seconds...");
            sleep(Duration::from_secs(10)).await;
        },
        Err(e) => {
            println!("Failed to connect to server: {}", e);
        }
    }
    
    println!("Test completed.");
    Ok(())
}