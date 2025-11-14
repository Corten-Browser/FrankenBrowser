//! Configuration loading, saving, and management
//!
//! This component provides configuration management for the FrankenBrowser project.
//! It handles loading configuration from TOML files, providing default values,
//! and saving configuration changes.
//!
//! # Example
//!
//! ```rust
//! use config_manager::Config;
//!
//! // Load config or use defaults
//! let config = Config::load_or_default().unwrap();
//!
//! // Access configuration sections
//! let network = config.network_config();
//! let adblock = config.adblock_config();
//! ```

use serde::{Deserialize, Serialize};
use shared_types::{BrowserError, Result};
use std::path::Path;

/// Main configuration structure for FrankenBrowser
///
/// Contains all configuration sections for the browser.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Config {
    /// Browser-specific settings
    pub browser: BrowserSettings,
    /// Network configuration
    pub network: NetworkSettings,
    /// AdBlock configuration
    pub adblock: AdBlockSettings,
    /// Privacy settings
    pub privacy: PrivacySettings,
    /// Appearance settings
    pub appearance: AppearanceSettings,
}

/// Browser-specific settings
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BrowserSettings {
    /// Default homepage URL
    pub homepage: String,
    /// Enable developer tools
    pub enable_devtools: bool,
    /// Default search engine
    pub default_search_engine: String,
}

/// Network configuration settings
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NetworkSettings {
    /// Maximum connections per host
    pub max_connections_per_host: u32,
    /// Request timeout in seconds
    pub timeout_seconds: u32,
    /// Enable cookies
    pub enable_cookies: bool,
    /// Enable cache
    pub enable_cache: bool,
    /// Cache size in megabytes
    pub cache_size_mb: u32,
}

/// AdBlock configuration settings
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AdBlockSettings {
    /// Enable ad blocking
    pub enabled: bool,
    /// Update filters on startup
    pub update_filters_on_startup: bool,
    /// Custom filter rules
    pub custom_filters: Vec<String>,
}

/// Privacy settings
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PrivacySettings {
    /// Send Do Not Track header
    pub do_not_track: bool,
    /// Clear cookies on exit
    pub clear_cookies_on_exit: bool,
    /// Block third-party cookies
    pub block_third_party_cookies: bool,
}

/// Appearance settings
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AppearanceSettings {
    /// Theme: "light", "dark", or "auto"
    pub theme: String,
    /// Default zoom level
    pub default_zoom: f64,
}

/// Network configuration subset for network components
#[derive(Debug, Clone, PartialEq)]
pub struct NetworkConfig {
    /// Maximum connections per host
    pub max_connections_per_host: u32,
    /// Request timeout in seconds
    pub timeout_seconds: u32,
    /// Enable cookies
    pub enable_cookies: bool,
    /// Enable cache
    pub enable_cache: bool,
    /// Cache size in megabytes
    pub cache_size_mb: u32,
}

/// AdBlock configuration subset for adblock components
#[derive(Debug, Clone, PartialEq)]
pub struct AdBlockConfig {
    /// Enable ad blocking
    pub enabled: bool,
    /// Update filters on startup
    pub update_filters_on_startup: bool,
    /// Custom filter rules
    pub custom_filters: Vec<String>,
}

/// Shell configuration subset for shell components
#[derive(Debug, Clone, PartialEq)]
pub struct ShellConfig {
    /// Default homepage URL
    pub homepage: String,
    /// Enable developer tools
    pub enable_devtools: bool,
    /// Theme setting
    pub theme: String,
    /// Default zoom level
    pub default_zoom: f64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            browser: BrowserSettings {
                homepage: "https://www.google.com".to_string(),
                enable_devtools: true,
                default_search_engine: "google".to_string(),
            },
            network: NetworkSettings {
                max_connections_per_host: 6,
                timeout_seconds: 30,
                enable_cookies: true,
                enable_cache: true,
                cache_size_mb: 500,
            },
            adblock: AdBlockSettings {
                enabled: true,
                update_filters_on_startup: false,
                custom_filters: vec![],
            },
            privacy: PrivacySettings {
                do_not_track: true,
                clear_cookies_on_exit: false,
                block_third_party_cookies: false,
            },
            appearance: AppearanceSettings {
                theme: "auto".to_string(),
                default_zoom: 1.0,
            },
        }
    }
}

