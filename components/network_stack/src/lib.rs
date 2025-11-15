//! HTTP client, request handling, cache, and cookies
//!
//! This is the network_stack component of the FrankenBrowser project.
//!
//! # Component Overview
//!
//! **Type**: library
//! **Level**: 1
//! **Token Budget**: 12000 tokens
//!
//! # Dependencies
//!
//! - shared_types
//! - message_bus
//! - config_manager
//!
//! # Features
//!
//! - **HTTP Client**: GET, POST, PUT, DELETE requests with timeout and compression support
//! - **HTTP Caching**: RFC 7234 compliant caching with LRU eviction
//!   - In-memory cache with configurable size limits
//!   - Optional disk-based cache using SQLite (planned)
//!   - Cache-Control header parsing (max-age, no-cache, no-store, must-revalidate)
//!   - ETag and Last-Modified support for conditional requests (planned)
//!   - Automatic cache invalidation on POST/PUT/DELETE requests
//! - **Cookie Management**: Automatic cookie store
//! - **Performance Tracking**: Resource timing data collection
//!
//! # Usage
//!
//! ```no_run
//! use network_stack::{NetworkStack, Result};
//! use config_manager::NetworkConfig;
//! use message_bus::MessageBus;
//! use url::Url;
//!
//! # async fn example() -> Result<()> {
//! // Create message bus and get sender
//! let mut bus = MessageBus::new();
//! bus.start().unwrap();
//! let sender = bus.sender();
//!
//! // Create network config with caching enabled
//! let config = NetworkConfig {
//!     max_connections_per_host: 6,
//!     timeout_seconds: 30,
//!     enable_cookies: true,
//!     enable_cache: true,
//!     cache_size_mb: 500,  // 500MB cache
//! };
//!
//! // Create and initialize network stack
//! let mut stack = NetworkStack::new(config, sender)?;
//! stack.initialize()?;
//!
//! // Fetch a resource (first request - network)
//! let url = Url::parse("https://example.com").unwrap();
//! let data = stack.fetch(url.clone()).await?;
//!
//! // Fetch again (cache hit - if cacheable)
//! let data2 = stack.fetch(url).await?;
//!
//! // Get timing data
//! let timings = stack.get_timing_data();
//! for timing in timings {
//!     println!("Fetched {} in {}ms (cache: {})",
//!              timing.url, timing.duration_ms, timing.from_cache);
//! }
//!
//! // Clear cache
//! stack.clear_cache();
//! # Ok(())
//! # }
//! ```

pub mod cache;
pub mod errors;
pub mod types;

// Re-export main types for convenience
pub use cache::{CacheControl, CacheEntry, HttpCache};
pub use errors::{Error, Result};
pub use types::{NetworkStack, ResourceTiming};

#[cfg(test)]
mod tests {
    use super::*;
    use config_manager::NetworkConfig;
    use message_bus::MessageBus;
    use url::Url;

    // Helper to create a test config
    fn test_config() -> NetworkConfig {
        NetworkConfig {
            max_connections_per_host: 6,
            timeout_seconds: 30,
            enable_cookies: true,
            enable_cache: true,
            cache_size_mb: 10, // Small cache for testing
        }
    }

    // Helper to create a test network stack
    fn test_stack() -> NetworkStack {
        let mut bus = MessageBus::new();
        bus.start().unwrap();
        let sender = bus.sender();
        NetworkStack::new(test_config(), sender).unwrap()
    }

    // ========================================
    // RED PHASE: Tests for NetworkStack creation
    // ========================================

    #[test]
    fn test_network_stack_new() {
        let stack = test_stack();
        // Should create successfully
        drop(stack);
    }

    #[test]
    fn test_network_stack_new_with_cache_disabled() {
        let mut bus = MessageBus::new();
        bus.start().unwrap();
        let sender = bus.sender();

        let mut config = test_config();
        config.enable_cache = false;

        let stack = NetworkStack::new(config, sender);
        assert!(stack.is_ok());
    }

    #[test]
    fn test_network_stack_new_with_cookies_disabled() {
        let mut bus = MessageBus::new();
        bus.start().unwrap();
        let sender = bus.sender();

        let mut config = test_config();
        config.enable_cookies = false;

        let stack = NetworkStack::new(config, sender);
        assert!(stack.is_ok());
    }

