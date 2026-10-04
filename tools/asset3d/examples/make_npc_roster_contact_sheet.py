#!/usr/bin/env python3
"""Build one labelled contact sheet for the generated NPC roster.

Every cell shows four yaw angles of the same idle clip (front, front-right,
right, back) plus the attack and hit stances from the front, so rotation and
animation coverage are inspectable per archetype instead of one hero frame.
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

# (relative render path, label) per cell lane, in draw order.
LANES = (
    (("idle", "front", "000.png"), "front"),
    (("idle", "front_right", "000.png"), "3/4"),
    (("idle", "right", "000.png"), "side"),
    (("idle", "back", "000.png"), "back"),
    (("walk", "front", "000.png"), "walk"),
    (("attack", "front", "001.png"), "strike"),
)
THUMB = 96


def fonts() -> tuple[ImageFont.ImageFont, ImageFont.ImageFont, ImageFont.ImageFont]:
    try:
        return (
            ImageFont.truetype("C:/Windows/Fonts/georgia.ttf", 22),
            ImageFont.truetype("C:/Windows/Fonts/consola.ttf", 12),
            ImageFont.truetype("C:/Windows/Fonts/consola.ttf", 10),
        )
    except OSError:
        default = ImageFont.load_default()
        return default, default, default


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("root", type=Path)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--names", nargs="+", help="Only these archetypes, in display order")
    parser.add_argument("--columns", type=int, default=4)
    parser.add_argument("--thumb-width", type=int, default=THUMB)
    parser.add_argument("--title", default="NON-PLAYER NPC ROSTER / BLENDER 5.1")
    args = parser.parse_args()
    root = args.root.resolve()
    profiles = json.loads((Path(__file__).with_name("npc_roster_profiles.json")).read_text(encoding="utf-8"))
    names = args.names or list(profiles)
    unknown = set(names) - profiles.keys()
    if unknown: parser.error(f"unknown archetypes: {', '.join(sorted(unknown))}")
    columns = args.columns
    if columns < 1 or args.thumb_width < 1: parser.error("columns and thumb-width must be positive")
    thumb, thumb_h = args.thumb_width, round(args.thumb_width * 160 / 128)
    cell_w, cell_h = 6 * (thumb + 6) + 12, thumb_h + 46
    rows = (len(names) + columns - 1) // columns
    head = 42
    width, height = columns * cell_w, head + rows * cell_h
    canvas = Image.new("RGBA", (width, height), (12, 14, 19, 255))
    draw = ImageDraw.Draw(canvas)
    title, label, lane_font = fonts()
    draw.text((16, 12), args.title, fill=(232, 222, 199), font=title)

    missing: list[str] = []
    for index, name in enumerate(names):
        col, row = index % columns, index // columns
        x0, y0 = col * cell_w, head + row * cell_h
        draw.rectangle((x0 + 2, y0 + 2, x0 + cell_w - 4, y0 + cell_h - 4), outline=(48, 46, 42, 255))
        draw.text((x0 + 10, y0 + 8), name, fill=(205, 166, 79), font=label)
        profile = profiles[name]
        meta = f"{profile.get('form', '?')} / {profile.get('weapon', '-')}"
        draw.text((x0 + 10, y0 + 24), meta, fill=(126, 122, 114), font=lane_font)
        for lane, (parts, lane_name) in enumerate(LANES):
            x = x0 + 8 + lane * (thumb + 6)
            y = y0 + 40
            path = root / name / "render" / name / Path(*parts)
            if not path.is_file():
                # A single-beat attack renders one frame; fall back to the last
                # frame that lane actually has rather than drawing an empty cell.
                siblings = sorted(path.parent.glob("*.png"))
                path = siblings[-1] if siblings else path
            if path.is_file():
                image = Image.open(path).convert("RGBA").resize((thumb, thumb_h), Image.Resampling.LANCZOS)
                canvas.alpha_composite(image, (x, y))
            else:
                draw.rectangle((x, y, x + thumb, y + thumb_h), outline=(96, 62, 62, 255))
                missing.append(f"{name}/{lane_name}")
            draw.text((x + 2, y + thumb_h + 2), lane_name, fill=(150, 146, 138), font=lane_font)

    args.output.parent.mkdir(parents=True, exist_ok=True)
    canvas.convert("RGB").save(args.output)
    print("roster-sheet", args.output)
    if missing:
        print("MISSING LANES", len(missing), " ".join(missing))


if __name__ == "__main__":
    main()
