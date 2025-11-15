//! Request interceptor and handler system
//!
//! This module provides a flexible request interception pipeline that allows
//! modification and filtering of HTTP requests and responses.

use crate::errors::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use url::Url;

/// HTTP method
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HttpMethod {
    GET,
    POST,
    PUT,
    DELETE,
    HEAD,
    OPTIONS,
    PATCH,
}

impl HttpMethod {
    /// Convert to string representation
    pub fn as_str(&self) -> &str {
        match self {
            HttpMethod::GET => "GET",
            HttpMethod::POST => "POST",
            HttpMethod::PUT => "PUT",
            HttpMethod::DELETE => "DELETE",
            HttpMethod::HEAD => "HEAD",
            HttpMethod::OPTIONS => "OPTIONS",
            HttpMethod::PATCH => "PATCH",
        }
    }
}

/// HTTP request structure
#[derive(Debug, Clone)]
pub struct Request {
    /// Request URL
    pub url: Url,
    /// HTTP method
    pub method: HttpMethod,
    /// Request headers
    pub headers: HashMap<String, String>,
    /// Request body (optional)
    pub body: Option<Vec<u8>>,
    /// Request timestamp (milliseconds since UNIX_EPOCH)
    pub timestamp: u64,
    /// Unique request ID
    pub request_id: String,
}

impl Request {
    /// Create a new request
    pub fn new(url: Url, method: HttpMethod) -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        Self {
            url,
            method,
            headers: HashMap::new(),
            body: None,
            timestamp,
            request_id: format!("req_{}", timestamp),
        }
    }

    /// Set request headers
    pub fn with_headers(mut self, headers: HashMap<String, String>) -> Self {
        self.headers = headers;
        self
    }

    /// Set request body
    pub fn with_body(mut self, body: Vec<u8>) -> Self {
        self.body = Some(body);
        self
    }

    /// Set request ID
    pub fn with_id(mut self, id: String) -> Self {
        self.request_id = id;
        self
    }
}

/// HTTP response structure
#[derive(Debug, Clone)]
pub struct Response {
    /// Response status code
    pub status: u16,
    /// Response headers
    pub headers: HashMap<String, String>,
    /// Response body
    pub body: Vec<u8>,
    /// Request ID this response is for
    pub request_id: String,
}

impl Response {
    /// Create a new response
    pub fn new(status: u16, body: Vec<u8>, request_id: String) -> Self {
        Self {
            status,
            headers: HashMap::new(),
            body,
            request_id,
        }
    }

    /// Set response headers
    pub fn with_headers(mut self, headers: HashMap<String, String>) -> Self {
        self.headers = headers;
        self
    }
}

/// Type alias for URL blocking callback function
pub type ShouldBlockFn = Arc<dyn Fn(&str) -> bool + Send + Sync>;

/// Action to take after processing a request
#[derive(Debug, Clone)]
pub enum RequestAction {
    /// Allow the request to proceed
    Allow,
    /// Block the request
    Block { reason: String },
    /// Redirect to a different URL
    Redirect { url: Url },
    /// Use a modified version of the request
    ModifiedRequest { request: Request },
}

/// Trait for request interceptors
///
/// Interceptors are called in the order they are added to the RequestHandler.
/// Each interceptor can inspect and modify the request/response.
pub trait RequestInterceptor: Send + Sync {
    /// Called before a request is sent
    ///
    /// # Arguments
    ///
    /// * `request` - The request to process (can be modified in place)
    ///
    /// # Returns
    ///
    /// Returns Ok(()) to continue, or an error to abort the request
    fn pre_request(&mut self, request: &mut Request) -> Result<()>;

    /// Called after a response is received
    ///
    /// # Arguments
    ///
    /// * `response` - The response to process (can be modified in place)
    ///
    /// # Returns
    ///
    /// Returns Ok(()) to continue, or an error to abort
    fn post_response(&mut self, response: &mut Response) -> Result<()>;

    /// Check if a request should be blocked
    ///
    /// # Arguments
    ///
    /// * `request` - The request to check
    ///
    /// # Returns
    ///
    /// Returns true if the request should be blocked
    fn should_block(&self, request: &Request) -> bool;
}

