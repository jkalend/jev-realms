"""Legacy imported-mesh Blender proof; retained only for comparison.

The first Blender proof used fresh primitives and produced an unacceptable
cone. This version imports the already-reviewed mesh, but still inherits its
primitive silhouette. The current custom humanoid source is
``create_blender_lich_v4.py``; this proof belongs under
``lich-blender-5.1-imported-cone-rejected/``.
"""

from __future__ import annotations

import json
import math
import sys
from pathlib import Path

import bpy  # type: ignore
from mathutils import Vector  # type: ignore

_TAIL = sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else []
_ROOT_INDEX = _TAIL.index("--output-root") + 1 if "--output-root" in _TAIL else None
ROOT = Path(_TAIL[_ROOT_INDEX]).resolve() if _ROOT_INDEX is not None else Path.cwd()
SOURCE = ROOT.parent / "lich-3d-example/source/Lich.glb"
CELL = (128, 160)
DIRECTIONS = [
    ("front", 0.0),
    ("front_right", math.radians(45)),
    ("right", math.radians(90)),
    ("back_right", math.radians(135)),
    ("back", math.pi),
    ("back_left", math.radians(225)),
    ("left", math.radians(270)),
    ("front_left", math.radians(315)),
]
ANIMATIONS = {"idle": 1, "walk": 2, "attack": 3, "hit": 1}


def finish_imported(obj: bpy.types.Object) -> None:
    obj["asset3d_id"] = "Lich"
    if obj.type == "MESH" and hasattr(obj.data, "materials"):
        for mat in obj.data.materials:
            if mat and mat.use_nodes:
                bsdf = mat.node_tree.nodes.get("Principled BSDF")
                if bsdf:
                    bsdf.inputs["Roughness"].default_value = 0.48
                    bsdf.inputs["Metallic"].default_value = 0.12


def look_at(obj: bpy.types.Object, target: Vector) -> None:
    obj.rotation_euler = (target - obj.location).to_track_quat("-Z", "Y").to_euler()


def marker(name: str, kind: str, location: tuple[float, float, float]) -> bpy.types.Object:
    existing = bpy.data.objects.get(name)
    if existing:
        obj = existing
    else:
        obj = bpy.data.objects.new(name, None)
        bpy.context.collection.objects.link(obj)
        obj.empty_display_type = "PLAIN_AXES"
        obj.empty_display_size = 0.2
    obj.location = location
    obj["asset3d_id"] = "Lich"
    obj["asset3d_marker"] = kind
    return obj


def ensure_rig(objects: list[bpy.types.Object]) -> bpy.types.Object:
    rig = next((obj for obj in objects if obj.type == "ARMATURE"), None)
    if rig is None:
        rig = next((obj for obj in objects if obj.name == "Lich_Rig"), None)
    if rig is None:
        data = bpy.data.armatures.new("Lich_RigData")
        rig = bpy.data.objects.new("Lich_Rig", data)
        bpy.context.collection.objects.link(rig)
        bpy.context.view_layer.objects.active = rig
        rig.select_set(True)
        bpy.ops.object.mode_set(mode="EDIT")
        root = data.edit_bones.new("root"); root.head = (0, 0, 0); root.tail = (0, 1, 0)
        spine = data.edit_bones.new("spine"); spine.head = (0, 1, 0); spine.tail = (0, 2.2, 0); spine.parent = root
        neck = data.edit_bones.new("neck"); neck.head = (0, 2.2, 0); neck.tail = (0, 2.7, 0); neck.parent = spine
        head = data.edit_bones.new("head"); head.head = (0, 2.7, 0); head.tail = (0, 3.35, 0); head.parent = neck
        for name, x in (("arm.L", -0.43), ("arm.R", 0.43)):
            bone = data.edit_bones.new(name); bone.head = (x, 2.18, 0); bone.tail = (x * 1.35, 1.45, 0.05); bone.parent = spine
        for name, x in (("leg.L", -0.30), ("leg.R", 0.30)):
            bone = data.edit_bones.new(name); bone.head = (x, 1.25, 0); bone.tail = (x, 0.1, 0.05); bone.parent = root
        bpy.ops.object.mode_set(mode="OBJECT")
        bpy.ops.object.select_all(action="DESELECT")
        rig.select_set(True)
        for obj in objects:
            if obj.type == "MESH":
                obj.select_set(True)
        bpy.context.view_layer.objects.active = rig
        bpy.ops.object.parent_set(type="ARMATURE_AUTO")
    rig["asset3d_id"] = "Lich"
    rig["asset3d_rig"] = True
    return rig


