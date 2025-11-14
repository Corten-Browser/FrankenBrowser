//! End-to-end workflow integration tests
//!
//! These tests verify complete workflows across multiple components
//! using REAL components (no mocking).

mod common;

use common::{setup_message_bus, wait_for_processing, MessageCollector};
use shared_types::{BrowserMessage, ResourceType};
use url::Url;

#[test]
fn test_complete_navigation_workflow() {
    // Test complete navigation flow through message bus
    let mut bus = setup_message_bus();
    let collector = MessageCollector::new();

    bus.register_handler(Box::new(collector.clone()));

    let sender = bus.sender();
    let tab_id = 1;
    let url = Url::parse("https://example.com").unwrap();

    // Step 1: Send navigate request
    sender
        .send(BrowserMessage::NavigateRequest {
            tab_id,
            url: url.clone(),
        })
        .unwrap();

    // Step 2: Simulate navigate response
    sender
        .send(BrowserMessage::NavigateResponse {
            tab_id,
            content: b"<!DOCTYPE html><html>...</html>".to_vec(),
        })
        .unwrap();

    wait_for_processing();
    bus.shutdown().unwrap();

    // Verify workflow messages
    let messages = collector.get_messages();
    assert_eq!(messages.len(), 2, "Should have 2 messages in workflow");

    match &messages[0] {
        BrowserMessage::NavigateRequest { tab_id: tid, url: u } => {
            assert_eq!(*tid, tab_id);
            assert_eq!(u.as_str(), "https://example.com/");
        }
        _ => panic!("First message should be NavigateRequest"),
    }

    match &messages[1] {
        BrowserMessage::NavigateResponse { tab_id: tid, content } => {
            assert_eq!(*tid, tab_id);
            assert!(!content.is_empty());
        }
        _ => panic!("Second message should be NavigateResponse"),
    }
}

#[test]
fn test_tab_management_workflow() {
    let mut bus = setup_message_bus();
    let collector = MessageCollector::new();

    bus.register_handler(Box::new(collector.clone()));

    let sender = bus.sender();

    // Workflow: Create tab -> Switch to it -> Navigate -> Close
    sender.send(BrowserMessage::CreateTab { parent_window: 1 }).unwrap();
    sender.send(BrowserMessage::SwitchTab { tab_id: 2 }).unwrap();

    let url = Url::parse("https://example.com").unwrap();
    sender
        .send(BrowserMessage::NavigateRequest { tab_id: 2, url })
        .unwrap();

    sender.send(BrowserMessage::CloseTab { tab_id: 2 }).unwrap();

    wait_for_processing();
    bus.shutdown().unwrap();

    let messages = collector.get_messages();
    assert_eq!(messages.len(), 4, "Should have 4 messages in workflow");

    // Verify message sequence
    assert!(matches!(messages[0], BrowserMessage::CreateTab { .. }));
    assert!(matches!(messages[1], BrowserMessage::SwitchTab { .. }));
    assert!(matches!(messages[2], BrowserMessage::NavigateRequest { .. }));
    assert!(matches!(messages[3], BrowserMessage::CloseTab { .. }));
}

#[test]
fn test_adblock_workflow() {
    let mut bus = setup_message_bus();
    let collector = MessageCollector::new();

    bus.register_handler(Box::new(collector.clone()));

    let sender = bus.sender();

    // Workflow: Request resource -> Check if should block -> Decision
    let ad_url = Url::parse("https://ads.example.com/banner.js").unwrap();

    sender
        .send(BrowserMessage::ShouldBlock {
            url: ad_url.clone(),
            resource_type: ResourceType::Script,
        })
        .unwrap();

    sender
        .send(BrowserMessage::BlockDecision {
            block: true,
            reason: Some("Matched filter: ||ads.example.com^".to_string()),
        })
        .unwrap();

    wait_for_processing();
    bus.shutdown().unwrap();

    let messages = collector.get_messages();
    assert_eq!(messages.len(), 2);

    match &messages[0] {
        BrowserMessage::ShouldBlock { url, resource_type } => {
            assert_eq!(url.as_str(), "https://ads.example.com/banner.js");
            assert!(matches!(resource_type, ResourceType::Script));
        }
        _ => panic!("Expected ShouldBlock"),
    }

    match &messages[1] {
        BrowserMessage::BlockDecision { block, reason } => {
            assert!(block);
            assert!(reason.is_some());
        }
        _ => panic!("Expected BlockDecision"),
    }
}

