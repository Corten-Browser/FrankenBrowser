//! JavaScript ↔ Rust IPC Bridge
//!
//! This module provides bidirectional communication between JavaScript and Rust
//! through an IPC (Inter-Process Communication) bridge.
//!
//! # Features
//!
//! - Message dispatching with typed channels
//! - Callback registration for handling messages
//! - Event emission from Rust to JavaScript
//! - Built-in handlers for common operations
//! - Type-safe serialization/deserialization

use crate::errors::{Error, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;
use uuid::Uuid;

/// IPC message sent between JavaScript and Rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpcMessage {
    /// Message channel/type (e.g., "navigation", "console", "error")
    pub channel: String,
    /// Message payload (arbitrary JSON data)
    pub data: JsonValue,
    /// Timestamp when message was created
    #[serde(with = "system_time_serde")]
    pub timestamp: SystemTime,
    /// Unique message identifier
    pub message_id: Uuid,
}

/// Serde serialization for SystemTime
mod system_time_serde {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::time::{SystemTime, UNIX_EPOCH};

    pub fn serialize<S>(time: &SystemTime, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let duration = time
            .duration_since(UNIX_EPOCH)
            .map_err(serde::ser::Error::custom)?;
        duration.as_secs().serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> std::result::Result<SystemTime, D::Error>
    where
        D: Deserializer<'de>,
    {
        let secs = u64::deserialize(deserializer)?;
        Ok(UNIX_EPOCH + std::time::Duration::from_secs(secs))
    }
}

impl IpcMessage {
    /// Create a new IPC message
    pub fn new(channel: impl Into<String>, data: JsonValue) -> Self {
        Self {
            channel: channel.into(),
            data,
            timestamp: SystemTime::now(),
            message_id: Uuid::new_v4(),
        }
    }

    /// Parse an IPC message from JSON string
    pub fn parse(json: &str) -> Result<Self> {
        serde_json::from_str(json)
            .map_err(|e| Error::Ipc(format!("Failed to parse IPC message: {}", e)))
    }

    /// Serialize IPC message to JSON string
    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string(self)
            .map_err(|e| Error::Ipc(format!("Failed to serialize IPC message: {}", e)))
    }
}

/// Callback function type for handling IPC messages
pub type IpcCallback = Box<dyn Fn(&IpcMessage) -> Result<JsonValue> + Send + Sync>;

/// JavaScript Bridge for IPC communication
pub struct JavaScriptBridge {
    /// Registered message handlers (channel -> list of callbacks)
    handlers: Arc<Mutex<HashMap<String, Vec<Arc<IpcCallback>>>>>,
}

impl JavaScriptBridge {
    /// Create a new JavaScript bridge
    pub fn new() -> Self {
        let bridge = Self {
            handlers: Arc::new(Mutex::new(HashMap::new())),
        };

        // Register built-in handlers
        bridge.register_built_in_handlers();

        bridge
    }

    /// Register a message handler for a specific channel
    pub fn register_handler(&self, channel: &str, callback: IpcCallback) {
        let mut handlers = self.handlers.lock().unwrap();
        let callback_arc = Arc::new(callback);

        handlers
            .entry(channel.to_string())
            .or_default()
            .push(callback_arc);
    }

    /// Unregister all handlers for a specific channel
    pub fn unregister_handler(&self, channel: &str) {
        let mut handlers = self.handlers.lock().unwrap();
        handlers.remove(channel);
    }

    /// Dispatch an IPC message to registered handlers
    pub fn dispatch_message(&self, message: IpcMessage) -> Result<JsonValue> {
        let handlers = self.handlers.lock().unwrap();

        match handlers.get(&message.channel) {
            Some(callbacks) => {
                // Execute all callbacks for this channel
                let mut results = Vec::new();

                for callback in callbacks {
                    match callback(&message) {
                        Ok(result) => results.push(result),
                        Err(e) => {
                            eprintln!("IPC callback error on channel '{}': {}", message.channel, e);
                            // Continue with other callbacks even if one fails
                        }
                    }
                }

                // Return array of all results
                Ok(JsonValue::Array(results))
            }
            None => {
                // No handlers registered for this channel
                eprintln!("No handlers registered for channel: {}", message.channel);
                Ok(JsonValue::Null)
            }
        }
    }

