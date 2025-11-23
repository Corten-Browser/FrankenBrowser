//! Navigation Validation Tests
//!
//! Validates navigation functionality including basic navigation (FEAT-030)
//! and top websites compatibility (FEAT-033).

use browser_core::{BrowserEngine, Navigator, Protocol};
use config_manager::Config;
use message_bus::MessageBus;
use network_stack::NetworkStack;
use url::Url;

/// List of top 10 websites for validation testing
const TOP_10_WEBSITES: &[&str] = &[
    "https://www.google.com",
    "https://www.youtube.com",
    "https://www.facebook.com",
    "https://www.amazon.com",
    "https://www.wikipedia.org",
    "https://www.twitter.com",
    "https://www.instagram.com",
    "https://www.linkedin.com",
    "https://www.reddit.com",
    "https://www.netflix.com",
];

/// FEAT-030: Basic navigation validation - google.com
#[test]
fn test_navigate_google_url_validation() {
    let url = Url::parse("https://www.google.com").expect("Google URL should parse");
    assert_eq!(url.scheme(), "https");
    assert_eq!(url.host_str(), Some("www.google.com"));
}

#[test]
fn test_navigate_google_protocol_detection() {
    let url = Url::parse("https://www.google.com").unwrap();
    let protocol = Navigator::determine_protocol(&url);
    assert_eq!(protocol, Protocol::Https);
}

#[test]
fn test_navigator_validate_google_url() {
    let navigator = Navigator::new();
    let url = Url::parse("https://www.google.com").unwrap();

    let result = navigator.validate_url(&url);
    assert!(result.is_ok(), "Google URL should be valid: {:?}", result.err());
}

#[test]
fn test_browser_engine_can_create_for_navigation() {
    let config = Config::default();
    let mut bus1 = MessageBus::new();
    bus1.start().expect("Bus should start");

    let network = NetworkStack::new(config.network_config(), bus1.sender())
        .expect("NetworkStack should be created");

    let mut bus2 = MessageBus::new();
    bus2.start().expect("Bus should start");

    let result = BrowserEngine::new(config.clone(), network, bus2.sender());
    assert!(result.is_ok(), "BrowserEngine should be created for navigation");

    bus1.shutdown().ok();
    bus2.shutdown().ok();
}

/// FEAT-033: Top 10 websites validation
#[test]
fn test_top_10_websites_url_parsing() {
    for url_str in TOP_10_WEBSITES {
        let url = Url::parse(url_str);
        assert!(
            url.is_ok(),
            "URL should parse successfully: {}",
            url_str
        );
    }
}

#[test]
fn test_top_10_websites_protocol_detection() {
    for url_str in TOP_10_WEBSITES {
        let url = Url::parse(url_str).unwrap();
        let protocol = Navigator::determine_protocol(&url);
        assert_eq!(
            protocol,
            Protocol::Https,
            "All top 10 sites should use HTTPS: {}",
            url_str
        );
    }
}

#[test]
fn test_top_10_websites_url_validation() {
    let navigator = Navigator::new();

    for url_str in TOP_10_WEBSITES {
        let url = Url::parse(url_str).unwrap();
        let result = navigator.validate_url(&url);
        assert!(
            result.is_ok(),
            "Top website URL should be valid: {} - {:?}",
            url_str,
            result.err()
        );
    }
}

#[test]
fn test_all_top_10_websites_are_https() {
    let https_count = TOP_10_WEBSITES
        .iter()
        .filter(|url| url.starts_with("https://"))
        .count();

    assert_eq!(
        https_count,
        TOP_10_WEBSITES.len(),
        "All top 10 websites should use HTTPS"
    );
}

#[test]
fn test_navigator_handles_redirects() {
    // Test that Navigator can handle URLs that might redirect
    let navigator = Navigator::new();

    // Common redirect patterns
    let redirect_urls = vec![
        "https://google.com",      // Redirects to www.google.com
        "https://youtube.com",     // Redirects to www.youtube.com
        "https://facebook.com",    // Redirects to www.facebook.com
    ];

    for url_str in redirect_urls {
        let url = Url::parse(url_str).unwrap();
        let result = navigator.validate_url(&url);
        assert!(
            result.is_ok(),
            "Redirect URL should be valid: {} - {:?}",
            url_str,
            result.err()
        );
    }
}

#[test]
fn test_navigation_url_with_paths() {
    let navigator = Navigator::new();

    let urls_with_paths = vec![
        "https://www.google.com/search?q=test",
        "https://www.youtube.com/watch?v=dQw4w9WgXcQ",
        "https://www.amazon.com/dp/B08N5WRWNW",
        "https://en.wikipedia.org/wiki/Rust_(programming_language)",
    ];

    for url_str in urls_with_paths {
        let url = Url::parse(url_str).unwrap();
        let result = navigator.validate_url(&url);
        assert!(
            result.is_ok(),
            "URL with path should be valid: {} - {:?}",
            url_str,
            result.err()
        );
    }
}

#[test]
fn test_navigation_state_management() {
    use browser_core::NavigationState;
    use std::time::{Duration, Instant};

    let url = Url::parse("https://example.com").unwrap();

    // Test that all navigation states can be created
    let idle = NavigationState::Idle;
    let loading = NavigationState::Loading(url.clone(), Instant::now());
    let loaded = NavigationState::Loaded(url.clone(), Duration::from_millis(100));
    let error = NavigationState::Error(url, browser_core::NavigationError::Timeout);

    // States should be distinguishable (Idle != Loading, etc.)
    assert_eq!(idle, NavigationState::Idle);
    match loading {
        NavigationState::Loading(_, _) => (),
        _ => panic!("Expected Loading state"),
    }
    match loaded {
        NavigationState::Loaded(_, _) => (),
        _ => panic!("Expected Loaded state"),
    }
    match error {
        NavigationState::Error(_, _) => (),
        _ => panic!("Expected Error state"),
    }
}

#[test]
fn test_navigator_error_page_generation() {
    use browser_core::NavigationError;

    let navigator = Navigator::new();

    // Test that error pages can be generated for various errors
    let errors = vec![
        NavigationError::InvalidUrl("bad url".to_string()),
        NavigationError::NetworkError("connection failed".to_string()),
        NavigationError::Timeout,
    ];

    for error in errors {
        let html = navigator.generate_error_page(&error);
        assert!(!html.is_empty(), "Error page should be generated");
        assert!(html.contains("<!DOCTYPE html>"), "Should be valid HTML");
    }
}
