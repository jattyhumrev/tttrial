# Hidden VNC API Reference

## Core APIs

### Desktop Management

#### `DesktopManager`

Manages Windows hidden desktop creation and lifecycle.

```rust
pub struct DesktopManager {
    desktop_handle: HDESK,
    desktop_name: String,
}
```

##### Methods

**`create_hidden_desktop(name: &str) -> Result<DesktopManager>`**
- Creates a new hidden desktop with the specified name
- Returns: `Result<DesktopManager>` - Desktop manager instance or error
- Errors: `HvncError::DesktopCreation` if creation fails

**`switch_to_desktop(&self) -> Result<()>`**
- Switches the current thread to the hidden desktop
- Returns: `Result<()>` - Success or error
- Errors: `HvncError::DesktopSwitch` if switch fails

**`cleanup(&mut self) -> Result<()>`**
- Cleans up desktop resources and handles
- Returns: `Result<()>` - Success or error
- Errors: `HvncError::ResourceCleanup` if cleanup fails

### Application Launcher

#### `ApplicationLauncher`

Launches and manages applications in hidden desktops.

```rust
pub struct ApplicationLauncher {
    desktop_handle: HDESK,
}
```

##### Methods

**`new(desktop: &DesktopManager) -> Self`**
- Creates a new application launcher for the specified desktop
- Parameters: `desktop` - Desktop manager instance
- Returns: `ApplicationLauncher` instance

**`launch_in_desktop(app_path: &str, args: Option<&str>, desktop: &DesktopManager) -> Result<ProcessInfo>`**
- Launches an application in the specified desktop
- Parameters:
  - `app_path` - Path to the executable
  - `args` - Optional command line arguments
  - `desktop` - Target desktop
- Returns: `Result<ProcessInfo>` - Process information or error
- Errors: `HvncError::ApplicationLaunchFailed` if launch fails

**`is_running(&self, process_info: &ProcessInfo) -> bool`**
- Checks if the specified process is still running
- Parameters: `process_info` - Process information
- Returns: `bool` - True if running, false otherwise

### Window Capture

#### `WindowCapture`

Captures window content and converts to image data.

```rust
pub struct WindowCapture {
    target_window: HWND,
    capture_rect: RECT,
}
```

##### Methods

**`find_window(criteria: &WindowCriteria) -> Result<Self>`**
- Finds a window based on specified criteria
- Parameters: `criteria` - Window search criteria
- Returns: `Result<WindowCapture>` - Capture instance or error
- Errors: `HvncError::WindowNotFound` if window not found

**`capture_frame(&self) -> Result<Frame>`**
- Captures the current window content as a frame
- Returns: `Result<Frame>` - Frame data or error
- Errors: `HvncError::WindowCaptureFailed` if capture fails

**`compress_frame(&self, frame: &Frame, quality: u8) -> Result<Vec<u8>>`**
- Compresses a frame to JPEG format
- Parameters:
  - `frame` - Frame to compress
  - `quality` - JPEG quality (1-100)
- Returns: `Result<Vec<u8>>` - Compressed data or error
- Errors: `HvncError::ImageCompression` if compression fails

### Network Server

#### `HvncServer`

Manages client connections and data streaming.

```rust
pub struct HvncServer {
    listener: TcpListener,
    config: ServerConfig,
}
```

##### Methods

**`new(config: ServerConfig) -> Result<Self>`**
- Creates a new HVNC server with the specified configuration
- Parameters: `config` - Server configuration
- Returns: `Result<HvncServer>` - Server instance or error
- Errors: `HvncError::Network` if server creation fails

**`start(&mut self) -> Result<()>`**
- Starts the server and begins listening for connections
- Returns: `Result<()>` - Success or error
- Errors: `HvncError::Network` if server start fails

**`handle_client(&mut self, stream: TcpStream) -> Result<()>`**
- Handles a client connection
- Parameters: `stream` - Client TCP stream
- Returns: `Result<()>` - Success or error
- Errors: Various network and protocol errors

### Network Client

#### `HvncClient`

Connects to HVNC server and manages data reception.

```rust
pub struct HvncClient {
    stream: TcpStream,
    config: ClientConfig,
}
```

##### Methods

**`connect(server_addr: &str, config: ClientConfig) -> Result<Self>`**
- Connects to an HVNC server
- Parameters:
  - `server_addr` - Server address (host:port)
  - `config` - Client configuration
