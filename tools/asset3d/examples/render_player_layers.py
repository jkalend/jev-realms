#!/usr/bin/env python3
"""Render bare player and tiered kit for the runtime 64px front-idle atlas.

    python tools/asset3d/examples/render_player_layers.py \
      --root docs/gfx/proto/ui-v1/layers --only all --force

Blender writes one front idle frame per layer; the authored .blend still has
complete rig/action tracks for the independent 3D asset pipeline.
"""

from __future__ import annotations

import argparse
import json
import subprocess
from pathlib import Path

EXAMPLES = Path(__file__).resolve().parent
BLENDER = r"F:/Blender/blender.exe"
PROFILES = json.loads((EXAMPLES / "player_profiles.json").read_text(encoding="utf-8"))
BUILDS = ("Male", "Female")
TIERS = range(4)
GENERATOR = EXAMPLES / "create_blender_player_v1.py"


def layer_name(kind: str, cls: str, build: str, tier: int | None) -> str:
    tag = "" if tier is None else f".t{tier}"
    return f"{cls}{build}.{kind}{tag}"


def render(root: Path, kind: str, cls: str, build: str, tier: int | None) -> Path:
    out = root / layer_name(kind, cls, build, tier)
    gear = {"body": "-1", "weapon": f"W{tier}", "armour": f"A{tier}"}[kind]
    subprocess.run(
        [BLENDER, "--background", "--factory-startup", "--python", str(GENERATOR), "--",
         "--class", cls, "--build", build, "--output-root", str(out),
         f"--gear={gear}", "--probe", "--atlas"],
        check=True, stdout=subprocess.DEVNULL,
    )
    return out


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, default=Path("docs/gfx/proto/ui-v1/layers"))
    parser.add_argument("--class", dest="cls", choices=tuple(PROFILES))
    parser.add_argument("--only", choices=("body", "weapon", "armour", "all"), default="all")
    parser.add_argument("--force", action="store_true", help="overwrite existing layer renders")
    args = parser.parse_args()
    classes = (args.cls,) if args.cls else tuple(PROFILES)
    jobs: list[tuple[str, str, str, int | None]] = []
    if args.only in ("body", "all"):
        jobs += [("body", c, b, None) for c in classes for b in BUILDS]
    if args.only in ("weapon", "all"):
        jobs += [("weapon", c, b, t) for c in classes for b in BUILDS for t in TIERS]
    if args.only in ("armour", "all"):
        jobs += [("armour", c, b, t) for c in classes for b in BUILDS for t in TIERS]

    done = 0
    for index, (kind, cls, build, tier) in enumerate(jobs, 1):
        out = args.root / layer_name(kind, cls, build, tier)
        marker = out / "render" / f"{cls}{build}" / "idle" / "front" / "000.png"
        if marker.is_file() and not args.force:
            continue
        print(f"LAYER {index}/{len(jobs)} {kind} {cls}.{build}" + ("" if tier is None else f" t{tier}"), flush=True)
        render(args.root, kind, cls, build, tier)
        done += 1
    print(f"PLAYER_LAYERS_COMPLETE rendered={done} total={len(jobs)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
