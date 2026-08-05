#!/usr/bin/env python3
"""
Generate Sentinel Authenticator app icons in all required sizes/formats.

Outputs to /home/z/my-project/src-tauri/icons/:
  - icon.png          (512x512, master)
  - 32x32.png
  - 128x128.png
  - 128x128@2x.png   (256x256)
  - icon.ico           (multi-size Windows ICO)
  - icon.icns          (macOS ICNS — empty placeholder, generated on macOS runner)
  - Square*.png        (Windows tile images, optional)
"""

import struct
import io
from pathlib import Path

from PIL import Image
import cairosvg

ICON_DIR = Path("/home/z/my-project/src-tauri/icons")
ICON_DIR.mkdir(parents=True, exist_ok=True)

# Sentinel shield SVG.
SVG = """<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512" width="512" height="512">
  <defs>
    <linearGradient id="bg" x1="0" y1="0" x2="0" y2="512" gradientUnits="userSpaceOnUse">
      <stop offset="0" stop-color="#1E1E26"/>
      <stop offset="1" stop-color="#0F0F14"/>
    </linearGradient>
    <linearGradient id="shield" x1="256" y1="80" x2="256" y2="448" gradientUnits="userSpaceOnUse">
      <stop offset="0" stop-color="#60A5FA"/>
      <stop offset="1" stop-color="#3B82F6"/>
    </linearGradient>
    <filter id="shadow" x="-20%" y="-20%" width="140%" height="140%">
      <feGaussianBlur in="SourceAlpha" stdDeviation="6"/>
      <feOffset dx="0" dy="4" result="offsetblur"/>
      <feComponentTransfer><feFuncA type="linear" slope="0.5"/></feComponentTransfer>
      <feMerge><feMergeNode/><feMergeNode in="SourceGraphic"/></feMerge>
    </filter>
  </defs>
  <rect width="512" height="512" rx="96" fill="url(#bg)"/>
  <g filter="url(#shadow)">
    <path d="M256 96 L128 144 L128 256 C128 336 184 396 256 416 C328 396 384 336 384 256 L384 144 Z"
          fill="url(#shield)" stroke="#93C5FD" stroke-width="4" stroke-linejoin="round"/>
    <circle cx="256" cy="232" r="36" fill="#FFFFFF" fill-opacity="0.96"/>
    <rect x="240" y="252" width="32" height="84" rx="16" fill="#FFFFFF" fill-opacity="0.96"/>
  </g>
</svg>
"""

def render_png(size: int) -> Image.Image:
    png_bytes = cairosvg.svg2png(
        bytestring=SVG.encode("utf-8"),
        output_width=size,
        output_height=size,
    )
    return Image.open(io.BytesIO(png_bytes)).convert("RGBA")

def write_png(img: Image.Image, path: Path):
    img.save(path, format="PNG", optimize=True)
    print(f"  wrote {path.name} ({path.stat().st_size} bytes)")

def write_ico(sizes: list[int], path: Path):
    header = struct.pack("<HHH", 0, 1, len(sizes))
    entries = b""
    images_data = b""
    offset = 6 + 16 * len(sizes)
    for size in sizes:
        img = render_png(size)
        buf = io.BytesIO()
        img.save(buf, format="PNG", optimize=True)
        png_data = buf.getvalue()
        w = size if size < 256 else 0
        h = size if size < 256 else 0
        entries += struct.pack(
            "<BBBBHHII",
            w, h, 0, 0, 1, 32, len(png_data), offset,
        )
        images_data += png_data
        offset += len(png_data)
    with open(path, "wb") as f:
        f.write(header)
        f.write(entries)
        f.write(images_data)
    print(f"  wrote {path.name} ({path.stat().st_size} bytes, {len(sizes)} sizes)")

def write_icns_placeholder(path: Path):
    img = render_png(512)
    buf = io.BytesIO()
    img.save(buf, format="PNG", optimize=True)
    png_data = buf.getvalue()
    total_len = 8 + 8 + len(png_data)
    with open(path, "wb") as f:
        f.write(b"icns")
        f.write(struct.pack(">I", total_len))
        f.write(b"ic09")
        f.write(struct.pack(">I", 8 + len(png_data)))
        f.write(png_data)
    print(f"  wrote {path.name} ({path.stat().st_size} bytes — macOS placeholder)")

def main():
    print(f"Generating Sentinel icons in {ICON_DIR}")
    write_png(render_png(512), ICON_DIR / "icon.png")
    write_png(render_png(32), ICON_DIR / "32x32.png")
    write_png(render_png(128), ICON_DIR / "128x128.png")
    write_png(render_png(256), ICON_DIR / "128x128@2x.png")
    for s in [30, 44, 71, 89, 107, 142, 150, 284, 310]:
        write_png(render_png(s), ICON_DIR / f"Square{s}x{s}Logo.png")
    write_ico([16, 24, 32, 48, 64, 128, 256], ICON_DIR / "icon.ico")
    write_icns_placeholder(ICON_DIR / "icon.icns")
    print("Done.")

if __name__ == "__main__":
    main()
