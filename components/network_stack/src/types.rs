//! Core types for network stack component

use crate::cache::HttpCache;
use crate::errors::{Error, Result};
use config_manager::NetworkConfig;
use message_bus::MessageSender;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use url::Url;

/// Resource timing information for performance tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceTiming {
    /// URL of the resource
    pub url: String,
    /// Time when request started
    pub start_time: Duration,
    /// Time when request ended
    pub end_time: Duration,
    /// Total duration in milliseconds
    pub duration_ms: u64,
    /// Size of the response in bytes
    pub size_bytes: usize,
    /// Whether the resource was served from cache
    pub from_cache: bool,
}

/// Main network stack structure
pub struct NetworkStack {
    /// HTTP client
    client: Client,
    /// Message bus sender for communication (reserved for future use)
    _sender: Box<dyn MessageSender>,
    /// HTTP cache
    cache: Option<HttpCache>,
    /// Configuration (reserved for future use)
    _config: NetworkConfig,
    /// Timing data for requests
    timing_data: Arc<Mutex<Vec<ResourceTiming>>>,
    /// Whether the stack is initialized
    initialized: bool,
}

impl NetworkStack {
    /// Create a new network stack
    ///
    /// # Arguments
    ///
    /// * `config` - Network configuration
    /// * `sender` - Message bus sender for component communication
    ///
    /// # Returns
    ///
    /// Returns a Result containing the NetworkStack or an error
    pub fn new(config: NetworkConfig, sender: Box<dyn MessageSender>) -> Result<Self> {
        // Build HTTP client with configuration
        let timeout = Duration::from_secs(config.timeout_seconds as u64);

        let client = Client::builder()
            .timeout(timeout)
            .cookie_store(config.enable_cookies)
            .gzip(true)
            .brotli(true)
            .user_agent("FrankensteinBrowser/1.0")
            .build()
            .map_err(|e| Error::InitializationError(e.to_string()))?;

        // Create cache if enabled
        let cache = if config.enable_cache {
            Some(HttpCache::new(config.cache_size_mb as usize, None))
        } else {
            None
        };

        Ok(Self {
            client,
            _sender: sender,
            cache,
            _config: config,
            timing_data: Arc::new(Mutex::new(Vec::new())),
            initialized: false,
        })
    }

    /// Initialize the network stack
    ///
    /// This prepares the network stack for use.
    ///
    /// # Returns
    ///
    /// Returns Ok(()) on success or an error
    pub fn initialize(&mut self) -> Result<()> {
        if self.initialized {
            return Err(Error::InitializationError(
                "Already initialized".to_string(),
            ));
        }

        self.initialized = true;
        Ok(())
    }

    /// Fetch a resource from a URL
    ///
    /// # Arguments
    ///
    /// * `url` - The URL to fetch
    ///
    /// # Returns
    ///
    /// Returns the response body as bytes or an error
    pub async fn fetch(&self, url: Url) -> Result<Vec<u8>> {
        self.fetch_with_method(url, "GET").await
    }

