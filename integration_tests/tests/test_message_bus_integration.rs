//! Integration tests for MessageBus component integration
//!
//! These tests verify that the MessageBus correctly routes messages between
//! components using REAL components (no mocking).

mod common;

use common::{setup_message_bus, wait_for_processing, MessageCollector};
use shared_types::{BrowserMessage, ResourceType};
use url::Url;

#[test]
fn test_message_bus_routes_navigate_request() {
    // REAL components - MessageBus and MessageCollector
    let mut bus = setup_message_bus();
    let collector = MessageCollector::new();

    bus.register_handler(Box::new(collector.clone()));

    let sender = bus.sender();

    // Send NavigateRequest message
    let url = Url::parse("https://example.com").unwrap();
    sender
        .send(BrowserMessage::NavigateRequest { tab_id: 1, url: url.clone() })
        .expect("Failed to send message");

    wait_for_processing();
    bus.shutdown().expect("Failed to shutdown bus");

    // Verify message was routed to handler
    let messages = collector.get_messages();
    assert_eq!(messages.len(), 1);

    match &messages[0] {
        BrowserMessage::NavigateRequest { tab_id, url: msg_url } => {
            assert_eq!(*tab_id, 1);
            assert_eq!(msg_url.as_str(), "https://example.com/");
        }
        _ => panic!("Expected NavigateRequest"),
    }
}

#[test]
fn test_message_bus_routes_should_block_message() {
    let mut bus = setup_message_bus();
    let collector = MessageCollector::new();

    bus.register_handler(Box::new(collector.clone()));

    let sender = bus.sender();

    // Send ShouldBlock message (for ad blocking)
    let url = Url::parse("https://ads.example.com/banner.js").unwrap();
    sender
        .send(BrowserMessage::ShouldBlock {
            url: url.clone(),
            resource_type: ResourceType::Script,
        })
        .expect("Failed to send message");

    wait_for_processing();
    bus.shutdown().expect("Failed to shutdown bus");

    // Verify message was routed
    let messages = collector.get_messages();
    assert_eq!(messages.len(), 1);

    match &messages[0] {
        BrowserMessage::ShouldBlock { url: msg_url, resource_type } => {
            assert_eq!(msg_url.as_str(), "https://ads.example.com/banner.js");
            assert!(matches!(resource_type, ResourceType::Script));
        }
        _ => panic!("Expected ShouldBlock"),
    }
}

#[test]
fn test_message_bus_routes_to_multiple_handlers() {
    let mut bus = setup_message_bus();
    let collector1 = MessageCollector::new();
    let collector2 = MessageCollector::new();

    bus.register_handler(Box::new(collector1.clone()));
    bus.register_handler(Box::new(collector2.clone()));

    let sender = bus.sender();

    // Send message
    sender
        .send(BrowserMessage::CreateTab { parent_window: 1 })
        .expect("Failed to send message");

    wait_for_processing();
    bus.shutdown().expect("Failed to shutdown bus");

    // Both handlers should receive the message
    assert_eq!(collector1.count(), 1);
    assert_eq!(collector2.count(), 1);

    match &collector1.get_messages()[0] {
        BrowserMessage::CreateTab { parent_window } => {
            assert_eq!(*parent_window, 1);
        }
        _ => panic!("Expected CreateTab"),
    }
}

#[test]
fn test_message_bus_handles_tab_management_messages() {
    let mut bus = setup_message_bus();
    let collector = MessageCollector::new();

    bus.register_handler(Box::new(collector.clone()));

    let sender = bus.sender();

    // Send tab management messages
    sender.send(BrowserMessage::CreateTab { parent_window: 1 }).unwrap();
    sender.send(BrowserMessage::SwitchTab { tab_id: 2 }).unwrap();
    sender.send(BrowserMessage::CloseTab { tab_id: 1 }).unwrap();

    wait_for_processing();
    bus.shutdown().expect("Failed to shutdown bus");

    let messages = collector.get_messages();
    assert_eq!(messages.len(), 3);

    // Verify message types
    assert!(matches!(messages[0], BrowserMessage::CreateTab { .. }));
    assert!(matches!(messages[1], BrowserMessage::SwitchTab { .. }));
    assert!(matches!(messages[2], BrowserMessage::CloseTab { .. }));
}

#[test]
fn test_message_bus_handles_navigation_workflow() {
    let mut bus = setup_message_bus();
    let collector = MessageCollector::new();

    bus.register_handler(Box::new(collector.clone()));

    let sender = bus.sender();

    // Simulate navigation workflow
    let url = Url::parse("https://example.com").unwrap();

    // Step 1: Navigate request
    sender.send(BrowserMessage::NavigateRequest {
        tab_id: 1,
        url: url.clone()
    }).unwrap();

    // Step 2: Navigate response (simulated)
    sender.send(BrowserMessage::NavigateResponse {
        tab_id: 1,
        content: vec![72, 101, 108, 108, 111], // "Hello"
    }).unwrap();

    wait_for_processing();
    bus.shutdown().expect("Failed to shutdown bus");

    let messages = collector.get_messages();
    assert_eq!(messages.len(), 2);

    match &messages[0] {
        BrowserMessage::NavigateRequest { tab_id, .. } => {
            assert_eq!(*tab_id, 1);
        }
        _ => panic!("Expected NavigateRequest"),
    }

    match &messages[1] {
        BrowserMessage::NavigateResponse { tab_id, content } => {
            assert_eq!(*tab_id, 1);
            assert_eq!(content, &vec![72, 101, 108, 108, 111]);
        }
        _ => panic!("Expected NavigateResponse"),
    }
}

#[test]
fn test_message_bus_concurrent_message_sending() {
    use std::thread;

    let mut bus = setup_message_bus();
    let collector = MessageCollector::new();

    bus.register_handler(Box::new(collector.clone()));

    let sender1 = bus.sender();
    let sender2 = bus.sender();

    // Spawn threads sending messages concurrently
    let handle1 = thread::spawn(move || {
        for i in 0..5 {
            sender1
                .send(BrowserMessage::CreateTab { parent_window: i })
                .unwrap();
        }
    });

    let handle2 = thread::spawn(move || {
        for i in 5..10 {
            sender2
                .send(BrowserMessage::CreateTab { parent_window: i })
                .unwrap();
        }
    });

    handle1.join().unwrap();
    handle2.join().unwrap();

    wait_for_processing();
    bus.shutdown().expect("Failed to shutdown bus");

    // All 10 messages should be received
    assert_eq!(collector.count(), 10);
}

#[test]
fn test_message_bus_shutdown_message() {
    let mut bus = setup_message_bus();
    let collector = MessageCollector::new();

    bus.register_handler(Box::new(collector.clone()));

    let sender = bus.sender();

    // Send shutdown message
    sender.send(BrowserMessage::Shutdown).unwrap();

    wait_for_processing();
    bus.shutdown().expect("Failed to shutdown bus");

    let messages = collector.get_messages();
    assert_eq!(messages.len(), 1);
    assert!(matches!(messages[0], BrowserMessage::Shutdown));
}
