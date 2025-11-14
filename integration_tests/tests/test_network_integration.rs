//! Integration tests for NetworkStack component integration
//!
//! These tests verify that the NetworkStack correctly integrates with other
//! components using REAL components (no mocking).

mod common;

use common::{setup_message_bus, test_network_config};
use network_stack::NetworkStack;
use url::Url;

#[test]
fn test_network_stack_creation_with_message_bus() {
    // REAL components - MessageBus and NetworkStack
    let mut bus = setup_message_bus();
    let sender = bus.sender();
    let config = test_network_config();

    // Create NetworkStack with real message bus sender
    let result = NetworkStack::new(config, sender);
    assert!(result.is_ok(), "NetworkStack should be created successfully");

    bus.shutdown().expect("Failed to shutdown bus");
}

#[test]
fn test_network_stack_initialization() {
    let mut bus = setup_message_bus();
    let sender = bus.sender();
    let config = test_network_config();

    let mut stack = NetworkStack::new(config, sender).unwrap();

    // Initialize the stack
    let result = stack.initialize();
    assert!(result.is_ok(), "NetworkStack should initialize successfully");

    bus.shutdown().expect("Failed to shutdown bus");
}

#[test]
fn test_network_stack_initialization_idempotency() {
    let mut bus = setup_message_bus();
    let sender = bus.sender();
    let config = test_network_config();

    let mut stack = NetworkStack::new(config, sender).unwrap();

    // First initialization should succeed
    assert!(stack.initialize().is_ok());

    // Second initialization should fail
    let result = stack.initialize();
    assert!(result.is_err(), "Double initialization should fail");

    bus.shutdown().expect("Failed to shutdown bus");
}

#[test]
fn test_network_stack_get_timing_data_empty() {
    let mut bus = setup_message_bus();
    let sender = bus.sender();
    let config = test_network_config();

    let stack = NetworkStack::new(config, sender).unwrap();

    // Before any fetches, timing data should be empty
    let timings = stack.get_timing_data();
    assert_eq!(timings.len(), 0, "Timing data should be empty initially");

    bus.shutdown().expect("Failed to shutdown bus");
}

#[test]
fn test_network_stack_cache_management() {
    let mut bus = setup_message_bus();
    let sender = bus.sender();
    let config = test_network_config();

    let mut stack = NetworkStack::new(config, sender).unwrap();
    stack.initialize().unwrap();

    // Clear cache should not panic
    stack.clear_cache();

    bus.shutdown().expect("Failed to shutdown bus");
}

#[test]
fn test_network_stack_timing_data_clear() {
    let mut bus = setup_message_bus();
    let sender = bus.sender();
    let config = test_network_config();

    let mut stack = NetworkStack::new(config, sender).unwrap();

    // Clear timing data should not panic
    stack.clear_timing_data();

    let timings = stack.get_timing_data();
    assert_eq!(timings.len(), 0);

    bus.shutdown().expect("Failed to shutdown bus");
}

#[test]
fn test_network_stack_with_cache_disabled() {
    let mut bus = setup_message_bus();
    let sender = bus.sender();

    let mut config = test_network_config();
    config.enable_cache = false;

    let mut stack = NetworkStack::new(config, sender).unwrap();
    stack.initialize().unwrap();

    // Cache operations should still work (just no-ops)
    stack.clear_cache();

    bus.shutdown().expect("Failed to shutdown bus");
}

#[test]
fn test_network_stack_with_cookies_disabled() {
    let mut bus = setup_message_bus();
    let sender = bus.sender();

    let mut config = test_network_config();
    config.enable_cookies = false;

    let stack = NetworkStack::new(config, sender);
    assert!(stack.is_ok(), "NetworkStack should work with cookies disabled");

    bus.shutdown().expect("Failed to shutdown bus");
}

// Network tests marked as #[ignore] - require actual network access
#[tokio::test]
#[ignore]
async fn test_network_stack_fetch_integration() {
    let mut bus = setup_message_bus();
    let sender = bus.sender();
    let config = test_network_config();

    let mut stack = NetworkStack::new(config, sender).unwrap();
    stack.initialize().unwrap();

    // Try to fetch a real URL
    let url = Url::parse("https://httpbin.org/get").unwrap();
    let result = stack.fetch(url).await;

    // This should succeed if network is available
    assert!(result.is_ok(), "Fetch should succeed with network access");

    // Timing data should be recorded
    let timings = stack.get_timing_data();
    assert_eq!(timings.len(), 1);
    assert!(!timings[0].from_cache);

    bus.shutdown().expect("Failed to shutdown bus");
}
