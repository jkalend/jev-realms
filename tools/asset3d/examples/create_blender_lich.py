"""Legacy primitive Blender proof; retained only for comparison.

Do not use this fixture for the current Lich candidate. The current custom
humanoid source is ``create_blender_lich_v4.py``. This legacy output belongs
under ``lich-blender-5.1-cone-rejected/``.

The companion export is intentionally separate so the production adapter is
also exercised:
    F:/Blender/blender.exe <output>/blender/Lich.blend --background \
      --python tools/asset3d/blender_export.py -- \
      --manifest <output>/lich_manifest.json --output-root <output>
"""

from __future__ import annotations

import argparse
import json
import math
import sys
from pathlib import Path

import bpy  # type: ignore
from mathutils import Vector  # type: ignore


def args() -> argparse.Namespace:
    tail = sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else []
    parser = argparse.ArgumentParser()
    parser.add_argument("--output-root", type=Path, required=True)
    return parser.parse_args(tail)


ROOT = args().output_root.resolve()
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


def material(name: str, color: tuple[float, float, float, float], metallic: float = 0.0) -> bpy.types.Material:
    mat = bpy.data.materials.new(name)
    mat.use_nodes = True
    bsdf = mat.node_tree.nodes.get("Principled BSDF")
    bsdf.inputs["Base Color"].default_value = color
    bsdf.inputs["Metallic"].default_value = metallic
    bsdf.inputs["Roughness"].default_value = 0.62 if metallic == 0 else 0.3
    return mat


def finish(obj: bpy.types.Object, name: str, mat: bpy.types.Material) -> bpy.types.Object:
    obj.name = name
    obj["asset3d_id"] = "Lich"
    if obj.data is not None and hasattr(obj.data, "materials"):
        obj.data.materials.append(mat)
    return obj


def apply_scale(obj: bpy.types.Object) -> None:
    bpy.ops.object.select_all(action="DESELECT")
    obj.select_set(True)
    bpy.context.view_layer.objects.active = obj
    bpy.ops.object.transform_apply(location=False, rotation=False, scale=True)


def uv_sphere(name: str, location: tuple[float, float, float], scale: tuple[float, float, float], mat: bpy.types.Material) -> bpy.types.Object:
    bpy.ops.mesh.primitive_uv_sphere_add(segments=16, ring_count=8, location=location)
    obj = bpy.context.object
    obj.scale = scale
    apply_scale(obj)
    return finish(obj, name, mat)


def cone(name: str, location: tuple[float, float, float], radius1: float, radius2: float, depth: float, mat: bpy.types.Material) -> bpy.types.Object:
    bpy.ops.mesh.primitive_cone_add(vertices=16, radius1=radius1, radius2=radius2, depth=depth, location=location)
    return finish(bpy.context.object, name, mat)


def cube(name: str, location: tuple[float, float, float], scale: tuple[float, float, float], mat: bpy.types.Material) -> bpy.types.Object:
    bpy.ops.mesh.primitive_cube_add(size=1, location=location)
    obj = bpy.context.object
    obj.scale = scale
    apply_scale(obj)
    return finish(obj, name, mat)


def cylinder_between(name: str, a: tuple[float, float, float], b: tuple[float, float, float], radius: float, mat: bpy.types.Material) -> bpy.types.Object:
    a_v, b_v = Vector(a), Vector(b)
    direction = b_v - a_v
    bpy.ops.mesh.primitive_cylinder_add(vertices=10, radius=radius, depth=direction.length, location=(a_v + b_v) / 2)
    obj = bpy.context.object
    obj.rotation_mode = "QUATERNION"
    obj.rotation_quaternion = Vector((0, 0, 1)).rotation_difference(direction.normalized())
    return finish(obj, name, mat)


def marker(name: str, kind: str, location: tuple[float, float, float]) -> bpy.types.Object:
    obj = bpy.data.objects.new(name, None)
    bpy.context.collection.objects.link(obj)
    obj.location = location
    obj.empty_display_type = "PLAIN_AXES"
    obj.empty_display_size = 0.2
    obj["asset3d_id"] = "Lich"
    obj["asset3d_marker"] = kind
    return obj


def look_at(obj: bpy.types.Object, target: Vector) -> None:
    obj.rotation_euler = (target - obj.location).to_track_quat("-Z", "Y").to_euler()


