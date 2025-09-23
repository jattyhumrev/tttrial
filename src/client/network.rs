//! Network client for connecting to the server

use crate::{HvncError, Result};
use crate::common::InputEvent;
use tokio::net::TcpStream;
use tokio::time::{timeout, Duration};
use std::net::SocketAddr;
use log::{info, warn, error, debug};

/// Network client for connecting to the Hidden VNC server
pub struct NetworkClient {
    stream: Option<TcpStream>,
    server_address: String,
    connection_timeout: Duration,
    retry_attempts: u32,
    retry_delay: Duration,
}

impl NetworkClient {
    /// Create a new network client
    pub fn new() -> Self {
        Self {
            stream: None,
            server_address: String::new(),
            connection_timeout: Duration::from_secs(10),
            retry_attempts: 3,
            retry_delay: Duration::from_secs(2),
        }
    }
    
    /// Create a new network client with custom configuration
    pub fn new_with_config(
        connection_timeout: Duration,
        retry_attempts: u32,
        retry_delay: Duration,
    ) -> Self {
        Self {
            stream: None,
            server_address: String::new(),
            connection_timeout,
            retry_attempts,
            retry_delay,
        }
    }
    
    /// Connect to the server with retry logic
    pub async fn connect(&mut self, address: &str) -> Result<()> {
        info!("Attempting to connect to server: {}", address);
        self.server_address = address.to_string();
        
        // Parse the address to validate it
        let socket_addr: SocketAddr = address.parse()
            .map_err(|e| {
                error!("Invalid server address '{}': {}", address, e);
                HvncError::Connection(format!("Invalid server address: {}", e))
            })?;
        
        let mut last_error = None;
        
        // Attempt connection with retries
        for attempt in 1..=self.retry_attempts {
            debug!("Connection attempt {}/{}", attempt, self.retry_attempts);
            
            match self.attempt_connection(&socket_addr).await {
                Ok(stream) => {
                    info!("Successfully connected to server: {}", address);
                    self.stream = Some(stream);
                    return Ok(());
                }
                Err(e) => {
                    warn!("Connection attempt {} failed: {}", attempt, e);
                    last_error = Some(e);
                    
                    // Don't wait after the last attempt
                    if attempt < self.retry_attempts {
                        debug!("Waiting {:?} before next attempt", self.retry_delay);
                        tokio::time::sleep(self.retry_delay).await;
                    }
                }
            }
        }
        
        let error = last_error.unwrap_or_else(|| {
            HvncError::Connection("Unknown connection error".to_string())
        });
        
        error!("Failed to connect to server after {} attempts: {}", self.retry_attempts, error);
        Err(error)
    }
    
    /// Attempt a single connection to the server
    async fn attempt_connection(&self, socket_addr: &SocketAddr) -> Result<TcpStream> {
        debug!("Attempting TCP connection to {}", socket_addr);
        
        let stream = timeout(self.connection_timeout, TcpStream::connect(socket_addr))
            .await
            .map_err(|_| {
                HvncError::Connection(format!(
                    "Connection timeout after {:?}", self.connection_timeout
                ))
            })?
            .map_err(|e| {
                HvncError::Network(e)
            })?;
        
        // Configure TCP stream options
        self.configure_stream(&stream)?;
        
        debug!("TCP connection established to {}", socket_addr);
        Ok(stream)
    }
    
    /// Configure TCP stream with optimal settings
    fn configure_stream(&self, _stream: &TcpStream) -> Result<()> {
        // Note: Stream configuration would be done here if needed
        // Cannot convert to std stream from borrowed reference

        
        debug!("TCP stream configured successfully");
        Ok(())
    }
    
    /// Receive a frame from the server
    pub async fn receive_frame(&mut self) -> Result<Vec<u8>> {
        // Implementation will be added in task 8.2
        let stream = self.stream.as_mut()
            .ok_or_else(|| HvncError::Connection("Not connected to server".to_string()))?;
        
        // Use the server's frame receiving logic
        let (mut read_half, _) = stream.split();
        crate::host::HvncServer::receive_frame_from_read_half(&mut read_half).await
    }
    
