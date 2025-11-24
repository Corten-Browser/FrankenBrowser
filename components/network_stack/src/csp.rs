//! Content Security Policy (CSP) Implementation
//!
//! This module implements CSP header parsing and enforcement as specified in:
//! https://www.w3.org/TR/CSP3/
//!
//! # Features
//!
//! - CSP header parsing (Content-Security-Policy, Content-Security-Policy-Report-Only)
//! - Directive validation (script-src, style-src, default-src, img-src, etc.)
//! - Source expression parsing ('self', 'unsafe-inline', 'unsafe-eval', URLs, etc.)
//! - Resource validation against CSP policies

use crate::errors::Result;
use crate::request_handler::{Request, RequestInterceptor, Response};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use url::Url;

/// CSP directive types
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CspDirective {
    /// Default source for all resource types
    DefaultSrc,
    /// Sources for scripts
    ScriptSrc,
    /// Sources for stylesheets
    StyleSrc,
    /// Sources for images
    ImgSrc,
    /// Sources for fonts
    FontSrc,
    /// Sources for media (audio, video)
    MediaSrc,
    /// Sources for connections (fetch, WebSocket, EventSource)
    ConnectSrc,
    /// Sources for frames
    FrameSrc,
    /// Sources for objects (plugins)
    ObjectSrc,
    /// Base URI for relative URLs
    BaseUri,
    /// Form action targets
    FormAction,
    /// Frame ancestors
    FrameAncestors,
    /// Upgrade insecure requests
    UpgradeInsecureRequests,
    /// Block all mixed content
    BlockAllMixedContent,
    /// Report URI (deprecated, use report-to)
    ReportUri,
    /// Report endpoint
    ReportTo,
    /// Unknown directive
    Unknown(String),
}

impl CspDirective {
    /// Parse a directive name from string
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "default-src" => CspDirective::DefaultSrc,
            "script-src" => CspDirective::ScriptSrc,
            "style-src" => CspDirective::StyleSrc,
            "img-src" => CspDirective::ImgSrc,
            "font-src" => CspDirective::FontSrc,
            "media-src" => CspDirective::MediaSrc,
            "connect-src" => CspDirective::ConnectSrc,
            "frame-src" => CspDirective::FrameSrc,
            "object-src" => CspDirective::ObjectSrc,
            "base-uri" => CspDirective::BaseUri,
            "form-action" => CspDirective::FormAction,
            "frame-ancestors" => CspDirective::FrameAncestors,
            "upgrade-insecure-requests" => CspDirective::UpgradeInsecureRequests,
            "block-all-mixed-content" => CspDirective::BlockAllMixedContent,
            "report-uri" => CspDirective::ReportUri,
            "report-to" => CspDirective::ReportTo,
            _ => CspDirective::Unknown(s.to_string()),
        }
    }

    /// Get the directive name as a string
    pub fn as_str(&self) -> &str {
        match self {
            CspDirective::DefaultSrc => "default-src",
            CspDirective::ScriptSrc => "script-src",
            CspDirective::StyleSrc => "style-src",
            CspDirective::ImgSrc => "img-src",
            CspDirective::FontSrc => "font-src",
            CspDirective::MediaSrc => "media-src",
            CspDirective::ConnectSrc => "connect-src",
            CspDirective::FrameSrc => "frame-src",
            CspDirective::ObjectSrc => "object-src",
            CspDirective::BaseUri => "base-uri",
            CspDirective::FormAction => "form-action",
            CspDirective::FrameAncestors => "frame-ancestors",
            CspDirective::UpgradeInsecureRequests => "upgrade-insecure-requests",
            CspDirective::BlockAllMixedContent => "block-all-mixed-content",
            CspDirective::ReportUri => "report-uri",
            CspDirective::ReportTo => "report-to",
            CspDirective::Unknown(s) => s,
        }
    }
}

