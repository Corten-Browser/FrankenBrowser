//! Platform-specific WebView implementations
//!
//! This module provides platform-specific WebView integration code:
//! - Linux: WebKit2GTK
//! - Windows: WebView2
//! - macOS: WKWebView
//!
//! Each platform has its own module with a platform-specific WebView wrapper.

use crate::errors::Result;
use std::path::PathBuf;

// Platform-specific modules with conditional compilation
#[cfg(target_os = "linux")]
pub mod linux;

#[cfg(target_os = "windows")]
pub mod windows;

#[cfg(target_os = "macos")]
pub mod macos;

// Re-export platform-specific types
#[cfg(target_os = "linux")]
pub use linux::LinuxWebView as PlatformWebView;

#[cfg(target_os = "windows")]
pub use windows::WindowsWebView as PlatformWebView;

#[cfg(target_os = "macos")]
pub use macos::MacOSWebView as PlatformWebView;

/// Configuration for WebView creation
///
/// This struct contains all the configuration options for creating a WebView
/// instance, including platform-specific settings.
#[derive(Debug, Clone)]
pub struct WebViewConfig {
    /// Enable JavaScript execution (default: true)
    pub enable_javascript: bool,

    /// Enable developer tools (default: false in release, true in debug)
    pub enable_devtools: bool,

    /// Custom user agent string (default: platform default)
    pub user_agent: Option<String>,

    /// Cache directory for WebView (default: system temp)
    pub cache_dir: Option<PathBuf>,

    /// Cookies storage path (default: None - in-memory only)
    pub cookies_path: Option<PathBuf>,

    /// JavaScript initialization script (injected on page load)
    pub init_script: Option<String>,

    /// Enable local storage (default: true)
    pub enable_local_storage: bool,

    /// Enable WebGL (default: true)
    pub enable_webgl: bool,
}

impl Default for WebViewConfig {
    fn default() -> Self {
        Self {
            enable_javascript: true,
            enable_devtools: cfg!(debug_assertions),
            user_agent: None,
            cache_dir: None,
            cookies_path: None,
            init_script: None,
            enable_local_storage: true,
            enable_webgl: true,
        }
    }
}

impl WebViewConfig {
    /// Create a new WebViewConfig with default settings
    pub fn new() -> Self {
        Self::default()
    }

    /// Set whether to enable JavaScript
    pub fn with_javascript(mut self, enable: bool) -> Self {
        self.enable_javascript = enable;
        self
    }

    /// Set whether to enable developer tools
    pub fn with_devtools(mut self, enable: bool) -> Self {
        self.enable_devtools = enable;
        self
    }

    /// Set custom user agent
    pub fn with_user_agent(mut self, agent: String) -> Self {
        self.user_agent = Some(agent);
        self
    }

    /// Set cache directory
    pub fn with_cache_dir(mut self, dir: PathBuf) -> Self {
        self.cache_dir = Some(dir);
        self
    }

    /// Set cookies storage path
    pub fn with_cookies_path(mut self, path: PathBuf) -> Self {
        self.cookies_path = Some(path);
        self
    }

    /// Set initialization script
    pub fn with_init_script(mut self, script: String) -> Self {
        self.init_script = Some(script);
        self
    }

    /// Set whether to enable local storage
    pub fn with_local_storage(mut self, enable: bool) -> Self {
        self.enable_local_storage = enable;
        self
    }

    /// Set whether to enable WebGL
    pub fn with_webgl(mut self, enable: bool) -> Self {
        self.enable_webgl = enable;
        self
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<()> {
        // Check cache directory exists or can be created
        if let Some(cache_dir) = &self.cache_dir {
            if !cache_dir.exists() {
                std::fs::create_dir_all(cache_dir).map_err(|e| {
                    crate::errors::Error::Initialization(format!(
                        "Failed to create cache directory: {}",
                        e
                    ))
                })?;
            }
        }

        // Check cookies path parent directory exists
        if let Some(cookies_path) = &self.cookies_path {
            if let Some(parent) = cookies_path.parent() {
                if !parent.exists() {
                    std::fs::create_dir_all(parent).map_err(|e| {
                        crate::errors::Error::Initialization(format!(
                            "Failed to create cookies directory: {}",
                            e
                        ))
                    })?;
                }
            }
        }

        Ok(())
    }
}

/// Platform detection utilities
pub fn get_platform_name() -> &'static str {
    #[cfg(target_os = "linux")]
    return "Linux (WebKit2GTK)";

    #[cfg(target_os = "windows")]
    return "Windows (WebView2)";

    #[cfg(target_os = "macos")]
    return "macOS (WKWebView)";

    #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
    return "Unknown";
}

