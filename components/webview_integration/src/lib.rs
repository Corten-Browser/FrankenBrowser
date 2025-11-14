//! WebView wrapper with platform-specific implementations
//!
//! This is the webview_integration component of the FrankenBrowser project.
//!
//! # Component Overview
//!
//! **Type**: library
//! **Level**: 2
//! **Token Budget**: 12000 tokens
//!
//! # Dependencies
//!
//! - shared_types
//! - message_bus
//!
//! # Usage
//!
//! See README.md and CLAUDE.md for detailed usage and development instructions.

pub mod errors;
pub mod types;

// Re-export main types for convenience
pub use errors::{Error, Result};
pub use types::WebViewWrapper;
