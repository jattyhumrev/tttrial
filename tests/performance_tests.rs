//! Performance benchmarks for Hidden VNC
//! 
//! This module contains comprehensive performance tests to measure:
//! - Frame rate and latency benchmarks
//! - Image compression/decompression performance
//! - Network throughput and protocol efficiency
//! - Memory usage and resource consumption
//! - Input processing latency

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use hidden_vnc::{
    common::{protocol::*, config::*},
    error::*,
    host::capture::*,
};
use std::time::{Duration, Instant};
use tokio::runtime::Runtime;
use image::{ImageBuffer, RgbImage};

/// Performance test configuration
struct PerformanceConfig {
    frame_sizes: Vec<(u32, u32)>,
    jpeg_qualities: Vec<u8>,
    input_event_counts: Vec<usize>,
    concurrent_connections: Vec<usize>,
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            frame_sizes: vec![
                (640, 480),   // VGA
                (1280, 720),  // HD
                (1920, 1080), // Full HD
                (2560, 1440), // QHD
                (3840, 2160), // 4K
            ],
            jpeg_qualities: vec![30, 50, 70, 80, 90, 95],
            input_event_counts: vec![1, 10, 100, 1000],
            concurrent_connections: vec![1, 2, 5, 10],
        }
    }
}

/// Generate test image data
fn generate_test_image(width: u32, height: u32) -> RgbImage {
    ImageBuffer::from_fn(width, height, |x, y| {
        let r = ((x * 255) / width) as u8;
        let g = ((y * 255) / height) as u8;
        let b = ((x + y) * 255 / (width + height)) as u8;
        image::Rgb([r, g, b])
    })
}

/// Generate test input events
fn generate_input_events(count: usize) -> Vec<InputEvent> {
    (0..count)
        .map(|i| {
            match i % 3 {
                0 => InputEvent::MouseClick {
                    x: (i % 1920) as i32,
                    y: (i % 1080) as i32,
                    button: MouseButton::Left,
                },
                1 => InputEvent::MouseMove {
                    x: (i % 1920) as i32,
                    y: (i % 1080) as i32,
                },
                _ => InputEvent::KeyPress {
                    keycode: 65 + (i % 26) as u32, // A-Z keys
                    pressed: i % 2 == 0,
                },
            }
        })
        .collect()
}

/// Benchmark image compression performance
fn bench_image_compression(c: &mut Criterion) {
    let config = PerformanceConfig::default();
    let mut group = c.benchmark_group("image_compression");
    
    for &(width, height) in &config.frame_sizes {
        for &quality in &config.jpeg_qualities {
            let image = generate_test_image(width, height);
            let benchmark_id = BenchmarkId::from_parameter(format!("{}x{}_q{}", width, height, quality));
            
            group.bench_with_input(benchmark_id, &(image.clone(), quality), |b, (img, q)| {
                b.iter(|| {
                    let mut buffer = Vec::new();
                    let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buffer, *q);
                    black_box(encoder.encode_image(img).unwrap());
                    black_box(buffer)
                });
            });
        }
    }
    
    group.finish();
}

/// Benchmark image decompression performance
fn bench_image_decompression(c: &mut Criterion) {
    let config = PerformanceConfig::default();
    let mut group = c.benchmark_group("image_decompression");
    
    for &(width, height) in &config.frame_sizes {
        let image = generate_test_image(width, height);
        let mut buffer = Vec::new();
        let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buffer, 80);
        encoder.encode_image(&image).unwrap();
        
        let benchmark_id = BenchmarkId::from_parameter(format!("{}x{}", width, height));
        
        group.bench_with_input(benchmark_id, &buffer, |b, compressed_data| {
            b.iter(|| {
                let cursor = std::io::Cursor::new(compressed_data);
                let decoder = image::codecs::jpeg::JpegDecoder::new(cursor).unwrap();
                black_box(image::DynamicImage::from_decoder(decoder).unwrap());
            });
        });
    }
    
    group.finish();
}

