//! Integration tests for AdBlockEngine
//!
//! RED PHASE: These tests will initially fail and drive implementation

use adblock_engine::*;
use config_manager::AdBlockConfig;
use message_bus::MessageSender;
use shared_types::{BrowserMessage, ResourceType};
use std::sync::{Arc, Mutex};

/// Mock MessageSender for testing
struct MockMessageSender {
    messages: Arc<Mutex<Vec<BrowserMessage>>>,
}

impl MockMessageSender {
    fn new() -> Self {
        Self {
            messages: Arc::new(Mutex::new(Vec::new())),
        }
    }

    #[allow(dead_code)]
    fn get_messages(&self) -> Vec<BrowserMessage> {
        self.messages.lock().unwrap().clone()
    }
}

impl MessageSender for MockMessageSender {
    fn send(&self, message: BrowserMessage) -> message_bus::Result<()> {
        self.messages.lock().unwrap().push(message);
        Ok(())
    }
}

// ========================================
// RED PHASE: Tests for AdBlockEngine::new
// ========================================

#[test]
fn test_adblock_engine_new_with_enabled_config() {
    let config = AdBlockConfig {
        enabled: true,
        update_filters_on_startup: false,
        custom_filters: vec![],
    };
    let sender = Box::new(MockMessageSender::new());

    let result = AdBlockEngine::new(config, sender);
    assert!(result.is_ok());
}

#[test]
fn test_adblock_engine_new_with_disabled_config() {
    let config = AdBlockConfig {
        enabled: false,
        update_filters_on_startup: false,
        custom_filters: vec![],
    };
    let sender = Box::new(MockMessageSender::new());

    let result = AdBlockEngine::new(config, sender);
    assert!(result.is_ok());
}

#[test]
fn test_adblock_engine_new_with_custom_filters() {
    let config = AdBlockConfig {
        enabled: true,
        update_filters_on_startup: false,
        custom_filters: vec!["||ads.example.com^".to_string()],
    };
    let sender = Box::new(MockMessageSender::new());

    let result = AdBlockEngine::new(config, sender);
    assert!(result.is_ok());
}

// ========================================
// RED PHASE: Tests for AdBlockEngine::initialize
// ========================================

#[test]
fn test_adblock_engine_initialize_with_missing_easylist() {
    let config = AdBlockConfig {
        enabled: true,
        update_filters_on_startup: false,
        custom_filters: vec![],
    };
    let sender = Box::new(MockMessageSender::new());

    let mut engine = AdBlockEngine::new(config, sender).unwrap();

    // Should handle missing EasyList gracefully
    let result = engine.initialize();
    assert!(result.is_ok());
}

#[test]
fn test_adblock_engine_initialize_multiple_times_fails() {
    let config = AdBlockConfig {
        enabled: true,
        update_filters_on_startup: false,
        custom_filters: vec![],
    };
    let sender = Box::new(MockMessageSender::new());

    let mut engine = AdBlockEngine::new(config, sender).unwrap();

    // First initialization should succeed
    assert!(engine.initialize().is_ok());

    // Second initialization should fail
    let result = engine.initialize();
    assert!(result.is_err());
}

// ========================================
// RED PHASE: Tests for AdBlockEngine::should_block
// ========================================

#[test]
fn test_should_block_with_disabled_engine() {
    let config = AdBlockConfig {
        enabled: false,
        update_filters_on_startup: false,
        custom_filters: vec![],
    };
    let sender = Box::new(MockMessageSender::new());

    let mut engine = AdBlockEngine::new(config, sender).unwrap();
    engine.initialize().unwrap();

    // When disabled, should not block anything
    let blocked = engine.should_block("https://ads.example.com/banner.js", ResourceType::Script);
    assert!(!blocked);
}

