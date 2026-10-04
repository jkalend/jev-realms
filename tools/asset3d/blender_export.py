"""Blender adapter for the Laya Realms 3D asset contract.

Run inside Blender, not with the system Python:

    blender scene.blend --background --python tools/asset3d/blender_export.py -- \
        --manifest tools/asset3d/examples/actor_manifest.json \
        --output-root build/asset3d

The adapter validates each asset collection before exporting a GLB. It does
not import game/runtime code. The resulting GLB and sidecar JSON are consumed
by tools/asset3d/pipeline.py after the directional PNG renders exist.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import sys
from pathlib import Path
from typing import Any


def parse_args(argv: list[str]) -> argparse.Namespace:
    tail = argv[sys.argv.index("--") + 1 :] if "--" in argv else []
    parser = argparse.ArgumentParser()
    parser.add_argument("--manifest", type=Path, required=True)
    parser.add_argument("--output-root", type=Path, required=True)
    parser.add_argument("--check-only", action="store_true")
    return parser.parse_args(tail)


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def asset_objects(scene: Any, asset_id: str) -> list[Any]:
    return [
        obj for obj in scene.objects
        if obj.get("asset3d_id") == asset_id or obj.name.startswith(asset_id + "__")
    ]


def marker(objects: list[Any], name: str) -> bool:
    return any(obj.get("asset3d_marker") == name for obj in objects)


def validate_asset(scene: Any, asset: dict[str, Any]) -> list[str]:
    asset_id = asset["id"]
    objects = asset_objects(scene, asset_id)
    errors: list[str] = []
    if not objects:
        return [f"{asset_id}: no objects tagged asset3d_id={asset_id!r}"]
    meshes = [obj for obj in objects if obj.type == "MESH"]
    if not meshes:
        errors.append(f"{asset_id}: no mesh objects")
    for obj in meshes:
        if not obj.data.uv_layers:
            errors.append(f"{asset_id}:{obj.name}: missing UV layer")
        if not obj.data.materials:
            errors.append(f"{asset_id}:{obj.name}: missing material slots")
        if any(abs(value - 1.0) > 1e-4 for value in obj.scale):
            errors.append(f"{asset_id}:{obj.name}: apply object scale before export")
    if not any(obj.type == "ARMATURE" for obj in objects) and not any(obj.get("asset3d_rig") for obj in objects):
        errors.append(f"{asset_id}: missing armature or asset3d_rig marker")
    if not marker(objects, "front") or not marker(objects, "back"):
        errors.append(f"{asset_id}: add empty markers with asset3d_marker='front' and 'back'")
    if not marker(objects, "pivot"):
        errors.append(f"{asset_id}: add an empty pivot marker with asset3d_marker='pivot'")
    return errors


def export_asset(scene: Any, asset: dict[str, Any], output_root: Path) -> dict[str, Any]:
    import bpy  # type: ignore

    asset_id = asset["id"]
    objects = asset_objects(scene, asset_id)
    source_dir = output_root / "source"
    source_dir.mkdir(parents=True, exist_ok=True)
    glb_path = source_dir / f"{asset_id}.glb"
    sidecar_path = source_dir / f"{asset_id}.json"

    bpy.ops.object.select_all(action="DESELECT")
    for obj in objects:
        obj.select_set(True)
    if objects:
        bpy.context.view_layer.objects.active = objects[0]
    bpy.ops.export_scene.gltf(
        filepath=str(glb_path),
        export_format="GLB",
        use_selection=True,
        export_animations=True,
        export_texcoords=True,
        export_normals=True,
        export_materials="EXPORT",
        export_cameras=False,
        export_lights=False,
        export_skins=True,
        export_morph=True,
        export_apply=True,
        export_animation_mode="ACTIONS",
    )
    sidecar = {
        "version": 1,
        "id": asset_id,
        "kind": asset["kind"],
        "source": glb_path.name,
        "sha256": sha256(glb_path),
        "front_marker": "front",
        "back_marker": "back",
        "pivot": asset["pivot"],
        "objects": sorted(obj.name for obj in objects),
    }
    sidecar_path.write_text(json.dumps(sidecar, indent=2) + "\n", encoding="utf-8")
    return sidecar


def main() -> int:
    args = parse_args(sys.argv)
    manifest = json.loads(args.manifest.read_text(encoding="utf-8"))
    if manifest.get("version") != 1:
        raise SystemExit("manifest.version must be 1")
    import bpy  # type: ignore

    errors: list[str] = []
    reports: list[dict[str, Any]] = []
    for asset in manifest["assets"]:
        asset_errors = validate_asset(bpy.context.scene, asset)
        errors.extend(asset_errors)
        if not asset_errors and not args.check_only:
            reports.append(export_asset(bpy.context.scene, asset, args.output_root))
        elif not asset_errors:
            reports.append({"id": asset["id"], "validated": True})
    if errors:
        for error in errors:
            print(f"ERROR: {error}", file=sys.stderr)
        return 2
    print(json.dumps({"version": 1, "assets": reports}, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
