//! macOS WebView implementation using WKWebView
//!
//! This module provides WKWebView-specific WebView integration for macOS systems.
//! WKWebView is Apple's modern web rendering engine based on WebKit.
//!
//! # Implementation Status
//!
//! This is a Phase 1 stub implementation providing:
//! - ✅ API compatibility with Linux/Windows platforms
//! - ✅ Cross-platform compilation and testing
//! - ✅ Comprehensive documentation for future implementation
//! - ⏸️ Full WKWebView integration deferred to Phase 3.2.3
//!
//! # Platform-Specific Features
//!
//! WKWebView provides macOS-specific capabilities:
//! - Native WebKit integration (system framework)
//! - Web Inspector (Safari developer tools)
//! - JavaScript/Swift message handlers
//! - Sandboxed web content process
//! - Continuity features (Handoff, AirDrop)
//! - Touch Bar support
//!
//! # Storage Locations
//!
//! macOS WebView data is stored in:
//! - Cookies: `~/Library/Cookies/FrankenBrowser/`
//! - Cache: `~/Library/Caches/FrankenBrowser/WebView/`
//! - User data: `~/Library/Application Support/FrankenBrowser/`
//!
//! # Sandboxing Considerations
//!
//! When building a sandboxed macOS app:
//! - Enable "Outgoing Connections (Client)" entitlement for network access
//! - Enable "User Selected File (Read/Write)" for local file access
//! - Configure Info.plist with appropriate usage descriptions

use crate::errors::{Error, Result};
use crate::platform::WebViewConfig;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// macOS WebView wrapper using WKWebView
///
/// This struct wraps the wry WebView and provides macOS-specific functionality
/// including WKWebView configuration, cookie management, and navigation handlers.
///
/// # Platform Support
///
/// This implementation uses conditional compilation:
/// - `#[cfg(target_os = "macos")]` for macOS-specific code
/// - `#[cfg(feature = "gui")]` for GUI mode with actual WebView
/// - `#[cfg(not(feature = "gui"))]` for headless testing mode
///
/// # Example
///
/// ```no_run
/// use webview_integration::platform::{WebViewConfig, MacOSWebView};
///
/// let config = WebViewConfig::new()
///     .with_javascript(true)
///     .with_devtools(true);
///
/// let webview = MacOSWebView::new(config).unwrap();
/// ```
#[cfg(feature = "gui")]
pub struct MacOSWebView {
    /// The underlying wry WebView (wraps WKWebView on macOS)
    webview: wry::WebView,

    /// Configuration used to create this WebView
    config: WebViewConfig,

    /// Current URL (for tracking navigation)
    current_url: Arc<Mutex<Option<String>>>,

    /// Load state tracking
    is_loading: Arc<Mutex<bool>>,
}

/// Headless mode stub for testing on non-macOS platforms
///
/// This allows tests to compile and run on Linux/Windows without requiring macOS.
#[cfg(not(feature = "gui"))]
pub struct MacOSWebView {
    #[allow(dead_code)] // Used in GUI mode and tests
    config: WebViewConfig,
    current_url: Arc<Mutex<Option<String>>>,
    is_loading: Arc<Mutex<bool>>,
}

