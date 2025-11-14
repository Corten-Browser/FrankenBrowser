//! Core types for network stack component

use crate::errors::{Error, Result};
use config_manager::NetworkConfig;
use lru::LruCache;
use message_bus::MessageSender;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::num::NonZeroUsize;
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

/// Cache entry with data and metadata
#[derive(Debug, Clone)]
struct CacheEntry {
    /// Cached response data
    data: Vec<u8>,
    /// Timestamp when cached (reserved for future TTL implementation)
    _cached_at: Instant,
    /// Size in bytes
    size: usize,
}

/// HTTP cache with LRU eviction policy
pub struct HttpCache {
    /// LRU cache storage
    cache: Arc<Mutex<LruCache<String, CacheEntry>>>,
    /// Maximum cache size in bytes
    max_size_bytes: usize,
    /// Current cache size in bytes
    current_size: Arc<Mutex<usize>>,
}

impl HttpCache {
    /// Create a new HTTP cache with specified size limit in MB
    pub fn new(max_size_mb: u32) -> Self {
        let max_size_bytes = (max_size_mb as usize) * 1024 * 1024;
        let capacity = NonZeroUsize::new(1000).unwrap(); // Max 1000 entries

        Self {
            cache: Arc::new(Mutex::new(LruCache::new(capacity))),
            max_size_bytes,
            current_size: Arc::new(Mutex::new(0)),
        }
    }

    /// Get cached data for a URL
    pub fn get(&self, url: &str) -> Option<Vec<u8>> {
        let mut cache = self.cache.lock().unwrap();
        cache.get(url).map(|entry| entry.data.clone())
    }

    /// Store data in cache for a URL
    pub fn put(&self, url: String, data: Vec<u8>) {
        let size = data.len();
        let entry = CacheEntry {
            data,
            _cached_at: Instant::now(),
            size,
        };

        let mut cache = self.cache.lock().unwrap();
        let mut current_size = self.current_size.lock().unwrap();

        // Evict entries if adding this would exceed max size
        while *current_size + size > self.max_size_bytes && !cache.is_empty() {
            if let Some((_, old_entry)) = cache.pop_lru() {
                *current_size -= old_entry.size;
            }
        }

        // Add new entry if it fits
        if size <= self.max_size_bytes {
            if let Some(old_entry) = cache.put(url, entry) {
                *current_size -= old_entry.size;
            }
            *current_size += size;
        }
    }

    /// Clear all cached data
    pub fn clear(&self) {
        let mut cache = self.cache.lock().unwrap();
        let mut current_size = self.current_size.lock().unwrap();
        cache.clear();
        *current_size = 0;
    }

    /// Get current cache size in bytes
    pub fn current_size(&self) -> usize {
        *self.current_size.lock().unwrap()
    }
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
            Some(HttpCache::new(config.cache_size_mb))
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
        if !self.initialized {
            return Err(Error::InitializationError(
                "Network stack not initialized".to_string(),
            ));
        }

        let url_str = url.to_string();
        let start = Instant::now();

        // Check cache first
        if let Some(ref cache) = self.cache {
            if let Some(cached_data) = cache.get(&url_str) {
                let end = Instant::now();
                let duration = end.duration_since(start);

                // Record timing
                self.record_timing(ResourceTiming {
                    url: url_str,
                    start_time: Duration::from_secs(0), // Relative to request start
                    end_time: duration,
                    duration_ms: duration.as_millis() as u64,
                    size_bytes: cached_data.len(),
                    from_cache: true,
                });

                return Ok(cached_data);
            }
        }

        // Fetch from network
        let response = self.client.get(url.clone()).send().await.map_err(|e| {
            if e.is_timeout() {
                Error::Timeout
            } else {
                Error::RequestFailed(e.to_string())
            }
        })?;

        let status = response.status();
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

        // Cache the response if caching is enabled
        if let Some(ref cache) = self.cache {
            cache.put(url_str.clone(), data.clone());
        }

        // Record timing
        self.record_timing(ResourceTiming {
            url: url_str,
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

    // ========================================
    // RED PHASE: Tests for HttpCache
    // ========================================

    #[test]
    fn test_http_cache_new() {
        let cache = HttpCache::new(100);
        assert_eq!(cache.current_size(), 0);
        assert_eq!(cache.max_size_bytes, 100 * 1024 * 1024);
    }

    #[test]
    fn test_http_cache_put_and_get() {
        let cache = HttpCache::new(100);
        let url = "https://example.com".to_string();
        let data = vec![1, 2, 3, 4, 5];

        cache.put(url.clone(), data.clone());

        let cached = cache.get(&url);
        assert!(cached.is_some());
        assert_eq!(cached.unwrap(), data);
    }

    #[test]
    fn test_http_cache_get_nonexistent() {
        let cache = HttpCache::new(100);
        let cached = cache.get("https://nonexistent.com");
        assert!(cached.is_none());
    }

    #[test]
    fn test_http_cache_size_tracking() {
        let cache = HttpCache::new(100);
        let data = vec![1, 2, 3, 4, 5]; // 5 bytes

        cache.put("https://example.com".to_string(), data.clone());
        assert_eq!(cache.current_size(), 5);

        cache.put("https://example2.com".to_string(), data.clone());
        assert_eq!(cache.current_size(), 10);
    }

    #[test]
    fn test_http_cache_lru_eviction() {
        let cache = HttpCache::new(1); // 1 MB = 1048576 bytes
        let large_data = vec![0u8; 700_000]; // 700 KB

        // Add first entry
        cache.put("https://first.com".to_string(), large_data.clone());
        assert!(cache.get("https://first.com").is_some());

        // Add second entry - should evict first
        cache.put("https://second.com".to_string(), large_data.clone());
        assert!(cache.get("https://second.com").is_some());
        assert!(cache.get("https://first.com").is_none()); // First should be evicted
    }

    #[test]
    fn test_http_cache_clear() {
        let cache = HttpCache::new(100);
        cache.put("https://example.com".to_string(), vec![1, 2, 3]);
        cache.put("https://example2.com".to_string(), vec![4, 5, 6]);

        assert_eq!(cache.current_size(), 6);

        cache.clear();
        assert_eq!(cache.current_size(), 0);
        assert!(cache.get("https://example.com").is_none());
        assert!(cache.get("https://example2.com").is_none());
    }

    #[test]
    fn test_http_cache_update_existing() {
        let cache = HttpCache::new(100);
        cache.put("https://example.com".to_string(), vec![1, 2, 3]);
        assert_eq!(cache.current_size(), 3);

        // Update with larger data
        cache.put("https://example.com".to_string(), vec![1, 2, 3, 4, 5]);
        assert_eq!(cache.current_size(), 5);

        let cached = cache.get("https://example.com").unwrap();
        assert_eq!(cached.len(), 5);
    }

    #[test]
    fn test_http_cache_too_large_entry() {
        let cache = HttpCache::new(1); // 1 MB
        let huge_data = vec![0u8; 2_000_000]; // 2 MB - larger than cache

        cache.put("https://example.com".to_string(), huge_data);

        // Entry should not be cached because it's larger than max cache size
        assert!(cache.get("https://example.com").is_none());
        assert_eq!(cache.current_size(), 0);
    }
}
