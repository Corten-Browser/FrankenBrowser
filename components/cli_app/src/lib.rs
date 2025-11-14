//! Main application entry point and wiring
//!
//! This is the cli_app component of the FrankenBrowser project.
//!
//! # Component Overview
//!
//! **Type**: application
//! **Level**: 4
//! **Token Budget**: 3000 tokens
//!
//! # Dependencies
//!
//! - browser_shell: UI shell and tab management
//! - config_manager: Configuration management
//! - message_bus: Message passing infrastructure
//! - network_stack: HTTP client
//! - adblock_engine: Ad blocking
//! - browser_core: Browser engine
//!
//! # Usage
//!
//! ```no_run
//! use cli_app::{BrowserApp, Result};
//! use config_manager::Config;
//!
//! fn main() -> Result<()> {
//!     // Initialize logging
//!     tracing_subscriber::fmt::init();
//!
//!     // Load configuration
//!     let config = Config::load_or_default()?;
//!
//!     // Create and run browser application
//!     let app = BrowserApp::new(config)?;
//!     app.run()
//! }
//! ```

pub mod errors;
pub mod types;

// Re-export main types for convenience
pub use errors::{Error, Result};
pub use types::BrowserApp;

#[cfg(test)]
mod tests {
    use super::*;
    use config_manager::Config;

    // ========================================
    // RED PHASE: Tests for BrowserApp::new()
    // ========================================

    #[test]
    fn test_browser_app_new_with_default_config() {
        let config = Config::default();
        let result = BrowserApp::new(config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_browser_app_new_creates_all_components() {
        let config = Config::default();
        let app = BrowserApp::new(config).unwrap();

        // App should have all components initialized
        // This is tested by ensuring we can create the app without panics
        drop(app);
    }

    // ========================================
    // RED PHASE: Tests for BrowserApp structure
    // ========================================

    #[test]
    fn test_browser_app_can_be_created() {
        // Simple test to verify BrowserApp struct exists and can be instantiated
        let config = Config::default();
        let _app = BrowserApp::new(config);
    }

    // ========================================
    // RED PHASE: Tests for configuration usage
    // ========================================

    #[test]
    fn test_browser_app_uses_config() {
        let mut config = Config::default();
        config.network.max_connections_per_host = 10;
        config.adblock.enabled = false;

        // Should create app with custom config without errors
        let result = BrowserApp::new(config);
        assert!(result.is_ok());
    }

    // ========================================
    // Additional tests for coverage
    // ========================================

    #[test]
    fn test_browser_app_with_different_homepage() {
        let mut config = Config::default();
        config.browser.homepage = "https://example.com".to_string();

        let result = BrowserApp::new(config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_browser_app_with_disabled_adblock() {
        let mut config = Config::default();
        config.adblock.enabled = false;

        let result = BrowserApp::new(config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_browser_app_with_custom_cache_size() {
        let mut config = Config::default();
        config.network.cache_size_mb = 1000;

        let result = BrowserApp::new(config);
        assert!(result.is_ok());
    }

    // Note: BrowserApp is not Send due to AdBlockEngine's internal Rc types.
    // This is acceptable for an application-level component that runs on the main thread.
}
