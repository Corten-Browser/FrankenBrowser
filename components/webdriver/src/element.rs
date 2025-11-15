//! WebDriver element handling and caching
//!
//! This module provides element reference management and caching for WebDriver protocol.
//! Elements are identified by UUIDs and cached to allow subsequent operations on them.

use crate::errors::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

/// Element reference per W3C WebDriver specification
///
/// Elements are identified by a UUID that maps to a cached element in the session.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ElementReference {
    /// Element UUID (unique identifier)
    pub element_id: String,
    /// Session ID this element belongs to
    pub session_id: String,
    /// CSS selector used to find this element
    pub selector: String,
    /// Index of this element (for findElements)
    pub index: usize,
}

impl ElementReference {
    /// Create a new element reference
    pub fn new(
        session_id: impl Into<String>,
        selector: impl Into<String>,
        index: usize,
    ) -> Self {
        Self {
            element_id: Uuid::new_v4().to_string(),
            session_id: session_id.into(),
            selector: selector.into(),
            index,
        }
    }

    /// Generate a new UUID for this element
    pub fn regenerate_id(&mut self) {
        self.element_id = Uuid::new_v4().to_string();
    }
}

/// Cached element with metadata
#[derive(Debug, Clone)]
pub struct CachedElement {
    /// Element reference
    pub reference: ElementReference,
    /// Element tag name (e.g., "button", "input")
    pub tag_name: Option<String>,
    /// Element attributes
    pub attributes: HashMap<String, String>,
    /// Whether element is stale (removed from DOM)
    pub is_stale: bool,
}

impl CachedElement {
    /// Create a new cached element
    pub fn new(reference: ElementReference) -> Self {
        Self {
            reference,
            tag_name: None,
            attributes: HashMap::new(),
            is_stale: false,
        }
    }

    /// Mark element as stale
    pub fn mark_stale(&mut self) {
        self.is_stale = true;
    }

    /// Check if element is valid (not stale)
    pub fn is_valid(&self) -> bool {
        !self.is_stale
    }
}

/// Element cache for managing element references in a session
///
/// The cache stores element references and provides lookup by UUID.
/// The cache is invalidated on navigation events to prevent stale element errors.
#[derive(Clone)]
pub struct ElementCache {
    /// Map of element_id -> CachedElement
    elements: Arc<Mutex<HashMap<String, CachedElement>>>,
}

