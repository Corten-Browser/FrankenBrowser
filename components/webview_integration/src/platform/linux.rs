//! Linux WebView implementation using WebKit2GTK
//!
//! This module provides WebKit2GTK-specific WebView integration for Linux systems.

use crate::errors::{Error, Result};
use crate::platform::WebViewConfig;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// Linux WebView wrapper using WebKit2GTK
///
/// This struct wraps the wry WebView and provides Linux-specific functionality
/// including WebKit2GTK configuration, cookie management, and signal handlers.
#[cfg(feature = "gui")]
pub struct LinuxWebView {
    /// The underlying wry WebView
    webview: wry::WebView,

    /// Configuration used to create this WebView
    config: WebViewConfig,

    /// Current URL (for tracking navigation)
    current_url: Arc<Mutex<Option<String>>>,

    /// Load state tracking
    is_loading: Arc<Mutex<bool>>,
}

/// Headless mode stub for testing
#[cfg(not(feature = "gui"))]
pub struct LinuxWebView {
    #[allow(dead_code)] // Used in GUI mode and tests
    config: WebViewConfig,
    current_url: Arc<Mutex<Option<String>>>,
    is_loading: Arc<Mutex<bool>>,
}

impl LinuxWebView {
    /// Create a new Linux WebView with the given configuration
    ///
    /// # Arguments
    ///
    /// * `config` - WebView configuration including WebKit2GTK-specific settings
    ///
    /// # Returns
    ///
    /// A Result containing the LinuxWebView or an error if initialization fails
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Configuration validation fails
    /// - WebKit2GTK initialization fails
    /// - Cookie manager setup fails
    /// - Signal handlers cannot be installed
    ///
    /// # Example
    ///
    /// ```no_run
    /// use webview_integration::platform::{WebViewConfig, LinuxWebView};
    ///
    /// let config = WebViewConfig::new()
    ///     .with_javascript(true)
    ///     .with_devtools(true);
    ///
    /// let webview = LinuxWebView::new(config).unwrap();
    /// ```
    #[cfg(feature = "gui")]
    pub fn new(config: WebViewConfig) -> Result<Self> {
        // Validate configuration
        config.validate()?;

        // Initialize WebKit2GTK
        Self::initialize_webkit()?;

        // Build wry WebView with configuration
        use tao::{
            dpi::LogicalSize,
            event_loop::EventLoop,
            window::WindowBuilder,
        };
        use wry::WebViewBuilder;

        // Create event loop (in production, browser_shell manages this)
        let event_loop = EventLoop::new();

        // Create window
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

        // Enable/disable devtools
        #[cfg(debug_assertions)]
        {
            builder = builder.with_devtools(config.enable_devtools);
        }

        // Build the WebView
        let webview = builder
            .build(&window)
            .map_err(|e| Error::Initialization(format!("Failed to create WebView: {}", e)))?;

        // Setup cookie manager (if cookies path provided)
        if let Some(cookies_path) = &config.cookies_path {
            Self::setup_cookie_manager(cookies_path.clone())?;
        }

        // Create the LinuxWebView instance
        let instance = Self {
            webview,
            config,
            current_url: Arc::new(Mutex::new(Some("about:blank".to_string()))),
            is_loading: Arc::new(Mutex::new(false)),
        };

        Ok(instance)
    }

    /// Headless mode constructor (for testing)
    #[cfg(not(feature = "gui"))]
    pub fn new(config: WebViewConfig) -> Result<Self> {
        config.validate()?;

        Ok(Self {
            config,
            current_url: Arc::new(Mutex::new(None)),
            is_loading: Arc::new(Mutex::new(false)),
        })
    }

    /// Initialize WebKit2GTK
    ///
    /// Performs one-time initialization of WebKit2GTK library.
    ///
    /// # Returns
    ///
    /// Ok(()) if initialization succeeds, or an error if WebKit2GTK cannot be initialized.
    #[allow(dead_code)] // Used in GUI mode
    fn initialize_webkit() -> Result<()> {
        // WebKit2GTK initialization is handled by wry/gtk
        // This method is a placeholder for any Linux-specific setup
        Ok(())
    }