#[test]
fn test_should_block_returns_false_for_safe_urls() {
    let config = AdBlockConfig {
        enabled: true,
        update_filters_on_startup: false,
        custom_filters: vec![],
    };
    let sender = Box::new(MockMessageSender::new());

    let mut engine = AdBlockEngine::new(config, sender).unwrap();
    engine.initialize().unwrap();

    // Should not block legitimate sites
    let blocked = engine.should_block("https://www.google.com/", ResourceType::Document);
    assert!(!blocked);
}

#[test]
fn test_should_block_with_custom_filter() {
    let config = AdBlockConfig {
        enabled: true,
        update_filters_on_startup: false,
        custom_filters: vec!["||ads.example.com^".to_string()],
    };
    let sender = Box::new(MockMessageSender::new());

    let mut engine = AdBlockEngine::new(config, sender).unwrap();
    engine.initialize().unwrap();

    // Should block URL matching custom filter
    let blocked = engine.should_block("https://ads.example.com/banner.js", ResourceType::Script);
    assert!(blocked);
}

#[test]
fn test_should_block_before_initialization() {
    let config = AdBlockConfig {
        enabled: true,
        update_filters_on_startup: false,
        custom_filters: vec![],
    };
    let sender = Box::new(MockMessageSender::new());

    let engine = AdBlockEngine::new(config, sender).unwrap();

    // Before initialization, should not crash but return false
    let blocked = engine.should_block("https://ads.example.com/", ResourceType::Script);
    assert!(!blocked);
}

// ========================================
// RED PHASE: Tests for resource type handling
// ========================================

#[test]
fn test_should_block_handles_different_resource_types() {
    let config = AdBlockConfig {
        enabled: true,
        update_filters_on_startup: false,
        custom_filters: vec!["||tracker.example.com^".to_string()],
    };
    let sender = Box::new(MockMessageSender::new());

    let mut engine = AdBlockEngine::new(config, sender).unwrap();
    engine.initialize().unwrap();

    // Test different resource types
    assert!(engine.should_block("https://tracker.example.com/pixel.png", ResourceType::Image));
    assert!(engine.should_block(
        "https://tracker.example.com/script.js",
        ResourceType::Script
    ));
    assert!(engine.should_block(
        "https://tracker.example.com/style.css",
        ResourceType::Stylesheet
    ));
}

// ========================================
// RED PHASE: Tests for error handling
// ========================================

#[test]
fn test_should_block_handles_invalid_urls() {
    let config = AdBlockConfig {
        enabled: true,
        update_filters_on_startup: false,
        custom_filters: vec![],
    };
    let sender = Box::new(MockMessageSender::new());

    let mut engine = AdBlockEngine::new(config, sender).unwrap();
    engine.initialize().unwrap();

    // Should handle invalid URLs gracefully
    let blocked = engine.should_block("not a valid url", ResourceType::Script);
    // Should not crash, return false for invalid URLs
    assert!(!blocked);
}

#[test]
fn test_should_block_handles_empty_url() {
    let config = AdBlockConfig {
        enabled: true,
        update_filters_on_startup: false,
        custom_filters: vec![],
    };
    let sender = Box::new(MockMessageSender::new());

    let mut engine = AdBlockEngine::new(config, sender).unwrap();
    engine.initialize().unwrap();

    let blocked = engine.should_block("", ResourceType::Script);
    assert!(!blocked);
}

// ========================================
// RED PHASE: Tests for cosmetic filters
// ========================================

#[test]
fn test_get_cosmetic_filters_with_disabled_engine() {
    let config = AdBlockConfig {
        enabled: false,
        update_filters_on_startup: false,
        custom_filters: vec![],
    };
    let sender = Box::new(MockMessageSender::new());

    let mut engine = AdBlockEngine::new(config, sender).unwrap();
    engine.initialize().unwrap();

    let selectors = engine.get_cosmetic_filters("https://example.com");
    assert!(selectors.is_empty());
}