impl Config {
    /// Load configuration from a file or return default configuration
    ///
    /// This function attempts to load configuration from the default location.
    /// If the file doesn't exist or cannot be read, it returns the default configuration.
    ///
    /// # Returns
    ///
    /// Returns a `Result<Config>` containing either the loaded configuration or default.
    ///
    /// # Errors
    ///
    /// Returns an error if the file exists but contains invalid TOML.
    pub fn load_or_default() -> Result<Self> {
        let default_path = std::env::var("FRANKENBROWSER_CONFIG")
            .unwrap_or_else(|_| "~/.config/frankenbrowser/config.toml".to_string());

        let expanded_path = shellexpand::tilde(&default_path);
        let path = Path::new(expanded_path.as_ref());

        if path.exists() {
            Self::load_from_file(path)
        } else {
            Ok(Self::default())
        }
    }

    /// Load configuration from a specific file
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the configuration file
    ///
    /// # Returns
    ///
    /// Returns a `Result<Config>` containing the loaded configuration.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The file doesn't exist
    /// - The file cannot be read
    /// - The file contains invalid TOML
    pub fn load_from_file(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path).map_err(|e| {
            BrowserError::Other(anyhow::anyhow!("Failed to read config file: {}", e))
        })?;

        let config: Config = toml::from_str(&content)
            .map_err(|e| BrowserError::Other(anyhow::anyhow!("Failed to parse TOML: {}", e)))?;

