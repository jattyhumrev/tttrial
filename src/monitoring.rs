//! Comprehensive monitoring and metrics system for Hidden VNC

use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use log::{info, warn, error, debug};
use serde::{Serialize, Deserialize};
use crate::error::{HvncError, ErrorCategory, ErrorSeverity};

/// Global metrics collector instance
static METRICS: once_cell::sync::Lazy<Arc<MetricsCollector>> = 
    once_cell::sync::Lazy::new(|| Arc::new(MetricsCollector::new()));

/// Get the global metrics collector
pub fn metrics() -> Arc<MetricsCollector> {
    Arc::clone(&METRICS)
}

/// Comprehensive metrics collector
pub struct MetricsCollector {
    counters: RwLock<HashMap<String, u64>>,
    gauges: RwLock<HashMap<String, f64>>,
    histograms: RwLock<HashMap<String, Histogram>>,
    error_metrics: RwLock<ErrorMetrics>,
    performance_metrics: RwLock<PerformanceMetrics>,
    system_metrics: Mutex<SystemMetrics>,
    start_time: Instant,
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            counters: RwLock::new(HashMap::new()),
            gauges: RwLock::new(HashMap::new()),
            histograms: RwLock::new(HashMap::new()),
            error_metrics: RwLock::new(ErrorMetrics::new()),
            performance_metrics: RwLock::new(PerformanceMetrics::new()),
            system_metrics: Mutex::new(SystemMetrics::new()),
            start_time: Instant::now(),
        }
    }
    
    /// Increment a counter
    pub fn increment_counter(&self, name: &str) {
        self.add_to_counter(name, 1);
    }
    
    /// Add value to a counter
    pub fn add_to_counter(&self, name: &str, value: u64) {
        let mut counters = self.counters.write().unwrap();
        *counters.entry(name.to_string()).or_insert(0) += value;
        debug!("Counter '{}' incremented by {}", name, value);
    }
    
    /// Set a gauge value
    pub fn set_gauge(&self, name: &str, value: f64) {
        let mut gauges = self.gauges.write().unwrap();
        gauges.insert(name.to_string(), value);
        debug!("Gauge '{}' set to {}", name, value);
    }
    
    /// Record a histogram value
    pub fn record_histogram(&self, name: &str, value: f64) {
        let mut histograms = self.histograms.write().unwrap();
        let histogram = histograms.entry(name.to_string()).or_insert_with(Histogram::new);
        histogram.record(value);
        debug!("Histogram '{}' recorded value {}", name, value);
    }
    
    /// Record an error
    pub fn record_error(&self, error: &HvncError) {
        let mut error_metrics = self.error_metrics.write().unwrap();
        error_metrics.record_error(error);
        
        // Also increment counters for error tracking
        self.increment_counter("errors.total");
        self.increment_counter(&format!("errors.category.{}", error.category()));
        self.increment_counter(&format!("errors.severity.{:?}", error.severity()));
        
        debug!("Error recorded: {} (category: {}, severity: {:?})", 
               error, error.category(), error.severity());
    }
    
    /// Record performance metrics
    pub fn record_performance(&self, operation: &str, duration: Duration) {
        let mut perf_metrics = self.performance_metrics.write().unwrap();
        perf_metrics.record_operation(operation, duration);
        
        // Also record in histogram
        self.record_histogram(&format!("performance.{}.duration_ms", operation), 
                             duration.as_millis() as f64);
        
        debug!("Performance recorded: {} took {:?}", operation, duration);
    }
    
    /// Update system metrics
    pub fn update_system_metrics(&self) {
        if let Ok(mut system_metrics) = self.system_metrics.try_lock() {
            system_metrics.update();
            
            // Update gauges with system metrics
            self.set_gauge("system.memory_usage_mb", system_metrics.memory_usage_mb);
            self.set_gauge("system.cpu_usage_percent", system_metrics.cpu_usage_percent);
            self.set_gauge("system.uptime_seconds", self.start_time.elapsed().as_secs() as f64);
        }
    }
    
    /// Get counter value
    pub fn get_counter(&self, name: &str) -> u64 {
        self.counters.read().unwrap().get(name).copied().unwrap_or(0)
    }
    
    /// Get gauge value
    pub fn get_gauge(&self, name: &str) -> Option<f64> {
        self.gauges.read().unwrap().get(name).copied()
    }
    
    /// Get histogram statistics
    pub fn get_histogram_stats(&self, name: &str) -> Option<HistogramStats> {
        self.histograms.read().unwrap().get(name).map(|h| h.stats())
    }
    
    /// Get comprehensive metrics snapshot
    pub fn get_metrics_snapshot(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            uptime_seconds: self.start_time.elapsed().as_secs(),
            counters: self.counters.read().unwrap().clone(),
            gauges: self.gauges.read().unwrap().clone(),
            histogram_stats: self.histograms.read().unwrap()
                .iter()
                .map(|(k, v)| (k.clone(), v.stats()))
                .collect(),
            error_metrics: self.error_metrics.read().unwrap().clone(),
            performance_metrics: self.performance_metrics.read().unwrap().clone(),
            system_metrics: self.system_metrics.lock().unwrap().clone(),
        }
    }
    
    /// Log metrics summary
    pub fn log_metrics_summary(&self) {
        let snapshot = self.get_metrics_snapshot();
        
        info!("=== METRICS SUMMARY ===");
        info!("Uptime: {} seconds", snapshot.uptime_seconds);
        
        // Log top counters
        let mut counters: Vec<_> = snapshot.counters.iter().collect();
        counters.sort_by(|a, b| b.1.cmp(a.1));
        info!("Top Counters:");
        for (name, value) in counters.iter().take(10) {
            info!("  {}: {}", name, value);
        }
        
        // Log important gauges
        info!("System Metrics:");
        info!("  Memory Usage: {:.1} MB", snapshot.system_metrics.memory_usage_mb);
        info!("  CPU Usage: {:.1}%", snapshot.system_metrics.cpu_usage_percent);
        
        // Log error summary
        info!("Error Summary:");
        info!("  Total Errors: {}", snapshot.error_metrics.total_errors);
        info!("  Error Rate: {:.2}/min", snapshot.error_metrics.error_rate_per_minute());
        
        // Log performance summary
        info!("Performance Summary:");
        for (operation, stats) in &snapshot.performance_metrics.operation_stats {
            info!("  {}: avg={:.1}ms, count={}", 
                  operation, stats.average_duration_ms(), stats.count);
        }
        
        info!("=== END METRICS SUMMARY ===");
    }
    
    /// Reset all metrics
    pub fn reset(&self) {
        self.counters.write().unwrap().clear();
        self.gauges.write().unwrap().clear();
        self.histograms.write().unwrap().clear();
        *self.error_metrics.write().unwrap() = ErrorMetrics::new();
        *self.performance_metrics.write().unwrap() = PerformanceMetrics::new();
        *self.system_metrics.lock().unwrap() = SystemMetrics::new();
        info!("All metrics reset");
    }
}