    /// Emit an event from Rust to JavaScript
    ///
    /// Returns JavaScript code that can be executed to dispatch the event
    pub fn emit_event(&self, channel: &str, data: JsonValue) -> Result<String> {
        let message = IpcMessage::new(channel, data);

        // Generate JavaScript code to dispatch the event
        Ok(format!(
            r#"
            if (window.ipc && window.ipc._dispatch) {{
                window.ipc._dispatch('{}', {});
            }}
            "#,
            channel,
            serde_json::to_string(&message.data)
                .map_err(|e| Error::Ipc(format!("Failed to serialize event data: {}", e)))?
        ))
    }

    /// Parse an IPC message from raw JSON string
    pub fn parse_ipc_message(&self, raw: &str) -> Result<IpcMessage> {
        IpcMessage::parse(raw)
    }

    /// Serialize a response value to JSON string
    pub fn serialize_response(&self, value: JsonValue) -> String {
        serde_json::to_string(&value).unwrap_or_else(|_| "null".to_string())
    }

    /// Generate the IPC initialization script for injection into WebView
    pub fn generate_ipc_script(&self) -> String {
        // The init.js script is already created in resources/init.js
        // This function returns the script content
        include_str!("../../../resources/init.js").to_string()
    }

    /// Create an event emitter script for a specific channel
    pub fn create_event_emitter(&self, channel: &str, data: JsonValue) -> String {
        format!(
            r#"
            if (window.ipc && window.ipc._dispatch) {{
                window.ipc._dispatch('{}', {});
            }}
            "#,
            channel,
            serde_json::to_string(&data).unwrap_or_else(|_| "null".to_string())
        )
    }

    /// Create a callback bridge script
    pub fn create_callback_bridge(&self, callback_id: &str) -> String {
        format!(
            r#"
            (function(callbackId) {{
                return function(...args) {{
                    if (window.ipc) {{
                        window.ipc.send('callback', {{
                            callbackId: callbackId,
                            args: args
                        }});
                    }}
                }};
            }})('{}')
            "#,
            callback_id
        )
    }

    /// Register built-in message handlers
    fn register_built_in_handlers(&self) {
        // Navigation handler
        self.register_handler(
            "navigation",
            Box::new(|msg| {
                eprintln!("Navigation request: {:?}", msg.data);
                // In production, this would send a BrowserMessage via message bus
                Ok(JsonValue::Object(
                    vec![(
                        "status".to_string(),
                        JsonValue::String("acknowledged".to_string()),
                    )]
                    .into_iter()
                    .collect(),
                ))
            }),
        );

        // Console handler
        self.register_handler(
            "console",
            Box::new(|msg| {
                // Extract console level and message
                if let Some(obj) = msg.data.as_object() {
                    let level = obj.get("level").and_then(|v| v.as_str()).unwrap_or("log");
                    let message = obj.get("message").and_then(|v| v.as_str()).unwrap_or("");

                    // Forward to Rust logging system
                    match level {
                        "error" => eprintln!("[JS Console Error] {}", message),
                        "warn" => eprintln!("[JS Console Warn] {}", message),
                        "info" => println!("[JS Console Info] {}", message),
                        "debug" => println!("[JS Console Debug] {}", message),
                        _ => println!("[JS Console] {}", message),
                    }
                }

                Ok(JsonValue::Null)
            }),
        );

        // Error handler
        self.register_handler(
            "error",
            Box::new(|msg| {
                // Extract error details
                if let Some(obj) = msg.data.as_object() {
                    let error_type = obj
                        .get("type")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown");
                    let message = obj
                        .get("message")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Unknown error");
                    let filename = obj.get("filename").and_then(|v| v.as_str());
                    let lineno = obj.get("lineno").and_then(|v| v.as_u64());

                    // Log to Rust error system
                    if let (Some(file), Some(line)) = (filename, lineno) {
                        eprintln!(
                            "[JS Error] {} at {}:{}: {}",
                            error_type, file, line, message
                        );
                    } else {
                        eprintln!("[JS Error] {}: {}", error_type, message);
                    }
                }

                Ok(JsonValue::Null)
            }),
        );

        // Page info handler
        self.register_handler(
            "pageInfo",
            Box::new(|_msg| {
                // Return current page metadata
                // In production, this would query actual page state
                Ok(JsonValue::Object(
                    vec![
                        ("url".to_string(), JsonValue::String("".to_string())),
                        ("title".to_string(), JsonValue::String("".to_string())),
                        (
                            "loadTime".to_string(),
                            JsonValue::Number(serde_json::Number::from(0)),
                        ),
                    ]
                    .into_iter()
                    .collect(),
                ))
            }),
        );

        // DOM ready handler
        self.register_handler(
            "dom-ready",
            Box::new(|msg| {
                println!("[IPC] DOM ready: {:?}", msg.data);
                Ok(JsonValue::Null)
            }),
        );

        // Page load handler
        self.register_handler(
            "page-load",
            Box::new(|msg| {
                println!("[IPC] Page loaded: {:?}", msg.data);
                Ok(JsonValue::Null)
            }),
        );

        // Init complete handler
        self.register_handler(
            "init-complete",
            Box::new(|msg| {
                println!("[IPC] WebView initialization complete: {:?}", msg.data);
                Ok(JsonValue::Null)
            }),
        );
    }
}