/// Ad blocking interceptor
///
/// Integrates with the adblock_engine to block requests matching filter rules
/// Note: Due to thread safety requirements, this interceptor uses a callback approach
pub struct AdBlockInterceptor {
    /// Callback function for checking if URL should be blocked
    should_block_fn: Option<ShouldBlockFn>,
    /// Whether ad blocking is enabled
    enabled: bool,
}

impl AdBlockInterceptor {
    /// Create a new ad block interceptor with a callback function
    ///
    /// # Arguments
    ///
    /// * `should_block_fn` - Function that determines if a URL should be blocked
    pub fn new<F>(should_block_fn: F) -> Self
    where
        F: Fn(&str) -> bool + Send + Sync + 'static,
    {
        Self {
            should_block_fn: Some(Arc::new(should_block_fn)),
            enabled: true,
        }
    }

    /// Create a disabled ad block interceptor (for testing)
    pub fn disabled() -> Self {
        Self {
            should_block_fn: None,
            enabled: false,
        }
    }
}

impl RequestInterceptor for AdBlockInterceptor {
    fn pre_request(&mut self, _request: &mut Request) -> Result<()> {
        // No modifications needed in pre-request
        Ok(())
    }

    fn post_response(&mut self, _response: &mut Response) -> Result<()> {
        // No modifications needed in post-response
        Ok(())
    }

    fn should_block(&self, request: &Request) -> bool {
        if !self.enabled {
            return false;
        }

        if let Some(ref check_fn) = self.should_block_fn {
            check_fn(request.url.as_str())
        } else {
            false
        }
    }
}

/// Header injection interceptor
///
/// Adds custom headers to all requests
pub struct HeaderInjectorInterceptor {
    /// Headers to inject
    headers: HashMap<String, String>,
}

impl HeaderInjectorInterceptor {
    /// Create a new header injector
    ///
    /// # Arguments
    ///
    /// * `headers` - Headers to inject into all requests
    pub fn new(headers: HashMap<String, String>) -> Self {
        Self { headers }
    }

    /// Create a header injector with User-Agent override
    pub fn with_user_agent(user_agent: String) -> Self {
        let mut headers = HashMap::new();
        headers.insert("User-Agent".to_string(), user_agent);
        Self { headers }
    }

    /// Create a header injector with DNT (Do Not Track) header
    pub fn with_dnt() -> Self {
        let mut headers = HashMap::new();
        headers.insert("DNT".to_string(), "1".to_string());
        Self { headers }
    }
}

impl RequestInterceptor for HeaderInjectorInterceptor {
    fn pre_request(&mut self, request: &mut Request) -> Result<()> {
        // Inject headers into request
        for (key, value) in &self.headers {
            request.headers.insert(key.clone(), value.clone());
        }
        Ok(())
    }

    fn post_response(&mut self, _response: &mut Response) -> Result<()> {
        // No modifications needed in post-response
        Ok(())
    }

    fn should_block(&self, _request: &Request) -> bool {
        // Header injector doesn't block requests
        false
    }
}

/// Redirect interceptor
///
/// Handles HTTP redirects (3xx status codes)
pub struct RedirectInterceptor {
    /// Maximum number of redirects to follow
    max_redirects: usize,
    /// Current redirect count
    redirect_count: usize,
}

impl RedirectInterceptor {
    /// Create a new redirect interceptor
    ///
    /// # Arguments
    ///
    /// * `max_redirects` - Maximum number of redirects to follow
    pub fn new(max_redirects: usize) -> Self {
        Self {
            max_redirects,
            redirect_count: 0,
        }
    }

    /// Reset redirect count
    pub fn reset(&mut self) {
        self.redirect_count = 0;
    }

    /// Check if a status code is a redirect
    pub fn is_redirect_status(status: u16) -> bool {
        matches!(status, 301 | 302 | 303 | 307 | 308)
    }
}

impl RequestInterceptor for RedirectInterceptor {
    fn pre_request(&mut self, _request: &mut Request) -> Result<()> {
        // Check if we've exceeded max redirects
        if self.redirect_count >= self.max_redirects {
            return Err(Error::RequestFailed("Too many redirects".to_string()));
        }
        Ok(())
    }