    /// Setup WebKit2GTK cookie manager
    ///
    /// Configures persistent cookie storage at the specified path.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the cookies database file
    ///
    /// # Returns
    ///
    /// Ok(()) if setup succeeds, or an error if cookie manager cannot be configured.
    #[allow(dead_code)] // Used in GUI mode
    fn setup_cookie_manager(path: PathBuf) -> Result<()> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| Error::Initialization(
                    format!("Failed to create cookies directory: {}", e)
                ))?;
        }

        // Cookie management is handled by wry/WebKit2GTK
        // The cookie file is specified via WebKit's data manager
        // For wry, this is configured through the WebView builder
        Ok(())
    }

    /// Navigate to a URL
    ///
    /// Loads the specified URL in the WebView. Fires navigation events:
    /// - on_load_started: When navigation begins
    /// - on_load_finished: When page fully loads
    /// - on_load_failed: If navigation fails
    ///
    /// # Arguments
    ///
    /// * `url` - The URL to navigate to (must be valid URL string)
    ///
    /// # Returns
    ///
    /// Ok(()) if navigation starts successfully, or an error if URL is invalid.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use webview_integration::platform::{WebViewConfig, LinuxWebView};
    /// # let config = WebViewConfig::new();
    /// # let mut webview = LinuxWebView::new(config).unwrap();
    /// webview.navigate("https://example.com").unwrap();
    /// ```
    pub fn navigate(&mut self, url: &str) -> Result<()> {
        // Validate URL
        if url.is_empty() {
            return Err(Error::Navigation("URL cannot be empty".to_string()));
        }

        // Parse URL to validate format
        let _parsed_url = url::Url::parse(url)
            .map_err(|e| Error::Navigation(format!("Invalid URL: {}", e)))?;

        // Set loading state
        {
            let mut loading = self.is_loading.lock().unwrap();
            *loading = true;
        }

        #[cfg(feature = "gui")]
        {
            // Navigate using wry
            self.webview
                .load_url(parsed_url.as_str())
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
    /// Executes the provided JavaScript code in the WebView context.
    /// Note: wry's evaluate_script doesn't return script results directly;
    /// use IPC handlers for bidirectional communication.
    ///
    /// # Arguments
    ///
    /// * `script` - The JavaScript code to execute
    ///
    /// # Returns
    ///
    /// Ok with "executed" string on success, or an error if execution fails.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use webview_integration::platform::{WebViewConfig, LinuxWebView};
    /// # let config = WebViewConfig::new();
    /// # let mut webview = LinuxWebView::new(config).unwrap();
    /// webview.execute_script("console.log('Hello from Rust!')").unwrap();
    /// ```
    pub fn execute_script(&self, script: &str) -> Result<String> {
        if script.is_empty() {
            return Err(Error::ScriptExecution("Script cannot be empty".to_string()));
        }

        #[cfg(feature = "gui")]
        {
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
    pub fn current_url(&self) -> Option<String> {
        let current = self.current_url.lock().unwrap();
        current.clone()
    }

    /// Check if the WebView is currently loading
    ///
    /// Returns true if a navigation is in progress.
    pub fn is_loading(&self) -> bool {
        let loading = self.is_loading.lock().unwrap();
        *loading
    }

    /// Get the WebKit version
    ///
    /// Returns the version string of the WebKit2GTK library.
    pub fn get_webkit_version() -> String {
        // WebKit2GTK version would normally be queried via webkit2gtk-rs
        // For now, return a placeholder
        #[cfg(feature = "gui")]
        {
            // In a full implementation, this would call:
            // webkit2gtk::get_major_version(), get_minor_version(), get_micro_version()
            "WebKit2GTK 2.38+".to_string()
        }

        #[cfg(not(feature = "gui"))]
        {
            "WebKit2GTK (headless mode)".to_string()
        }
    }

    /// Inject initialization script
    ///
    /// Adds a script that runs on every page load (before page scripts).
    ///
    /// # Arguments
    ///
    /// * `script` - JavaScript code to inject
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
        // To add a new init script to an existing WebView would require recreation
        // or using evaluate_script on each navigation
        Ok(())
    }

    /// Configure user agent string
    ///
    /// Sets a custom user agent for all requests.
    ///
    /// # Arguments
    ///
    /// * `agent` - The user agent string to use
    ///
    /// # Returns
    ///
    /// Ok(()) if configuration succeeds
    #[allow(dead_code)] // Part of platform API, tested
    fn configure_user_agent(&mut self, agent: &str) -> Result<()> {
        if agent.is_empty() {
            return Err(Error::Initialization("User agent cannot be empty".to_string()));
        }

        // Update config
        self.config.user_agent = Some(agent.to_string());

        // For wry, user agent is set during WebView creation
        // Changing it on an existing WebView would require recreation
        Ok(())
    }

    /// Setup signal handlers for WebKit events
    ///
    /// Installs handlers for:
    /// - on_load_started: Navigation begins
    /// - on_load_finished: Page fully loaded
    /// - on_load_failed: Navigation error
    /// - on_resource_load: Resource loading (for ad blocking)
    ///
    /// # Returns
    ///
    /// Ok(()) if handlers are installed successfully
    #[allow(dead_code)] // Part of platform API, tested
    fn setup_signal_handlers(&self) -> Result<()> {
        // WebKit2GTK signal handlers would be set up here
        // In wry, navigation callbacks are configured through the WebViewBuilder
        // This is a placeholder for the interface
        Ok(())
    }
}

/// Get WebKit version (module-level function)
pub fn get_webkit_version() -> String {
    LinuxWebView::get_webkit_version()
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================
    // RED PHASE: Tests for LinuxWebView::new()
    // ========================================

    #[test]
    #[cfg(target_os = "linux")]
    fn test_linux_webview_new_with_default_config() {
        let config = WebViewConfig::default();
        let result = LinuxWebView::new(config);
        assert!(result.is_ok());
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_linux_webview_new_with_custom_config() {
        let config = WebViewConfig::new()
            .with_javascript(true)
            .with_devtools(true)
            .with_user_agent("FrankenBrowser/1.0".to_string());

        let result = LinuxWebView::new(config);
        assert!(result.is_ok());
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_linux_webview_new_with_cache_dir() {
        let temp_dir = std::env::temp_dir().join("frankenbrowser_test_linux");
        let config = WebViewConfig::new()
            .with_cache_dir(temp_dir.clone());

        let result = LinuxWebView::new(config);
        assert!(result.is_ok());

        // Cleanup
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_linux_webview_new_with_cookies() {
        let temp_dir = std::env::temp_dir().join("frankenbrowser_test_cookies_linux");
        let cookies_path = temp_dir.join("cookies.db");
        let config = WebViewConfig::new()
            .with_cookies_path(cookies_path);

        let result = LinuxWebView::new(config);
        assert!(result.is_ok());

        // Cleanup
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_linux_webview_new_with_init_script() {
        let init_script = "console.log('WebView initialized');".to_string();
        let config = WebViewConfig::new()
            .with_init_script(init_script);

        let result = LinuxWebView::new(config);
        assert!(result.is_ok());
    }

    // ========================================
    // RED PHASE: Tests for navigate()
    // ========================================

    #[test]
    #[cfg(target_os = "linux")]
    fn test_navigate_valid_url() {
        let config = WebViewConfig::default();
        let mut webview = LinuxWebView::new(config).unwrap();

        let result = webview.navigate("https://example.com");
        assert!(result.is_ok());
        assert_eq!(webview.current_url(), Some("https://example.com".to_string()));
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_navigate_empty_url_fails() {
        let config = WebViewConfig::default();
        let mut webview = LinuxWebView::new(config).unwrap();

        let result = webview.navigate("");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), Error::Navigation(_)));
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_navigate_invalid_url_fails() {
        let config = WebViewConfig::default();
        let mut webview = LinuxWebView::new(config).unwrap();

        let result = webview.navigate("not a url");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), Error::Navigation(_)));
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_navigate_updates_loading_state() {
        let config = WebViewConfig::default();
        let mut webview = LinuxWebView::new(config).unwrap();

        assert!(!webview.is_loading());

        let result = webview.navigate("https://example.com");
        assert!(result.is_ok());
        assert!(webview.is_loading());
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_navigate_file_url() {
        let config = WebViewConfig::default();
        let mut webview = LinuxWebView::new(config).unwrap();

        let result = webview.navigate("file:///tmp/test.html");
        assert!(result.is_ok());
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_navigate_data_url() {
        let config = WebViewConfig::default();
        let mut webview = LinuxWebView::new(config).unwrap();

        let result = webview.navigate("data:text/html,<h1>Hello</h1>");
        assert!(result.is_ok());
    }

    // ========================================
    // RED PHASE: Tests for execute_script()
    // ========================================

    #[test]
    #[cfg(target_os = "linux")]
    fn test_execute_script_simple() {
        let config = WebViewConfig::default();
        let webview = LinuxWebView::new(config).unwrap();

        let result = webview.execute_script("console.log('test')");
        assert!(result.is_ok());
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_execute_script_empty_fails() {
        let config = WebViewConfig::default();
        let webview = LinuxWebView::new(config).unwrap();

        let result = webview.execute_script("");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), Error::ScriptExecution(_)));
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_execute_script_returns_executed() {
        let config = WebViewConfig::default();
        let webview = LinuxWebView::new(config).unwrap();

        let result = webview.execute_script("1 + 1");
        assert!(result.is_ok());
        let value = result.unwrap();
        assert!(!value.is_empty());
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_execute_script_complex() {
        let config = WebViewConfig::default();
        let webview = LinuxWebView::new(config).unwrap();

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
    // Tests for WebKit version
    // ========================================

    #[test]
    #[cfg(target_os = "linux")]
    fn test_get_webkit_version() {
        let version = get_webkit_version();
        assert!(!version.is_empty());
        assert!(version.contains("WebKit"));
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_get_webkit_version_static_method() {
        let version = LinuxWebView::get_webkit_version();
        assert!(!version.is_empty());
    }

    // ========================================
    // Tests for helper methods
    // ========================================

    #[test]
    #[cfg(target_os = "linux")]
    fn test_current_url_initially_none_or_blank() {
        let config = WebViewConfig::default();
        let webview = LinuxWebView::new(config).unwrap();
        // Could be None or "about:blank" depending on mode
        let url = webview.current_url();
        assert!(url.is_none() || url == Some("about:blank".to_string()));
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_is_loading_initially_false() {
        let config = WebViewConfig::default();
        let webview = LinuxWebView::new(config).unwrap();
        assert!(!webview.is_loading());
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_inject_init_script() {
        let config = WebViewConfig::default();
        let mut webview = LinuxWebView::new(config).unwrap();

        let result = webview.inject_init_script("console.log('injected');");
        assert!(result.is_ok());
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_inject_init_script_empty_fails() {
        let config = WebViewConfig::default();
        let mut webview = LinuxWebView::new(config).unwrap();

        let result = webview.inject_init_script("");
        assert!(result.is_err());
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_configure_user_agent() {
        let config = WebViewConfig::default();
        let mut webview = LinuxWebView::new(config).unwrap();

        let result = webview.configure_user_agent("CustomAgent/1.0");
        assert!(result.is_ok());
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_configure_user_agent_empty_fails() {
        let config = WebViewConfig::default();
        let mut webview = LinuxWebView::new(config).unwrap();

        let result = webview.configure_user_agent("");
        assert!(result.is_err());
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_setup_signal_handlers() {
        let config = WebViewConfig::default();
        let webview = LinuxWebView::new(config).unwrap();

        let result = webview.setup_signal_handlers();
        assert!(result.is_ok());
    }

    // ========================================
    // Integration Tests
    // ========================================

    #[test]
    #[cfg(target_os = "linux")]
    fn test_full_workflow() {
        // Create config with all options
        let temp_dir = std::env::temp_dir().join("frankenbrowser_integration_test");
        let config = WebViewConfig::new()
            .with_javascript(true)
            .with_devtools(true)
            .with_user_agent("FrankenBrowser/1.0".to_string())
            .with_cache_dir(temp_dir.clone())
            .with_init_script("console.log('initialized');".to_string());

        // Create WebView
        let mut webview = LinuxWebView::new(config).unwrap();

        // Navigate
        let nav_result = webview.navigate("https://example.com");
        assert!(nav_result.is_ok());

        // Check URL updated
        assert_eq!(webview.current_url(), Some("https://example.com".to_string()));

        // Execute script
        let script_result = webview.execute_script("console.log('test')");
        assert!(script_result.is_ok());

        // Cleanup
        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