- Returns: `Result<HvncClient>` - Client instance or error
- Errors: `HvncError::ConnectionRefused` if connection fails

**`receive_frame(&mut self) -> Result<Frame>`**
- Receives a frame from the server
- Returns: `Result<Frame>` - Frame data or error
- Errors: `HvncError::Network` if reception fails

**`send_input(&mut self, event: &InputEvent) -> Result<()>`**
- Sends an input event to the server
- Parameters: `event` - Input event to send
- Returns: `Result<()>` - Success or error
- Errors: `HvncError::Network` if send fails

## Data Structures

### `Frame`

Represents a captured frame of image data.

```rust
pub struct Frame {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}
```

**Fields:**
- `width` - Frame width in pixels
- `height` - Frame height in pixels
- `data` - RGB image data (width × height × 3 bytes)

### `InputEvent`

Represents user input events.

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum InputEvent {
    MouseClick { x: i32, y: i32, button: MouseButton },
    MouseMove { x: i32, y: i32 },
    KeyPress { keycode: u32, pressed: bool },
}
```

**Variants:**
- `MouseClick` - Mouse button click at coordinates (x, y)
- `MouseMove` - Mouse movement to coordinates (x, y)
- `KeyPress` - Keyboard key press/release with virtual keycode

### `MouseButton`

Mouse button types.

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}
```

### `ProcessInfo`

Information about a launched process.

```rust
pub struct ProcessInfo {
    pub id: u32,
    pub handle: HANDLE,
    pub thread_id: u32,
}
```

**Fields:**
- `id` - Process ID
- `handle` - Process handle
- `thread_id` - Main thread ID

### `WindowCriteria`

Criteria for finding windows.

```rust
pub enum WindowCriteria {
    Title(String),
    ProcessId(u32),
    ClassName(String),
}
```

**Variants:**
- `Title` - Find by window title
- `ProcessId` - Find by process ID
- `ClassName` - Find by window class name

## Configuration

### `ServerConfig`

Server configuration options.

```rust
pub struct ServerConfig {
    pub bind_address: String,
    pub port: u16,
    pub jpeg_quality: u8,
    pub frame_rate: u32,
    pub timeout: Duration,
}
```

**Fields:**
- `bind_address` - Address to bind server (default: "127.0.0.1")
- `port` - Port to listen on (default: 5900)
- `jpeg_quality` - JPEG compression quality 1-100 (default: 80)
- `frame_rate` - Target frame rate in FPS (default: 30)
- `timeout` - Client timeout duration (default: 300 seconds)

### `ClientConfig`

Client configuration options.

```rust
pub struct ClientConfig {
    pub timeout: Duration,
    pub reconnect: bool,
    pub scale_factor: f32,
    pub fullscreen: bool,
}
```

**Fields:**
- `timeout` - Connection timeout (default: 30 seconds)
- `reconnect` - Enable automatic reconnection (default: false)
- `scale_factor` - Display scaling factor (default: 1.0)
- `fullscreen` - Start in fullscreen mode (default: false)

## Error Types

### `HvncError`

Comprehensive error enumeration for all HVNC operations.

```rust
#[derive(Error, Debug)]
pub enum HvncError {
    // Windows API Errors
    WindowsApi { message: String, code: Option<u32> },
    WindowsHandle(String),
    WindowsPermission(String),
    
    // Network Errors
    Network(std::io::Error),
    ConnectionTimeout(String),
    ConnectionRefused(String),
    ConnectionLost(String),
    Protocol(String),
    Authentication(String),
    
    // Image Processing Errors
    Image(image::ImageError),
    ImageFormat(String),
    ImageCompression(String),
    ImageDecompression(String),
    
    // Application Errors
    ApplicationNotFound { name: String },
    ApplicationLaunchFailed { name: String, reason: String },
    ApplicationCrashed { name: String, exit_code: Option<u32> },
    ApplicationTimeout(String),
    
    // Window Management Errors
    WindowNotFound { title: String },
    WindowAccessDenied(String),
    WindowCaptureFailed(String),
    WindowResizeFailed(String),
    
    // Desktop Management Errors
    DesktopCreation { name: String, reason: String },
    DesktopAccess(String),
    DesktopSwitch(String),
    
    // Input Handling Errors
    InputValidation(String),
    InputProcessing(String),
    InputRateLimit(String),
    InputCoordinateOutOfBounds { x: i32, y: i32 },
    InvalidKeycode { keycode: u32 },
    
    // Configuration Errors
    Configuration(String),
    InvalidParameter { parameter: String, value: String },
    MissingParameter(String),
    
    // Resource Management Errors
    ResourceAllocation { resource: String, reason: String },
    ResourceCleanup(String),
    MemoryAllocation { size: usize },
    FileSystem(String),
    
    // Security Errors
    Security(String),
    AccessDenied(String),
    PermissionDenied { operation: String },
    
    // Generic Errors
    OperationFailed(String),
    Internal(String),
    NotImplemented(String),
    InvalidState(String),
    
    // Multiple errors
    Multiple(Vec<HvncError>),
}
```

