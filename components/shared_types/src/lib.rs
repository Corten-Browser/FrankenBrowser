//! Common types, messages, and error definitions
//!
//! This is the shared_types component of the FrankenBrowser project.
//!
//! # Component Overview
//!
//! **Type**: library
//! **Level**: 0
//! **Token Budget**: 5000 tokens
//!
//! # Dependencies
//!
//! None (base component)
//!
//! # Usage
//!
//! ```rust
//! use shared_types::{BrowserMessage, ResourceType, TabId, RequestId};
//! use url::Url;
//!
//! // Create a navigate request
//! let msg = BrowserMessage::NavigateRequest {
//!     tab_id: 1,
//!     url: Url::parse("https://example.com").unwrap(),
//! };
//!
//! // Check resource type
//! let resource = ResourceType::Script;
//! ```

pub mod errors;
pub mod types;

// Re-export main types for convenience
pub use errors::{BrowserError, Result};
pub use types::{BrowserMessage, RequestId, ResourceType, TabId};

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use url::Url;

    // ========================================
    // RED PHASE: Tests for ResourceType
    // ========================================

    #[test]
    fn test_resource_type_variants_exist() {
        // Test that all ResourceType variants exist
        let _document = ResourceType::Document;
        let _script = ResourceType::Script;
        let _image = ResourceType::Image;
        let _stylesheet = ResourceType::Stylesheet;
        let _font = ResourceType::Font;
        let _media = ResourceType::Media;
        let _websocket = ResourceType::Websocket;
        let _xhr = ResourceType::Xhr;
        let _other = ResourceType::Other;
    }

    #[test]
    fn test_resource_type_serialization() {
        // Test that ResourceType can be serialized
        let resource = ResourceType::Script;
        let json = serde_json::to_string(&resource).unwrap();
        assert!(json.contains("Script"));
    }

    #[test]
    fn test_resource_type_deserialization() {
        // Test that ResourceType can be deserialized
        let json = r#""Script""#;
        let resource: ResourceType = serde_json::from_str(json).unwrap();
        assert!(matches!(resource, ResourceType::Script));
    }

    #[test]
    fn test_resource_type_debug() {
        // Test that ResourceType implements Debug
        let resource = ResourceType::Image;
        let debug_str = format!("{:?}", resource);
        assert!(debug_str.contains("Image"));
    }

    #[test]
    fn test_resource_type_clone() {
        // Test that ResourceType implements Clone
        let resource = ResourceType::Font;
        let cloned = resource.clone();
        assert!(matches!(cloned, ResourceType::Font));
    }

    #[test]
    fn test_resource_type_send_sync() {
        // Test that ResourceType implements Send + Sync
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<ResourceType>();
    }

    // ========================================
    // RED PHASE: Tests for BrowserMessage
    // ========================================

    #[test]
    fn test_browser_message_navigate_request() {
        let url = Url::parse("https://example.com").unwrap();
        let msg = BrowserMessage::NavigateRequest {
            tab_id: 1,
            url: url.clone(),
        };

        match msg {
            BrowserMessage::NavigateRequest {
                tab_id,
                url: msg_url,
            } => {
                assert_eq!(tab_id, 1);
                assert_eq!(msg_url, url);
            }
            _ => panic!("Expected NavigateRequest"),
        }
    }

    #[test]
    fn test_browser_message_navigate_response() {
        let content = vec![1, 2, 3, 4];
        let msg = BrowserMessage::NavigateResponse {
            tab_id: 1,
            content: content.clone(),
        };

        match msg {
            BrowserMessage::NavigateResponse {
                tab_id,
                content: msg_content,
            } => {
                assert_eq!(tab_id, 1);
                assert_eq!(msg_content, content);
            }
            _ => panic!("Expected NavigateResponse"),
        }
    }

    #[test]
    fn test_browser_message_navigate_error() {
        let msg = BrowserMessage::NavigateError {
            tab_id: 1,
            error: "Failed to load".to_string(),
        };

        match msg {
            BrowserMessage::NavigateError { tab_id, error } => {
                assert_eq!(tab_id, 1);
                assert_eq!(error, "Failed to load");
            }
            _ => panic!("Expected NavigateError"),
        }
    }

    #[test]
    fn test_browser_message_http_request() {
        let url = Url::parse("https://api.example.com").unwrap();
        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "application/json".to_string());

        let msg = BrowserMessage::HttpRequest {
            request_id: 123,
            url: url.clone(),
            headers: headers.clone(),
        };

        match msg {
            BrowserMessage::HttpRequest {
                request_id,
                url: msg_url,
                headers: msg_headers,
            } => {
                assert_eq!(request_id, 123);
                assert_eq!(msg_url, url);
                assert_eq!(msg_headers, headers);
            }
            _ => panic!("Expected HttpRequest"),
        }
    }

    #[test]
    fn test_browser_message_http_response() {
        let body = vec![5, 6, 7, 8];
        let msg = BrowserMessage::HttpResponse {
            request_id: 123,
            status: 200,
            body: body.clone(),
        };

        match msg {
            BrowserMessage::HttpResponse {
                request_id,
                status,
                body: msg_body,
            } => {
                assert_eq!(request_id, 123);
                assert_eq!(status, 200);
                assert_eq!(msg_body, body);
            }
            _ => panic!("Expected HttpResponse"),
        }
    }

    #[test]
    fn test_browser_message_should_block() {
        let url = Url::parse("https://ads.example.com").unwrap();
        let msg = BrowserMessage::ShouldBlock {
            url: url.clone(),
            resource_type: ResourceType::Script,
        };

        match msg {
            BrowserMessage::ShouldBlock {
                url: msg_url,
                resource_type,
            } => {
                assert_eq!(msg_url, url);
                assert!(matches!(resource_type, ResourceType::Script));
            }
            _ => panic!("Expected ShouldBlock"),
        }
    }

    #[test]
    fn test_browser_message_block_decision() {
        let msg = BrowserMessage::BlockDecision {
            block: true,
            reason: Some("Ad blocked".to_string()),
        };

        match msg {
            BrowserMessage::BlockDecision { block, reason } => {
                assert!(block);
                assert_eq!(reason, Some("Ad blocked".to_string()));
            }
            _ => panic!("Expected BlockDecision"),
        }
    }

    #[test]
    fn test_browser_message_block_decision_no_reason() {
        let msg = BrowserMessage::BlockDecision {
            block: false,
            reason: None,
        };

        match msg {
            BrowserMessage::BlockDecision { block, reason } => {
                assert!(!block);
                assert_eq!(reason, None);
            }
            _ => panic!("Expected BlockDecision"),
        }
    }

    #[test]
    fn test_browser_message_create_tab() {
        let msg = BrowserMessage::CreateTab { parent_window: 1 };

        match msg {
            BrowserMessage::CreateTab { parent_window } => {
                assert_eq!(parent_window, 1);
            }
            _ => panic!("Expected CreateTab"),
        }
    }

    #[test]
    fn test_browser_message_close_tab() {
        let msg = BrowserMessage::CloseTab { tab_id: 5 };

        match msg {
            BrowserMessage::CloseTab { tab_id } => {
                assert_eq!(tab_id, 5);
            }
            _ => panic!("Expected CloseTab"),
        }
    }

    #[test]
    fn test_browser_message_switch_tab() {
        let msg = BrowserMessage::SwitchTab { tab_id: 3 };

        match msg {
            BrowserMessage::SwitchTab { tab_id } => {
                assert_eq!(tab_id, 3);
            }
            _ => panic!("Expected SwitchTab"),
        }
    }

    #[test]
    fn test_browser_message_shutdown() {
        let msg = BrowserMessage::Shutdown;
        assert!(matches!(msg, BrowserMessage::Shutdown));
    }

    #[test]
    fn test_browser_message_reload() {
        let msg = BrowserMessage::Reload { tab_id: 2 };

        match msg {
            BrowserMessage::Reload { tab_id } => {
                assert_eq!(tab_id, 2);
            }
            _ => panic!("Expected Reload"),
        }
    }

    #[test]
    fn test_browser_message_go_back() {
        let msg = BrowserMessage::GoBack { tab_id: 4 };

        match msg {
            BrowserMessage::GoBack { tab_id } => {
                assert_eq!(tab_id, 4);
            }
            _ => panic!("Expected GoBack"),
        }
    }

    #[test]
    fn test_browser_message_go_forward() {
        let msg = BrowserMessage::GoForward { tab_id: 6 };

        match msg {
            BrowserMessage::GoForward { tab_id } => {
                assert_eq!(tab_id, 6);
            }
            _ => panic!("Expected GoForward"),
        }
    }

    #[test]
    fn test_browser_message_serialization() {
        let url = Url::parse("https://example.com").unwrap();
        let msg = BrowserMessage::NavigateRequest { tab_id: 1, url };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("NavigateRequest"));
    }

    #[test]
    fn test_browser_message_deserialization() {
        let json = r#"{"NavigateRequest":{"tab_id":1,"url":"https://example.com/"}}"#;
        let msg: BrowserMessage = serde_json::from_str(json).unwrap();

        match msg {
            BrowserMessage::NavigateRequest { tab_id, .. } => {
                assert_eq!(tab_id, 1);
            }
            _ => panic!("Expected NavigateRequest"),
        }
    }

    #[test]
    fn test_browser_message_debug() {
        let msg = BrowserMessage::Shutdown;
        let debug_str = format!("{:?}", msg);
        assert!(debug_str.contains("Shutdown"));
    }

    #[test]
    fn test_browser_message_clone() {
        let msg = BrowserMessage::CloseTab { tab_id: 1 };
        let cloned = msg.clone();

        match cloned {
            BrowserMessage::CloseTab { tab_id } => {
                assert_eq!(tab_id, 1);
            }
            _ => panic!("Expected CloseTab"),
        }
    }

    #[test]
    fn test_browser_message_send_sync() {
        // Test that BrowserMessage implements Send + Sync
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<BrowserMessage>();
    }

    // ========================================
    // RED PHASE: Tests for Type Aliases
    // ========================================

    #[test]
    fn test_tab_id_type() {
        let tab_id: TabId = 42;
        assert_eq!(tab_id, 42);
    }

    #[test]
    fn test_request_id_type() {
        let request_id: RequestId = 12345;
        assert_eq!(request_id, 12345);
    }

    // ========================================
    // RED PHASE: Tests for Error Types
    // ========================================

    #[test]
    fn test_browser_error_display() {
        let error = BrowserError::Network("Connection failed".to_string());
        let display = format!("{}", error);
        assert!(display.contains("Connection failed"));
    }

    #[test]
    fn test_browser_error_debug() {
        let error = BrowserError::Network("Connection failed".to_string());
        let debug = format!("{:?}", error);
        assert!(debug.contains("Network"));
    }

    #[test]
    fn test_browser_error_navigation() {
        let error = BrowserError::Navigation("Invalid URL".to_string());
        assert!(matches!(error, BrowserError::Navigation(_)));
    }

    #[test]
    fn test_browser_error_tab_not_found() {
        let error = BrowserError::TabNotFound(5);
        match error {
            BrowserError::TabNotFound(id) => assert_eq!(id, 5),
            _ => panic!("Expected TabNotFound"),
        }
    }

    #[test]
    fn test_browser_error_send_sync() {
        // Test that BrowserError implements Send + Sync
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<BrowserError>();
    }

    #[test]
    fn test_result_type_ok() {
        let result: Result<i32> = Ok(42);
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_result_type_err() {
        let result: Result<i32> = Err(BrowserError::Network("Failed".to_string()));
        assert!(result.is_err());
    }

    #[test]
    fn test_browser_error_from_anyhow() {
        let anyhow_error = anyhow::anyhow!("Something went wrong");
        let browser_error = BrowserError::from(anyhow_error);
        assert!(matches!(browser_error, BrowserError::Other(_)));
    }
}