        Ok(config)
    }

    /// Save configuration to a file
    ///
    /// # Arguments
    ///
    /// * `path` - Path where the configuration should be saved
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The file cannot be created
    /// - The file cannot be written
    /// - The configuration cannot be serialized
    pub fn save_to_file(&self, path: &Path) -> Result<()> {
        // Create parent directory if it doesn't exist
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                BrowserError::Other(anyhow::anyhow!("Failed to create config directory: {}", e))
            })?;
        }

        let toml_string = toml::to_string_pretty(self).map_err(|e| {
            BrowserError::Other(anyhow::anyhow!("Failed to serialize config: {}", e))
        })?;

        std::fs::write(path, toml_string).map_err(|e| {
            BrowserError::Other(anyhow::anyhow!("Failed to write config file: {}", e))
        })?;

        Ok(())
    }

    /// Extract network configuration subset
    ///
    /// Returns a `NetworkConfig` containing only network-related settings.
    pub fn network_config(&self) -> NetworkConfig {
        NetworkConfig {
            max_connections_per_host: self.network.max_connections_per_host,
            timeout_seconds: self.network.timeout_seconds,
            enable_cookies: self.network.enable_cookies,
            enable_cache: self.network.enable_cache,
            cache_size_mb: self.network.cache_size_mb,
        }
    }

    /// Extract adblock configuration subset
    ///
    /// Returns an `AdBlockConfig` containing only adblock-related settings.
    pub fn adblock_config(&self) -> AdBlockConfig {
        AdBlockConfig {
            enabled: self.adblock.enabled,
            update_filters_on_startup: self.adblock.update_filters_on_startup,
            custom_filters: self.adblock.custom_filters.clone(),
        }
    }

    /// Extract shell configuration subset
    ///
    /// Returns a `ShellConfig` containing settings relevant to the shell/UI.
    pub fn shell_config(&self) -> ShellConfig {
        ShellConfig {
            homepage: self.browser.homepage.clone(),
            enable_devtools: self.browser.enable_devtools,
            theme: self.appearance.theme.clone(),
            default_zoom: self.appearance.default_zoom,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    // ========================================
    // RED PHASE: Tests for Default Configuration
    // ========================================

    #[test]
    fn test_config_default_browser_settings() {
        let config = Config::default();
        assert_eq!(config.browser.homepage, "https://www.google.com");
        assert!(config.browser.enable_devtools);
        assert_eq!(config.browser.default_search_engine, "google");
    }

    #[test]
    fn test_config_default_network_settings() {
        let config = Config::default();
        assert_eq!(config.network.max_connections_per_host, 6);
        assert_eq!(config.network.timeout_seconds, 30);
        assert!(config.network.enable_cookies);
        assert!(config.network.enable_cache);
        assert_eq!(config.network.cache_size_mb, 500);
    }

    #[test]
    fn test_config_default_adblock_settings() {
        let config = Config::default();
        assert!(config.adblock.enabled);
        assert!(!config.adblock.update_filters_on_startup);
        assert!(config.adblock.custom_filters.is_empty());
    }

    #[test]
    fn test_config_default_privacy_settings() {
        let config = Config::default();
        assert!(config.privacy.do_not_track);
        assert!(!config.privacy.clear_cookies_on_exit);
        assert!(!config.privacy.block_third_party_cookies);
    }

    #[test]
    fn test_config_default_appearance_settings() {
        let config = Config::default();
        assert_eq!(config.appearance.theme, "auto");
        assert_eq!(config.appearance.default_zoom, 1.0);
    }

    // ========================================
    // RED PHASE: Tests for Loading/Saving
    // ========================================

    #[test]
    fn test_save_and_load_config() {
        let config = Config::default();
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();

        // Save config
        config.save_to_file(path).unwrap();

        // Load config
        let loaded_config = Config::load_from_file(path).unwrap();

        assert_eq!(config, loaded_config);
    }

    #[test]
    fn test_save_creates_parent_directory() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config_path = temp_dir.path().join("subdir/config.toml");

        let config = Config::default();
        config.save_to_file(&config_path).unwrap();

        assert!(config_path.exists());
        assert!(config_path.parent().unwrap().exists());
    }

    #[test]
    fn test_load_from_nonexistent_file_returns_error() {
        let result = Config::load_from_file(Path::new("/nonexistent/path/config.toml"));
        assert!(result.is_err());
    }

    #[test]
    fn test_load_from_invalid_toml_returns_error() {
        let temp_file = NamedTempFile::new().unwrap();
        std::fs::write(temp_file.path(), "invalid toml content {{{").unwrap();

        let result = Config::load_from_file(temp_file.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_load_custom_config_values() {
        let custom_toml = r#"
[browser]
homepage = "https://example.com"
enable_devtools = false
default_search_engine = "duckduckgo"

[network]
max_connections_per_host = 10
timeout_seconds = 60
enable_cookies = false
enable_cache = false
cache_size_mb = 1000

[adblock]
enabled = false
update_filters_on_startup = true
custom_filters = ["rule1", "rule2"]

[privacy]
do_not_track = false
clear_cookies_on_exit = true
block_third_party_cookies = true

[appearance]
theme = "dark"
default_zoom = 1.5
"#;

        let temp_file = NamedTempFile::new().unwrap();
        std::fs::write(temp_file.path(), custom_toml).unwrap();

        let config = Config::load_from_file(temp_file.path()).unwrap();

        // Verify browser settings
        assert_eq!(config.browser.homepage, "https://example.com");
        assert!(!config.browser.enable_devtools);
        assert_eq!(config.browser.default_search_engine, "duckduckgo");

        // Verify network settings
        assert_eq!(config.network.max_connections_per_host, 10);
        assert_eq!(config.network.timeout_seconds, 60);
        assert!(!config.network.enable_cookies);
        assert!(!config.network.enable_cache);
        assert_eq!(config.network.cache_size_mb, 1000);

        // Verify adblock settings
        assert!(!config.adblock.enabled);
        assert!(config.adblock.update_filters_on_startup);
        assert_eq!(config.adblock.custom_filters, vec!["rule1", "rule2"]);

        // Verify privacy settings
        assert!(!config.privacy.do_not_track);
        assert!(config.privacy.clear_cookies_on_exit);
        assert!(config.privacy.block_third_party_cookies);

        // Verify appearance settings
        assert_eq!(config.appearance.theme, "dark");
        assert_eq!(config.appearance.default_zoom, 1.5);
    }

    #[test]
    fn test_saved_config_is_valid_toml() {
        let config = Config::default();
        let temp_file = NamedTempFile::new().unwrap();

        config.save_to_file(temp_file.path()).unwrap();

        let content = std::fs::read_to_string(temp_file.path()).unwrap();
        let parsed: toml::Value = toml::from_str(&content).unwrap();

        assert!(parsed.get("browser").is_some());
        assert!(parsed.get("network").is_some());
        assert!(parsed.get("adblock").is_some());
        assert!(parsed.get("privacy").is_some());
        assert!(parsed.get("appearance").is_some());
    }

    // ========================================
    // RED PHASE: Tests for Sub-Config Extraction
    // ========================================

    #[test]
    fn test_network_config_extraction() {
        let config = Config::default();
        let network_config = config.network_config();

        assert_eq!(network_config.max_connections_per_host, 6);
        assert_eq!(network_config.timeout_seconds, 30);
        assert!(network_config.enable_cookies);
        assert!(network_config.enable_cache);
        assert_eq!(network_config.cache_size_mb, 500);
    }

    #[test]
    fn test_adblock_config_extraction() {
        let config = Config::default();
        let adblock_config = config.adblock_config();

        assert!(adblock_config.enabled);
        assert!(!adblock_config.update_filters_on_startup);
        assert!(adblock_config.custom_filters.is_empty());
    }

    #[test]
    fn test_shell_config_extraction() {
        let config = Config::default();
        let shell_config = config.shell_config();

        assert_eq!(shell_config.homepage, "https://www.google.com");
        assert!(shell_config.enable_devtools);
        assert_eq!(shell_config.theme, "auto");
        assert_eq!(shell_config.default_zoom, 1.0);
    }

    #[test]
    fn test_sub_config_reflects_main_config_changes() {
        let mut config = Config::default();
        config.network.max_connections_per_host = 20;
        config.adblock.enabled = false;
        config.appearance.theme = "dark".to_string();

        let network_config = config.network_config();
        let adblock_config = config.adblock_config();
        let shell_config = config.shell_config();

        assert_eq!(network_config.max_connections_per_host, 20);
        assert!(!adblock_config.enabled);
        assert_eq!(shell_config.theme, "dark");
    }

    // ========================================
    // RED PHASE: Tests for Clone and Debug
    // ========================================

    #[test]
    fn test_config_clone() {
        let config = Config::default();
        let cloned = config.clone();
        assert_eq!(config, cloned);
    }

    #[test]
    fn test_config_debug() {
        let config = Config::default();
        let debug_str = format!("{:?}", config);
        assert!(debug_str.contains("Config"));
        assert!(debug_str.contains("browser"));
    }

    #[test]
    fn test_network_config_clone() {
        let config = Config::default();
        let network_config = config.network_config();
        let cloned = network_config.clone();
        assert_eq!(network_config, cloned);
    }

    #[test]
    fn test_adblock_config_clone() {
        let config = Config::default();
        let adblock_config = config.adblock_config();
        let cloned = adblock_config.clone();
        assert_eq!(adblock_config, cloned);
    }

    #[test]
    fn test_shell_config_clone() {
        let config = Config::default();
        let shell_config = config.shell_config();
        let cloned = shell_config.clone();
        assert_eq!(shell_config, cloned);
    }

    // ========================================
    // RED PHASE: Tests for Serialization
    // ========================================

    #[test]
    fn test_config_serialization() {
        let config = Config::default();
        let toml_str = toml::to_string(&config).unwrap();
        assert!(toml_str.contains("[browser]"));
        assert!(toml_str.contains("[network]"));
        assert!(toml_str.contains("[adblock]"));
        assert!(toml_str.contains("[privacy]"));
        assert!(toml_str.contains("[appearance]"));
    }

    #[test]
    fn test_config_deserialization() {
        let toml_str = toml::to_string(&Config::default()).unwrap();
        let config: Config = toml::from_str(&toml_str).unwrap();
        assert_eq!(config, Config::default());
    }

    #[test]
    fn test_roundtrip_serialization() {
        let original = Config::default();
        let toml_str = toml::to_string(&original).unwrap();
        let deserialized: Config = toml::from_str(&toml_str).unwrap();
        assert_eq!(original, deserialized);
    }

    // ========================================
    // Additional Tests for Edge Cases
    // ========================================

    #[test]
    fn test_empty_custom_filters() {
        let config = Config::default();
        assert!(config.adblock.custom_filters.is_empty());
        let adblock_config = config.adblock_config();
        assert!(adblock_config.custom_filters.is_empty());
    }

    #[test]
    fn test_custom_filters_with_values() {
        let mut config = Config::default();
        config.adblock.custom_filters = vec!["filter1".to_string(), "filter2".to_string()];

        let adblock_config = config.adblock_config();
        assert_eq!(adblock_config.custom_filters.len(), 2);
        assert_eq!(adblock_config.custom_filters[0], "filter1");
        assert_eq!(adblock_config.custom_filters[1], "filter2");
    }

    #[test]
    fn test_theme_values() {
        let mut config = Config::default();

        // Test default
        assert_eq!(config.appearance.theme, "auto");

        // Test light theme
        config.appearance.theme = "light".to_string();
        assert_eq!(config.shell_config().theme, "light");

        // Test dark theme
        config.appearance.theme = "dark".to_string();
        assert_eq!(config.shell_config().theme, "dark");
    }

    #[test]
    fn test_zoom_level_values() {
        let mut config = Config::default();

        // Test default zoom
        assert_eq!(config.appearance.default_zoom, 1.0);

        // Test different zoom levels
        config.appearance.default_zoom = 0.5;
        assert_eq!(config.shell_config().default_zoom, 0.5);

        config.appearance.default_zoom = 2.0;
        assert_eq!(config.shell_config().default_zoom, 2.0);
    }
}