/// CSP source expression types
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CspSource {
    /// 'self' - same origin
    Self_,
    /// 'unsafe-inline' - allow inline scripts/styles
    UnsafeInline,
    /// 'unsafe-eval' - allow eval()
    UnsafeEval,
    /// 'strict-dynamic' - trust scripts loaded by trusted scripts
    StrictDynamic,
    /// 'none' - block all
    None,
    /// 'wasm-unsafe-eval' - allow WebAssembly
    WasmUnsafeEval,
    /// Specific URL scheme (https:, data:, blob:)
    Scheme(String),
    /// Specific host (example.com, *.example.com)
    Host(String),
    /// Nonce value ('nonce-abc123')
    Nonce(String),
    /// Hash value ('sha256-abc...')
    Hash { algorithm: String, value: String },
}

impl CspSource {
    /// Parse a source expression from string
    pub fn from_str(s: &str) -> Self {
        let s_lower = s.to_lowercase();

        if s_lower == "'self'" {
            CspSource::Self_
        } else if s_lower == "'unsafe-inline'" {
            CspSource::UnsafeInline
        } else if s_lower == "'unsafe-eval'" {
            CspSource::UnsafeEval
        } else if s_lower == "'strict-dynamic'" {
            CspSource::StrictDynamic
        } else if s_lower == "'none'" {
            CspSource::None
        } else if s_lower == "'wasm-unsafe-eval'" {
            CspSource::WasmUnsafeEval
        } else if s_lower.starts_with("'nonce-") && s_lower.ends_with('\'') {
            let nonce = &s[7..s.len() - 1];
            CspSource::Nonce(nonce.to_string())
        } else if s_lower.starts_with("'sha256-") || s_lower.starts_with("'sha384-") || s_lower.starts_with("'sha512-") {
            if let Some(dash_pos) = s[1..].find('-') {
                let algorithm = &s[1..dash_pos + 1];
                let value = &s[dash_pos + 2..s.len() - 1];
                CspSource::Hash {
                    algorithm: algorithm.to_string(),
                    value: value.to_string(),
                }
            } else {
                CspSource::Host(s.to_string())
            }
        } else if s.ends_with(':') {
            CspSource::Scheme(s[..s.len() - 1].to_string())
        } else {
            CspSource::Host(s.to_string())
        }
    }

    /// Check if this source matches a URL
    pub fn matches(&self, url: &Url, document_origin: Option<&Url>) -> bool {
        match self {
            CspSource::Self_ => {
                if let Some(origin) = document_origin {
                    url.scheme() == origin.scheme()
                        && url.host() == origin.host()
                        && url.port() == origin.port()
                } else {
                    false
                }
            }
            CspSource::UnsafeInline => {
                // This source allows inline scripts/styles, not URL matching
                false
            }
            CspSource::UnsafeEval => {
                // This source allows eval(), not URL matching
                false
            }
            CspSource::StrictDynamic => {
                // Dynamic trust propagation, not direct URL matching
                false
            }
            CspSource::None => false,
            CspSource::WasmUnsafeEval => {
                // Allows WASM, not URL matching
                false
            }
            CspSource::Scheme(scheme) => url.scheme() == scheme,
            CspSource::Host(pattern) => {
                if let Some(url_host) = url.host_str() {
                    if pattern.starts_with("*.") {
                        // Wildcard subdomain matching - *.example.com matches sub.example.com
                        // but NOT example.com itself
                        let domain = &pattern[2..];
                        if url_host == domain {
                            // Exact match to base domain - not allowed by wildcard
                            false
                        } else if url_host.ends_with(domain) {
                            // Check that there's a dot before the domain part
                            let prefix_len = url_host.len() - domain.len();
                            prefix_len > 0 && url_host.chars().nth(prefix_len - 1) == Some('.')
                        } else {
                            false
                        }
                    } else {
                        url_host == pattern
                    }
                } else {
                    false
                }
            }
            CspSource::Nonce(_) => {
                // Nonce matching requires script/style element context
                false
            }
            CspSource::Hash { .. } => {
                // Hash matching requires content hashing
                false
            }
        }
    }
}

/// Resource type for CSP checking
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceType {
    Script,
    Style,
    Image,
    Font,
    Media,
    Connect,
    Frame,
    Object,
    Other,
}