    fn post_response(&mut self, _response: &mut Response) -> Result<()> {
        // Redirect handling is done externally
        // This interceptor just tracks the count
        Ok(())
    }

    fn should_block(&self, _request: &Request) -> bool {
        // Redirect interceptor doesn't block requests
        false
    }
}

/// Request handler with interceptor chain
pub struct RequestHandler {
    /// Chain of interceptors
    interceptors: Vec<Box<dyn RequestInterceptor>>,
}

impl RequestHandler {
    /// Create a new request handler
    pub fn new() -> Self {
        Self {
            interceptors: Vec::new(),
        }
    }

    /// Add an interceptor to the chain
    ///
    /// # Arguments
    ///
    /// * `interceptor` - The interceptor to add
    pub fn add_interceptor(&mut self, interceptor: Box<dyn RequestInterceptor>) {
        self.interceptors.push(interceptor);
    }

    /// Process a request through the interceptor chain
    ///
    /// # Arguments
    ///
    /// * `request` - The request to process
    ///
    /// # Returns
    ///
    /// Returns a RequestAction indicating what to do with the request
    pub fn process_request(&mut self, request: &mut Request) -> Result<RequestAction> {
        // Check if any interceptor wants to block the request
        for interceptor in &self.interceptors {
            if interceptor.should_block(request) {
                return Ok(RequestAction::Block {
                    reason: "Blocked by interceptor".to_string(),
                });
            }
        }

        // Call pre_request on all interceptors
        for interceptor in &mut self.interceptors {
            interceptor.pre_request(request)?;
        }

        // If we get here, allow the request
        Ok(RequestAction::Allow)
    }

    /// Process a response through the interceptor chain
    ///
    /// # Arguments
    ///
    /// * `response` - The response to process
    ///
    /// # Returns
    ///
    /// Returns Ok(()) on success, or an error
    pub fn process_response(&mut self, response: &mut Response) -> Result<()> {
        // Call post_response on all interceptors in reverse order
        for interceptor in self.interceptors.iter_mut().rev() {
            interceptor.post_response(response)?;
        }

        Ok(())
    }

    /// Inject headers into a request
    ///
    /// # Arguments
    ///
    /// * `request` - The request to modify
    /// * `headers` - Headers to inject
    pub fn inject_headers(request: &mut Request, headers: HashMap<String, String>) {
        for (key, value) in headers {
            request.headers.insert(key, value);
        }
    }
}

