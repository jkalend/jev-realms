#!/usr/bin/env python3
"""Render all 6 player classes in 3D (both Male and Female builds) and generate

Before/After comparisons and a Master Roster Showcase sheet.
"""

from __future__ import annotations

import argparse
import concurrent.futures
import json
import subprocess
import sys
from pathlib import Path
from PIL import Image, ImageDraw, ImageFont

BLENDER = r"F:/Blender/blender.exe"
ROOT = Path(__file__).resolve().parents[3]
EXAMPLES = ROOT / "tools/asset3d/examples"
GENERATOR = EXAMPLES / "create_blender_player_v1.py"
PROFILES_FILE = EXAMPLES / "player_profiles.json"
COMP_DIR = ROOT / "docs/gfx/proto/visual-v3/comparisons"
PLAYER_DIR = ROOT / "docs/gfx/proto/ui-v1/player"

CLASSES = ["Keepwarden", "Gravebound", "Redwake", "Waysworn", "SigilSworn", "Fensworn"]
BUILDS = ["Male", "Female"]

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
        f_head = ImageFont.truetype("C:/Windows/Fonts/georgia.ttf", 16)
        f_sub = ImageFont.truetype("C:/Windows/Fonts/consola.ttf", 13)
        f_tag = ImageFont.truetype("C:/Windows/Fonts/consola.ttf", 11)
        return f_title, f_head, f_sub, f_tag
    except Exception:
        d = ImageFont.load_default()
        return d, d, d, d


def render_character(cls: str, build: str, tier: int = 1, force: bool = False) -> tuple[str, str, bool]:
    out_dir = PLAYER_DIR / f"{cls}.{build}"
    marker = out_dir / "render" / f"{cls}{build}" / "idle" / "front" / "000.png"
    # check if full angles were rendered (check right angle)
    full_marker = out_dir / "render" / f"{cls}{build}" / "idle" / "right" / "000.png"
    if marker.is_file() and full_marker.is_file() and not force:
        return cls, build, True

    cmd = [
        BLENDER, "--background", "--factory-startup",
        "--python", str(GENERATOR), "--",
        "--class", cls,
        "--build", build,
        "--output-root", str(out_dir),
        f"--gear={tier}",
    ]
    res = subprocess.run(cmd, stdout=subprocess.DEVNULL, stderr=subprocess.PIPE, text=True)
    if res.returncode != 0:
        print(f"Error rendering {cls}.{build}: {res.stderr}", file=sys.stderr)
        return cls, build, False
    return cls, build, True


def make_comparison_cards(cls: str):
    f_title, _, f_sub, f_tag = get_fonts()
    v1_dir = PLAYER_DIR / f"{cls}Male" / "render" / f"{cls}Male"
    v3_dir = PLAYER_DIR / f"{cls}.Male" / "render" / f"{cls}Male"

    thumb_w = 128
    thumb_h = round(thumb_w * 160 / 128)

    # 1. Action comparison
    for lanes, filename, subtitle_tag, title_tag in [
        (LANES, f"{cls}_before_after.png", "Idle Front, Idle 3/4, Walk, Attack", "Combat Actions"),
        (ANGLE_LANES, f"{cls}_angles_before_after.png", "Front, Side (Right), 3/4 Back, Back View", "360° Multi-Angle Silhouette & Dorsal View"),
    ]:
        header_h = 60
        pad = 12
        card_w = len(lanes) * thumb_w + (len(lanes) - 1) * pad + 24
        w = 2 * card_w + pad
        h = header_h + thumb_h + 36

        canvas = Image.new("RGBA", (w, h), (14, 16, 20, 255))
        draw = ImageDraw.Draw(canvas)

        draw.text((pad, 12), f"Player Character ({cls}) — {title_tag}", fill=(232, 222, 199), font=f_title)
        draw.text((pad, 36), f"Before (v1 Cylinder Model) vs After (v3 Humanoid Overhaul) — {subtitle_tag}", fill=(150, 146, 138), font=f_sub)

        # Before Box
        bx0, by0 = pad, header_h
        draw.rectangle((bx0, by0, bx0 + card_w, by0 + thumb_h + 28), outline=(60, 50, 50, 255), fill=(20, 20, 24, 255))
        draw.text((bx0 + 8, by0 + 6), f"BEFORE (v1) — {cls} Male", fill=(220, 100, 100), font=f_sub)

        for i, (parts, label) in enumerate(lanes):
            frame_path = v1_dir / Path(*parts)
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
        draw.text((ax0 + 8, by0 + 6), f"AFTER (v3) — {cls} Male", fill=(100, 220, 140), font=f_sub)

        for i, (parts, label) in enumerate(lanes):
            frame_path = v3_dir / Path(*parts)
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


