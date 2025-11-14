//! Message bus implementation for component communication
//!
//! This is the message_bus component of the FrankenBrowser project.
//!
//! # Component Overview
//!
//! **Type**: library
//! **Level**: 0
//! **Token Budget**: 8000 tokens
//!
//! # Dependencies
//!
//! - shared_types: For BrowserMessage and error types
//! - crossbeam-channel: For multi-producer, multi-consumer channels
//! - tokio: For async runtime support
//!
//! # Usage
//!
//! ```rust
//! use message_bus::{MessageBus, MessageHandler, MessageSender};
//! use shared_types::BrowserMessage;
//!
//! // Create a message bus
//! let mut bus = MessageBus::new();
//!
//! // Register a handler
//! struct MyHandler;
//! impl MessageHandler for MyHandler {
//!     fn handle(&self, message: BrowserMessage) -> message_bus::Result<()> {
//!         println!("Received: {:?}", message);
//!         Ok(())
//!     }
//! }
//!
//! bus.register_handler(Box::new(MyHandler));
//!
//! // Start the bus
//! bus.start().unwrap();
//!
//! // Get a sender and send messages
//! let sender = bus.sender();
//! sender.send(BrowserMessage::Shutdown).unwrap();
//!
//! // Shutdown when done
//! bus.shutdown().unwrap();
//! ```

pub mod errors;
pub mod types;

// Re-export main types for convenience
pub use errors::{Error, Result};
pub use types::{MessageBus, MessageHandler, MessageSender};

#[cfg(test)]
mod tests {
    use super::*;
    use shared_types::BrowserMessage;
    use std::sync::{Arc, Mutex};
    use url::Url;

    // ========================================
    // RED PHASE: Tests for MessageSender trait
    // ========================================

    #[test]
    fn test_message_sender_send_success() {
        let mut bus = MessageBus::new();
        bus.start().unwrap();

        let sender = bus.sender();
        let result = sender.send(BrowserMessage::Shutdown);
        assert!(result.is_ok());

        bus.shutdown().unwrap();
    }

    #[test]
    fn test_message_sender_send_after_shutdown() {
        let mut bus = MessageBus::new();
        bus.start().unwrap();
        let sender = bus.sender();

        bus.shutdown().unwrap();

        // Sending after shutdown should fail
        let result = sender.send(BrowserMessage::Shutdown);
        assert!(result.is_err());
    }

    #[test]
    fn test_message_sender_is_send_sync() {
        // MessageSender must implement Send + Sync for thread safety
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<Box<dyn MessageSender>>();
    }

    // ========================================
    // RED PHASE: Tests for MessageHandler trait
    // ========================================

    struct TestHandler {
        received: Arc<Mutex<Vec<BrowserMessage>>>,
    }

    impl MessageHandler for TestHandler {
        fn handle(&self, message: BrowserMessage) -> Result<()> {
            self.received.lock().unwrap().push(message);
            Ok(())
        }
    }

    #[test]
    fn test_message_handler_receives_message() {
        let received = Arc::new(Mutex::new(Vec::new()));
        let handler = TestHandler {
            received: Arc::clone(&received),
        };

        let msg = BrowserMessage::Shutdown;
        handler.handle(msg.clone()).unwrap();

        let msgs = received.lock().unwrap();
        assert_eq!(msgs.len(), 1);
        assert!(matches!(msgs[0], BrowserMessage::Shutdown));
    }

    #[test]
    fn test_message_handler_error_handling() {
        struct ErrorHandler;
        impl MessageHandler for ErrorHandler {
            fn handle(&self, _message: BrowserMessage) -> Result<()> {
                Err(Error::HandlerError("Test error".to_string()))
            }
        }

        let handler = ErrorHandler;
        let result = handler.handle(BrowserMessage::Shutdown);
        assert!(result.is_err());
    }

    // ========================================
    // RED PHASE: Tests for MessageBus
    // ========================================

    #[test]
    fn test_message_bus_new() {
        let _bus = MessageBus::new();
        // Bus should be created successfully
    }

    #[test]
    fn test_message_bus_start() {
        let mut bus = MessageBus::new();
        let result = bus.start();
        assert!(result.is_ok());
        bus.shutdown().unwrap();
    }

    #[test]
    fn test_message_bus_start_twice_fails() {
        let mut bus = MessageBus::new();
        bus.start().unwrap();

        // Starting twice should fail
        let result = bus.start();
        assert!(result.is_err());

        bus.shutdown().unwrap();
    }

    #[test]
    fn test_message_bus_shutdown() {
        let mut bus = MessageBus::new();
        bus.start().unwrap();

        let result = bus.shutdown();
        assert!(result.is_ok());
    }

    #[test]
    fn test_message_bus_shutdown_twice_fails() {
        let mut bus = MessageBus::new();
        bus.start().unwrap();
        bus.shutdown().unwrap();

        // Shutting down twice should fail
        let result = bus.shutdown();
        assert!(result.is_err());
    }

