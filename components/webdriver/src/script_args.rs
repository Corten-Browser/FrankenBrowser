//! Script argument serialization for WebDriver executeScript and executeAsyncScript
//!
//! This module handles conversion of JSON arguments to JavaScript values for script execution.
//! It supports all W3C WebDriver types including primitives, collections, and element references.

use crate::errors::{Error, Result};
use serde_json::Value as JsonValue;
use std::collections::HashMap;

/// Script argument types per W3C WebDriver specification
#[derive(Debug, Clone, PartialEq)]
pub enum ScriptArgument {
    /// JavaScript null
    Null,
    /// Boolean value
    Boolean(bool),
    /// Number (JavaScript uses f64 for all numbers)
    Number(f64),
    /// String value
    String(String),
    /// Array of arguments (recursive)
    Array(Vec<ScriptArgument>),
    /// Object with string keys (recursive)
    Object(HashMap<String, ScriptArgument>),
    /// WebDriver element reference (special handling)
    Element(String), // Element UUID
}

impl ScriptArgument {
    /// Convert JSON value to ScriptArgument
    ///
    /// # Arguments
    ///
    /// * `value` - JSON value from WebDriver request
    ///
    /// # Returns
    ///
    /// ScriptArgument representation of the JSON value
    ///
    /// # Examples
    ///
    /// ```
    /// use serde_json::json;
    /// use webdriver::script_args::ScriptArgument;
    ///
    /// let arg = ScriptArgument::from_json(json!(42));
    /// assert_eq!(arg, ScriptArgument::Number(42.0));
    ///
    /// let arg = ScriptArgument::from_json(json!("hello"));
    /// assert_eq!(arg, ScriptArgument::String("hello".to_string()));
    /// ```
    pub fn from_json(value: JsonValue) -> ScriptArgument {
        match value {
            JsonValue::Null => ScriptArgument::Null,
            JsonValue::Bool(b) => ScriptArgument::Boolean(b),
            JsonValue::Number(n) => {
                // Convert to f64 (JavaScript number type)
                let num = n.as_f64().unwrap_or(0.0);
                ScriptArgument::Number(num)
            }
            JsonValue::String(s) => ScriptArgument::String(s),
            JsonValue::Array(arr) => {
                let args: Vec<ScriptArgument> = arr.into_iter().map(Self::from_json).collect();
                ScriptArgument::Array(args)
            }
            JsonValue::Object(obj) => {
                // Check if this is a WebDriver element reference
                // Per W3C spec: {"element-6066-11e4-a52e-4f735466cecf": "<uuid>"}
                if let Some(element_id) = obj.get("element-6066-11e4-a52e-4f735466cecf") {
                    if let Some(id) = element_id.as_str() {
                        return ScriptArgument::Element(id.to_string());
                    }
                }

                // Regular object - recursively convert values
                let mut map = HashMap::new();
                for (key, val) in obj {
                    map.insert(key, Self::from_json(val));
                }
                ScriptArgument::Object(map)
            }
        }
    }

    /// Convert ScriptArgument to JavaScript code representation
    ///
    /// # Returns
    ///
    /// JavaScript code string that evaluates to the argument value
    ///
    /// # Examples
    ///
    /// ```
    /// use webdriver::script_args::ScriptArgument;
    ///
    /// assert_eq!(ScriptArgument::Null.to_javascript(), "null");
    /// assert_eq!(ScriptArgument::Boolean(true).to_javascript(), "true");
    /// assert_eq!(ScriptArgument::Number(42.5).to_javascript(), "42.5");
    /// assert_eq!(ScriptArgument::String("hello".to_string()).to_javascript(), "\"hello\"");
    /// ```
    pub fn to_javascript(&self) -> String {
        match self {
            ScriptArgument::Null => "null".to_string(),
            ScriptArgument::Boolean(b) => b.to_string(),
            ScriptArgument::Number(n) => {
                // Handle special float values
                if n.is_nan() {
                    "NaN".to_string()
                } else if n.is_infinite() {
                    if n.is_sign_positive() {
                        "Infinity".to_string()
                    } else {
                        "-Infinity".to_string()
                    }
                } else {
                    n.to_string()
                }
            }
            ScriptArgument::String(s) => {
                // Escape special characters for JavaScript string literal
                let escaped = s
                    .replace('\\', "\\\\")
                    .replace('"', "\\\"")
                    .replace('\n', "\\n")
                    .replace('\r', "\\r")
                    .replace('\t', "\\t");
                format!("\"{}\"", escaped)
            }
            ScriptArgument::Array(arr) => {
                let items: Vec<String> = arr.iter().map(|a| a.to_javascript()).collect();
                format!("[{}]", items.join(", "))
            }
            ScriptArgument::Object(obj) => {
                let items: Vec<String> = obj
                    .iter()
                    .map(|(k, v)| {
                        // Escape key as JavaScript property name
                        let escaped_key = k
                            .replace('\\', "\\\\")
                            .replace('"', "\\\"");
                        format!("\"{}\": {}", escaped_key, v.to_javascript())
                    })
                    .collect();
                format!("{{{}}}", items.join(", "))
            }
            ScriptArgument::Element(element_id) => {
                // Convert element reference to DOM element lookup
                // This will be integrated with element cache in the handler
                format!("__webdriver_get_element(\"{}\")", element_id)
            }
        }
    }

