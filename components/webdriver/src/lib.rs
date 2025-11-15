//! WebDriver protocol implementation for FrankenBrowser
//!
//! This component implements the W3C WebDriver specification, allowing
//! remote control of the browser for automation and testing.
//!
//! # Component Overview
//!
//! **Type**: library
//! **Level**: 3 (integration)
//! **Token Budget**: 15000 tokens
//!
//! # Dependencies
//!
//! - shared_types: Common browser types
//! - message_bus: Inter-component communication
//! - browser_core: Browser engine
//! - webview_integration: WebView control
//! - axum: HTTP server framework
//! - serde: Serialization
//!
//! # Usage
//!
//! ```no_run
//! use webdriver::server::start_server;
//!
//! #[tokio::main]
//! async fn main() {
//!     // Start WebDriver server on port 4444
//!     start_server(4444).await.unwrap();
//! }
//! ```
//!
//! # WebDriver Endpoints
//!
//! - `GET /status` - Server status
//! - `POST /session` - Create new session
//! - `DELETE /session/{session_id}` - Delete session
//! - `POST /session/{session_id}/url` - Navigate to URL
//! - `GET /session/{session_id}/url` - Get current URL
//! - `POST /session/{session_id}/element` - Find element
//! - `POST /session/{session_id}/execute/sync` - Execute JavaScript
//! - `GET /session/{session_id}/screenshot` - Take screenshot
//! - `GET /session/{session_id}/window` - Get window handle
//!
//! # W3C WebDriver Specification
//!
//! This implementation follows the W3C WebDriver specification:
//! https://w3c.github.io/webdriver/

pub mod dom_interface;
pub mod element;
pub mod errors;
pub mod script_args;
pub mod server;
pub mod session;

// Re-export main types
pub use dom_interface::{DomInterface, LocatorStrategy};
pub use element::{CachedElement, ElementCache, ElementReference};
pub use errors::{Error, Result};
pub use server::{start_server, WebDriverState};
pub use session::{Capabilities, Session, SessionManager};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_exports() {
        // Verify main types are exported
        let _manager = SessionManager::new();
        let _caps = Capabilities::default();
    }

    #[test]
    fn test_webdriver_state_creation() {
        let state = WebDriverState::new();
        assert_eq!(state.session_manager.session_count(), 0);
    }
}
