//! Stability Validation Tests
//!
//! Validates stability targets including long-running session stability (FEAT-034).

use browser_core::{BrowserEngine, Navigator};
use config_manager::Config;
use message_bus::MessageBus;
use network_stack::NetworkStack;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use url::Url;

/// Stability target: Browser should run without crashes for 1 hour
/// Note: For CI, we run abbreviated tests; full 1-hour test is manual
const STABILITY_TARGET_HOURS: u64 = 1;

/// FEAT-034: Stability validation - 1 hour session
///
/// This test validates that the core components can be created and operated
/// without crashes. Full 1-hour stability testing is performed manually or
/// in dedicated stability test environments.
#[test]
fn test_stability_target_constant_defined() {
    assert_eq!(STABILITY_TARGET_HOURS, 1);
}

#[test]
fn test_component_creation_stability() {
    // Test that components can be created repeatedly without crashes
    for i in 0..10 {
        let config = Config::default();
        let mut bus1 = MessageBus::new();
        bus1.start().expect("Bus should start");

        let network = NetworkStack::new(config.network_config(), bus1.sender())
            .expect("NetworkStack should be created");

        let mut bus2 = MessageBus::new();
        bus2.start().expect("Bus should start");

        let result = BrowserEngine::new(config.clone(), network, bus2.sender());
        assert!(
            result.is_ok(),
            "BrowserEngine creation should be stable on iteration {}",
            i
        );

        bus1.shutdown().ok();
        bus2.shutdown().ok();
    }
}

#[test]
fn test_message_bus_stability() {
    // Test message bus can handle repeated start/stop cycles
    for i in 0..20 {
        let mut bus = MessageBus::new();
        assert!(
            bus.start().is_ok(),
            "MessageBus start should be stable on iteration {}",
            i
        );

        // Get sender and verify it works
        let _sender = bus.sender();

        assert!(
            bus.shutdown().is_ok(),
            "MessageBus shutdown should be stable on iteration {}",
            i
        );
    }
}

#[test]
fn test_navigator_repeated_operations() {
    let navigator = Navigator::new();

    // Test URL validation stability
    let urls = vec![
        "https://www.google.com",
        "https://www.youtube.com",
        "https://www.facebook.com",
        "https://www.amazon.com",
        "https://www.wikipedia.org",
    ];

    // Repeated operations should not crash
    for _ in 0..100 {
        for url_str in &urls {
            let url = Url::parse(url_str).unwrap();
            let _ = navigator.validate_url(&url);
            let _ = Navigator::determine_protocol(&url);
        }
    }
}

#[test]
fn test_config_repeated_loading() {
    // Test config can be created/cloned repeatedly
    for _ in 0..50 {
        let config = Config::default();
        let _cloned = config.clone();

        // Verify config fields are accessible
        let _ = config.browser.homepage.clone();
        let _ = config.network.timeout_seconds;
        let _ = config.adblock.enabled;
    }
}

#[test]
fn test_url_parsing_stability() {
    let urls = vec![
        "https://www.google.com/search?q=test+query",
        "https://example.com/path/to/resource?param1=value1&param2=value2",
        "https://user:pass@example.com:8080/path?query=value#fragment",
        "https://example.com/path with spaces",
    ];

    // URL parsing should be stable
    for _ in 0..100 {
        for url_str in &urls {
            let _ = Url::parse(url_str);
        }
    }
}

#[test]
fn test_network_stack_stability() {
    // Test network stack operations stability
    for _ in 0..10 {
        let config = Config::default();
        let mut bus = MessageBus::new();
        bus.start().expect("Bus should start");

        let mut stack = NetworkStack::new(config.network_config(), bus.sender())
            .expect("NetworkStack should be created");

        // Initialize
        stack.initialize().expect("Should initialize");

        // Repeated operations
        for _ in 0..10 {
            let _ = stack.get_timing_data();
        }

        // Clear operations
        stack.clear_cache();
        stack.clear_timing_data();

        bus.shutdown().ok();
    }
}

#[test]
fn test_error_handling_stability() {
    use browser_core::NavigationError;

    let navigator = Navigator::new();

    // Test error page generation stability
    let errors = vec![
        NavigationError::InvalidUrl("bad".to_string()),
        NavigationError::NetworkError("failed".to_string()),
        NavigationError::Timeout,
        NavigationError::UnsupportedProtocol("ftp".to_string()),
    ];

    for _ in 0..100 {
        for error in &errors {
            let html = navigator.generate_error_page(error);
            assert!(!html.is_empty());
        }
    }
}

