#!/usr/bin/env python3
"""Export, validate, bake and preview every generated NPC roster asset."""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
PROFILES = ROOT / "tools/asset3d/examples/npc_roster_profiles.json"
BLENDER = r"F:/Blender/blender.exe"
EXPORT = ROOT / "tools/asset3d/blender_export.py"
PIPELINE = ROOT / "tools/asset3d/pipeline.py"
PREVIEW = ROOT / "tools/asset3d/examples/make_blender_lich_preview.py"


def outputs_present(root: Path, name: str) -> bool:
    return all(
        path.is_file()
        for path in (
            root / name / "source" / f"{name}.glb",
            root / name / f"{name.lower()}_manifest.json",
            root / name / f"{name.lower()}-preview.png",
        )
    ) and any((root / name / "baked").glob("*.png"))


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--roster-root", type=Path, required=True)
    parser.add_argument("--only", nargs="*", help="restrict to these archetype names")
    parser.add_argument("--force", action="store_true", help="reprocess assets that already have outputs")
    args = parser.parse_args()
    names = list(json.loads(PROFILES.read_text(encoding="utf-8")))
    if args.only:
        unknown = [name for name in args.only if name not in names]
        if unknown:
            raise SystemExit(f"unknown NPC(s): {' '.join(unknown)}")
        names = args.only
    for index, name in enumerate(names, 1):
        root = args.roster_root / name
        if not args.force and outputs_present(root, name):
            print(f"SKIP {index}/{len(names)} {name} already processed", flush=True)
            continue
        blend = root / "blender" / f"{name}.blend"
        manifest = root / f"{name.lower()}_manifest.json"
        print(f"PROCESS {index}/{len(names)} {name}", flush=True)
        subprocess.run([BLENDER, str(blend), "--background", "--python", str(EXPORT), "--", "--manifest", str(manifest), "--output-root", str(root)], check=True)
        subprocess.run([sys.executable, str(PIPELINE), "validate", str(manifest)], check=True)
        subprocess.run([sys.executable, str(PIPELINE), "bake", str(manifest), "--output-dir", str(root / "baked"), "--sheet-name", f"{name.lower()}-baked"], check=True)
        subprocess.run([sys.executable, str(PREVIEW), str(root), "--id", name, "--title", f"{name.upper()} / BLENDER 5.1 NPC ROSTER", "--output", str(root / f"{name.lower()}-preview.png")], check=True)
    print("ROSTER_PROCESS_COMPLETE", flush=True)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
