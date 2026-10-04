#!/usr/bin/env python3
"""Render the 17 humanoid NPCs in v3 and generate Before/After comparison sheets."""

from __future__ import annotations

import argparse
import subprocess
import sys
from pathlib import Path
from PIL import Image, ImageDraw, ImageFont

BLENDER = Path("F:/Blender/blender.exe")
SCRIPT = Path("tools/asset3d/examples/create_blender_npc_roster_v1.py")
V2_ROOT = Path("docs/gfx/proto/visual-v2/npc-roster-v2")
V3_ROOT = Path("docs/gfx/proto/visual-v3/npc-roster-v3")
COMP_DIR = Path("docs/gfx/proto/visual-v3/comparisons")

HUMANOIDS = [
    "Commoner", "Vendor", "Thief", "Traveller", "Bandit", "Smuggler",
    "Skeleton", "Chief", "Matriarch", "Adjudicator", "Oracle", "Companion",
    "Tidemother", "Cragmother", "Tollmaster", "Alchemist", "OathlessCurate"
]

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


def render_npc_v3(npc: str, force: bool = False) -> bool:
    output_dir = V3_ROOT / npc
    test_frame = output_dir / "render" / npc / "idle" / "front" / "000.png"
    if test_frame.is_file() and not force:
        print(f"[{npc}] already rendered in v3, skipping Blender execution.")
        return True

    print(f"[{npc}] Rendering in v3...")
    cmd = [
        str(BLENDER),
        "--background",
        "--python", str(SCRIPT),
        "--",
        "--npc", npc,
        "--output-root", str(output_dir),
        "--visual-v3",
    ]
    res = subprocess.run(cmd, capture_output=True, text=True)
    if res.returncode != 0:
        print(f"[{npc}] Error during render:\n{res.stderr}\n{res.stdout}")
        return False
    print(f"[{npc}] Successfully rendered.")
    return True


def get_frame(root: Path, npc: str, lane_parts: tuple[str, ...]) -> Image.Image | None:
    p = root / npc / "render" / npc / Path(*lane_parts)
    if not p.is_file():
        siblings = sorted(p.parent.glob("*.png")) if p.parent.is_dir() else []
        p = siblings[-1] if siblings else p
    if p.is_file():
        return Image.open(p).convert("RGBA")
    return None