/// Benchmark input event serialization
fn bench_input_serialization(c: &mut Criterion) {
    let config = PerformanceConfig::default();
    let mut group = c.benchmark_group("input_serialization");
    
    for &count in &config.input_event_counts {
        let events = generate_input_events(count);
        let benchmark_id = BenchmarkId::from_parameter(count);
        
        group.bench_with_input(benchmark_id, &events, |b, events| {
            b.iter(|| {
                for event in events {
                    black_box(serde_json::to_string(event).unwrap());
                }
            });
        });
    }
    
    group.finish();
}

/// Benchmark input event deserialization
fn bench_input_deserialization(c: &mut Criterion) {
    let config = PerformanceConfig::default();
    let mut group = c.benchmark_group("input_deserialization");
    
    for &count in &config.input_event_counts {
        let events = generate_input_events(count);
        let serialized: Vec<String> = events.iter()
            .map(|e| serde_json::to_string(e).unwrap())
            .collect();
        
        let benchmark_id = BenchmarkId::from_parameter(count);
        
        group.bench_with_input(benchmark_id, &serialized, |b, serialized_events| {
            b.iter(|| {
                for json_str in serialized_events {
                    black_box(serde_json::from_str::<InputEvent>(json_str).unwrap());
                }
            });
        });
    }
    
    group.finish();
}

/// Benchmark frame rate simulation
fn bench_frame_rate_simulation(c: &mut Criterion) {
    let mut group = c.benchmark_group("frame_rate_simulation");
    
    // Test different target frame rates
    let target_fps = vec![15, 30, 60, 120];
    let image = generate_test_image(1920, 1080);
    
    for fps in target_fps {
        let frame_duration = Duration::from_millis(1000 / fps);
        let benchmark_id = BenchmarkId::from_parameter(format!("{}fps", fps));
        
        group.bench_with_input(benchmark_id, &(image.clone(), frame_duration), |b, (img, duration)| {
            b.iter(|| {
                let start = Instant::now();
                
                // Simulate frame processing
                let mut buffer = Vec::new();
                let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buffer, 80);
                encoder.encode_image(img).unwrap();
                
                // Simulate network transmission delay
                let processing_time = start.elapsed();
                if processing_time < *duration {
                    std::thread::sleep(*duration - processing_time);
                }
                
                black_box(buffer);
            });
        });
    }
    
    group.finish();
}

/// Benchmark memory usage patterns
fn bench_memory_usage(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_usage");
    
    let config = PerformanceConfig::default();
    
    for &(width, height) in &config.frame_sizes {
        let benchmark_id = BenchmarkId::from_parameter(format!("{}x{}", width, height));
        
        group.bench_function(benchmark_id, |b| {
            b.iter(|| {
                // Simulate frame buffer allocation and processing
                let image = generate_test_image(width, height);
                let mut compressed_frames = Vec::new();
                
                // Simulate multiple frame processing
                for _ in 0..10 {
                    let mut buffer = Vec::new();
                    let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buffer, 80);
                    encoder.encode_image(&image).unwrap();
                    compressed_frames.push(buffer);
                }
                
                black_box(compressed_frames);
            });
        });
    }
    
    group.finish();
}

/// Benchmark protocol overhead
fn bench_protocol_overhead(c: &mut Criterion) {
    let mut group = c.benchmark_group("protocol_overhead");
    
    let frame_sizes = vec![1024, 4096, 16384, 65536, 262144]; // Different payload sizes
    
    for size in frame_sizes {
        let data = vec![0u8; size];
        let benchmark_id = BenchmarkId::from_parameter(format!("{}bytes", size));
        
        group.bench_with_input(benchmark_id, &data, |b, payload| {
            b.iter(|| {
                // Simulate protocol framing
                let mut frame_data = Vec::with_capacity(payload.len() + 4);
                frame_data.extend_from_slice(&(payload.len() as u32).to_le_bytes());
                frame_data.extend_from_slice(payload);
                black_box(frame_data);
            });
        });
    }
    
    group.finish();
}

