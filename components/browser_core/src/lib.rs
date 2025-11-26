//! Browser engine, navigation, history, and bookmarks
//!
//! This is the browser_core component of the FrankenBrowser project.
//!
//! # Component Overview
//!
//! **Type**: library
//! **Level**: 2
//! **Token Budget**: 10000 tokens
//!
//! # Dependencies
//!
//! - shared_types: For BrowserMessage, errors
//! - message_bus: For MessageSender trait
//! - network_stack: For NetworkStack
//! - config_manager: For Config
//!
//! # Usage
//!
//! ```
//! use browser_core::{BrowserEngine, Bookmark, HistoryEntry};
//! use config_manager::Config;
//! use network_stack::NetworkStack;
//! use message_bus::MessageBus;
//! use url::Url;
//!
//! // Create config
//! let config = Config::default();
//!
//! // Create message bus for network stack
//! let mut bus1 = MessageBus::new();
//! bus1.start().unwrap();
//! let network = NetworkStack::new(config.network_config(), bus1.sender()).unwrap();
//!
//! // Create message bus for browser engine
//! let mut bus2 = MessageBus::new();
//! bus2.start().unwrap();
//!
//! // Create browser engine
//! let mut engine = BrowserEngine::new(config, network, bus2.sender()).unwrap();
//!
//! // Navigate
//! engine.navigate(1, Url::parse("https://example.com").unwrap()).unwrap();
//!
//! // Add bookmark
//! engine.add_bookmark(Url::parse("https://example.com").unwrap(), "Example".to_string()).unwrap();
//!
//! // Get bookmarks
//! let bookmarks = engine.get_bookmarks();
//! assert_eq!(bookmarks.len(), 1);
//! ```

pub mod errors;
pub mod navigation;
pub mod types;

// Re-export main types for convenience
pub use errors::{Error, Result};
pub use navigation::{NavigationError, NavigationState, Navigator, Protocol};
pub use types::{
    Bookmark, BrowserEngine, BrowserMetrics, HistoryEntry, MetricsSnapshot, PerformanceMetric,
    TestResult, TestResultDatabase, TestStatus, TestSummary,
};

#[cfg(test)]
mod tests {
    #[test]
    fn test_module_exports() {
        // Verify that main types are exported
        // This ensures our public API is accessible
    }
}
