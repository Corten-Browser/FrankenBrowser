//! Ad blocking filter engine using EasyList
//!
//! This component provides ad blocking functionality using the adblock crate
//! and EasyList filter rules.
//!
//! # Component Overview
//!
//! **Type**: library
//! **Level**: 1
//! **Token Budget**: 8000 tokens
//!
//! # Dependencies
//!
//! - shared_types: Common types for browser components
//! - message_bus: Message passing infrastructure
//! - config_manager: Configuration management
//! - adblock: Ad blocking engine
//!
//! # Usage
//!
//! ```ignore
//! use adblock_engine::AdBlockEngine;
//! use config_manager::AdBlockConfig;
//! use shared_types::ResourceType;
//!
//! // Create configuration
//! let config = AdBlockConfig {
//!     enabled: true,
//!     update_filters_on_startup: false,
//!     custom_filters: vec!["||ads.example.com^".to_string()],
//! };
//!
//! // Create and initialize engine
//! let mut engine = AdBlockEngine::new(config, sender)?;
//! engine.initialize()?;
//!
//! // Check if URL should be blocked
//! let blocked = engine.should_block(
//!     "https://ads.example.com/banner.js",
//!     ResourceType::Script
//! );
//!
//! if blocked {
//!     println!("Ad blocked!");
//! }
//! ```

pub mod errors;
pub mod types;

// Re-export main types for convenience
pub use errors::{Error, Result};
pub use types::AdBlockEngine;