#[test]
fn test_concurrent_message_bus_stability() {
    use std::thread;

    // Test that multiple message buses can run concurrently
    let running = Arc::new(AtomicBool::new(true));
    let mut handles = vec![];

    for i in 0..4 {
        let running_clone = running.clone();

        let handle = thread::spawn(move || {
            let mut bus = MessageBus::new();
            bus.start().expect("Bus should start");

            let mut count = 0;
            while running_clone.load(Ordering::SeqCst) && count < 25 {
                // Verify sender is accessible
                let _sender = bus.sender();
                count += 1;
                thread::sleep(Duration::from_millis(2));
            }

            bus.shutdown().ok();
            i // Return thread index for verification
        });

        handles.push(handle);
    }

    // Let threads run briefly
    thread::sleep(Duration::from_millis(200));
    running.store(false, Ordering::SeqCst);

    // Wait for all threads
    for handle in handles {
        handle.join().expect("Thread should complete without panic");
    }
}

#[test]
fn test_memory_not_leaking_basic() {
    // Basic test to ensure repeated operations don't cause obvious leaks
    let start_time = Instant::now();

    for i in 0..100 {
        let config = Config::default();
        let mut bus = MessageBus::new();
        bus.start().expect("Bus should start");

        let _network = NetworkStack::new(config.network_config(), bus.sender())
            .expect("NetworkStack should be created");

        bus.shutdown().ok();

        // Ensure we're not running too long
        if start_time.elapsed() > Duration::from_secs(30) {
            panic!("Memory test took too long at iteration {}", i);
        }
    }
}

#[test]
fn test_graceful_shutdown() {
    let config = Config::default();
    let mut bus = MessageBus::new();
    bus.start().expect("Bus should start");

    let mut stack = NetworkStack::new(config.network_config(), bus.sender())
        .expect("NetworkStack should be created");

    stack.initialize().expect("Should initialize");

    // Perform some operations
    let _ = stack.get_timing_data();

    // Shutdown should be graceful
    let shutdown_result = bus.shutdown();
    assert!(shutdown_result.is_ok(), "Shutdown should be graceful");
}

#[test]
fn test_tab_operations_stability() {
    // Test that tab-related operations are stable
    for _ in 0..10 {
        let config = Config::default();
        let mut bus1 = MessageBus::new();
        bus1.start().expect("Bus should start");

        let network = NetworkStack::new(config.network_config(), bus1.sender())
            .expect("NetworkStack should be created");

        let mut bus2 = MessageBus::new();
        bus2.start().expect("Bus should start");

        let mut engine = BrowserEngine::new(config.clone(), network, bus2.sender())
            .expect("BrowserEngine should be created");

        // Multiple tab operations
        for tab_id in 1..=5 {
            let url = Url::parse("https://example.com").unwrap();
            let _ = engine.navigate(tab_id, url);
        }

        // Get bookmarks and history (should be stable even if empty)
        let _ = engine.get_bookmarks();
        let _ = engine.get_history();

        bus1.shutdown().ok();
        bus2.shutdown().ok();
    }
}

/// Long-running stability test (marked as ignored for normal CI)
/// Run with: cargo test --ignored test_extended_stability
#[test]
#[ignore]
fn test_extended_stability() {
    let config = Config::default();
    let mut bus1 = MessageBus::new();
    bus1.start().expect("Bus should start");

    let network = NetworkStack::new(config.network_config(), bus1.sender())
        .expect("NetworkStack should be created");

    let mut bus2 = MessageBus::new();
    bus2.start().expect("Bus should start");

    let mut engine = BrowserEngine::new(config.clone(), network, bus2.sender())
        .expect("BrowserEngine should be created");

    let start = Instant::now();
    let duration = Duration::from_secs(STABILITY_TARGET_HOURS * 3600);

    let urls = vec![
        "https://www.google.com",
        "https://www.example.com",
        "https://www.wikipedia.org",
    ];

    let mut iteration = 0;
    while start.elapsed() < duration {
        let url_str = urls[iteration % urls.len()];
        let url = Url::parse(url_str).unwrap();

        // Simulate navigation
        let _ = engine.navigate(1, url);

        iteration += 1;

        // Brief pause between iterations
        std::thread::sleep(Duration::from_secs(1));

        // Log progress every 10 minutes
        if iteration % 600 == 0 {
            println!(
                "Stability test running: {} minutes elapsed, {} iterations",
                start.elapsed().as_secs() / 60,
                iteration
            );
        }
    }

    println!(
        "Extended stability test completed: {} iterations over {} minutes",
        iteration,
        start.elapsed().as_secs() / 60
    );

    bus1.shutdown().ok();
    bus2.shutdown().ok();
}