impl ElementCache {
    /// Create a new empty element cache
    pub fn new() -> Self {
        Self {
            elements: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Generate a new element ID (UUID)
    pub fn new_element_id() -> String {
        Uuid::new_v4().to_string()
    }

    /// Cache an element with the given selector and index
    ///
    /// Returns the generated element reference
    pub fn cache_element(
        &self,
        session_id: &str,
        selector: &str,
        index: usize,
    ) -> ElementReference {
        let reference = ElementReference::new(session_id, selector, index);
        let cached = CachedElement::new(reference.clone());

        let mut elements = self.elements.lock().unwrap();
        elements.insert(reference.element_id.clone(), cached);

        reference
    }

    /// Get a cached element by ID
    ///
    /// Returns an error if the element is not found or is stale.
    pub fn get_element(&self, element_id: &str) -> Result<CachedElement> {
        let elements = self.elements.lock().unwrap();

        match elements.get(element_id) {
            Some(cached) => {
                if cached.is_stale {
                    Err(Error::StaleElementReference(element_id.to_string()))
                } else {
                    Ok(cached.clone())
                }
            }
            None => Err(Error::NoSuchElement(element_id.to_string())),
        }
    }

    /// Update a cached element's metadata
    pub fn update_element(
        &self,
        element_id: &str,
        tag_name: Option<String>,
        attributes: HashMap<String, String>,
    ) -> Result<()> {
        let mut elements = self.elements.lock().unwrap();

        match elements.get_mut(element_id) {
            Some(cached) => {
                cached.tag_name = tag_name;
                cached.attributes = attributes;
                Ok(())
            }
            None => Err(Error::NoSuchElement(element_id.to_string())),
        }
    }

    /// Mark an element as stale
    pub fn mark_stale(&self, element_id: &str) -> Result<()> {
        let mut elements = self.elements.lock().unwrap();

        match elements.get_mut(element_id) {
            Some(cached) => {
                cached.mark_stale();
                Ok(())
            }
            None => Err(Error::NoSuchElement(element_id.to_string())),
        }
    }

    /// Invalidate all cached elements (e.g., on navigation)
    ///
    /// This marks all elements as stale to prevent stale element errors
    /// when the DOM changes.
    pub fn invalidate_cache(&self) {
        let mut elements = self.elements.lock().unwrap();

        for cached in elements.values_mut() {
            cached.mark_stale();
        }
    }

    /// Clear all cached elements
    pub fn clear(&self) {
        let mut elements = self.elements.lock().unwrap();
        elements.clear();
    }

    /// Get the number of cached elements
    pub fn len(&self) -> usize {
        let elements = self.elements.lock().unwrap();
        elements.len()
    }

    /// Check if the cache is empty
    pub fn is_empty(&self) -> bool {
        let elements = self.elements.lock().unwrap();
        elements.is_empty()
    }
}

impl Default for ElementCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================
    // ElementReference Tests
    // ========================================

    #[test]
    fn test_element_reference_new() {
        let reference = ElementReference::new("session-123", "#button", 0);

        assert_eq!(reference.session_id, "session-123");
        assert_eq!(reference.selector, "#button");
        assert_eq!(reference.index, 0);
        assert!(!reference.element_id.is_empty());
        // UUID should be valid
        assert!(Uuid::parse_str(&reference.element_id).is_ok());
    }

    #[test]
    fn test_element_reference_regenerate_id() {
        let mut reference = ElementReference::new("session-123", "#button", 0);
        let original_id = reference.element_id.clone();

        reference.regenerate_id();

        assert_ne!(reference.element_id, original_id);
        assert!(Uuid::parse_str(&reference.element_id).is_ok());
    }

    #[test]
    fn test_element_reference_clone() {
        let reference = ElementReference::new("session-123", "#button", 0);
        let cloned = reference.clone();

        assert_eq!(reference.element_id, cloned.element_id);
        assert_eq!(reference.session_id, cloned.session_id);
        assert_eq!(reference.selector, cloned.selector);
    }

    // ========================================
    // CachedElement Tests
    // ========================================

    #[test]
    fn test_cached_element_new() {
        let reference = ElementReference::new("session-123", "#button", 0);
        let cached = CachedElement::new(reference.clone());

        assert_eq!(cached.reference.element_id, reference.element_id);
        assert_eq!(cached.tag_name, None);
        assert!(cached.attributes.is_empty());
        assert!(!cached.is_stale);
        assert!(cached.is_valid());
    }

    #[test]
    fn test_cached_element_mark_stale() {
        let reference = ElementReference::new("session-123", "#button", 0);
        let mut cached = CachedElement::new(reference);

        assert!(cached.is_valid());

        cached.mark_stale();

        assert!(cached.is_stale);
        assert!(!cached.is_valid());
    }

    // ========================================
    // ElementCache Tests
    // ========================================

    #[test]
    fn test_element_cache_new() {
        let cache = ElementCache::new();
        assert!(cache.is_empty());
        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn test_element_cache_new_element_id() {
        let id1 = ElementCache::new_element_id();
        let id2 = ElementCache::new_element_id();

        assert_ne!(id1, id2);
        assert!(Uuid::parse_str(&id1).is_ok());
        assert!(Uuid::parse_str(&id2).is_ok());
    }

    #[test]
    fn test_element_cache_cache_element() {
        let cache = ElementCache::new();

        let reference = cache.cache_element("session-123", "#button", 0);

        assert_eq!(reference.session_id, "session-123");
        assert_eq!(reference.selector, "#button");
        assert_eq!(reference.index, 0);
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn test_element_cache_get_element_success() {
        let cache = ElementCache::new();
        let reference = cache.cache_element("session-123", "#button", 0);

        let result = cache.get_element(&reference.element_id);

        assert!(result.is_ok());
        let cached = result.unwrap();
        assert_eq!(cached.reference.element_id, reference.element_id);
        assert!(!cached.is_stale);
    }

    #[test]
    fn test_element_cache_get_element_not_found() {
        let cache = ElementCache::new();

        let result = cache.get_element("nonexistent-id");

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), Error::NoSuchElement(_)));
    }