/// Histogram for tracking value distributions
#[derive(Debug, Clone)]
pub struct Histogram {
    values: Vec<f64>,
    min: f64,
    max: f64,
    sum: f64,
    count: u64,
}

impl Histogram {
    pub fn new() -> Self {
        Self {
            values: Vec::new(),
            min: f64::INFINITY,
            max: f64::NEG_INFINITY,
            sum: 0.0,
            count: 0,
        }
    }
    
    pub fn record(&mut self, value: f64) {
        self.values.push(value);
        self.min = self.min.min(value);
        self.max = self.max.max(value);
        self.sum += value;
        self.count += 1;
        
        // Keep only recent values to prevent memory growth
        if self.values.len() > 1000 {
            self.values.drain(0..500); // Remove oldest half
        }
    }
    
    pub fn stats(&self) -> HistogramStats {
        if self.count == 0 {
            return HistogramStats::default();
        }
        
        let mut sorted_values = self.values.clone();
        sorted_values.sort_by(|a, b| a.partial_cmp(b).unwrap());
        
        let percentile = |p: f64| -> f64 {
            if sorted_values.is_empty() {
                return 0.0;
            }
            let index = ((sorted_values.len() - 1) as f64 * p / 100.0) as usize;
            sorted_values[index]
        };
        
        HistogramStats {
            count: self.count,
            min: self.min,
            max: self.max,
            mean: self.sum / self.count as f64,
            p50: percentile(50.0),
            p90: percentile(90.0),
            p95: percentile(95.0),
            p99: percentile(99.0),
        }
    }
}

