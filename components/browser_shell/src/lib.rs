//! Window and tab management, UI shell
//!
//! This is the browser_shell component of the FrankenBrowser project.
//!
//! # Component Overview
//!
//! **Type**: library
//! **Level**: 3 (integration level)
//! **Token Budget**: 14000 tokens
//!
//! # Dependencies
//!
//! - shared_types
//! - message_bus
//! - config_manager
//! - network_stack
//! - adblock_engine
//! - browser_core
//! - webview_integration
//!
//! # Usage
//!
//! ```rust
//! use browser_shell::BrowserShell;
//! use config_manager::ShellConfig;
//! use message_bus::MessageBus;
//! use std::sync::Arc;
//! use tokio::runtime::Runtime;
//!
//! let config = ShellConfig {
//!     homepage: "https://www.example.com".to_string(),
//!     enable_devtools: true,
//!     theme: "light".to_string(),
//!     default_zoom: 1.0,
//! };
//!
//! let mut bus = MessageBus::new();
//! bus.start().unwrap();
//! let sender = bus.sender();
//! let runtime = Arc::new(Runtime::new().unwrap());
//!
//! let mut shell = BrowserShell::new(config, sender, runtime).unwrap();
//!
//! // Create tabs
//! let tab1 = shell.create_tab().unwrap();
//! let tab2 = shell.create_tab().unwrap();
//!
//! // Switch tabs
//! shell.switch_to_tab(tab2).unwrap();
//!
//! // Get tab count
//! assert_eq!(shell.get_tab_count(), 2);
//!
//! // Run the shell (in headless mode, this is a no-op)
//! shell.run().unwrap();
//! ```

pub mod errors;
pub mod types;

// Re-export main types for convenience
pub use errors::{Error, Result};
pub use types::{BrowserShell, Tab};