    #[test]
    fn test_element_cache_get_element_stale() {
        let cache = ElementCache::new();
        let reference = cache.cache_element("session-123", "#button", 0);

        // Mark element as stale
        cache.mark_stale(&reference.element_id).unwrap();

        // Getting stale element should fail
        let result = cache.get_element(&reference.element_id);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            Error::StaleElementReference(_)
        ));
    }

    #[test]
    fn test_element_cache_update_element() {
        let cache = ElementCache::new();
        let reference = cache.cache_element("session-123", "#button", 0);

        let mut attributes = HashMap::new();
        attributes.insert("id".to_string(), "submit-btn".to_string());

        let result = cache.update_element(
            &reference.element_id,
            Some("button".to_string()),
            attributes.clone(),
        );

        assert!(result.is_ok());

        let cached = cache.get_element(&reference.element_id).unwrap();
        assert_eq!(cached.tag_name, Some("button".to_string()));
        assert_eq!(cached.attributes.get("id"), Some(&"submit-btn".to_string()));
    }

    #[test]
    fn test_element_cache_update_element_not_found() {
        let cache = ElementCache::new();

        let result = cache.update_element("nonexistent-id", None, HashMap::new());

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), Error::NoSuchElement(_)));
    }

    #[test]
    fn test_element_cache_invalidate_cache() {
        let cache = ElementCache::new();

        let ref1 = cache.cache_element("session-123", "#button1", 0);
        let ref2 = cache.cache_element("session-123", "#button2", 0);

        assert_eq!(cache.len(), 2);

        // Invalidate all elements
        cache.invalidate_cache();

        // Cache still has elements, but they're stale
        assert_eq!(cache.len(), 2);

        // Getting elements should fail with stale error
        let result1 = cache.get_element(&ref1.element_id);
        let result2 = cache.get_element(&ref2.element_id);

        assert!(matches!(
            result1.unwrap_err(),
            Error::StaleElementReference(_)
        ));
        assert!(matches!(
            result2.unwrap_err(),
            Error::StaleElementReference(_)
        ));
    }

    #[test]
    fn test_element_cache_clear() {
        let cache = ElementCache::new();

        cache.cache_element("session-123", "#button1", 0);
        cache.cache_element("session-123", "#button2", 0);

        assert_eq!(cache.len(), 2);

        cache.clear();

        assert_eq!(cache.len(), 0);
        assert!(cache.is_empty());
    }

    #[test]
    fn test_element_cache_multiple_sessions() {
        let cache = ElementCache::new();

        let ref1 = cache.cache_element("session-1", "#button", 0);
        let ref2 = cache.cache_element("session-2", "#button", 0);

        assert_ne!(ref1.element_id, ref2.element_id);
        assert_eq!(ref1.session_id, "session-1");
        assert_eq!(ref2.session_id, "session-2");

        // Both elements should be retrievable
        assert!(cache.get_element(&ref1.element_id).is_ok());
        assert!(cache.get_element(&ref2.element_id).is_ok());
    }

    #[test]
    fn test_element_cache_thread_safety() {
        use std::sync::Arc;
        use std::thread;

        let cache = Arc::new(ElementCache::new());

        let handles: Vec<_> = (0..5)
            .map(|i| {
                let cache = Arc::clone(&cache);
                thread::spawn(move || {
                    let selector = format!("#button-{}", i);
                    cache.cache_element("session-123", &selector, i)
                })
            })
            .collect();

        let references: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();

        assert_eq!(cache.len(), 5);

        // All elements should be retrievable
        for reference in references {
            assert!(cache.get_element(&reference.element_id).is_ok());
        }
    }
}
