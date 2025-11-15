//! DOM interface for WebDriver element finding and interaction
//!
//! This module provides DOM access through JavaScript execution in the WebView.
//! It supports all W3C WebDriver locator strategies and element interaction commands.

use crate::element::ElementReference;
use crate::errors::{Error, Result};
use crate::session::Session;

/// Locator strategy per W3C WebDriver specification
#[derive(Debug, Clone, PartialEq)]
pub enum LocatorStrategy {
    CssSelector,
    XPath,
    Id,
    Name,
    TagName,
    ClassName,
    LinkText,
    PartialLinkText,
}

impl LocatorStrategy {
    /// Parse a locator strategy from string
    pub fn from_str(s: &str) -> Result<Self> {
        match s {
            "css selector" => Ok(Self::CssSelector),
            "xpath" => Ok(Self::XPath),
            "id" => Ok(Self::Id),
            "name" => Ok(Self::Name),
            "tag name" => Ok(Self::TagName),
            "class name" => Ok(Self::ClassName),
            "link text" => Ok(Self::LinkText),
            "partial link text" => Ok(Self::PartialLinkText),
            _ => Err(Error::InvalidArgument(format!(
                "Invalid locator strategy: {}",
                s
            ))),
        }
    }

    /// Convert to W3C string representation
    pub fn to_str(&self) -> &'static str {
        match self {
            Self::CssSelector => "css selector",
            Self::XPath => "xpath",
            Self::Id => "id",
            Self::Name => "name",
            Self::TagName => "tag name",
            Self::ClassName => "class name",
            Self::LinkText => "link text",
            Self::PartialLinkText => "partial link text",
        }
    }
}

/// DOM interface for finding and interacting with elements
pub struct DomInterface;

impl DomInterface {
    /// Create a new DOM interface
    pub fn new() -> Self {
        Self
    }

    /// Find a single element using the specified locator strategy
    pub fn find_element(
        &self,
        session: &Session,
        strategy: &str,
        value: &str,
    ) -> Result<ElementReference> {
        let locator = LocatorStrategy::from_str(strategy)?;

        // Generate JavaScript to find the element
        let script = self.generate_find_script(&locator, value, false);

        // Execute script to find element
        let result = session
            .execute_script(&script)
            .map_err(|e| Error::JavaScriptError(format!("Failed to find element: {}", e)))?;

        // Parse result - should be true if element found, false otherwise
        let found = result.trim() == "true";

        if !found {
            return Err(Error::NoSuchElement(format!(
                "Element not found with {} = '{}'",
                strategy, value
            )));
        }

        // Create element reference with the selector
        // In production, we'd cache the actual DOM element
        let selector = self.selector_to_css(&locator, value);
        let element_ref = ElementReference::new(&session.id, selector, 0);

        Ok(element_ref)
    }

    /// Find multiple elements using the specified locator strategy
    pub fn find_elements(
        &self,
        session: &Session,
        strategy: &str,
        value: &str,
    ) -> Result<Vec<ElementReference>> {
        let locator = LocatorStrategy::from_str(strategy)?;

        // Generate JavaScript to find elements
        let script = self.generate_find_script(&locator, value, true);

        // Execute script
        let result = session
            .execute_script(&script)
            .map_err(|e| Error::JavaScriptError(format!("Failed to find elements: {}", e)))?;

        // Parse result as integer (count of elements found)
        let count: usize = result
            .trim()
            .parse()
            .map_err(|_| Error::JavaScriptError("Invalid element count".to_string()))?;

        // Create element references for each found element
        let selector = self.selector_to_css(&locator, value);
        let elements: Vec<ElementReference> = (0..count)
            .map(|i| ElementReference::new(&session.id, &selector, i))
            .collect();

        Ok(elements)
    }

