/// Configuration Validation Tests
///
/// Validates that configuration system works correctly with various inputs.

use config_manager::Config;
use std::fs;
use std::path::Path;

#[test]
fn test_default_config_values() {
    let config = Config::default();

    // Browser defaults
    assert!(!config.browser.homepage.is_empty());
    assert!(config.browser.homepage.starts_with("http"));
    assert!(!config.browser.default_search_engine.is_empty());

    // Network defaults
    assert!(config.network.max_connections_per_host > 0);
    assert!(config.network.timeout_seconds > 0);
    assert!(config.network.cache_size_mb > 0);

    // Adblock defaults - either enabled or disabled, both valid
    let _ = config.adblock.enabled;

    // Privacy defaults
    let _ = config.privacy.do_not_track;

    // Appearance defaults
    assert!(!config.appearance.theme.is_empty());
    assert!(config.appearance.default_zoom > 0.0);
}

#[test]
fn test_config_serialization() {
    let config = Config::default();

    // Should serialize to TOML
    let toml_str = toml::to_string(&config);
    assert!(toml_str.is_ok());

    let toml_content = toml_str.unwrap();
    assert!(toml_content.contains("[browser]"));
    assert!(toml_content.contains("[network]"));
    assert!(toml_content.contains("[adblock]"));
    assert!(toml_content.contains("[privacy]"));
    assert!(toml_content.contains("[appearance]"));
}

#[test]
fn test_config_deserialization() {
    let toml_content = r#"
[browser]
homepage = "https://duckduckgo.com"
enable_devtools = false
default_search_engine = "duckduckgo"

[network]
max_connections_per_host = 10
timeout_seconds = 60
enable_cookies = true
enable_cache = true
cache_size_mb = 1000

[adblock]
enabled = true
update_filters_on_startup = true
custom_filters = ["filter1.txt", "filter2.txt"]

[privacy]
do_not_track = true
clear_cookies_on_exit = true
block_third_party_cookies = true

[appearance]
theme = "dark"
default_zoom = 1.25
"#;

    let config: Result<Config, _> = toml::from_str(toml_content);
    assert!(config.is_ok());

    let config = config.unwrap();
    assert_eq!(config.browser.homepage, "https://duckduckgo.com");
    assert!(!config.browser.enable_devtools);
    assert_eq!(config.network.max_connections_per_host, 10);
    assert_eq!(config.network.timeout_seconds, 60);
    assert!(config.adblock.enabled);
    assert_eq!(config.adblock.custom_filters.len(), 2);
    assert_eq!(config.appearance.theme, "dark");
    assert_eq!(config.appearance.default_zoom, 1.25);
}

#[test]
fn test_config_round_trip() {
    let config1 = Config::default();

    // Serialize
    let toml_str = toml::to_string(&config1).unwrap();

    // Deserialize
    let config2: Config = toml::from_str(&toml_str).unwrap();

    // Should be equivalent
    assert_eq!(config1.browser.homepage, config2.browser.homepage);
    assert_eq!(config1.network.timeout_seconds, config2.network.timeout_seconds);
    assert_eq!(config1.adblock.enabled, config2.adblock.enabled);
}

#[test]
fn test_invalid_config_handling() {
    let invalid_configs = vec![
        // Missing required field
        r#"[browser]"#,
        // Invalid TOML syntax
        r#"[browser
        homepage = "test"#,
        // Invalid value type
        r#"[network]
        timeout_seconds = "not_a_number""#,
    ];

    for invalid in invalid_configs {
        let result: Result<Config, _> = toml::from_str(invalid);
        assert!(result.is_err(), "Should reject invalid config: {}", invalid);
    }
}

#[test]
fn test_config_validation_rules() {
    // Test that config values are within reasonable bounds
    let config = Config::default();

    // Network timeouts should be positive
    assert!(config.network.timeout_seconds > 0);
    assert!(config.network.timeout_seconds < 300); // Less than 5 minutes

    // Max connections should be reasonable
    assert!(config.network.max_connections_per_host > 0);
    assert!(config.network.max_connections_per_host <= 20);

    // Cache size should be reasonable
    assert!(config.network.cache_size_mb > 0);
    assert!(config.network.cache_size_mb <= 10000); // Less than 10GB

    // Zoom should be reasonable
    assert!(config.appearance.default_zoom > 0.1);
    assert!(config.appearance.default_zoom < 5.0);
}

#[test]
fn test_config_partial_updates() {
    // Test that partial config updates work
    let mut config = Config::default();

    // Modify specific fields
    config.browser.homepage = "https://example.com".to_string();
    config.network.timeout_seconds = 45;

    // Serialize
    let toml_str = toml::to_string(&config).unwrap();

    // Deserialize
    let updated: Config = toml::from_str(&toml_str).unwrap();

    // Updated fields should persist
    assert_eq!(updated.browser.homepage, "https://example.com");
    assert_eq!(updated.network.timeout_seconds, 45);

    // Other fields should remain default
    assert_eq!(updated.adblock.enabled, config.adblock.enabled);
}

#[test]
fn test_config_file_operations() {
    use std::io::Write;
    use tempfile::NamedTempFile;

    // Create temp file
    let mut temp_file = NamedTempFile::new().unwrap();

    // Write config
    let config = Config::default();
    let toml_str = toml::to_string(&config).unwrap();
    temp_file.write_all(toml_str.as_bytes()).unwrap();
    temp_file.flush().unwrap();

    // Read config back
    let content = fs::read_to_string(temp_file.path()).unwrap();
    let loaded: Config = toml::from_str(&content).unwrap();

    assert_eq!(config.browser.homepage, loaded.browser.homepage);
}

#[test]
fn test_config_theme_values() {
    // Test valid theme values
    let valid_themes = vec!["auto", "light", "dark"];

    for theme in valid_themes {
        let toml = format!(
            r#"
[browser]
homepage = "https://example.com"
enable_devtools = true
default_search_engine = "google"

[network]
max_connections_per_host = 6
timeout_seconds = 30
enable_cookies = true
enable_cache = true
cache_size_mb = 500

[adblock]
enabled = true
update_filters_on_startup = false
custom_filters = []

[privacy]
do_not_track = true
clear_cookies_on_exit = false
block_third_party_cookies = false

[appearance]
theme = "{}"
default_zoom = 1.0
"#,
            theme
        );

        let config: Result<Config, _> = toml::from_str(&toml);
        assert!(config.is_ok());
        assert_eq!(config.unwrap().appearance.theme, theme);
    }
}

#[test]
fn test_config_custom_filters() {
    let toml = r#"
[browser]
homepage = "https://example.com"
enable_devtools = true
default_search_engine = "google"

[network]
max_connections_per_host = 6
timeout_seconds = 30
enable_cookies = true
enable_cache = true
cache_size_mb = 500

[adblock]
enabled = true
update_filters_on_startup = false
custom_filters = ["filter1.txt", "filter2.txt", "filter3.txt"]

[privacy]
do_not_track = true
clear_cookies_on_exit = false
block_third_party_cookies = false

[appearance]
theme = "auto"
default_zoom = 1.0
"#;

    let config: Config = toml::from_str(toml).unwrap();
    assert_eq!(config.adblock.custom_filters.len(), 3);
    assert_eq!(config.adblock.custom_filters[0], "filter1.txt");
}
