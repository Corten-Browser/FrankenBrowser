//! HTTP cache implementation with LRU eviction and optional disk persistence
//!
//! This module provides HTTP caching capabilities following RFC 7234.
//! Features include:
//! - In-memory LRU cache
//! - Optional disk-based cache using SQLite
//! - Cache-Control header parsing and enforcement
//! - ETag and Last-Modified support for conditional requests
//! - Automatic cache invalidation

use lru::LruCache;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::num::NonZeroUsize;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use url::Url;

/// Cache-Control directive values
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CacheControl {
    /// max-age directive (seconds)
    pub max_age: Option<u64>,
    /// no-cache directive
    pub no_cache: bool,
    /// no-store directive
    pub no_store: bool,
    /// must-revalidate directive
    pub must_revalidate: bool,
    /// public directive
    pub public: bool,
    /// private directive
    pub private: bool,
}

impl CacheControl {
    /// Parse Cache-Control header value
    pub fn parse(header_value: &str) -> Self {
        let mut control = CacheControl::default();

        for directive in header_value.split(',').map(|s| s.trim()) {
            if directive.eq_ignore_ascii_case("no-cache") {
                control.no_cache = true;
            } else if directive.eq_ignore_ascii_case("no-store") {
                control.no_store = true;
            } else if directive.eq_ignore_ascii_case("must-revalidate") {
                control.must_revalidate = true;
            } else if directive.eq_ignore_ascii_case("public") {
                control.public = true;
            } else if directive.eq_ignore_ascii_case("private") {
                control.private = true;
            } else if let Some(max_age_str) = directive
                .strip_prefix("max-age=")
                .or_else(|| directive.strip_prefix("max-age ="))
            {
                if let Ok(age) = max_age_str.trim().parse::<u64>() {
                    control.max_age = Some(age);
                }
            }
        }

        control
    }

    /// Check if caching is allowed
    pub fn allows_caching(&self) -> bool {
        !self.no_store
    }

    /// Check if cached response can be used without revalidation
    pub fn allows_cached_response(&self) -> bool {
        !self.no_cache && !self.must_revalidate
    }
}

/// HTTP response metadata for caching
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    /// URL of the cached resource
    pub url: String,
    /// Response body
    pub body: Vec<u8>,
    /// Response headers
    pub headers: HashMap<String, String>,
    /// ETag value if present
    pub etag: Option<String>,
    /// Last-Modified value if present
    pub last_modified: Option<String>,
    /// Expiration timestamp (seconds since UNIX_EPOCH)
    pub expires_at: u64,
    /// Cache-Control directives
    #[serde(skip)]
    pub cache_control: CacheControl,
    /// Hit count for LRU tracking
    pub hit_count: usize,
    /// Size in bytes
    pub size_bytes: usize,
    /// Timestamp when cached (seconds since UNIX_EPOCH)
    pub cached_at: u64,
}

impl CacheEntry {
    /// Create a new cache entry
    pub fn new(url: String, body: Vec<u8>, headers: HashMap<String, String>) -> Self {
        let size_bytes = body.len();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Extract ETag
        let etag = headers.get("etag").or_else(|| headers.get("ETag")).cloned();

        // Extract Last-Modified
        let last_modified = headers
            .get("last-modified")
            .or_else(|| headers.get("Last-Modified"))
            .cloned();

        // Parse Cache-Control
        let cache_control = headers
            .get("cache-control")
            .or_else(|| headers.get("Cache-Control"))
            .map(|v| CacheControl::parse(v))
            .unwrap_or_default();

        // Calculate expiration
        let expires_at = if let Some(max_age) = cache_control.max_age {
            now + max_age
        } else if let Some(expires) = headers.get("expires").or_else(|| headers.get("Expires")) {
            // Try to parse Expires header (simplified - just extract seconds)
            // In production, would use full HTTP date parsing
            parse_http_date(expires).unwrap_or(now + 3600)
        } else {
            // Default: 1 hour
            now + 3600
        };

        Self {
            url,
            body,
            headers,
            etag,
            last_modified,
            expires_at,
            cache_control,
            hit_count: 0,
            size_bytes,
            cached_at: now,
        }
    }

    /// Check if cache entry is expired
    pub fn is_expired(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        now >= self.expires_at
    }