    /// Fetch a resource with a specific HTTP method
    ///
    /// # Arguments
    ///
    /// * `url` - The URL to fetch
    /// * `method` - HTTP method (GET, POST, PUT, DELETE, etc.)
    ///
    /// # Returns
    ///
    /// Returns the response body as bytes or an error
    pub async fn fetch_with_method(&self, url: Url, method: &str) -> Result<Vec<u8>> {
        if !self.initialized {
            return Err(Error::InitializationError(
                "Network stack not initialized".to_string(),
            ));
        }

        let start = Instant::now();

        // Invalidate cache for POST/PUT/DELETE requests
        if method == "POST" || method == "PUT" || method == "DELETE" {
            if let Some(ref cache) = self.cache {
                cache.invalidate(&url);
            }
        }

        // Check cache first (only for GET requests)
        if method == "GET" {
            if let Some(ref cache) = self.cache {
                if let Some(cached_entry) = cache.get(&url) {
                    // Check if we can use cached response without revalidation
                    if cached_entry.can_use_without_revalidation() {
                        let end = Instant::now();
                        let duration = end.duration_since(start);

                        // Record timing
                        self.record_timing(ResourceTiming {
                            url: url.as_str().to_string(),
                            start_time: Duration::from_secs(0),
                            end_time: duration,
                            duration_ms: duration.as_millis() as u64,
                            size_bytes: cached_entry.body.len(),
                            from_cache: true,
                        });

                        return Ok(cached_entry.body);
                    }
                    // TODO: Implement conditional requests with If-None-Match/If-Modified-Since
                    // For now, just fetch fresh if revalidation needed
                }
            }
        }

        // Build request with appropriate method
        let request_builder = match method {
            "GET" => self.client.get(url.clone()),
            "POST" => self.client.post(url.clone()),
            "PUT" => self.client.put(url.clone()),
            "DELETE" => self.client.delete(url.clone()),
            "HEAD" => self.client.head(url.clone()),
            _ => self.client.get(url.clone()),
        };

        // TODO: Add conditional request headers (If-None-Match, If-Modified-Since)
        // if let Some(ref cache) = self.cache {
        //     if let Some(cached_entry) = cache.get(&url) {
        //         if let Some(ref etag) = cached_entry.etag {
        //             request_builder = request_builder.header("If-None-Match", etag);
        //         }
        //         if let Some(ref last_modified) = cached_entry.last_modified {
        //             request_builder = request_builder.header("If-Modified-Since", last_modified);
        //         }
        //     }
        // }

        // Send request
        let response = request_builder.send().await.map_err(|e| {
            if e.is_timeout() {
                Error::Timeout
            } else {
                Error::RequestFailed(e.to_string())
            }
        })?;

        let status = response.status();

        // Extract headers for caching
        let mut headers = HashMap::new();
        for (name, value) in response.headers().iter() {
            if let Ok(value_str) = value.to_str() {
                headers.insert(name.as_str().to_string(), value_str.to_string());
            }
        }

        // Handle 304 Not Modified (TODO: return cached content)
        if status.as_u16() == 304 {
            // TODO: Return cached content
            return Err(Error::RequestFailed(
                "304 Not Modified (conditional request not implemented yet)".to_string(),
            ));
        }

        if !status.is_success() {
            return Err(Error::RequestFailed(format!(
                "HTTP error: {}",
                status.as_u16()
            )));
        }

        let bytes = response
            .bytes()
            .await
            .map_err(|e| Error::RequestFailed(format!("Failed to read response body: {}", e)))?;

        let data = bytes.to_vec();
        let end = Instant::now();
        let duration = end.duration_since(start);

        // Cache the response if caching is enabled (only for GET requests)
        if method == "GET" {
            if let Some(ref cache) = self.cache {
                if HttpCache::is_cacheable(status.as_u16(), &headers) {
                    cache.put(url.clone(), data.clone(), headers);
                }
            }
        }

        // Record timing
        self.record_timing(ResourceTiming {
            url: url.as_str().to_string(),
            start_time: Duration::from_secs(0),
            end_time: duration,
            duration_ms: duration.as_millis() as u64,
            size_bytes: data.len(),
            from_cache: false,
        });

        Ok(data)
    }

    /// Get all timing data collected so far
    ///
    /// # Returns
    ///
    /// Returns a vector of ResourceTiming records
    pub fn get_timing_data(&self) -> Vec<ResourceTiming> {
        self.timing_data.lock().unwrap().clone()
    }

    /// Clear all timing data
    pub fn clear_timing_data(&mut self) {
        self.timing_data.lock().unwrap().clear();
    }

    /// Clear the HTTP cache
    pub fn clear_cache(&mut self) {
        if let Some(ref cache) = self.cache {
            cache.clear();
        }
    }

    /// Record timing information for a request
    fn record_timing(&self, timing: ResourceTiming) {
        self.timing_data.lock().unwrap().push(timing);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================
    // RED PHASE: Tests for ResourceTiming
    // ========================================

    #[test]
    fn test_resource_timing_creation() {
        let timing = ResourceTiming {
            url: "https://example.com".to_string(),
            start_time: Duration::from_secs(0),
            end_time: Duration::from_millis(150),
            duration_ms: 150,
            size_bytes: 1024,
            from_cache: false,
        };

        assert_eq!(timing.url, "https://example.com");
        assert_eq!(timing.duration_ms, 150);
        assert_eq!(timing.size_bytes, 1024);
        assert!(!timing.from_cache);
    }

    #[test]
    fn test_resource_timing_serialization() {
        let timing = ResourceTiming {
            url: "https://example.com".to_string(),
            start_time: Duration::from_secs(0),
            end_time: Duration::from_millis(100),
            duration_ms: 100,
            size_bytes: 512,
            from_cache: true,
        };

        let json = serde_json::to_string(&timing).unwrap();
        assert!(json.contains("example.com"));
        assert!(json.contains("100"));
        assert!(json.contains("512"));
        assert!(json.contains("true"));
    }

    #[test]
    fn test_resource_timing_deserialization() {
        let json = r#"{
            "url": "https://example.com",
            "start_time": {"secs": 0, "nanos": 0},
            "end_time": {"secs": 0, "nanos": 100000000},
            "duration_ms": 100,
            "size_bytes": 512,
            "from_cache": true
        }"#;

        let timing: ResourceTiming = serde_json::from_str(json).unwrap();
        assert_eq!(timing.url, "https://example.com");
        assert_eq!(timing.duration_ms, 100);
        assert_eq!(timing.size_bytes, 512);
        assert!(timing.from_cache);
    }
}
