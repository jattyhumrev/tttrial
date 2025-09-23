# Hidden
 VNC Developer Guide

## Architecture Overview

Hidden VNC is built using Rust and follows a modular architecture with clear separation between host and client components.

### High-Level Architecture

```
┌─────────────────┐    SSH Tunnel    ┌─────────────────┐
│   HVNC Client   │◄─────────────────►│   HVNC Host     │
│                 │                   │                 │
│ ┌─────────────┐ │                   │ ┌─────────────┐ │
│ │   Display   │ │                   │ │   Capture   │ │
│ │   Window    │ │                   │ │   Engine    │ │
│ └─────────────┘ │                   │ └─────────────┘ │
│ ┌─────────────┐ │                   │ ┌─────────────┐ │
│ │   Input     │ │                   │ │   Input     │ │
│ │  Capture    │ │                   │ │  Injection  │ │
│ └─────────────┘ │                   │ └─────────────┘ │
│ ┌─────────────┐ │                   │ ┌─────────────┐ │
│ │   Network   │ │                   │ │   Network   │ │
│ │   Client    │ │                   │ │   Server    │ │
│ └─────────────┘ │                   │ └─────────────┘ │
└─────────────────┘                   └─────────────────┘
                                      ┌─────────────────┐
                                      │ Hidden Desktop  │
                                      │ ┌─────────────┐ │
                                      │ │ Target App  │ │
                                      │ └─────────────┘ │
                                      └─────────────────┘
```

### Core Components

#### Host Side Components
- **Desktop Manager**: Creates and manages hidden Windows desktops
- **Application Launcher**: Launches target applications in hidden desktop
- **Window Capture**: Captures application window content
- **Input Processor**: Handles remote input events
- **Network Server**: Manages client connections and data streaming

#### Client Side Components
- **Network Client**: Connects to host server via SSH tunnel
- **Display Manager**: Renders remote desktop content
- **Input Capture**: Captures local user input
- **Event Processor**: Processes and forwards input events

## Module Structure

### Project Layout
```
hidden-vnc/
├── src/
│   ├── bin/
│   │   ├── server.rs          # Host server binary
│   │   └── client.rs          # Client binary
│   ├── host/                  # Host-side modules
│   │   ├── desktop.rs         # Hidden desktop management
│   │   ├── launcher.rs        # Application launching
│   │   ├── capture.rs         # Window capture
│   │   ├── input.rs           # Input processing
│   │   └── server.rs          # Network server
│   ├── client/                # Client-side modules
│   │   ├── display.rs         # Display management
│   │   ├── input.rs           # Input capture
│   │   └── network.rs         # Network client
│   ├── common/                # Shared modules
│   │   ├── protocol.rs        # Network protocol definitions
│   │   └── config.rs          # Configuration structures
│   ├── error.rs               # Error handling
│   └── monitoring.rs          # Performance monitoring
├── tests/                     # Test modules
├── docs/                      # Documentation
└── Cargo.toml                 # Project configuration
```

## Key APIs and Interfaces

### Desktop Management API

```rust
// src/host/desktop.rs
pub struct DesktopManager {
    desktop_handle: HDESK,
    desktop_name: String,
}

impl DesktopManager {
    /// Creates a new hidden desktop
    pub fn create_hidden_desktop(name: &str) -> Result<Self>;
    
    /// Switches to the hidden desktop
    pub fn switch_to_desktop(&self) -> Result<()>;
    
    /// Cleans up desktop resources
    pub fn cleanup(&mut self) -> Result<()>;
}
```

### Application Launcher API

```rust
// src/host/launcher.rs
pub struct ApplicationLauncher {
    desktop_handle: HDESK,
}

impl ApplicationLauncher {
    /// Launches application in specified desktop
    pub fn launch_in_desktop(
        &self,
        app_path: &str,
        args: Option<&str>,
        desktop: &DesktopManager,
    ) -> Result<ProcessInfo>;
    
    /// Monitors application status
    pub fn is_running(&self, process_info: &ProcessInfo) -> bool;
}
```