    /// Generate JavaScript to find element(s)
    fn generate_find_script(
        &self,
        strategy: &LocatorStrategy,
        value: &str,
        find_multiple: bool,
    ) -> String {
        // Escape value for JavaScript string
        let escaped_value = Self::escape_js_string(value);

        match strategy {
            LocatorStrategy::CssSelector => {
                if find_multiple {
                    format!(
                        "document.querySelectorAll('{}').length",
                        escaped_value
                    )
                } else {
                    format!(
                        "document.querySelector('{}') !== null",
                        escaped_value
                    )
                }
            }
            LocatorStrategy::XPath => {
                if find_multiple {
                    format!(
                        r#"(function() {{
                            var result = document.evaluate('{}', document, null,
                                XPathResult.ORDERED_NODE_SNAPSHOT_TYPE, null);
                            return result.snapshotLength;
                        }})()"#,
                        escaped_value
                    )
                } else {
                    format!(
                        r#"(function() {{
                            var result = document.evaluate('{}', document, null,
                                XPathResult.FIRST_ORDERED_NODE_TYPE, null);
                            return result.singleNodeValue !== null;
                        }})()"#,
                        escaped_value
                    )
                }
            }
            LocatorStrategy::Id => {
                if find_multiple {
                    format!("document.getElementById('{}') ? 1 : 0", escaped_value)
                } else {
                    format!(
                        "document.getElementById('{}') !== null",
                        escaped_value
                    )
                }
            }
            LocatorStrategy::Name => {
                if find_multiple {
                    format!(
                        "document.getElementsByName('{}').length",
                        escaped_value
                    )
                } else {
                    format!(
                        "document.getElementsByName('{}').length > 0",
                        escaped_value
                    )
                }
            }
            LocatorStrategy::TagName => {
                if find_multiple {
                    format!(
                        "document.getElementsByTagName('{}').length",
                        escaped_value
                    )
                } else {
                    format!(
                        "document.getElementsByTagName('{}').length > 0",
                        escaped_value
                    )
                }
            }
            LocatorStrategy::ClassName => {
                if find_multiple {
                    format!(
                        "document.getElementsByClassName('{}').length",
                        escaped_value
                    )
                } else {
                    format!(
                        "document.getElementsByClassName('{}').length > 0",
                        escaped_value
                    )
                }
            }
            LocatorStrategy::LinkText => {
                if find_multiple {
                    format!(
                        r#"(function() {{
                            var links = Array.from(document.getElementsByTagName('a'));
                            return links.filter(function(el) {{
                                return el.textContent.trim() === '{}';
                            }}).length;
                        }})()"#,
                        escaped_value
                    )
                } else {
                    format!(
                        r#"(function() {{
                            var links = Array.from(document.getElementsByTagName('a'));
                            return links.some(function(el) {{
                                return el.textContent.trim() === '{}';
                            }});
                        }})()"#,
                        escaped_value
                    )
                }
            }
            LocatorStrategy::PartialLinkText => {
                if find_multiple {
                    format!(
                        r#"(function() {{
                            var links = Array.from(document.getElementsByTagName('a'));
                            return links.filter(function(el) {{
                                return el.textContent.indexOf('{}') !== -1;
                            }}).length;
                        }})()"#,
                        escaped_value
                    )
                } else {
                    format!(
                        r#"(function() {{
                            var links = Array.from(document.getElementsByTagName('a'));
                            return links.some(function(el) {{
                                return el.textContent.indexOf('{}') !== -1;
                            }});
                        }})()"#,
                        escaped_value
                    )
                }
            }
        }
    }

    /// Convert locator strategy to CSS selector (for caching purposes)
    fn selector_to_css(&self, strategy: &LocatorStrategy, value: &str) -> String {
        match strategy {
            LocatorStrategy::CssSelector => value.to_string(),
            LocatorStrategy::Id => format!("#{}", value),
            LocatorStrategy::ClassName => format!(".{}", value),
            LocatorStrategy::TagName => value.to_string(),
            LocatorStrategy::Name => format!("[name='{}']", value),
            _ => format!("/* {} = {} */", strategy.to_str(), value),
        }
    }

    /// Escape a string for use in JavaScript
    fn escape_js_string(s: &str) -> String {
        s.replace('\\', "\\\\")
            .replace('\'', "\\'")
            .replace('"', "\\\"")
            .replace('\n', "\\n")
            .replace('\r', "\\r")
            .replace('\t', "\\t")
    }

    /// Generate script to click an element
    pub fn generate_click_script(&self, selector: &str, index: usize) -> String {
        format!(
            r#"(function() {{
                var elements = document.querySelectorAll('{}');
                if (elements.length > {}) {{
                    elements[{}].click();
                    return true;
                }}
                return false;
            }})()"#,
            Self::escape_js_string(selector),
            index,
            index
        )
    }

    /// Generate script to send keys to an element
    pub fn generate_send_keys_script(&self, selector: &str, index: usize, text: &str) -> String {
        let escaped_text = Self::escape_js_string(text);
        format!(
            r#"(function() {{
                var elements = document.querySelectorAll('{}');
                if (elements.length > {}) {{
                    var element = elements[{}];
                    element.value = '{}';
                    element.dispatchEvent(new Event('input', {{ bubbles: true }}));
                    element.dispatchEvent(new Event('change', {{ bubbles: true }}));
                    return true;
                }}
                return false;
            }})()"#,
            Self::escape_js_string(selector),
            index,
            index,
            escaped_text
        )
    }

    /// Generate script to clear an element
    pub fn generate_clear_script(&self, selector: &str, index: usize) -> String {
        format!(
            r#"(function() {{
                var elements = document.querySelectorAll('{}');
                if (elements.length > {}) {{
                    var element = elements[{}];
                    element.value = '';
                    element.dispatchEvent(new Event('input', {{ bubbles: true }}));
                    element.dispatchEvent(new Event('change', {{ bubbles: true }}));
                    return true;
                }}
                return false;
            }})()"#,
            Self::escape_js_string(selector),
            index,
            index
        )
    }

    /// Generate script to get element text
    pub fn generate_get_text_script(&self, selector: &str, index: usize) -> String {
        format!(
            r#"(function() {{
                var elements = document.querySelectorAll('{}');
                if (elements.length > {}) {{
                    var element = elements[{}];
                    return element.textContent || element.innerText || '';
                }}
                return '';
            }})()"#,
            Self::escape_js_string(selector),
            index,
            index
        )
    }

    /// Generate script to get element attribute
    pub fn generate_get_attribute_script(
        &self,
        selector: &str,
        index: usize,
        attribute: &str,
    ) -> String {
        format!(
            r#"(function() {{
                var elements = document.querySelectorAll('{}');
                if (elements.length > {}) {{
                    return elements[{}].getAttribute('{}');
                }}
                return null;
            }})()"#,
            Self::escape_js_string(selector),
            index,
            index,
            Self::escape_js_string(attribute)
        )
    }

    /// Generate script to check if element is displayed
    pub fn generate_is_displayed_script(&self, selector: &str, index: usize) -> String {
        format!(
            r#"(function() {{
                var elements = document.querySelectorAll('{}');
                if (elements.length > {}) {{
                    var element = elements[{}];
                    return element.offsetParent !== null;
                }}
                return false;
            }})()"#,
            Self::escape_js_string(selector),
            index,
            index
        )
    }

    /// Generate script to check if element is enabled
    pub fn generate_is_enabled_script(&self, selector: &str, index: usize) -> String {
        format!(
            r#"(function() {{
                var elements = document.querySelectorAll('{}');
                if (elements.length > {}) {{
                    var element = elements[{}];
                    return !element.disabled;
                }}
                return false;
            }})()"#,
            Self::escape_js_string(selector),
            index,
            index
        )
    }

    /// Generate script to check if element is selected
    pub fn generate_is_selected_script(&self, selector: &str, index: usize) -> String {
        format!(
            r#"(function() {{
                var elements = document.querySelectorAll('{}');
                if (elements.length > {}) {{
                    var element = elements[{}];
                    return element.selected || element.checked || false;
                }}
                return false;
            }})()"#,
            Self::escape_js_string(selector),
            index,
            index
        )
    }
}