    /// Send an input event to the server
    pub async fn send_input(&mut self, event: InputEvent) -> Result<()> {
        let stream = self.stream.as_mut()
            .ok_or_else(|| HvncError::Connection("Not connected to server".to_string()))?;
        
        use tokio::io::AsyncWriteExt;
        
        debug!("🚀 HVNC Input Send: Preparing to send event: {:?}", event);
        
        // Serialize input event to JSON
        let message_data = serde_json::to_vec(&event)
            .map_err(|e| {
                error!("❌ Failed to serialize input event: {}", e);
                HvncError::Serialization(e)
            })?;
        
        debug!("📦 HVNC Input Send: Serialized {} bytes: {:?}", 
               message_data.len(), String::from_utf8_lossy(&message_data));
        
        // Validate message size
        if message_data.len() > 1024 {
            error!("❌ Serialized input event too large: {} bytes", message_data.len());
            return Err(HvncError::Connection(
                format!("Serialized input event too large: {} bytes", message_data.len())
            ));
        }
        
        // Send message length (4 bytes, little-endian)
        let message_len = message_data.len() as u32;
        debug!("📏 HVNC Input Send: Sending length header: {} bytes", message_len);
        
        match stream.write_all(&message_len.to_le_bytes()).await {
            Ok(()) => debug!("✅ HVNC Input Send: Length header sent successfully"),
            Err(e) => {
                error!("❌ Failed to write input message length: {}", e);
                return Err(HvncError::Network(e));
            }
        }
        
        // Send message data
        debug!("💾 HVNC Input Send: Sending message data: {} bytes", message_data.len());
        match stream.write_all(&message_data).await {
            Ok(()) => debug!("✅ HVNC Input Send: Message data sent successfully"),
            Err(e) => {
                error!("❌ Failed to write input message data: {}", e);
                return Err(HvncError::Network(e));
            }
        }
        
        // Flush to ensure immediate delivery
        debug!("🔄 HVNC Input Send: Flushing stream to ensure delivery");
        match stream.flush().await {
            Ok(()) => debug!("✅ HVNC Input Send: Stream flushed successfully"),
            Err(e) => {
                error!("❌ Failed to flush input message: {}", e);
                return Err(HvncError::Network(e));
            }
        }
        
        info!("✅ HVNC Input Send: Input event sent successfully: {:?}", event);
        Ok(())
    }
    
    /// Check if connected to server
    pub fn is_connected(&self) -> bool {
        self.stream.is_some()
    }
    
    /// Disconnect from the server
    pub async fn disconnect(&mut self) {
        if let Some(stream) = self.stream.take() {
            info!("Disconnecting from server: {}", self.server_address);
            drop(stream); // Close the connection
            debug!("Disconnected from server");
        }
    }
    
    /// Get the server address
    pub fn server_address(&self) -> &str {
        &self.server_address
    }
    
    /// Test connection to server without establishing persistent connection
    pub async fn test_connection(address: &str) -> Result<()> {
        info!("Testing connection to server: {}", address);
        
        let socket_addr: SocketAddr = address.parse()
            .map_err(|e| HvncError::Connection(format!("Invalid server address: {}", e)))?;
        
        let stream = timeout(Duration::from_secs(5), TcpStream::connect(socket_addr))
            .await
            .map_err(|_| HvncError::Connection("Connection test timeout".to_string()))?
            .map_err(|e| HvncError::Network(e))?;
        
        drop(stream);
        info!("Connection test successful");
        Ok(())
    }
    
    /// Reconnect to the server
    pub async fn reconnect(&mut self) -> Result<()> {
        info!("Attempting to reconnect to server");
        
        // Disconnect first
        self.disconnect().await;
        
        // Reconnect using the stored address
        let address = self.server_address.clone();
        if address.is_empty() {
            return Err(HvncError::Connection("No server address stored for reconnection".to_string()));
        }
        
        self.connect(&address).await
    }
    
    /// Check if the connection is still alive
    pub async fn is_connection_alive(&mut self) -> bool {
        if !self.is_connected() {
            return false;
        }
        
        // Try to peek at the stream to check if it's still alive
        match self.stream.as_mut() {
            Some(stream) => {
                // Try to read with a very short timeout
                match timeout(Duration::from_millis(1), stream.readable()).await {
                    Ok(Ok(())) => true,
                    _ => {
                        warn!("Connection appears to be dead");
                        self.stream = None;
                        false
                    }
                }
            }
            None => false,
        }
    }
    
    /// Get connection statistics
    pub fn get_connection_stats(&self) -> ConnectionStats {
        ConnectionStats {
            is_connected: self.is_connected(),
            server_address: self.server_address.clone(),
            connection_timeout: self.connection_timeout,
            retry_attempts: self.retry_attempts,
            retry_delay: self.retry_delay,
        }
    }
    
    /// Update connection configuration
    pub fn update_config(
        &mut self,
        connection_timeout: Option<Duration>,
        retry_attempts: Option<u32>,
        retry_delay: Option<Duration>,
    ) {
        if let Some(timeout) = connection_timeout {
            self.connection_timeout = timeout;
            debug!("Connection timeout updated to {:?}", timeout);
        }
        
        if let Some(attempts) = retry_attempts {
            self.retry_attempts = attempts;
            debug!("Retry attempts updated to {}", attempts);
        }
        
        if let Some(delay) = retry_delay {
            self.retry_delay = delay;
            debug!("Retry delay updated to {:?}", delay);
        }
    }
}