/// Histogram statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistogramStats {
    pub count: u64,
    pub min: f64,
    pub max: f64,
    pub mean: f64,
    pub p50: f64,
    pub p90: f64,
    pub p95: f64,
    pub p99: f64,
}

impl Default for HistogramStats {
    fn default() -> Self {
        Self {
            count: 0,
            min: 0.0,
            max: 0.0,
            mean: 0.0,
            p50: 0.0,
            p90: 0.0,
            p95: 0.0,
            p99: 0.0,
        }
    }
}

/// Error tracking metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorMetrics {
    pub total_errors: u64,
    pub errors_by_category: HashMap<String, u64>,
    pub errors_by_severity: HashMap<String, u64>,
    pub recent_errors: Vec<ErrorRecord>,
    pub first_error_time: Option<u64>,
    pub last_error_time: Option<u64>,
}

impl ErrorMetrics {
    pub fn new() -> Self {
        Self {
            total_errors: 0,
            errors_by_category: HashMap::new(),
            errors_by_severity: HashMap::new(),
            recent_errors: Vec::new(),
            first_error_time: None,
            last_error_time: None,
        }
    }
    
    pub fn record_error(&mut self, error: &HvncError) {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        
        self.total_errors += 1;
        
        // Track by category
        let category = error.category().to_string();
        *self.errors_by_category.entry(category).or_insert(0) += 1;
        
        // Track by severity
        let severity = format!("{:?}", error.severity());
        *self.errors_by_severity.entry(severity).or_insert(0) += 1;
        
        // Record recent error
        let error_record = ErrorRecord {
            timestamp: now,
            category: error.category(),
            severity: error.severity(),
            message: error.to_string(),
        };
        
        self.recent_errors.push(error_record);
        
        // Keep only recent errors (last 100)
        if self.recent_errors.len() > 100 {
            self.recent_errors.drain(0..50); // Remove oldest half
        }
        
        // Update timestamps
        if self.first_error_time.is_none() {
            self.first_error_time = Some(now);
        }
        self.last_error_time = Some(now);
    }
    
    pub fn error_rate_per_minute(&self) -> f64 {
        if let (Some(first), Some(last)) = (self.first_error_time, self.last_error_time) {
            let duration_minutes = (last - first) as f64 / 60.0;
            if duration_minutes > 0.0 {
                return self.total_errors as f64 / duration_minutes;
            }
        }
        0.0
    }
}

/// Individual error record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorRecord {
    pub timestamp: u64,
    pub category: ErrorCategory,
    pub severity: ErrorSeverity,
    pub message: String,
}

/// Performance tracking metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub operation_stats: HashMap<String, OperationStats>,
}

impl PerformanceMetrics {
    pub fn new() -> Self {
        Self {
            operation_stats: HashMap::new(),
        }
    }
    
    pub fn record_operation(&mut self, operation: &str, duration: Duration) {
        let stats = self.operation_stats.entry(operation.to_string()).or_insert_with(OperationStats::new);
        stats.record(duration);
    }
}

/// Statistics for a specific operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationStats {
    pub count: u64,
    pub total_duration_ms: u64,
    pub min_duration_ms: u64,
    pub max_duration_ms: u64,
    pub recent_durations: Vec<u64>,
}

impl OperationStats {
    pub fn new() -> Self {
        Self {
            count: 0,
            total_duration_ms: 0,
            min_duration_ms: u64::MAX,
            max_duration_ms: 0,
            recent_durations: Vec::new(),
        }
    }
    