def add_lighting(scene: bpy.types.Scene) -> None:
    for name, loc, energy, color in (
        ("Key", (4.5, -4.5, 7.5), 1250, (1.0, 0.78, 0.58)),
        ("Fill", (-4.0, -2.0, 3.8), 700, (0.34, 0.43, 0.85)),
        ("Rim", (0.0, 4.5, 6.5), 1500, (0.48, 0.72, 1.0)),
    ):
        data = bpy.data.lights.new(f"Lich_{name}", type="AREA")
        data.energy = energy
        data.color = color
        light = bpy.data.objects.new(f"Lich_{name}", data)
        bpy.context.collection.objects.link(light)
        light.location = loc
        look_at(light, Vector((0, 1.7, 0)))


def add_animation(rig: bpy.types.Object) -> None:
    rig.animation_data_create()
    action = bpy.data.actions.new("Lich_Idle")
    rig.animation_data.action = action
    for frame, bob, sway in ((1, 0.0, 0.0), (5, 0.035, 0.018), (9, 0.0, 0.0)):
        rig.location = (0.0, 0.0, bob)
        rig.rotation_euler = (0.0, sway, 0.0)
        rig.keyframe_insert(data_path="location", frame=frame, group="Lich")
        rig.keyframe_insert(data_path="rotation_euler", frame=frame, group="Lich")
    bpy.context.scene.frame_start = 1
    bpy.context.scene.frame_end = 9


def render_frames(rig: bpy.types.Object) -> None:
    scene = bpy.context.scene
    for animation, count in ANIMATIONS.items():
        for direction, angle in DIRECTIONS:
            target = ROOT / "render/Lich" / animation / direction
            target.mkdir(parents=True, exist_ok=True)
            for frame in range(count):
                rig.rotation_euler = (0.0, 0.0, angle)
                rig.location = (0.0, 0.0, 0.0)
                rig.scale = (1.0, 1.0, 1.0)
                if animation == "walk":
                    rig.location.x = -0.10 if frame == 0 else 0.10
                    rig.rotation_euler.y = -0.04 if frame == 0 else 0.04
                elif animation == "attack":
                    rig.rotation_euler.x = -0.06 * frame
                    rig.scale = (1.0 + frame * 0.015,) * 3
                elif animation == "hit":
                    rig.rotation_euler.y = -0.10
                    rig.location.x = -0.08
                scene.render.filepath = str(target / f"{frame:03d}.png")
                bpy.ops.render.render(write_still=True)


def main() -> None:
    if not SOURCE.is_file():
        raise SystemExit(f"known-good source GLB missing: {SOURCE}")
    bpy.ops.wm.read_factory_settings(use_empty=True)
    scene = bpy.context.scene
    scene.render.engine = "BLENDER_EEVEE"
    scene.render.resolution_x, scene.render.resolution_y = CELL
    scene.render.resolution_percentage = 100
    scene.render.image_settings.file_format = "PNG"
    scene.render.image_settings.color_mode = "RGBA"
    scene.render.film_transparent = True
    scene.render.fps = 8
    if scene.world is None:
        scene.world = bpy.data.worlds.new("LichWorld")
    scene.world.color = (0.006, 0.008, 0.014)

    before = set(bpy.data.objects)
    bpy.ops.import_scene.gltf(filepath=str(SOURCE))
    imported = [obj for obj in bpy.data.objects if obj not in before]
    for obj in list(imported):
        if obj.type == "MESH" and not obj.data.materials:
            bpy.data.objects.remove(obj, do_unlink=True)
            imported.remove(obj)
    for obj in imported:
        finish_imported(obj)
    rig = ensure_rig(imported)
    marker("Lich_front", "front", (0, 2.8, -1.4))
    marker("Lich_back", "back", (0, 2.8, 1.4))
    marker("Lich_pivot", "pivot", (0, 0, 0))
    add_lighting(scene)
    add_animation(rig)

    data = bpy.data.cameras.new("Lich_Camera")
    camera = bpy.data.objects.new("Lich_Camera", data)
    bpy.context.collection.objects.link(camera)
    camera.location = (5.8, -7.8, 4.8)
    data.type = "ORTHO"
    data.ortho_scale = 5.2
    look_at(camera, Vector((0, 1.65, 0)))
    scene.camera = camera

    blend_path = ROOT / "blender/Lich.blend"
    blend_path.parent.mkdir(parents=True, exist_ok=True)
    bpy.ops.wm.save_as_mainfile(filepath=str(blend_path))
    render_frames(rig)
    manifest = {
        "version": 1,
        "assets": [{
            "id": "Lich", "kind": "boss", "source": "source/Lich.glb",
            "render_root": "render/Lich", "cell_size": {"width": CELL[0], "height": CELL[1]},
            "directions": [name for name, _ in DIRECTIONS], "animations": ANIMATIONS,
            "uvs": True, "materials": True, "rig": True, "pivot": "feet_center",
            "metadata": {"source_kind": "blender_5.1_imported_reviewed_mesh", "front_axis": "-Z"},
        }]
    }
    (ROOT / "lich_manifest.json").write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    print("BLENDER_LICH_V2", blend_path, "imported_objects", len(imported))


if __name__ == "__main__":
    main()
