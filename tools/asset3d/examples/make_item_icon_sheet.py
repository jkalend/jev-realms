#!/usr/bin/env python3
"""Label every painted item icon on a review sheet, at 1x and 2x."""

from __future__ import annotations

import argparse
import json
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

SCALE = 3
LABEL_W = 240
PAD = 12
HEADER = 64


def fonts() -> tuple[ImageFont.ImageFont, ImageFont.ImageFont, ImageFont.ImageFont]:
    try:
        return (
            ImageFont.truetype("C:/Windows/Fonts/georgia.ttf", 24),
            ImageFont.truetype("C:/Windows/Fonts/consola.ttf", 15),
            ImageFont.truetype("C:/Windows/Fonts/georgia.ttf", 19),
        )
    except OSError:
        default = ImageFont.load_default()
        return default, default, default


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("sheet", type=Path)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()

    image = Image.open(args.sheet).convert("RGBA")
    manifest = json.loads(args.sheet.with_suffix(".json").read_text(encoding="utf-8"))
    cell = manifest["cell_size"]["width"]
    entries = manifest["items"]
    rows = (len(entries) + 2 - 1) // 2

    icon = cell * SCALE
    row_h = max(icon, 34) + 30
    width = 2 * (icon + PAD + LABEL_W) + PAD
    height = HEADER + rows * row_h + PAD

    canvas = Image.new("RGBA", (width, height), (14, 15, 19, 255))
    draw = ImageDraw.Draw(canvas)
    title, body, head = fonts()
    draw.text((PAD + 4, 16), "ITEM ICON SET / 45 PLATES", fill=(232, 222, 199, 255), font=title)
    draw.text((PAD + 4, 44), f"{cell}x{cell} cells, shown at {SCALE}x - every key maps to a core::model::Item variant", fill=(140, 136, 128, 255), font=body)

    for index, entry in enumerate(entries):
        col, row = index // rows, index % rows
        x = PAD + col * (icon + PAD + LABEL_W)
        y = HEADER + row * row_h
        source = image.crop((entry["col"] * cell, entry["row"] * cell, (entry["col"] + 1) * cell, (entry["row"] + 1) * cell))
        big = source.resize((icon, icon), Image.Resampling.NEAREST)
        draw.rectangle((x - 2, y - 2, x + icon + 1, y + icon + 1), outline=(58, 56, 52, 255))
        # Checker so the transparent plate boundary is visible.
        for cy in range(0, icon, 12):
            for cx in range(0, icon, 12):
                if (cx // 12 + cy // 12) % 2 == 0:
                    draw.rectangle((x + cx, y + cy, x + min(cx + 11, icon - 1), y + min(cy + 11, icon - 1)), fill=(24, 26, 31, 255))
        canvas.alpha_composite(big, (x, y))
        draw.text((x + icon + 14, y + icon // 2 - 20), entry["name"], fill=(205, 166, 79), font=head)
        draw.text((x + icon + 14, y + icon // 2 + 4), f"cell {entry['col']},{entry['row']}", fill=(120, 116, 108), font=body)

    args.output.parent.mkdir(parents=True, exist_ok=True)
    canvas.convert("RGB").save(args.output)
    print("item-sheet", args.output, f"{canvas.size[0]}x{canvas.size[1]}")


if __name__ == "__main__":
    main()