impl ResourceType {
    /// Get the appropriate CSP directive for this resource type
    pub fn directive(&self) -> CspDirective {
        match self {
            ResourceType::Script => CspDirective::ScriptSrc,
            ResourceType::Style => CspDirective::StyleSrc,
            ResourceType::Image => CspDirective::ImgSrc,
            ResourceType::Font => CspDirective::FontSrc,
            ResourceType::Media => CspDirective::MediaSrc,
            ResourceType::Connect => CspDirective::ConnectSrc,
            ResourceType::Frame => CspDirective::FrameSrc,
            ResourceType::Object => CspDirective::ObjectSrc,
            ResourceType::Other => CspDirective::DefaultSrc,
        }
    }
}

/// Parsed Content Security Policy
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ContentSecurityPolicy {
    /// Directives and their source lists
    pub directives: HashMap<CspDirective, Vec<CspSource>>,
    /// Whether this is a report-only policy
    pub report_only: bool,
    /// Original header value
    pub raw: String,
}

impl ContentSecurityPolicy {
    /// Parse a CSP header value
    ///
    /// # Arguments
    ///
    /// * `header_value` - The Content-Security-Policy header value
    /// * `report_only` - Whether this is a report-only policy
    ///
    /// # Returns
    ///
    /// Returns a parsed ContentSecurityPolicy
    pub fn parse(header_value: &str, report_only: bool) -> Self {
        let mut directives = HashMap::new();

        // Split by semicolon to get individual directives
        for directive_str in header_value.split(';') {
            let directive_str = directive_str.trim();
            if directive_str.is_empty() {
                continue;
            }

            // Split directive into name and values
            let mut parts = directive_str.split_whitespace();
            if let Some(name) = parts.next() {
                let directive = CspDirective::from_str(name);
                let sources: Vec<CspSource> = parts
                    .map(|s| CspSource::from_str(s))
                    .collect();

                directives.insert(directive, sources);
            }
        }

        Self {
            directives,
            report_only,
            raw: header_value.to_string(),
        }
    }

    /// Check if a resource URL is allowed by this policy
    ///
    /// # Arguments
    ///
    /// * `url` - The resource URL to check
    /// * `resource_type` - The type of resource
    /// * `document_origin` - The origin of the document (for 'self' matching)
    ///
    /// # Returns
    ///
    /// Returns true if the resource is allowed, false if blocked
    pub fn allows(&self, url: &Url, resource_type: ResourceType, document_origin: Option<&Url>) -> bool {
        // Get the appropriate directive for this resource type
        let directive = resource_type.directive();

        // First check specific directive, fall back to default-src
        let sources = self.directives.get(&directive)
            .or_else(|| self.directives.get(&CspDirective::DefaultSrc));

        match sources {
            None => {
                // No policy for this resource type, allow by default
                true
            }
            Some(source_list) => {
                // Check if 'none' is present (blocks everything)
                if source_list.iter().any(|s| matches!(s, CspSource::None)) {
                    return false;
                }

                // Check if any source matches
                source_list.iter().any(|source| source.matches(url, document_origin))
            }
        }
    }

    /// Check if inline scripts are allowed
    pub fn allows_inline_script(&self) -> bool {
        let sources = self.directives.get(&CspDirective::ScriptSrc)
            .or_else(|| self.directives.get(&CspDirective::DefaultSrc));

        match sources {
            None => true,
            Some(source_list) => {
                source_list.iter().any(|s| matches!(s, CspSource::UnsafeInline))
            }
        }
    }

    /// Check if inline styles are allowed
    pub fn allows_inline_style(&self) -> bool {
        let sources = self.directives.get(&CspDirective::StyleSrc)
            .or_else(|| self.directives.get(&CspDirective::DefaultSrc));

        match sources {
            None => true,
            Some(source_list) => {
                source_list.iter().any(|s| matches!(s, CspSource::UnsafeInline))
            }
        }
    }

    /// Check if eval() is allowed
    pub fn allows_eval(&self) -> bool {
        let sources = self.directives.get(&CspDirective::ScriptSrc)
            .or_else(|| self.directives.get(&CspDirective::DefaultSrc));

        match sources {
            None => true,
            Some(source_list) => {
                source_list.iter().any(|s| matches!(s, CspSource::UnsafeEval))
            }
        }
    }