    /// Convert multiple arguments to JavaScript array
    ///
    /// # Arguments
    ///
    /// * `args` - Vector of ScriptArguments
    ///
    /// # Returns
    ///
    /// JavaScript code for an arguments array
    ///
    /// # Examples
    ///
    /// ```
    /// use webdriver::script_args::ScriptArgument;
    ///
    /// let args = vec![
    ///     ScriptArgument::Number(42.0),
    ///     ScriptArgument::String("test".to_string()),
    /// ];
    /// let js = ScriptArgument::arguments_to_javascript(&args);
    /// assert_eq!(js, "[42, \"test\"]");
    /// ```
    pub fn arguments_to_javascript(args: &[ScriptArgument]) -> String {
        let items: Vec<String> = args.iter().map(|a| a.to_javascript()).collect();
        format!("[{}]", items.join(", "))
    }
}

/// Parse JavaScript return value to JSON
///
/// # Arguments
///
/// * `result` - JavaScript result string from script execution
///
/// # Returns
///
/// JSON value representation of the result
///
/// # Errors
///
/// Returns JavaScriptError if parsing fails
pub fn parse_script_result(result: &str) -> Result<JsonValue> {
    // Try to parse as JSON first
    if let Ok(value) = serde_json::from_str(result) {
        return Ok(value);
    }

    // Handle special JavaScript values that aren't valid JSON
    let trimmed = result.trim();
    match trimmed {
        "undefined" => Ok(JsonValue::Null),
        "NaN" => Ok(JsonValue::Null), // Per WebDriver spec, NaN -> null
        "Infinity" | "-Infinity" => Ok(JsonValue::Null), // Per WebDriver spec, Infinity -> null
        _ => {
            // If it's a simple string without quotes, wrap it
            if !trimmed.starts_with('"') && !trimmed.starts_with('[') && !trimmed.starts_with('{') {
                Ok(JsonValue::String(trimmed.to_string()))
            } else {
                // Unable to parse
                Err(Error::JavaScriptError(format!(
                    "Failed to parse script result: {}",
                    result
                )))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // ============================================================================
    // Argument Serialization Tests (TDD - RED phase)
    // ============================================================================

    #[test]
    fn test_null_argument() {
        let arg = ScriptArgument::from_json(json!(null));
        assert_eq!(arg, ScriptArgument::Null);
        assert_eq!(arg.to_javascript(), "null");
    }

    #[test]
    fn test_boolean_arguments() {
        let arg_true = ScriptArgument::from_json(json!(true));
        assert_eq!(arg_true, ScriptArgument::Boolean(true));
        assert_eq!(arg_true.to_javascript(), "true");

        let arg_false = ScriptArgument::from_json(json!(false));
        assert_eq!(arg_false, ScriptArgument::Boolean(false));
        assert_eq!(arg_false.to_javascript(), "false");
    }

    #[test]
    fn test_number_arguments() {
        // Integer
        let arg = ScriptArgument::from_json(json!(42));
        assert_eq!(arg, ScriptArgument::Number(42.0));
        assert_eq!(arg.to_javascript(), "42");

        // Float
        let arg = ScriptArgument::from_json(json!(3.14159));
        assert_eq!(arg, ScriptArgument::Number(3.14159));
        assert_eq!(arg.to_javascript(), "3.14159");

        // Negative
        let arg = ScriptArgument::from_json(json!(-100));
        assert_eq!(arg, ScriptArgument::Number(-100.0));
        assert_eq!(arg.to_javascript(), "-100");

        // Zero
        let arg = ScriptArgument::from_json(json!(0));
        assert_eq!(arg, ScriptArgument::Number(0.0));
        assert_eq!(arg.to_javascript(), "0");
    }

    #[test]
    fn test_special_number_values() {
        // NaN
        let arg = ScriptArgument::Number(f64::NAN);
        assert_eq!(arg.to_javascript(), "NaN");

        // Infinity
        let arg = ScriptArgument::Number(f64::INFINITY);
        assert_eq!(arg.to_javascript(), "Infinity");

        // -Infinity
        let arg = ScriptArgument::Number(f64::NEG_INFINITY);
        assert_eq!(arg.to_javascript(), "-Infinity");
    }

    #[test]
    fn test_string_arguments() {
        let arg = ScriptArgument::from_json(json!("hello world"));
        assert_eq!(arg, ScriptArgument::String("hello world".to_string()));
        assert_eq!(arg.to_javascript(), "\"hello world\"");

        // Empty string
        let arg = ScriptArgument::from_json(json!(""));
        assert_eq!(arg, ScriptArgument::String("".to_string()));
        assert_eq!(arg.to_javascript(), "\"\"");
    }

    #[test]
    fn test_string_escaping() {
        // Backslashes
        let arg = ScriptArgument::String("path\\to\\file".to_string());
        assert_eq!(arg.to_javascript(), "\"path\\\\to\\\\file\"");

        // Quotes
        let arg = ScriptArgument::String("He said \"hello\"".to_string());
        assert_eq!(arg.to_javascript(), "\"He said \\\"hello\\\"\"");

        // Newlines and tabs
        let arg = ScriptArgument::String("line1\nline2\ttab".to_string());
        assert_eq!(arg.to_javascript(), "\"line1\\nline2\\ttab\"");
    }

    #[test]
    fn test_array_arguments() {
        // Simple array
        let arg = ScriptArgument::from_json(json!([1, 2, 3]));
        let expected = ScriptArgument::Array(vec![
            ScriptArgument::Number(1.0),
            ScriptArgument::Number(2.0),
            ScriptArgument::Number(3.0),
        ]);
        assert_eq!(arg, expected);
        assert_eq!(arg.to_javascript(), "[1, 2, 3]");

        // Mixed types
        let arg = ScriptArgument::from_json(json!([42, "test", true, null]));
        assert_eq!(arg.to_javascript(), "[42, \"test\", true, null]");

        // Empty array
        let arg = ScriptArgument::from_json(json!([]));
        assert_eq!(arg, ScriptArgument::Array(vec![]));
        assert_eq!(arg.to_javascript(), "[]");
    }

    #[test]
    fn test_nested_arrays() {
        let arg = ScriptArgument::from_json(json!([[1, 2], [3, 4]]));
        assert_eq!(arg.to_javascript(), "[[1, 2], [3, 4]]");
    }

    #[test]
    fn test_object_arguments() {
        let arg = ScriptArgument::from_json(json!({"name": "Alice", "age": 30}));
        let js = arg.to_javascript();

        // Object keys may be in any order
        assert!(js.contains("\"name\": \"Alice\""));
        assert!(js.contains("\"age\": 30"));
        assert!(js.starts_with('{'));
        assert!(js.ends_with('}'));

        // Empty object
        let arg = ScriptArgument::from_json(json!({}));
        assert_eq!(arg.to_javascript(), "{}");
    }

    #[test]
    fn test_nested_objects() {
        let arg = ScriptArgument::from_json(json!({
            "user": {
                "name": "Bob",
                "active": true
            },
            "count": 5
        }));
        let js = arg.to_javascript();

        assert!(js.contains("\"user\""));
        assert!(js.contains("\"name\": \"Bob\""));
        assert!(js.contains("\"active\": true"));
        assert!(js.contains("\"count\": 5"));
    }

    #[test]
    fn test_element_reference() {
        // W3C WebDriver element reference format
        let arg = ScriptArgument::from_json(json!({
            "element-6066-11e4-a52e-4f735466cecf": "abc-123-def-456"
        }));

        assert_eq!(arg, ScriptArgument::Element("abc-123-def-456".to_string()));
        assert_eq!(
            arg.to_javascript(),
            "__webdriver_get_element(\"abc-123-def-456\")"
        );
    }

    #[test]
    fn test_element_in_array() {
        let arg = ScriptArgument::from_json(json!([
            {"element-6066-11e4-a52e-4f735466cecf": "elem-1"},
            {"element-6066-11e4-a52e-4f735466cecf": "elem-2"}
        ]));

        let js = arg.to_javascript();
        assert!(js.contains("__webdriver_get_element(\"elem-1\")"));
        assert!(js.contains("__webdriver_get_element(\"elem-2\")"));
    }

    #[test]
    fn test_element_in_object() {
        let arg = ScriptArgument::from_json(json!({
            "target": {"element-6066-11e4-a52e-4f735466cecf": "elem-1"},
            "value": 42
        }));

        let js = arg.to_javascript();
        assert!(js.contains("\"target\": __webdriver_get_element(\"elem-1\")"));
        assert!(js.contains("\"value\": 42"));
    }

    #[test]
    fn test_arguments_to_javascript() {
        let args = vec![
            ScriptArgument::Number(42.0),
            ScriptArgument::String("test".to_string()),
            ScriptArgument::Boolean(true),
        ];

        let js = ScriptArgument::arguments_to_javascript(&args);
        assert_eq!(js, "[42, \"test\", true]");
    }

    #[test]
    fn test_empty_arguments() {
        let args: Vec<ScriptArgument> = vec![];
        let js = ScriptArgument::arguments_to_javascript(&args);
        assert_eq!(js, "[]");
    }

    // ============================================================================
    // Result Parsing Tests
    // ============================================================================

    #[test]
    fn test_parse_null_result() {
        let result = parse_script_result("null").unwrap();
        assert_eq!(result, JsonValue::Null);
    }

    #[test]
    fn test_parse_boolean_results() {
        let result = parse_script_result("true").unwrap();
        assert_eq!(result, JsonValue::Bool(true));

        let result = parse_script_result("false").unwrap();
        assert_eq!(result, JsonValue::Bool(false));
    }

    #[test]
    fn test_parse_number_results() {
        let result = parse_script_result("42").unwrap();
        assert_eq!(result, json!(42));

        let result = parse_script_result("3.14159").unwrap();
        assert_eq!(result, json!(3.14159));
    }

    #[test]
    fn test_parse_string_results() {
        let result = parse_script_result("\"hello\"").unwrap();
        assert_eq!(result, json!("hello"));

        let result = parse_script_result("\"\"").unwrap();
        assert_eq!(result, json!(""));
    }

    #[test]
    fn test_parse_array_results() {
        let result = parse_script_result("[1, 2, 3]").unwrap();
        assert_eq!(result, json!([1, 2, 3]));

        let result = parse_script_result("[]").unwrap();
        assert_eq!(result, json!([]));
    }

    #[test]
    fn test_parse_object_results() {
        let result = parse_script_result(r#"{"name": "Alice", "age": 30}"#).unwrap();
        assert_eq!(result, json!({"name": "Alice", "age": 30}));

        let result = parse_script_result("{}").unwrap();
        assert_eq!(result, json!({}));
    }

    #[test]
    fn test_parse_undefined_result() {
        let result = parse_script_result("undefined").unwrap();
        assert_eq!(result, JsonValue::Null); // undefined -> null per WebDriver spec
    }

    #[test]
    fn test_parse_special_values() {
        // NaN -> null
        let result = parse_script_result("NaN").unwrap();
        assert_eq!(result, JsonValue::Null);

        // Infinity -> null
        let result = parse_script_result("Infinity").unwrap();
        assert_eq!(result, JsonValue::Null);

        let result = parse_script_result("-Infinity").unwrap();
        assert_eq!(result, JsonValue::Null);
    }

    #[test]
    fn test_parse_unquoted_string_fallback() {
        // For non-JSON strings, wrap them
        let result = parse_script_result("simple_value").unwrap();
        assert_eq!(result, JsonValue::String("simple_value".to_string()));
    }

    #[test]
    fn test_parse_invalid_result_error() {
        let result = parse_script_result("[invalid json");
        assert!(result.is_err());
    }
}
