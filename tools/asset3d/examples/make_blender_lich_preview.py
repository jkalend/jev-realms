#!/usr/bin/env python3
"""Create a human-review contact sheet from a baked Blender asset tree."""
from __future__ import annotations

import argparse
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

DIRECTIONS = ["front", "front_right", "right", "back_right", "back", "back_left", "left", "front_left"]
ANIMATIONS = {"idle": 1, "walk": 2, "attack": 3, "hit": 1}


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("root", type=Path)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--id", default="Lich", help="asset id under render/<id>")
    parser.add_argument("--title", default=None)
    args = parser.parse_args()
    root = args.root.resolve()
    render = root / "render" / args.id
    canvas = Image.new("RGBA", (1500, 760), (12, 14, 19, 255))
    draw = ImageDraw.Draw(canvas)
    font_path = Path(__file__).resolve().parents[3] / "assets/fonts/Cinzel-Regular.ttf"
    try:
        title = ImageFont.truetype(str(font_path), 28)
        label = ImageFont.truetype(str(font_path), 12)
        mono = ImageFont.truetype("C:/Windows/Fonts/consola.ttf", 12)
    except OSError:
        title = label = mono = ImageFont.load_default()
    draw.text((40, 24), args.title or f"{args.id.upper()} / BLENDER 5.1 PIPELINE", fill=(232, 222, 199), font=title)
    draw.text((42, 64), "real Blender scene → GLB → 8 directions × animation frames → validated atlas bake", fill=(157, 150, 132), font=mono)
    x0, y0 = 38, 110
    for col, direction in enumerate(DIRECTIONS):
        draw.text((x0 + col * 178, y0), direction, fill=(205, 166, 79), font=label)
    for row, (animation, count) in enumerate(ANIMATIONS.items()):
        y = y0 + 32 + row * 142
        draw.text((4, y + 48), animation, fill=(148, 145, 133), font=mono)
        for col, direction in enumerate(DIRECTIONS):
            image = Image.open(render / animation / direction / "000.png").convert("RGBA")
            image = image.resize((70, 88), Image.Resampling.LANCZOS)
            canvas.alpha_composite(image, (x0 + col * 178, y))
    args.output.parent.mkdir(parents=True, exist_ok=True)
    canvas.convert("RGB").save(args.output, quality=95)
    print("preview", args.output)


if __name__ == "__main__":
    main()