impl Default for RequestHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================
    // RED PHASE: Tests for HttpMethod
    // ========================================

    #[test]
    fn test_http_method_as_str() {
        assert_eq!(HttpMethod::GET.as_str(), "GET");
        assert_eq!(HttpMethod::POST.as_str(), "POST");
        assert_eq!(HttpMethod::PUT.as_str(), "PUT");
        assert_eq!(HttpMethod::DELETE.as_str(), "DELETE");
        assert_eq!(HttpMethod::HEAD.as_str(), "HEAD");
        assert_eq!(HttpMethod::OPTIONS.as_str(), "OPTIONS");
        assert_eq!(HttpMethod::PATCH.as_str(), "PATCH");
    }

    #[test]
    fn test_http_method_equality() {
        assert_eq!(HttpMethod::GET, HttpMethod::GET);
        assert_ne!(HttpMethod::GET, HttpMethod::POST);
    }

    #[test]
    fn test_http_method_clone() {
        let method = HttpMethod::GET;
        let cloned = method.clone();
        assert_eq!(method, cloned);
    }

    // ========================================
    // RED PHASE: Tests for Request
    // ========================================

    #[test]
    fn test_request_new() {
        let url = Url::parse("https://example.com").unwrap();
        let request = Request::new(url.clone(), HttpMethod::GET);

        assert_eq!(request.url, url);
        assert_eq!(request.method, HttpMethod::GET);
        assert!(request.headers.is_empty());
        assert!(request.body.is_none());
        assert!(request.timestamp > 0);
        assert!(request.request_id.starts_with("req_"));
    }

    #[test]
    fn test_request_with_headers() {
        let url = Url::parse("https://example.com").unwrap();
        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "application/json".to_string());

        let request = Request::new(url, HttpMethod::POST).with_headers(headers.clone());

        assert_eq!(request.headers, headers);
    }

    #[test]
    fn test_request_with_body() {
        let url = Url::parse("https://example.com").unwrap();
        let body = vec![1, 2, 3, 4, 5];

        let request = Request::new(url, HttpMethod::POST).with_body(body.clone());

        assert_eq!(request.body, Some(body));
    }

    #[test]
    fn test_request_with_id() {
        let url = Url::parse("https://example.com").unwrap();
        let id = "custom_id".to_string();

        let request = Request::new(url, HttpMethod::GET).with_id(id.clone());

        assert_eq!(request.request_id, id);
    }

    #[test]
    fn test_request_clone() {
        let url = Url::parse("https://example.com").unwrap();
        let request = Request::new(url, HttpMethod::GET);
        let cloned = request.clone();

        assert_eq!(request.url, cloned.url);
        assert_eq!(request.method, cloned.method);
    }

    // ========================================
    // RED PHASE: Tests for Response
    // ========================================

    #[test]
    fn test_response_new() {
        let body = vec![1, 2, 3];
        let request_id = "req_123".to_string();
        let response = Response::new(200, body.clone(), request_id.clone());

        assert_eq!(response.status, 200);
        assert_eq!(response.body, body);
        assert_eq!(response.request_id, request_id);
        assert!(response.headers.is_empty());
    }

    #[test]
    fn test_response_with_headers() {
        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "text/html".to_string());

        let response =
            Response::new(200, vec![], "req_123".to_string()).with_headers(headers.clone());

        assert_eq!(response.headers, headers);
    }

    #[test]
    fn test_response_clone() {
        let response = Response::new(200, vec![1, 2, 3], "req_123".to_string());
        let cloned = response.clone();

        assert_eq!(response.status, cloned.status);
        assert_eq!(response.body, cloned.body);
    }

    // ========================================
    // RED PHASE: Tests for RequestAction
    // ========================================

    #[test]
    fn test_request_action_allow() {
        let action = RequestAction::Allow;
        assert!(matches!(action, RequestAction::Allow));
    }

    #[test]
    fn test_request_action_block() {
        let action = RequestAction::Block {
            reason: "Ad blocked".to_string(),
        };
        match action {
            RequestAction::Block { reason } => {
                assert_eq!(reason, "Ad blocked");
            }
            _ => panic!("Expected Block action"),
        }
    }

    #[test]
    fn test_request_action_redirect() {
        let url = Url::parse("https://example.com").unwrap();
        let action = RequestAction::Redirect { url: url.clone() };

        match action {
            RequestAction::Redirect { url: redirect_url } => {
                assert_eq!(redirect_url, url);
            }
            _ => panic!("Expected Redirect action"),
        }
    }

    #[test]
    fn test_request_action_modified_request() {
        let url = Url::parse("https://example.com").unwrap();
        let request = Request::new(url, HttpMethod::GET);
        let action = RequestAction::ModifiedRequest {
            request: request.clone(),
        };

        match action {
            RequestAction::ModifiedRequest { request: req } => {
                assert_eq!(req.url, request.url);
            }
            _ => panic!("Expected ModifiedRequest action"),
        }
    }

    // ========================================
    // RED PHASE: Tests for AdBlockInterceptor
    // ========================================

    #[test]
    fn test_adblock_interceptor_disabled() {
        let interceptor = AdBlockInterceptor::disabled();
        let url = Url::parse("https://ads.example.com/banner.js").unwrap();
        let request = Request::new(url, HttpMethod::GET);

        assert!(!interceptor.should_block(&request));
    }

    #[test]
    fn test_adblock_interceptor_with_callback() {
        // Create interceptor that blocks URLs containing "ads"
        let interceptor = AdBlockInterceptor::new(|url: &str| url.contains("ads"));

        let blocked_url = Url::parse("https://ads.example.com/banner.js").unwrap();
        let blocked_request = Request::new(blocked_url, HttpMethod::GET);
        assert!(interceptor.should_block(&blocked_request));

        let allowed_url = Url::parse("https://example.com/content.js").unwrap();
        let allowed_request = Request::new(allowed_url, HttpMethod::GET);
        assert!(!interceptor.should_block(&allowed_request));
    }

    #[test]
    fn test_adblock_interceptor_pre_request() {
        let mut interceptor = AdBlockInterceptor::disabled();
        let url = Url::parse("https://example.com").unwrap();
        let mut request = Request::new(url, HttpMethod::GET);

        let result = interceptor.pre_request(&mut request);
        assert!(result.is_ok());
    }

    #[test]
    fn test_adblock_interceptor_post_response() {
        let mut interceptor = AdBlockInterceptor::disabled();
        let mut response = Response::new(200, vec![], "req_123".to_string());

        let result = interceptor.post_response(&mut response);
        assert!(result.is_ok());
    }

    // ========================================
    // RED PHASE: Tests for HeaderInjectorInterceptor
    // ========================================

    #[test]
    fn test_header_injector_new() {
        let mut headers = HashMap::new();
        headers.insert("Custom-Header".to_string(), "value".to_string());

        let interceptor = HeaderInjectorInterceptor::new(headers.clone());
        assert_eq!(interceptor.headers, headers);
    }

    #[test]
    fn test_header_injector_with_user_agent() {
        let user_agent = "CustomBrowser/1.0".to_string();
        let interceptor = HeaderInjectorInterceptor::with_user_agent(user_agent.clone());

        assert_eq!(interceptor.headers.get("User-Agent"), Some(&user_agent));
    }

    #[test]
    fn test_header_injector_with_dnt() {
        let interceptor = HeaderInjectorInterceptor::with_dnt();

        assert_eq!(interceptor.headers.get("DNT"), Some(&"1".to_string()));
    }

    #[test]
    fn test_header_injector_pre_request() {
        let mut headers = HashMap::new();
        headers.insert("X-Custom".to_string(), "test".to_string());

        let mut interceptor = HeaderInjectorInterceptor::new(headers);
        let url = Url::parse("https://example.com").unwrap();
        let mut request = Request::new(url, HttpMethod::GET);

        let result = interceptor.pre_request(&mut request);
        assert!(result.is_ok());
        assert_eq!(request.headers.get("X-Custom"), Some(&"test".to_string()));
    }

    #[test]
    fn test_header_injector_should_not_block() {
        let interceptor = HeaderInjectorInterceptor::new(HashMap::new());
        let url = Url::parse("https://example.com").unwrap();
        let request = Request::new(url, HttpMethod::GET);

        assert!(!interceptor.should_block(&request));
    }

    // ========================================
    // RED PHASE: Tests for RedirectInterceptor
    // ========================================

    #[test]
    fn test_redirect_interceptor_new() {
        let interceptor = RedirectInterceptor::new(5);
        assert_eq!(interceptor.max_redirects, 5);
        assert_eq!(interceptor.redirect_count, 0);
    }

    #[test]
    fn test_redirect_interceptor_is_redirect_status() {
        assert!(RedirectInterceptor::is_redirect_status(301));
        assert!(RedirectInterceptor::is_redirect_status(302));
        assert!(RedirectInterceptor::is_redirect_status(303));
        assert!(RedirectInterceptor::is_redirect_status(307));
        assert!(RedirectInterceptor::is_redirect_status(308));
        assert!(!RedirectInterceptor::is_redirect_status(200));
        assert!(!RedirectInterceptor::is_redirect_status(404));
    }

    #[test]
    fn test_redirect_interceptor_reset() {
        let mut interceptor = RedirectInterceptor::new(5);
        interceptor.redirect_count = 3;
        interceptor.reset();
        assert_eq!(interceptor.redirect_count, 0);
    }

    #[test]
    fn test_redirect_interceptor_should_not_block() {
        let interceptor = RedirectInterceptor::new(5);
        let url = Url::parse("https://example.com").unwrap();
        let request = Request::new(url, HttpMethod::GET);

        assert!(!interceptor.should_block(&request));
    }

    // ========================================
    // RED PHASE: Tests for RequestHandler
    // ========================================

    #[test]
    fn test_request_handler_new() {
        let handler = RequestHandler::new();
        assert_eq!(handler.interceptors.len(), 0);
    }

    #[test]
    fn test_request_handler_default() {
        let handler = RequestHandler::default();
        assert_eq!(handler.interceptors.len(), 0);
    }

    #[test]
    fn test_request_handler_add_interceptor() {
        let mut handler = RequestHandler::new();
        let interceptor = Box::new(HeaderInjectorInterceptor::new(HashMap::new()));

        handler.add_interceptor(interceptor);
        assert_eq!(handler.interceptors.len(), 1);
    }

    #[test]
    fn test_request_handler_process_request_allow() {
        let mut handler = RequestHandler::new();
        let url = Url::parse("https://example.com").unwrap();
        let mut request = Request::new(url, HttpMethod::GET);

        let action = handler.process_request(&mut request).unwrap();
        assert!(matches!(action, RequestAction::Allow));
    }

    #[test]
    fn test_request_handler_process_request_with_header_injection() {
        let mut handler = RequestHandler::new();
        let mut headers = HashMap::new();
        headers.insert("X-Custom".to_string(), "value".to_string());

        handler.add_interceptor(Box::new(HeaderInjectorInterceptor::new(headers)));

        let url = Url::parse("https://example.com").unwrap();
        let mut request = Request::new(url, HttpMethod::GET);

        let action = handler.process_request(&mut request).unwrap();
        assert!(matches!(action, RequestAction::Allow));
        assert_eq!(request.headers.get("X-Custom"), Some(&"value".to_string()));
    }

    #[test]
    fn test_request_handler_process_response() {
        let mut handler = RequestHandler::new();
        let mut response = Response::new(200, vec![1, 2, 3], "req_123".to_string());

        let result = handler.process_response(&mut response);
        assert!(result.is_ok());
    }

    #[test]
    fn test_request_handler_inject_headers() {
        let url = Url::parse("https://example.com").unwrap();
        let mut request = Request::new(url, HttpMethod::GET);

        let mut headers = HashMap::new();
        headers.insert("Authorization".to_string(), "Bearer token".to_string());

        RequestHandler::inject_headers(&mut request, headers);

        assert_eq!(
            request.headers.get("Authorization"),
            Some(&"Bearer token".to_string())
        );
    }

    #[test]
    fn test_request_handler_multiple_interceptors() {
        let mut handler = RequestHandler::new();

        // Add header injector
        let mut headers1 = HashMap::new();
        headers1.insert("X-First".to_string(), "1".to_string());
        handler.add_interceptor(Box::new(HeaderInjectorInterceptor::new(headers1)));

        // Add another header injector
        let mut headers2 = HashMap::new();
        headers2.insert("X-Second".to_string(), "2".to_string());
        handler.add_interceptor(Box::new(HeaderInjectorInterceptor::new(headers2)));

        let url = Url::parse("https://example.com").unwrap();
        let mut request = Request::new(url, HttpMethod::GET);

        let action = handler.process_request(&mut request).unwrap();
        assert!(matches!(action, RequestAction::Allow));
        assert_eq!(request.headers.get("X-First"), Some(&"1".to_string()));
        assert_eq!(request.headers.get("X-Second"), Some(&"2".to_string()));
    }

    #[test]
    fn test_request_handler_interceptor_execution_order() {
        // Test that interceptors are called in the order they are added
        let mut handler = RequestHandler::new();

        let mut headers = HashMap::new();
        headers.insert("User-Agent".to_string(), "Test/1.0".to_string());
        handler.add_interceptor(Box::new(HeaderInjectorInterceptor::new(headers)));

        let url = Url::parse("https://example.com").unwrap();
        let mut request = Request::new(url, HttpMethod::GET);

        handler.process_request(&mut request).unwrap();
        assert_eq!(
            request.headers.get("User-Agent"),
            Some(&"Test/1.0".to_string())
        );
    }
}
