//! Type definitions for FrankenBrowser
//!
//! This module contains core types used for communication between
//! FrankenBrowser components, including message types and resource types.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use url::Url;

/// Type alias for tab identifiers
pub type TabId = u32;

/// Type alias for request identifiers
pub type RequestId = u64;

/// Enum representing different types of web resources
///
/// This is used to categorize resources for blocking and filtering decisions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResourceType {
    /// HTML document
    Document,
    /// JavaScript file
    Script,
    /// Image file
    Image,
    /// CSS stylesheet
    Stylesheet,
    /// Font file
    Font,
    /// Audio or video media
    Media,
    /// WebSocket connection
    Websocket,
    /// XMLHttpRequest
    Xhr,
    /// Other resource type
    Other,
}

/// Enum representing all possible messages that can be sent between browser components
///
/// This enum serves as the communication protocol for the modular browser architecture.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BrowserMessage {
    /// Request to navigate a tab to a URL
    NavigateRequest {
        /// The tab to navigate
        tab_id: u32,
        /// The URL to navigate to
        url: Url,
    },

    /// Response with the content of a navigation
    NavigateResponse {
        /// The tab that was navigated
        tab_id: u32,
        /// The HTML content received
        content: Vec<u8>,
    },

    /// Error that occurred during navigation
    NavigateError {
        /// The tab where the error occurred
        tab_id: u32,
        /// Error message
        error: String,
    },

    /// HTTP request message
    HttpRequest {
        /// Unique identifier for this request
        request_id: u64,
        /// URL to request
        url: Url,
        /// HTTP headers
        headers: HashMap<String, String>,
    },

    /// HTTP response message
    HttpResponse {
        /// Request ID this responds to
        request_id: u64,
        /// HTTP status code
        status: u16,
        /// Response body
        body: Vec<u8>,
    },

    /// Ask if a resource should be blocked
    ShouldBlock {
        /// URL of the resource
        url: Url,
        /// Type of resource
        resource_type: ResourceType,
    },

    /// Decision on whether to block a resource
    BlockDecision {
        /// Whether to block the resource
        block: bool,
        /// Optional reason for the decision
        reason: Option<String>,
    },

    /// Request to create a new tab
    CreateTab {
        /// Parent window ID
        parent_window: u32,
    },

    /// Request to close a tab
    CloseTab {
        /// Tab ID to close
        tab_id: u32,
    },

    /// Request to switch to a different tab
    SwitchTab {
        /// Tab ID to switch to
        tab_id: u32,
    },

    /// Request to shut down the browser
    Shutdown,

    /// Request to reload a tab
    Reload {
        /// Tab ID to reload
        tab_id: u32,
    },

    /// Request to go back in history
    GoBack {
        /// Tab ID to navigate back
        tab_id: u32,
    },

    /// Request to go forward in history
    GoForward {
        /// Tab ID to navigate forward
        tab_id: u32,
    },
}

// Ensure Send + Sync for thread safety
static_assertions::assert_impl_all!(ResourceType: Send, Sync);
static_assertions::assert_impl_all!(BrowserMessage: Send, Sync);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resource_type_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<ResourceType>();
    }

    #[test]
    fn test_browser_message_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<BrowserMessage>();
    }

    #[test]
    fn test_tab_id_is_u32() {
        let id: TabId = 42;
        assert_eq!(id, 42u32);
    }

    #[test]
    fn test_request_id_is_u64() {
        let id: RequestId = 12345;
        assert_eq!(id, 12345u64);
    }
}