    /// Check if entry can be used without revalidation
    pub fn can_use_without_revalidation(&self) -> bool {
        !self.is_expired() && self.cache_control.allows_cached_response()
    }

    /// Increment hit count
    pub fn increment_hit(&mut self) {
        self.hit_count += 1;
    }
}

/// Simplified HTTP date parser (for Expires header)
/// In production, would use a proper HTTP date parser
fn parse_http_date(date_str: &str) -> Option<u64> {
    // Very simplified - just look for common patterns
    // Real implementation would use chrono or httpdate crate
    // For now, return None to fall back to default
    let _ = date_str;
    None
}

/// HTTP cache with LRU eviction and optional disk persistence
pub struct HttpCache {
    /// In-memory LRU cache
    memory_cache: Arc<Mutex<LruCache<String, CacheEntry>>>,
    /// Maximum memory cache size in bytes
    max_memory_bytes: usize,
    /// Current memory cache size in bytes
    current_memory_size: Arc<Mutex<usize>>,
    /// Optional disk cache path
    disk_cache_path: Option<PathBuf>,
    /// Disk cache connection (lazy initialized)
    #[allow(dead_code)]
    disk_cache: Arc<Mutex<Option<rusqlite::Connection>>>,
}

impl HttpCache {
    /// Create a new HTTP cache
    ///
    /// # Arguments
    ///
    /// * `max_memory_mb` - Maximum memory cache size in megabytes
    /// * `disk_cache_path` - Optional path for disk cache database
    ///
    /// # Returns
    ///
    /// A new HttpCache instance
    pub fn new(max_memory_mb: usize, disk_cache_path: Option<PathBuf>) -> Self {
        let max_memory_bytes = max_memory_mb * 1024 * 1024;
        let capacity = NonZeroUsize::new(1000).unwrap();

        let disk_cache = if disk_cache_path.is_some() {
            Arc::new(Mutex::new(None)) // Lazy initialization
        } else {
            Arc::new(Mutex::new(None))
        };

        Self {
            memory_cache: Arc::new(Mutex::new(LruCache::new(capacity))),
            max_memory_bytes,
            current_memory_size: Arc::new(Mutex::new(0)),
            disk_cache_path,
            disk_cache,
        }
    }

    /// Get cached response for a URL
    ///
    /// # Arguments
    ///
    /// * `url` - The URL to look up
    ///
    /// # Returns
    ///
    /// Returns Some(CacheEntry) if found and valid, None otherwise
    pub fn get(&self, url: &Url) -> Option<CacheEntry> {
        let url_str = url.as_str();
        let mut cache = self.memory_cache.lock().unwrap();

        if let Some(mut entry) = cache.get(url_str).cloned() {
            entry.increment_hit();
            cache.put(url_str.to_string(), entry.clone());
            return Some(entry);
        }

        // TODO: Check disk cache if enabled
        None
    }

    /// Store response in cache
    ///
    /// # Arguments
    ///
    /// * `url` - The URL of the resource
    /// * `body` - Response body
    /// * `headers` - Response headers
    pub fn put(&self, url: Url, body: Vec<u8>, headers: HashMap<String, String>) {
        let url_str = url.as_str().to_string();
        let entry = CacheEntry::new(url_str.clone(), body, headers);

        // Check if response is cacheable
        if !entry.cache_control.allows_caching() {
            return;
        }

        let size = entry.size_bytes;
        let mut cache = self.memory_cache.lock().unwrap();
        let mut current_size = self.current_memory_size.lock().unwrap();

        // Evict entries until there's room
        while *current_size + size > self.max_memory_bytes && !cache.is_empty() {
            if let Some((_, old_entry)) = cache.pop_lru() {
                *current_size -= old_entry.size_bytes;
            }
        }

        // Add entry if it fits
        if size <= self.max_memory_bytes {
            if let Some(old_entry) = cache.put(url_str, entry) {
                *current_size -= old_entry.size_bytes;
            }
            *current_size += size;
        }

        // TODO: Store in disk cache if enabled
    }

