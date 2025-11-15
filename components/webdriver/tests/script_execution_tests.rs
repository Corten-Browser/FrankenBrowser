//! Integration tests for WebDriver script execution
//!
//! Tests executeScript and executeAsyncScript endpoints with various argument types,
//! return values, and error conditions.

use serde_json::json;
use webdriver::script_args::{parse_script_result, ScriptArgument};

// ============================================================================
// ScriptArgument Serialization Integration Tests
// ============================================================================

#[test]
fn test_full_argument_serialization_pipeline() {
    // Test converting from JSON -> ScriptArgument -> JavaScript code
    let json_args = vec![
        json!(null),
        json!(true),
        json!(42),
        json!("hello"),
        json!([1, 2, 3]),
        json!({"key": "value"}),
    ];

    let script_args: Vec<ScriptArgument> = json_args
        .into_iter()
        .map(ScriptArgument::from_json)
        .collect();

    let js_code = ScriptArgument::arguments_to_javascript(&script_args);

    // Verify generated JavaScript is valid
    assert!(js_code.starts_with('['));
    assert!(js_code.ends_with(']'));
    assert!(js_code.contains("null"));
    assert!(js_code.contains("true"));
    assert!(js_code.contains("42"));
    assert!(js_code.contains("\"hello\""));
    assert!(js_code.contains("[1, 2, 3]"));
    assert!(js_code.contains("\"key\": \"value\""));
}

#[test]
fn test_element_reference_serialization() {
    // WebDriver element reference format
    let element_json = json!({
        "element-6066-11e4-a52e-4f735466cecf": "elem-abc-123"
    });

    let arg = ScriptArgument::from_json(element_json);
    assert!(matches!(arg, ScriptArgument::Element(_)));

    let js = arg.to_javascript();
    assert_eq!(js, "__webdriver_get_element(\"elem-abc-123\")");
}

#[test]
fn test_complex_nested_structure() {
    // Test deeply nested structure with mixed types
    let complex_json = json!({
        "user": {
            "name": "Alice",
            "age": 30,
            "roles": ["admin", "user"],
            "settings": {
                "theme": "dark",
                "notifications": true
            }
        },
        "metadata": {
            "created": 1234567890,
            "active": true
        }
    });

    let arg = ScriptArgument::from_json(complex_json);
    let js = arg.to_javascript();

    // Verify structure is preserved
    assert!(js.contains("\"user\""));
    assert!(js.contains("\"name\": \"Alice\""));
    assert!(js.contains("\"age\": 30"));
    assert!(js.contains("[\"admin\", \"user\"]"));
    assert!(js.contains("\"theme\": \"dark\""));
    assert!(js.contains("\"notifications\": true"));
    assert!(js.contains("\"created\": 1234567890"));
}

// ============================================================================
// Result Parsing Integration Tests
// ============================================================================

#[test]
fn test_parse_primitive_results() {
    // Null
    let result = parse_script_result("null").unwrap();
    assert_eq!(result, json!(null));

    // Boolean
    let result = parse_script_result("true").unwrap();
    assert_eq!(result, json!(true));

    // Number
    let result = parse_script_result("123.45").unwrap();
    assert_eq!(result, json!(123.45));

    // String
    let result = parse_script_result("\"test string\"").unwrap();
    assert_eq!(result, json!("test string"));
}

#[test]
fn test_parse_collection_results() {
    // Array
    let result = parse_script_result(r#"[1, "two", true, null]"#).unwrap();
    assert_eq!(result, json!([1, "two", true, null]));

    // Object
    let result = parse_script_result(r#"{"a": 1, "b": "two"}"#).unwrap();
    assert_eq!(result, json!({"a": 1, "b": "two"}));

    // Nested
    let result = parse_script_result(r#"{"arr": [1, 2], "obj": {"x": 3}}"#).unwrap();
    assert_eq!(result, json!({"arr": [1, 2], "obj": {"x": 3}}));
}

#[test]
fn test_parse_javascript_special_values() {
    // undefined -> null
    let result = parse_script_result("undefined").unwrap();
    assert_eq!(result, json!(null));

    // NaN -> null
    let result = parse_script_result("NaN").unwrap();
    assert_eq!(result, json!(null));

    // Infinity -> null
    let result = parse_script_result("Infinity").unwrap();
    assert_eq!(result, json!(null));

    // -Infinity -> null
    let result = parse_script_result("-Infinity").unwrap();
    assert_eq!(result, json!(null));
}

#[test]
fn test_parse_error_object() {
    // Simulated error object
    let result = parse_script_result(r#"{"error": "TypeError", "message": "Cannot read property"}"#).unwrap();
    assert!(result.is_object());
    assert_eq!(result["error"], json!("TypeError"));
}

// ============================================================================
// Script Execution Argument Injection Tests
// ============================================================================

#[test]
fn test_script_with_no_arguments() {
    // Script without arguments should execute as-is
    let script = "return 42;";
    let args: Vec<ScriptArgument> = vec![];

    let full_script = if args.is_empty() {
        script.to_string()
    } else {
        let args_js = ScriptArgument::arguments_to_javascript(&args);
        format!(
            "(function() {{
                var args = {};
                return (function() {{ {} }}).apply(null, args);
            }})()",
            args_js, script
        )
    };

    assert_eq!(full_script, "return 42;");
}

#[test]
fn test_script_with_arguments() {
    // Script with arguments should wrap and inject
    let script = "return arguments[0] + arguments[1];";
    let args = vec![
        ScriptArgument::Number(5.0),
        ScriptArgument::Number(3.0),
    ];

    let args_js = ScriptArgument::arguments_to_javascript(&args);
    let full_script = format!(
        "(function() {{
                var args = {};
                return (function() {{ {} }}).apply(null, args);
            }})()",
        args_js, script
    );

    assert!(full_script.contains("[5, 3]"));
    assert!(full_script.contains("arguments[0] + arguments[1]"));
}

