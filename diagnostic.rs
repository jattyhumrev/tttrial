use std::ptr;
use std::io::Write;

fn main() {
    println!("🔍 HVNC Diagnostic Tool - Finding the Black Screen Issue");
    println!("=====================================================");
    
    // Step 1: Find notepad window
    let window = unsafe {
        winapi::um::winuser::FindWindowA(
            ptr::null(),
            "Untitled - Notepad\0".as_ptr() as *const i8
        )
    };
    
    if window.is_null() {
        println!("❌ Please start notepad first!");
        println!("Run: Start-Process notepad");
        return;
    }
    
    println!("✅ Found notepad window: {:?}", window);
    
    // Step 2: Test basic capture
    unsafe {
        let mut rect = winapi::shared::windef::RECT {
            left: 0, top: 0, right: 0, bottom: 0
        };
        
        if winapi::um::winuser::GetWindowRect(window, &mut rect) == 0 {
            println!("❌ Failed to get window rectangle");
            return;
        }
        
        let width = rect.right - rect.left;
        let height = rect.bottom - rect.top;
        
        println!("📐 Window dimensions: {}x{}", width, height);
        
        if width <= 0 || height <= 0 {
            println!("❌ Invalid window dimensions!");
            return;
        }
        
        // Test GetDC
        let dc = winapi::um::winuser::GetDC(window);
        if dc.is_null() {
            println!("❌ Failed to get window DC");
            return;
        }
        
        println!("✅ Got window DC: {:?}", dc);
        
        // Test CreateCompatibleDC
        let mem_dc = winapi::um::wingdi::CreateCompatibleDC(dc);
        if mem_dc.is_null() {
            println!("❌ Failed to create compatible DC");
            winapi::um::winuser::ReleaseDC(window, dc);
            return;
        }
        
        println!("✅ Created compatible DC: {:?}", mem_dc);
        
        // Test CreateCompatibleBitmap
        let bitmap = winapi::um::wingdi::CreateCompatibleBitmap(dc, width, height);
        if bitmap.is_null() {
            println!("❌ Failed to create compatible bitmap");
            winapi::um::wingdi::DeleteDC(mem_dc);
            winapi::um::winuser::ReleaseDC(window, dc);
            return;
        }
        
        println!("✅ Created compatible bitmap: {:?}", bitmap);
        
        // Test SelectObject
        let old_bitmap = winapi::um::wingdi::SelectObject(mem_dc, bitmap as *mut _);
        if old_bitmap.is_null() {
            println!("❌ Failed to select bitmap into DC");
        } else {
            println!("✅ Selected bitmap into DC");
        }
        
        // Test BitBlt
        let blt_result = winapi::um::wingdi::BitBlt(
            mem_dc,
            0, 0,
            width, height,
            dc,
            0, 0,
            winapi::um::wingdi::SRCCOPY,
        );
        
        if blt_result == 0 {
            let error = winapi::um::errhandlingapi::GetLastError();
            println!("❌ BitBlt failed! Error code: {}", error);
            
            // Common error codes:
            match error {
                5 => println!("   → Access Denied (ERROR_ACCESS_DENIED) - Need Administrator privileges"),
                87 => println!("   → Invalid Parameter (ERROR_INVALID_PARAMETER)"),
                1400 => println!("   → Invalid Window Handle (ERROR_INVALID_WINDOW_HANDLE)"),
                _ => println!("   → Unknown error"),
            }
        } else {
            println!("✅ BitBlt successful!");
            
            // Try to get some pixel data to verify it's not all black
            let pixel_dc = winapi::um::winuser::GetDC(ptr::null_mut());
            if !pixel_dc.is_null() {
                let pixel = winapi::um::wingdi::GetPixel(mem_dc, 10, 10);
                println!("🎨 Sample pixel at (10,10): 0x{:06X}", pixel);
                
                if pixel == 0 || pixel == 0xFFFFFF {
                    println!("⚠️  Pixel is black/white - might indicate capture issue");
                } else {
                    println!("✅ Pixel has color - capture likely working");
                }
                
                winapi::um::winuser::ReleaseDC(ptr::null_mut(), pixel_dc);
            }
        }
        
        // Test PrintWindow (Option C method)
        println!("\n🔬 Testing PrintWindow (Option C method):");
        let print_result = winapi::um::winuser::PrintWindow(
            window,
            mem_dc,
            winapi::um::winuser::PW_RENDERFULLCONTENT,
        );
        
        if print_result == 0 {
            let error = winapi::um::errhandlingapi::GetLastError();
            println!("❌ PrintWindow failed! Error code: {}", error);
        } else {
            println!("✅ PrintWindow successful!");
            
            // Check pixel after PrintWindow
            let pixel = winapi::um::wingdi::GetPixel(mem_dc, 10, 10);
            println!("🎨 PrintWindow pixel at (10,10): 0x{:06X}", pixel);
        }
        
        // Cleanup
        winapi::um::wingdi::SelectObject(mem_dc, old_bitmap);
        winapi::um::wingdi::DeleteObject(bitmap as *mut _);
        winapi::um::wingdi::DeleteDC(mem_dc);
        winapi::um::winuser::ReleaseDC(window, dc);
    }
    
    println!("\n📋 Diagnostic Summary:");
    println!("If you see ❌ BitBlt failed with Error 5 → Run as Administrator");
    println!("If you see ✅ BitBlt successful but black pixels → Capture method needs adjustment");
    println!("If you see ✅ PrintWindow successful → Option C should work with admin rights");
    
    println!("\n💡 Next steps:");
    println!("1. If BitBlt failed → Run as Administrator");
    println!("2. If capture works → The issue is in JPEG encoding/decoding"); 
    println!("3. If PrintWindow works → Option C is ready for testing");
}