    // ========================================
    // RED PHASE: Tests for initialization
    // ========================================

    #[test]
    fn test_network_stack_initialize() {
        let mut stack = test_stack();
        let result = stack.initialize();
        assert!(result.is_ok());
    }

    #[test]
    fn test_network_stack_initialize_twice_fails() {
        let mut stack = test_stack();
        stack.initialize().unwrap();

        // Second initialization should fail
        let result = stack.initialize();
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), Error::InitializationError(_)));
    }

    #[test]
    fn test_network_stack_fetch_without_initialize_fails() {
        let stack = test_stack();
        let url = Url::parse("https://example.com").unwrap();

        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(async { stack.fetch(url).await });

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), Error::InitializationError(_)));
    }

    // ========================================
    // RED PHASE: Tests for timing data
    // ========================================

    #[test]
    fn test_network_stack_get_timing_data_empty() {
        let stack = test_stack();
        let timings = stack.get_timing_data();
        assert_eq!(timings.len(), 0);
    }

    #[test]
    fn test_network_stack_clear_timing_data() {
        let mut stack = test_stack();
        stack.clear_timing_data();
        let timings = stack.get_timing_data();
        assert_eq!(timings.len(), 0);
    }

    // ========================================
    // RED PHASE: Tests for cache management
    // ========================================

    #[test]
    fn test_network_stack_clear_cache() {
        let mut stack = test_stack();
        stack.initialize().unwrap();
        stack.clear_cache();
        // Should not panic
    }

    #[test]
    fn test_network_stack_clear_cache_when_disabled() {
        let mut bus = MessageBus::new();
        bus.start().unwrap();
        let sender = bus.sender();

        let mut config = test_config();
        config.enable_cache = false;

        let mut stack = NetworkStack::new(config, sender).unwrap();
        stack.initialize().unwrap();
        stack.clear_cache();
        // Should not panic even with cache disabled
    }

    // ========================================
    // Integration tests (require network)
    // ========================================

    #[tokio::test]
    #[ignore] // Ignored by default - requires network access
    async fn test_network_stack_fetch_real_url() {
        let mut stack = test_stack();
        stack.initialize().unwrap();

        let url = Url::parse("https://httpbin.org/get").unwrap();
        let result = stack.fetch(url).await;

        assert!(result.is_ok());
        let data = result.unwrap();
        assert!(!data.is_empty());

        // Check timing was recorded
        let timings = stack.get_timing_data();
        assert_eq!(timings.len(), 1);
        assert!(!timings[0].from_cache);
        assert!(timings[0].duration_ms > 0);
    }

    #[tokio::test]
    #[ignore] // Ignored by default - requires network access
    async fn test_network_stack_cache_hit() {
        let mut stack = test_stack();
        stack.initialize().unwrap();

        let url = Url::parse("https://httpbin.org/get").unwrap();

        // First fetch - should go to network
        let result1 = stack.fetch(url.clone()).await;
        assert!(result1.is_ok());

        // Second fetch - should come from cache
        let result2 = stack.fetch(url.clone()).await;
        assert!(result2.is_ok());

        let timings = stack.get_timing_data();
        assert_eq!(timings.len(), 2);
        assert!(!timings[0].from_cache); // First request
        assert!(timings[1].from_cache); // Second request (cached)
        assert!(timings[1].duration_ms < timings[0].duration_ms); // Cache should be faster
    }

    #[tokio::test]
    #[ignore] // Ignored by default - requires network access
    async fn test_network_stack_fetch_invalid_url() {
        let mut stack = test_stack();
        stack.initialize().unwrap();

        let url = Url::parse("https://this-domain-definitely-does-not-exist-12345.com").unwrap();
        let result = stack.fetch(url).await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), Error::RequestFailed(_)));
    }

    #[tokio::test]
    #[ignore] // Ignored by default - requires network access
    async fn test_network_stack_fetch_404() {
        let mut stack = test_stack();
        stack.initialize().unwrap();

        let url = Url::parse("https://httpbin.org/status/404").unwrap();
        let result = stack.fetch(url).await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), Error::RequestFailed(_)));
    }
}