    /// Check if this policy has upgrade-insecure-requests
    pub fn upgrade_insecure_requests(&self) -> bool {
        self.directives.contains_key(&CspDirective::UpgradeInsecureRequests)
    }

    /// Check if this policy blocks mixed content
    pub fn blocks_mixed_content(&self) -> bool {
        self.directives.contains_key(&CspDirective::BlockAllMixedContent)
    }

    /// Get the report URI if specified
    pub fn report_uri(&self) -> Option<&str> {
        self.directives.get(&CspDirective::ReportUri)
            .and_then(|sources| sources.first())
            .and_then(|source| {
                if let CspSource::Host(uri) = source {
                    Some(uri.as_str())
                } else {
                    None
                }
            })
    }
}

/// CSP Manager for tracking policies per-origin
#[derive(Debug, Default)]
pub struct CspManager {
    /// Policies keyed by document URL
    policies: Arc<RwLock<HashMap<String, ContentSecurityPolicy>>>,
}

impl CspManager {
    /// Create a new CSP manager
    pub fn new() -> Self {
        Self {
            policies: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Set the CSP for a document
    pub fn set_policy(&self, document_url: &str, policy: ContentSecurityPolicy) {
        let mut policies = self.policies.write().unwrap();
        policies.insert(document_url.to_string(), policy);
    }

    /// Get the CSP for a document
    pub fn get_policy(&self, document_url: &str) -> Option<ContentSecurityPolicy> {
        let policies = self.policies.read().unwrap();
        policies.get(document_url).cloned()
    }

    /// Remove the CSP for a document
    pub fn remove_policy(&self, document_url: &str) {
        let mut policies = self.policies.write().unwrap();
        policies.remove(document_url);
    }

    /// Clear all policies
    pub fn clear(&self) {
        let mut policies = self.policies.write().unwrap();
        policies.clear();
    }

    /// Check if a resource is allowed for a document
    pub fn is_allowed(
        &self,
        document_url: &str,
        resource_url: &Url,
        resource_type: ResourceType,
    ) -> bool {
        let policies = self.policies.read().unwrap();

        if let Some(policy) = policies.get(document_url) {
            let document_origin = Url::parse(document_url).ok();
            policy.allows(resource_url, resource_type, document_origin.as_ref())
        } else {
            // No policy, allow by default
            true
        }
    }
}

impl Clone for CspManager {
    fn clone(&self) -> Self {
        Self {
            policies: Arc::clone(&self.policies),
        }
    }
}

/// CSP interceptor for extracting and storing CSP headers from responses
pub struct CspInterceptor {
    /// CSP manager for storing policies
    manager: CspManager,
}

impl CspInterceptor {
    /// Create a new CSP interceptor
    pub fn new() -> Self {
        Self {
            manager: CspManager::new(),
        }
    }

    /// Create a CSP interceptor with a shared manager
    pub fn with_manager(manager: CspManager) -> Self {
        Self { manager }
    }

    /// Get a reference to the CSP manager
    pub fn manager(&self) -> &CspManager {
        &self.manager
    }
}

impl Default for CspInterceptor {
    fn default() -> Self {
        Self::new()
    }
}

impl RequestInterceptor for CspInterceptor {
    fn pre_request(&mut self, _request: &mut Request) -> Result<()> {
        // No modifications needed in pre-request
        Ok(())
    }

    fn post_response(&mut self, response: &mut Response) -> Result<()> {
        // Extract CSP headers from response
        let csp_header = response.headers.get("content-security-policy")
            .or_else(|| response.headers.get("Content-Security-Policy"));

        let csp_report_only = response.headers.get("content-security-policy-report-only")
            .or_else(|| response.headers.get("Content-Security-Policy-Report-Only"));

        // Parse and store CSP policies
        // Note: We'd need the document URL to properly store this
        // For now, we just validate that CSP headers can be parsed
        if let Some(csp_value) = csp_header {
            let _policy = ContentSecurityPolicy::parse(csp_value, false);
            // In a full implementation, we'd store this associated with the document URL
        }

        if let Some(csp_value) = csp_report_only {
            let _policy = ContentSecurityPolicy::parse(csp_value, true);
        }

        Ok(())
    }

    fn should_block(&self, _request: &Request) -> bool {
        // CSP interceptor doesn't block requests in pre-flight
        // Blocking happens at resource loading time based on stored policies
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================
    // Tests for CspDirective
    // ========================================

    #[test]
    fn test_csp_directive_from_str() {
        assert_eq!(CspDirective::from_str("default-src"), CspDirective::DefaultSrc);
        assert_eq!(CspDirective::from_str("script-src"), CspDirective::ScriptSrc);
        assert_eq!(CspDirective::from_str("style-src"), CspDirective::StyleSrc);
        assert_eq!(CspDirective::from_str("img-src"), CspDirective::ImgSrc);
        assert_eq!(CspDirective::from_str("SCRIPT-SRC"), CspDirective::ScriptSrc);
    }

    #[test]
    fn test_csp_directive_unknown() {
        match CspDirective::from_str("custom-directive") {
            CspDirective::Unknown(s) => assert_eq!(s, "custom-directive"),
            _ => panic!("Expected Unknown directive"),
        }
    }

    #[test]
    fn test_csp_directive_as_str() {
        assert_eq!(CspDirective::DefaultSrc.as_str(), "default-src");
        assert_eq!(CspDirective::ScriptSrc.as_str(), "script-src");
        assert_eq!(CspDirective::StyleSrc.as_str(), "style-src");
    }

    // ========================================
    // Tests for CspSource
    // ========================================

    #[test]
    fn test_csp_source_self() {
        assert_eq!(CspSource::from_str("'self'"), CspSource::Self_);
        assert_eq!(CspSource::from_str("'SELF'"), CspSource::Self_);
    }

    #[test]
    fn test_csp_source_unsafe_inline() {
        assert_eq!(CspSource::from_str("'unsafe-inline'"), CspSource::UnsafeInline);
    }

    #[test]
    fn test_csp_source_unsafe_eval() {
        assert_eq!(CspSource::from_str("'unsafe-eval'"), CspSource::UnsafeEval);
    }

    #[test]
    fn test_csp_source_none() {
        assert_eq!(CspSource::from_str("'none'"), CspSource::None);
    }

    #[test]
    fn test_csp_source_scheme() {
        assert_eq!(CspSource::from_str("https:"), CspSource::Scheme("https".to_string()));
        assert_eq!(CspSource::from_str("data:"), CspSource::Scheme("data".to_string()));
    }

    #[test]
    fn test_csp_source_host() {
        assert_eq!(CspSource::from_str("example.com"), CspSource::Host("example.com".to_string()));
        assert_eq!(CspSource::from_str("*.example.com"), CspSource::Host("*.example.com".to_string()));
    }

    #[test]
    fn test_csp_source_nonce() {
        match CspSource::from_str("'nonce-abc123'") {
            CspSource::Nonce(n) => assert_eq!(n, "abc123"),
            _ => panic!("Expected Nonce"),
        }
    }

    #[test]
    fn test_csp_source_matches_self() {
        let source = CspSource::Self_;
        let origin = Url::parse("https://example.com").unwrap();

        let same_origin = Url::parse("https://example.com/page").unwrap();
        assert!(source.matches(&same_origin, Some(&origin)));

        let different_origin = Url::parse("https://other.com/page").unwrap();
        assert!(!source.matches(&different_origin, Some(&origin)));
    }

    #[test]
    fn test_csp_source_matches_scheme() {
        let source = CspSource::Scheme("https".to_string());

        let https_url = Url::parse("https://example.com").unwrap();
        assert!(source.matches(&https_url, None));

        let http_url = Url::parse("http://example.com").unwrap();
        assert!(!source.matches(&http_url, None));
    }

    #[test]
    fn test_csp_source_matches_host() {
        let source = CspSource::Host("example.com".to_string());

        let matching = Url::parse("https://example.com/page").unwrap();
        assert!(source.matches(&matching, None));

        let not_matching = Url::parse("https://other.com/page").unwrap();
        assert!(!source.matches(&not_matching, None));
    }

    #[test]
    fn test_csp_source_matches_wildcard_host() {
        let source = CspSource::Host("*.example.com".to_string());

        let subdomain = Url::parse("https://sub.example.com/page").unwrap();
        assert!(source.matches(&subdomain, None));

        let exact = Url::parse("https://example.com/page").unwrap();
        assert!(!source.matches(&exact, None));
    }

    // ========================================
    // Tests for ContentSecurityPolicy
    // ========================================

    #[test]
    fn test_csp_parse_simple() {
        let csp = ContentSecurityPolicy::parse("default-src 'self'", false);

        assert!(!csp.report_only);
        assert!(csp.directives.contains_key(&CspDirective::DefaultSrc));

        let sources = csp.directives.get(&CspDirective::DefaultSrc).unwrap();
        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0], CspSource::Self_);
    }

    #[test]
    fn test_csp_parse_multiple_directives() {
        let csp = ContentSecurityPolicy::parse(
            "default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'",
            false
        );

        assert!(csp.directives.contains_key(&CspDirective::DefaultSrc));
        assert!(csp.directives.contains_key(&CspDirective::ScriptSrc));
        assert!(csp.directives.contains_key(&CspDirective::StyleSrc));
    }

    #[test]
    fn test_csp_parse_complex() {
        let csp = ContentSecurityPolicy::parse(
            "default-src 'none'; script-src 'self' https://trusted.com; style-src 'self' 'unsafe-inline'; img-src https:",
            false
        );

        let script_sources = csp.directives.get(&CspDirective::ScriptSrc).unwrap();
        assert_eq!(script_sources.len(), 2);

        let img_sources = csp.directives.get(&CspDirective::ImgSrc).unwrap();
        assert_eq!(img_sources.len(), 1);
        assert_eq!(img_sources[0], CspSource::Scheme("https".to_string()));
    }

    #[test]
    fn test_csp_allows_self_script() {
        let csp = ContentSecurityPolicy::parse("script-src 'self'", false);
        let origin = Url::parse("https://example.com").unwrap();

        let same_origin = Url::parse("https://example.com/script.js").unwrap();
        assert!(csp.allows(&same_origin, ResourceType::Script, Some(&origin)));

        let different = Url::parse("https://other.com/script.js").unwrap();
        assert!(!csp.allows(&different, ResourceType::Script, Some(&origin)));
    }

    #[test]
    fn test_csp_allows_with_fallback_to_default() {
        let csp = ContentSecurityPolicy::parse("default-src 'self'", false);
        let origin = Url::parse("https://example.com").unwrap();

        // Script-src not specified, should fall back to default-src
        let script_url = Url::parse("https://example.com/script.js").unwrap();
        assert!(csp.allows(&script_url, ResourceType::Script, Some(&origin)));
    }

    #[test]
    fn test_csp_blocks_none() {
        let csp = ContentSecurityPolicy::parse("script-src 'none'", false);
        let origin = Url::parse("https://example.com").unwrap();

        let script_url = Url::parse("https://example.com/script.js").unwrap();
        assert!(!csp.allows(&script_url, ResourceType::Script, Some(&origin)));
    }

    #[test]
    fn test_csp_allows_inline_script() {
        let csp_with = ContentSecurityPolicy::parse("script-src 'self' 'unsafe-inline'", false);
        assert!(csp_with.allows_inline_script());

        let csp_without = ContentSecurityPolicy::parse("script-src 'self'", false);
        assert!(!csp_without.allows_inline_script());
    }

    #[test]
    fn test_csp_allows_inline_style() {
        let csp_with = ContentSecurityPolicy::parse("style-src 'self' 'unsafe-inline'", false);
        assert!(csp_with.allows_inline_style());

        let csp_without = ContentSecurityPolicy::parse("style-src 'self'", false);
        assert!(!csp_without.allows_inline_style());
    }

    #[test]
    fn test_csp_allows_eval() {
        let csp_with = ContentSecurityPolicy::parse("script-src 'self' 'unsafe-eval'", false);
        assert!(csp_with.allows_eval());

        let csp_without = ContentSecurityPolicy::parse("script-src 'self'", false);
        assert!(!csp_without.allows_eval());
    }

    #[test]
    fn test_csp_upgrade_insecure_requests() {
        let csp_with = ContentSecurityPolicy::parse("upgrade-insecure-requests; default-src 'self'", false);
        assert!(csp_with.upgrade_insecure_requests());

        let csp_without = ContentSecurityPolicy::parse("default-src 'self'", false);
        assert!(!csp_without.upgrade_insecure_requests());
    }

    // ========================================
    // Tests for CspManager
    // ========================================

    #[test]
    fn test_csp_manager_set_get_policy() {
        let manager = CspManager::new();
        let policy = ContentSecurityPolicy::parse("default-src 'self'", false);

        manager.set_policy("https://example.com", policy.clone());

        let retrieved = manager.get_policy("https://example.com");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().raw, policy.raw);
    }

