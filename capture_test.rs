use std::ptr;

fn main() {
    println!("🔬 Professional HVNC Capture Method Test");
    println!("========================================");
    
    // Find notepad window
    let window = unsafe {
        winapi::um::winuser::FindWindowA(
            ptr::null(),
            "Untitled - Notepad\0".as_ptr() as *const i8
        )
    };
    
    if window.is_null() {
        println!("❌ Notepad window not found. Please start notepad and try again.");
        return;
    }
    
    println!("✅ Found notepad window: {:?}", window);
    
    // Test window validation (like professional implementations do)
    unsafe {
        let is_window = winapi::um::winuser::IsWindow(window);
        let is_visible = winapi::um::winuser::IsWindowVisible(window);
        
        println!("📊 Window validation:");
        println!("   - IsWindow: {}", is_window != 0);
        println!("   - IsVisible: {}", is_visible != 0);
        
        // Get window rectangle (dimensions)
        let mut rect = winapi::shared::windef::RECT {
            left: 0, top: 0, right: 0, bottom: 0
        };
        
        if winapi::um::winuser::GetWindowRect(window, &mut rect) != 0 {
            let width = rect.right - rect.left;
            let height = rect.bottom - rect.top;
            println!("   - Dimensions: {}x{}", width, height);
            println!("   - Position: ({}, {})", rect.left, rect.top);
        }
        
        // Get process ID (like RustDesk/VenomRAT do)
        let mut process_id = 0u32;
        winapi::um::winuser::GetWindowThreadProcessId(window, &mut process_id);
        println!("   - Process ID: {}", process_id);
        
        // Test window title extraction (like EduContin does)
        let mut window_title = [0u8; 256];
        let title_len = winapi::um::winuser::GetWindowTextA(
            window, 
            window_title.as_mut_ptr() as *mut i8, 
            window_title.len() as i32
        );
        
        if title_len > 0 {
            let title = String::from_utf8_lossy(&window_title[..title_len as usize]);
            println!("   - Title: '{}'", title);
        }
    }
    
    // Test capture prerequisites  
    println!("\n🔧 Testing capture prerequisites:");
    
    unsafe {
        // Test GetDC (basic)
        let dc = winapi::um::winuser::GetDC(window);
        if !dc.is_null() {
            println!("✅ GetDC: Success");
            winapi::um::winuser::ReleaseDC(window, dc);
        } else {
            println!("❌ GetDC: Failed");
        }
        
        // Test GetWindowDC (enhanced)
        let window_dc = winapi::um::winuser::GetWindowDC(window);
        if !window_dc.is_null() {
            println!("✅ GetWindowDC: Success");
            winapi::um::winuser::ReleaseDC(window, window_dc);
        } else {
            println!("❌ GetWindowDC: Failed");
        }
        
        // Test CreateCompatibleDC (memory)
        let screen_dc = winapi::um::winuser::GetDC(ptr::null_mut());
        if !screen_dc.is_null() {
            let mem_dc = winapi::um::wingdi::CreateCompatibleDC(screen_dc);
            if !mem_dc.is_null() {
                println!("✅ CreateCompatibleDC: Success");
                winapi::um::wingdi::DeleteDC(mem_dc);
            } else {
                println!("❌ CreateCompatibleDC: Failed");
            }
            winapi::um::winuser::ReleaseDC(ptr::null_mut(), screen_dc);
        }
    }
    
    println!("\n🚀 Professional methods would now attempt:");
    println!("   1. PrintWindow with PW_RENDERFULLCONTENT");
    println!("   2. Desktop context switching with BitBlt");
    println!("   3. Enhanced DC with show/hide sequence");
    println!("   4. Multi-attempt retry with different strategies");
    
    println!("\n⚠️  Note: Actual capture requires Administrator privileges");
    println!("   The professional methods are integrated and ready!");
}