### Window Capture API

```rust
// src/host/capture.rs
pub struct WindowCapture {
    target_window: HWND,
    capture_rect: RECT,
}

impl WindowCapture {
    /// Finds target window by title or process
    pub fn find_window(criteria: &WindowCriteria) -> Result<Self>;
    
    /// Captures window content as RGB buffer
    pub fn capture_frame(&self) -> Result<Frame>;
    
    /// Compresses frame to JPEG
    pub fn compress_frame(&self, frame: &Frame, quality: u8) -> Result<Vec<u8>>;
}
```

### Network Protocol API

```rust
// src/common/protocol.rs
#[derive(Serialize, Deserialize)]
pub enum InputEvent {
    MouseClick { x: i32, y: i32, button: MouseButton },
    MouseMove { x: i32, y: i32 },
    KeyPress { keycode: u32, pressed: bool },
}

pub struct Frame {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}
```

## Development Setup

### Prerequisites

1. **Rust Toolchain**
   ```bash
   # Install Rust
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   
   # Windows-specific targets
   rustup target add x86_64-pc-windows-msvc
   ```

2. **Development Tools**
   ```bash
   # Code formatting
   rustup component add rustfmt
   
   # Linting
   rustup component add clippy
   
   # Documentation generation
   cargo install cargo-doc
   ```

3. **Windows SDK** (for host development)
   - Visual Studio Build Tools
   - Windows 10/11 SDK

### Building the Project

```bash
# Clone repository
git clone https://github.com/your-org/hidden-vnc.git
cd hidden-vnc

# Build debug version
cargo build

# Build release version
cargo build --release

# Build specific binary
cargo build --bin hvnc-server
cargo build --bin hvnc-client
```

### Running Tests

```bash
# Run all tests
cargo test

# Run specific test module
cargo test integration_tests

# Run with output
cargo test -- --nocapture

# Run performance benchmarks
cargo bench
```

## Code Style and Standards

### Rust Style Guidelines

1. **Formatting**: Use `rustfmt` for consistent formatting
   ```bash
   cargo fmt
   ```

2. **Linting**: Use `clippy` for code quality
   ```bash
   cargo clippy -- -D warnings
   ```

3. **Documentation**: Document all public APIs
   ```rust
   /// Creates a new hidden desktop with the specified name.
   /// 
   /// # Arguments
   /// * `name` - The name for the hidden desktop
   /// 
   /// # Returns
   /// * `Result<DesktopManager>` - Success or error
   /// 
   /// # Examples
   /// ```
   /// let desktop = DesktopManager::create_hidden_desktop("test")?;
   /// ```
   pub fn create_hidden_desktop(name: &str) -> Result<DesktopManager> {
       // Implementation
   }
   ```

### Error Handling Standards

```rust
// Use custom error types
use crate::error::{HvncError, Result};

// Provide context for errors
fn example_function() -> Result<()> {
    some_operation()
        .map_err(|e| HvncError::OperationFailed(format!("Context: {}", e)))?;
    Ok(())
}

// Use proper error propagation
fn another_function() -> Result<String> {
    let result = risky_operation()?;
    Ok(result.to_string())
}
```

### Logging Standards

```rust
use log::{debug, info, warn, error};

// Use appropriate log levels
fn process_frame() -> Result<()> {
    debug!("Starting frame processing");
    
    match capture_window() {
        Ok(frame) => {
            info!("Captured frame: {}x{}", frame.width, frame.height);
            Ok(())
        }
        Err(e) => {
            error!("Frame capture failed: {}", e);
            Err(e)
        }
    }
}
```

## Testing Guidelines

### Unit Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_desktop_creation() {
        let desktop = DesktopManager::create_hidden_desktop("test_desktop");
        assert!(desktop.is_ok());
        
        let mut desktop = desktop.unwrap();
        assert!(desktop.cleanup().is_ok());
    }
    
    #[tokio::test]
    async fn test_async_operation() {
        let result = async_function().await;
        assert!(result.is_ok());
    }
}
```