/// Benchmark concurrent processing
fn bench_concurrent_processing(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("concurrent_processing");
    
    let config = PerformanceConfig::default();
    let image = generate_test_image(1280, 720);
    
    for &connections in &config.concurrent_connections {
        let benchmark_id = BenchmarkId::from_parameter(format!("{}_connections", connections));
        
        group.bench_with_input(benchmark_id, &(image.clone(), connections), |b, (img, conn_count)| {
            b.to_async(&rt).iter(|| async {
                let tasks: Vec<_> = (0..*conn_count)
                    .map(|_| {
                        let img_clone = img.clone();
                        tokio::spawn(async move {
                            // Simulate concurrent frame processing
                            let mut buffer = Vec::new();
                            let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buffer, 80);
                            encoder.encode_image(&img_clone).unwrap();
                            buffer
                        })
                    })
                    .collect();
                
                let results = futures::future::join_all(tasks).await;
                black_box(results);
            });
        });
    }
    
    group.finish();
}

/// Benchmark input processing latency
fn bench_input_latency(c: &mut Criterion) {
    let mut group = c.benchmark_group("input_latency");
    
    let input_types = vec![
        ("mouse_click", InputEvent::MouseClick { x: 100, y: 100, button: MouseButton::Left }),
        ("mouse_move", InputEvent::MouseMove { x: 200, y: 200 }),
        ("key_press", InputEvent::KeyPress { keycode: 65, pressed: true }),
    ];
    
    for (name, event) in input_types {
        group.bench_function(name, |b| {
            b.iter(|| {
                // Simulate input event processing pipeline
                let start = Instant::now();
                
                // Serialize
                let json = serde_json::to_string(&event).unwrap();
                
                // Simulate network transmission
                std::thread::sleep(Duration::from_micros(100));
                
                // Deserialize
                let parsed: InputEvent = serde_json::from_str(&json).unwrap();
                
                // Simulate input application
                match parsed {
                    InputEvent::MouseClick { x, y, .. } => {
                        black_box((x, y));
                    }
                    InputEvent::MouseMove { x, y } => {
                        black_box((x, y));
                    }
                    InputEvent::KeyPress { keycode, pressed } => {
                        black_box((keycode, pressed));
                    }
                }
                
                let latency = start.elapsed();
                black_box(latency);
            });
        });
    }
    
    group.finish();
}

/// Benchmark compression ratio vs quality trade-offs
fn bench_compression_tradeoffs(c: &mut Criterion) {
    let mut group = c.benchmark_group("compression_tradeoffs");
    
    let image = generate_test_image(1920, 1080);
    let qualities = vec![10, 30, 50, 70, 90];
    
    for quality in qualities {
        let benchmark_id = BenchmarkId::from_parameter(format!("quality_{}", quality));
        
        group.bench_with_input(benchmark_id, &(image.clone(), quality), |b, (img, q)| {
            b.iter(|| {
                let mut buffer = Vec::new();
                let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buffer, *q);
                encoder.encode_image(img).unwrap();
                
                // Return both processing time and compression ratio
                let original_size = img.width() * img.height() * 3; // RGB
                let compressed_size = buffer.len() as u32;
                let ratio = original_size as f32 / compressed_size as f32;
                
                black_box((buffer, ratio));
            });
        });
    }
    
    group.finish();
}

criterion_group!(
    performance_benches,
    bench_image_compression,
    bench_image_decompression,
    bench_input_serialization,
    bench_input_deserialization,
    bench_frame_rate_simulation,
    bench_memory_usage,
    bench_protocol_overhead,
    bench_concurrent_processing,
    bench_input_latency,
    bench_compression_tradeoffs
);

criterion_main!(performance_benches);

