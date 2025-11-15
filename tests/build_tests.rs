//! Tests for build infrastructure
//!
//! This test suite verifies:
//! - Resource files exist
//! - Icons are present
//! - EasyList filters can be downloaded (if not present)
//! - Build script configuration is correct

use std::path::Path;

#[test]
fn test_init_js_exists() {
    let init_js = Path::new("resources/init.js");
    assert!(
        init_js.exists(),
        "resources/init.js should exist for WebView initialization"
    );

    // Verify it's not empty
    let content = std::fs::read_to_string(init_js).expect("Failed to read init.js");
    assert!(
        content.len() > 100,
        "init.js should contain substantial initialization code"
    );

    // Verify it contains key components
    assert!(
        content.contains("window.ipc"),
        "init.js should define IPC interface"
    );
    assert!(
        content.contains("console.log"),
        "init.js should override console methods"
    );
    assert!(
        content.contains("dom-ready"),
        "init.js should send DOM ready event"
    );
}

#[test]
fn test_browser_icon_png_exists() {
    let icon_path = Path::new("resources/icons/browser.png");
    assert!(
        icon_path.exists(),
        "resources/icons/browser.png should exist for Linux/macOS"
    );

    // Verify it's a valid file (not empty)
    let metadata = std::fs::metadata(icon_path).expect("Failed to read icon metadata");
    assert!(metadata.len() > 0, "Icon file should not be empty");
}

#[test]
fn test_browser_icon_ico_exists() {
    let icon_path = Path::new("resources/icons/browser.ico");
    assert!(
        icon_path.exists(),
        "resources/icons/browser.ico should exist for Windows"
    );

    // Verify it's a valid file (not empty)
    let metadata = std::fs::metadata(icon_path).expect("Failed to read icon metadata");
    assert!(metadata.len() > 0, "Icon file should not be empty");
}

#[test]
fn test_filters_directory_exists() {
    let filters_dir = Path::new("resources/filters");
    assert!(
        filters_dir.exists(),
        "resources/filters directory should exist"
    );
    assert!(
        filters_dir.is_dir(),
        "resources/filters should be a directory"
    );
}

#[test]
fn test_easylist_filter_accessible() {
    let easylist_path = Path::new("resources/filters/easylist.txt");

    if easylist_path.exists() {
        // Verify it's not empty
        let metadata = std::fs::metadata(easylist_path).expect("Failed to read easylist metadata");
        assert!(
            metadata.len() > 1000,
            "EasyList file should contain filter rules"
        );

        // Verify it contains filter syntax
        let content = std::fs::read_to_string(easylist_path).expect("Failed to read easylist");
        assert!(
            content.contains("||") || content.contains("##"),
            "EasyList should contain ad blocking filter syntax"
        );
    } else {
        // If not present, that's okay - build script will download it
        eprintln!("Note: easylist.txt not present, will be downloaded during build");
    }
}

#[test]
fn test_easyprivacy_filter_accessible() {
    let easyprivacy_path = Path::new("resources/filters/easyprivacy.txt");

    if easyprivacy_path.exists() {
        // Verify it's not empty
        let metadata =
            std::fs::metadata(easyprivacy_path).expect("Failed to read easyprivacy metadata");
        assert!(
            metadata.len() > 1000,
            "EasyPrivacy file should contain filter rules"
        );
    } else {
        // If not present, that's okay - build script will download it
        eprintln!("Note: easyprivacy.txt not present, will be downloaded during build");
    }
}

#[test]
fn test_build_script_exists() {
    let build_script = Path::new("build.rs");
    assert!(
        build_script.exists(),
        "build.rs should exist at project root"
    );

    // Verify it contains expected build logic
    let content = std::fs::read_to_string(build_script).expect("Failed to read build.rs");
    assert!(
        content.contains("EASYLIST_URL"),
        "build.rs should define EasyList download URL"
    );
    assert!(
        content.contains("download_file"),
        "build.rs should have download functionality"
    );

    // Platform-specific checks
    #[cfg(target_os = "windows")]
    assert!(
        content.contains("winres"),
        "build.rs should use winres on Windows"
    );
}