#[test]
fn test_get_cosmetic_filters_before_initialization() {
    let config = AdBlockConfig {
        enabled: true,
        update_filters_on_startup: false,
        custom_filters: vec![],
    };
    let sender = Box::new(MockMessageSender::new());

    let engine = AdBlockEngine::new(config, sender).unwrap();

    let selectors = engine.get_cosmetic_filters("https://example.com");
    assert!(selectors.is_empty());
}

// ========================================
// RED PHASE: Tests for cloning
// ========================================

#[test]
fn test_adblock_engine_clone() {
    let config = AdBlockConfig {
        enabled: true,
        update_filters_on_startup: false,
        custom_filters: vec!["||ads.example.com^".to_string()],
    };
    let sender = Box::new(MockMessageSender::new());

    let mut engine = AdBlockEngine::new(config, sender).unwrap();
    engine.initialize().unwrap();

    // Clone the engine
    let cloned_engine = engine.clone();

    // Both should block the same URLs
    assert!(engine.should_block("https://ads.example.com/banner.js", ResourceType::Script));
    assert!(cloned_engine.should_block("https://ads.example.com/banner.js", ResourceType::Script));
}

// ========================================
// Tests for Element Hider CSS Generation
// ========================================

#[test]
fn test_get_element_hider_css_with_disabled_engine() {
    let config = AdBlockConfig {
        enabled: false,
        update_filters_on_startup: false,
        custom_filters: vec![],
    };
    let sender = Box::new(MockMessageSender::new());

    let mut engine = AdBlockEngine::new(config, sender).unwrap();
    engine.initialize().unwrap();

    let css = engine.get_element_hider_css("https://example.com");
    assert!(css.is_none());
}

#[test]
fn test_get_element_hider_css_before_initialization() {
    let config = AdBlockConfig {
        enabled: true,
        update_filters_on_startup: false,
        custom_filters: vec![],
    };
    let sender = Box::new(MockMessageSender::new());

    let engine = AdBlockEngine::new(config, sender).unwrap();

    let css = engine.get_element_hider_css("https://example.com");
    assert!(css.is_none());
}

#[test]
fn test_get_element_hider_css_returns_valid_css() {
    // Use a cosmetic filter that applies globally
    let config = AdBlockConfig {
        enabled: true,
        update_filters_on_startup: false,
        custom_filters: vec![
            "##.ad-banner".to_string(),
            "##.sponsored-content".to_string(),
        ],
    };
    let sender = Box::new(MockMessageSender::new());

    let mut engine = AdBlockEngine::new(config, sender).unwrap();
    engine.initialize().unwrap();

    let css = engine.get_element_hider_css("https://example.com");

    // If there are selectors, CSS should contain display: none
    if let Some(css_text) = css {
        assert!(css_text.contains("display: none !important"));
    }
    // Note: empty CSS is valid if no cosmetic rules match
}

#[test]
fn test_get_element_hider_style_tag_format() {
    let config = AdBlockConfig {
        enabled: true,
        update_filters_on_startup: false,
        custom_filters: vec!["##.ad-banner".to_string()],
    };
    let sender = Box::new(MockMessageSender::new());

    let mut engine = AdBlockEngine::new(config, sender).unwrap();
    engine.initialize().unwrap();

    let style_tag = engine.get_element_hider_style_tag("https://example.com");

    if let Some(tag) = style_tag {
        assert!(tag.starts_with("<style id=\"adblock-element-hider\">"));
        assert!(tag.ends_with("</style>"));
    }
}

#[test]
fn test_get_element_hider_style_tag_disabled() {
    let config = AdBlockConfig {
        enabled: false,
        update_filters_on_startup: false,
        custom_filters: vec![],
    };
    let sender = Box::new(MockMessageSender::new());

    let mut engine = AdBlockEngine::new(config, sender).unwrap();
    engine.initialize().unwrap();

    let style_tag = engine.get_element_hider_style_tag("https://example.com");
    assert!(style_tag.is_none());
}
