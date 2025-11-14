//! Integration tests for AdBlockEngine component integration
//!
//! These tests verify that the AdBlockEngine correctly integrates with other
//! components using REAL components (no mocking).

mod common;

use adblock_engine::AdBlockEngine;
use common::{setup_message_bus, test_adblock_config};
use shared_types::ResourceType;

#[test]
fn test_adblock_engine_creation_with_message_bus() {
    // REAL components - MessageBus and AdBlockEngine
    let mut bus = setup_message_bus();
    let sender = bus.sender();
    let config = test_adblock_config();

    // Create AdBlockEngine with real message bus sender
    let result = AdBlockEngine::new(config, sender);
    assert!(result.is_ok(), "AdBlockEngine should be created successfully");

    bus.shutdown().expect("Failed to shutdown bus");
}

#[test]
fn test_adblock_engine_initialization() {
    let mut bus = setup_message_bus();
    let sender = bus.sender();
    let config = test_adblock_config();

    let mut engine = AdBlockEngine::new(config, sender).unwrap();

    // Initialize the engine
    let result = engine.initialize();
    assert!(result.is_ok(), "AdBlockEngine should initialize successfully");

    bus.shutdown().expect("Failed to shutdown bus");
}

#[test]
fn test_adblock_engine_blocks_custom_filter() {
    let mut bus = setup_message_bus();
    let sender = bus.sender();
    let config = test_adblock_config();

    let mut engine = AdBlockEngine::new(config, sender).unwrap();
    engine.initialize().unwrap();

    // Test custom filter: ||ads.example.com^
    let should_block = engine.should_block(
        "https://ads.example.com/banner.js",
        ResourceType::Script,
    );

    assert!(should_block, "Should block ads.example.com based on custom filter");

    bus.shutdown().expect("Failed to shutdown bus");
}

#[test]
fn test_adblock_engine_blocks_doubleclick() {
    let mut bus = setup_message_bus();
    let sender = bus.sender();
    let config = test_adblock_config();

    let mut engine = AdBlockEngine::new(config, sender).unwrap();
    engine.initialize().unwrap();

    // Test custom filter: ||doubleclick.net^
    let should_block = engine.should_block(
        "https://doubleclick.net/ad.js",
        ResourceType::Script,
    );

    assert!(should_block, "Should block doubleclick.net based on custom filter");

    bus.shutdown().expect("Failed to shutdown bus");
}

#[test]
fn test_adblock_engine_allows_non_ad_url() {
    let mut bus = setup_message_bus();
    let sender = bus.sender();
    let config = test_adblock_config();

    let mut engine = AdBlockEngine::new(config, sender).unwrap();
    engine.initialize().unwrap();

    // Regular URL should not be blocked
    let should_block = engine.should_block(
        "https://example.com/script.js",
        ResourceType::Script,
    );

    assert!(!should_block, "Should not block regular example.com");

    bus.shutdown().expect("Failed to shutdown bus");
}

#[test]
fn test_adblock_engine_handles_different_resource_types() {
    let mut bus = setup_message_bus();
    let sender = bus.sender();
    let config = test_adblock_config();

    let mut engine = AdBlockEngine::new(config, sender).unwrap();
    engine.initialize().unwrap();

    // Test same URL with different resource types
    let ad_url = "https://ads.example.com/banner";

    let blocked_script = engine.should_block(ad_url, ResourceType::Script);
    let blocked_image = engine.should_block(ad_url, ResourceType::Image);
    let blocked_stylesheet = engine.should_block(ad_url, ResourceType::Stylesheet);

    // All should be blocked regardless of type
    assert!(blocked_script, "Script should be blocked");
    assert!(blocked_image, "Image should be blocked");
    assert!(blocked_stylesheet, "Stylesheet should be blocked");

    bus.shutdown().expect("Failed to shutdown bus");
}

#[test]
fn test_adblock_engine_with_disabled_config() {
    let mut bus = setup_message_bus();
    let sender = bus.sender();

    let mut config = test_adblock_config();
    config.enabled = false;

    let engine = AdBlockEngine::new(config, sender);
    assert!(engine.is_ok(), "AdBlockEngine should work when disabled");

    bus.shutdown().expect("Failed to shutdown bus");
}

#[test]
fn test_adblock_engine_initialization_idempotency() {
    let mut bus = setup_message_bus();
    let sender = bus.sender();
    let config = test_adblock_config();

    let mut engine = AdBlockEngine::new(config, sender).unwrap();

    // First initialization should succeed
    assert!(engine.initialize().is_ok());

    // Second initialization should fail
    let result = engine.initialize();
    assert!(result.is_err(), "Double initialization should fail");

    bus.shutdown().expect("Failed to shutdown bus");
}