impl Default for DomInterface {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================
    // LocatorStrategy Tests
    // ========================================

    #[test]
    fn test_locator_strategy_from_str_css() {
        let strategy = LocatorStrategy::from_str("css selector").unwrap();
        assert_eq!(strategy, LocatorStrategy::CssSelector);
        assert_eq!(strategy.to_str(), "css selector");
    }

    #[test]
    fn test_locator_strategy_from_str_xpath() {
        let strategy = LocatorStrategy::from_str("xpath").unwrap();
        assert_eq!(strategy, LocatorStrategy::XPath);
        assert_eq!(strategy.to_str(), "xpath");
    }

    #[test]
    fn test_locator_strategy_from_str_id() {
        let strategy = LocatorStrategy::from_str("id").unwrap();
        assert_eq!(strategy, LocatorStrategy::Id);
        assert_eq!(strategy.to_str(), "id");
    }

    #[test]
    fn test_locator_strategy_from_str_name() {
        let strategy = LocatorStrategy::from_str("name").unwrap();
        assert_eq!(strategy, LocatorStrategy::Name);
        assert_eq!(strategy.to_str(), "name");
    }

    #[test]
    fn test_locator_strategy_from_str_tag_name() {
        let strategy = LocatorStrategy::from_str("tag name").unwrap();
        assert_eq!(strategy, LocatorStrategy::TagName);
        assert_eq!(strategy.to_str(), "tag name");
    }

