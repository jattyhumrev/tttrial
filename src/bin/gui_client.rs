//! GUI Client Binary for Hidden VNC
//! 
//! This provides a user-friendly graphical interface for connecting to
//! Hidden VNC hosts with application selection and quality controls.

use hidden_vnc::{
    client::{GuiClient, ApplicationType, QualitySettings, HostInfo, SystemApp},
    Result, HvncError,
};
use std::io::{self, Write};
use log::{info, error};
use clap::{Arg, Command};

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command line arguments
    let matches = Command::new("hvnc-gui-client")
        .version("0.1.0")
        .author("Hidden VNC Team")
        .about("Hidden VNC GUI Client")
        .arg(
            Arg::new("verbose")
                .short('v')
                .long("verbose")
                .help("Enable verbose logging")
                .action(clap::ArgAction::SetTrue)
        )
        .get_matches();

    // Initialize logging
    let log_level = if matches.get_flag("verbose") { "debug" } else { "info" };
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(log_level)).init();

    info!("Starting Hidden VNC GUI Client v0.1.0");

    // Create GUI client
    let gui_client = GuiClient::new();

    // Start host discovery
    gui_client.start_host_discovery()?;

    // Run main GUI loop
    run_gui_interface(gui_client).await
}

/// Run the main GUI interface (console-based for now)
async fn run_gui_interface(gui_client: GuiClient) -> Result<()> {
    println!("\n=== Hidden VNC GUI Client ===");
    println!("Discovering hosts...");
    
    // Wait a moment for host discovery
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    loop {
        // Main menu
        println!("\n--- Main Menu ---");
        println!("1. View Available Hosts");
        println!("2. Connect to Host");
        println!("3. View Active Connections");
        println!("4. Disconnect from Host");
        println!("5. Quality Settings");
        println!("6. Exit");
        print!("Select option (1-6): ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        let choice = input.trim();

        match choice {
            "1" => show_available_hosts(&gui_client),
            "2" => connect_to_host(&gui_client).await?,
            "3" => show_active_connections(&gui_client),
            "4" => disconnect_from_host(&gui_client)?,
            "5" => show_quality_settings(&gui_client),
            "6" => {
                println!("Goodbye!");
                break;
            }
            _ => println!("Invalid option. Please try again."),
        }
    }

    Ok(())
}

/// Show available hosts
fn show_available_hosts(gui_client: &GuiClient) {
    println!("\n--- Available Hosts ---");
    let hosts = gui_client.get_discovered_hosts();
    
    if hosts.is_empty() {
        println!("No hosts discovered yet. Please wait...");
        return;
    }

    for (i, host) in hosts.iter().enumerate() {
        println!("{}. {} ({}:{})", 
                 i + 1, 
                 host.name, 
                 host.address, 
                 host.port);
        println!("   Status: {:?}", host.status);
        println!("   Username: {}", host.username);
        println!("   Applications: {} available", host.applications.len());
    }
}

/// Connect to a host
async fn connect_to_host(gui_client: &GuiClient) -> Result<()> {
    let hosts = gui_client.get_discovered_hosts();
    
    if hosts.is_empty() {
        println!("No hosts available. Please wait for discovery...");
        return Ok(());
    }

    // Select host
    println!("\n--- Select Host ---");
    for (i, host) in hosts.iter().enumerate() {
        println!("{}. {} ({})", i + 1, host.name, host.address);
    }
    print!("Select host (1-{}): ", hosts.len());
    io::stdout().flush().unwrap();

    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    let host_index: usize = input.trim().parse().unwrap_or(0);
    
    if host_index == 0 || host_index > hosts.len() {
        println!("Invalid host selection.");
        return Ok(());
    }

    let selected_host = &hosts[host_index - 1];
    
    // Select application
    let application = select_application(gui_client, &selected_host.address)?;
    
    // Select quality
    let quality = select_quality_settings(gui_client)?;

    // Connect
    println!("\nConnecting to {} with {:?}...", selected_host.name, application);
    
    match gui_client.connect_to_host(&selected_host.address, application, quality) {
        Ok(connection_id) => {
            println!("✓ Successfully connected! Connection ID: {}", connection_id);
            println!("The remote application should now be available.");
        }
        Err(e) => {
            println!("✗ Connection failed: {}", e);
        }
    }

    Ok(())
}

/// Select application to launch
fn select_application(gui_client: &GuiClient, host_id: &str) -> Result<ApplicationType> {
    println!("\n--- Select Application ---");
    
    let applications = gui_client.get_available_applications(host_id);
    
    // Show predefined applications
    println!("Browsers:");
    println!("1. Chrome");
    println!("2. Firefox");
    println!("3. Edge");
    
    println!("\nSystem Applications:");
    println!("4. Notepad");
    println!("5. Calculator");
    println!("6. Paint");
    println!("7. WordPad");
    println!("8. File Explorer");
    
    println!("\nCustom:");
    println!("9. Custom Application");
    
    print!("Select application (1-9): ");
    io::stdout().flush().unwrap();

    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    let choice: u32 = input.trim().parse().unwrap_or(0);

    let mut application = match choice {
        1 => ApplicationType::Chrome { incognito: None, url: None },
        2 => ApplicationType::Firefox { private: None, url: None },
        3 => ApplicationType::Edge { inprivate: None, url: None },
        4 => ApplicationType::System(SystemApp::Notepad),
        5 => ApplicationType::System(SystemApp::Calculator),
        6 => ApplicationType::System(SystemApp::Paint),
        7 => ApplicationType::System(SystemApp::WordPad),
        8 => ApplicationType::System(SystemApp::Explorer),
        9 => {
            // Custom application
            print!("Enter application path: ");
            io::stdout().flush().unwrap();
            let mut path = String::new();
            io::stdin().read_line(&mut path).unwrap();
            let path = path.trim().to_string();
            
            print!("Enter arguments (optional): ");
            io::stdout().flush().unwrap();
            let mut args_input = String::new();
            io::stdin().read_line(&mut args_input).unwrap();
            let args: Vec<String> = if args_input.trim().is_empty() {
                vec![]
            } else {
                args_input.trim().split_whitespace().map(|s| s.to_string()).collect()
            };
            
            ApplicationType::Custom { path, args }
        }
        _ => {
            println!("Invalid selection, using Notepad as default.");
            ApplicationType::System(SystemApp::Notepad)
        }
    };

    // Ask for private/incognito mode and URL if it's a browser
    application = match application {
        ApplicationType::Chrome { .. } => {
            print!("Use Incognito mode? (y/n) [n]: ");
            io::stdout().flush().unwrap();
            let mut incognito_input = String::new();
            io::stdin().read_line(&mut incognito_input).unwrap();
            let incognito = if incognito_input.trim().to_lowercase() == "y" {
                Some(true)
            } else {
                None
            };

            print!("Enter URL (optional): ");
            io::stdout().flush().unwrap();
            let mut url_input = String::new();
            io::stdin().read_line(&mut url_input).unwrap();
            let url = if url_input.trim().is_empty() {
                None
            } else {
                Some(url_input.trim().to_string())
            };

            ApplicationType::Chrome { incognito, url }
        }
        ApplicationType::Firefox { .. } => {
            print!("Use Private mode? (y/n) [n]: ");
            io::stdout().flush().unwrap();
            let mut private_input = String::new();
            io::stdin().read_line(&mut private_input).unwrap();
            let private = if private_input.trim().to_lowercase() == "y" {
                Some(true)
            } else {
                None
            };

            print!("Enter URL (optional): ");
            io::stdout().flush().unwrap();
            let mut url_input = String::new();
            io::stdin().read_line(&mut url_input).unwrap();
            let url = if url_input.trim().is_empty() {
                None
            } else {
                Some(url_input.trim().to_string())
            };

            ApplicationType::Firefox { private, url }
        }
        ApplicationType::Edge { .. } => {
            print!("Use InPrivate mode? (y/n) [n]: ");
            io::stdout().flush().unwrap();
            let mut inprivate_input = String::new();
            io::stdin().read_line(&mut inprivate_input).unwrap();
            let inprivate = if inprivate_input.trim().to_lowercase() == "y" {
                Some(true)
            } else {
                None
            };

            print!("Enter URL (optional): ");
            io::stdout().flush().unwrap();
            let mut url_input = String::new();
            io::stdin().read_line(&mut url_input).unwrap();
            let url = if url_input.trim().is_empty() {
                None
            } else {
                Some(url_input.trim().to_string())
            };

            ApplicationType::Edge { inprivate, url }
        }
        _ => application,
    };

    Ok(application)
}

/// Select quality settings
fn select_quality_settings(gui_client: &GuiClient) -> Result<QualitySettings> {
    println!("\n--- Quality Settings ---");
    
    let presets = gui_client.get_quality_presets();
    let preset_names: Vec<&String> = presets.keys().collect();
    
    for (i, name) in preset_names.iter().enumerate() {
        let preset = presets.get(*name).unwrap();
        println!("{}. {} (Quality: {}, FPS: {}, Scale: {})", 
                 i + 1, 
                 name, 
                 preset.jpeg_quality, 
                 preset.frame_rate, 
                 preset.scale_factor);
    }
    
    println!("{}. Custom Settings", preset_names.len() + 1);
    
    print!("Select quality preset (1-{}): ", preset_names.len() + 1);
    io::stdout().flush().unwrap();

    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    let choice: usize = input.trim().parse().unwrap_or(0);

    if choice > 0 && choice <= preset_names.len() {
        let preset_name = preset_names[choice - 1];
        Ok(presets.get(preset_name).unwrap().clone())
    } else if choice == preset_names.len() + 1 {
        // Custom settings
        println!("\n--- Custom Quality Settings ---");
        
        print!("JPEG Quality (1-100) [80]: ");
        io::stdout().flush().unwrap();
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        let jpeg_quality: u8 = input.trim().parse().unwrap_or(80).clamp(1, 100);
        
        print!("Frame Rate (1-120) [30]: ");
        io::stdout().flush().unwrap();
        input.clear();
        io::stdin().read_line(&mut input).unwrap();
        let frame_rate: u32 = input.trim().parse().unwrap_or(30).clamp(1, 120);
        
        print!("Scale Factor (0.1-2.0) [1.0]: ");
        io::stdout().flush().unwrap();
        input.clear();
        io::stdin().read_line(&mut input).unwrap();
        let scale_factor: f32 = input.trim().parse().unwrap_or(1.0_f32).clamp(0.1, 2.0);
        
        Ok(QualitySettings {
            jpeg_quality,
            frame_rate,
            scale_factor,
            bandwidth_limit: None,
        })
    } else {
        println!("Invalid selection, using balanced preset.");
        Ok(presets.get("Balanced").unwrap().clone())
    }
}

/// Show active connections
fn show_active_connections(gui_client: &GuiClient) {
    println!("\n--- Active Connections ---");
    let connections = gui_client.get_active_connections();
    
    if connections.is_empty() {
        println!("No active connections.");
        return;
    }

    for (i, connection_id) in connections.iter().enumerate() {
        println!("{}. Connection: {}", i + 1, connection_id);
    }
}

/// Disconnect from host
fn disconnect_from_host(gui_client: &GuiClient) -> Result<()> {
    let connections = gui_client.get_active_connections();
    
    if connections.is_empty() {
        println!("No active connections to disconnect.");
        return Ok(());
    }

    println!("\n--- Disconnect from Host ---");
    for (i, connection_id) in connections.iter().enumerate() {
        println!("{}. {}", i + 1, connection_id);
    }
    
    print!("Select connection to disconnect (1-{}): ", connections.len());
    io::stdout().flush().unwrap();

    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    let choice: usize = input.trim().parse().unwrap_or(0);
    
    if choice > 0 && choice <= connections.len() {
        let connection_id = &connections[choice - 1];
        match gui_client.disconnect_from_host(connection_id) {
            Ok(()) => println!("✓ Successfully disconnected from {}", connection_id),
            Err(e) => println!("✗ Disconnect failed: {}", e),
        }
    } else {
        println!("Invalid selection.");
    }

    Ok(())
}

/// Show quality settings information
fn show_quality_settings(gui_client: &GuiClient) {
    println!("\n--- Quality Settings Information ---");
    
    let presets = gui_client.get_quality_presets();
    
    for (name, settings) in presets {
        println!("\n{}:", name);
        println!("  JPEG Quality: {} (1-100, higher = better quality)", settings.jpeg_quality);
        println!("  Frame Rate: {} FPS (higher = smoother)", settings.frame_rate);
        println!("  Scale Factor: {}x (higher = larger display)", settings.scale_factor);
        if let Some(bandwidth) = settings.bandwidth_limit {
            println!("  Bandwidth Limit: {} KB/s", bandwidth);
        } else {
            println!("  Bandwidth Limit: Unlimited");
        }
    }
    
    println!("\nRecommendations:");
    println!("- High Quality: Best for fast networks and detailed work");
    println!("- Balanced: Good compromise for most situations");
    println!("- Low Bandwidth: Best for slow connections");
    println!("- Mobile: Optimized for mobile/cellular connections");
}