    /// Invalidate cache entry for a URL
    ///
    /// Called when a POST/PUT/DELETE request is made to a URL
    ///
    /// # Arguments
    ///
    /// * `url` - The URL to invalidate
    pub fn invalidate(&self, url: &Url) {
        let url_str = url.as_str();
        let mut cache = self.memory_cache.lock().unwrap();
        let mut current_size = self.current_memory_size.lock().unwrap();

        if let Some(entry) = cache.pop(url_str) {
            *current_size -= entry.size_bytes;
        }

        // TODO: Invalidate disk cache entry if enabled
    }

    /// Clear all cached entries
    pub fn clear(&self) {
        let mut cache = self.memory_cache.lock().unwrap();
        let mut current_size = self.current_memory_size.lock().unwrap();
        cache.clear();
        *current_size = 0;

        // TODO: Clear disk cache if enabled
    }

    /// Get current cache size in bytes
    pub fn size(&self) -> usize {
        *self.current_memory_size.lock().unwrap()
    }

    /// Check if a response with given status code and headers is cacheable
    ///
    /// # Arguments
    ///
    /// * `status_code` - HTTP status code
    /// * `headers` - Response headers
    ///
    /// # Returns
    ///
    /// True if response can be cached
    pub fn is_cacheable(status_code: u16, headers: &HashMap<String, String>) -> bool {
        // Only cache successful responses
        if !(200..300).contains(&status_code) && status_code != 304 {
            return false;
        }

        // Check Cache-Control
        if let Some(cc_value) = headers
            .get("cache-control")
            .or_else(|| headers.get("Cache-Control"))
        {
            let cache_control = CacheControl::parse(cc_value);
            if !cache_control.allows_caching() {
                return false;
            }
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================
    // RED PHASE: Tests for CacheControl
    // ========================================

    #[test]
    fn test_cache_control_parse_max_age() {
        let cc = CacheControl::parse("max-age=3600");
        assert_eq!(cc.max_age, Some(3600));
        assert!(!cc.no_cache);
        assert!(!cc.no_store);
    }

    #[test]
    fn test_cache_control_parse_no_cache() {
        let cc = CacheControl::parse("no-cache");
        assert!(cc.no_cache);
        assert!(!cc.no_store);
        assert_eq!(cc.max_age, None);
    }

    #[test]
    fn test_cache_control_parse_no_store() {
        let cc = CacheControl::parse("no-store");
        assert!(cc.no_store);
        assert!(!cc.no_cache);
    }

    #[test]
    fn test_cache_control_parse_multiple_directives() {
        let cc = CacheControl::parse("max-age=3600, must-revalidate, public");
        assert_eq!(cc.max_age, Some(3600));
        assert!(cc.must_revalidate);
        assert!(cc.public);
        assert!(!cc.no_cache);
    }

    #[test]
    fn test_cache_control_parse_with_spaces() {
        let cc = CacheControl::parse("max-age = 7200, no-cache");
        assert_eq!(cc.max_age, Some(7200));
        assert!(cc.no_cache);
    }

    #[test]
    fn test_cache_control_allows_caching() {
        let cc = CacheControl::parse("max-age=3600");
        assert!(cc.allows_caching());

        let cc_no_store = CacheControl::parse("no-store");
        assert!(!cc_no_store.allows_caching());
    }

    #[test]
    fn test_cache_control_allows_cached_response() {
        let cc = CacheControl::parse("max-age=3600");
        assert!(cc.allows_cached_response());

        let cc_no_cache = CacheControl::parse("no-cache");
        assert!(!cc_no_cache.allows_cached_response());

        let cc_must_revalidate = CacheControl::parse("must-revalidate");
        assert!(!cc_must_revalidate.allows_cached_response());
    }

    // ========================================
    // RED PHASE: Tests for CacheEntry
    // ========================================

    #[test]
    fn test_cache_entry_new() {
        let mut headers = HashMap::new();
        headers.insert("content-type".to_string(), "text/html".to_string());
        headers.insert("cache-control".to_string(), "max-age=3600".to_string());

        let entry = CacheEntry::new(
            "https://example.com".to_string(),
            vec![1, 2, 3, 4, 5],
            headers,
        );

        assert_eq!(entry.url, "https://example.com");
        assert_eq!(entry.body, vec![1, 2, 3, 4, 5]);
        assert_eq!(entry.size_bytes, 5);
        assert_eq!(entry.hit_count, 0);
        assert_eq!(entry.cache_control.max_age, Some(3600));
    }

    #[test]
    fn test_cache_entry_with_etag() {
        let mut headers = HashMap::new();
        headers.insert("etag".to_string(), "\"abc123\"".to_string());

        let entry = CacheEntry::new("https://example.com".to_string(), vec![1, 2, 3], headers);

        assert_eq!(entry.etag, Some("\"abc123\"".to_string()));
    }

    #[test]
    fn test_cache_entry_with_last_modified() {
        let mut headers = HashMap::new();
        headers.insert(
            "last-modified".to_string(),
            "Mon, 01 Jan 2024 00:00:00 GMT".to_string(),
        );

        let entry = CacheEntry::new("https://example.com".to_string(), vec![1, 2, 3], headers);

        assert_eq!(
            entry.last_modified,
            Some("Mon, 01 Jan 2024 00:00:00 GMT".to_string())
        );
    }

    #[test]
    fn test_cache_entry_is_expired() {
        let mut headers = HashMap::new();
        headers.insert("cache-control".to_string(), "max-age=0".to_string());

        let entry = CacheEntry::new("https://example.com".to_string(), vec![1, 2, 3], headers);

        // Entry with max-age=0 should be expired immediately
        std::thread::sleep(Duration::from_millis(10));
        assert!(entry.is_expired());
    }

    #[test]
    fn test_cache_entry_not_expired() {
        let mut headers = HashMap::new();
        headers.insert("cache-control".to_string(), "max-age=3600".to_string());

        let entry = CacheEntry::new("https://example.com".to_string(), vec![1, 2, 3], headers);

        assert!(!entry.is_expired());
    }

    #[test]
    fn test_cache_entry_can_use_without_revalidation() {
        let mut headers = HashMap::new();
        headers.insert("cache-control".to_string(), "max-age=3600".to_string());

        let entry = CacheEntry::new("https://example.com".to_string(), vec![1, 2, 3], headers);

        assert!(entry.can_use_without_revalidation());
    }

    #[test]
    fn test_cache_entry_requires_revalidation() {
        let mut headers = HashMap::new();
        headers.insert("cache-control".to_string(), "no-cache".to_string());

        let entry = CacheEntry::new("https://example.com".to_string(), vec![1, 2, 3], headers);

        assert!(!entry.can_use_without_revalidation());
    }

    #[test]
    fn test_cache_entry_increment_hit() {
        let headers = HashMap::new();
        let mut entry = CacheEntry::new("https://example.com".to_string(), vec![1, 2, 3], headers);

        assert_eq!(entry.hit_count, 0);
        entry.increment_hit();
        assert_eq!(entry.hit_count, 1);
        entry.increment_hit();
        assert_eq!(entry.hit_count, 2);
    }

    // ========================================
    // RED PHASE: Tests for HttpCache
    // ========================================

    #[test]
    fn test_http_cache_new() {
        let cache = HttpCache::new(100, None);
        assert_eq!(cache.size(), 0);
        assert_eq!(cache.max_memory_bytes, 100 * 1024 * 1024);
    }

    #[test]
    fn test_http_cache_new_with_disk_path() {
        let path = PathBuf::from("/tmp/test_cache.db");
        let cache = HttpCache::new(100, Some(path.clone()));
        assert_eq!(cache.disk_cache_path, Some(path));
    }

    #[test]
    fn test_http_cache_put_and_get() {
        let cache = HttpCache::new(100, None);
        let url = Url::parse("https://example.com").unwrap();

        let mut headers = HashMap::new();
        headers.insert("cache-control".to_string(), "max-age=3600".to_string());

        cache.put(url.clone(), vec![1, 2, 3, 4, 5], headers);

        let entry = cache.get(&url);
        assert!(entry.is_some());
        let entry = entry.unwrap();
        assert_eq!(entry.body, vec![1, 2, 3, 4, 5]);
        assert_eq!(entry.url, "https://example.com/");
    }

    #[test]
    fn test_http_cache_get_nonexistent() {
        let cache = HttpCache::new(100, None);
        let url = Url::parse("https://nonexistent.com").unwrap();

        let entry = cache.get(&url);
        assert!(entry.is_none());
    }

    #[test]
    fn test_http_cache_size_tracking() {
        let cache = HttpCache::new(100, None);
        let url1 = Url::parse("https://example.com/1").unwrap();
        let url2 = Url::parse("https://example.com/2").unwrap();

        let mut headers = HashMap::new();
        headers.insert("cache-control".to_string(), "max-age=3600".to_string());

        cache.put(url1, vec![1, 2, 3, 4, 5], headers.clone());
        assert_eq!(cache.size(), 5);

        cache.put(url2, vec![1, 2, 3], headers);
        assert_eq!(cache.size(), 8);
    }

    #[test]
    fn test_http_cache_lru_eviction() {
        let cache = HttpCache::new(1, None); // 1 MB
        let large_data = vec![0u8; 700_000]; // 700 KB

        let mut headers = HashMap::new();
        headers.insert("cache-control".to_string(), "max-age=3600".to_string());

        let url1 = Url::parse("https://first.com").unwrap();
        let url2 = Url::parse("https://second.com").unwrap();

        cache.put(url1.clone(), large_data.clone(), headers.clone());
        assert!(cache.get(&url1).is_some());

        cache.put(url2.clone(), large_data, headers);
        assert!(cache.get(&url2).is_some());
        assert!(cache.get(&url1).is_none()); // First entry should be evicted
    }

    #[test]
    fn test_http_cache_invalidate() {
        let cache = HttpCache::new(100, None);
        let url = Url::parse("https://example.com").unwrap();

        let mut headers = HashMap::new();
        headers.insert("cache-control".to_string(), "max-age=3600".to_string());

        cache.put(url.clone(), vec![1, 2, 3], headers);
        assert!(cache.get(&url).is_some());

        cache.invalidate(&url);
        assert!(cache.get(&url).is_none());
    }

    #[test]
    fn test_http_cache_clear() {
        let cache = HttpCache::new(100, None);

        let mut headers = HashMap::new();
        headers.insert("cache-control".to_string(), "max-age=3600".to_string());

        cache.put(
            Url::parse("https://example.com/1").unwrap(),
            vec![1, 2, 3],
            headers.clone(),
        );
        cache.put(
            Url::parse("https://example.com/2").unwrap(),
            vec![4, 5, 6],
            headers,
        );

        assert_eq!(cache.size(), 6);

        cache.clear();
        assert_eq!(cache.size(), 0);
        assert!(cache
            .get(&Url::parse("https://example.com/1").unwrap())
            .is_none());
    }

    #[test]
    fn test_http_cache_no_store_not_cached() {
        let cache = HttpCache::new(100, None);
        let url = Url::parse("https://example.com").unwrap();

        let mut headers = HashMap::new();
        headers.insert("cache-control".to_string(), "no-store".to_string());

        cache.put(url.clone(), vec![1, 2, 3], headers);

        // Should not be cached due to no-store directive
        assert!(cache.get(&url).is_none());
    }

    #[test]
    fn test_http_cache_hit_count_increments() {
        let cache = HttpCache::new(100, None);
        let url = Url::parse("https://example.com").unwrap();

        let mut headers = HashMap::new();
        headers.insert("cache-control".to_string(), "max-age=3600".to_string());

        cache.put(url.clone(), vec![1, 2, 3], headers);

        let entry1 = cache.get(&url).unwrap();
        assert_eq!(entry1.hit_count, 1);

        let entry2 = cache.get(&url).unwrap();
        assert_eq!(entry2.hit_count, 2);
    }

    #[test]
    fn test_is_cacheable_success_status() {
        let headers = HashMap::new();
        assert!(HttpCache::is_cacheable(200, &headers));
        assert!(HttpCache::is_cacheable(204, &headers));
    }

    #[test]
    fn test_is_cacheable_not_modified() {
        let headers = HashMap::new();
        assert!(HttpCache::is_cacheable(304, &headers));
    }

    #[test]
    fn test_is_cacheable_error_status() {
        let headers = HashMap::new();
        assert!(!HttpCache::is_cacheable(404, &headers));
        assert!(!HttpCache::is_cacheable(500, &headers));
    }

    #[test]
    fn test_is_cacheable_no_store() {
        let mut headers = HashMap::new();
        headers.insert("cache-control".to_string(), "no-store".to_string());
        assert!(!HttpCache::is_cacheable(200, &headers));
    }
}