    #[test]
    fn test_message_bus_sender() {
        let mut bus = MessageBus::new();
        bus.start().unwrap();

        let _sender = bus.sender();
        // Sender should be created successfully

        bus.shutdown().unwrap();
    }

    #[test]
    fn test_message_bus_register_handler() {
        let mut bus = MessageBus::new();

        let received = Arc::new(Mutex::new(Vec::new()));
        let handler = TestHandler {
            received: Arc::clone(&received),
        };

        bus.register_handler(Box::new(handler));
        // Handler should be registered successfully
    }

    #[test]
    fn test_message_bus_message_routing() {
        let mut bus = MessageBus::new();

        let received = Arc::new(Mutex::new(Vec::new()));
        let handler = TestHandler {
            received: Arc::clone(&received),
        };

        bus.register_handler(Box::new(handler));
        bus.start().unwrap();

        let sender = bus.sender();
        sender
            .send(BrowserMessage::CreateTab { parent_window: 1 })
            .unwrap();
        sender.send(BrowserMessage::CloseTab { tab_id: 2 }).unwrap();

        // Give time for messages to be processed
        std::thread::sleep(std::time::Duration::from_millis(50));

        bus.shutdown().unwrap();

        let msgs = received.lock().unwrap();
        assert_eq!(msgs.len(), 2);
    }

    #[test]
    fn test_message_bus_multiple_handlers() {
        let mut bus = MessageBus::new();

        let received1 = Arc::new(Mutex::new(Vec::new()));
        let received2 = Arc::new(Mutex::new(Vec::new()));

        let handler1 = TestHandler {
            received: Arc::clone(&received1),
        };
        let handler2 = TestHandler {
            received: Arc::clone(&received2),
        };

        bus.register_handler(Box::new(handler1));
        bus.register_handler(Box::new(handler2));
        bus.start().unwrap();

        let sender = bus.sender();
        sender.send(BrowserMessage::Shutdown).unwrap();

        // Give time for messages to be processed
        std::thread::sleep(std::time::Duration::from_millis(50));

        bus.shutdown().unwrap();

        // Both handlers should receive the message
        assert_eq!(received1.lock().unwrap().len(), 1);
        assert_eq!(received2.lock().unwrap().len(), 1);
    }

    #[test]
    fn test_message_bus_navigate_request() {
        let mut bus = MessageBus::new();

        let received = Arc::new(Mutex::new(Vec::new()));
        let handler = TestHandler {
            received: Arc::clone(&received),
        };

        bus.register_handler(Box::new(handler));
        bus.start().unwrap();

        let sender = bus.sender();
        let url = Url::parse("https://example.com").unwrap();
        sender
            .send(BrowserMessage::NavigateRequest {
                tab_id: 1,
                url: url.clone(),
            })
            .unwrap();

        std::thread::sleep(std::time::Duration::from_millis(50));
        bus.shutdown().unwrap();

        let msgs = received.lock().unwrap();
        assert_eq!(msgs.len(), 1);
        match &msgs[0] {
            BrowserMessage::NavigateRequest {
                tab_id,
                url: msg_url,
            } => {
                assert_eq!(*tab_id, 1);
                assert_eq!(msg_url.as_str(), "https://example.com/");
            }
            _ => panic!("Expected NavigateRequest"),
        }
    }

    #[test]
    fn test_message_bus_concurrent_senders() {
        use std::thread;

        let mut bus = MessageBus::new();

        let received = Arc::new(Mutex::new(Vec::new()));
        let handler = TestHandler {
            received: Arc::clone(&received),
        };

        bus.register_handler(Box::new(handler));
        bus.start().unwrap();

        let sender1 = bus.sender();
        let sender2 = bus.sender();

        // Spawn two threads sending messages concurrently
        let handle1 = thread::spawn(move || {
            for i in 0..10 {
                sender1
                    .send(BrowserMessage::CreateTab { parent_window: i })
                    .unwrap();
            }
        });

        let handle2 = thread::spawn(move || {
            for i in 10..20 {
                sender2
                    .send(BrowserMessage::CreateTab { parent_window: i })
                    .unwrap();
            }
        });

        handle1.join().unwrap();
        handle2.join().unwrap();

        std::thread::sleep(std::time::Duration::from_millis(100));
        bus.shutdown().unwrap();

        // Should receive all 20 messages
        let msgs = received.lock().unwrap();
        assert_eq!(msgs.len(), 20);
    }

    #[test]
    fn test_message_bus_handler_error_does_not_crash() {
        struct FaultyHandler;
        impl MessageHandler for FaultyHandler {
            fn handle(&self, _message: BrowserMessage) -> Result<()> {
                Err(Error::HandlerError("Intentional error".to_string()))
            }
        }

        let mut bus = MessageBus::new();
        bus.register_handler(Box::new(FaultyHandler));
        bus.start().unwrap();

        let sender = bus.sender();
        // Should not panic even if handler returns error
        sender.send(BrowserMessage::Shutdown).unwrap();

        std::thread::sleep(std::time::Duration::from_millis(50));
        bus.shutdown().unwrap();
    }
}
