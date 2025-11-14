//! Build script for adblock-engine component
//!
//! Automatically downloads the EasyList filter file if it doesn't exist.
//! This ensures the ad blocker has up-to-date filter rules.

use std::fs;
use std::path::Path;

const EASYLIST_URL: &str = "https://easylist.to/easylist/easylist.txt";
const FILTER_PATH: &str = "../../resources/filters/easylist.txt";

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    let filter_path = Path::new(FILTER_PATH);

    // Create resources/filters directory if it doesn't exist
    if let Some(parent) = filter_path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)
                .expect("Failed to create resources/filters directory");
            println!("Created directory: {:?}", parent);
        }
    }

    // Download EasyList if it doesn't exist or is older than 7 days
    let should_download = if filter_path.exists() {
        // Check file age
        match fs::metadata(filter_path) {
            Ok(metadata) => {
                match metadata.modified() {
                    Ok(modified) => {
                        match modified.elapsed() {
                            Ok(age) => {
                                // Redownload if older than 7 days
                                age.as_secs() > (7 * 24 * 60 * 60)
                            }
                            Err(_) => false, // If we can't check age, don't redownload
                        }
                    }
                    Err(_) => false,
                }
            }
            Err(_) => true,
        }
    } else {
        true
    };

    if should_download {
        println!("Downloading EasyList filter rules...");
        match download_easylist(filter_path) {
            Ok(size) => {
                println!("Successfully downloaded EasyList ({} bytes)", size);
                println!("Filter file: {:?}", filter_path);
            }
            Err(e) => {
                eprintln!("Warning: Failed to download EasyList: {}", e);
                eprintln!("Ad blocking will not be available.");
                eprintln!("You can manually download from: {}", EASYLIST_URL);
                eprintln!("And place it at: {:?}", filter_path);

                // Don't fail the build if download fails
                // The component will handle missing filters gracefully
            }
        }
    } else {
        println!("EasyList filter file exists and is recent");
        if let Ok(metadata) = fs::metadata(filter_path) {
            println!("Filter file size: {} bytes", metadata.len());
        }
    }
}

fn download_easylist(dest_path: &Path) -> Result<u64, Box<dyn std::error::Error>> {
    // Use curl if available (more reliable in build scripts)
    if which("curl") {
        use std::process::Command;

        let output = Command::new("curl")
            .arg("-fsSL")
            .arg(EASYLIST_URL)
            .arg("-o")
            .arg(dest_path)
            .output()?;

        if !output.status.success() {
            return Err(format!(
                "curl failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )
            .into());
        }

        let metadata = fs::metadata(dest_path)?;
        Ok(metadata.len())
    } else {
        // Fallback: try wget
        if which("wget") {
            use std::process::Command;

            let output = Command::new("wget")
                .arg("-q")
                .arg("-O")
                .arg(dest_path)
                .arg(EASYLIST_URL)
                .output()?;

            if !output.status.success() {
                return Err(format!(
                    "wget failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                )
                .into());
            }

            let metadata = fs::metadata(dest_path)?;
            Ok(metadata.len())
        } else {
            Err("Neither curl nor wget found. Cannot download EasyList.".into())
        }
    }
}

fn which(command: &str) -> bool {
    use std::process::Command;

    Command::new("which")
        .arg(command)
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}
