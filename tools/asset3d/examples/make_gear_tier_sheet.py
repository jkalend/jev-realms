#!/usr/bin/env python3
"""Preview *shipped* 64px player atlas cells and boss relic overlays.

    python tools/asset3d/examples/make_gear_tier_sheet.py \
      --class Keepwarden --build Male --output docs/gfx/proto/ui-v1/gear-ladder.png
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

TIERS = ("Worn leather", "Iron", "Bright steel", "Gilded masterwork")
RELICS = (
    "Rallybreaker", "Fangmantle", "Graveglass", "Saltcrown", "Stoneheart",
    "GnawboneCrown", "TollcoinCharm", "WisplightLantern", "Hartshorn",
)
CELL = 64
ZOOM = 5


def atlas_cell(sheet: Image.Image, mapping: dict, key: str) -> Image.Image:
    col, row = mapping[key]
    return sheet.crop((col * CELL, row * CELL, (col + 1) * CELL, (row + 1) * CELL))


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--cls", "--class", dest="cls", default="Keepwarden")
    parser.add_argument("--build", default="Male")
    parser.add_argument("--atlas", type=Path, default=Path("assets/atlas"))
    parser.add_argument("--output", type=Path, default=Path("docs/gfx/proto/ui-v1/gear-ladder.png"))
    args = parser.parse_args()

    sheets = {s["name"]: s for s in json.loads((args.atlas / "manifest.json").read_text(encoding="utf-8"))["sheets"]}
    players = sheets["players"]
    relics = sheets["player_relics"]
    player_image = Image.open(args.atlas / players["file"]).convert("RGBA")
    relic_image = Image.open(args.atlas / relics["file"]).convert("RGBA")
    base_key = f"Player.{args.cls}.{args.build}"
    jobs = [
        (f"TIER {tier} / {TIERS[tier]}",
         atlas_cell(player_image, players["mapping"], f"{base_key}.W{tier}A{tier}"))
        for tier in range(4)
    ]
    boss_base = atlas_cell(player_image, players["mapping"], f"{base_key}.W2A2")
    for relic in RELICS:
        worn = boss_base.copy()
        worn.alpha_composite(atlas_cell(
            relic_image, relics["mapping"], f"Player.Relic.{relic}.{args.build}"))
        jobs.append((relic.upper(), worn))

    columns, tile_w, tile_h, top = 4, 378, 393, 100
    canvas = Image.new("RGB", (columns * tile_w + 16, top + 4 * tile_h + 24), (15, 18, 22))
    draw = ImageDraw.Draw(canvas)
    title = ImageFont.truetype("C:/Windows/Fonts/georgia.ttf", 26)
    label_font = ImageFont.truetype("C:/Windows/Fonts/consola.ttf", 17)
    detail_font = ImageFont.truetype("C:/Windows/Fonts/consola.ttf", 12)
    draw.text((18, 16), f"EQUIPPED ATLAS / {args.cls.upper()} {args.build.upper()}", font=title,
              fill=(236, 228, 202))
    draw.text((18, 57), "Actual shipped 64 x 64 cells. Relics shown over W2A2; no painted mock gear.",
              font=detail_font, fill=(160, 168, 169))
    for index, (label, art) in enumerate(jobs):
        x = 16 + index % columns * tile_w
        y = top + index // columns * tile_h
        draw.text((x + 4, y), label, font=label_font, fill=(225, 190, 108))
        for cy in range(0, CELL * ZOOM, 20):
            for cx in range(0, CELL * ZOOM, 20):
                if (cx // 20 + cy // 20) % 2 == 0:
                    draw.rectangle((x + cx, y + 38 + cy, x + cx + 19, y + 57 + cy),
                                   fill=(28, 33, 39))
        canvas.paste(art.resize((CELL * ZOOM, CELL * ZOOM), Image.Resampling.NEAREST),
                     (x, y + 38), art.resize((CELL * ZOOM, CELL * ZOOM), Image.Resampling.NEAREST))
        canvas.paste(art, (x + 325, y + 38), art)
        draw.text((x + 322, y + 106), "1:1", font=detail_font, fill=(160, 168, 169))
    args.output.parent.mkdir(parents=True, exist_ok=True)
    canvas.save(args.output)
    print("gear-ladder", args.output, f"{canvas.width}x{canvas.height}", "runtime cells", len(jobs))


if __name__ == "__main__":
    main()