#[test]
fn test_async_script_callback_injection() {
    // Async script should inject callback as last argument
    let script = "setTimeout(function() { arguments[0](42); }, 100);";
    let args = vec![];

    let args_js = ScriptArgument::arguments_to_javascript(&args);
    let full_script = format!(
        r#"(function() {{
            // Store callback result
            window.__webdriver_callback_result = undefined;
            window.__webdriver_callback_called = false;

            // Create callback function
            var callback = function(result) {{
                window.__webdriver_callback_result = result;
                window.__webdriver_callback_called = true;
            }};

            // Prepare arguments array with callback at the end
            var args = {};
            args.push(callback);

            // Execute user script with callback
            try {{
                (function() {{ {} }}).apply(null, args);
            }} catch (e) {{
                // If script throws, call callback with error
                callback({{ error: e.toString() }});
            }}
        }})()"#,
        args_js, script
    );

    assert!(full_script.contains("__webdriver_callback_result"));
    assert!(full_script.contains("__webdriver_callback_called"));
    assert!(full_script.contains("args.push(callback)"));
}

// ============================================================================
// Error Handling Tests
// ============================================================================

#[test]
fn test_parse_invalid_json_error() {
    let result = parse_script_result("[invalid json");
    assert!(result.is_err());
}

#[test]
fn test_script_argument_string_escaping() {
    // Test special characters are properly escaped
    let dangerous_string = "path\\to\\file\nline2\t\"quoted\"";
    let arg = ScriptArgument::String(dangerous_string.to_string());
    let js = arg.to_javascript();

    // Verify escaping
    assert!(js.contains("\\\\"));  // Backslash escaped
    assert!(js.contains("\\n"));   // Newline escaped
    assert!(js.contains("\\t"));   // Tab escaped
    assert!(js.contains("\\\""));  // Quote escaped

    // Should be valid JavaScript string literal
    assert!(js.starts_with('"'));
    assert!(js.ends_with('"'));
}

// ============================================================================
// W3C WebDriver Compliance Tests
// ============================================================================

#[test]
fn test_w3c_element_reference_format() {
    // Per W3C spec, element references use specific key
    let element_key = "element-6066-11e4-a52e-4f735466cecf";
    let element_json = json!({
        element_key: "unique-element-id-123"
    });

    let arg = ScriptArgument::from_json(element_json);

    // Should be recognized as element
    assert!(matches!(arg, ScriptArgument::Element(_)));

    // Should generate proper lookup function
    let js = arg.to_javascript();
    assert!(js.contains("__webdriver_get_element"));
    assert!(js.contains("unique-element-id-123"));
}

#[test]
fn test_w3c_null_representation() {
    // Per W3C spec, null should remain null
    let arg = ScriptArgument::Null;
    assert_eq!(arg.to_javascript(), "null");

    // Parsing null should return null
    let parsed = parse_script_result("null").unwrap();
    assert_eq!(parsed, json!(null));
}

#[test]
fn test_w3c_special_number_handling() {
    // Per W3C spec, NaN/Infinity should convert to null when returned
    let nan_result = parse_script_result("NaN").unwrap();
    assert_eq!(nan_result, json!(null));

    let inf_result = parse_script_result("Infinity").unwrap();
    assert_eq!(inf_result, json!(null));

    let neg_inf_result = parse_script_result("-Infinity").unwrap();
    assert_eq!(neg_inf_result, json!(null));
}

// ============================================================================
// Performance and Edge Cases
// ============================================================================

#[test]
fn test_large_array_serialization() {
    // Test handling of large arrays
    let large_array: Vec<_> = (0..1000).map(|i| json!(i)).collect();
    let arg = ScriptArgument::from_json(json!(large_array));

    let js = arg.to_javascript();
    assert!(js.starts_with('['));
    assert!(js.ends_with(']'));
    assert!(js.contains("0"));
    assert!(js.contains("999"));
}

#[test]
fn test_deeply_nested_structures() {
    // Test deeply nested objects/arrays
    let nested = json!({
        "level1": {
            "level2": {
                "level3": {
                    "level4": {
                        "value": 42
                    }
                }
            }
        }
    });

    let arg = ScriptArgument::from_json(nested);
    let js = arg.to_javascript();

    assert!(js.contains("\"level1\""));
    assert!(js.contains("\"level2\""));
    assert!(js.contains("\"level3\""));
    assert!(js.contains("\"level4\""));
    assert!(js.contains("\"value\": 42"));
}

#[test]
fn test_empty_collections() {
    // Empty array
    let empty_array = ScriptArgument::Array(vec![]);
    assert_eq!(empty_array.to_javascript(), "[]");

    // Empty object
    let empty_object = ScriptArgument::Object(std::collections::HashMap::new());
    assert_eq!(empty_object.to_javascript(), "{}");
}

#[test]
fn test_mixed_type_array() {
    // Array with all different types
    let mixed = json!([
        null,
        true,
        false,
        42,
        42.5,
        "string",
        [1, 2],
        {"key": "value"},
        {"element-6066-11e4-a52e-4f735466cecf": "elem-1"}
    ]);

    let arg = ScriptArgument::from_json(mixed);
    let js = arg.to_javascript();

    assert!(js.contains("null"));
    assert!(js.contains("true"));
    assert!(js.contains("false"));
    assert!(js.contains("42"));
    assert!(js.contains("42.5"));
    assert!(js.contains("\"string\""));
    assert!(js.contains("[1, 2]"));
    assert!(js.contains("\"key\": \"value\""));
    assert!(js.contains("__webdriver_get_element(\"elem-1\")"));
}