### Error Helper Methods

**`is_recoverable(&self) -> bool`**
- Determines if an error is recoverable
- Returns: `bool` - True if error can be recovered from

**`should_retry(&self) -> bool`**
- Determines if operation should be retried
- Returns: `bool` - True if retry is recommended

**`severity(&self) -> ErrorSeverity`**
- Gets the severity level of the error
- Returns: `ErrorSeverity` - Error severity level

**`category(&self) -> ErrorCategory`**
- Gets the category of the error
- Returns: `ErrorCategory` - Error category

## Constants

### Protocol Constants

```rust
pub mod constants {
    /// Default server port
    pub const DEFAULT_PORT: u16 = 5900;
    
    /// Maximum frame size (10MB)
    pub const MAX_FRAME_SIZE: usize = 10 * 1024 * 1024;
    
    /// Default JPEG quality
    pub const DEFAULT_JPEG_QUALITY: u8 = 80;
    
    /// Default frame rate (30 FPS)
    pub const DEFAULT_FRAME_RATE_MS: u64 = 33;
    
    /// Maximum input events per second
    pub const MAX_INPUT_RATE: u32 = 1000;
    
    /// Connection timeout in seconds
    pub const CONNECTION_TIMEOUT_SECS: u64 = 30;
    
    /// Maximum coordinate values
    pub const MAX_COORDINATE: i32 = 32767;
    
    /// Maximum keycode value
    pub const MAX_KEYCODE: u32 = 255;
}
```

## Usage Examples

### Basic Server Setup

```rust
use hidden_vnc::*;

#[tokio::main]
async fn main() -> Result<()> {
    // Create hidden desktop
    let desktop = DesktopManager::create_hidden_desktop("hvnc_desktop")?;
    
    // Launch application
    let launcher = ApplicationLauncher::new(&desktop);
    let process = launcher.launch_in_desktop("notepad.exe", None, &desktop)?;
    
    // Setup window capture
    let capture = WindowCapture::find_window(&WindowCriteria::ProcessId(process.id))?;
    
    // Configure and start server
    let config = ServerConfig {
        bind_address: "127.0.0.1".to_string(),
        port: 5900,
        jpeg_quality: 80,
        frame_rate: 30,
        timeout: Duration::from_secs(300),
    };
    
    let mut server = HvncServer::new(config)?;
    server.start()?;
    
    Ok(())
}
```

### Basic Client Setup

```rust
use hidden_vnc::*;

#[tokio::main]
async fn main() -> Result<()> {
    // Configure client
    let config = ClientConfig {
        timeout: Duration::from_secs(30),
        reconnect: true,
        scale_factor: 1.0,
        fullscreen: false,
    };
    
    // Connect to server
    let mut client = HvncClient::connect("127.0.0.1:3390", config)?;
    
    // Main event loop
    loop {
        // Receive and display frame
        let frame = client.receive_frame()?;
        display_frame(&frame)?;
        
        // Process input events
        if let Some(event) = capture_input()? {
            client.send_input(&event)?;
        }
    }
}
```

### Error Handling Example

```rust
use hidden_vnc::*;

fn handle_operation() -> Result<()> {
    match risky_operation() {
        Ok(result) => {
            info!("Operation succeeded: {:?}", result);
            Ok(())
        }
        Err(e) => {
            match e {
                HvncError::WindowsPermission(_) => {
                    error!("Permission denied - run as administrator");
                    Err(e)
                }
                HvncError::ConnectionTimeout(_) if e.should_retry() => {
                    warn!("Connection timeout - retrying...");
                    retry_operation()
                }
                _ => {
                    error!("Unrecoverable error: {}", e);
                    Err(e)
                }
            }
        }
    }
}
```

---

*This API reference provides detailed information about all public interfaces in the Hidden VNC library. For usage examples and tutorials, refer to the Developer Guide.*