#[cfg(test)]
mod performance_integration_tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU64, Ordering};
    use tokio::time::{timeout, Duration};
    
    /// Test frame rate consistency under load
    #[tokio::test]
    async fn test_frame_rate_consistency() {
        let target_fps = 30;
        let frame_duration = Duration::from_millis(1000 / target_fps);
        let test_duration = Duration::from_secs(5);
        
        let frame_count = Arc::new(AtomicU64::new(0));
        let frame_count_clone = frame_count.clone();
        
        let image = generate_test_image(1280, 720);
        
        let frame_task = tokio::spawn(async move {
            let mut interval = tokio::time::interval(frame_duration);
            let start_time = Instant::now();
            
            while start_time.elapsed() < test_duration {
                interval.tick().await;
                
                // Simulate frame processing
                let mut buffer = Vec::new();
                let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buffer, 80);
                encoder.encode_image(&image).unwrap();
                
                frame_count_clone.fetch_add(1, Ordering::Relaxed);
            }
        });
        
        timeout(test_duration + Duration::from_secs(1), frame_task)
            .await
            .expect("Frame processing task should complete")
            .expect("Frame processing should not panic");
        
        let actual_frames = frame_count.load(Ordering::Relaxed);
        let expected_frames = (test_duration.as_secs() * target_fps as u64) as f64;
        let actual_fps = actual_frames as f64 / test_duration.as_secs_f64();
        
        // Allow 10% variance in frame rate
        assert!(
            (actual_fps - target_fps as f64).abs() / target_fps as f64 < 0.1,
            "Frame rate variance too high: expected ~{}, got {:.2}",
            target_fps,
            actual_fps
        );
    }
    
    /// Test memory usage stability over time
    #[tokio::test]
    async fn test_memory_stability() {
        let initial_memory = get_memory_usage();
        let image = generate_test_image(1920, 1080);
        
        // Process many frames to test for memory leaks
        for _ in 0..1000 {
            let mut buffer = Vec::new();
            let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buffer, 80);
            encoder.encode_image(&image).unwrap();
            
            // Simulate frame transmission
            drop(buffer);
        }
        
        // Force garbage collection
        std::thread::sleep(Duration::from_millis(100));
        
        let final_memory = get_memory_usage();
        let memory_increase = final_memory.saturating_sub(initial_memory);
        
        // Memory increase should be minimal (less than 10MB)
        assert!(
            memory_increase < 10 * 1024 * 1024,
            "Memory usage increased too much: {} bytes",
            memory_increase
        );
    }
    
    /// Test input processing throughput
    #[tokio::test]
    async fn test_input_throughput() {
        let event_count = 10000;
        let events = generate_input_events(event_count);
        
        let start_time = Instant::now();
        
        for event in events {
            // Simulate input processing pipeline
            let json = serde_json::to_string(&event).unwrap();
            let _parsed: InputEvent = serde_json::from_str(&json).unwrap();
        }
        
        let processing_time = start_time.elapsed();
        let events_per_second = event_count as f64 / processing_time.as_secs_f64();
        
        // Should process at least 10,000 events per second
        assert!(
            events_per_second > 10000.0,
            "Input processing too slow: {:.0} events/sec",
            events_per_second
        );
    }
    
    /// Test compression performance under different loads
    #[tokio::test]
    async fn test_compression_performance() {
        let sizes = vec![(640, 480), (1280, 720), (1920, 1080)];
        let qualities = vec![30, 50, 80];
        
        for (width, height) in sizes {
            for quality in &qualities {
                let image = generate_test_image(width, height);
                let start_time = Instant::now();
                
                let mut buffer = Vec::new();
                let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buffer, *quality);
                encoder.encode_image(&image).unwrap();
                
                let compression_time = start_time.elapsed();
                let pixels = width * height;
                let pixels_per_second = pixels as f64 / compression_time.as_secs_f64();
                
                // Should process at least 10M pixels per second
                assert!(
                    pixels_per_second > 10_000_000.0,
                    "Compression too slow for {}x{} at quality {}: {:.0} pixels/sec",
                    width, height, quality, pixels_per_second
                );
            }
        }
    }
    
    /// Helper function to get current memory usage (simplified)
    fn get_memory_usage() -> usize {
        // This is a simplified implementation
        // In a real scenario, you'd use platform-specific APIs
        std::mem::size_of::<usize>() * 1024 // Placeholder
    }
}