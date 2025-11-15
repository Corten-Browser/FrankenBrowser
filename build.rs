//! Build script for FrankenBrowser
//!
//! This script performs platform-specific build tasks:
//! - Windows: Compiles application icon into executable
//! - All platforms: Downloads EasyList filters if not present
//! - All platforms: Verifies resource files exist

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

/// EasyList download URL
const EASYLIST_URL: &str = "https://easylist.to/easylist/easylist.txt";

/// EasyPrivacy download URL
const EASYPRIVACY_URL: &str = "https://easylist.to/easylist/easyprivacy.txt";

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=resources/");

    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();

    // Platform-specific builds
    match target_os.as_str() {
        "windows" => build_windows(),
        "linux" => build_linux(),
        "macos" => build_macos(),
        _ => println!("cargo:warning=Unknown target OS: {}", target_os),
    }

    // Download filters if not present
    ensure_filters_exist();

    // Verify resources exist
    verify_resources();
}

/// Windows-specific build configuration
#[cfg(target_os = "windows")]
fn build_windows() {
    println!("cargo:warning=Building for Windows");

    // Compile application icon
    if Path::new("resources/icons/browser.ico").exists() {
        let mut res = winres::WindowsResource::new();
        res.set_icon("resources/icons/browser.ico");
        res.set("ProductName", "FrankenBrowser");
        res.set("FileDescription", "A frankenstein web browser");
        res.set("CompanyName", "Browser Team");
        res.set("LegalCopyright", "MIT OR Apache-2.0");

        if let Err(e) = res.compile() {
            println!("cargo:warning=Failed to compile Windows resource: {}", e);
        } else {
            println!("cargo:warning=Windows icon compiled successfully");
        }
    } else {
        println!("cargo:warning=Windows icon not found at resources/icons/browser.ico");
    }
}

/// Linux-specific build configuration
#[cfg(target_os = "linux")]
fn build_linux() {
    println!("cargo:warning=Building for Linux");

    // Linux-specific build steps (if any)
    // Currently just verification that icon exists
    if !Path::new("resources/icons/browser.png").exists() {
        println!("cargo:warning=Linux icon not found at resources/icons/browser.png");
    }
}

/// macOS-specific build configuration
#[cfg(target_os = "macos")]
fn build_macos() {
    println!("cargo:warning=Building for macOS");

    // macOS-specific build steps (if any)
    // Currently just verification that icon exists
    if !Path::new("resources/icons/browser.png").exists() {
        println!("cargo:warning=macOS icon not found at resources/icons/browser.png");
    }
}

/// Fallback for non-Windows platforms
#[cfg(not(target_os = "windows"))]
fn build_windows() {
    // No-op on non-Windows platforms
}

/// Fallback for non-Linux platforms
#[cfg(not(target_os = "linux"))]
fn build_linux() {
    // No-op on non-Linux platforms
}

/// Fallback for non-macOS platforms
#[cfg(not(target_os = "macos"))]
fn build_macos() {
    // No-op on non-macOS platforms
}

/// Ensure EasyList filter files exist, download if missing
fn ensure_filters_exist() {
    let filters_dir = PathBuf::from("resources/filters");

    // Create filters directory if it doesn't exist
    if !filters_dir.exists() {
        if let Err(e) = fs::create_dir_all(&filters_dir) {
            println!("cargo:warning=Failed to create filters directory: {}", e);
            return;
        }
    }

    let easylist_path = filters_dir.join("easylist.txt");
    let easyprivacy_path = filters_dir.join("easyprivacy.txt");

    // Download EasyList if not present
    if !easylist_path.exists() {
        println!("cargo:warning=EasyList not found, downloading...");
        if let Err(e) = download_file(EASYLIST_URL, &easylist_path) {
            println!("cargo:warning=Failed to download EasyList: {}", e);
            println!("cargo:warning=Browser will work but ad blocking will be limited");
        } else {
            println!("cargo:warning=EasyList downloaded successfully");
        }
    } else {
        println!("cargo:warning=EasyList found at {:?}", easylist_path);
    }

    // Download EasyPrivacy if not present
    if !easyprivacy_path.exists() {
        println!("cargo:warning=EasyPrivacy not found, downloading...");
        if let Err(e) = download_file(EASYPRIVACY_URL, &easyprivacy_path) {
            println!("cargo:warning=Failed to download EasyPrivacy: {}", e);
            println!("cargo:warning=Browser will work but privacy protection will be limited");
        } else {
            println!("cargo:warning=EasyPrivacy downloaded successfully");
        }
    } else {
        println!("cargo:warning=EasyPrivacy found at {:?}", easyprivacy_path);
    }
}

/// Download a file from URL to path
fn download_file(url: &str, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    // Use reqwest blocking client for build script
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()?;

    let response = client.get(url).send()?;

    if !response.status().is_success() {
        return Err(format!("HTTP error: {}", response.status()).into());
    }

    let content = response.text()?;
    fs::write(path, content)?;

    Ok(())
}

/// Verify required resource files exist
fn verify_resources() {
    let required_files = vec![
        "resources/init.js",
        "resources/icons/browser.png",
    ];

    let mut missing = Vec::new();

    for file in required_files {
        if !Path::new(file).exists() {
            missing.push(file);
        }
    }

    if !missing.is_empty() {
        println!("cargo:warning=Missing resource files:");
        for file in missing {
            println!("cargo:warning=  - {}", file);
        }
        println!("cargo:warning=Build may fail or browser may not work correctly");
    } else {
        println!("cargo:warning=All required resources verified");
    }
}
