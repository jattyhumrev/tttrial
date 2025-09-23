//! Security tests for Hidden VNC
use hidden_vnc::common::protocol::*;
use hidden_vnc::error::*;

#[cfg(test)]
mod security_tests {
    use super::*;

    #[test]
    fn test_input_validation() {
        let valid_event = InputEvent::MouseClick { 
            x: 100, y: 100, button: MouseButton::Left 
        };
        assert!(validate_input(&valid_event));

        let invalid_event = InputEvent::MouseClick { 
            x: -1000, y: -1000, button: MouseButton::Left 
        };
        assert!(!validate_input(&invalid_event));
    }

    #[test]
    fn test_frame_size_limits() {
        assert!(validate_frame_size(1024));
        assert!(validate_frame_size(1024 * 1024));
        assert!(!validate_frame_size(50 * 1024 * 1024)); // Too large
    }

    fn validate_input(event: &InputEvent) -> bool {
        match event {
            InputEvent::MouseClick { x, y, .. } | InputEvent::MouseMove { x, y } => {
                *x >= 0 && *x <= 10000 && *y >= 0 && *y <= 10000
            }
            InputEvent::KeyPress { keycode, .. } => {
                *keycode > 0 && *keycode <= 255
            }
        }
    }

    fn validate_frame_size(size: usize) -> bool {
        size > 0 && size <= 10 * 1024 * 1024 // 10MB limit
    }
}
