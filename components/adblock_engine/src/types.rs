//! Type definitions for adblock_engine component

use crate::errors::{Error, Result};
use adblock::request::Request;
use adblock::Engine;
use config_manager::AdBlockConfig;
use message_bus::MessageSender;
use shared_types::ResourceType;
use std::sync::{Arc, Mutex};

/// Ad blocking filter engine
///
/// This engine uses the adblock crate to filter web requests based on EasyList
/// and custom filter rules.
#[derive(Clone)]
#[allow(clippy::arc_with_non_send_sync)]
pub struct AdBlockEngine {
    /// Configuration for ad blocking
    config: AdBlockConfig,

    /// Message sender for component communication
    #[allow(dead_code)]
    sender: Arc<Box<dyn MessageSender>>,

    /// The adblock engine (wrapped in Mutex for interior mutability)
    engine: Arc<Mutex<Option<Engine>>>,

    /// Whether the engine has been initialized
    initialized: Arc<Mutex<bool>>,
}

impl AdBlockEngine {
    /// Create a new AdBlockEngine
    ///
    /// # Arguments
    ///
    /// * `config` - AdBlock configuration settings
    /// * `sender` - Message sender for component communication
    ///
    /// # Returns
    ///
    /// Returns a new AdBlockEngine instance.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use adblock_engine::AdBlockEngine;
    /// use config_manager::AdBlockConfig;
    ///
    /// let config = AdBlockConfig {
    ///     enabled: true,
    ///     update_filters_on_startup: false,
    ///     custom_filters: vec![],
    /// };
    ///
    /// let engine = AdBlockEngine::new(config, sender)?;
    /// ```
    #[allow(clippy::arc_with_non_send_sync)]
    pub fn new(config: AdBlockConfig, sender: Box<dyn MessageSender>) -> Result<Self> {
        Ok(Self {
            config,
            sender: Arc::new(sender),
            engine: Arc::new(Mutex::new(None)),
            initialized: Arc::new(Mutex::new(false)),
        })
    }

    /// Initialize the ad blocking engine
    ///
    /// This loads filter rules from EasyList and custom filters.
    /// If the EasyList file is missing, it will use only custom filters.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success, or an error if initialization fails.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The engine is already initialized
    /// - Filter rules cannot be parsed
    pub fn initialize(&mut self) -> Result<()> {
        // Check if already initialized
        {
            let initialized = self.initialized.lock().unwrap();
            if *initialized {
                return Err(Error::AlreadyInitialized);
            }
        }

        // Create new adblock engine
        let mut filter_rules = Vec::new();

        // Try to load EasyList from file
        let easylist_path = "../../resources/filters/easylist.txt";
        if let Ok(content) = std::fs::read_to_string(easylist_path) {
            // Parse EasyList rules
            for line in content.lines() {
                let trimmed = line.trim();
                // Skip comments and empty lines
                if !trimmed.is_empty() && !trimmed.starts_with('!') && !trimmed.starts_with('[') {
                    filter_rules.push(trimmed.to_string());
                }
            }
        }
        // Note: If EasyList doesn't exist, we continue with just custom filters

        // Add custom filters
        for filter in &self.config.custom_filters {
            filter_rules.push(filter.clone());
        }

        // Create adblock engine with filter rules
        // Use default ParseOptions
        let engine = Engine::from_rules(filter_rules, Default::default());

        // Store the engine
        {
            let mut engine_guard = self.engine.lock().unwrap();
            *engine_guard = Some(engine);
        }

        // Mark as initialized
        {
            let mut initialized = self.initialized.lock().unwrap();
            *initialized = true;
        }

        Ok(())
    }

    /// Check if a URL should be blocked
    ///
    /// # Arguments
    ///
    /// * `url` - The URL to check
    /// * `resource_type` - The type of resource being loaded
    ///
    /// # Returns
    ///
    /// Returns `true` if the URL should be blocked, `false` otherwise.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use shared_types::ResourceType;
    ///
    /// let blocked = engine.should_block(
    ///     "https://ads.example.com/banner.js",
    ///     ResourceType::Script
    /// );
    /// ```
    pub fn should_block(&self, url: &str, resource_type: ResourceType) -> bool {
        // If ad blocking is disabled, never block
        if !self.config.enabled {
            return false;
        }

        // If not initialized, don't block (safe default)
        {
            let initialized = self.initialized.lock().unwrap();
            if !*initialized {
                return false;
            }
        }

        // Get the engine
        let engine_guard = self.engine.lock().unwrap();
        let engine = match engine_guard.as_ref() {
            Some(e) => e,
            None => return false,
        };

        // Convert ResourceType to adblock resource type string
        let adblock_resource_type = match resource_type {
            ResourceType::Document => "document",
            ResourceType::Script => "script",
            ResourceType::Image => "image",
            ResourceType::Stylesheet => "stylesheet",
            ResourceType::Font => "font",
            ResourceType::Media => "media",
            ResourceType::Websocket => "websocket",
            ResourceType::Xhr => "xmlhttprequest",
            ResourceType::Other => "other",
        };

        // Create a Request object
        // Use empty string as source URL (first-party request)
        let request = match Request::new(url, "", adblock_resource_type) {
            Ok(req) => req,
            Err(_) => {
                // Invalid URL, don't block
                return false;
            }
        };

        // Check if URL should be blocked
        let check_result = engine.check_network_request(&request);

        check_result.matched
    }

    /// Get cosmetic filtering CSS selectors for a given URL
    ///
    /// # Arguments
    ///
    /// * `url` - The URL to get selectors for
    ///
    /// # Returns
    ///
    /// Returns a vector of CSS selectors to hide elements.
    pub fn get_cosmetic_filters(&self, url: &str) -> Vec<String> {
        // If ad blocking is disabled, return empty
        if !self.config.enabled {
            return vec![];
        }

        // If not initialized, return empty
        {
            let initialized = self.initialized.lock().unwrap();
            if !*initialized {
                return vec![];
            }
        }

        // Get the engine
        let engine_guard = self.engine.lock().unwrap();
        let engine = match engine_guard.as_ref() {
            Some(e) => e,
            None => return vec![],
        };

        // Get cosmetic filters for URL
        let cosmetic = engine.url_cosmetic_resources(url);

        // Combine hide selectors
        let mut selectors = vec![];

        // Add general hide selectors
        for selector in cosmetic.hide_selectors {
            selectors.push(selector);
        }

        selectors
    }
}