/// Get the WebKit/WebView version for the current platform
pub fn get_webview_version() -> String {
    #[cfg(target_os = "linux")]
    return linux::get_webkit_version();

    #[cfg(target_os = "windows")]
    return "WebView2 (runtime version)".to_string();

    #[cfg(target_os = "macos")]
    return "WKWebView (system version)".to_string();

    #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
    return "Unknown".to_string();
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================
    // RED PHASE: Tests for WebViewConfig
    // ========================================

    #[test]
    fn test_webview_config_default() {
        let config = WebViewConfig::default();
        assert!(config.enable_javascript);
        assert!(config.enable_local_storage);
        assert!(config.enable_webgl);
        assert_eq!(config.user_agent, None);
        assert_eq!(config.cache_dir, None);
        assert_eq!(config.cookies_path, None);
        assert_eq!(config.init_script, None);
    }

    #[test]
    fn test_webview_config_new() {
        let config = WebViewConfig::new();
        assert!(config.enable_javascript);
    }

    #[test]
    fn test_webview_config_builder_javascript() {
        let config = WebViewConfig::new().with_javascript(false);
        assert!(!config.enable_javascript);
    }

    #[test]
    fn test_webview_config_builder_devtools() {
        let config = WebViewConfig::new().with_devtools(true);
        assert!(config.enable_devtools);
    }

    #[test]
    fn test_webview_config_builder_user_agent() {
        let config = WebViewConfig::new().with_user_agent("CustomAgent/1.0".to_string());
        assert_eq!(config.user_agent, Some("CustomAgent/1.0".to_string()));
    }

    #[test]
    fn test_webview_config_builder_cache_dir() {
        let path = PathBuf::from("/tmp/test_cache");
        let config = WebViewConfig::new().with_cache_dir(path.clone());
        assert_eq!(config.cache_dir, Some(path));
    }

    #[test]
    fn test_webview_config_builder_cookies_path() {
        let path = PathBuf::from("/tmp/test_cookies.db");
        let config = WebViewConfig::new().with_cookies_path(path.clone());
        assert_eq!(config.cookies_path, Some(path));
    }

    #[test]
    fn test_webview_config_builder_init_script() {
        let script = "console.log('init');".to_string();
        let config = WebViewConfig::new().with_init_script(script.clone());
        assert_eq!(config.init_script, Some(script));
    }

    #[test]
    fn test_webview_config_builder_local_storage() {
        let config = WebViewConfig::new().with_local_storage(false);
        assert!(!config.enable_local_storage);
    }

    #[test]
    fn test_webview_config_builder_webgl() {
        let config = WebViewConfig::new().with_webgl(false);
        assert!(!config.enable_webgl);
    }

    #[test]
    fn test_webview_config_builder_chaining() {
        let config = WebViewConfig::new()
            .with_javascript(true)
            .with_devtools(true)
            .with_user_agent("Test/1.0".to_string())
            .with_local_storage(true)
            .with_webgl(true);

        assert!(config.enable_javascript);
        assert!(config.enable_devtools);
        assert_eq!(config.user_agent, Some("Test/1.0".to_string()));
        assert!(config.enable_local_storage);
        assert!(config.enable_webgl);
    }

    #[test]
    fn test_webview_config_validate_no_paths() {
        let config = WebViewConfig::new();
        let result = config.validate();
        assert!(result.is_ok());
    }

    #[test]
    fn test_webview_config_validate_with_cache_dir() {
        let temp_dir = std::env::temp_dir().join("frankenbrowser_test_cache");
        let config = WebViewConfig::new().with_cache_dir(temp_dir.clone());

        let result = config.validate();
        assert!(result.is_ok());

        // Cleanup
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_webview_config_validate_with_cookies_path() {
        let temp_dir = std::env::temp_dir().join("frankenbrowser_test_cookies");
        let cookies_path = temp_dir.join("cookies.db");
        let config = WebViewConfig::new().with_cookies_path(cookies_path);

        let result = config.validate();
        assert!(result.is_ok());

        // Cleanup
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    // ========================================
    // Tests for platform utilities
    // ========================================

    #[test]
    fn test_get_platform_name() {
        let platform = get_platform_name();
        assert!(!platform.is_empty());

        #[cfg(target_os = "linux")]
        assert_eq!(platform, "Linux (WebKit2GTK)");

        #[cfg(target_os = "windows")]
        assert_eq!(platform, "Windows (WebView2)");

        #[cfg(target_os = "macos")]
        assert_eq!(platform, "macOS (WKWebView)");
    }

    #[test]
    fn test_get_webview_version() {
        let version = get_webview_version();
        assert!(!version.is_empty());
    }
}
