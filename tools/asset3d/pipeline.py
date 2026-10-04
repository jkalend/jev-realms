#!/usr/bin/env python3
"""Validate and bake authored 3D assets into the Laya sprite contract.

This module is intentionally independent of the Rust runtime. Its public
interface is the CLI:

    python tools/asset3d/pipeline.py validate <manifest> [--no-files]
    python tools/asset3d/pipeline.py inspect <manifest>
    python tools/asset3d/pipeline.py bake <manifest> --output-dir <dir>

The validator owns source/render contract checks. The baker owns deterministic
atlas packing and emits an intermediate fragment for the later manifest
adapter; it never overwrites assets/atlas/manifest.json.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import struct
import sys
from pathlib import Path
from typing import Any

try:
    from PIL import Image
except ImportError:  # validation/inspection remain useful without Pillow
    Image = None  # type: ignore[assignment]

ASSET_KINDS = {"actor", "player", "boss", "prop"}
REQUIRED_ANIMATIONS = {"idle", "walk", "attack", "hit"}
DIRECTION_8 = [
    "front",
    "front_right",
    "right",
    "back_right",
    "back",
    "back_left",
    "left",
    "front_left",
]
ID_RE = re.compile(r"^[A-Za-z][A-Za-z0-9_]*$")
DIRECTION_RE = re.compile(r"^[a-z][a-z0-9_]*$")


class PipelineError(ValueError):
    """A user-correctable manifest, source, or render contract failure."""


def load_manifest(path: Path) -> dict[str, Any]:
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except FileNotFoundError as exc:
        raise PipelineError(f"manifest does not exist: {path}") from exc
    except json.JSONDecodeError as exc:
        raise PipelineError(f"manifest is not valid JSON: {path}: {exc}") from exc
    if not isinstance(data, dict):
        raise PipelineError("manifest root must be an object")
    return data


def resolve_inside(root: Path, relative: str, label: str) -> Path:
    candidate = (root / relative).resolve()
    try:
        candidate.relative_to(root.resolve())
    except ValueError as exc:
        raise PipelineError(f"{label} escapes manifest root: {relative}") from exc
    return candidate


def require_keys(value: dict[str, Any], keys: set[str], label: str) -> None:
    missing = sorted(keys - value.keys())
    if missing:
        raise PipelineError(f"{label} missing required fields: {', '.join(missing)}")


def validate_action_profile(profile: Any, asset: dict[str, Any], label: str) -> None:
    if not isinstance(profile, dict):
        raise PipelineError(f"{label} must be an object")
    profile_id = profile.get("id")
    if not isinstance(profile_id, str) or not ID_RE.fullmatch(profile_id):
        raise PipelineError(f"{label}.id must match {ID_RE.pattern}")
    weapon = profile.get("weapon")
    if not isinstance(weapon, str) or not weapon.strip():
        raise PipelineError(f"{label}.weapon must be a non-empty string")
    clips = profile.get("clips")
    if not isinstance(clips, dict) or not clips:
        raise PipelineError(f"{label}.clips must be a non-empty object")
    for action, names in clips.items():
        if not isinstance(action, str) or not action:
            raise PipelineError(f"{label}.clips keys must be action names")
        if not isinstance(names, list) or not names or any(not isinstance(name, str) or not name.strip() for name in names):
            raise PipelineError(f"{label}.clips.{action} must be a non-empty string array")
    if asset["kind"] in {"actor", "player", "boss"} and not REQUIRED_ANIMATIONS.issubset(clips):
        raise PipelineError(f"{label}.clips must include {sorted(REQUIRED_ANIMATIONS)}")
    default_action = profile.get("default_action")
    if not isinstance(default_action, str) or not default_action.strip():
        raise PipelineError(f"{label}.default_action must be a non-empty string")
    all_clips = {name for names in clips.values() for name in names}
    if default_action not in all_clips:
        raise PipelineError(f"{label}.default_action must name one of the declared clips")
    if asset["kind"] == "boss":
        boss = profile.get("boss")
        if not isinstance(boss, dict) or not isinstance(boss.get("signature"), str) or not boss["signature"].strip():
            raise PipelineError(f"{label}.boss.signature must be a non-empty string")
        phases = boss.get("phase_actions")
        if not isinstance(phases, list) or not phases or any(not isinstance(name, str) or not name.strip() for name in phases):
            raise PipelineError(f"{label}.boss.phase_actions must be a non-empty string array")

def validate_asset_shape(asset: dict[str, Any], index: int) -> None:
    label = f"assets[{index}]"
    if not isinstance(asset, dict):
        raise PipelineError(f"{label} must be an object")
    require_keys(
        asset,
        {
            "id", "kind", "source", "render_root", "cell_size", "directions",
            "animations", "uvs", "materials", "rig", "pivot",
        },
        label,
    )
    asset_id = asset["id"]
    if not isinstance(asset_id, str) or not ID_RE.fullmatch(asset_id):
        raise PipelineError(f"{label}.id must match {ID_RE.pattern}")
    if asset["kind"] not in ASSET_KINDS:
        raise PipelineError(f"{label}.kind must be one of {sorted(ASSET_KINDS)}")
    if not isinstance(asset["source"], str) or not asset["source"].endswith(".glb"):
        raise PipelineError(f"{label}.source must be a relative .glb path")
    if not isinstance(asset["render_root"], str) or not asset["render_root"]:
        raise PipelineError(f"{label}.render_root must be a non-empty relative path")
    cell = asset["cell_size"]
    if not isinstance(cell, dict) or not isinstance(cell.get("width"), int) or not isinstance(cell.get("height"), int):
        raise PipelineError(f"{label}.cell_size must contain integer width/height")
    if cell["width"] < 16 or cell["height"] < 16:
        raise PipelineError(f"{label}.cell_size must be at least 16x16")
    directions = asset["directions"]
    if not isinstance(directions, list) or len(directions) not in (8, 16):
        raise PipelineError(f"{label}.directions must contain 8 or 16 entries")
    if len(set(directions)) != len(directions) or any(not isinstance(d, str) or not DIRECTION_RE.fullmatch(d) for d in directions):
        raise PipelineError(f"{label}.directions must be unique lowercase labels")
    if len(directions) == 8 and directions != DIRECTION_8:
        raise PipelineError(f"{label}.directions must use the canonical order {DIRECTION_8}")
    animations = asset["animations"]
    if not isinstance(animations, dict) or not animations:
        raise PipelineError(f"{label}.animations must be a non-empty object")
    if any(not isinstance(name, str) or not isinstance(count, int) or count < 1 for name, count in animations.items()):
        raise PipelineError(f"{label}.animations must map names to positive frame counts")
    if asset["kind"] in {"actor", "player", "boss"} and not REQUIRED_ANIMATIONS.issubset(animations):
        raise PipelineError(f"{label}.animations must include {sorted(REQUIRED_ANIMATIONS)}")
    if "action_profile" in asset:
        validate_action_profile(asset["action_profile"], asset, f"{label}.action_profile")
    for field in ("uvs", "materials", "rig"):
        if asset[field] is not True:
            raise PipelineError(f"{label}.{field} must be true for production assets")
    if asset["pivot"] not in {"feet_center", "ground_center"}:
        raise PipelineError(f"{label}.pivot must be feet_center or ground_center")


def expected_render_paths(asset: dict[str, Any], root: Path) -> list[Path]:
    render_root = resolve_inside(root, asset["render_root"], f"{asset['id']}.render_root")
    paths: list[Path] = []
    for animation, count in sorted(asset["animations"].items()):
        for direction in asset["directions"]:
            for frame in range(count):
                paths.append(render_root / animation / direction / f"{frame:03d}.png")
    return paths


def validate_manifest(manifest: dict[str, Any], root: Path, check_files: bool = True) -> dict[str, Any]:
    if manifest.get("version") != 1:
        raise PipelineError("manifest.version must be 1")
    assets = manifest.get("assets")
    if not isinstance(assets, list) or not assets:
        raise PipelineError("manifest.assets must be a non-empty array")
    ids: set[str] = set()
    render_count = 0
    for index, asset in enumerate(assets):
        validate_asset_shape(asset, index)
        if asset["id"] in ids:
            raise PipelineError(f"duplicate asset id: {asset['id']}")
        ids.add(asset["id"])
        if check_files:
            source = resolve_inside(root, asset["source"], f"{asset['id']}.source")
            if not source.is_file():
                raise PipelineError(f"missing GLB source: {source}")
            profile = asset.get("action_profile")
            required_clips = None
            if isinstance(profile, dict) and isinstance(profile.get("clips"), dict):
                required_clips = {name for names in profile["clips"].values() for name in names}
                boss_profile = profile.get("boss")
                if isinstance(boss_profile, dict) and isinstance(boss_profile.get("phase_actions"), list):
                    required_clips.update(boss_profile["phase_actions"])
            validate_glb(source, required_clips)
            render_root = resolve_inside(root, asset["render_root"], f"{asset['id']}.render_root")
            if not render_root.is_dir():
                raise PipelineError(f"missing render root: {render_root}")
            for path in expected_render_paths(asset, root):
                if not path.is_file():
                    raise PipelineError(f"missing directional render: {path}")
                render_count += 1
    return {"version": 1, "assets": len(assets), "render_files": render_count, "file_checked": check_files}



def validate_glb(path: Path, required_animation_names: set[str] | None = None) -> None:
    try:
        data = path.read_bytes()
    except OSError as exc:
        raise PipelineError(f"cannot read GLB source: {path}") from exc
    if len(data) < 20 or data[:4] != b"glTF":
        raise PipelineError(f"invalid GLB magic: {path}")
    version, total_length = struct.unpack_from("<II", data, 4)
    if version != 2 or total_length != len(data):
        raise PipelineError(f"invalid GLB header/length: {path}")
    json_length, json_type = struct.unpack_from("<II", data, 12)
    if json_type != 0x4E4F534A or 20 + json_length > len(data):
        raise PipelineError(f"invalid GLB JSON chunk: {path}")
    try:
        gltf = json.loads(data[20 : 20 + json_length].decode("utf-8"))
    except (UnicodeDecodeError, json.JSONDecodeError) as exc:
        raise PipelineError(f"invalid GLB JSON: {path}") from exc
    if gltf.get("asset", {}).get("version") != "2.0":
        raise PipelineError(f"GLB is not glTF 2.0: {path}")
    for key in ("meshes", "nodes", "materials", "buffers"):
        if not gltf.get(key):
            raise PipelineError(f"GLB missing {key}: {path}")
    if required_animation_names:
        names = {animation.get("name") for animation in gltf.get("animations", [])}
        missing = sorted(required_animation_names - names)
        if missing:
            raise PipelineError(f"GLB missing declared action clips {missing}: {path}")

def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def load_rgba(path: Path, expected: tuple[int, int], label: str) -> Image.Image:
    if Image is None:
        raise PipelineError("Pillow is required for bake; install Pillow in the asset tooling environment")
    try:
        image = Image.open(path)
        image.load()
    except Exception as exc:  # Pillow exposes several decode exception types
        raise PipelineError(f"cannot decode render {label}: {path}") from exc
    if image.size != expected:
        raise PipelineError(f"render size mismatch {label}: {path} is {image.size}, expected {expected}")
    if "A" not in image.getbands():
        raise PipelineError(f"render has no alpha channel: {path}")
    image = image.convert("RGBA")
    if image.getchannel("A").getbbox() is None:
        raise PipelineError(f"render is fully transparent: {path}")
    return image


def bake_manifest(manifest: dict[str, Any], root: Path, output_dir: Path, sheet_name: str) -> dict[str, Any]:
    report = validate_manifest(manifest, root, check_files=True)
    assets = manifest["assets"]
    cell_width = max(asset["cell_size"]["width"] for asset in assets)
    cell_height = max(asset["cell_size"]["height"] for asset in assets)
    entries: list[dict[str, Any]] = []
    for asset in assets:
        source = resolve_inside(root, asset["source"], f"{asset['id']}.source")
        source_hash = sha256(source)
        for animation, count in sorted(asset["animations"].items()):
            for direction in asset["directions"]:
                for frame in range(count):
                    path = resolve_inside(root, asset["render_root"], asset["id"]) / animation / direction / f"{frame:03d}.png"
                    image = load_rgba(path, (asset["cell_size"]["width"], asset["cell_size"]["height"]), f"{asset['id']}/{animation}/{direction}/{frame:03d}")
                    entries.append({
                        "asset_id": asset["id"],
                        "kind": asset["kind"],
                        "animation": animation,
                        "direction": direction,
                        "frame": frame,
                        "source": path.relative_to(root).as_posix(),
                        "source_sha256": sha256(path),
                        "glb_sha256": source_hash,
                        "image": image,
                    })

    columns = 8
    rows = (len(entries) + columns - 1) // columns
    sheet = Image.new("RGBA", (columns * cell_width, rows * cell_height), (0, 0, 0, 0))
    fragment_cells: list[dict[str, Any]] = []
    base_mapping: dict[str, list[int]] = {}
    states: dict[str, dict[str, dict[str, list[int]]]] = {}
    for index, entry in enumerate(entries):
        col, row = index % columns, index // columns
        image = entry.pop("image")
        x, y = col * cell_width, row * cell_height
        sheet.alpha_composite(image, (x + (cell_width - image.width) // 2, y + (cell_height - image.height) // 2))
        cell = [col, row]
        key = f"{entry['asset_id']}.{entry['animation']}.{entry['direction']}.{entry['frame']:03d}"
        fragment_cells.append({"key": key, **entry, "cell": cell})
        asset_id = entry["asset_id"]
        base_mapping.setdefault(asset_id, cell)
        states.setdefault(asset_id, {}).setdefault(entry["animation"], {})[entry["direction"]] = cell
    output_dir.mkdir(parents=True, exist_ok=True)
    sheet_path = output_dir / f"{sheet_name}.png"
    fragment_path = output_dir / f"{sheet_name}.fragment.json"
    sheet.save(sheet_path)
    fragment = {
        "version": 1,
        "sheet": {
            "name": sheet_name,
            "file": sheet_path.name,
            "cell_size": {"width": cell_width, "height": cell_height},
            "mapping": base_mapping,
            "states": states,
        },
        "cells": fragment_cells,
        "validation": report,
    }
    fragment_path.write_text(json.dumps(fragment, indent=2) + "\n", encoding="utf-8")
    return {"sheet": str(sheet_path), "fragment": str(fragment_path), "cells": len(entries), "size": [sheet.width, sheet.height]}


def inspect_manifest(manifest: dict[str, Any]) -> dict[str, Any]:
    assets = manifest.get("assets", [])
    return {
        "version": manifest.get("version"),
        "asset_count": len(assets),
        "assets": [
            {
                "id": asset.get("id"),
                "kind": asset.get("kind"),
                "directions": len(asset.get("directions", [])),
                "animations": asset.get("animations", {}),
                "cell_size": asset.get("cell_size"),
            }
            for asset in assets
        ],
    }


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)
    validate = sub.add_parser("validate", help="validate manifest structure and optional source/render files")
    validate.add_argument("manifest", type=Path)
    validate.add_argument("--no-files", action="store_true", help="validate the contract without requiring source/render files")
    validate.add_argument("--json", action="store_true", dest="as_json")
    inspect = sub.add_parser("inspect", help="print a compact manifest summary")
    inspect.add_argument("manifest", type=Path)
    inspect.add_argument("--json", action="store_true", dest="as_json")
    bake = sub.add_parser("bake", help="pack validated directional PNG renders into an intermediate atlas fragment")
    bake.add_argument("manifest", type=Path)
    bake.add_argument("--output-dir", type=Path, required=True)
    bake.add_argument("--sheet-name", default="actors-3d-baked")
    return parser


def main(argv: list[str] | None = None) -> int:
    args = build_parser().parse_args(argv)
    manifest_path = args.manifest.resolve()
    root = manifest_path.parent
    try:
        manifest = load_manifest(manifest_path)
        if args.command == "validate":
            result = validate_manifest(manifest, root, check_files=not args.no_files)
        elif args.command == "inspect":
            result = inspect_manifest(manifest)
        else:
            result = bake_manifest(manifest, root, args.output_dir.resolve(), args.sheet_name)
        print(json.dumps(result, indent=2))
        return 0
    except PipelineError as exc:
        print(f"asset3d pipeline error: {exc}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