### Integration Testing

```rust
// tests/integration_tests.rs
use hidden_vnc::*;

#[tokio::test]
async fn test_full_workflow() {
    // Setup
    let desktop = DesktopManager::create_hidden_desktop("integration_test")?;
    let launcher = ApplicationLauncher::new(&desktop);
    
    // Test application launch
    let process = launcher.launch_in_desktop("notepad.exe", None, &desktop)?;
    assert!(launcher.is_running(&process));
    
    // Test window capture
    let capture = WindowCapture::find_window(&WindowCriteria::ProcessId(process.id))?;
    let frame = capture.capture_frame()?;
    assert!(frame.width > 0 && frame.height > 0);
    
    // Cleanup
    process.terminate()?;
    desktop.cleanup()?;
}
```

### Performance Testing

```rust
// tests/performance_tests.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_frame_compression(c: &mut Criterion) {
    let frame = generate_test_frame(1920, 1080);
    
    c.bench_function("frame_compression", |b| {
        b.iter(|| {
            let compressed = compress_frame(black_box(&frame), 80);
            black_box(compressed)
        })
    });
}

criterion_group!(benches, bench_frame_compression);
criterion_main!(benches);
```

## Contributing Guidelines

### Development Workflow

1. **Fork and Clone**
   ```bash
   git clone https://github.com/your-username/hidden-vnc.git
   cd hidden-vnc
   git remote add upstream https://github.com/original-org/hidden-vnc.git
   ```

2. **Create Feature Branch**
   ```bash
   git checkout -b feature/new-feature
   ```

3. **Development Process**
   ```bash
   # Make changes
   # Run tests
   cargo test
   
   # Check formatting
   cargo fmt --check
   
   # Run linting
   cargo clippy -- -D warnings
   
   # Update documentation
   cargo doc --no-deps
   ```

4. **Commit and Push**
   ```bash
   git add .
   git commit -m "feat: add new feature description"
   git push origin feature/new-feature
   ```

5. **Create Pull Request**
   - Provide clear description
   - Include test results
   - Reference related issues

### Commit Message Format

```
type(scope): description

[optional body]

[optional footer]
```

Types:
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation changes
- `style`: Code style changes
- `refactor`: Code refactoring
- `test`: Test additions/changes
- `chore`: Build/tooling changes

Examples:
```
feat(capture): add support for multi-monitor capture
fix(network): resolve connection timeout issues
docs(api): update desktop management documentation
```

## Deployment and Build Instructions

### Release Build Process

1. **Version Update**
   ```toml
   # Cargo.toml
   [package]
   version = "1.0.1"
   ```

2. **Build Release Binaries**
   ```bash
   # Windows host binary
   cargo build --release --bin hvnc-server
   
   # Cross-platform client binary
   cargo build --release --bin hvnc-client --target x86_64-pc-windows-msvc
   cargo build --release --bin hvnc-client --target x86_64-unknown-linux-gnu
   cargo build --release --bin hvnc-client --target x86_64-apple-darwin
   ```

3. **Run Full Test Suite**
   ```bash
   cargo test --release
   cargo bench
   ```

4. **Generate Documentation**
   ```bash
   cargo doc --no-deps --release
   ```

### Packaging

```bash
# Create release directory
mkdir release/v1.0.1

# Copy binaries
cp target/release/hvnc-server.exe release/v1.0.1/
cp target/release/hvnc-client.exe release/v1.0.1/

# Copy documentation
cp -r docs/ release/v1.0.1/
cp README.md release/v1.0.1/

# Create archive
cd release
zip -r hidden-vnc-v1.0.1.zip v1.0.1/
```

### Continuous Integration

```yaml
# .github/workflows/ci.yml
name: CI

on: [push, pull_request]

jobs:
  test:
    runs-on: windows-latest
    steps:
    - uses: actions/checkout@v2
    - uses: actions-rs/toolchain@v1
      with:
        toolchain: stable
    - name: Run tests
      run: cargo test --verbose
    - name: Run clippy
      run: cargo clippy -- -D warnings
    - name: Check formatting
      run: cargo fmt -- --check
```