def build_scene() -> bpy.types.Object:
    bpy.ops.wm.read_factory_settings(use_empty=True)
    scene = bpy.context.scene
    try:
        scene.render.engine = "BLENDER_EEVEE_NEXT"
    except TypeError:
        scene.render.engine = "BLENDER_EEVEE"
    scene.render.resolution_x, scene.render.resolution_y = CELL
    scene.render.resolution_percentage = 100
    scene.render.image_settings.file_format = "PNG"
    scene.render.image_settings.color_mode = "RGBA"
    scene.render.film_transparent = True
    scene.render.fps = 8
    if scene.world is None:
        scene.world = bpy.data.worlds.new("LichWorld")
    scene.world.color = (0.008, 0.01, 0.016)

    violet = material("Lich_Violet", (0.23, 0.07, 0.32, 1.0))
    violet_light = material("Lich_VioletLight", (0.42, 0.16, 0.52, 1.0))
    bone = material("Lich_Bone", (0.72, 0.68, 0.56, 1.0))
    bone_dark = material("Lich_BoneDark", (0.26, 0.25, 0.23, 1.0))
    gold = material("Lich_Gold", (0.72, 0.46, 0.10, 1.0), 0.75)
    teal = material("Lich_Teal", (0.10, 0.58, 0.58, 1.0), 0.2)
    black = material("Lich_Black", (0.015, 0.02, 0.025, 1.0))
    steel = material("Lich_Steel", (0.18, 0.22, 0.25, 1.0), 0.7)

    robe = cone("Lich__Robe", (0, 1.15, 0), 0.78, 0.30, 2.15, violet)
    torso = uv_sphere("Lich__Torso", (0, 2.03, 0), (0.48, 0.52, 0.30), violet_light)
    belt = cube("Lich__Belt", (0, 1.30, 0.05), (0.68, 0.08, 0.40), gold)
    shoulder_l = uv_sphere("Lich__ShoulderL", (-0.43, 2.25, 0), (0.25, 0.22, 0.25), violet)
    shoulder_r = uv_sphere("Lich__ShoulderR", (0.43, 2.25, 0), (0.25, 0.22, 0.25), violet)
    arm_l = cylinder_between("Lich__ArmL", (-0.43, 2.16, 0), (-0.62, 1.55, 0.05), 0.14, violet)
    arm_r = cylinder_between("Lich__ArmR", (0.43, 2.16, 0), (0.62, 1.55, 0.05), 0.14, violet)
    hand_l = uv_sphere("Lich__HandL", (-0.64, 1.48, 0.06), (0.13, 0.16, 0.13), bone)
    hand_r = uv_sphere("Lich__HandR", (0.64, 1.48, 0.06), (0.13, 0.16, 0.13), bone)
    hood = uv_sphere("Lich__Hood", (0, 2.80, -0.08), (0.50, 0.54, 0.43), violet)
    face = uv_sphere("Lich__Face", (0, 2.78, 0.28), (0.27, 0.30, 0.10), bone)
    eye_l = cube("Lich__EyeL", (-0.10, 2.86, 0.38), (0.07, 0.07, 0.04), black)
    eye_r = cube("Lich__EyeR", (0.10, 2.86, 0.38), (0.07, 0.07, 0.04), black)
    nose = cube("Lich__Nose", (0, 2.76, 0.43), (0.07, 0.13, 0.05), bone)
    jaw = cube("Lich__Jaw", (0, 2.60, 0.39), (0.19, 0.05, 0.05), bone_dark)
    for i in range(5):
        cube(f"Lich__Crown{i}", (-0.32 + i * 0.16, 3.20 + (0.08 if i % 2 == 0 else 0), 0), (0.08, 0.28 if i % 2 == 0 else 0.20, 0.08), gold)
    staff = cylinder_between("Lich__Staff", (0.78, 0.16, 0.22), (0.78, 3.15, 0.22), 0.055, gold)
    orb = uv_sphere("Lich__Orb", (0.78, 3.30, 0.22), (0.18, 0.18, 0.18), teal)
    clasp_l = cube("Lich__ClaspL", (-0.32, 1.78, 0.26), (0.15, 0.34, 0.06), gold)
    clasp_r = cube("Lich__ClaspR", (0.32, 1.78, 0.26), (0.15, 0.34, 0.06), gold)
    for side in (-1, 1):
        foot = cube(f"Lich__Foot{'L' if side < 0 else 'R'}", (side * 0.30, 0.12, 0.10), (0.30, 0.20, 0.52), black)
        knee = cylinder_between(f"Lich__Knee{'L' if side < 0 else 'R'}", (side * 0.30, 0.25, 0.04), (side * 0.30, 1.10, 0.02), 0.17, violet)
        leg = cylinder_between(f"Lich__Leg{'L' if side < 0 else 'R'}", (side * 0.30, 0.30, 0.02), (side * 0.30, 1.25, 0.0), 0.18, violet)
    back_seam = cube("Lich__BackSeam", (0, 2.62, -0.40), (0.07, 0.70, 0.05), violet_light)
    back_plate = uv_sphere("Lich__BackPlate", (0, 2.05, -0.31), (0.30, 0.55, 0.06), violet_light)

    arm_data = bpy.data.armatures.new("Lich_RigData")
    rig = bpy.data.objects.new("Lich_Rig", arm_data)
    bpy.context.collection.objects.link(rig)
    rig["asset3d_id"] = "Lich"
    rig["asset3d_rig"] = True
    bpy.context.view_layer.objects.active = rig
    rig.select_set(True)
    bpy.ops.object.mode_set(mode="EDIT")
    bones = arm_data.edit_bones
    root = bones.new("root"); root.head = (0, 0, 0); root.tail = (0, 1, 0)
    spine = bones.new("spine"); spine.head = (0, 1, 0); spine.tail = (0, 2.2, 0); spine.parent = root
    neck = bones.new("neck"); neck.head = (0, 2.2, 0); neck.tail = (0, 2.7, 0); neck.parent = spine
    head = bones.new("head"); head.head = (0, 2.7, 0); head.tail = (0, 3.35, 0); head.parent = neck
    for name, x in (("arm.L", -0.43), ("arm.R", 0.43)):
        bone = bones.new(name); bone.head = (x, 2.18, 0); bone.tail = (x * 1.35, 1.45, 0.05); bone.parent = spine
    for name, x in (("leg.L", -0.30), ("leg.R", 0.30)):
        bone = bones.new(name); bone.head = (x, 1.25, 0); bone.tail = (x, 0.1, 0.05); bone.parent = root
    bpy.ops.object.mode_set(mode="OBJECT")

    mesh_objects = [obj for obj in bpy.context.scene.objects if obj.type == "MESH"]
    bpy.ops.object.select_all(action="DESELECT")
    rig.select_set(True)
    for obj in mesh_objects:
        obj.select_set(True)
    bpy.context.view_layer.objects.active = rig
    bpy.ops.object.parent_set(type="ARMATURE_AUTO")
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

    for name, kind, loc in (("Lich_front", "front", (0, 2.8, -1.4)), ("Lich_back", "back", (0, 2.8, 1.4)), ("Lich_pivot", "pivot", (0, 0, 0))):
        marker(name, kind, loc)

    camera_data = bpy.data.cameras.new("Lich_Camera")
    camera = bpy.data.objects.new("Lich_Camera", camera_data)
    bpy.context.collection.objects.link(camera)
    camera.location = (5.8, -7.8, 4.8)
    camera_data.type = "ORTHO"
    camera_data.ortho_scale = 5.4
    look_at(camera, Vector((0, 1.65, 0)))
    scene.camera = camera
    for name, loc, energy, color in (("Key", (4, -4, 7), 950, (1.0, 0.82, 0.65)), ("Fill", (-4, -2, 3), 500, (0.38, 0.48, 0.8)), ("Rim", (0, 4, 6), 1100, (0.55, 0.75, 1.0))):
        data = bpy.data.lights.new(f"Lich_{name}", type="AREA")
        data.energy = energy
        data.color = color
        light = bpy.data.objects.new(f"Lich_{name}", data)
        bpy.context.collection.objects.link(light)
        light.location = loc
        look_at(light, Vector((0, 1.6, 0)))
    return rig


def render_frames(rig: bpy.types.Object) -> None:
    scene = bpy.context.scene
    for animation, count in ANIMATIONS.items():
        for direction, angle in DIRECTIONS:
            target = ROOT / "render/Lich" / animation / direction
            target.mkdir(parents=True, exist_ok=True)
            for frame in range(count):
                rig.rotation_euler = (0, 0, angle)
                rig.location = (0, 0, 0)
                rig.scale = (1, 1, 1)
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
    ROOT.mkdir(parents=True, exist_ok=True)
    rig = build_scene()
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
            "metadata": {"source_kind": "blender_5_1", "front_axis": "-Z"},
        }]
    }
    (ROOT / "lich_manifest.json").write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    print("BLENDER_LICH_SCENE", blend_path)


if __name__ == "__main__":
    main()