def make_single_comparison(npc: str, thumb_w: int = 128) -> Path | None:
    thumb_h = round(thumb_w * 160 / 128)
    f_title, f_sub, f_tag = get_fonts()

    # 4 lanes for BEFORE, 4 lanes for AFTER
    header_h = 60
    pad = 12
    card_w = 4 * thumb_w + 3 * pad + 24
    w = 2 * card_w + pad
    h = header_h + thumb_h + 36

    canvas = Image.new("RGBA", (w, h), (14, 16, 20, 255))
    draw = ImageDraw.Draw(canvas)

    draw.text((pad, 12), f"{npc} — Visual Redesign Comparison", fill=(232, 222, 199), font=f_title)
    draw.text((pad, 36), "Before (v2 production) vs After (v3 overhaul)", fill=(150, 146, 138), font=f_sub)

    # Before Box
    bx0, by0 = pad, header_h
    draw.rectangle((bx0, by0, bx0 + card_w, by0 + thumb_h + 28), outline=(60, 50, 50, 255), fill=(20, 20, 24, 255))
    draw.text((bx0 + 8, by0 + 6), "BEFORE (v2)", fill=(220, 100, 100), font=f_sub)

    for i, (parts, label) in enumerate(LANES):
        frame = get_frame(V2_ROOT, npc, parts)
        fx = bx0 + 8 + i * (thumb_w + pad // 2)
        fy = by0 + 24
        if frame:
            img = frame.resize((thumb_w, thumb_h), Image.Resampling.LANCZOS)
            canvas.alpha_composite(img, (fx, fy))
        else:
            draw.rectangle((fx, fy, fx + thumb_w, fy + thumb_h), outline=(50, 50, 50))
        draw.text((fx + 4, fy + thumb_h - 14), label, fill=(130, 126, 120), font=f_tag)

    # After Box
    ax0 = bx0 + card_w + pad
    draw.rectangle((ax0, by0, ax0 + card_w, by0 + thumb_h + 28), outline=(40, 70, 50, 255), fill=(18, 26, 22, 255))
    draw.text((ax0 + 8, by0 + 6), "AFTER (v3 OVERHAUL)", fill=(100, 220, 140), font=f_sub)

    for i, (parts, label) in enumerate(LANES):
        frame = get_frame(V3_ROOT, npc, parts)
        fx = ax0 + 8 + i * (thumb_w + pad // 2)
        fy = by0 + 24
        if frame:
            img = frame.resize((thumb_w, thumb_h), Image.Resampling.LANCZOS)
            canvas.alpha_composite(img, (fx, fy))
        else:
            draw.rectangle((fx, fy, fx + thumb_w, fy + thumb_h), outline=(50, 50, 50))
        draw.text((fx + 4, fy + thumb_h - 14), label, fill=(130, 126, 120), font=f_tag)

    COMP_DIR.mkdir(parents=True, exist_ok=True)
    out_path = COMP_DIR / f"{npc}_before_after.png"
    canvas.convert("RGB").save(out_path)
    return out_path


ANGLE_LANES = (
    (("idle", "front", "000.png"), "Front"),
    (("idle", "right", "000.png"), "Side (Right)"),
    (("idle", "back_right", "000.png"), "3/4 Back"),
    (("idle", "back", "000.png"), "Back View"),
)


def make_single_angles_comparison(npc: str, thumb_w: int = 128) -> Path | None:
    thumb_h = round(thumb_w * 160 / 128)
    f_title, f_sub, f_tag = get_fonts()

    header_h = 60
    pad = 12
    card_w = 4 * thumb_w + 3 * pad + 24
    w = 2 * card_w + pad
    h = header_h + thumb_h + 36

    canvas = Image.new("RGBA", (w, h), (14, 16, 20, 255))
    draw = ImageDraw.Draw(canvas)

    draw.text((pad, 12), f"{npc} — 360° Multi-Angle Silhouette & Dorsal View", fill=(232, 222, 199), font=f_title)
    draw.text((pad, 36), "Before (v2 production) vs After (v3 overhaul) — Front, Side & Back Profile", fill=(150, 146, 138), font=f_sub)

    # Before Box
    bx0, by0 = pad, header_h
    draw.rectangle((bx0, by0, bx0 + card_w, by0 + thumb_h + 28), outline=(60, 50, 50, 255), fill=(20, 20, 24, 255))
    draw.text((bx0 + 8, by0 + 6), "BEFORE (v2) — Angles", fill=(220, 100, 100), font=f_sub)

    for i, (parts, label) in enumerate(ANGLE_LANES):
        frame = get_frame(V2_ROOT, npc, parts)
        fx = bx0 + 8 + i * (thumb_w + pad // 2)
        fy = by0 + 24
        if frame:
            img = frame.resize((thumb_w, thumb_h), Image.Resampling.LANCZOS)
            canvas.alpha_composite(img, (fx, fy))
        else:
            draw.rectangle((fx, fy, fx + thumb_w, fy + thumb_h), outline=(50, 50, 50))
        draw.text((fx + 4, fy + thumb_h - 14), label, fill=(130, 126, 120), font=f_tag)

    # After Box
    ax0 = bx0 + card_w + pad
    draw.rectangle((ax0, by0, ax0 + card_w, by0 + thumb_h + 28), outline=(40, 70, 50, 255), fill=(18, 26, 22, 255))
    draw.text((ax0 + 8, by0 + 6), "AFTER (v3 OVERHAUL) — Dorsal & Side Details", fill=(100, 220, 140), font=f_sub)

    for i, (parts, label) in enumerate(ANGLE_LANES):
        frame = get_frame(V3_ROOT, npc, parts)
        fx = ax0 + 8 + i * (thumb_w + pad // 2)
        fy = by0 + 24
        if frame:
            img = frame.resize((thumb_w, thumb_h), Image.Resampling.LANCZOS)
            canvas.alpha_composite(img, (fx, fy))
        else:
            draw.rectangle((fx, fy, fx + thumb_w, fy + thumb_h), outline=(50, 50, 50))
        draw.text((fx + 4, fy + thumb_h - 14), label, fill=(130, 126, 120), font=f_tag)

    COMP_DIR.mkdir(parents=True, exist_ok=True)
    out_path = COMP_DIR / f"{npc}_angles_before_after.png"
    canvas.convert("RGB").save(out_path)
    return out_path


def make_master_comparison_sheet(thumb_w: int = 96) -> Path:
    thumb_h = round(thumb_w * 160 / 128)
    f_title, f_sub, f_tag = get_fonts()

    row_h = thumb_h + 28
    pad = 8
    cols = 6  # 3 before, 3 after
    meta_w = 140
    lane_w = cols * (thumb_w + pad)
    width = meta_w + lane_w + 30
    head_h = 50
    height = head_h + len(HUMANOIDS) * row_h + 20

    canvas = Image.new("RGBA", (width, height), (12, 14, 18, 255))
    draw = ImageDraw.Draw(canvas)

    draw.text((16, 12), "ALL 17 HUMANOID NPCS: BEFORE (v2) vs AFTER (v3) — ACTIONS", fill=(232, 222, 199), font=f_title)
    draw.text((16, 34), "Direct side-by-side comparison across Idle, Walk, and Attack frames", fill=(140, 136, 128), font=f_tag)

    lanes_3 = (
        (("idle", "front", "000.png"), "Idle"),
        (("walk", "front", "000.png"), "Walk"),
        (("attack", "front", "000.png"), "Attack"),
    )

    for r, npc in enumerate(HUMANOIDS):
        y = head_h + r * row_h
        draw.rectangle((10, y + 2, width - 10, y + row_h - 2), outline=(32, 34, 40), fill=(16, 18, 22, 255))
        draw.text((20, y + row_h // 2 - 12), npc, fill=(210, 175, 95), font=f_sub)

        # 3 Before frames
        bx0 = meta_w
        draw.text((bx0, y + 4), "BEFORE (v2)", fill=(200, 90, 90), font=f_tag)
        for i, (parts, label) in enumerate(lanes_3):
            fx = bx0 + i * (thumb_w + pad)
            fy = y + 18
            frame = get_frame(V2_ROOT, npc, parts)
            if frame:
                img = frame.resize((thumb_w, thumb_h), Image.Resampling.LANCZOS)
                canvas.alpha_composite(img, (fx, fy))
            else:
                draw.rectangle((fx, fy, fx + thumb_w, fy + thumb_h), outline=(40, 40, 40))

        # 3 After frames
        ax0 = meta_w + 3 * (thumb_w + pad) + 16
        draw.text((ax0, y + 4), "AFTER (v3 OVERHAUL)", fill=(90, 210, 130), font=f_tag)
        for i, (parts, label) in enumerate(lanes_3):
            fx = ax0 + i * (thumb_w + pad)
            fy = y + 18
            frame = get_frame(V3_ROOT, npc, parts)
            if frame:
                img = frame.resize((thumb_w, thumb_h), Image.Resampling.LANCZOS)
                canvas.alpha_composite(img, (fx, fy))
            else:
                draw.rectangle((fx, fy, fx + thumb_w, fy + thumb_h), outline=(40, 40, 40))

    out_path = Path("docs/gfx/proto/visual-v3/all_humanoids_before_after.png")
    out_path.parent.mkdir(parents=True, exist_ok=True)
    canvas.convert("RGB").save(out_path)
    print(f"Master comparison saved: {out_path}")
    return out_path


def make_master_angles_sheet(thumb_w: int = 96) -> Path:
    thumb_h = round(thumb_w * 160 / 128)
    f_title, f_sub, f_tag = get_fonts()

    # Per NPC row:
    # [NPC Name] | BEFORE: Front, Side, Back | AFTER: Front, Side, Back
    row_h = thumb_h + 28
    pad = 8
    cols = 6  # 3 before, 3 after
    meta_w = 140
    lane_w = cols * (thumb_w + pad)
    width = meta_w + lane_w + 30
    head_h = 50
    height = head_h + len(HUMANOIDS) * row_h + 20

    canvas = Image.new("RGBA", (width, height), (12, 14, 18, 255))
    draw = ImageDraw.Draw(canvas)

    draw.text((16, 12), "ALL 17 HUMANOID NPCS: FRONT vs SIDE vs BACK (v2 vs v3)", fill=(232, 222, 199), font=f_title)
    draw.text((16, 34), "Multi-angle evaluation showing side silhouettes, headwear drapery, and dorsal props", fill=(140, 136, 128), font=f_tag)

    lanes_3 = (
        (("idle", "front", "000.png"), "Front"),
        (("idle", "right", "000.png"), "Side"),
        (("idle", "back", "000.png"), "Back"),
    )

    for r, npc in enumerate(HUMANOIDS):
        y = head_h + r * row_h
        draw.rectangle((10, y + 2, width - 10, y + row_h - 2), outline=(32, 34, 40), fill=(16, 18, 22, 255))
        draw.text((20, y + row_h // 2 - 12), npc, fill=(210, 175, 95), font=f_sub)

        # 3 Before frames
        bx0 = meta_w
        draw.text((bx0, y + 4), "BEFORE (v2)", fill=(200, 90, 90), font=f_tag)
        for i, (parts, label) in enumerate(lanes_3):
            fx = bx0 + i * (thumb_w + pad)
            fy = y + 18
            frame = get_frame(V2_ROOT, npc, parts)
            if frame:
                img = frame.resize((thumb_w, thumb_h), Image.Resampling.LANCZOS)
                canvas.alpha_composite(img, (fx, fy))
            else:
                draw.rectangle((fx, fy, fx + thumb_w, fy + thumb_h), outline=(40, 40, 40))

        # 3 After frames
        ax0 = meta_w + 3 * (thumb_w + pad) + 16
        draw.text((ax0, y + 4), "AFTER (v3 OVERHAUL)", fill=(90, 210, 130), font=f_tag)
        for i, (parts, label) in enumerate(lanes_3):
            fx = ax0 + i * (thumb_w + pad)
            fy = y + 18
            frame = get_frame(V3_ROOT, npc, parts)
            if frame:
                img = frame.resize((thumb_w, thumb_h), Image.Resampling.LANCZOS)
                canvas.alpha_composite(img, (fx, fy))
            else:
                draw.rectangle((fx, fy, fx + thumb_w, fy + thumb_h), outline=(40, 40, 40))

    out_path = Path("docs/gfx/proto/visual-v3/all_humanoids_angles_before_after.png")
    out_path.parent.mkdir(parents=True, exist_ok=True)
    canvas.convert("RGB").save(out_path)
    print(f"Master angles comparison saved: {out_path}")
    return out_path


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--names", nargs="+", default=HUMANOIDS)
    parser.add_argument("--force", action="store_true")
    parser.add_argument("--skip-render", action="store_true")
    args = parser.parse_args()

    for name in args.names:
        if not args.skip_render:
            render_npc_v3(name, force=args.force)
        p1 = make_single_comparison(name)
        p2 = make_single_angles_comparison(name)
        if p1:
            print(f"Saved comparison for {name}: {p1}")
        if p2:
            print(f"Saved angles comparison for {name}: {p2}")

    master = make_master_comparison_sheet()
    angles_master = make_master_angles_sheet()
    print("ALL DONE.")
    print("Master actions sheet:", master)
    print("Master angles sheet:", angles_master)


if __name__ == "__main__":
    main()
