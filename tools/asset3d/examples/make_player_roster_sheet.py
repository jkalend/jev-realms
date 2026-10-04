#!/usr/bin/env python3
"""Show all twelve shipped player builds with their bare and equipped cells.

    python tools/asset3d/examples/make_player_roster_sheet.py \
      --atlas assets/atlas --output docs/gfx/proto/ui-v1/player-roster.png
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

from make_gear_tier_sheet import CELL, atlas_cell

ZOOM = 2
LABELS = ("BARE", "LEATHER", "STEEL", "MASTERWORK")


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--atlas", type=Path, default=Path("assets/atlas"))
    parser.add_argument("--output", type=Path, default=Path("docs/gfx/proto/ui-v1/player-roster.png"))
    args = parser.parse_args()

    profiles = json.loads(Path(__file__).with_name("player_profiles.json").read_text(encoding="utf-8"))
    manifest = json.loads((args.atlas / "manifest.json").read_text(encoding="utf-8"))
    sheet = next(entry for entry in manifest["sheets"] if entry["name"] == "players")
    atlas = Image.open(args.atlas / sheet["file"]).convert("RGBA")
    mapping = sheet["mapping"]
    panel_w, row_h, header = 666, 266, 104
    canvas = Image.new("RGB", (panel_w * 2 + 32, row_h * len(profiles) + header + 20), (14, 16, 20))
    draw = ImageDraw.Draw(canvas)
    title = ImageFont.truetype("C:/Windows/Fonts/georgia.ttf", 29)
    heading = ImageFont.truetype("C:/Windows/Fonts/georgia.ttf", 17)
    text = ImageFont.truetype("C:/Windows/Fonts/consola.ttf", 12)
    draw.text((16, 16), "PLAYER ROSTER / SHIPPED ATLAS", font=title, fill=(234, 225, 202))
    draw.text((16, 61), "Six classes, both builds. Every cell below is from assets/atlas/players.png (64px source).",
              font=text, fill=(145, 153, 155))
    for row, (cls, cfg) in enumerate(profiles.items()):
        for column, build in enumerate(("Male", "Female")):
            x, y = 16 + column * panel_w, header + row * row_h
            key = f"Player.{cls}.{build}"
            draw.text((x, y), f"{cls} / {build}", font=heading, fill=(218, 189, 122))
            draw.text((x + 225, y + 4), f"{cfg['calling']} / {cfg['weapon']}",
                      font=text, fill=(151, 161, 162))
            keys = (key, f"{key}.W0A0", f"{key}.W2A2", f"{key}.W3A3")
            for index, (name, plate_key) in enumerate(zip(LABELS, keys)):
                px = x + 4 + index * 160
                py = y + 34
                draw.rectangle((px, py, px + CELL * ZOOM - 1, py + CELL * ZOOM - 1),
                               fill=(29, 34, 41))
                art = atlas_cell(atlas, mapping, plate_key)
                large = art.resize((CELL * ZOOM, CELL * ZOOM), Image.Resampling.NEAREST)
                canvas.paste(large, (px, py), large)
                draw.text((px, py + CELL * ZOOM + 7), name, font=text, fill=(176, 182, 175))
                canvas.paste(art, (px + 85, py + CELL * ZOOM + 24), art)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    canvas.save(args.output)
    print("player-roster", args.output, f"{canvas.width}x{canvas.height}", "runtime builds", len(profiles) * 2)


if __name__ == "__main__":
    main()