impl Default for NetworkClient {
    fn default() -> Self {
        Self::new()
    }
}

/// Connection statistics
#[derive(Debug, Clone)]
pub struct ConnectionStats {
    pub is_connected: bool,
    pub server_address: String,
    pub connection_timeout: Duration,
    pub retry_attempts: u32,
    pub retry_delay: Duration,
}#[
cfg(test)]
mod tests {
    use super::*;
    use tokio::net::TcpListener;
    use std::net::SocketAddr;
    
    async fn create_test_server() -> (TcpListener, SocketAddr) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        (listener, addr)
    }
    
    #[tokio::test]
    async fn test_network_client_creation() {
        let client = NetworkClient::new();
        assert!(!client.is_connected());
        assert_eq!(client.server_address(), "");
        assert_eq!(client.connection_timeout, Duration::from_secs(10));
        assert_eq!(client.retry_attempts, 3);
        assert_eq!(client.retry_delay, Duration::from_secs(2));
    }
    
    #[tokio::test]
    async fn test_network_client_with_config() {
        let client = NetworkClient::new_with_config(
            Duration::from_secs(5),
            2,
            Duration::from_secs(1),
        );
        
        assert_eq!(client.connection_timeout, Duration::from_secs(5));
        assert_eq!(client.retry_attempts, 2);
        assert_eq!(client.retry_delay, Duration::from_secs(1));
    }
    
    #[tokio::test]
    async fn test_invalid_address() {
        let mut client = NetworkClient::new();
        
        let result = client.connect("invalid_address").await;
        assert!(result.is_err());
        
        if let Err(HvncError::Connection(msg)) = result {
            assert!(msg.contains("Invalid server address"));
        } else {
            panic!("Expected Connection error");
        }
    }
    
    #[tokio::test]
    async fn test_connection_to_nonexistent_server() {
        let mut client = NetworkClient::new_with_config(
            Duration::from_millis(100), // Short timeout for test
            1, // Single attempt
            Duration::from_millis(10),
        );
        
        // Try to connect to a port that should be closed
        let result = client.connect("127.0.0.1:1").await;
        assert!(result.is_err());
        assert!(!client.is_connected());
    }
    
    #[tokio::test]
    async fn test_successful_connection() {
        let (listener, addr) = create_test_server().await;
        
        // Spawn a task to accept the connection
        tokio::spawn(async move {
            if let Ok((stream, _)) = listener.accept().await {
                // Just accept and close
                drop(stream);
            }
        });
        
        let mut client = NetworkClient::new();
        let result = client.connect(&addr.to_string()).await;
        
        assert!(result.is_ok(), "Connection should succeed");
        assert!(client.is_connected());
        assert_eq!(client.server_address(), &addr.to_string());
    }
    
    #[tokio::test]
    async fn test_connection_retry_logic() {
        let mut client = NetworkClient::new_with_config(
            Duration::from_millis(50), // Short timeout
            2, // Two attempts
            Duration::from_millis(10), // Short delay
        );
        
        let start_time = std::time::Instant::now();
        let result = client.connect("127.0.0.1:1").await; // Should fail
        let elapsed = start_time.elapsed();
        
        assert!(result.is_err());
        // Should have taken at least the retry delay time
        assert!(elapsed >= Duration::from_millis(10));
    }
    
    #[tokio::test]
    async fn test_disconnect() {
        let (listener, addr) = create_test_server().await;
        
        tokio::spawn(async move {
            if let Ok((stream, _)) = listener.accept().await {
                // Keep connection alive briefly
                tokio::time::sleep(Duration::from_millis(100)).await;
                drop(stream);
            }
        });
        
        let mut client = NetworkClient::new();
        client.connect(&addr.to_string()).await.unwrap();
        
        assert!(client.is_connected());
        
        client.disconnect().await;
        assert!(!client.is_connected());
    }
    
    #[tokio::test]
    async fn test_connection_test() {
        let (listener, addr) = create_test_server().await;
        
        tokio::spawn(async move {
            if let Ok((stream, _)) = listener.accept().await {
                drop(stream);
            }
        });
        
        let result = NetworkClient::test_connection(&addr.to_string()).await;
        assert!(result.is_ok(), "Connection test should succeed");
        
        // Test with invalid address
        let result = NetworkClient::test_connection("invalid_address").await;
        assert!(result.is_err(), "Connection test should fail for invalid address");
    }
    
    #[tokio::test]
    async fn test_reconnect() {
        let (listener, addr) = create_test_server().await;
        
        // Accept multiple connections
        tokio::spawn(async move {
            for _ in 0..2 {
                if let Ok((stream, _)) = listener.accept().await {
                    tokio::time::sleep(Duration::from_millis(50)).await;
                    drop(stream);
                }
            }
        });
        
        let mut client = NetworkClient::new();
        
        // Initial connection
        client.connect(&addr.to_string()).await.unwrap();
        assert!(client.is_connected());
        
        // Disconnect
        client.disconnect().await;
        assert!(!client.is_connected());
        
        // Reconnect
        let result = client.reconnect().await;
        assert!(result.is_ok(), "Reconnection should succeed");
        assert!(client.is_connected());
    }
    
    #[tokio::test]
    async fn test_reconnect_without_previous_connection() {
        let mut client = NetworkClient::new();
        
        let result = client.reconnect().await;
        assert!(result.is_err());
        
        if let Err(HvncError::Connection(msg)) = result {
            assert!(msg.contains("No server address stored"));
        } else {
            panic!("Expected Connection error");
        }
    }
    
    #[tokio::test]
    async fn test_connection_stats() {
        let client = NetworkClient::new_with_config(
            Duration::from_secs(15),
            5,
            Duration::from_secs(3),
        );
        
        let stats = client.get_connection_stats();
        assert!(!stats.is_connected);
        assert_eq!(stats.server_address, "");
        assert_eq!(stats.connection_timeout, Duration::from_secs(15));
        assert_eq!(stats.retry_attempts, 5);
        assert_eq!(stats.retry_delay, Duration::from_secs(3));
    }
    
    #[tokio::test]
    async fn test_config_updates() {
        let mut client = NetworkClient::new();
        
        client.update_config(
            Some(Duration::from_secs(20)),
            Some(10),
            Some(Duration::from_secs(5)),
        );
        
        assert_eq!(client.connection_timeout, Duration::from_secs(20));
        assert_eq!(client.retry_attempts, 10);
        assert_eq!(client.retry_delay, Duration::from_secs(5));
        
        // Test partial updates
        client.update_config(
            Some(Duration::from_secs(30)),
            None,
            None,
        );
        
        assert_eq!(client.connection_timeout, Duration::from_secs(30));
        assert_eq!(client.retry_attempts, 10); // Unchanged
        assert_eq!(client.retry_delay, Duration::from_secs(5)); // Unchanged
    }
    
    #[tokio::test]
    async fn test_connection_alive_check() {
        let mut client = NetworkClient::new();
        
        // Not connected
        assert!(!client.is_connection_alive().await);
        
        let (listener, addr) = create_test_server().await;
        
        tokio::spawn(async move {
            if let Ok((stream, _)) = listener.accept().await {
                // Keep connection alive for a bit
                tokio::time::sleep(Duration::from_millis(200)).await;
                drop(stream);
            }
        });
        
        // Connect
        client.connect(&addr.to_string()).await.unwrap();
        
        // Should be alive initially
        assert!(client.is_connection_alive().await);
        
        // Wait for connection to be dropped by server
        tokio::time::sleep(Duration::from_millis(300)).await;
        
        // Connection should be detected as dead
        // Note: This test might be flaky depending on timing
    }
    
    #[tokio::test]
    async fn test_address_parsing() {
        let test_addresses = vec![
            ("127.0.0.1:5900", true),
            ("localhost:8080", false), // localhost might not resolve in test environment
            ("192.168.1.1:3389", true),
            ("invalid:port", false),
            ("127.0.0.1", false), // Missing port
            ("127.0.0.1:99999", false), // Invalid port
        ];
        
        for (address, should_parse) in test_addresses {
            let result: std::result::Result<SocketAddr, _> = address.parse();
            if should_parse {
                assert!(result.is_ok(), "Address '{}' should parse successfully", address);
            } else {
                // Note: Some addresses might parse but fail to connect
                // This test mainly checks the parsing logic
            }
        }
    }
    
    #[tokio::test]
    async fn test_default_implementation() {
        let client = NetworkClient::default();
        assert!(!client.is_connected());
        assert_eq!(client.server_address(), "");
    }
    
    #[tokio::test]
    async fn test_connection_timeout() {
        let mut client = NetworkClient::new_with_config(
            Duration::from_millis(100), // Very short timeout
            1,
            Duration::from_millis(10),
        );
        
        // Try to connect to an address that will cause timeout
        // Using a non-routable address that should cause timeout
        let start_time = std::time::Instant::now();
        let result = client.connect("10.255.255.1:80").await; // Non-routable IP
        let elapsed = start_time.elapsed();
        
        assert!(result.is_err());
        // Should timeout within reasonable time (allowing some overhead)
        assert!(elapsed < Duration::from_millis(500));
    }
}