    #[test]
    fn test_locator_strategy_from_str_class_name() {
        let strategy = LocatorStrategy::from_str("class name").unwrap();
        assert_eq!(strategy, LocatorStrategy::ClassName);
        assert_eq!(strategy.to_str(), "class name");
    }

    #[test]
    fn test_locator_strategy_from_str_link_text() {
        let strategy = LocatorStrategy::from_str("link text").unwrap();
        assert_eq!(strategy, LocatorStrategy::LinkText);
        assert_eq!(strategy.to_str(), "link text");
    }

    #[test]
    fn test_locator_strategy_from_str_partial_link_text() {
        let strategy = LocatorStrategy::from_str("partial link text").unwrap();
        assert_eq!(strategy, LocatorStrategy::PartialLinkText);
        assert_eq!(strategy.to_str(), "partial link text");
    }

    #[test]
    fn test_locator_strategy_from_str_invalid() {
        let result = LocatorStrategy::from_str("invalid");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), Error::InvalidArgument(_)));
    }

    // ========================================
    // DomInterface Tests
    // ========================================

    #[test]
    fn test_dom_interface_new() {
        let interface = DomInterface::new();
        // Just verify creation works
        let _ = interface;
    }

    #[test]
    fn test_escape_js_string() {
        assert_eq!(
            DomInterface::escape_js_string("hello"),
            "hello"
        );
        assert_eq!(
            DomInterface::escape_js_string("hello'world"),
            "hello\\'world"
        );
        assert_eq!(
            DomInterface::escape_js_string("hello\"world"),
            "hello\\\"world"
        );
        assert_eq!(
            DomInterface::escape_js_string("hello\nworld"),
            "hello\\nworld"
        );
        assert_eq!(
            DomInterface::escape_js_string("hello\\world"),
            "hello\\\\world"
        );
    }

    #[test]
    fn test_generate_find_script_css_selector() {
        let interface = DomInterface::new();
        let strategy = LocatorStrategy::CssSelector;

        let script = interface.generate_find_script(&strategy, "#button", false);
        assert!(script.contains("querySelector"));
        assert!(script.contains("#button"));
        assert!(script.contains("!== null"));

        let script_multiple = interface.generate_find_script(&strategy, "#button", true);
        assert!(script_multiple.contains("querySelectorAll"));
        assert!(script_multiple.contains(".length"));
    }

    #[test]
    fn test_generate_find_script_xpath() {
        let interface = DomInterface::new();
        let strategy = LocatorStrategy::XPath;

        let script = interface.generate_find_script(&strategy, "//button", false);
        assert!(script.contains("document.evaluate"));
        assert!(script.contains("//button"));
        assert!(script.contains("FIRST_ORDERED_NODE_TYPE"));

        let script_multiple = interface.generate_find_script(&strategy, "//button", true);
        assert!(script_multiple.contains("ORDERED_NODE_SNAPSHOT_TYPE"));
        assert!(script_multiple.contains("snapshotLength"));
    }

    #[test]
    fn test_generate_find_script_id() {
        let interface = DomInterface::new();
        let strategy = LocatorStrategy::Id;

        let script = interface.generate_find_script(&strategy, "submit", false);
        assert!(script.contains("getElementById"));
        assert!(script.contains("submit"));
    }

    #[test]
    fn test_generate_find_script_name() {
        let interface = DomInterface::new();
        let strategy = LocatorStrategy::Name;

        let script = interface.generate_find_script(&strategy, "username", false);
        assert!(script.contains("getElementsByName"));
        assert!(script.contains("username"));
    }

    #[test]
    fn test_generate_find_script_link_text() {
        let interface = DomInterface::new();
        let strategy = LocatorStrategy::LinkText;

        let script = interface.generate_find_script(&strategy, "Click here", false);
        assert!(script.contains("getElementsByTagName('a')"));
        assert!(script.contains("textContent"));
        assert!(script.contains("Click here"));
    }

    #[test]
    fn test_selector_to_css() {
        let interface = DomInterface::new();

        assert_eq!(
            interface.selector_to_css(&LocatorStrategy::CssSelector, "#button"),
            "#button"
        );
        assert_eq!(
            interface.selector_to_css(&LocatorStrategy::Id, "submit"),
            "#submit"
        );
        assert_eq!(
            interface.selector_to_css(&LocatorStrategy::ClassName, "btn"),
            ".btn"
        );
        assert_eq!(
            interface.selector_to_css(&LocatorStrategy::TagName, "button"),
            "button"
        );
        assert_eq!(
            interface.selector_to_css(&LocatorStrategy::Name, "username"),
            "[name='username']"
        );
    }

    #[test]
    fn test_generate_click_script() {
        let interface = DomInterface::new();

        let script = interface.generate_click_script("#button", 0);
        assert!(script.contains("querySelectorAll"));
        assert!(script.contains("#button"));
        assert!(script.contains(".click()"));
        assert!(script.contains("[0]"));
    }

    #[test]
    fn test_generate_send_keys_script() {
        let interface = DomInterface::new();

        let script = interface.generate_send_keys_script("#input", 0, "test value");
        assert!(script.contains("querySelectorAll"));
        assert!(script.contains("#input"));
        assert!(script.contains("test value"));
        assert!(script.contains(".value ="));
        assert!(script.contains("dispatchEvent"));
        assert!(script.contains("input"));
    }

    #[test]
    fn test_generate_clear_script() {
        let interface = DomInterface::new();

        let script = interface.generate_clear_script("#input", 0);
        assert!(script.contains("querySelectorAll"));
        assert!(script.contains("#input"));
        assert!(script.contains(".value = ''"));
        assert!(script.contains("dispatchEvent"));
    }

    #[test]
    fn test_generate_get_text_script() {
        let interface = DomInterface::new();

        let script = interface.generate_get_text_script("#button", 0);
        assert!(script.contains("querySelectorAll"));
        assert!(script.contains("#button"));
        assert!(script.contains("textContent"));
        assert!(script.contains("innerText"));
    }

    #[test]
    fn test_generate_get_attribute_script() {
        let interface = DomInterface::new();

        let script = interface.generate_get_attribute_script("#button", 0, "class");
        assert!(script.contains("querySelectorAll"));
        assert!(script.contains("#button"));
        assert!(script.contains("getAttribute"));
        assert!(script.contains("class"));
    }

    #[test]
    fn test_generate_is_displayed_script() {
        let interface = DomInterface::new();

        let script = interface.generate_is_displayed_script("#button", 0);
        assert!(script.contains("querySelectorAll"));
        assert!(script.contains("#button"));
        assert!(script.contains("offsetParent"));
    }

    #[test]
    fn test_generate_is_enabled_script() {
        let interface = DomInterface::new();

        let script = interface.generate_is_enabled_script("#button", 0);
        assert!(script.contains("querySelectorAll"));
        assert!(script.contains("#button"));
        assert!(script.contains("!element.disabled"));
    }

    #[test]
    fn test_generate_is_selected_script() {
        let interface = DomInterface::new();

        let script = interface.generate_is_selected_script("#checkbox", 0);
        assert!(script.contains("querySelectorAll"));
        assert!(script.contains("#checkbox"));
        assert!(script.contains("selected"));
        assert!(script.contains("checked"));
    }

    #[test]
    fn test_js_escape_complex() {
        // Test complex string with multiple escape characters
        let input = "hello'world\"test\nline\ttab\\backslash";
        let escaped = DomInterface::escape_js_string(input);

        assert!(escaped.contains("\\'"));
        assert!(escaped.contains("\\\""));
        assert!(escaped.contains("\\n"));
        assert!(escaped.contains("\\t"));
        assert!(escaped.contains("\\\\"));
    }

    #[test]
    fn test_script_injection_prevention() {
        // Test that malicious input is properly escaped
        let interface = DomInterface::new();

        let malicious = "'); alert('XSS'); ('";
        let script = interface.generate_click_script(malicious, 0);

        // Should contain escaped version
        assert!(script.contains("\\'"));
        // Should not contain unescaped parentheses from malicious input in dangerous context
        assert!(!script.contains("'); alert('XSS')"));
    }
}