## Performance Optimization

### Profiling

```bash
# CPU profiling
cargo install flamegraph
cargo flamegraph --bin hvnc-server

# Memory profiling
cargo install heaptrack
heaptrack target/release/hvnc-server
```

### Optimization Techniques

1. **Frame Compression**
   ```rust
   // Optimize JPEG quality based on content
   fn adaptive_quality(frame: &Frame) -> u8 {
       let complexity = calculate_complexity(frame);
       match complexity {
           0..=30 => 95,   // Simple content, high quality
           31..=70 => 80,  // Medium content, balanced
           _ => 60,        // Complex content, lower quality
       }
   }
   ```

2. **Memory Management**
   ```rust
   // Reuse buffers to reduce allocations
   struct FrameProcessor {
       buffer: Vec<u8>,
   }
   
   impl FrameProcessor {
       fn process_frame(&mut self, frame: &Frame) -> Result<Vec<u8>> {
           self.buffer.clear();
           self.buffer.reserve(frame.data.len());
           // Process into existing buffer
           Ok(self.buffer.clone())
       }
   }
   ```

3. **Async Optimization**
   ```rust
   // Use async for I/O bound operations
   async fn stream_frames(mut stream: TcpStream) -> Result<()> {
       let mut interval = tokio::time::interval(Duration::from_millis(33)); // 30 FPS
       
       loop {
           interval.tick().await;
           let frame = capture_frame().await?;
           stream.write_all(&frame).await?;
       }
   }
   ```

## Security Considerations

### Input Validation

```rust
fn validate_input_event(event: &InputEvent) -> Result<()> {
    match event {
        InputEvent::MouseClick { x, y, .. } | InputEvent::MouseMove { x, y } => {
            if *x < 0 || *x > MAX_SCREEN_WIDTH || *y < 0 || *y > MAX_SCREEN_HEIGHT {
                return Err(HvncError::InputValidation("Coordinates out of bounds".into()));
            }
        }
        InputEvent::KeyPress { keycode, .. } => {
            if *keycode > 255 {
                return Err(HvncError::InputValidation("Invalid keycode".into()));
            }
        }
    }
    Ok(())
}
```

### Resource Limits

```rust
const MAX_FRAME_SIZE: usize = 10 * 1024 * 1024; // 10MB
const MAX_INPUT_RATE: u32 = 1000; // events per second

fn enforce_limits(frame_size: usize, input_rate: u32) -> Result<()> {
    if frame_size > MAX_FRAME_SIZE {
        return Err(HvncError::ResourceLimit("Frame too large".into()));
    }
    if input_rate > MAX_INPUT_RATE {
        return Err(HvncError::ResourceLimit("Input rate too high".into()));
    }
    Ok(())
}
```

## Debugging and Troubleshooting

### Debug Builds

```bash
# Build with debug symbols
cargo build --debug

# Run with debug logging
RUST_LOG=debug cargo run --bin hvnc-server
```

### Common Debug Techniques

```rust
// Add debug prints
debug!("Processing frame: {}x{}", width, height);

// Use debug assertions
debug_assert!(frame.width > 0, "Frame width must be positive");

// Conditional compilation for debug code
#[cfg(debug_assertions)]
fn debug_frame_info(frame: &Frame) {
    println!("Frame debug: {}x{}, {} bytes", frame.width, frame.height, frame.data.len());
}
```

### Memory Debugging

```bash
# Run with address sanitizer
RUSTFLAGS="-Z sanitizer=address" cargo run --target x86_64-unknown-linux-gnu

# Check for memory leaks
valgrind --leak-check=full target/debug/hvnc-server
```

---

*This developer guide provides comprehensive information for contributing to and extending the Hidden VNC project. For user-focused documentation, refer to the User Guide.*