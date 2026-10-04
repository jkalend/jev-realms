#!/usr/bin/env python3
"""
Generate side-by-side comparison boards for the Backdrop Texture Experiment:
Flat Procedural Gradient (Before) vs Painterly Environmental Texture (After).
"""

import os
from PIL import Image, ImageDraw, ImageFont

PROTO_DIR = os.path.join(os.path.dirname(os.path.dirname(__file__)), "docs", "gfx", "proto")
COMP_DIR = os.path.join(PROTO_DIR, "visual-v3", "comparisons")
os.makedirs(COMP_DIR, exist_ok=True)

COMPARISONS = [
    {
        "title": "Town / Millbrook Overworld — Sky & Cloud Strata",
        "before": os.path.join(PROTO_DIR, "e4-iso-gradient.png"),
        "after": os.path.join(PROTO_DIR, "e4-iso.png"),
        "out": "Backdrop_Millbrook_Comparison.png",
    },
    {
        "title": "Underkeep Floor 3 (Lich Boss) — Subterranean Cyclopean Chasm & Cyan Moss",
        "before": os.path.join(PROTO_DIR, "e4-iso-lich-gradient.png"),
        "after": os.path.join(PROTO_DIR, "e4-iso-lich.png"),
        "out": "Backdrop_Lich_Comparison.png",
    },
    {
        "title": "Final Trial (Adjudicator) — Imperial Astral Void & Golden Constellations",
        "before": os.path.join(PROTO_DIR, "levels_gradient", "12.png"),
        "after": os.path.join(PROTO_DIR, "levels", "12.png"),
        "out": "Backdrop_Adjudicator_Comparison.png",
    },
    {
        "title": "Crag Ridge (Cragmother) — Volcanic Basalt Caldera & Molten Lava Fissures",
        "before": os.path.join(PROTO_DIR, "levels_gradient", "13.png"),
        "after": os.path.join(PROTO_DIR, "levels", "13.png"),
        "out": "Backdrop_Cragmother_Comparison.png",
    },
    {
        "title": "Saltmarsh (Tidemother) — Deep Oceanic Swells & Coastal Sea Foam",
        "before": os.path.join(PROTO_DIR, "levels_gradient", "14.png"),
        "after": os.path.join(PROTO_DIR, "levels", "14.png"),
        "out": "Backdrop_Tidemother_Comparison.png",
    },
    {
        "title": "Fen Barrow (Gnaw-Thane) — Damp Slate Chasm & Ethereal Mist",
        "before": os.path.join(PROTO_DIR, "levels_gradient", "15.png"),
        "after": os.path.join(PROTO_DIR, "levels", "15.png"),
        "out": "Backdrop_FenBarrow_Comparison.png",
    },
]

def build_comparison_board(item):
    if not os.path.exists(item["before"]) or not os.path.exists(item["after"]):
        print(f"Skipping {item['out']}, missing file(s)")
        return

    im_before = Image.open(item["before"]).convert("RGBA")
    im_after = Image.open(item["after"]).convert("RGBA")

    # Resize to standardized display width (e.g. 960x540 each)
    target_w, target_h = 960, 540
    im_before = im_before.resize((target_w, target_h), Image.Resampling.LANCZOS)
    im_after = im_after.resize((target_w, target_h), Image.Resampling.LANCZOS)

    header_h = 80
    border = 16
    gap = 20
    total_w = border * 2 + target_w * 2 + gap
    total_h = header_h + target_h + border * 2

    canvas = Image.new("RGBA", (total_w, total_h), (11, 14, 16, 255)) # #0b0e10
    draw = ImageDraw.Draw(canvas)

    # Title header
    font_path = r"assets\fonts\Cinzel-Regular.ttf"
    title_font = None
    sub_font = None
    if os.path.exists(font_path):
        try:
            title_font = ImageFont.truetype(font_path, 28)
            sub_font = ImageFont.truetype(font_path, 18)
        except Exception:
            pass

    # Draw header text
    title_text = item["title"]
    draw.text((border, 16), title_text, fill=(231, 222, 198, 255), font=title_font)
    draw.text((border, 50), "ENVIRONMENTAL BACKDROP EXPERIMENT: BEFORE vs AFTER", fill=(197, 164, 93, 255), font=sub_font)

    # Paste panels
    x_before = border
    y_panels = header_h + border
    x_after = border + target_w + gap

    canvas.paste(im_before, (x_before, y_panels))
    canvas.paste(im_after, (x_after, y_panels))

    # Outer border rings for plates
    draw.rectangle([x_before - 2, y_panels - 2, x_before + target_w + 1, y_panels + target_h + 1], outline=(59, 56, 46, 255), width=2)
    draw.rectangle([x_after - 2, y_panels - 2, x_after + target_w + 1, y_panels + target_h + 1], outline=(81, 69, 46, 255), width=2)

    # Badges
    # Before badge
    draw.rectangle([x_before + 16, y_panels + 16, x_before + 320, y_panels + 52], fill=(20, 24, 26, 230), outline=(59, 56, 46, 255))
    draw.text((x_before + 28, y_panels + 22), "BEFORE: Flat Gradient Backdrop", fill=(166, 162, 148, 255), font=sub_font)

    # After badge
    draw.rectangle([x_after + 16, y_panels + 16, x_after + 350, y_panels + 52], fill=(27, 51, 36, 235), outline=(40, 84, 56, 255))
    draw.text((x_after + 28, y_panels + 22), "AFTER: Painterly Textured Backdrop", fill=(92, 196, 138, 255), font=sub_font)

    out_path = os.path.join(COMP_DIR, item["out"])
    canvas.save(out_path, "PNG", optimize=True)
    print(f"Generated comparison board: {out_path}")

def main():
    for item in COMPARISONS:
        build_comparison_board(item)

if __name__ == "__main__":
    main()
