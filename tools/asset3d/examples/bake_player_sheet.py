#!/usr/bin/env python3
"""Composite the player layers into one plate per (class, build, weapon, armour).

The 3D pipeline renders the body, each weapon tier and each plate tier as
independent layers (`render_player_layers.py`). Compositing them here means the
view can swap a weapon or a plate by changing one key, with no re-render.

Layout: a 17-column grid, 12 rows - sixteen columns for the 4x4
weapon/armour matrix plus a dedicated bare body in column 16. The
`Player.<Class>.<Build>` fallback never aliases W0A0.

Run:
    python tools/asset3d/examples/bake_player_sheet.py \
      --layers docs/gfx/proto/ui-v1/layers \
      --relics docs/gfx/proto/ui-v1/relics
"""

from __future__ import annotations

import argparse
import json
import shutil
from pathlib import Path

from PIL import Image, ImageChops

CELL = 64
FIT_HEIGHT = 62
COLUMNS = 17
TIERS = 4
PROFILES = Path(__file__).with_name("player_profiles.json")
MANIFEST = Path("assets/atlas/manifest.json")
SHEET_NAME = "players"
SHEET_FILE = "players.png"
RELIC_SHEET = "player_relics"


def fit_cell(image: Image.Image, box: tuple[int, int, int, int]) -> Image.Image:
    """Use the BARE body's frame for every layer of a given class/build.

    Cropping each weapon to its own bounds used to rescale and recenter it; a
    longer sword could land at the same height as a short one (or off the hand).
    """
    scale = FIT_HEIGHT / (box[3] - box[1])
    width = max(1, round(image.width * scale))
    height = max(1, round(image.height * scale))
    x = round((CELL - (box[2] - box[0]) * scale) / 2 - box[0] * scale)
    y = round(CELL - FIT_HEIGHT - box[1] * scale)
    cell = Image.new("RGBA", (CELL, CELL), (0, 0, 0, 0))
    cell.alpha_composite(image.resize((width, height), Image.Resampling.LANCZOS), (x, y))
    return cell


def gear_delta(bare: Image.Image, worn: Image.Image) -> Image.Image:
    """Isolate authored gear from a complete Blender body+gear render."""
    channels = ImageChops.difference(bare.convert("RGB"), worn.convert("RGB")).split()
    change = ImageChops.lighter(channels[0], ImageChops.lighter(channels[1], channels[2]))
    change = ImageChops.lighter(change, ImageChops.difference(bare.getchannel("A"), worn.getchannel("A")))
    mask = change.point(lambda value: 255 if value > 10 else 0)
    overlay = worn.copy()
    overlay.putalpha(ImageChops.multiply(worn.getchannel("A"), mask))
    return overlay


def over(base: Image.Image, layer: Image.Image) -> Image.Image:
    out = base.copy()
    out.alpha_composite(layer)
    return out


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--layers", type=Path, required=True)
    parser.add_argument("--manifest", type=Path, default=MANIFEST)
    parser.add_argument("--relics", type=Path, help="relic overlay directory, if baked")
    args = parser.parse_args()

    profiles = json.loads(PROFILES.read_text(encoding="utf-8"))
    rows = [(cls, build) for cls in profiles for build in ("Male", "Female")]
    sheet = Image.new("RGBA", (COLUMNS * CELL, len(rows) * CELL), (0, 0, 0, 0))
    mapping: dict[str, list[int]] = {}
    missing: list[str] = []

    def render(kind: str, cls: str, build: str, tier: int | None) -> Image.Image | None:
        tag = "" if tier is None else f".t{tier}"
        path = args.layers / f"{cls}{build}.{kind}{tag}" / "render" / f"{cls}{build}" / "idle" / "front" / "000.png"
        return Image.open(path).convert("RGBA") if path.is_file() else None

    for row, (cls, build) in enumerate(rows):
        asset = f"{cls}{build}"
        bare = render("body", cls, build, None)
        if bare is None:
            missing.append(f"{asset} body")
            continue
        box = bare.getchannel("A").getbbox()
        if box is None:
            raise SystemExit(f"{asset}: empty body render")
        body = fit_cell(bare, box)
        sheet.alpha_composite(body, ((COLUMNS - 1) * CELL, row * CELL))
        mapping[f"Player.{cls}.{build}"] = [COLUMNS - 1, row]

        weapon = [render("weapon", cls, build, t) for t in range(TIERS)]
        armour = [render("armour", cls, build, t) for t in range(TIERS)]
        for w in range(TIERS):
            for a in range(TIERS):
                if weapon[w] is None or armour[a] is None:
                    missing.append(f"{asset} W{w}A{a}")
                    continue
                col = a * TIERS + w
                plate = over(over(body, fit_cell(gear_delta(bare, armour[a]), box)),
                             fit_cell(gear_delta(bare, weapon[w]), box))
                sheet.alpha_composite(plate, (col * CELL, row * CELL))
                mapping[f"Player.{cls}.{build}.W{w}A{a}"] = [col, row]

    if missing:
        raise SystemExit(f"missing layers: {', '.join(missing[:12])}{' ...' if len(missing) > 12 else ''}")

    manifest = json.loads(args.manifest.read_text(encoding="utf-8"))
    # Drop by FILE as well as by name: a sheet that was once written under the
    # wrong key would otherwise linger alongside its replacement.
    owned_files = {SHEET_FILE, f"{RELIC_SHEET}.png"}
    sheets = [
        s for s in manifest["sheets"]
        if s["name"] not in (SHEET_NAME, RELIC_SHEET, f"{RELIC_SHEET}.png") and s["file"] not in owned_files
    ]
    sheets.append({"name": SHEET_NAME, "file": SHEET_FILE, "mapping": mapping})
    relic_meta = args.relics / "relics.json" if args.relics else None
    if relic_meta and relic_meta.is_file():
        relics = json.loads(relic_meta.read_text(encoding="utf-8"))
        sheets.append({"name": RELIC_SHEET, "file": relics["sheet"], "mapping": relics["mapping"]})
        shutil.copyfile(args.relics / relics["sheet"], args.manifest.parent / relics["sheet"])
        print("  relic sheets", len(relics["mapping"]))
    manifest["sheets"] = sheets
    args.manifest.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    out = args.manifest.parent / SHEET_FILE
    sheet.save(out)
    print("player-sheet", out, f"{sheet.size[0]}x{sheet.size[1]}", "keys", len(mapping))
    print("  sample:", json.dumps(dict(list(mapping.items())[:4])))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
