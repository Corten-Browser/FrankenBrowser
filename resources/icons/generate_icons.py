#!/usr/bin/env python3
"""
Generate placeholder icons for FrankenBrowser
Creates simple colored placeholder icons for Windows (.ico) and Linux/macOS (.png)
"""

from PIL import Image, ImageDraw, ImageFont
import os

def create_browser_icon(size=256):
    """Create a simple browser icon"""
    # Create a new image with a gradient blue background
    img = Image.new('RGBA', (size, size), (255, 255, 255, 0))
    draw = ImageDraw.Draw(img)

    # Draw a rounded rectangle background (browser window)
    margin = size // 10
    draw.rounded_rectangle(
        [(margin, margin), (size - margin, size - margin)],
        radius=size // 20,
        fill=(66, 135, 245, 255),  # Blue color
        outline=(40, 90, 200, 255),
        width=size // 50
    )

    # Draw address bar
    bar_margin = margin * 2
    bar_height = size // 8
    draw.rounded_rectangle(
        [(bar_margin, bar_margin), (size - bar_margin, bar_margin + bar_height)],
        radius=size // 40,
        fill=(255, 255, 255, 255),
        outline=(200, 200, 200, 255),
        width=2
    )

    # Draw content area
    content_y = bar_margin + bar_height + margin
    draw.rectangle(
        [(bar_margin, content_y), (size - bar_margin, size - margin)],
        fill=(240, 245, 255, 255)
    )

    # Draw some lines to simulate content
    line_margin = bar_margin + size // 20
    line_y = content_y + size // 20
    line_spacing = size // 15
    for i in range(3):
        y = line_y + i * line_spacing
        draw.rectangle(
            [(line_margin, y), (size - bar_margin - size // 20, y + size // 60)],
            fill=(150, 180, 220, 255)
        )

    return img

def save_png(img, path):
    """Save image as PNG"""
    img.save(path, 'PNG')
    print(f"Created PNG: {path}")

def save_ico(img, path):
    """Save image as ICO (Windows icon)"""
    # ICO format supports multiple sizes
    sizes = [(256, 256), (128, 128), (64, 64), (48, 48), (32, 32), (16, 16)]
    images = [img.resize(size, Image.Resampling.LANCZOS) for size in sizes]
    images[0].save(path, format='ICO', sizes=sizes)
    print(f"Created ICO: {path}")

def main():
    script_dir = os.path.dirname(os.path.abspath(__file__))

    # Create the icon
    icon = create_browser_icon(256)

    # Save as PNG for Linux/macOS
    png_path = os.path.join(script_dir, 'browser.png')
    save_png(icon, png_path)

    # Save as ICO for Windows
    ico_path = os.path.join(script_dir, 'browser.ico')
    save_ico(icon, ico_path)

    print("Icon generation complete!")

if __name__ == '__main__':
    main()
