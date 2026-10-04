#!/usr/bin/env python3
"""Generate Before/After comparison card for the Lich boss."""

from pathlib import Path
from PIL import Image, ImageDraw, ImageFont

V2_LICH = Path("docs/gfx/proto/visual-v2/lich-blender-5.1/render/Lich")
V3_LICH = Path("docs/gfx/proto/visual-v3/lich-blender-5.1/render/Lich")
OUT_FILE = Path("docs/gfx/proto/visual-v3/comparisons/Lich_before_after.png")

LANES = (
    (("idle", "front", "000.png"), "Idle Front"),
    (("idle", "front_right", "000.png"), "Idle 3/4"),
    (("walk", "front", "000.png"), "Walk"),
    (("attack", "front", "000.png"), "Attack"),
)


def get_fonts():
    try:
        f_title = ImageFont.truetype("C:/Windows/Fonts/georgia.ttf", 20)
        f_sub = ImageFont.truetype("C:/Windows/Fonts/consola.ttf", 13)
        f_tag = ImageFont.truetype("C:/Windows/Fonts/consola.ttf", 11)
        return f_title, f_sub, f_tag
    except Exception:
        d = ImageFont.load_default()
        return d, d, d


def main():
    thumb_w = 128
    thumb_h = round(thumb_w * 160 / 128)
    f_title, f_sub, f_tag = get_fonts()

    header_h = 60
    pad = 12
    card_w = 4 * thumb_w + 3 * pad + 24
    w = 2 * card_w + pad
    h = header_h + thumb_h + 36

    canvas = Image.new("RGBA", (w, h), (14, 16, 20, 255))
    draw = ImageDraw.Draw(canvas)

    draw.text((pad, 12), "Lich (Underkeep Dungeon Boss) — Visual Redesign Comparison", fill=(232, 222, 199), font=f_title)
    draw.text((pad, 36), "Before (v2 production) vs After (v3 overhaul)", fill=(150, 146, 138), font=f_sub)

    # Before Box (v2)
    bx0, by0 = pad, header_h
    draw.rectangle((bx0, by0, bx0 + card_w, by0 + thumb_h + 28), outline=(60, 50, 50, 255), fill=(20, 20, 24, 255))
    draw.text((bx0 + 8, by0 + 6), "BEFORE (v2)", fill=(220, 100, 100), font=f_sub)

    for i, (parts, label) in enumerate(LANES):
        p = V2_LICH / Path(*parts)
        fx = bx0 + 8 + i * (thumb_w + pad // 2)
        fy = by0 + 24
        if p.is_file():
            img = Image.open(p).convert("RGBA").resize((thumb_w, thumb_h), Image.Resampling.LANCZOS)
            canvas.alpha_composite(img, (fx, fy))
        else:
            draw.rectangle((fx, fy, fx + thumb_w, fy + thumb_h), outline=(50, 50, 50))
        draw.text((fx + 4, fy + thumb_h - 14), label, fill=(130, 126, 120), font=f_tag)

    # After Box (v3)
    ax0 = bx0 + card_w + pad
    draw.rectangle((ax0, by0, ax0 + card_w, by0 + thumb_h + 28), outline=(40, 70, 50, 255), fill=(18, 26, 22, 255))
    draw.text((ax0 + 8, by0 + 6), "AFTER (v3 OVERHAUL)", fill=(100, 220, 140), font=f_sub)

    for i, (parts, label) in enumerate(LANES):
        p = V3_LICH / Path(*parts)
        fx = ax0 + 8 + i * (thumb_w + pad // 2)
        fy = by0 + 24
        if p.is_file():
            img = Image.open(p).convert("RGBA").resize((thumb_w, thumb_h), Image.Resampling.LANCZOS)
            canvas.alpha_composite(img, (fx, fy))
        else:
            draw.rectangle((fx, fy, fx + thumb_w, fy + thumb_h), outline=(50, 50, 50))
        draw.text((fx + 4, fy + thumb_h - 14), label, fill=(130, 126, 120), font=f_tag)

    OUT_FILE.parent.mkdir(parents=True, exist_ok=True)
    canvas.convert("RGB").save(OUT_FILE)
    print("SAVED", OUT_FILE)


if __name__ == "__main__":
    main()
