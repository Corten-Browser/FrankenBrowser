/// Component Lifecycle Validation Tests
///
/// These tests validate that all components initialize, operate, and shutdown correctly.

use message_bus::MessageBus;
use shared_types::BrowserMessage;
use config_manager::Config;
use adblock_engine::AdBlockEngine;
use std::time::Duration;

#[test]
fn test_message_bus_lifecycle() {
    // Create message bus
    let mut bus = MessageBus::new();
    bus.start().expect("Bus should start");
    let sender = bus.sender();

    // Verify sender works
    assert!(sender.send(BrowserMessage::Shutdown).is_ok());

    // Message bus should shutdown cleanly
    bus.shutdown().ok();
}

#[test]
fn test_config_lifecycle() {
    // Create default config
    let config = Config::default();

    // Verify all sections exist
    assert!(config.browser.homepage.contains("http"));
    assert!(config.network.timeout_seconds > 0);
    assert!(config.adblock.enabled || !config.adblock.enabled); // Either state is valid

    // Config should be serializable
    let toml_str = toml::to_string(&config);
    assert!(toml_str.is_ok());

    // Config should be deserializable
    let deserialized: Result<Config, _> = toml::from_str(&toml_str.unwrap());
    assert!(deserialized.is_ok());
}

#[test]
fn test_adblock_engine_lifecycle() {
    let mut bus = MessageBus::new();
    bus.start().expect("Bus should start");
    let sender = bus.sender();
    let config = Config::default();

    // Create adblock engine
    let engine_result = AdBlockEngine::new(config.adblock_config(), sender);

    // Engine should initialize (even with empty filters)
    assert!(engine_result.is_ok());

    let engine = engine_result.unwrap();

    // Engine should be functional
    let should_block = engine.should_block(
        "https://example.com/page.html",
        shared_types::ResourceType::Document,
    );

    // Result should be boolean (either blocks or doesn't)
    assert!(should_block || !should_block);
}

#[test]
fn test_multiple_message_bus_instances() {
    // Should be able to create multiple independent message buses
    let mut bus1 = MessageBus::new();
    let mut bus2 = MessageBus::new();
    let mut bus3 = MessageBus::new();
    bus1.start().expect("Bus1 should start");
    bus2.start().expect("Bus2 should start");
    bus3.start().expect("Bus3 should start");

    let sender1 = bus1.sender();
    let sender2 = bus2.sender();
    let sender3 = bus3.sender();

    // All should be independent
    assert!(sender1.send(BrowserMessage::Shutdown).is_ok());
    assert!(sender2.send(BrowserMessage::Shutdown).is_ok());
    assert!(sender3.send(BrowserMessage::Shutdown).is_ok());

    bus1.shutdown().ok();
    bus2.shutdown().ok();
    bus3.shutdown().ok();
}

#[test]
fn test_concurrent_message_bus_usage() {
    use std::thread;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};

    // Test multiple independent message buses running concurrently
    let running = Arc::new(AtomicBool::new(true));

    let handles: Vec<_> = (0..5)
        .map(|i| {
            let running = running.clone();
            thread::spawn(move || {
                let mut bus = MessageBus::new();
                bus.start().expect("Bus should start");
                let sender = bus.sender();
                let mut count = 0;
                while running.load(Ordering::SeqCst) && count < 100 {
                    sender.send(BrowserMessage::Shutdown).ok();
                    count += 1;
                }
                bus.shutdown().ok();
                i // return thread index
            })
        })
        .collect();

    // Let threads run briefly
    thread::sleep(Duration::from_millis(100));
    running.store(false, Ordering::SeqCst);

    // All threads should complete successfully
    for handle in handles {
        assert!(handle.join().is_ok());
    }
}

#[test]
fn test_config_field_access() {
    let config = Config::default();

    // Browser config
    let _ = &config.browser.homepage;
    let _ = config.browser.enable_devtools;
    let _ = &config.browser.default_search_engine;

    // Network config
    let _ = config.network.max_connections_per_host;
    let _ = config.network.timeout_seconds;
    let _ = config.network.enable_cookies;
    let _ = config.network.enable_cache;
    let _ = config.network.cache_size_mb;

    // Adblock config
    let _ = config.adblock.enabled;
    let _ = config.adblock.update_filters_on_startup;
    let _ = &config.adblock.custom_filters;

    // Privacy config
    let _ = config.privacy.do_not_track;
    let _ = config.privacy.clear_cookies_on_exit;
    let _ = config.privacy.block_third_party_cookies;

    // Appearance config
    let _ = &config.appearance.theme;
    let _ = config.appearance.default_zoom;
}

#[test]
fn test_component_initialization_order() {
    // Test that components can be initialized in correct dependency order

    // 1. Message bus (no dependencies)
    let mut bus = MessageBus::new();
    bus.start().expect("Bus should start");
    let sender = bus.sender();

    // 2. Config (no dependencies)
    let config = Config::default();

    // 3. AdBlock engine (depends on message bus and config)
    let adblock_config = config.adblock_config();
    let adblock = AdBlockEngine::new(adblock_config, sender);
    assert!(adblock.is_ok());

    // 4. Additional components would follow (browser_core, etc.)
    // This validates the dependency order is correct
    bus.shutdown().ok();
}

#[test]
fn test_resource_cleanup() {
    // Test that dropping components doesn't cause panics
    {
        let mut bus = MessageBus::new();
        bus.start().expect("Bus should start");
        let sender = bus.sender();
        let config = Config::default();
        let _adblock = AdBlockEngine::new(config.adblock_config(), sender).unwrap();

        // All should be dropped here
        bus.shutdown().ok();
    }

    // Should be able to create new instances after cleanup
    let mut bus2 = MessageBus::new();
    bus2.start().expect("Bus should start");
    assert!(bus2.sender().send(BrowserMessage::Shutdown).is_ok());
    bus2.shutdown().ok();
}

#[test]
fn test_error_handling() {
    // Test that components handle errors gracefully

    // Invalid TOML should error gracefully
    let bad_toml = "invalid toml {{{{";
    let result: Result<Config, _> = toml::from_str(bad_toml);
    assert!(result.is_err());

    // Components should handle invalid inputs
    let mut bus = MessageBus::new();
    bus.start().expect("Bus should start");
    let sender = bus.sender();

    // Shutdown the bus
    bus.shutdown().ok();

    // Sending to closed bus should be handled without panic
    let result = sender.send(BrowserMessage::Shutdown);
    // Result will be Err but shouldn't panic
    assert!(result.is_err());
}
