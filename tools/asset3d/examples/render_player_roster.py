#!/usr/bin/env python3
"""Render every player class in both body builds, then export/validate/bake them.

The player is the one asset that must exist twice over, so it gets its own
driver rather than being folded into the NPC roster pass: `Player.<Class>.<Build>`
keys the sprite, and each asset is authored with a piece of gear equipped so the
"items are visible on the character" contract is proved by the render, not by a
comment.

Run:
    python tools/asset3d/examples/render_player_roster.py --tier 2
    python tools/asset3d/examples/render_player_roster.py --process
"""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
EXAMPLES = Path(__file__).resolve().parent
BLENDER = r"F:/Blender/blender.exe"
PROFILES = json.loads((EXAMPLES / "player_profiles.json").read_text(encoding="utf-8"))
BUILDS = ("Male", "Female")
PIPELINE = ROOT / "tools/asset3d/pipeline.py"
EXPORT = ROOT / "tools/asset3d/blender_export.py"
PREVIEW = ROOT / "tools/asset3d/examples/make_blender_lich_preview.py"


def asset_name(cls: str, build: str) -> str:
    # Identifier-safe asset id; the dotted form is the sprite key only.
    return f"{cls}{build}"


def render(roster_root: Path, tier: int, force: bool = False) -> None:
    for index, cls in enumerate(PROFILES, 1):
        for build in BUILDS:
            asset = asset_name(cls, build)
            if not force and (roster_root / asset / "blender" / f"{asset}.blend").is_file():
                print(f"SKIP {asset} already authored", flush=True)
                continue
            print(f"PLAYER {index}/{len(PROFILES)} {asset}", flush=True)
            subprocess.run(
                [BLENDER, "--background", "--factory-startup", "--python", str(EXAMPLES / "create_blender_player_v1.py"),
                 "--", "--class", cls, "--build", build, "--output-root", str(roster_root / asset), f"--gear={tier}"],
                check=True, stdout=subprocess.DEVNULL,
            )


def process(roster_root: Path) -> None:
    for cls in PROFILES:
        for build in BUILDS:
            asset = asset_name(cls, build)
            root = roster_root / asset
            blend = root / "blender" / f"{asset}.blend"
            manifest = root / f"{asset.lower()}_manifest.json"
            print(f"PROCESS {asset}", flush=True)
            subprocess.run([BLENDER, str(blend), "--background", "--python", str(EXPORT), "--",
                            "--manifest", str(manifest), "--output-root", str(root)], check=True, stdout=subprocess.DEVNULL)
            subprocess.run([sys.executable, str(PIPELINE), "validate", str(manifest)], check=True)
            subprocess.run([sys.executable, str(PIPELINE), "bake", str(manifest),
                            "--output-dir", str(root / "baked"), "--sheet-name", f"{asset.lower()}-baked"], check=True)
            subprocess.run([sys.executable, str(PREVIEW), str(root), "--id", asset,
                            "--title", f"{asset.upper()} / BLENDER 5.1 PLAYER", "--output", str(root / f"{asset.lower()}-preview.png")],
                           check=True)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--roster-root", type=Path, default=ROOT / "docs/gfx/proto/ui-v1/player")
    parser.add_argument("--tier", type=int, default=2, help="gear tier to wear in the review render")
    parser.add_argument("--process", action="store_true", help="export/validate/bake after rendering")
    parser.add_argument("--force", action="store_true", help="re-author assets that already have a .blend")
    args = parser.parse_args()
    if not args.process:
        render(args.roster_root, args.tier, args.force)
    else:
        process(args.roster_root)
    print("PLAYER_ROSTER_COMPLETE", flush=True)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
