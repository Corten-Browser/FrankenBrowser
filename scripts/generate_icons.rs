#!/usr/bin/env rust-script
//! Generate placeholder icons for FrankenBrowser
//!
//! ```cargo
//! [dependencies]
//! image = "0.24"
//! ```

use image::{ImageBuffer, Rgba, RgbaImage};
use std::path::Path;

fn create_browser_icon(size: u32) -> RgbaImage {
    let mut img = ImageBuffer::new(size, size);

    // Background color - blue
    let bg_color = Rgba([66, 135, 245, 255]);
    let white = Rgba([255, 255, 255, 255]);
    let light_blue = Rgba([240, 245, 255, 255]);
    let line_color = Rgba([150, 180, 220, 255]);

    let margin = size / 10;

    // Fill background
    for y in 0..size {
        for x in 0..size {
            if x >= margin && x < size - margin && y >= margin && y < size - margin {
                img.put_pixel(x, y, bg_color);
            }
        }
    }

    // Draw address bar
    let bar_margin = margin * 2;
    let bar_height = size / 8;
    for y in bar_margin..bar_margin + bar_height {
        for x in bar_margin..size - bar_margin {
            img.put_pixel(x, y, white);
        }
    }

    // Draw content area
    let content_y = bar_margin + bar_height + margin;
    for y in content_y..size - margin {
        for x in bar_margin..size - bar_margin {
            img.put_pixel(x, y, light_blue);
        }
    }

    // Draw content lines
    let line_margin = bar_margin + size / 20;
    let mut line_y = content_y + size / 20;
    let line_spacing = size / 15;
    let line_height = size / 60;

    for _ in 0..3 {
        for y in line_y..line_y + line_height {
            for x in line_margin..size - bar_margin - size / 20 {
                img.put_pixel(x, y, line_color);
            }
        }
        line_y += line_spacing;
    }

    img
}

fn main() {
    let icon_256 = create_browser_icon(256);

    // Save as PNG
    icon_256
        .save("resources/icons/browser.png")
        .expect("Failed to save PNG");
    println!("Created: resources/icons/browser.png");

    // For ICO, save multiple sizes
    // Note: image crate doesn't directly support ICO format,
    // but we'll create a PNG that can be used on all platforms
    println!("Icon generation complete!");
}