impl MacOSWebView {
    /// Create a new macOS WebView with the given configuration
    ///
    /// # Arguments
    ///
    /// * `config` - WebView configuration including WKWebView-specific settings
    ///
    /// # Returns
    ///
    /// A Result containing the MacOSWebView or an error if initialization fails
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Configuration validation fails
    /// - WKWebView initialization fails
    /// - Cookie manager setup fails
    /// - Navigation handlers cannot be installed
    ///
    /// # Platform-Specific Details
    ///
    /// On macOS:
    /// - Uses WKWebView from WebKit framework
    /// - Supports Web Inspector (Safari DevTools)
    /// - Integrates with macOS Keychain for authentication
    /// - Respects system proxy settings
    ///
    /// # Example
    ///
    /// ```no_run
    /// use webview_integration::platform::{WebViewConfig, MacOSWebView};
    ///
    /// let config = WebViewConfig::new()
    ///     .with_javascript(true)
    ///     .with_devtools(true)
    ///     .with_user_agent("FrankenBrowser/1.0 (macOS)".to_string());
    ///
    /// let webview = MacOSWebView::new(config).unwrap();
    /// ```
    #[cfg(feature = "gui")]
    pub fn new(config: WebViewConfig) -> Result<Self> {
        // Validate configuration
        config.validate()?;

        // Initialize WKWebView (platform-specific setup)
        Self::initialize_wkwebview()?;

        // Build wry WebView with configuration
        use tao::{dpi::LogicalSize, event_loop::EventLoop, window::WindowBuilder};
        use wry::WebViewBuilder;

        // Create event loop (in production, browser_shell manages this)
        let event_loop = EventLoop::new();

        // Create window with macOS-specific styling
        let window = WindowBuilder::new()
            .with_title("FrankenBrowser")
            .with_inner_size(LogicalSize::new(1024, 768))
            .build(&event_loop)
            .map_err(|e| Error::Initialization(format!("Failed to create window: {}", e)))?;

        // Build WebView with configuration
        let mut builder = WebViewBuilder::new();

        // Set initial URL
        builder = builder.with_url("about:blank");

        // Configure user agent
        if let Some(user_agent) = &config.user_agent {
            builder = builder.with_user_agent(user_agent);
        }

        // Add initialization script
        if let Some(init_script) = &config.init_script {
            builder = builder.with_initialization_script(init_script);
        }

        // Enable/disable devtools (Web Inspector on macOS)
        #[cfg(debug_assertions)]
        {
            builder = builder.with_devtools(config.enable_devtools);
        }

        // Build the WebView (wry creates WKWebView on macOS)
        let webview = builder
            .build(&window)
            .map_err(|e| Error::Initialization(format!("Failed to create WKWebView: {}", e)))?;

        // Setup cookie manager (if cookies path provided)
        if let Some(cookies_path) = &config.cookies_path {
            Self::setup_cookie_manager(cookies_path.clone())?;
        }

        // Create the MacOSWebView instance
        let instance = Self {
            webview,
            config,
            current_url: Arc::new(Mutex::new(Some("about:blank".to_string()))),
            is_loading: Arc::new(Mutex::new(false)),
        };

        Ok(instance)
    }

    /// Headless mode constructor (for testing on non-macOS platforms)
    ///
    /// This allows cross-platform testing without requiring macOS or WKWebView.
    #[cfg(not(feature = "gui"))]
    pub fn new(config: WebViewConfig) -> Result<Self> {
        config.validate()?;

        Ok(Self {
            config,
            current_url: Arc::new(Mutex::new(None)),
            is_loading: Arc::new(Mutex::new(false)),
        })
    }

    /// Initialize WKWebView framework
    ///
    /// Performs one-time initialization of WKWebView components.
    ///
    /// # Implementation Notes
    ///
    /// WKWebView initialization on macOS involves:
    /// - Setting up WKProcessPool (shared across WebViews)
    /// - Configuring WKWebsiteDataStore (cookies, cache, local storage)
    /// - Setting WKPreferences (JavaScript, plugins, etc.)
    /// - Configuring WKUserContentController (script injection)
    ///
    /// # Future Implementation
    ///
    /// Phase 3.2.3 will add:
    /// - WKProcessPool configuration for process management
    /// - WKWebsiteDataStore setup for persistent data
    /// - WKPreferences configuration for web content
    ///
    /// # Returns
    ///
    /// Ok(()) if initialization succeeds, or an error if WKWebView cannot be initialized.
    #[allow(dead_code)] // Used in GUI mode
    fn initialize_wkwebview() -> Result<()> {
        // WKWebView initialization is handled by wry/tao on macOS
        // This method is a placeholder for any macOS-specific setup
        // that might be needed in the future.
        //
        // Future work (Phase 3.2.3):
        // - Configure WKProcessPool for process management
        // - Set up WKWebsiteDataStore for cookies/cache
        // - Configure WKPreferences for JavaScript/plugins
        // - Set up WKUserContentController for script injection
        Ok(())
    }

