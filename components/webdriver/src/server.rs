//! WebDriver HTTP server implementation
//!
//! This module provides the W3C WebDriver protocol HTTP server using axum.

use crate::dom_interface::DomInterface;
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
        // Element finding endpoints
        .route(
            "/session/:session_id/element",
            post(find_element_handler),
        )
        .route(
            "/session/:session_id/elements",
            post(find_elements_handler),
        )
        // Element interaction endpoints
        .route(
            "/session/:session_id/element/:element_id/click",
            post(click_element_handler),
        )
        .route(
            "/session/:session_id/element/:element_id/value",
            post(send_keys_handler),
        )
        .route(
            "/session/:session_id/element/:element_id/clear",
            post(clear_element_handler),
        )
        // Element query endpoints
        .route(
            "/session/:session_id/element/:element_id/text",
            get(get_element_text_handler),
        )
        .route(
            "/session/:session_id/element/:element_id/attribute/:name",
            get(get_element_attribute_handler),
        )
        .route(
            "/session/:session_id/element/:element_id/displayed",
            get(is_element_displayed_handler),
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
        // Window management endpoints
        .route("/session/:session_id/window", get(get_window_handle_handler))
        .route("/session/:session_id/window", post(switch_to_window_handler))
        .route("/session/:session_id/window", delete(close_window_handler))
        .route("/session/:session_id/window/handles", get(get_all_window_handles_handler))
        .route("/session/:session_id/window/new", post(new_window_handler))
        .route("/session/:session_id/window/rect", get(get_window_rect_handler))
        .route("/session/:session_id/window/rect", post(set_window_rect_handler))
        .route("/session/:session_id/window/maximize", post(maximize_window_handler))
        .route("/session/:session_id/window/minimize", post(minimize_window_handler))
        .route("/session/:session_id/window/fullscreen", post(fullscreen_window_handler))
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

    // Create session (returns session_id)
    let session_id = state
        .session_manager
        .create_session(capabilities.clone())
        .map_err(WebDriverError::from)?;

    Ok(Json(CreateSessionResponse {
        value: SessionValue {
            session_id,
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
    let session_arc = state
        .session_manager
        .get_session(&session_id)
        .map_err(WebDriverError::from)?;

    let mut session = session_arc.lock().unwrap();
    session.navigate(req.url).map_err(WebDriverError::from)?;

    Ok(StatusCode::OK)
}

/// GET /session/:session_id/url - Get current URL
async fn get_url_handler(
    State(state): State<WebDriverState>,
    Path(session_id): Path<String>,
) -> WebDriverResult<Json<UrlResponse>> {
    let session_arc = state
        .session_manager
        .get_session(&session_id)
        .map_err(WebDriverError::from)?;

    let session = session_arc.lock().unwrap();
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
    let session_arc = state
        .session_manager
        .get_session(&session_id)
        .map_err(WebDriverError::from)?;

    let session = session_arc.lock().unwrap();

    // Use DomInterface to find element
    let dom = DomInterface::new();
    let element_ref = dom
        .find_element(&session, &req.using, &req.value)
        .map_err(WebDriverError::from)?;

    // Cache element in session
    session.element_cache.cache_element(
        &session.id,
        &element_ref.selector,
        element_ref.index,
    );

    Ok(Json(FindElementResponse {
        value: ElementReference {
            element: element_ref.element_id,
        },
    }))
}

/// POST /session/:session_id/elements - Find multiple elements
async fn find_elements_handler(
    State(state): State<WebDriverState>,
    Path(session_id): Path<String>,
    Json(req): Json<FindElementRequest>,
) -> WebDriverResult<Json<FindElementsResponse>> {
    let session_arc = state
        .session_manager
        .get_session(&session_id)
        .map_err(WebDriverError::from)?;

    let session = session_arc.lock().unwrap();

    // Use DomInterface to find elements
    let dom = DomInterface::new();
    let elements = dom
        .find_elements(&session, &req.using, &req.value)
        .map_err(WebDriverError::from)?;

    // Cache all found elements
    let element_refs: Vec<ElementReference> = elements
        .iter()
        .map(|elem_ref| {
            session.element_cache.cache_element(
                &session.id,
                &elem_ref.selector,
                elem_ref.index,
            );
            ElementReference {
                element: elem_ref.element_id.clone(),
            }
        })
        .collect();

    Ok(Json(FindElementsResponse {
        value: element_refs,
    }))
}

/// POST /session/:session_id/element/:element_id/click - Click element
async fn click_element_handler(
    State(state): State<WebDriverState>,
    Path((session_id, element_id)): Path<(String, String)>,
) -> WebDriverResult<StatusCode> {
    let session_arc = state
        .session_manager
        .get_session(&session_id)
        .map_err(WebDriverError::from)?;

    let session = session_arc.lock().unwrap();

    // Get element from cache
    let cached_element = session
        .element_cache
        .get_element(&element_id)
        .map_err(WebDriverError::from)?;

    // Generate click script
    let dom = DomInterface::new();
    let script = dom.generate_click_script(
        &cached_element.reference.selector,
        cached_element.reference.index,
    );

    // Execute script
    let result = session
        .execute_script(&script)
        .map_err(WebDriverError::from)?;

    // Check if click succeeded
    if result.trim() == "true" {
        Ok(StatusCode::OK)
    } else {
        Err(WebDriverError::from(Error::NoSuchElement(format!(
            "Element not found: {}",
            element_id
        ))))
    }
}

/// POST /session/:session_id/element/:element_id/value - Send keys to element
async fn send_keys_handler(
    State(state): State<WebDriverState>,
    Path((session_id, element_id)): Path<(String, String)>,
    Json(req): Json<SendKeysRequest>,
) -> WebDriverResult<StatusCode> {
    let session_arc = state
        .session_manager
        .get_session(&session_id)
        .map_err(WebDriverError::from)?;

    let session = session_arc.lock().unwrap();

    // Get element from cache
    let cached_element = session
        .element_cache
        .get_element(&element_id)
        .map_err(WebDriverError::from)?;

    // Generate send keys script
    let dom = DomInterface::new();
    let script = dom.generate_send_keys_script(
        &cached_element.reference.selector,
        cached_element.reference.index,
        &req.text,
    );

    // Execute script
    session
        .execute_script(&script)
        .map_err(WebDriverError::from)?;

    Ok(StatusCode::OK)
}

/// POST /session/:session_id/element/:element_id/clear - Clear element
async fn clear_element_handler(
    State(state): State<WebDriverState>,
    Path((session_id, element_id)): Path<(String, String)>,
) -> WebDriverResult<StatusCode> {
    let session_arc = state
        .session_manager
        .get_session(&session_id)
        .map_err(WebDriverError::from)?;

    let session = session_arc.lock().unwrap();

    // Get element from cache
    let cached_element = session
        .element_cache
        .get_element(&element_id)
        .map_err(WebDriverError::from)?;

    // Generate clear script
    let dom = DomInterface::new();
    let script = dom.generate_clear_script(
        &cached_element.reference.selector,
        cached_element.reference.index,
    );

    // Execute script
    session
        .execute_script(&script)
        .map_err(WebDriverError::from)?;

    Ok(StatusCode::OK)
}

/// GET /session/:session_id/element/:element_id/text - Get element text
async fn get_element_text_handler(
    State(state): State<WebDriverState>,
    Path((session_id, element_id)): Path<(String, String)>,
) -> WebDriverResult<Json<TextResponse>> {
    let session_arc = state
        .session_manager
        .get_session(&session_id)
        .map_err(WebDriverError::from)?;

    let session = session_arc.lock().unwrap();

    // Get element from cache
    let cached_element = session
        .element_cache
        .get_element(&element_id)
        .map_err(WebDriverError::from)?;

    // Generate get text script
    let dom = DomInterface::new();
    let script = dom.generate_get_text_script(
        &cached_element.reference.selector,
        cached_element.reference.index,
    );

    // Execute script
    let text = session
        .execute_script(&script)
        .map_err(WebDriverError::from)?;

    Ok(Json(TextResponse {
        value: text.trim_matches('"').to_string(),
    }))
}

/// GET /session/:session_id/element/:element_id/attribute/:name - Get element attribute
async fn get_element_attribute_handler(
    State(state): State<WebDriverState>,
    Path((session_id, element_id, attribute_name)): Path<(String, String, String)>,
) -> WebDriverResult<Json<AttributeResponse>> {
    let session_arc = state
        .session_manager
        .get_session(&session_id)
        .map_err(WebDriverError::from)?;

    let session = session_arc.lock().unwrap();

    // Get element from cache
    let cached_element = session
        .element_cache
        .get_element(&element_id)
        .map_err(WebDriverError::from)?;

    // Generate get attribute script
    let dom = DomInterface::new();
    let script = dom.generate_get_attribute_script(
        &cached_element.reference.selector,
        cached_element.reference.index,
        &attribute_name,
    );

    // Execute script
    let value = session
        .execute_script(&script)
        .map_err(WebDriverError::from)?;

    // Parse result (null or string value)
    let attribute_value = if value.trim() == "null" {
        None
    } else {
        Some(value.trim_matches('"').to_string())
    };

    Ok(Json(AttributeResponse {
        value: attribute_value,
    }))
}

/// GET /session/:session_id/element/:element_id/displayed - Check if element is displayed
async fn is_element_displayed_handler(
    State(state): State<WebDriverState>,
    Path((session_id, element_id)): Path<(String, String)>,
) -> WebDriverResult<Json<BooleanResponse>> {
    let session_arc = state
        .session_manager
        .get_session(&session_id)
        .map_err(WebDriverError::from)?;

    let session = session_arc.lock().unwrap();

    // Get element from cache
    let cached_element = session
        .element_cache
        .get_element(&element_id)
        .map_err(WebDriverError::from)?;

    // Generate is displayed script
    let dom = DomInterface::new();
    let script = dom.generate_is_displayed_script(
        &cached_element.reference.selector,
        cached_element.reference.index,
    );

    // Execute script
    let result = session
        .execute_script(&script)
        .map_err(WebDriverError::from)?;

    let is_displayed = result.trim() == "true";

    Ok(Json(BooleanResponse {
        value: is_displayed,
    }))
}

/// POST /session/:session_id/execute/sync - Execute JavaScript
async fn execute_script_handler(
    State(state): State<WebDriverState>,
    Path(session_id): Path<String>,
    Json(req): Json<ExecuteScriptRequest>,
) -> WebDriverResult<Json<ExecuteScriptResponse>> {
    let session_arc = state
        .session_manager
        .get_session(&session_id)
        .map_err(WebDriverError::from)?;

    let session = session_arc.lock().unwrap();

    // Execute script using session's webview
    let result_str = session.execute_script(&req.script)
        .map_err(WebDriverError::from)?;

    // Parse result as JSON value (or return as string if parsing fails)
    let result = serde_json::from_str(&result_str)
        .unwrap_or(serde_json::Value::String(result_str));

    Ok(Json(ExecuteScriptResponse { value: result }))
}

/// GET /session/:session_id/screenshot - Take screenshot
async fn screenshot_handler(
    State(state): State<WebDriverState>,
    Path(session_id): Path<String>,
) -> WebDriverResult<Json<ScreenshotResponse>> {
    let session_arc = state
        .session_manager
        .get_session(&session_id)
        .map_err(WebDriverError::from)?;

    let session = session_arc.lock().unwrap();

    // Take screenshot using session's webview
    let png_bytes = session.screenshot()
        .map_err(WebDriverError::from)?;

    // Base64 encode per W3C WebDriver spec
    let base64_png = base64::engine::general_purpose::STANDARD.encode(&png_bytes);

    Ok(Json(ScreenshotResponse {
        value: base64_png,
    }))
}

/// GET /session/:session_id/window - Get current window handle
async fn get_window_handle_handler(
    State(state): State<WebDriverState>,
    Path(session_id): Path<String>,
) -> WebDriverResult<Json<WindowHandleResponse>> {
    let session_arc = state
        .session_manager
        .get_session(&session_id)
        .map_err(WebDriverError::from)?;

    let session = session_arc.lock().unwrap();

    let current_handle = session
        .get_current_window()
        .ok_or_else(|| Error::InvalidSession("No active window".to_string()))
        .map_err(WebDriverError::from)?;

    Ok(Json(WindowHandleResponse {
        value: current_handle,
    }))
}

/// GET /session/:session_id/window/handles - Get all window handles
async fn get_all_window_handles_handler(
    State(state): State<WebDriverState>,
    Path(session_id): Path<String>,
) -> WebDriverResult<Json<WindowHandlesResponse>> {
    let session_arc = state
        .session_manager
        .get_session(&session_id)
        .map_err(WebDriverError::from)?;

    let session = session_arc.lock().unwrap();

    Ok(Json(WindowHandlesResponse {
        value: session.get_all_window_handles(),
    }))
}

/// POST /session/:session_id/window - Switch to window
async fn switch_to_window_handler(
    State(state): State<WebDriverState>,
    Path(session_id): Path<String>,
    Json(req): Json<SwitchToWindowRequest>,
) -> WebDriverResult<StatusCode> {
    let session_arc = state
        .session_manager
        .get_session(&session_id)
        .map_err(WebDriverError::from)?;

    let mut session = session_arc.lock().unwrap();
    session
        .switch_to_window(&req.handle)
        .map_err(WebDriverError::from)?;

    Ok(StatusCode::OK)
}

/// DELETE /session/:session_id/window - Close current window
async fn close_window_handler(
    State(state): State<WebDriverState>,
    Path(session_id): Path<String>,
) -> WebDriverResult<Json<WindowHandlesResponse>> {
    let session_arc = state
        .session_manager
        .get_session(&session_id)
        .map_err(WebDriverError::from)?;

    let mut session = session_arc.lock().unwrap();

    // Get current window handle
    let current_handle = session
        .get_current_window()
        .ok_or_else(|| Error::InvalidSession("No active window".to_string()))
        .map_err(WebDriverError::from)?;

    // Close the current window
    let remaining_handles = session
        .close_window(&current_handle)
        .map_err(WebDriverError::from)?;

    // If no windows remain, delete the session
    if remaining_handles.is_empty() {
        drop(session); // Release lock before deleting session
        state
            .session_manager
            .delete_session(&session_id)
            .map_err(WebDriverError::from)?;
    }

    Ok(Json(WindowHandlesResponse {
        value: remaining_handles,
    }))
}

/// POST /session/:session_id/window/new - Create new window
async fn new_window_handler(
    State(state): State<WebDriverState>,
    Path(session_id): Path<String>,
    Json(req): Json<NewWindowRequest>,
) -> WebDriverResult<Json<NewWindowResponse>> {
    let session_arc = state
        .session_manager
        .get_session(&session_id)
        .map_err(WebDriverError::from)?;

    let mut session = session_arc.lock().unwrap();

    // Validate window type
    if req.window_type != "tab" && req.window_type != "window" {
        return Err(WebDriverError::from(Error::InvalidArgument(format!(
            "Invalid window type: {}. Must be 'tab' or 'window'",
            req.window_type
        ))));
    }

    // Create new window handle
    // TODO: Integrate with browser_shell to create actual tab/window
    let new_handle = session.new_window_handle(None);

    // Switch to the new window
    session
        .switch_to_window(&new_handle)
        .map_err(WebDriverError::from)?;

    Ok(Json(NewWindowResponse {
        value: NewWindowValue {
            handle: new_handle,
            window_type: req.window_type,
        },
    }))
}

/// GET /session/:session_id/window/rect - Get window rect
async fn get_window_rect_handler(
    State(state): State<WebDriverState>,
    Path(session_id): Path<String>,
) -> WebDriverResult<Json<WindowRectResponse>> {
    // Verify session exists
    let _session = state
        .session_manager
        .get_session(&session_id)
        .map_err(WebDriverError::from)?;

    // TODO: Integrate with actual window system to get real dimensions
    // For now, return default window rect
    Ok(Json(WindowRectResponse {
        value: WindowRect {
            x: 0,
            y: 0,
            width: 1280,
            height: 720,
        },
    }))
}

/// POST /session/:session_id/window/rect - Set window rect
async fn set_window_rect_handler(
    State(state): State<WebDriverState>,
    Path(session_id): Path<String>,
    Json(req): Json<WindowRectRequest>,
) -> WebDriverResult<Json<WindowRectResponse>> {
    // Verify session exists
    let _session = state
        .session_manager
        .get_session(&session_id)
        .map_err(WebDriverError::from)?;

    // TODO: Integrate with actual window system to set dimensions
    // For now, return the requested rect (or defaults if not provided)
    Ok(Json(WindowRectResponse {
        value: WindowRect {
            x: req.x.unwrap_or(0),
            y: req.y.unwrap_or(0),
            width: req.width.unwrap_or(1280),
            height: req.height.unwrap_or(720),
        },
    }))
}

/// POST /session/:session_id/window/maximize - Maximize window
async fn maximize_window_handler(
    State(state): State<WebDriverState>,
    Path(session_id): Path<String>,
) -> WebDriverResult<Json<WindowRectResponse>> {
    // Verify session exists
    let _session = state
        .session_manager
        .get_session(&session_id)
        .map_err(WebDriverError::from)?;

    // TODO: Integrate with actual window system to maximize window
    // For now, return maximized dimensions (typical full HD)
    Ok(Json(WindowRectResponse {
        value: WindowRect {
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
        },
    }))
}

/// POST /session/:session_id/window/minimize - Minimize window
async fn minimize_window_handler(
    State(state): State<WebDriverState>,
    Path(session_id): Path<String>,
) -> WebDriverResult<Json<WindowRectResponse>> {
    // Verify session exists
    let _session = state
        .session_manager
        .get_session(&session_id)
        .map_err(WebDriverError::from)?;

    // TODO: Integrate with actual window system to minimize window
    // For now, return minimized dimensions (0x0 per WebDriver spec)
    Ok(Json(WindowRectResponse {
        value: WindowRect {
            x: 0,
            y: 0,
            width: 0,
            height: 0,
        },
    }))
}

/// POST /session/:session_id/window/fullscreen - Fullscreen window
async fn fullscreen_window_handler(
    State(state): State<WebDriverState>,
    Path(session_id): Path<String>,
) -> WebDriverResult<Json<WindowRectResponse>> {
    // Verify session exists
    let _session = state
        .session_manager
        .get_session(&session_id)
        .map_err(WebDriverError::from)?;

    // TODO: Integrate with actual window system to fullscreen window
    // For now, return fullscreen dimensions
    Ok(Json(WindowRectResponse {
        value: WindowRect {
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
        },
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

/// Find elements response (multiple elements)
#[derive(Serialize, Debug)]
pub struct FindElementsResponse {
    pub value: Vec<ElementReference>,
}

/// Send keys request
#[derive(Deserialize, Debug)]
pub struct SendKeysRequest {
    pub text: String,
}

/// Text response
#[derive(Serialize, Debug)]
pub struct TextResponse {
    pub value: String,
}

/// Attribute response
#[derive(Serialize, Debug)]
pub struct AttributeResponse {
    pub value: Option<String>,
}

/// Boolean response
#[derive(Serialize, Debug)]
pub struct BooleanResponse {
    pub value: bool,
}

/// Window handles response
#[derive(Serialize, Debug)]
pub struct WindowHandlesResponse {
    pub value: Vec<String>,
}

/// Switch to window request
#[derive(Deserialize, Debug)]
pub struct SwitchToWindowRequest {
    pub handle: String,
}

/// New window request
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct NewWindowRequest {
    #[serde(rename = "type")]
    pub window_type: String, // "tab" or "window"
}

/// New window response
#[derive(Serialize, Debug)]
pub struct NewWindowResponse {
    pub value: NewWindowValue,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct NewWindowValue {
    pub handle: String,
    #[serde(rename = "type")]
    pub window_type: String,
}

/// Window rect value
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct WindowRect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

/// Window rect response
#[derive(Serialize, Debug)]
pub struct WindowRectResponse {
    pub value: WindowRect,
}

/// Window rect request
#[derive(Deserialize, Debug)]
pub struct WindowRectRequest {
    pub x: Option<i32>,
    pub y: Option<i32>,
    pub width: Option<u32>,
    pub height: Option<u32>,
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