impl Default for JavaScriptBridge {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // ========================================
    // RED PHASE: Tests for IpcMessage
    // ========================================

    #[test]
    fn test_ipc_message_new() {
        let data = json!({"key": "value"});
        let msg = IpcMessage::new("test-channel", data.clone());

        assert_eq!(msg.channel, "test-channel");
        assert_eq!(msg.data, data);
        assert!(msg.message_id.to_string().len() > 0);
    }

    #[test]
    fn test_ipc_message_parse_valid_json() {
        let json_str = r#"{
            "channel": "test",
            "data": {"key": "value"},
            "timestamp": 1234567890,
            "message_id": "550e8400-e29b-41d4-a716-446655440000"
        }"#;

        let result = IpcMessage::parse(json_str);
        assert!(result.is_ok());

        let msg = result.unwrap();
        assert_eq!(msg.channel, "test");
        assert_eq!(msg.data["key"], "value");
    }

    #[test]
    fn test_ipc_message_parse_invalid_json() {
        let result = IpcMessage::parse("invalid json");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), Error::Ipc(_)));
    }

    #[test]
    fn test_ipc_message_to_json() {
        let data = json!({"key": "value"});
        let msg = IpcMessage::new("test", data);

        let result = msg.to_json();
        assert!(result.is_ok());

        let json_str = result.unwrap();
        assert!(json_str.contains("test"));
        assert!(json_str.contains("key"));
        assert!(json_str.contains("value"));
    }

    #[test]
    fn test_ipc_message_roundtrip() {
        let data = json!({"key": "value", "number": 42});
        let msg = IpcMessage::new("roundtrip", data.clone());

        let json_str = msg.to_json().unwrap();
        let parsed = IpcMessage::parse(&json_str).unwrap();

        assert_eq!(parsed.channel, "roundtrip");
        assert_eq!(parsed.data, data);
    }

    // ========================================
    // RED PHASE: Tests for JavaScriptBridge
    // ========================================

    #[test]
    fn test_bridge_new() {
        let bridge = JavaScriptBridge::new();
        let handlers = bridge.handlers.lock().unwrap();
        // Built-in handlers should be registered
        assert!(handlers.contains_key("navigation"));
        assert!(handlers.contains_key("console"));
        assert!(handlers.contains_key("error"));
        assert!(handlers.contains_key("pageInfo"));
    }

    #[test]
    fn test_bridge_register_handler() {
        let bridge = JavaScriptBridge::new();

        bridge.register_handler("custom", Box::new(|_msg| Ok(json!({"response": "custom"}))));

        let handlers = bridge.handlers.lock().unwrap();
        assert!(handlers.contains_key("custom"));
    }

    #[test]
    fn test_bridge_unregister_handler() {
        let bridge = JavaScriptBridge::new();

        bridge.register_handler("temp", Box::new(|_msg| Ok(json!(null))));

        {
            let handlers = bridge.handlers.lock().unwrap();
            assert!(handlers.contains_key("temp"));
        }

        bridge.unregister_handler("temp");

        let handlers = bridge.handlers.lock().unwrap();
        assert!(!handlers.contains_key("temp"));
    }

    #[test]
    fn test_bridge_dispatch_message_with_handler() {
        let bridge = JavaScriptBridge::new();

        bridge.register_handler(
            "test-dispatch",
            Box::new(|msg| {
                assert_eq!(msg.channel, "test-dispatch");
                Ok(json!({"handled": true}))
            }),
        );

        let msg = IpcMessage::new("test-dispatch", json!({"input": "data"}));
        let result = bridge.dispatch_message(msg);

        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(response.is_array());
        assert_eq!(response.as_array().unwrap().len(), 1);
    }

    #[test]
    fn test_bridge_dispatch_message_without_handler() {
        let bridge = JavaScriptBridge::new();

        let msg = IpcMessage::new("non-existent", json!({}));
        let result = bridge.dispatch_message(msg);

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), JsonValue::Null);
    }

    #[test]
    fn test_bridge_dispatch_multiple_handlers() {
        let bridge = JavaScriptBridge::new();

        bridge.register_handler("multi", Box::new(|_msg| Ok(json!({"handler": 1}))));
        bridge.register_handler("multi", Box::new(|_msg| Ok(json!({"handler": 2}))));

        let msg = IpcMessage::new("multi", json!({}));
        let result = bridge.dispatch_message(msg).unwrap();

        assert!(result.is_array());
        let arr = result.as_array().unwrap();
        assert_eq!(arr.len(), 2);
    }

    #[test]
    fn test_bridge_emit_event() {
        let bridge = JavaScriptBridge::new();

        let result = bridge.emit_event("test-event", json!({"key": "value"}));
        assert!(result.is_ok());

        let script = result.unwrap();
        assert!(script.contains("ipc._dispatch"));
        assert!(script.contains("test-event"));
    }

    #[test]
    fn test_bridge_parse_ipc_message() {
        let bridge = JavaScriptBridge::new();

        let json_str = r#"{
            "channel": "test",
            "data": {"key": "value"},
            "timestamp": 1234567890,
            "message_id": "550e8400-e29b-41d4-a716-446655440000"
        }"#;

        let result = bridge.parse_ipc_message(json_str);
        assert!(result.is_ok());
    }

    #[test]
    fn test_bridge_serialize_response() {
        let bridge = JavaScriptBridge::new();

        let value = json!({"key": "value", "number": 42});
        let json_str = bridge.serialize_response(value);

        assert!(json_str.contains("key"));
        assert!(json_str.contains("value"));
        assert!(json_str.contains("42"));
    }

    #[test]
    fn test_bridge_serialize_response_handles_errors() {
        let bridge = JavaScriptBridge::new();

        // Even if serialization theoretically fails, we get valid JSON
        let result = bridge.serialize_response(json!(null));
        assert_eq!(result, "null");
    }

    #[test]
    fn test_bridge_generate_ipc_script() {
        let bridge = JavaScriptBridge::new();

        let script = bridge.generate_ipc_script();
        assert!(script.contains("window.ipc"));
        assert!(script.contains("send"));
        assert!(script.contains("on"));
        assert!(script.contains("_dispatch"));
    }

    #[test]
    fn test_bridge_create_event_emitter() {
        let bridge = JavaScriptBridge::new();

        let script = bridge.create_event_emitter("custom-event", json!({"data": 123}));
        assert!(script.contains("ipc._dispatch"));
        assert!(script.contains("custom-event"));
        assert!(script.contains("123"));
    }

    #[test]
    fn test_bridge_create_callback_bridge() {
        let bridge = JavaScriptBridge::new();

        let script = bridge.create_callback_bridge("callback-123");
        assert!(script.contains("callback-123"));
        assert!(script.contains("callbackId"));
        assert!(script.contains("window.ipc.send"));
    }

    // ========================================
    // Tests for built-in handlers
    // ========================================

    #[test]
    fn test_navigation_handler() {
        let bridge = JavaScriptBridge::new();

        let msg = IpcMessage::new(
            "navigation",
            json!({
                "command": "navigate",
                "url": "https://example.com"
            }),
        );

        let result = bridge.dispatch_message(msg);
        assert!(result.is_ok());

        let response = result.unwrap();
        assert!(response.is_array());
    }

    #[test]
    fn test_console_handler() {
        let bridge = JavaScriptBridge::new();

        let msg = IpcMessage::new(
            "console",
            json!({
                "level": "log",
                "message": "Test message"
            }),
        );

        let result = bridge.dispatch_message(msg);
        assert!(result.is_ok());
    }

    #[test]
    fn test_error_handler() {
        let bridge = JavaScriptBridge::new();

        let msg = IpcMessage::new(
            "error",
            json!({
                "type": "error",
                "message": "Test error",
                "filename": "test.js",
                "lineno": 42
            }),
        );

        let result = bridge.dispatch_message(msg);
        assert!(result.is_ok());
    }

    #[test]
    fn test_page_info_handler() {
        let bridge = JavaScriptBridge::new();

        let msg = IpcMessage::new("pageInfo", json!({}));

        let result = bridge.dispatch_message(msg);
        assert!(result.is_ok());

        let response = result.unwrap();
        assert!(response.is_array());
        let first = &response.as_array().unwrap()[0];
        assert!(first.is_object());
        assert!(first.as_object().unwrap().contains_key("url"));
    }

    #[test]
    fn test_dom_ready_handler() {
        let bridge = JavaScriptBridge::new();

        let msg = IpcMessage::new(
            "dom-ready",
            json!({
                "url": "https://example.com",
                "title": "Test Page"
            }),
        );

        let result = bridge.dispatch_message(msg);
        assert!(result.is_ok());
    }

    #[test]
    fn test_page_load_handler() {
        let bridge = JavaScriptBridge::new();

        let msg = IpcMessage::new(
            "page-load",
            json!({
                "url": "https://example.com",
                "loadTime": 1234
            }),
        );

        let result = bridge.dispatch_message(msg);
        assert!(result.is_ok());
    }

    // ========================================
    // Tests for error handling
    // ========================================

    #[test]
    fn test_handler_error_propagation() {
        let bridge = JavaScriptBridge::new();

        // Register handler that returns error
        bridge.register_handler(
            "error-test",
            Box::new(|_msg| Err(Error::Ipc("Test error".to_string()))),
        );

        let msg = IpcMessage::new("error-test", json!({}));
        let result = bridge.dispatch_message(msg);

        // Dispatch should succeed even if handler fails
        assert!(result.is_ok());
        // But should return empty array (no successful results)
        let response = result.unwrap();
        assert!(response.is_array());
        assert_eq!(response.as_array().unwrap().len(), 0);
    }

    #[test]
    fn test_complex_data_serialization() {
        let bridge = JavaScriptBridge::new();

        let complex_data = json!({
            "nested": {
                "array": [1, 2, 3],
                "object": {
                    "key": "value"
                }
            },
            "number": 42.5,
            "boolean": true,
            "null": null
        });

        let msg = IpcMessage::new("test", complex_data.clone());
        let json_str = msg.to_json().unwrap();
        let parsed = IpcMessage::parse(&json_str).unwrap();

        assert_eq!(parsed.data, complex_data);
    }

    #[test]
    fn test_thread_safety() {
        use std::sync::Arc;
        use std::thread;

        let bridge = Arc::new(JavaScriptBridge::new());

        let handles: Vec<_> = (0..5)
            .map(|i| {
                let bridge = Arc::clone(&bridge);
                thread::spawn(move || {
                    let channel = format!("thread-{}", i);
                    bridge
                        .register_handler(&channel, Box::new(move |_msg| Ok(json!({"thread": i}))));

                    let msg = IpcMessage::new(&channel, json!({}));
                    bridge.dispatch_message(msg)
                })
            })
            .collect();

        for handle in handles {
            let result = handle.join().unwrap();
            assert!(result.is_ok());
        }
    }
}
