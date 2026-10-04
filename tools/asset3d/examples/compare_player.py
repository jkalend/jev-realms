#!/usr/bin/env python3
"""Generate Before/After comparison sheets for the Player character overhaul."""

from __future__ import annotations

from pathlib import Path
from PIL import Image, ImageDraw, ImageFont

V1_DIR = Path("docs/gfx/proto/ui-v1/player/KeepwardenMale/render/KeepwardenMale")
V3_DIR = Path("docs/gfx/proto/ui-v1/player/Keepwarden.Male/render/KeepwardenMale")
COMP_DIR = Path("docs/gfx/proto/visual-v3/comparisons")

LANES = (
    (("idle", "front", "000.png"), "Idle Front"),
    (("idle", "front_right", "000.png"), "Idle 3/4"),
    (("walk", "front", "000.png"), "Walk"),
    (("attack", "front", "001.png"), "Attack"),
)

ANGLE_LANES = (
    (("idle", "front", "000.png"), "Front"),
    (("idle", "right", "000.png"), "Side (Right)"),
    (("idle", "back_right", "000.png"), "3/4 Back"),
    (("idle", "back", "000.png"), "Back View"),
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


def make_comparison(title: str, subtitle: str, lanes: tuple, filename: str, thumb_w: int = 128):
    thumb_h = round(thumb_w * 160 / 128)
    f_title, f_sub, f_tag = get_fonts()

    header_h = 60
    pad = 12
    card_w = len(lanes) * thumb_w + (len(lanes) - 1) * pad + 24
    w = 2 * card_w + pad
    h = header_h + thumb_h + 36

    canvas = Image.new("RGBA", (w, h), (14, 16, 20, 255))
    draw = ImageDraw.Draw(canvas)

    draw.text((pad, 12), title, fill=(232, 222, 199), font=f_title)
    draw.text((pad, 36), subtitle, fill=(150, 146, 138), font=f_sub)

    # Before Box
    bx0, by0 = pad, header_h
    draw.rectangle((bx0, by0, bx0 + card_w, by0 + thumb_h + 28), outline=(60, 50, 50, 255), fill=(20, 20, 24, 255))
    draw.text((bx0 + 8, by0 + 6), "BEFORE (v1 Cylinder Model)", fill=(220, 100, 100), font=f_sub)

    for i, (parts, label) in enumerate(lanes):
        frame_path = V1_DIR / Path(*parts)
        fx = bx0 + 8 + i * (thumb_w + pad // 2)
        fy = by0 + 24
        if frame_path.is_file():
            img = Image.open(frame_path).convert("RGBA").resize((thumb_w, thumb_h), Image.Resampling.LANCZOS)
            canvas.alpha_composite(img, (fx, fy))
        else:
            draw.rectangle((fx, fy, fx + thumb_w, fy + thumb_h), outline=(50, 50, 50))
        draw.text((fx + 4, fy + thumb_h - 14), label, fill=(130, 126, 120), font=f_tag)

    # After Box
    ax0 = bx0 + card_w + pad
    draw.rectangle((ax0, by0, ax0 + card_w, by0 + thumb_h + 28), outline=(40, 70, 50, 255), fill=(18, 26, 22, 255))
    draw.text((ax0 + 8, by0 + 6), "AFTER (v3 Humanoid Overhaul)", fill=(100, 220, 140), font=f_sub)

    for i, (parts, label) in enumerate(lanes):
        frame_path = V3_DIR / Path(*parts)
        fx = ax0 + 8 + i * (thumb_w + pad // 2)
        fy = by0 + 24
        if frame_path.is_file():
            img = Image.open(frame_path).convert("RGBA").resize((thumb_w, thumb_h), Image.Resampling.LANCZOS)
            canvas.alpha_composite(img, (fx, fy))
        else:
            draw.rectangle((fx, fy, fx + thumb_w, fy + thumb_h), outline=(50, 50, 50))
        draw.text((fx + 4, fy + thumb_h - 14), label, fill=(130, 126, 120), font=f_tag)

    COMP_DIR.mkdir(parents=True, exist_ok=True)
    out_path = COMP_DIR / filename
    canvas.convert("RGB").save(out_path)
    print(f"Saved {out_path}")


def main():
    make_comparison(
        "Player Character (Keepwarden) — Humanoid Visual Overhaul",
        "Before (v1 rigid cylinder joints) vs After (v3 athletic humanoid anatomy & smooth weighting)",
        LANES,
        "Player_before_after.png",
    )
    make_comparison(
        "Player Character (Keepwarden) — 360° Multi-Angle Silhouette & Dorsal View",
        "Before (v1 rigid joints) vs After (v3 seamless joints, tailored belt/skirt, contoured boots)",
        ANGLE_LANES,
        "Player_angles_before_after.png",
    )


if __name__ == "__main__":
    main()