    /// Setup WKWebView cookie manager
    ///
    /// Configures persistent cookie storage at the specified path.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the cookies storage directory
    ///
    /// # Implementation Notes
    ///
    /// WKWebView cookie management involves:
    /// - WKHTTPCookieStore for cookie storage
    /// - WKWebsiteDataStore for persistent data
    /// - Integration with macOS HTTPCookieStorage
    ///
    /// # macOS Storage Locations
    ///
    /// Default cookie storage:
    /// - `~/Library/Cookies/FrankenBrowser/` (default)
    /// - Custom path if specified in config
    ///
    /// # Future Implementation
    ///
    /// Phase 3.2.3 will add:
    /// - WKHTTPCookieStore configuration
    /// - Cookie persistence to specified path
    /// - Integration with system cookie storage
    ///
    /// # Returns
    ///
    /// Ok(()) if setup succeeds, or an error if cookie manager cannot be configured.
    #[allow(dead_code)] // Used in GUI mode
    fn setup_cookie_manager(path: PathBuf) -> Result<()> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                Error::Initialization(format!("Failed to create cookies directory: {}", e))
            })?;
        }

        // Cookie management is handled by wry/WKWebView
        // WKHTTPCookieStore is used for cookie storage
        // The cookie file is specified via WKWebsiteDataStore
        //
        // Future work (Phase 3.2.3):
        // - Configure WKHTTPCookieStore with custom path
        // - Set up cookie persistence policy
        // - Handle cookie synchronization with system
        Ok(())
    }

    /// Configure user agent string
    ///
    /// Sets a custom user agent for all HTTP requests.
    ///
    /// # Arguments
    ///
    /// * `agent` - The user agent string to use
    ///
    /// # Implementation Notes
    ///
    /// WKWebView user agent configuration:
    /// - Set via WKWebViewConfiguration.applicationNameForUserAgent
    /// - Or override entirely with customUserAgent
    /// - Affects all HTTP/HTTPS requests
    ///
    /// # Future Implementation
    ///
    /// Phase 3.2.3 will add:
    /// - WKWebViewConfiguration.customUserAgent setting
    /// - User agent override per navigation
    ///
    /// # Returns
    ///
    /// Ok(()) if configuration succeeds
    ///
    /// # Note
    ///
    /// User agent should be set during WebView creation. Changing it
    /// on an existing WebView requires recreation.
    #[allow(dead_code)] // Part of platform API, tested
    fn configure_user_agent(&mut self, agent: &str) -> Result<()> {
        if agent.is_empty() {
            return Err(Error::Initialization(
                "User agent cannot be empty".to_string(),
            ));
        }

        // Update config
        self.config.user_agent = Some(agent.to_string());

        // For wry/WKWebView, user agent is set during WebView creation
        // via WKWebViewConfiguration.customUserAgent
        // Changing it on an existing WebView would require recreation
        //
        // Future work (Phase 3.2.3):
        // - Support dynamic user agent changes if possible
        // - Or provide guidance on recreating WebView
        Ok(())
    }

    /// Setup navigation event handlers
    ///
    /// Installs handlers for WKWebView navigation events:
    /// - decidePolicyForNavigationAction: Before navigation starts
    /// - didStartProvisionalNavigation: Navigation begins
    /// - didCommitNavigation: Content starts loading
    /// - didFinishNavigation: Page fully loaded
    /// - didFailNavigation: Navigation error
    ///
    /// # Implementation Notes
    ///
    /// WKWebView provides rich navigation delegate callbacks:
    /// - Policy decisions (allow/deny navigation)
    /// - Progress tracking (0.0 to 1.0)
    /// - Error handling with NSError codes
    /// - Response validation
    ///
    /// # Future Implementation
    ///
    /// Phase 3.2.3 will add:
    /// - WKNavigationDelegate implementation
    /// - Navigation policy callbacks
    /// - Progress tracking (estimatedProgress)
    /// - Error handling for failed navigations
    ///
    /// # Returns
    ///
    /// Ok(()) if handlers are installed successfully
    #[allow(dead_code)] // Part of platform API, tested
    fn setup_navigation_handlers(&self) -> Result<()> {
        // Navigation callbacks are configured through wry's WebViewBuilder
        // WKNavigationDelegate is used for navigation events
        // This is a placeholder for the interface
        //
        // Future work (Phase 3.2.3):
        // - Implement WKNavigationDelegate callbacks
        // - Handle decidePolicyForNavigationAction
        // - Track didStartProvisionalNavigation
        // - Monitor didFinishNavigation
        // - Handle didFailNavigation errors
        Ok(())
    }

    /// Inject initialization script
    ///
    /// Adds a script that runs on every page load (before page scripts).
    ///
    /// # Arguments
    ///
    /// * `script` - JavaScript code to inject
    ///
    /// # Implementation Notes
    ///
    /// WKWebView script injection uses WKUserContentController:
    /// - addUserScript() for page-load injection
    /// - WKUserScriptInjectionTime for timing (document start/end)
    /// - forMainFrameOnly for targeting main frame vs all frames
    ///
    /// # Future Implementation
    ///
    /// Phase 3.2.3 will add:
    /// - WKUserContentController.addUserScript() integration
    /// - Script injection timing control
    /// - Main frame vs all frames targeting
    ///
    /// # Returns
    ///
    /// Ok(()) if injection succeeds, or an error if script injection fails.
    #[allow(dead_code)] // Part of platform API, tested
    fn inject_init_script(&mut self, script: &str) -> Result<()> {
        if script.is_empty() {
            return Err(Error::ScriptExecution("Script cannot be empty".to_string()));
        }

        // Update config with new init script
        self.config.init_script = Some(script.to_string());

        // For wry, init scripts are set during WebView creation
        // via WKUserContentController.addUserScript()
        // To add a new init script to an existing WebView would require:
        // - Accessing WKUserContentController
        // - Adding WKUserScript with injectionTime = .atDocumentStart
        //
        // Future work (Phase 3.2.3):
        // - Support dynamic script injection via WKUserContentController
        // - Or provide guidance on script timing
        Ok(())
    }

    /// Navigate to a URL
    ///
    /// Loads the specified URL in the WebView. Fires navigation events:
    /// - decidePolicyForNavigationAction: Before navigation (WKWebView)
    /// - didStartProvisionalNavigation: When navigation begins
    /// - didFinishNavigation: When page fully loads
    /// - didFailNavigation: If navigation fails
    ///
    /// # Arguments
    ///
    /// * `url` - The URL to navigate to (must be valid URL string)
    ///
    /// # Returns
    ///
    /// Ok(()) if navigation starts successfully, or an error if URL is invalid.
    ///
    /// # Supported URL Schemes
    ///
    /// - `https://` - Secure web pages
    /// - `http://` - Web pages (WKWebView allows by default)
    /// - `file://` - Local files (requires sandbox entitlement)
    /// - `data:` - Data URIs
    /// - `about:` - Special pages (about:blank)
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use webview_integration::platform::{WebViewConfig, MacOSWebView};
    /// # let config = WebViewConfig::new();
    /// # let mut webview = MacOSWebView::new(config).unwrap();
    /// webview.navigate("https://example.com").unwrap();
    /// ```
    pub fn navigate(&mut self, url: &str) -> Result<()> {
        // Validate URL
        if url.is_empty() {
            return Err(Error::Navigation("URL cannot be empty".to_string()));
        }

        // Parse URL to validate format
        let _parsed_url =
            url::Url::parse(url).map_err(|e| Error::Navigation(format!("Invalid URL: {}", e)))?;

        // Set loading state
        {
            let mut loading = self.is_loading.lock().unwrap();
            *loading = true;
        }

        #[cfg(feature = "gui")]
        {
            // Navigate using wry (which uses WKWebView.load() on macOS)
            self.webview
                .load_url(_parsed_url.as_str())
                .map_err(|e| Error::Navigation(format!("Failed to navigate: {}", e)))?;
        }

        // Update current URL
        {
            let mut current = self.current_url.lock().unwrap();
            *current = Some(url.to_string());
        }

        Ok(())
    }

    /// Execute JavaScript in the WebView
    ///
    /// Executes the provided JavaScript code in the WKWebView context.
    ///
    /// # Arguments
    ///
    /// * `script` - The JavaScript code to execute
    ///
    /// # Returns
    ///
    /// Ok with "executed" string on success, or an error if execution fails.
    ///
    /// # Implementation Notes
    ///
    /// WKWebView script execution:
    /// - Uses evaluateJavaScript(_:completionHandler:)
    /// - Runs in web content process (sandboxed)
    /// - Can return values via completion handler
    /// - Supports async execution
    ///
    /// Note: wry's evaluate_script doesn't return script results directly;
    /// use IPC handlers (WKScriptMessageHandler) for bidirectional communication.
    ///
    /// # Future Implementation
    ///
    /// Phase 3.2.3 will add:
    /// - Result capture from evaluateJavaScript
    /// - WKScriptMessageHandler for IPC
    /// - Error handling for script execution failures
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use webview_integration::platform::{WebViewConfig, MacOSWebView};
    /// # let config = WebViewConfig::new();
    /// # let mut webview = MacOSWebView::new(config).unwrap();
    /// webview.execute_script("console.log('Hello from Rust!')").unwrap();
    /// ```
    pub fn execute_script(&self, script: &str) -> Result<String> {
        if script.is_empty() {
            return Err(Error::ScriptExecution("Script cannot be empty".to_string()));
        }

        #[cfg(feature = "gui")]
        {
            // Execute script using wry (which uses WKWebView.evaluateJavaScript on macOS)
            self.webview
                .evaluate_script(script)
                .map_err(|e| Error::ScriptExecution(format!("Script execution failed: {}", e)))?;
            Ok("executed".to_string())
        }

        #[cfg(not(feature = "gui"))]
        {
            // In headless mode, simulate execution
            Ok("null".to_string())
        }
    }

    /// Get the current URL
    ///
    /// Returns the URL currently loaded in the WebView, or None if no page is loaded.
    ///
    /// # Implementation Notes
    ///
    /// On macOS, this would typically query WKWebView.url property.
    /// Currently tracks URL via internal state.
    pub fn current_url(&self) -> Option<String> {
        let current = self.current_url.lock().unwrap();
        current.clone()
    }

    /// Check if the WebView is currently loading
    ///
    /// Returns true if a navigation is in progress.
    ///
    /// # Implementation Notes
    ///
    /// On macOS, this would typically query WKWebView.isLoading property.
    /// Currently tracks loading state via internal state.
    pub fn is_loading(&self) -> bool {
        let loading = self.is_loading.lock().unwrap();
        *loading
    }

    /// Get the WKWebView version
    ///
    /// Returns the version string of the WKWebView/WebKit framework.
    ///
    /// # Returns
    ///
    /// Version string indicating WKWebView (WebKit)
    ///
    /// # Implementation Notes
    ///
    /// WKWebView version detection on macOS:
    /// - WebKit framework version from CFBundleVersion
    /// - System WebKit (built into macOS)
    /// - Version tied to macOS version
    ///
    /// # Future Implementation
    ///
    /// Phase 3.2.3 will add:
    /// - Query WebKit framework version
    /// - Detect macOS version
    /// - Report WebKit build number
    ///
    /// # Example
    ///
    /// ```
    /// use webview_integration::platform::MacOSWebView;
    /// let version = MacOSWebView::get_wkwebview_version();
    /// assert!(version.contains("WKWebView"));
    /// ```
    pub fn get_wkwebview_version() -> String {
        #[cfg(feature = "gui")]
        {
            // In a full implementation, this would query:
            // - WebKit framework version (CFBundleVersion)
            // - macOS version (ProcessInfo.operatingSystemVersion)
            // - WebKit build number
            //
            // For now, return a descriptive placeholder
            "WKWebView (WebKit)".to_string()
        }

        #[cfg(not(feature = "gui"))]
        {
            "WKWebView (headless mode)".to_string()
        }
    }
}

