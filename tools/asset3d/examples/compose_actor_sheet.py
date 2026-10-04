#!/usr/bin/env python3
"""Compose the improved Blender NPC renders into the game's `actors` atlas sheet.

Each NPC is authored once in Blender and baked to a *fragment*: a PNG of
128x160 or 256x320 cells plus a JSON sidecar naming every cell
`<Arch>.<action>.<dir>.<frame>` and where it sits. This step flattens those 26
fragments into the single sheet `assets/atlas/actors.png` that the view reads,
so the 3D roster reaches the game with no renderer changes: the atlas
contract is just "key -> rect", and composite keys are still keys.

Two things matter for the result reading as animation rather than as 26
unrelated pictures:

*   **One transform per archetype.** The bbox of the opaque pixels is taken
    over the union of all that archetype's cells, not per cell. Cropping to a
    per-frame bbox would rescale and re-anchor every frame, so a walk cycle
    would breathe and its feet would slide as the stance widened.
*   **Feet on the floor.** Every cell is bottom-anchored at the same baseline,
    so an actor's feet stay put while the body above them moves.

The bare `<Arch>` key is also written, pointing at the idle front frame, so
anything that still asks for a flat plate gets the 3D character rather than
falling back to a colour quad.

Run:
    python tools/asset3d/examples/compose_actor_sheet.py
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path

from PIL import Image

# 96px cells: an NPC draws larger than one tile, so a 64px cell would be
# resampled down and the character's face and fabric would blur away.
CELL = 96
FIT_HEIGHT = 93
COLUMNS = 24
MANIFEST = Path("assets/atlas/manifest.json")
SHEET_NAME = "actors"
SHEET_FILE = "actors.png"
VISUAL_V2 = Path("docs/gfx/proto/visual-v2")
VISUAL_V3 = Path("docs/gfx/proto/visual-v3")


def get_npc_root(name: str) -> Path:
    v3_path = VISUAL_V3 / "npc-roster-v3" / name
    if (v3_path / "baked").is_dir():
        return v3_path
    return VISUAL_V2 / "npc-roster-v2" / name


# The hand-refined Lich and Guard plus the 24 improved standalone NPC models.
# Only the sheet layout depends on order; lookups use archetype keys.
ROSTER = sorted(p.name for p in (VISUAL_V2 / "npc-roster-v2").iterdir() if p.is_dir())
HEROES = [
    ("Guard", VISUAL_V2 / "guard-blender-5.1"),
    ("Lich", VISUAL_V3 / "lich-blender-5.1" if (VISUAL_V3 / "lich-blender-5.1" / "baked").is_dir() else VISUAL_V2 / "lich-blender-5.1"),
]


def fragment(root: Path) -> tuple[Path, dict]:
    """The baked PNG and its fragment sidecar for one archetype directory."""
    sides = sorted((root / "baked").glob("*.fragment.json"))
    if not sides:
        raise SystemExit(f"{root}: no baked fragment")
    frag = json.loads(sides[0].read_text(encoding="utf-8"))
    return sides[0].parent / frag["sheet"]["file"], frag


def crop_of(frag: dict, key: str) -> tuple[int, int, int, int]:
    """Pixel box of a named cell in the fragment's own baked sheet."""
    for cell in frag["cells"]:
        if cell["key"] == key:
            cw = frag["sheet"]["cell_size"]["width"]
            ch = frag["sheet"]["cell_size"]["height"]
            col, row = cell["cell"]
            return (col * cw, row * ch, (col + 1) * cw, (row + 1) * ch)
    raise SystemExit(f"{frag['sheet']['name']}: no cell {key}")


def archetype_cells(png: Image.Image, frag: dict) -> list[tuple[str, Image.Image]]:
    """Every cell of one archetype, cropped but not yet scaled."""
    return [(cell["key"], png.crop(crop_of(frag, cell["key"])).convert("RGBA")) for cell in frag["cells"]]


def standing(cell: Image.Image, box: tuple[int, int, int, int]) -> Image.Image:
    """Scale by the archetype's shared bbox and stand the figure on the floor."""
    x0, y0, x1, y1 = box
    character = cell.crop(box)
    scale = FIT_HEIGHT / max(1, y1 - y0)
    width = max(1, round((x1 - x0) * scale))
    out = Image.new("RGBA", (CELL, CELL), (0, 0, 0, 0))
    out.alpha_composite(character.resize((width, FIT_HEIGHT), Image.Resampling.LANCZOS), ((CELL - width) // 2, CELL - FIT_HEIGHT))
    return out


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--manifest", type=Path, default=MANIFEST)
    args = parser.parse_args()

    sources = [(name, root) for name, root in HEROES] + [(name, get_npc_root(name)) for name in ROSTER]
    rows = 0
    cells: list[Image.Image] = []
    mapping: dict[str, list[int]] = {}

    for index, (name, root) in enumerate(sources):
        png_path, frag = fragment(root)
        png = Image.open(png_path)
        parts = archetype_cells(png, frag)
        # One bbox for the whole archetype, so every frame shares a transform.
        boxes = [cell.getchannel("A").getbbox() for _, cell in parts]
        boxes = [b for b in boxes if b is not None]
        if not boxes:
            raise SystemExit(f"{name}: no opaque pixels in any cell")
        shared = (min(b[0] for b in boxes), min(b[1] for b in boxes), max(b[2] for b in boxes), max(b[3] for b in boxes))
        for key, cell in parts:
            mapping[key] = [len(cells) % COLUMNS, len(cells) // COLUMNS]
            cells.append(standing(cell, shared))
        # The flat key is the character standing still, facing the camera.
        flat = f"{name}.idle.front.000"
        if flat in mapping:
            mapping[name] = mapping[flat]

    # Pad the last row so the sheet is a whole number of rows.
    while len(cells) % COLUMNS:
        cells.append(Image.new("RGBA", (CELL, CELL), (0, 0, 0, 0)))
    rows = len(cells) // COLUMNS

    sheet = Image.new("RGBA", (COLUMNS * CELL, rows * CELL), (0, 0, 0, 0))
    for slot, cell in enumerate(cells):
        sheet.alpha_composite(cell, ((slot % COLUMNS) * CELL, (slot // COLUMNS) * CELL))

    manifest = json.loads(args.manifest.read_text(encoding="utf-8"))
    sheets = [s for s in manifest["sheets"] if s["name"] != SHEET_NAME and s["file"] != SHEET_FILE]
    sheets.append({"name": SHEET_NAME, "file": SHEET_FILE, "cell_size": CELL, "mapping": mapping})
    manifest["sheets"] = sheets
    args.manifest.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    out = args.manifest.parent / SHEET_FILE
    sheet.save(out)
    print("actor-sheet", out, f"{sheet.size[0]}x{sheet.size[1]}", "archetypes", len(sources), "keys", len(mapping))
    print("  sample:", json.dumps({k: mapping[k] for k in list(mapping)[:3]}))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
