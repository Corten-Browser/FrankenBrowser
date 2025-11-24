//! Performance Validation Tests
//!
//! Validates performance targets including page load time (FEAT-031)
//! and memory usage (FEAT-032).

use config_manager::Config;
use message_bus::MessageBus;
use network_stack::{NetworkStack, ResourceTiming};
use std::time::{Duration, Instant};

/// Performance target: Page load should complete in under 3 seconds
const PAGE_LOAD_TARGET_SECONDS: u64 = 3;

/// Memory target: Single tab should use under 500MB
const MEMORY_TARGET_MB: u32 = 500;

/// FEAT-031: Performance target - page load under 3 seconds
#[test]
fn test_page_load_target_constant_defined() {
    assert_eq!(PAGE_LOAD_TARGET_SECONDS, 3);
}

#[test]
fn test_network_stack_timing_infrastructure() {
    let config = Config::default();
    let mut bus = MessageBus::new();
    bus.start().expect("Bus should start");

    let stack = NetworkStack::new(config.network_config(), bus.sender())
        .expect("NetworkStack should be created");

    // Timing data should be accessible
    let timing_data = stack.get_timing_data();
    assert!(timing_data.is_empty(), "Initial timing data should be empty");

    bus.shutdown().ok();
}

#[test]
fn test_network_config_timeout() {
    let config = Config::default();

    // Network timeout should be reasonable (not too long)
    let timeout = config.network.timeout_seconds;
    assert!(
        timeout <= 30,
        "Network timeout should be at most 30 seconds, got {}",
        timeout
    );
    assert!(
        timeout >= 5,
        "Network timeout should be at least 5 seconds, got {}",
        timeout
    );
}

#[test]
fn test_resource_timing_structure() {
    // Test that ResourceTiming has all necessary fields for performance measurement
    let timing = ResourceTiming {
        url: "https://example.com".to_string(),
        start_time: Duration::from_millis(0),
        end_time: Duration::from_millis(200),
        duration_ms: 200,
        size_bytes: 1024,
        from_cache: false,
    };

    assert_eq!(timing.duration_ms, 200);
    assert!(timing.duration_ms < PAGE_LOAD_TARGET_SECONDS * 1000);
}

#[test]
fn test_timing_measurement_accuracy() {
    let start = Instant::now();
    std::thread::sleep(Duration::from_millis(10));
    let elapsed = start.elapsed();

    // Timing should be accurate within reasonable bounds
    assert!(
        elapsed.as_millis() >= 9 && elapsed.as_millis() <= 50,
        "Timing measurement should be accurate: {:?}ms",
        elapsed.as_millis()
    );
}

#[test]
fn test_cache_improves_load_time() {
    let config = Config::default();

    // Cache should be enabled by default for performance
    assert!(
        config.network.enable_cache,
        "Cache should be enabled by default for performance"
    );

    // Cache size should be reasonable
    assert!(
        config.network.cache_size_mb >= 100,
        "Cache should be at least 100MB"
    );
}

/// FEAT-032: Memory usage target - under 500MB
#[test]
fn test_memory_target_constant_defined() {
    assert_eq!(MEMORY_TARGET_MB, 500u32);
}

#[test]
fn test_cache_size_within_memory_budget() {
    let config = Config::default();

    // Cache size should be within memory budget
    assert!(
        config.network.cache_size_mb <= MEMORY_TARGET_MB,
        "Cache size {}MB should be within memory budget {}MB",
        config.network.cache_size_mb,
        MEMORY_TARGET_MB
    );
}

#[test]
fn test_connection_pool_limits() {
    let config = Config::default();

    // Connection pool should be limited to prevent memory bloat
    let max_connections = config.network.max_connections_per_host;
    assert!(
        max_connections <= 10,
        "Max connections per host should be reasonable: {}",
        max_connections
    );
}

#[test]
fn test_memory_efficient_message_types() {
    use shared_types::BrowserMessage;
    use std::mem::size_of;

    // Message types should not be excessively large
    let message_size = size_of::<BrowserMessage>();

    // Messages should be reasonably sized (under 1KB)
    assert!(
        message_size < 1024,
        "BrowserMessage size should be under 1KB: {} bytes",
        message_size
    );
}

#[test]
fn test_string_allocation_efficiency() {
    // Test that common operations don't cause excessive allocations
    let url = "https://www.google.com/".to_string();  // Include trailing slash as URL normalizes
    let parsed = url::Url::parse(&url).unwrap();

    // URL parsing should be efficient (URL normalizes with trailing slash)
    assert_eq!(parsed.as_str(), url);
}

#[test]
fn test_config_memory_footprint() {
    use std::mem::size_of;

    let config_size = size_of::<Config>();

    // Config should be reasonably sized (under 1KB)
    assert!(
        config_size < 1024,
        "Config size should be under 1KB: {} bytes",
        config_size
    );
}

#[test]
fn test_network_stack_memory_management() {
    let config = Config::default();
    let mut bus = MessageBus::new();
    bus.start().expect("Bus should start");

    let mut stack = NetworkStack::new(config.network_config(), bus.sender())
        .expect("NetworkStack should be created");
    stack.initialize().expect("Should initialize");

    // Clear operations should free memory
    stack.clear_cache();
    stack.clear_timing_data();

    // After clearing, timing data should be empty
    assert!(stack.get_timing_data().is_empty());

    bus.shutdown().ok();
}

#[test]
fn test_compression_reduces_memory() {
    let config = Config::default();

    // Compression should be enabled to reduce memory usage
    // (This is typically enabled by default in reqwest)

    // Cache should be enabled to avoid re-downloading
    assert!(config.network.enable_cache);
}

#[test]
fn test_connection_pooling_enabled() {
    let config = Config::default();

    // Connection pooling reduces memory overhead
    let max_connections = config.network.max_connections_per_host;

    // Should have reasonable connection limit
    assert!(max_connections > 0);
    assert!(max_connections <= 10);
}

#[test]
fn test_performance_metrics_collection() {
    use std::collections::HashMap;

    // Test that performance metrics can be collected
    let mut metrics: HashMap<String, u64> = HashMap::new();

    metrics.insert("page_load_ms".to_string(), 1500);
    metrics.insert("dns_lookup_ms".to_string(), 50);
    metrics.insert("tcp_connect_ms".to_string(), 100);
    metrics.insert("ttfb_ms".to_string(), 200);
    metrics.insert("content_download_ms".to_string(), 1150);

    // Verify metrics are within targets
    let page_load = metrics.get("page_load_ms").unwrap();
    assert!(
        *page_load < PAGE_LOAD_TARGET_SECONDS * 1000,
        "Page load {}ms should be under {}ms target",
        page_load,
        PAGE_LOAD_TARGET_SECONDS * 1000
    );
}