/// Get WKWebView version (module-level function)
///
/// Returns the version string of the WKWebView framework.
///
/// # Example
///
/// ```
/// use webview_integration::platform::macos::get_wkwebview_version;
/// let version = get_wkwebview_version();
/// assert!(!version.is_empty());
/// ```
pub fn get_wkwebview_version() -> String {
    MacOSWebView::get_wkwebview_version()
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================
    // RED PHASE: Tests for MacOSWebView::new()
    // ========================================

    #[test]
    #[cfg(target_os = "macos")]
    fn test_macos_webview_new_with_default_config() {
        let config = WebViewConfig::default();
        let result = MacOSWebView::new(config);
        assert!(result.is_ok());
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_macos_webview_new_with_custom_config() {
        let config = WebViewConfig::new()
            .with_javascript(true)
            .with_devtools(true)
            .with_user_agent("FrankenBrowser/1.0 (macOS)".to_string());

        let result = MacOSWebView::new(config);
        assert!(result.is_ok());
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_macos_webview_new_with_cache_dir() {
        let temp_dir = std::env::temp_dir().join("frankenbrowser_test_macos");
        let config = WebViewConfig::new().with_cache_dir(temp_dir.clone());

        let result = MacOSWebView::new(config);
        assert!(result.is_ok());

        // Cleanup
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_macos_webview_new_with_cookies() {
        let temp_dir = std::env::temp_dir().join("frankenbrowser_test_cookies_macos");
        let cookies_path = temp_dir.join("cookies.db");
        let config = WebViewConfig::new().with_cookies_path(cookies_path);

        let result = MacOSWebView::new(config);
        assert!(result.is_ok());

        // Cleanup
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_macos_webview_new_with_init_script() {
        let init_script = "console.log('WKWebView initialized');".to_string();
        let config = WebViewConfig::new().with_init_script(init_script);

        let result = MacOSWebView::new(config);
        assert!(result.is_ok());
    }

    // ========================================
    // Cross-platform tests (run on all platforms)
    // ========================================

    #[test]
    fn test_macos_webview_headless_creation() {
        // This test works on all platforms in headless mode
        let config = WebViewConfig::default();
        #[cfg(not(feature = "gui"))]
        {
            let result = MacOSWebView::new(config);
            assert!(result.is_ok());
        }
        #[cfg(feature = "gui")]
        {
            // Skip on non-macOS platforms in GUI mode
            #[cfg(not(target_os = "macos"))]
            {
                // Test would fail on non-macOS, so skip
                assert!(true);
            }
            #[cfg(target_os = "macos")]
            {
                let result = MacOSWebView::new(config);
                assert!(result.is_ok());
            }
        }
    }

    // ========================================
    // RED PHASE: Tests for navigate()
    // ========================================

    #[test]
    #[cfg(target_os = "macos")]
    fn test_navigate_valid_url() {
        let config = WebViewConfig::default();
        let mut webview = MacOSWebView::new(config).unwrap();

        let result = webview.navigate("https://example.com");
        assert!(result.is_ok());
        assert_eq!(
            webview.current_url(),
            Some("https://example.com".to_string())
        );
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_navigate_empty_url_fails() {
        let config = WebViewConfig::default();
        let mut webview = MacOSWebView::new(config).unwrap();

        let result = webview.navigate("");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), Error::Navigation(_)));
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_navigate_invalid_url_fails() {
        let config = WebViewConfig::default();
        let mut webview = MacOSWebView::new(config).unwrap();

        let result = webview.navigate("not a url");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), Error::Navigation(_)));
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_navigate_updates_loading_state() {
        let config = WebViewConfig::default();
        let mut webview = MacOSWebView::new(config).unwrap();

        assert!(!webview.is_loading());

        let result = webview.navigate("https://example.com");
        assert!(result.is_ok());
        assert!(webview.is_loading());
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_navigate_file_url() {
        let config = WebViewConfig::default();
        let mut webview = MacOSWebView::new(config).unwrap();

        let result = webview.navigate("file:///tmp/test.html");
        assert!(result.is_ok());
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_navigate_data_url() {
        let config = WebViewConfig::default();
        let mut webview = MacOSWebView::new(config).unwrap();

        let result = webview.navigate("data:text/html,<h1>Hello</h1>");
        assert!(result.is_ok());
    }

    // ========================================
    // RED PHASE: Tests for execute_script()
    // ========================================

    #[test]
    #[cfg(target_os = "macos")]
    fn test_execute_script_simple() {
        let config = WebViewConfig::default();
        let webview = MacOSWebView::new(config).unwrap();

        let result = webview.execute_script("console.log('test')");
        assert!(result.is_ok());
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_execute_script_empty_fails() {
        let config = WebViewConfig::default();
        let webview = MacOSWebView::new(config).unwrap();

        let result = webview.execute_script("");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), Error::ScriptExecution(_)));
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_execute_script_returns_executed() {
        let config = WebViewConfig::default();
        let webview = MacOSWebView::new(config).unwrap();

        let result = webview.execute_script("1 + 1");
        assert!(result.is_ok());
        let value = result.unwrap();
        assert!(!value.is_empty());
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_execute_script_complex() {
        let config = WebViewConfig::default();
        let webview = MacOSWebView::new(config).unwrap();

        let script = r#"
            (function() {
                const div = document.createElement('div');
                div.id = 'test';
                document.body.appendChild(div);
            })();
        "#;

        let result = webview.execute_script(script);
        assert!(result.is_ok());
    }

    // ========================================
    // Tests for WKWebView version
    // ========================================

    #[test]
    fn test_get_wkwebview_version() {
        let version = get_wkwebview_version();
        assert!(!version.is_empty());
        assert!(version.contains("WKWebView"));
    }

    #[test]
    fn test_get_wkwebview_version_static_method() {
        let version = MacOSWebView::get_wkwebview_version();
        assert!(!version.is_empty());
    }

    // ========================================
    // Tests for helper methods
    // ========================================

    #[test]
    #[cfg(target_os = "macos")]
    fn test_current_url_initially_none_or_blank() {
        let config = WebViewConfig::default();
        let webview = MacOSWebView::new(config).unwrap();
        // Could be None or "about:blank" depending on mode
        let url = webview.current_url();
        assert!(url.is_none() || url == Some("about:blank".to_string()));
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_is_loading_initially_false() {
        let config = WebViewConfig::default();
        let webview = MacOSWebView::new(config).unwrap();
        assert!(!webview.is_loading());
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_inject_init_script() {
        let config = WebViewConfig::default();
        let mut webview = MacOSWebView::new(config).unwrap();

        let result = webview.inject_init_script("console.log('injected');");
        assert!(result.is_ok());
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_inject_init_script_empty_fails() {
        let config = WebViewConfig::default();
        let mut webview = MacOSWebView::new(config).unwrap();

        let result = webview.inject_init_script("");
        assert!(result.is_err());
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_configure_user_agent() {
        let config = WebViewConfig::default();
        let mut webview = MacOSWebView::new(config).unwrap();

        let result = webview.configure_user_agent("CustomAgent/1.0 (macOS)");
        assert!(result.is_ok());
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_configure_user_agent_empty_fails() {
        let config = WebViewConfig::default();
        let mut webview = MacOSWebView::new(config).unwrap();

        let result = webview.configure_user_agent("");
        assert!(result.is_err());
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_setup_navigation_handlers() {
        let config = WebViewConfig::default();
        let webview = MacOSWebView::new(config).unwrap();

        let result = webview.setup_navigation_handlers();
        assert!(result.is_ok());
    }

    // ========================================
    // Integration Tests
    // ========================================

    #[test]
    #[cfg(target_os = "macos")]
    fn test_full_workflow() {
        // Create config with all options
        let temp_dir = std::env::temp_dir().join("frankenbrowser_integration_test_macos");
        let config = WebViewConfig::new()
            .with_javascript(true)
            .with_devtools(true)
            .with_user_agent("FrankenBrowser/1.0 (macOS)".to_string())
            .with_cache_dir(temp_dir.clone())
            .with_init_script("console.log('initialized');".to_string());

        // Create WebView
        let mut webview = MacOSWebView::new(config).unwrap();

        // Navigate
        let nav_result = webview.navigate("https://example.com");
        assert!(nav_result.is_ok());

        // Check URL updated
        assert_eq!(
            webview.current_url(),
            Some("https://example.com".to_string())
        );

        // Execute script
        let script_result = webview.execute_script("console.log('test')");
        assert!(script_result.is_ok());

        // Cleanup
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    // ========================================
    // Cross-platform fallback tests
    // ========================================

    #[test]
    #[cfg(not(target_os = "macos"))]
    fn test_macos_webview_on_non_macos_headless() {
        // When not on macOS, we can still test the headless implementation
        #[cfg(not(feature = "gui"))]
        {
            let config = WebViewConfig::default();
            let result = MacOSWebView::new(config);
            assert!(result.is_ok());

            if let Ok(webview) = result {
                assert!(!webview.is_loading());
                assert!(webview.current_url().is_none());
            }
        }
        #[cfg(feature = "gui")]
        {
            // In GUI mode on non-macOS, just verify the test compiles
            assert!(true);
        }
    }
}