    #[test]
    fn test_csp_manager_remove_policy() {
        let manager = CspManager::new();
        let policy = ContentSecurityPolicy::parse("default-src 'self'", false);

        manager.set_policy("https://example.com", policy);
        manager.remove_policy("https://example.com");

        assert!(manager.get_policy("https://example.com").is_none());
    }

    #[test]
    fn test_csp_manager_clear() {
        let manager = CspManager::new();

        manager.set_policy("https://example.com", ContentSecurityPolicy::default());
        manager.set_policy("https://other.com", ContentSecurityPolicy::default());

        manager.clear();

        assert!(manager.get_policy("https://example.com").is_none());
        assert!(manager.get_policy("https://other.com").is_none());
    }

    #[test]
    fn test_csp_manager_is_allowed() {
        let manager = CspManager::new();
        let policy = ContentSecurityPolicy::parse("script-src 'self'", false);
        manager.set_policy("https://example.com/page", policy);

        let allowed = Url::parse("https://example.com/script.js").unwrap();
        assert!(manager.is_allowed("https://example.com/page", &allowed, ResourceType::Script));

        let blocked = Url::parse("https://evil.com/script.js").unwrap();
        assert!(!manager.is_allowed("https://example.com/page", &blocked, ResourceType::Script));
    }

    #[test]
    fn test_csp_manager_no_policy_allows_all() {
        let manager = CspManager::new();

        let url = Url::parse("https://evil.com/script.js").unwrap();
        assert!(manager.is_allowed("https://example.com", &url, ResourceType::Script));
    }