    pub fn record(&mut self, duration: Duration) {
        let duration_ms = duration.as_millis() as u64;
        
        self.count += 1;
        self.total_duration_ms += duration_ms;
        self.min_duration_ms = self.min_duration_ms.min(duration_ms);
        self.max_duration_ms = self.max_duration_ms.max(duration_ms);
        
        self.recent_durations.push(duration_ms);
        
        // Keep only recent durations
        if self.recent_durations.len() > 100 {
            self.recent_durations.drain(0..50);
        }
    }
    
    pub fn average_duration_ms(&self) -> f64 {
        if self.count > 0 {
            self.total_duration_ms as f64 / self.count as f64
        } else {
            0.0
        }
    }
}

/// System resource metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetrics {
    pub memory_usage_mb: f64,
    pub cpu_usage_percent: f64,
    pub thread_count: u32,
    pub handle_count: u32,
    pub last_updated: u64,
}

impl SystemMetrics {
    pub fn new() -> Self {
        Self {
            memory_usage_mb: 0.0,
            cpu_usage_percent: 0.0,
            thread_count: 0,
            handle_count: 0,
            last_updated: 0,
        }
    }
    
    pub fn update(&mut self) {
        // Update system metrics (simplified implementation)
        // In a real implementation, this would use platform-specific APIs
        
        self.memory_usage_mb = self.get_memory_usage();
        self.cpu_usage_percent = self.get_cpu_usage();
        self.thread_count = self.get_thread_count();
        self.handle_count = self.get_handle_count();
        self.last_updated = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    }
    
    #[cfg(windows)]
    fn get_memory_usage(&self) -> f64 {
        use winapi::um::processthreadsapi::GetCurrentProcess;
        use winapi::um::psapi::{GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS};
        use std::mem;
        
        unsafe {
            let process = GetCurrentProcess();
            let mut pmc: PROCESS_MEMORY_COUNTERS = mem::zeroed();
            
            if winapi::um::psapi::GetProcessMemoryInfo(
                process,
                &mut pmc,
                mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32,
            ) != 0 {
                return pmc.WorkingSetSize as f64 / 1024.0 / 1024.0; // Convert to MB
            }
        }
        0.0
    }
    
    #[cfg(not(windows))]
    fn get_memory_usage(&self) -> f64 {
        // Placeholder for non-Windows platforms
        0.0
    }
    
    fn get_cpu_usage(&self) -> f64 {
        // Simplified CPU usage calculation
        // In a real implementation, this would track CPU time over intervals
        0.0
    }
    
    fn get_thread_count(&self) -> u32 {
        // Placeholder - would use platform-specific APIs
        std::thread::available_parallelism().map(|n| n.get() as u32).unwrap_or(1)
    }
    
    fn get_handle_count(&self) -> u32 {
        // Placeholder - would use platform-specific APIs
        0
    }
}

/// Complete metrics snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSnapshot {
    pub timestamp: u64,
    pub uptime_seconds: u64,
    pub counters: HashMap<String, u64>,
    pub gauges: HashMap<String, f64>,
    pub histogram_stats: HashMap<String, HistogramStats>,
    pub error_metrics: ErrorMetrics,
    pub performance_metrics: PerformanceMetrics,
    pub system_metrics: SystemMetrics,
}

/// Performance measurement helper
pub struct PerformanceTimer {
    operation: String,
    start_time: Instant,
}

impl PerformanceTimer {
    pub fn new(operation: impl Into<String>) -> Self {
        Self {
            operation: operation.into(),
            start_time: Instant::now(),
        }
    }
}

impl Drop for PerformanceTimer {
    fn drop(&mut self) {
        let duration = self.start_time.elapsed();
        metrics().record_performance(&self.operation, duration);
    }
}

/// Macro for easy performance timing
#[macro_export]
macro_rules! time_operation {
    ($operation:expr, $code:block) => {{
        let _timer = $crate::monitoring::PerformanceTimer::new($operation);
        $code
    }};
}

/// Macro for easy counter increment
#[macro_export]
macro_rules! increment_counter {
    ($name:expr) => {
        $crate::monitoring::metrics().increment_counter($name);
    };
}

/// Macro for easy gauge setting
#[macro_export]
macro_rules! set_gauge {
    ($name:expr, $value:expr) => {
        $crate::monitoring::metrics().set_gauge($name, $value);
    };
}

