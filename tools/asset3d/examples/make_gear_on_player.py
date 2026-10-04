#!/usr/bin/env python3
"""Compare actual shipped 64px player cells at five equipment states.

    python tools/asset3d/examples/make_gear_on_player.py \
      --class Keepwarden --build Male \
      --output docs/gfx/proto/ui-v1/mockups/gear-on-player.png
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

from make_gear_tier_sheet import CELL, atlas_cell

SCALE = 5


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--cls", "--class", dest="cls", default="Keepwarden")
    parser.add_argument("--build", default="Male")
    parser.add_argument("--atlas", type=Path, default=Path("assets/atlas"))
    parser.add_argument("--output", type=Path, default=Path("docs/gfx/proto/ui-v1/mockups/gear-on-player.png"))
    args = parser.parse_args()

    manifest = json.loads((args.atlas / "manifest.json").read_text(encoding="utf-8"))
    sheets = {sheet["name"]: sheet for sheet in manifest["sheets"]}
    players, relics = sheets["players"], sheets["player_relics"]
    player_image = Image.open(args.atlas / players["file"]).convert("RGBA")
    relic_image = Image.open(args.atlas / relics["file"]).convert("RGBA")
    prefix = f"Player.{args.cls}.{args.build}"
    configs = [("BARE / starting character", prefix)] + [
        (label, f"{prefix}.W{tier}A{tier}") for tier, label in enumerate((
            "WORN / leather", "STANDARD / iron", "FINE / steel", "MASTERWORK / gold"))
    ]
    boss = atlas_cell(player_image, players["mapping"], f"{prefix}.W2A2")
    boss.alpha_composite(atlas_cell(relic_image, relics["mapping"],
                                    f"Player.Relic.Stoneheart.{args.build}"))
    configs.append(("BOSS / Stoneheart", boss))

    pad, header, tile_w, tile_h = 15, 112, 345, 380
    canvas = Image.new("RGB", (tile_w * 3 + pad * 2, header + tile_h * 2 + pad), (14, 16, 20))
    draw = ImageDraw.Draw(canvas)
    title = ImageFont.truetype("C:/Windows/Fonts/georgia.ttf", 27)
    label_font = ImageFont.truetype("C:/Windows/Fonts/consola.ttf", 16)
    detail = ImageFont.truetype("C:/Windows/Fonts/consola.ttf", 12)
    draw.text((pad, 18), f"EQUIPPED PLAYER / {args.cls.upper()} {args.build.upper()}",
              font=title, fill=(236, 228, 202))
    draw.text((pad, 60), "Every panel is a shipped 64px atlas cell; no hand-painted gear.",
              font=detail, fill=(156, 166, 171))
    for index, (label, key_or_image) in enumerate(configs):
        x, y = pad + index % 3 * tile_w, header + index // 3 * tile_h
        art = (key_or_image if isinstance(key_or_image, Image.Image)
               else atlas_cell(player_image, players["mapping"], key_or_image))
        draw.text((x + 4, y + 2), label, font=label_font, fill=(224, 189, 108))
        draw.rectangle((x, y + 34, x + CELL * SCALE - 1, y + 33 + CELL * SCALE),
                       fill=(29, 33, 39), outline=(68, 71, 77))
        enlarged = art.resize((CELL * SCALE, CELL * SCALE), Image.Resampling.NEAREST)
        canvas.paste(enlarged, (x, y + 34), enlarged)
        canvas.paste(art, (x + 275, y + 34), art)
        draw.text((x + 277, y + 104), "1:1", font=detail, fill=(156, 166, 171))
    args.output.parent.mkdir(parents=True, exist_ok=True)
    canvas.save(args.output)
    print("gear-on-player", args.output, f"{canvas.width}x{canvas.height}", "runtime cells", len(configs))


if __name__ == "__main__":
    main()
