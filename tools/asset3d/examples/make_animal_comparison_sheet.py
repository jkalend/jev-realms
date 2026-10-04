#!/usr/bin/env python3
"""Generate a high-resolution side-by-side comparison sheet between the original
and improved animal / beast models (Wolf, Bear, Rat, GnawThane, PaleStag).
"""

from __future__ import annotations

from pathlib import Path
from PIL import Image, ImageDraw, ImageFont

ROOT = Path(__file__).resolve().parents[3]
ORIGINAL_ROOT = ROOT / "docs/gfx/proto/visual-v2/npc-roster-v2"
IMPROVED_ROOT = ROOT / "docs/gfx/proto/visual-v2/animal-study"
OUTPUT_IMAGE = ROOT / "docs/gfx/proto/visual-v2/animal-study/animals_before_after_comparison.png"

ANIMALS = [
    ("Wolf", "Beast (Wild Predator)", "Heavy 360° full-neck fur mane (dorsal crest, throat tufts, wide lateral ruffs), seamless shoulder/haunch joints, brush tail"),
    ("Bear", "Beast (Apex Predator)", "Massive grizzly shoulder hump, seamless muscular shoulders/haunches, honey-tan muzzle mask, 5 curved digging claws"),
    ("Rat", "Beast (Warren Scuttler)", "Kyphotic hunchback arch, seamless flank & leg joints, sniffing wedge snout, smooth pink paws/ears, flexible sinuous tail"),
    ("GnawThane", "Boss (Rat-King Below)", "Hulking hunched frame, seamless shoulder/hip musculature, dorsal crystal spine shards, shard crown, glowing scarlet eyes"),
    ("PaleStag", "Boss (Spectral Monarch)", "Regal proud neck, seamless cervid shoulders/haunches, deep chest, 10-point imperial branching antler rack"),
]

LANES = [
    ("idle", "front", "000.png", "Front"),
    ("idle", "front_right", "000.png", "3/4 View"),
    ("idle", "right", "000.png", "Profile"),
    ("idle", "back", "000.png", "Rear"),
    ("walk", "front_right", "000.png", "Walk"),
    ("attack", "front_right", "001.png", "Attack Lunge"),
]

THUMB_W = 120
THUMB_H = 150


def get_fonts():
    try:
        return (
            ImageFont.truetype("C:/Windows/Fonts/georgia.ttf", 26),
            ImageFont.truetype("C:/Windows/Fonts/georgiab.ttf", 16),
            ImageFont.truetype("C:/Windows/Fonts/consola.ttf", 13),
            ImageFont.truetype("C:/Windows/Fonts/consola.ttf", 11),
        )
    except OSError:
        d = ImageFont.load_default()
        return d, d, d, d


def main():
    title_font, heading_font, sub_font, small_font = get_fonts()

    card_padding = 16
    lane_gap = 10
    grid_w = len(LANES) * (THUMB_W + lane_gap) - lane_gap
    row_block_w = grid_w + 140
    row_block_h = 2 * (THUMB_H + 34) + 68

    total_w = row_block_w + card_padding * 2
    total_h = 100 + len(ANIMALS) * (row_block_h + 16) + 40

    canvas = Image.new("RGBA", (total_w, total_h), (10, 13, 16, 255))
    draw = ImageDraw.Draw(canvas)

    # Title header
    draw.text((card_padding, 20), "ANIMAL MODEL REVIEW & COMPARISON", fill=(235, 225, 205), font=title_font)
    draw.text((card_padding, 56), "Pre-Promotion Visual Quality Pass — Side-by-side review of Original vs. Improved Roster Geometry", fill=(160, 150, 130), font=sub_font)
    draw.line((card_padding, 85, total_w - card_padding, 85), fill=(70, 60, 42, 255), width=2)

    y_offset = 105

    for name, subtitle, highlights in ANIMALS:
        card_top = y_offset
        card_bottom = card_top + row_block_h

        # Card background
        draw.rounded_rectangle((card_padding, card_top, total_w - card_padding, card_bottom),
                               radius=6, fill=(18, 22, 27, 255), outline=(42, 48, 54, 255), width=1)

        # Header bar in card
        draw.text((card_padding + 16, card_top + 12), name.upper(), fill=(215, 178, 88), font=heading_font)
        draw.text((card_padding + 150, card_top + 14), f"—  {subtitle}", fill=(170, 165, 150), font=sub_font)
        draw.text((card_padding + 16, card_top + 34), f"Key Enhancements: {highlights}", fill=(130, 160, 140), font=small_font)

        # Draw Original Row and Improved Row
        for row_idx, (version_label, root_dir, badge_color) in enumerate([
            ("ORIGINAL", ORIGINAL_ROOT, (180, 80, 80)),
            ("IMPROVED", IMPROVED_ROOT, (80, 180, 120))
        ]):
            row_y = card_top + 60 + row_idx * (THUMB_H + 34)

            # Badge
            draw.rounded_rectangle((card_padding + 16, row_y + 40, card_padding + 115, row_y + 68),
                                   radius=4, fill=(badge_color[0]//4, badge_color[1]//4, badge_color[2]//4, 255),
                                   outline=badge_color, width=1)
            draw.text((card_padding + 26, row_y + 46), version_label, fill=badge_color, font=small_font)

            # Lanes
            for lane_idx, (anim, direct, frame_file, label) in enumerate(LANES):
                x = card_padding + 130 + lane_idx * (THUMB_W + lane_gap)
                y = row_y

                # Label on top
                if row_idx == 0:
                    draw.text((x + THUMB_W // 2, card_top + 52), label, fill=(180, 175, 160), font=small_font, anchor="mb")

                img_path = root_dir / name / "render" / name / anim / direct / frame_file
                if not img_path.is_file():
                    # Fallback to any frame in direct
                    candidates = sorted((root_dir / name / "render" / name / anim / direct).glob("*.png"))
                    img_path = candidates[-1] if candidates else img_path

                if img_path.is_file():
                    with Image.open(img_path) as im:
                        im = im.convert("RGBA")
                        im.thumbnail((THUMB_W, THUMB_H), Image.Resampling.LANCZOS)
                        px = x + (THUMB_W - im.width) // 2
                        py = y + (THUMB_H - im.height) // 2
                        # Cell backdrop box
                        draw.rounded_rectangle((x, y, x + THUMB_W, y + THUMB_H), radius=3, fill=(12, 14, 18, 255), outline=(32, 36, 42, 255))
                        canvas.alpha_composite(im, (px, py))
                else:
                    draw.rounded_rectangle((x, y, x + THUMB_W, y + THUMB_H), radius=3, fill=(24, 18, 18, 255), outline=(80, 40, 40, 255))
                    draw.text((x + THUMB_W // 2, y + THUMB_H // 2), "N/A", fill=(140, 100, 100), font=small_font, anchor="mm")

        y_offset += row_block_h + 16

    OUTPUT_IMAGE.parent.mkdir(parents=True, exist_ok=True)
    canvas.convert("RGB").save(OUTPUT_IMAGE, quality=95)
    print("SAVED_COMPARISON_SHEET", OUTPUT_IMAGE)


if __name__ == "__main__":
    main()
