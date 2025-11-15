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
use base64::Engine;
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
    State(state): State<WebDriverState>,
    Path(session_id): Path<String>,
    Json(req): Json<FindElementRequest>,
) -> WebDriverResult<Json<FindElementResponse>> {
    // Verify session exists
    let _session = state
        .session_manager
        .get_session(&session_id)
        .map_err(WebDriverError::from)?;

    // Validate locator strategy
    validate_locator_strategy(&req.using)?;

    // Find element
    // TODO: Integrate with actual WebView DOM when browser instance is available
    // For now, return a placeholder element reference
    let element_id = find_element(&req.using, &req.value);

    Ok(Json(FindElementResponse {
        value: ElementReference {
            element: element_id,
        },
    }))
}

/// Validate locator strategy per W3C WebDriver spec
fn validate_locator_strategy(strategy: &str) -> std::result::Result<(), Error> {
    match strategy {
        "css selector" | "link text" | "partial link text" | "tag name" | "xpath" => Ok(()),
        _ => Err(Error::InvalidArgument(format!(
            "Invalid locator strategy: {}",
            strategy
        ))),
    }
}

/// Find element using the specified locator strategy
///
/// This is a placeholder implementation that returns a mock element ID.
/// In production, this would:
/// 1. Query the DOM using the locator strategy
/// 2. Return an actual element reference
/// 3. Store element state for subsequent operations
fn find_element(_strategy: &str, _value: &str) -> String {
    // Generate a unique element ID (UUID format per W3C WebDriver spec)
    // In production, this would map to an actual DOM element
    uuid::Uuid::new_v4().to_string()
}

/// POST /session/:session_id/execute/sync - Execute JavaScript
async fn execute_script_handler(
    State(state): State<WebDriverState>,
    Path(session_id): Path<String>,
    Json(req): Json<ExecuteScriptRequest>,
) -> WebDriverResult<Json<ExecuteScriptResponse>> {
    // Verify session exists
    let _session = state
        .session_manager
        .get_session(&session_id)
        .map_err(WebDriverError::from)?;

    // Execute script
    // TODO: Integrate with actual WebView when browser instance is available
    // For now, return null as a placeholder (valid JavaScript value)
    let result = execute_javascript(&req.script, &req.args);

    Ok(Json(ExecuteScriptResponse { value: result }))
}

/// Execute JavaScript code and return the result
///
/// This is a placeholder implementation that returns null.
/// In production, this would:
/// 1. Inject the script into the WebView
/// 2. Pass arguments as function parameters
/// 3. Return the actual JavaScript result
fn execute_javascript(_script: &str, _args: &[serde_json::Value]) -> serde_json::Value {
    // Placeholder: return null
    // Real implementation would:
    // - Format script with arguments
    // - Call webview.execute_script()
    // - Parse and return the result
    serde_json::Value::Null
}

/// GET /session/:session_id/screenshot - Take screenshot
async fn screenshot_handler(
    State(state): State<WebDriverState>,
    Path(session_id): Path<String>,
) -> WebDriverResult<Json<ScreenshotResponse>> {
    // Verify session exists
    let _session = state
        .session_manager
        .get_session(&session_id)
        .map_err(WebDriverError::from)?;

    // Generate screenshot
    // TODO: Integrate with actual WebView when browser instance is available
    // For now, return a minimal 1x1 transparent PNG as placeholder
    let png_bytes = create_placeholder_screenshot();

    // Base64 encode per W3C WebDriver spec
    let base64_png = base64::engine::general_purpose::STANDARD.encode(&png_bytes);

    Ok(Json(ScreenshotResponse {
        value: base64_png,
    }))
}

/// Create a minimal 1x1 transparent PNG for placeholder screenshots
///
/// This is a valid PNG file that can be decoded by any image viewer.
/// In production, this would be replaced with actual WebView screenshot data.
fn create_placeholder_screenshot() -> Vec<u8> {
    // Minimal 1x1 transparent PNG (67 bytes)
    vec![
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, // PNG signature
        0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52, // IHDR chunk
        0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, // 1x1 dimensions
        0x08, 0x06, 0x00, 0x00, 0x00, 0x1F, 0x15, 0xC4,
        0x89, 0x00, 0x00, 0x00, 0x0A, 0x49, 0x44, 0x41, // IDAT chunk
        0x54, 0x78, 0x9C, 0x63, 0x00, 0x01, 0x00, 0x00,
        0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00, // Image data
        0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, // IEND chunk
        0x42, 0x60, 0x82,
    ]
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

/// Screenshot response
#[derive(Serialize, Debug)]
pub struct ScreenshotResponse {
    pub value: String, // Base64-encoded PNG
}

/// Execute script response
#[derive(Serialize, Debug)]
pub struct ExecuteScriptResponse {
    pub value: serde_json::Value, // JavaScript result as JSON
}

/// Find element response
#[derive(Serialize, Debug)]
pub struct FindElementResponse {
    pub value: ElementReference,
}

/// Element reference per W3C WebDriver spec
#[derive(Serialize, Debug)]
pub struct ElementReference {
    #[serde(rename = "element-6066-11e4-a52e-4f735466cecf")]
    pub element: String, // Element ID (UUID)
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