/// Macro for easy histogram recording
#[macro_export]
macro_rules! record_histogram {
    ($name:expr, $value:expr) => {
        $crate::monitoring::metrics().record_histogram($name, $value);
    };
}

/// Macro for easy error recording
#[macro_export]
macro_rules! record_error {
    ($error:expr) => {
        $crate::monitoring::metrics().record_error($error);
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;
    
    #[test]
    fn test_metrics_collector_creation() {
        let collector = MetricsCollector::new();
        assert_eq!(collector.get_counter("test"), 0);
        assert_eq!(collector.get_gauge("test"), None);
    }
    
    #[test]
    fn test_counter_operations() {
        let collector = MetricsCollector::new();
        
        // Test increment
        collector.increment_counter("test_counter");
        assert_eq!(collector.get_counter("test_counter"), 1);
        
        collector.increment_counter("test_counter");
        assert_eq!(collector.get_counter("test_counter"), 2);
        
        // Test add
        collector.add_to_counter("test_counter", 5);
        assert_eq!(collector.get_counter("test_counter"), 7);
        
        // Test non-existent counter
        assert_eq!(collector.get_counter("non_existent"), 0);
    }
    
    #[test]
    fn test_gauge_operations() {
        let collector = MetricsCollector::new();
        
        // Test set gauge
        collector.set_gauge("test_gauge", 42.5);
        assert_eq!(collector.get_gauge("test_gauge"), Some(42.5));
        
        // Test update gauge
        collector.set_gauge("test_gauge", 100.0);
        assert_eq!(collector.get_gauge("test_gauge"), Some(100.0));
        
        // Test non-existent gauge
        assert_eq!(collector.get_gauge("non_existent"), None);
    }
    
    #[test]
    fn test_histogram_operations() {
        let collector = MetricsCollector::new();
        
        // Record some values
        collector.record_histogram("test_histogram", 10.0);
        collector.record_histogram("test_histogram", 20.0);
        collector.record_histogram("test_histogram", 30.0);
        
        let stats = collector.get_histogram_stats("test_histogram").unwrap();
        assert_eq!(stats.count, 3);
        assert_eq!(stats.min, 10.0);
        assert_eq!(stats.max, 30.0);
        assert_eq!(stats.mean, 20.0);
        
        // Test non-existent histogram
        assert!(collector.get_histogram_stats("non_existent").is_none());
    }
    
    #[test]
    fn test_histogram_percentiles() {
        let mut histogram = Histogram::new();
        
        // Add values 1-100
        for i in 1..=100 {
            histogram.record(i as f64);
        }
        
        let stats = histogram.stats();
        assert_eq!(stats.count, 100);
        assert_eq!(stats.min, 1.0);
        assert_eq!(stats.max, 100.0);
        assert_eq!(stats.mean, 50.5);
        
        // Check percentiles (approximate)
        assert!((stats.p50 - 50.0).abs() < 2.0);
        assert!((stats.p90 - 90.0).abs() < 2.0);
        assert!((stats.p95 - 95.0).abs() < 2.0);
        assert!((stats.p99 - 99.0).abs() < 2.0);
    }
    
    #[test]
    fn test_error_metrics() {
        let collector = MetricsCollector::new();
        
        // Record some errors
        let error1 = HvncError::Configuration("test error 1".to_string());
        let error2 = HvncError::Network(std::io::Error::new(std::io::ErrorKind::TimedOut, "timeout"));
        let error3 = HvncError::Configuration("test error 2".to_string());
        
        collector.record_error(&error1);
        collector.record_error(&error2);
        collector.record_error(&error3);
        
        // Check counters
        assert_eq!(collector.get_counter("errors.total"), 3);
        assert_eq!(collector.get_counter("errors.category.Configuration"), 2);
        assert_eq!(collector.get_counter("errors.category.Network"), 1);
        
        // Check error metrics
        let snapshot = collector.get_metrics_snapshot();
        assert_eq!(snapshot.error_metrics.total_errors, 3);
        assert_eq!(snapshot.error_metrics.errors_by_category.get("Configuration"), Some(&2));
        assert_eq!(snapshot.error_metrics.errors_by_category.get("Network"), Some(&1));
        assert_eq!(snapshot.error_metrics.recent_errors.len(), 3);
    }
    
    #[test]
    fn test_performance_metrics() {
        let collector = MetricsCollector::new();
        
        // Record some performance metrics
        collector.record_performance("test_operation", Duration::from_millis(100));
        collector.record_performance("test_operation", Duration::from_millis(200));
        collector.record_performance("test_operation", Duration::from_millis(150));
        
        let snapshot = collector.get_metrics_snapshot();
        let stats = snapshot.performance_metrics.operation_stats.get("test_operation").unwrap();
        
        assert_eq!(stats.count, 3);
        assert_eq!(stats.min_duration_ms, 100);
        assert_eq!(stats.max_duration_ms, 200);
        assert_eq!(stats.average_duration_ms(), 150.0);
        
        // Check histogram was also updated
        let hist_stats = collector.get_histogram_stats("performance.test_operation.duration_ms").unwrap();
        assert_eq!(hist_stats.count, 3);
        assert_eq!(hist_stats.mean, 150.0);
    }
    
    #[test]
    fn test_performance_timer() {
        let collector = MetricsCollector::new();
        
        {
            let _timer = PerformanceTimer::new("timer_test");
            thread::sleep(Duration::from_millis(10));
        } // Timer drops here and records performance
        
        let snapshot = collector.get_metrics_snapshot();
        let stats = snapshot.performance_metrics.operation_stats.get("timer_test");
        
        assert!(stats.is_some());
        let stats = stats.unwrap();
        assert_eq!(stats.count, 1);
        assert!(stats.min_duration_ms >= 10); // Should be at least 10ms
    }
    
    #[test]
    fn test_metrics_snapshot() {
        let collector = MetricsCollector::new();
        
        // Add some metrics
        collector.increment_counter("test_counter");
        collector.set_gauge("test_gauge", 42.0);
        collector.record_histogram("test_histogram", 10.0);
        collector.record_error(&HvncError::Configuration("test".to_string()));
        collector.record_performance("test_op", Duration::from_millis(100));
        
        let snapshot = collector.get_metrics_snapshot();
        
        assert!(snapshot.timestamp > 0);
        assert!(snapshot.uptime_seconds >= 0);
        assert_eq!(snapshot.counters.get("test_counter"), Some(&1));
        assert_eq!(snapshot.gauges.get("test_gauge"), Some(&42.0));
        assert!(snapshot.histogram_stats.contains_key("test_histogram"));
        assert_eq!(snapshot.error_metrics.total_errors, 1);
        assert!(snapshot.performance_metrics.operation_stats.contains_key("test_op"));
    }
    
    #[test]
    fn test_metrics_reset() {
        let collector = MetricsCollector::new();
        
        // Add some metrics
        collector.increment_counter("test_counter");
        collector.set_gauge("test_gauge", 42.0);
        collector.record_histogram("test_histogram", 10.0);
        
        // Verify metrics exist
        assert_eq!(collector.get_counter("test_counter"), 1);
        assert_eq!(collector.get_gauge("test_gauge"), Some(42.0));
        assert!(collector.get_histogram_stats("test_histogram").is_some());
        
        // Reset metrics
        collector.reset();
        
        // Verify metrics are cleared
        assert_eq!(collector.get_counter("test_counter"), 0);
        assert_eq!(collector.get_gauge("test_gauge"), None);
        assert!(collector.get_histogram_stats("test_histogram").is_none());
    }
    
    #[test]
    fn test_error_metrics_rate_calculation() {
        let mut error_metrics = ErrorMetrics::new();
        
        // Record errors with timestamps
        let base_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        error_metrics.first_error_time = Some(base_time);
        error_metrics.last_error_time = Some(base_time + 60); // 1 minute later
        error_metrics.total_errors = 10;
        
        let rate = error_metrics.error_rate_per_minute();
        assert_eq!(rate, 10.0); // 10 errors in 1 minute
        
        // Test with no time range
        let mut empty_metrics = ErrorMetrics::new();
        assert_eq!(empty_metrics.error_rate_per_minute(), 0.0);
    }
    
    #[test]
    fn test_operation_stats() {
        let mut stats = OperationStats::new();
        
        stats.record(Duration::from_millis(100));
        stats.record(Duration::from_millis(200));
        stats.record(Duration::from_millis(300));
        
        assert_eq!(stats.count, 3);
        assert_eq!(stats.min_duration_ms, 100);
        assert_eq!(stats.max_duration_ms, 300);
        assert_eq!(stats.average_duration_ms(), 200.0);
        assert_eq!(stats.recent_durations.len(), 3);
    }
    
    #[test]
    fn test_histogram_memory_management() {
        let mut histogram = Histogram::new();
        
        // Add more than 1000 values to trigger cleanup
        for i in 0..1500 {
            histogram.record(i as f64);
        }
        
        // Should have cleaned up to keep memory usage reasonable
        assert!(histogram.values.len() <= 1000);
        assert_eq!(histogram.count, 1500); // Count should still be accurate
    }
    
    #[test]
    fn test_operation_stats_memory_management() {
        let mut stats = OperationStats::new();
        
        // Add more than 100 durations to trigger cleanup
        for i in 0..150 {
            stats.record(Duration::from_millis(i));
        }
        
        // Should have cleaned up recent durations
        assert!(stats.recent_durations.len() <= 100);
        assert_eq!(stats.count, 150); // Count should still be accurate
    }
    
    #[test]
    fn test_error_metrics_memory_management() {
        let mut error_metrics = ErrorMetrics::new();
        
        // Add more than 100 errors to trigger cleanup
        for i in 0..150 {
            let error = HvncError::Configuration(format!("error {}", i));
            error_metrics.record_error(&error);
        }
        
        // Should have cleaned up recent errors
        assert!(error_metrics.recent_errors.len() <= 100);
        assert_eq!(error_metrics.total_errors, 150); // Count should still be accurate
    }
    
    #[test]
    fn test_system_metrics() {
        let mut system_metrics = SystemMetrics::new();
        system_metrics.update();
        
        // Basic validation that update doesn't crash
        assert!(system_metrics.last_updated > 0);
        assert!(system_metrics.memory_usage_mb >= 0.0);
        assert!(system_metrics.cpu_usage_percent >= 0.0);
        assert!(system_metrics.thread_count >= 1);
    }
    
    #[test]
    fn test_global_metrics_instance() {
        let metrics1 = metrics();
        let metrics2 = metrics();
        
        // Should be the same instance
        assert!(Arc::ptr_eq(&metrics1, &metrics2));
        
        // Test that it works
        metrics1.increment_counter("global_test");
        assert_eq!(metrics2.get_counter("global_test"), 1);
    }
    
    #[test]
    fn test_histogram_stats_default() {
        let stats = HistogramStats::default();
        assert_eq!(stats.count, 0);
        assert_eq!(stats.min, 0.0);
        assert_eq!(stats.max, 0.0);
        assert_eq!(stats.mean, 0.0);
        assert_eq!(stats.p50, 0.0);
        assert_eq!(stats.p90, 0.0);
        assert_eq!(stats.p95, 0.0);
        assert_eq!(stats.p99, 0.0);
    }
    
    #[test]
    fn test_empty_histogram_stats() {
        let histogram = Histogram::new();
        let stats = histogram.stats();
        
        assert_eq!(stats.count, 0);
        assert_eq!(stats.min, 0.0);
        assert_eq!(stats.max, 0.0);
        assert_eq!(stats.mean, 0.0);
    }
    
    #[test]
    fn test_single_value_histogram() {
        let mut histogram = Histogram::new();
        histogram.record(42.0);
        
        let stats = histogram.stats();
        assert_eq!(stats.count, 1);
        assert_eq!(stats.min, 42.0);
        assert_eq!(stats.max, 42.0);
        assert_eq!(stats.mean, 42.0);
        assert_eq!(stats.p50, 42.0);
        assert_eq!(stats.p90, 42.0);
        assert_eq!(stats.p95, 42.0);
        assert_eq!(stats.p99, 42.0);
    }
}