#[test]
fn test_navigation_with_error_workflow() {
    let mut bus = setup_message_bus();
    let collector = MessageCollector::new();

    bus.register_handler(Box::new(collector.clone()));

    let sender = bus.sender();
    let tab_id = 1;
    let url = Url::parse("https://invalid-domain-12345.com").unwrap();

    // Step 1: Navigate request
    sender
        .send(BrowserMessage::NavigateRequest {
            tab_id,
            url: url.clone(),
        })
        .unwrap();

    // Step 2: Error response
    sender
        .send(BrowserMessage::NavigateError {
            tab_id,
            error: "DNS resolution failed".to_string(),
        })
        .unwrap();

    wait_for_processing();
    bus.shutdown().unwrap();

    let messages = collector.get_messages();
    assert_eq!(messages.len(), 2);

    assert!(matches!(messages[0], BrowserMessage::NavigateRequest { .. }));

    match &messages[1] {
        BrowserMessage::NavigateError { tab_id: tid, error } => {
            assert_eq!(*tid, tab_id);
            assert!(error.contains("DNS") || error.contains("failed"));
        }
        _ => panic!("Expected NavigateError"),
    }
}

#[test]
fn test_browser_history_workflow() {
    let mut bus = setup_message_bus();
    let collector = MessageCollector::new();

    bus.register_handler(Box::new(collector.clone()));

    let sender = bus.sender();
    let tab_id = 1;

    // Navigate to page 1
    let url1 = Url::parse("https://page1.com").unwrap();
    sender
        .send(BrowserMessage::NavigateRequest {
            tab_id,
            url: url1,
        })
        .unwrap();

    // Navigate to page 2
    let url2 = Url::parse("https://page2.com").unwrap();
    sender
        .send(BrowserMessage::NavigateRequest {
            tab_id,
            url: url2,
        })
        .unwrap();

    // Go back
    sender.send(BrowserMessage::GoBack { tab_id }).unwrap();

    // Go forward
    sender.send(BrowserMessage::GoForward { tab_id }).unwrap();

    wait_for_processing();
    bus.shutdown().unwrap();

    let messages = collector.get_messages();
    assert_eq!(messages.len(), 4);

    assert!(matches!(messages[0], BrowserMessage::NavigateRequest { .. }));
    assert!(matches!(messages[1], BrowserMessage::NavigateRequest { .. }));
    assert!(matches!(messages[2], BrowserMessage::GoBack { .. }));
    assert!(matches!(messages[3], BrowserMessage::GoForward { .. }));
}

#[test]
fn test_reload_workflow() {
    let mut bus = setup_message_bus();
    let collector = MessageCollector::new();

    bus.register_handler(Box::new(collector.clone()));

    let sender = bus.sender();
    let tab_id = 1;

    // Navigate
    let url = Url::parse("https://example.com").unwrap();
    sender
        .send(BrowserMessage::NavigateRequest {
            tab_id,
            url: url.clone(),
        })
        .unwrap();

    // Reload
    sender.send(BrowserMessage::Reload { tab_id }).unwrap();

    wait_for_processing();
    bus.shutdown().unwrap();

    let messages = collector.get_messages();
    assert_eq!(messages.len(), 2);

    assert!(matches!(messages[0], BrowserMessage::NavigateRequest { .. }));

    match &messages[1] {
        BrowserMessage::Reload { tab_id: tid } => {
            assert_eq!(*tid, tab_id);
        }
        _ => panic!("Expected Reload"),
    }
}

#[test]
fn test_multiple_tabs_workflow() {
    let mut bus = setup_message_bus();
    let collector = MessageCollector::new();

    bus.register_handler(Box::new(collector.clone()));

    let sender = bus.sender();

    // Create 3 tabs
    sender.send(BrowserMessage::CreateTab { parent_window: 1 }).unwrap();
    sender.send(BrowserMessage::CreateTab { parent_window: 1 }).unwrap();
    sender.send(BrowserMessage::CreateTab { parent_window: 1 }).unwrap();

    // Navigate in different tabs
    let url1 = Url::parse("https://tab1.com").unwrap();
    sender
        .send(BrowserMessage::NavigateRequest { tab_id: 1, url: url1 })
        .unwrap();

    let url2 = Url::parse("https://tab2.com").unwrap();
    sender
        .send(BrowserMessage::NavigateRequest { tab_id: 2, url: url2 })
        .unwrap();

    // Switch tabs
    sender.send(BrowserMessage::SwitchTab { tab_id: 2 }).unwrap();
    sender.send(BrowserMessage::SwitchTab { tab_id: 3 }).unwrap();

    // Close middle tab
    sender.send(BrowserMessage::CloseTab { tab_id: 2 }).unwrap();

    wait_for_processing();
    bus.shutdown().unwrap();

    let messages = collector.get_messages();
    assert_eq!(messages.len(), 8);
}

#[test]
fn test_shutdown_workflow() {
    let mut bus = setup_message_bus();
    let collector = MessageCollector::new();

    bus.register_handler(Box::new(collector.clone()));

    let sender = bus.sender();

    // Simulate shutdown: Close all tabs then shutdown
    sender.send(BrowserMessage::CloseTab { tab_id: 1 }).unwrap();
    sender.send(BrowserMessage::CloseTab { tab_id: 2 }).unwrap();
    sender.send(BrowserMessage::Shutdown).unwrap();

    wait_for_processing();
    bus.shutdown().unwrap();

    let messages = collector.get_messages();
    assert_eq!(messages.len(), 3);

    assert!(matches!(messages[2], BrowserMessage::Shutdown));
}
