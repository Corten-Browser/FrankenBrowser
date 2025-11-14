//! Common test utilities for integration tests
//!
//! This module provides shared testing infrastructure for cross-component
//! integration tests.

use message_bus::MessageBus;
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Test helper to create a message bus with handlers
pub fn setup_message_bus() -> MessageBus {
    let mut bus = MessageBus::new();
    bus.start().expect("Failed to start message bus");
    bus
}

/// Test helper to wait for async operations
pub fn wait_for_processing() {
    std::thread::sleep(Duration::from_millis(100));
}

/// Test helper to create a network config for testing
pub fn test_network_config() -> config_manager::NetworkConfig {
    config_manager::NetworkConfig {
        max_connections_per_host: 6,
        timeout_seconds: 5, // Shorter timeout for tests
        enable_cookies: true,
        enable_cache: true,
        cache_size_mb: 10, // Small cache for testing
    }
}

/// Test helper to create an adblock config for testing
pub fn test_adblock_config() -> config_manager::AdBlockConfig {
    config_manager::AdBlockConfig {
        enabled: true,
        update_filters_on_startup: false,
        custom_filters: vec![
            // Test filter rules
            "||ads.example.com^".to_string(),
            "||doubleclick.net^".to_string(),
        ],
    }
}

/// Message collector for testing message routing
#[derive(Clone)]
pub struct MessageCollector {
    pub messages: Arc<Mutex<Vec<shared_types::BrowserMessage>>>,
}

impl MessageCollector {
    pub fn new() -> Self {
        Self {
            messages: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn collect(&self, message: shared_types::BrowserMessage) {
        self.messages.lock().unwrap().push(message);
    }

    pub fn get_messages(&self) -> Vec<shared_types::BrowserMessage> {
        self.messages.lock().unwrap().clone()
    }

    pub fn count(&self) -> usize {
        self.messages.lock().unwrap().len()
    }

    pub fn clear(&self) {
        self.messages.lock().unwrap().clear();
    }
}

impl message_bus::MessageHandler for MessageCollector {
    fn handle(&self, message: shared_types::BrowserMessage) -> message_bus::Result<()> {
        self.collect(message);
        Ok(())
    }
}