def make_master_showcase(profiles: dict):
    """Grand Showcase sheet showing all 6 classes, both Male and Female, with 4 views each."""
    f_title, f_head, f_sub, f_tag = get_fonts()
    thumb_w = 112
    thumb_h = round(thumb_w * 160 / 128)
    pad = 12

    # 4 views per build: Front, 3/4, Side, Back
    views = (
        (("idle", "front", "000.png"), "Front"),
        (("idle", "front_right", "000.png"), "3/4 Iso"),
        (("idle", "right", "000.png"), "Side"),
        (("idle", "back", "000.png"), "Back"),
    )

    build_w = 4 * thumb_w + 3 * (pad // 2) + 16
    row_h = thumb_h + 52
    header_h = 90
    w = 280 + 2 * build_w + pad * 2
    h = header_h + len(CLASSES) * row_h + 20

    canvas = Image.new("RGBA", (w, h), (14, 16, 20, 255))
    draw = ImageDraw.Draw(canvas)

    draw.text((pad, 14), "PLAYABLE ORDERS / 3D HUMANOID LINEUP", fill=(234, 225, 202), font=f_title)
    draw.text((pad, 42), "All six playable orders rendered in Blender 5.1 with athletic humanoid anatomy, seamless joints & class kit.",
              fill=(150, 155, 156), font=f_sub)
    draw.text((pad, 62), "Showing Male and Female builds across Front, 3/4 Isometric, Profile, and Back silhouettes.",
              fill=(115, 122, 125), font=f_tag)

    for r_idx, cls in enumerate(CLASSES):
        cfg = profiles[cls]
        y0 = header_h + r_idx * row_h

        # Row background
        draw.rectangle((pad, y0, w - pad, y0 + row_h - 6), fill=(20, 24, 28, 255), outline=(42, 48, 56, 255))

        # Class info card (left)
        draw.text((pad + 16, y0 + 16), cls.upper(), fill=(218, 189, 122), font=f_head)
        draw.text((pad + 16, y0 + 40), f"Calling: {cfg['calling']}", fill=(180, 185, 180), font=f_sub)
        draw.text((pad + 16, y0 + 60), f"Weapon: {cfg['weapon']}", fill=(150, 155, 156), font=f_tag)
        draw.text((pad + 16, y0 + 78), f"Headwear: {cfg['head']}", fill=(150, 155, 156), font=f_tag)
        draw.text((pad + 16, y0 + 96), f"Garment: {cfg['garment']}", fill=(150, 155, 156), font=f_tag)
        draw.text((pad + 16, y0 + 114), f"Lore: {cfg['build_note']}", fill=(110, 116, 120), font=f_tag)

        # Male Build
        mx0 = pad + 270
        draw.rectangle((mx0, y0 + 6, mx0 + build_w, y0 + row_h - 12), fill=(16, 20, 24, 255), outline=(32, 40, 48, 255))
        draw.text((mx0 + 8, y0 + 10), "MALE BUILD", fill=(130, 175, 215), font=f_tag)
        m_dir = PLAYER_DIR / f"{cls}.Male" / "render" / f"{cls}Male"
        for v_idx, (parts, label) in enumerate(views):
            fp = m_dir / Path(*parts)
            fx = mx0 + 8 + v_idx * (thumb_w + pad // 2)
            fy = y0 + 26
            if fp.is_file():
                img = Image.open(fp).convert("RGBA").resize((thumb_w, thumb_h), Image.Resampling.LANCZOS)
                canvas.alpha_composite(img, (fx, fy))
            else:
                draw.rectangle((fx, fy, fx + thumb_w, fy + thumb_h), outline=(40, 40, 40))
            draw.text((fx + 4, fy + thumb_h - 14), label, fill=(110, 115, 118), font=f_tag)

        # Female Build
        fx0 = mx0 + build_w + 12
        draw.rectangle((fx0, y0 + 6, fx0 + build_w, y0 + row_h - 12), fill=(16, 20, 24, 255), outline=(32, 40, 48, 255))
        draw.text((fx0 + 8, y0 + 10), "FEMALE BUILD", fill=(215, 150, 175), font=f_tag)
        f_dir = PLAYER_DIR / f"{cls}.Female" / "render" / f"{cls}Female"
        for v_idx, (parts, label) in enumerate(views):
            fp = f_dir / Path(*parts)
            fx = fx0 + 8 + v_idx * (thumb_w + pad // 2)
            fy = y0 + 26
            if fp.is_file():
                img = Image.open(fp).convert("RGBA").resize((thumb_w, thumb_h), Image.Resampling.LANCZOS)
                canvas.alpha_composite(img, (fx, fy))
            else:
                draw.rectangle((fx, fy, fx + thumb_w, fy + thumb_h), outline=(40, 40, 40))
            draw.text((fx + 4, fy + thumb_h - 14), label, fill=(110, 115, 118), font=f_tag)

    out_showcase = ROOT / "docs/gfx/proto/visual-v3/all_players_showcase.png"
    canvas.convert("RGB").save(out_showcase)
    print(f"Master Showcase saved: {out_showcase}")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--force", action="store_true")
    parser.add_argument("--tier", type=int, default=1)
    parser.add_argument("--workers", type=int, default=3)
    args = parser.parse_args()

    profiles = json.loads(PROFILES_FILE.read_text(encoding="utf-8"))

    jobs = [(cls, b, args.tier, args.force) for cls in CLASSES for b in BUILDS]
    print(f"Rendering {len(jobs)} characters using {args.workers} workers...")

    with concurrent.futures.ThreadPoolExecutor(max_workers=args.workers) as pool:
        futures = [pool.submit(render_character, c, b, t, f) for c, b, t, f in jobs]
        for f in concurrent.futures.as_completed(futures):
            c, b, ok = f.result()
            print(f"Done {c}.{b} (success={ok})", flush=True)

    print("\nGenerating comparison cards for each class...")
    for cls in CLASSES:
        make_comparison_cards(cls)

    print("\nGenerating Grand Showcase sheet...")
    make_master_showcase(profiles)

    print("\nALL PLAYER RENDERS & SHOWCASES COMPLETE.")


if __name__ == "__main__":
    main()
