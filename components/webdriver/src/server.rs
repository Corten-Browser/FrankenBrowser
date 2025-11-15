//! WebDriver HTTP server implementation (simplified)
//!
//! This module provides the WebDriver protocol server structure.
//! Full HTTP server implementation with axum will be added in future iterations.

use crate::errors::{Error, Result};
use crate::session::{Capabilities, SessionManager};
use serde::{Deserialize, Serialize};

/// WebDriver server state
#[derive(Clone)]
pub struct WebDriverState {
    pub session_manager: SessionManager,
}

impl WebDriverState {
    pub fn new() -> Self {
        Self {
            session_manager: SessionManager::new(),
        }
    }
}

impl Default for WebDriverState {
    fn default() -> Self {
        Self::new()
    }
}

/// Start WebDriver server (placeholder)
///
/// Full implementation will use axum or similar HTTP server framework.
/// For now, this demonstrates the intended structure.
pub async fn start_server(_port: u16) -> Result<()> {
    Err(Error::NotImplemented(
        "Full HTTP server not yet implemented. Use session management directly.".to_string(),
    ))
}

// ============================================================================
// Request/Response types per W3C WebDriver specification
// ============================================================================

/// Session creation request
#[derive(Deserialize, Serialize, Debug)]
pub struct CreateSessionRequest {
    pub capabilities: CapabilitiesRequest,
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CapabilitiesRequest {
    #[serde(default)]
    pub always_match: Option<Capabilities>,
    #[serde(default)]
    pub first_match: Option<Vec<Capabilities>>,
}

/// Session creation response
#[derive(Serialize, Debug)]
pub struct CreateSessionResponse {
    pub value: SessionValue,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SessionValue {
    pub session_id: String,
    pub capabilities: Capabilities,
}

/// Navigation request
#[derive(Deserialize, Debug)]
pub struct NavigateRequest {
    pub url: String,
}

/// Find element request
#[derive(Deserialize, Debug)]
pub struct FindElementRequest {
    pub using: String,
    pub value: String,
}

/// Execute script request
#[derive(Deserialize, Debug)]
pub struct ExecuteScriptRequest {
    pub script: String,
    pub args: Vec<serde_json::Value>,
}

/// Status response
#[derive(Serialize, Debug)]
pub struct StatusResponse {
    pub value: StatusValue,
}

#[derive(Serialize, Debug)]
pub struct StatusValue {
    pub ready: bool,
    pub message: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_webdriver_state_creation() {
        let state = WebDriverState::new();
        assert_eq!(state.session_manager.session_count(), 0);
    }

    #[test]
    fn test_create_session_request_serialization() {
        let req = CreateSessionRequest {
            capabilities: CapabilitiesRequest {
                always_match: Some(Capabilities::default()),
                first_match: None,
            },
        };

        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("capabilities"));
    }

    #[test]
    fn test_status_response_serialization() {
        let status = StatusResponse {
            value: StatusValue {
                ready: true,
                message: "FrankenBrowser WebDriver ready".to_string(),
            },
        };

        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("ready"));
        assert!(json.contains("FrankenBrowser"));
    }
}
