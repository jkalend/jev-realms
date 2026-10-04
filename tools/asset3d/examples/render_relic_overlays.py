#!/usr/bin/env python3
"""Render the nine boss relics as overlay plates the view can hang on the player.

A relic is a chest pendant, not a whole character, so it cannot live in the
gear matrix (12 x 4 x 4 x 9 plates is not a sheet anyone should bake). Instead
each overlay is the DIFFERENCE between the player wearing the relic and the same
player bare, mapped through the body layer's own fit transform. The result is a
transparent 64x64 cell with the pendant already in the right place, so the view
draws it on top of the body plate with no offset maths.

One plate per relic per build: the chest socket scales with the build.

Run:
    python tools/asset3d/examples/render_relic_overlays.py \
      --layers docs/gfx/proto/ui-v1/layers --output docs/gfx/proto/ui-v1/relics
"""

from __future__ import annotations

import argparse
import json
import subprocess
from pathlib import Path

from PIL import Image

from bake_player_sheet import fit_cell, gear_delta

EXAMPLES = Path(__file__).resolve().parent
BLENDER = r"F:/Blender/blender.exe"
GENERATOR = EXAMPLES / "create_blender_player_v1.py"
RELICS = (
    "Rallybreaker", "Fangmantle", "Graveglass", "Saltcrown", "Stoneheart",
    "GnawboneCrown", "TollcoinCharm", "WisplightLantern", "Hartshorn",
)
BUILDS = ("Male", "Female")
CELL = 64
COLUMNS = 9
REFERENCE = "Keepwarden"


def render(root: Path, cls: str, build: str, relic: str, force: bool) -> Path:
    out = root / f"{cls}{build}.relic.{relic}"
    if not force and (out / "render" / f"{cls}{build}" / "idle" / "front" / "000.png").is_file():
        return out
    subprocess.run(
        [BLENDER, "--background", "--factory-startup", "--python", str(GENERATOR), "--",
         "--class", cls, "--build", build, "--output-root", str(out), "--gear=-1",
         f"--relic={relic}", "--probe", "--atlas"],
        check=True, stdout=subprocess.DEVNULL,
    )
    return out




def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--layers", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--force", action="store_true", help="overwrite existing relic renders")
    args = parser.parse_args()

    args.output.mkdir(parents=True, exist_ok=True)
    sheet = Image.new("RGBA", (COLUMNS * CELL, len(BUILDS) * CELL), (0, 0, 0, 0))
    mapping: dict[str, list[int]] = {}
    missing: list[str] = []

    for row, build in enumerate(BUILDS):
        base_dir = args.layers / f"{REFERENCE}{build}.body"
        base = base_dir / "render" / f"{REFERENCE}{build}" / "idle" / "front" / "000.png"
        if not base.is_file():
            raise SystemExit(f"missing body layer for {REFERENCE}.{build}: run render_player_layers --only body")
        base_image = Image.open(base).convert("RGBA")
        box = base_image.getchannel("A").getbbox()
        if box is None:
            raise SystemExit(f"empty body layer: {base}")

        for col, relic in enumerate(RELICS):
            out = render(args.layers, REFERENCE, build, relic, args.force)
            worn_path = out / "render" / f"{REFERENCE}{build}" / "idle" / "front" / "000.png"
            if not worn_path.is_file():
                missing.append(f"{relic}.{build}")
                continue
            worn = Image.open(worn_path).convert("RGBA")
            sheet.alpha_composite(fit_cell(gear_delta(base_image, worn), box),
                                  (col * CELL, row * CELL))
            mapping[f"Player.Relic.{relic}.{build}"] = [col, row]

    (args.output / "relics.json").write_text(json.dumps({
        "sheet": "player_relics.png",
        "cell_size": {"width": CELL, "height": CELL},
        "mapping": mapping,
    }, indent=2) + "\n", encoding="utf-8")
    sheet.save(args.output / "player_relics.png")
    print("relic-overlays", args.output / "player_relics.png", "keys", len(mapping))
    if missing:
        print("MISSING", len(missing), " ".join(missing))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
