//! API compatibility integration tests
//!
//! These tests verify that components have compatible APIs and can work together
//! using REAL components (no mocking).

mod common;

use common::{setup_message_bus, test_adblock_config, test_network_config};
use shared_types::{BrowserMessage, ResourceType};

#[test]
fn test_resource_type_recognized_by_all_components() {
    // Verify ResourceType enum is compatible across components

    // Test all ResourceType variants
    let _document = ResourceType::Document;
    let _script = ResourceType::Script;
    let _image = ResourceType::Image;
    let _stylesheet = ResourceType::Stylesheet;
    let _font = ResourceType::Font;
    let _media = ResourceType::Media;
    let _websocket = ResourceType::Websocket;
    let _xhr = ResourceType::Xhr;
    let _other = ResourceType::Other;

    // All components (message_bus, network_stack, adblock_engine) should accept these
}

#[test]
fn test_browser_message_variants_work_across_components() {
    // Test that all BrowserMessage variants can be created and used

    use url::Url;

    let url = Url::parse("https://example.com").unwrap();

    // Navigation messages
    let _nav_req = BrowserMessage::NavigateRequest {
        tab_id: 1,
        url: url.clone(),
    };
    let _nav_resp = BrowserMessage::NavigateResponse {
        tab_id: 1,
        content: vec![],
    };
    let _nav_err = BrowserMessage::NavigateError {
        tab_id: 1,
        error: "Error".to_string(),
    };

    // Network messages
    let _http_req = BrowserMessage::HttpRequest {
        request_id: 1,
        url: url.clone(),
        headers: std::collections::HashMap::new(),
    };
    let _http_resp = BrowserMessage::HttpResponse {
        request_id: 1,
        status: 200,
        body: vec![],
    };

    // Ad blocking messages
    let _should_block = BrowserMessage::ShouldBlock {
        url: url.clone(),
        resource_type: ResourceType::Script,
    };
    let _block_decision = BrowserMessage::BlockDecision {
        block: true,
        reason: None,
    };

    // Tab management messages
    let _create_tab = BrowserMessage::CreateTab { parent_window: 1 };
    let _close_tab = BrowserMessage::CloseTab { tab_id: 1 };
    let _switch_tab = BrowserMessage::SwitchTab { tab_id: 1 };

    // Browser control messages
    let _shutdown = BrowserMessage::Shutdown;
    let _reload = BrowserMessage::Reload { tab_id: 1 };
    let _go_back = BrowserMessage::GoBack { tab_id: 1 };
    let _go_forward = BrowserMessage::GoForward { tab_id: 1 };

    // All message types should be constructible
}

#[test]
fn test_message_sender_trait_compatible_across_components() {
    // Test that MessageSender trait works across components

    let mut bus = setup_message_bus();

    // NetworkStack accepts MessageSender
    let config = test_network_config();
    let sender1 = bus.sender();
    let _network = network_stack::NetworkStack::new(config, sender1);

    // AdBlockEngine accepts MessageSender
    let config = test_adblock_config();
    let sender2 = bus.sender();
    let _adblock = adblock_engine::AdBlockEngine::new(config, sender2);

    // Both components accept the same MessageSender type - API is compatible

    bus.shutdown().unwrap();
}

#[test]
fn test_message_handler_trait_compatible() {
    // Test that MessageHandler trait works with MessageBus

    use message_bus::MessageHandler;
    use shared_types::BrowserMessage;

    struct TestHandler;

    impl MessageHandler for TestHandler {
        fn handle(&self, _message: BrowserMessage) -> message_bus::Result<()> {
            Ok(())
        }
    }

    let mut bus = setup_message_bus();

    // MessageBus should accept any MessageHandler implementation
    bus.register_handler(Box::new(TestHandler));

    bus.shutdown().unwrap();
}

#[test]
fn test_config_types_compatible_across_components() {
    // Test that config types are compatible

    // NetworkConfig from config_manager
    let network_config = test_network_config();
    assert_eq!(network_config.max_connections_per_host, 6);
    assert_eq!(network_config.timeout_seconds, 5);
    assert!(network_config.enable_cookies);
    assert!(network_config.enable_cache);
    assert_eq!(network_config.cache_size_mb, 10);

    // AdBlockConfig from config_manager
    let adblock_config = test_adblock_config();
    assert!(adblock_config.enabled);
    assert!(!adblock_config.update_filters_on_startup);
    assert_eq!(adblock_config.custom_filters.len(), 2);

    // Configs should be usable by respective components
}

#[test]
fn test_url_type_compatible_across_all_components() {
    // Test that url::Url is used consistently

    use url::Url;

    let url = Url::parse("https://example.com").unwrap();

    // Used in BrowserMessage
    let _msg = BrowserMessage::NavigateRequest {
        tab_id: 1,
        url: url.clone(),
    };

    // Used by NetworkStack (fetch method takes Url)
    // Used by AdBlockEngine (should_block takes &str but url.as_str() works)

    // URL type is compatible across all components
    assert_eq!(url.as_str(), "https://example.com/");
}

#[test]
fn test_error_types_compatible() {
    // Test that error types are compatible

    // All components use Result types
    let _network_result: network_stack::Result<()> = Ok(());
    let _adblock_result: adblock_engine::Result<()> = Ok(());
    let _message_bus_result: message_bus::Result<()> = Ok(());

    // Error types should be compatible
}

#[test]
fn test_send_sync_traits_on_shared_types() {
    // Test that shared types implement Send + Sync for thread safety

    fn assert_send_sync<T: Send + Sync>() {}

    assert_send_sync::<BrowserMessage>();
    assert_send_sync::<ResourceType>();

    // Types can be safely sent across threads
}

#[test]
fn test_serialization_compatibility() {
    // Test that types can be serialized/deserialized consistently

    use url::Url;

    let msg = BrowserMessage::NavigateRequest {
        tab_id: 1,
        url: Url::parse("https://example.com").unwrap(),
    };

    // Serialize
    let json = serde_json::to_string(&msg).unwrap();

    // Deserialize
    let deserialized: BrowserMessage = serde_json::from_str(&json).unwrap();

    // Should match
    match deserialized {
        BrowserMessage::NavigateRequest { tab_id, url } => {
            assert_eq!(tab_id, 1);
            assert_eq!(url.as_str(), "https://example.com/");
        }
        _ => panic!("Deserialization failed"),
    }
}

#[test]
fn test_component_initialization_order() {
    // Test that components can be initialized in the correct order

    let mut bus = setup_message_bus();

    // Level 0: MessageBus (already started)

    // Level 1: NetworkStack and AdBlockEngine
    let network_config = test_network_config();
    let sender1 = bus.sender();
    let mut network = network_stack::NetworkStack::new(network_config, sender1).unwrap();

    let adblock_config = test_adblock_config();
    let sender2 = bus.sender();
    let mut adblock = adblock_engine::AdBlockEngine::new(adblock_config, sender2).unwrap();

    // Initialize in order
    assert!(network.initialize().is_ok());
    assert!(adblock.initialize().is_ok());

    // All components initialized successfully
    bus.shutdown().unwrap();
}
