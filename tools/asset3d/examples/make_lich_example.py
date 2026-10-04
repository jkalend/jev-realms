#!/usr/bin/env python3
"""Create the first end-to-end Lich source/bake example.

This is a procedural fixture that proves the production asset seam without
requiring Blender in the development environment. Replace the generated GLB
with a Blender-authored Lich later; the manifest/render layout stays the same.
"""

from __future__ import annotations

import hashlib
import json
import subprocess
import sys
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

ROOT = Path(__file__).resolve().parents[3]
OUT = ROOT / "docs/gfx/proto/visual-v2/lich-3d-example"
SOURCE = OUT / "source/Lich.glb"
RENDER_ROOT = OUT / "render/Lich"
MANIFEST = OUT / "lich_manifest.json"

sys.path.insert(0, str(ROOT / "docs/gfx/proto/visual-v2"))
sys.path.insert(0, str(ROOT / "tools/asset3d"))

import generate_3d_bosses as boss_models  # noqa: E402
import generate_3d_npcs as npc_models  # noqa: E402
from glb_writer import write_glb  # noqa: E402

DIRECTIONS = [
    ("front", 0),
    ("front_right", 35),
    ("right", 90),
    ("back_right", 135),
    ("back", 180),
    ("back_left", -135),
    ("left", -90),
    ("front_left", -35),
]
ANIMATIONS = {"idle": 1, "walk": 2, "attack": 3, "hit": 1}
CELL = (128, 160)


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def place_sprite(sprite: Image.Image, animation: str, frame: int) -> Image.Image:
    canvas = Image.new("RGBA", CELL, (0, 0, 0, 0))
    copy = sprite.copy()
    copy.thumbnail((116, 146), Image.Resampling.LANCZOS)
    dx = 0
    dy = 0
    if animation == "walk":
        dx = -4 if frame == 0 else 4
    elif animation == "attack":
        dx = 3 + frame
        dy = -frame
    elif animation == "hit":
        dx = -3
        dy = 2
    x = (CELL[0] - copy.width) // 2 + dx
    y = CELL[1] - copy.height - 3 + dy
    canvas.alpha_composite(copy, (x, y))
    return canvas


def render_frames(mesh: object) -> None:
    for animation, count in ANIMATIONS.items():
        for direction, angle in DIRECTIONS:
            target_dir = RENDER_ROOT / animation / direction
            target_dir.mkdir(parents=True, exist_ok=True)
            sprite = npc_models.render_mesh(mesh, yaw=angle, pitch=10, scale=46)
            for frame in range(count):
                place_sprite(sprite, animation, frame).save(target_dir / f"{frame:03d}.png")


def write_manifest() -> None:
    manifest = {
        "version": 1,
        "assets": [{
            "id": "Lich",
            "kind": "boss",
            "source": "source/Lich.glb",
            "render_root": "render/Lich",
            "cell_size": {"width": CELL[0], "height": CELL[1]},
            "directions": [name for name, _ in DIRECTIONS],
            "animations": ANIMATIONS,
            "uvs": True,
            "materials": True,
            "rig": True,
            "pivot": "feet_center",
            "metadata": {
                "source_kind": "procedural_fixture",
                "front_axis": "-Z",
                "yaw": "positive rotates toward the character's left",
                "note": "Replace this GLB with a Blender-authored rig before production approval"
            }
        }]
    }
    MANIFEST.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")


def write_preview() -> None:
    scale = 0.55
    cell_w, cell_h = int(CELL[0] * scale), int(CELL[1] * scale)
    canvas = Image.new("RGBA", (1500, 760), (12, 14, 19, 255))
    draw = ImageDraw.Draw(canvas)
    try:
        title_font = ImageFont.truetype(str(ROOT / "assets/fonts/Cinzel-Regular.ttf"), 28)
        label_font = ImageFont.truetype(str(ROOT / "assets/fonts/Cinzel-Regular.ttf"), 12)
        mono_font = ImageFont.truetype("C:/Windows/Fonts/consola.ttf", 12)
    except OSError:
        title_font = label_font = mono_font = ImageFont.load_default()
    draw.text((40, 24), "LICH / TRUE 3D PIPELINE EXAMPLE", fill=(232, 222, 199), font=title_font)
    draw.text((42, 64), "GLB source → 8 directions × idle/walk/attack/hit → validated atlas fragment", fill=(157, 150, 132), font=mono_font)
    x0, y0 = 38, 110
    for col, (direction, _) in enumerate(DIRECTIONS):
        x = x0 + col * 178
        draw.text((x, y0), direction, fill=(205, 166, 79), font=label_font)
    for row, (animation, count) in enumerate(ANIMATIONS.items()):
        y = y0 + 32 + row * 142
        draw.text((4, y + 48), animation, fill=(148, 145, 133), font=mono_font)
        for col, (direction, _) in enumerate(DIRECTIONS):
            frame = min(count - 1, 0)
            image = Image.open(RENDER_ROOT / animation / direction / f"{frame:03d}.png").convert("RGBA")
            image = image.resize((cell_w, cell_h), Image.Resampling.LANCZOS)
            canvas.alpha_composite(image, (x0 + col * 178, y))
    canvas.convert("RGB").save(OUT / "lich-3d-example.png", quality=95)


def run(command: list[str]) -> None:
    subprocess.run(command, check=True, cwd=ROOT)


def main() -> None:
    OUT.mkdir(parents=True, exist_ok=True)
    mesh = boss_models.make_lich()
    write_glb(
        mesh,
        SOURCE,
        "Lich",
        npc_models.MATERIALS,
        extras={
            "asset3d_id": "Lich",
            "asset3d_rig": "Lich_Joint minimal skin",
            "front_axis": "-Z",
            "back_axis": "+Z",
            "pivot": "feet_center",
        },
    )
    sidecar = {
        "version": 1,
        "id": "Lich",
        "source": "Lich.glb",
        "sha256": sha256(SOURCE),
        "source_kind": "procedural_fixture",
        "front_marker": "Lich_front",
        "back_marker": "Lich_back",
        "pivot": "Lich_pivot",
        "rig": "Lich_Joint minimal skin; replace with authored multi-bone armature"
    }
    (OUT / "source/Lich.json").write_text(json.dumps(sidecar, indent=2) + "\n", encoding="utf-8")
    render_frames(mesh)
    write_manifest()
    run([sys.executable, "tools/asset3d/pipeline.py", "validate", str(MANIFEST)])
    run([sys.executable, "tools/asset3d/pipeline.py", "bake", str(MANIFEST), "--output-dir", str(OUT / "baked"), "--sheet-name", "lich-3d-baked"])
    write_preview()
    print("generated Lich pipeline example in", OUT)


if __name__ == "__main__":
    main()