#[test]
fn test_init_js_ipc_interface() {
    let init_js = Path::new("resources/init.js");
    let content = std::fs::read_to_string(init_js).expect("Failed to read init.js");

    // Verify IPC interface methods
    assert!(
        content.contains("send:") || content.contains("ipc.send"),
        "init.js should define ipc.send method"
    );
    assert!(
        content.contains("on:") || content.contains("ipc.on"),
        "init.js should define ipc.on method for callbacks"
    );
    assert!(
        content.contains("_dispatch:") || content.contains("ipc._dispatch"),
        "init.js should define internal dispatch method"
    );
}

#[test]
fn test_init_js_console_redirection() {
    let init_js = Path::new("resources/init.js");
    let content = std::fs::read_to_string(init_js).expect("Failed to read init.js");

    // Verify console methods are redirected
    assert!(
        content.contains("console.log ="),
        "init.js should override console.log"
    );
    assert!(
        content.contains("console.error ="),
        "init.js should override console.error"
    );
    assert!(
        content.contains("console.warn ="),
        "init.js should override console.warn"
    );
}

#[test]
fn test_init_js_error_handlers() {
    let init_js = Path::new("resources/init.js");
    let content = std::fs::read_to_string(init_js).expect("Failed to read init.js");

    // Verify error handling
    assert!(
        content.contains("addEventListener('error'"),
        "init.js should register global error handler"
    );
    assert!(
        content.contains("addEventListener('unhandledrejection'"),
        "init.js should handle unhandled promise rejections"
    );
}

#[test]
fn test_init_js_dom_events() {
    let init_js = Path::new("resources/init.js");
    let content = std::fs::read_to_string(init_js).expect("Failed to read init.js");

    // Verify DOM event listeners
    assert!(
        content.contains("DOMContentLoaded"),
        "init.js should listen for DOM ready"
    );
    assert!(
        content.contains("addEventListener('load'"),
        "init.js should listen for page load"
    );
    assert!(
        content.contains("addEventListener('beforeunload'"),
        "init.js should listen for page unload"
    );
}

#[test]
fn test_init_js_browser_api() {
    let init_js = Path::new("resources/init.js");
    let content = std::fs::read_to_string(init_js).expect("Failed to read init.js");

    // Verify browser API extensions
    assert!(
        content.contains("window.frankenbrowser"),
        "init.js should define frankenbrowser API"
    );
    assert!(
        content.contains("navigate:"),
        "init.js should provide navigate function"
    );
    assert!(
        content.contains("back:"),
        "init.js should provide back function"
    );
    assert!(
        content.contains("forward:"),
        "init.js should provide forward function"
    );
    assert!(
        content.contains("reload:"),
        "init.js should provide reload function"
    );
}

#[test]
fn test_resource_directory_structure() {
    // Verify expected directory structure
    assert!(
        Path::new("resources").exists(),
        "resources directory should exist"
    );
    assert!(
        Path::new("resources/icons").exists(),
        "resources/icons directory should exist"
    );
    assert!(
        Path::new("resources/filters").exists(),
        "resources/filters directory should exist"
    );
    assert!(
        Path::new("resources/config").exists(),
        "resources/config directory should exist"
    );
}

#[cfg(test)]
mod build_verification {
    use super::*;

    #[test]
    fn verify_all_resources_present() {
        let required_files = vec![
            "resources/init.js",
            "resources/icons/browser.png",
            "resources/icons/browser.ico",
        ];

        let mut missing = Vec::new();
        for file in &required_files {
            if !Path::new(file).exists() {
                missing.push(file);
            }
        }

        assert!(
            missing.is_empty(),
            "Missing required resource files: {:?}",
            missing
        );
    }

    #[test]
    fn verify_build_dependencies_configured() {
        // This test verifies that Cargo.toml has necessary build dependencies
        let cargo_toml = Path::new("Cargo.toml");
        assert!(cargo_toml.exists(), "Cargo.toml should exist");

        let content = std::fs::read_to_string(cargo_toml).expect("Failed to read Cargo.toml");
        assert!(
            content.contains("[build-dependencies]"),
            "Cargo.toml should have build-dependencies section"
        );
        assert!(
            content.contains("reqwest"),
            "build-dependencies should include reqwest for downloads"
        );
    }
}