    // ========================================
    // Tests for CspInterceptor
    // ========================================

    #[test]
    fn test_csp_interceptor_creation() {
        let interceptor = CspInterceptor::new();
        assert!(interceptor.manager().get_policy("any").is_none());
    }

    #[test]
    fn test_csp_interceptor_should_not_block() {
        let interceptor = CspInterceptor::new();
        let url = Url::parse("https://example.com").unwrap();
        let request = Request::new(url, crate::request_handler::HttpMethod::GET);

        assert!(!interceptor.should_block(&request));
    }

    #[test]
    fn test_csp_interceptor_post_response() {
        let mut interceptor = CspInterceptor::new();

        let mut headers = HashMap::new();
        headers.insert("content-security-policy".to_string(), "default-src 'self'".to_string());

        let mut response = Response::new(200, vec![], "req_123".to_string())
            .with_headers(headers);

        let result = interceptor.post_response(&mut response);
        assert!(result.is_ok());
    }

    // ========================================
    // Tests for ResourceType
    // ========================================

    #[test]
    fn test_resource_type_directive() {
        assert_eq!(ResourceType::Script.directive(), CspDirective::ScriptSrc);
        assert_eq!(ResourceType::Style.directive(), CspDirective::StyleSrc);
        assert_eq!(ResourceType::Image.directive(), CspDirective::ImgSrc);
        assert_eq!(ResourceType::Font.directive(), CspDirective::FontSrc);
        assert_eq!(ResourceType::Media.directive(), CspDirective::MediaSrc);
        assert_eq!(ResourceType::Connect.directive(), CspDirective::ConnectSrc);
        assert_eq!(ResourceType::Frame.directive(), CspDirective::FrameSrc);
        assert_eq!(ResourceType::Object.directive(), CspDirective::ObjectSrc);
        assert_eq!(ResourceType::Other.directive(), CspDirective::DefaultSrc);
    }
}
