//! WebDriver HTTP server implementation
//!
//! This module provides the W3C WebDriver protocol HTTP server using axum.

use crate::errors::{Error, Result, WebDriverErrorResponse};
use crate::session::{Capabilities, SessionManager};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{delete, get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use tower_http::cors::{Any, CorsLayer};

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

/// Start WebDriver server on the specified port
pub async fn start_server(port: u16) -> Result<()> {
    let state = WebDriverState::new();

    let app = create_router(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|e| Error::ServerError(format!("Failed to bind to {}: {}", addr, e)))?;

    tracing::info!("WebDriver server listening on {}", addr);

    axum::serve(listener, app)
        .await
        .map_err(|e| Error::ServerError(format!("Server error: {}", e)))?;

    Ok(())
}

/// Create the WebDriver router with all endpoints
fn create_router(state: WebDriverState) -> Router {
    // CORS layer to allow WebDriver clients from any origin
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        // Status endpoint
        .route("/status", get(status_handler))
        // Session endpoints
        .route("/session", post(create_session_handler))
        .route("/session/:session_id", delete(delete_session_handler))
        // Navigation endpoints
        .route("/session/:session_id/url", post(navigate_handler))
        .route("/session/:session_id/url", get(get_url_handler))
        // Element endpoints
        .route(
            "/session/:session_id/element",
            post(find_element_handler),
        )
        // Script execution endpoints
        .route(
            "/session/:session_id/execute/sync",
            post(execute_script_handler),
        )
        // Screenshot endpoint
        .route(
            "/session/:session_id/screenshot",
            get(screenshot_handler),
        )
        // Window handle endpoint
        .route("/session/:session_id/window", get(window_handle_handler))
        .layer(cors)
        .with_state(state)
}

// ============================================================================
// Handler Functions
// ============================================================================

/// GET /status - Server status
async fn status_handler() -> Json<StatusResponse> {
    Json(StatusResponse {
        value: StatusValue {
            ready: true,
            message: "FrankenBrowser WebDriver ready".to_string(),
        },
    })
}

/// POST /session - Create new session
async fn create_session_handler(
    State(state): State<WebDriverState>,
    Json(req): Json<CreateSessionRequest>,
) -> WebDriverResult<Json<CreateSessionResponse>> {
    // Negotiate capabilities (simplified: use alwaysMatch or first of firstMatch)
    let capabilities = req
        .capabilities
        .always_match
        .or_else(|| req.capabilities.first_match.and_then(|v| v.into_iter().next()))
        .unwrap_or_default();

    let session = state
        .session_manager
        .create_session(capabilities.clone())
        .map_err(WebDriverError::from)?;

    Ok(Json(CreateSessionResponse {
        value: SessionValue {
            session_id: session.id.clone(),
            capabilities,
        },
    }))
}

/// DELETE /session/:session_id - Delete session
async fn delete_session_handler(
    State(state): State<WebDriverState>,
    Path(session_id): Path<String>,
) -> WebDriverResult<StatusCode> {
    state
        .session_manager
        .delete_session(&session_id)
        .map_err(WebDriverError::from)?;

    Ok(StatusCode::OK)
}

/// POST /session/:session_id/url - Navigate to URL
async fn navigate_handler(
    State(state): State<WebDriverState>,
    Path(session_id): Path<String>,
    Json(req): Json<NavigateRequest>,
) -> WebDriverResult<StatusCode> {
    let mut session = state
        .session_manager
        .get_session(&session_id)
        .map_err(WebDriverError::from)?;

    session.navigate(req.url).map_err(WebDriverError::from)?;

    state
        .session_manager
        .update_session(session)
        .map_err(WebDriverError::from)?;

    Ok(StatusCode::OK)
}

/// GET /session/:session_id/url - Get current URL
async fn get_url_handler(
    State(state): State<WebDriverState>,
    Path(session_id): Path<String>,
) -> WebDriverResult<Json<UrlResponse>> {
    let session = state
        .session_manager
        .get_session(&session_id)
        .map_err(WebDriverError::from)?;

    let url = session
        .get_url()
        .ok_or_else(|| Error::InvalidSession("No URL set".to_string()))
        .map_err(WebDriverError::from)?;

    Ok(Json(UrlResponse {
        value: url.to_string(),
    }))
}

/// POST /session/:session_id/element - Find element
async fn find_element_handler(
    State(_state): State<WebDriverState>,
    Path(_session_id): Path<String>,
    Json(_req): Json<FindElementRequest>,
) -> WebDriverResult<StatusCode> {
    // Placeholder: Element finding not fully implemented
    Err(WebDriverError::from(Error::NotImplemented(
        "Element finding not yet implemented".to_string(),
    )))
}

/// POST /session/:session_id/execute/sync - Execute JavaScript
async fn execute_script_handler(
    State(_state): State<WebDriverState>,
    Path(_session_id): Path<String>,
    Json(_req): Json<ExecuteScriptRequest>,
) -> WebDriverResult<StatusCode> {
    // Placeholder: Script execution not fully implemented
    Err(WebDriverError::from(Error::NotImplemented(
        "Script execution not yet implemented".to_string(),
    )))
}

/// GET /session/:session_id/screenshot - Take screenshot
async fn screenshot_handler(
    State(_state): State<WebDriverState>,
    Path(_session_id): Path<String>,
) -> WebDriverResult<StatusCode> {
    // Placeholder: Screenshot not fully implemented
    Err(WebDriverError::from(Error::NotImplemented(
        "Screenshot not yet implemented".to_string(),
    )))
}

/// GET /session/:session_id/window - Get window handle
async fn window_handle_handler(
    State(state): State<WebDriverState>,
    Path(session_id): Path<String>,
) -> WebDriverResult<Json<WindowHandleResponse>> {
    let session = state
        .session_manager
        .get_session(&session_id)
        .map_err(WebDriverError::from)?;

    Ok(Json(WindowHandleResponse {
        value: session.window_handle.clone(),
    }))
}

// ============================================================================
// Error handling
// ============================================================================

/// WebDriver error wrapper for proper HTTP responses
struct WebDriverError(Error);

impl From<Error> for WebDriverError {
    fn from(err: Error) -> Self {
        WebDriverError(err)
    }
}

impl IntoResponse for WebDriverError {
    fn into_response(self) -> Response {
        let error_response: WebDriverErrorResponse = self.0.into();
        let status = match error_response.value.error.as_str() {
            "invalid session id" => StatusCode::NOT_FOUND,
            "invalid argument" => StatusCode::BAD_REQUEST,
            "no such element" => StatusCode::NOT_FOUND,
            "no such window" => StatusCode::NOT_FOUND,
            "unsupported operation" => StatusCode::NOT_IMPLEMENTED,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };

        (status, Json(error_response)).into_response()
    }
}

/// Result type for WebDriver handlers
type WebDriverResult<T> = std::result::Result<T, WebDriverError>;

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

/// URL response
#[derive(Serialize, Debug)]
pub struct UrlResponse {
    pub value: String,
}

/// Window handle response
#[derive(Serialize, Debug)]
pub struct WindowHandleResponse {
    pub value: String,
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
