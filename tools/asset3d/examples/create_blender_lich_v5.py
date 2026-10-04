"""Refine the Blender Lich toward the canonical skeletal undead identity.

The v5 candidate keeps the lathed robe from v4, then replaces the round doll
face/hood with an open peaked cowl, a separate angular cape, a skull-like head,
exposed rib hints, bone hands and a natural asymmetrical staff-hand pose.
Directional PNGs use a separate turntable; idle/walk/attack/hit use articulated
arm/shoulder bones instead of root-only offsets.

Run:
    F:/Blender/blender.exe --background --python tools/asset3d/examples/create_blender_lich_v5.py -- \
      --output-root docs/gfx/proto/visual-v2/lich-blender-5.1-v5
"""

from __future__ import annotations

import json
import math
import sys
from pathlib import Path

import bmesh
import bpy  # type: ignore
from mathutils import Matrix  # type: ignore

sys.path.insert(0, str(Path(__file__).resolve().parent))
import create_blender_lich_v4 as base  # type: ignore

ROOT = base.ROOT
PROBE = base.PROBE
CELL = base.CELL
BASE_ROT = base.BASE_ROT
DIRECTIONS = base.DIRECTIONS
ANIMATIONS = base.ANIMATIONS
ACTION_PROFILE = {
    "id": "boss_lich_staff",
    "weapon": "staff",
    "default_action": "lich_staff_idle_guard",
    "clips": {
        "idle": ["lich_staff_idle_guard"],
        "walk": ["lich_staff_walk_a", "lich_staff_walk_b"],
        "attack": ["lich_staff_cast_windup", "lich_staff_cast_release", "lich_staff_cast_recover"],
        "hit": ["lich_staff_hit_recoil"],
    },
    "boss": {
        "signature": "summon_and_curse",
        "phase_actions": ["lich_phase_summon", "lich_phase_curse", "lich_phase_reinforce"],
    },
}


def mesh_object(name: str, vertices: list[tuple[float, float, float]], faces: list[tuple[int, ...]], mat: bpy.types.Material, subsurf: int = 1, thickness: float = 0.0) -> bpy.types.Object:
    mesh = bpy.data.meshes.new(f"{name}Mesh")
    mesh.from_pydata(vertices, [], faces)
    mesh.validate()
    bm = bmesh.new()
    bm.from_mesh(mesh)
    bmesh.ops.recalc_face_normals(bm, faces=list(bm.faces))
    bm.to_mesh(mesh)
    bm.free()
    obj = bpy.data.objects.new(name, mesh)
    bpy.context.collection.objects.link(obj)
    base.tag(obj, name, mat)
    if subsurf:
        mod = obj.modifiers.new("SoftTailoredSurface", "SUBSURF")
        mod.levels = subsurf
        mod.render_levels = subsurf
        base.apply_modifier(obj, mod)
    if thickness:
        mod = obj.modifiers.new("ClothThickness", "SOLIDIFY")
        mod.thickness = thickness
        mod.offset = 0.0
        base.apply_modifier(obj, mod)
    base.ensure_uv(obj)
    return obj


def make_cape(mat: bpy.types.Material) -> bpy.types.Object:
    # A broad sheet behind the robe: wider and pointed at the hem, visibly
    # separate from the round body silhouette in side and rear views.
    rows = [
        (2.18, 0.46, -0.27),
        (1.65, 0.55, -0.34),
        (1.10, 0.64, -0.41),
        (0.48, 0.73, -0.49),
        (0.16, 0.67, -0.45),
    ]
    columns = 9
    vertices: list[tuple[float, float, float]] = []
    for y, width, z in rows:
        for column in range(columns):
            t = -1.0 + 2.0 * column / (columns - 1)
            x = t * width
            scallop = 0.055 * (1.0 - t * t)
            fold = .040 * math.sin(4 * math.pi * column / (columns - 1)) * (1 - t * t)
            vertices.append((x, y - (0.05 if column in (0, columns - 1) and y < 0.5 else 0.0), z - scallop - fold))
    faces: list[tuple[int, ...]] = []
    for row in range(len(rows) - 1):
        for column in range(columns - 1):
            a = row * columns + column
            faces.append((a, a + 1, a + columns + 1, a + columns))
    return mesh_object("Lich_Cape", vertices, faces, mat, subsurf=1, thickness=0.045)


def make_hood(mat: bpy.types.Material, rim_mat: bpy.types.Material) -> tuple[bpy.types.Object, bpy.types.Object, bpy.types.Object]:
    # The cowl wraps the back and temples only; the open wedge faces +Z
    # (world -Y after BASE_ROT), leaving the cranium, sockets and jaw exposed.
    profile = [
        (2.30, 0.29, 0.17),
        (2.43, 0.38, 0.24),
        (2.66, 0.43, 0.27),
        (2.91, 0.38, 0.23),
        (3.14, 0.11, 0.09),
    ]
    steps = 25
    start = math.radians(48)
    end = math.radians(312)
    vertices: list[tuple[float, float, float]] = []
    for y, rx, rz in profile:
        for step in range(steps):
            theta = start + (end - start) * step / (steps - 1)
            vertices.append((rx * math.sin(theta), y, rz * math.cos(theta)))
    faces: list[tuple[int, ...]] = []
    for row in range(len(profile) - 1):
        for column in range(steps - 1):
            a = row * steps + column
            faces.append((a, a + 1, a + steps + 1, a + steps))
    hood = mesh_object("Lich_Hood", vertices, faces, mat, subsurf=1, thickness=0.055)
    edge_l = [(rx * math.sin(start), y, rz * math.cos(start) + 0.01) for y, rx, rz in profile]
    edge_r = [(rx * math.sin(end), y, rz * math.cos(end) + 0.01) for y, rx, rz in profile]
    return hood, base.add_curve_tube("Lich_HoodEdgeL", edge_l, 0.024, rim_mat), base.add_curve_tube("Lich_HoodEdgeR", edge_r, 0.024, rim_mat)


def add_skull(materials: dict[str, bpy.types.Material]) -> None:
    # The broad exposed forehead and narrow, separate lower jaw read as bone,
    # not a glowing ornament in the center of the hood.
    base.add_ico("Lich_SkullCranium", (0, 2.80, 0.085), (0.245, 0.285, 0.195), materials["bone"])
    base.add_ico("Lich_SkullMuzzle", (0, 2.635, 0.245), (0.125, 0.068, 0.092), materials["bone"])
    base.add_ico("Lich_MouthCavity", (0, 2.515, 0.330), (0.145, 0.080, 0.029), materials["black"])
    for side, label in ((-1, "L"), (1, "R")):
        base.add_curve_tube(
            f"Lich_Cheekbone{label}",
            [(side * 0.22, 2.72, 0.245), (side * 0.18, 2.61, 0.32), (side * 0.10, 2.58, 0.34)],
            0.028, materials["bone"],
        )
        base.add_curve_tube(
            f"Lich_JawRamus{label}",
            [(side * 0.19, 2.64, 0.22), (side * 0.17, 2.49, 0.30), (side * 0.105, 2.445, 0.345)],
            0.033, materials["bone"],
        )
        base.add_ico(f"Lich_Socket{label}", (side * 0.108, 2.765, 0.292), (0.092, 0.088, 0.040), materials["black"])
        base.add_ico(f"Lich_EyeGlow{label}", (side * 0.108, 2.765, 0.332), (0.028, 0.025, 0.011), materials["teal"])
        base.add_curve_tube(
            f"Lich_BrowRidge{label}",
            [(side * 0.025, 2.852, 0.295), (side * 0.11, 2.870, 0.293), (side * 0.20, 2.826, 0.25)],
            0.023, materials["bone"],
        )
    base.add_curve_tube("Lich_JawChin", [(-0.11, 2.45, 0.34), (0, 2.43, 0.355), (0.11, 2.45, 0.34)], 0.032, materials["bone"])
    base.add_curve_tube("Lich_NoseBridge", [(0, 2.76, 0.303), (0, 2.69, 0.362)], 0.018, materials["bone"])
    mesh_object(
        "Lich_NasalCavity",
        [(-0.046, 2.677, 0.363), (0.046, 2.677, 0.363), (0, 2.603, 0.375)],
        [(0, 1, 2)], materials["black"], subsurf=0, thickness=0.008,
    )
    for index, x in enumerate((-0.090, -0.045, 0.0, 0.045, 0.090)):
        base.add_ico(f"Lich_UpperTeeth{index}", (x, 2.551, 0.368), (0.018, 0.032, 0.014), materials["ivory"])
        base.add_ico(f"Lich_LowerTeeth{index}", (x, 2.482, 0.366), (0.017, 0.027, 0.014), materials["ivory"])


def add_ribs(materials: dict[str, bpy.types.Material]) -> None:
    # Separate paired ribs with dark gaps, rather than a circular chest badge.
    for side, label in ((-1, "L"), (1, "R")):
        base.add_curve_tube(f"Lich_RibShadow{label}", [(side * 0.13, 1.55, 0.299), (side * 0.17, 1.82, 0.301), (side * 0.12, 2.02, 0.294)], 0.052, materials["black"])
        for index, y in enumerate((1.61, 1.72, 1.83, 1.94)):
            outer = 0.25 - index * 0.013
            base.add_curve_tube(
                f"Lich_Rib{label}{index}",
                [(side * 0.035, y - 0.015, 0.363), (side * 0.14, y - 0.045, 0.351),
                 (side * outer, y + 0.035, 0.319)],
                0.025, materials["bone"],
            )
        base.add_curve_tube(f"Lich_Collarbone{label}", [(0, 1.98, 0.357), (side * 0.17, 2.045, 0.334), (side * 0.32, 2.025, 0.293)], 0.031, materials["bone"])
    base.add_curve_tube("Lich_Sternum", [(0, 1.58, 0.348), (0, 1.98, 0.359)], 0.026, materials["ivory"])


def add_book(materials: dict[str, bpy.types.Material]) -> None:
    # The closed tome sits on the front-left hip, immediately below the belt.
    # Its back cover nearly touches the robe; two visible straps bridge it to
    # the belt rather than suspending a decorative plaque in front of the body.
    def box(name: str, center: tuple[float, float, float], size: tuple[float, float, float], mat: bpy.types.Material) -> None:
        bpy.ops.mesh.primitive_cube_add(size=1, location=center)
        obj = bpy.context.object
        obj.scale = size
        bpy.ops.object.transform_apply(location=False, rotation=False, scale=True)
        base.tag(obj, name, mat)
        bevel = obj.modifiers.new("WornEdges", "BEVEL")
        bevel.width = 0.008
        bevel.segments = 2
        base.apply_modifier(obj, bevel)
        base.ensure_uv(obj)

    for index, (top_x, top_z, bottom_x) in enumerate(((-0.33, 0.16, -0.41), (-0.15, 0.31, -0.18))):
        base.add_curve_tube(
            f"Lich_BookStrap{index}",
            [(top_x, 1.27, top_z), (bottom_x, 1.19, 0.27), (bottom_x, 1.10, 0.31)],
            0.028, materials["gold"],
        )
    box("Lich_BookBackCover", (-0.30, 0.90, 0.282), (0.48, 0.57, 0.033), materials["violet_dark"])
    box("Lich_BookPages", (-0.285, 0.90, 0.337), (0.47, 0.52, 0.085), materials["ivory"])
    box("Lich_BookFrontCover", (-0.30, 0.90, 0.396), (0.48, 0.57, 0.033), materials["violet_dark"])
    box("Lich_BookSpine", (-0.553, 0.90, 0.339), (0.055, 0.575, 0.143), materials["gold"])
    # Pale fore-edge and top/bottom leaf stacks show its thickness from the
    # front as well as the three-quarter directions.
    box("Lich_BookForeEdge", (-0.045, 0.90, 0.341), (0.028, 0.51, 0.083), materials["ivory"])
    for edge, y in (("Top", 1.165), ("Bottom", 0.635)):
        box(f"Lich_BookPage{edge}", (-0.285, y, 0.341), (0.46, 0.026, 0.082), materials["ivory"])
        for side, x in (("L", -0.515), ("R", -0.085)):
            box(f"Lich_BookCorner{edge}{side}", (x, y, 0.421), (0.066, 0.068, 0.022), materials["gold"])
    box("Lich_BookClasp", (-0.060, 0.905, 0.439), (0.150, 0.085, 0.035), materials["gold"])
    base.add_curve_tube(
        "Lich_BookSigil",
        [(-0.30, 1.059, 0.420), (-0.388, 0.937, 0.420), (-0.30, 0.801, 0.420),
         (-0.212, 0.937, 0.420), (-0.30, 1.059, 0.420)],
        0.016, materials["gold"],
    )
    base.add_ico("Lich_BookSeal", (-0.30, 0.937, 0.429), (0.043, 0.062, 0.015), materials["teal"])


def add_arms(materials: dict[str, bpy.types.Material]) -> None:
    # Left hangs close to the body; right bends naturally to the staff. This
    # asymmetry is visible in front, three-quarter and side views.
    base.add_curve_tube("Lich_ArmL", [(-0.40, 2.04, 0), (-0.66, 1.76, -0.03), (-0.59, 1.40, 0.05), (-0.56, 1.25, 0.08)], 0.115, materials["violet"])
    base.add_curve_tube("Lich_ArmR", [(0.40, 2.04, 0), (0.76, 1.77, 0.10), (0.79, 1.46, 0.20), (0.78, 1.30, 0.22)], 0.11, materials["violet"])
    base.add_ico("Lich_HandL", (-0.56, 1.23, 0.08), (0.105, 0.14, 0.09), materials["bone"])
    base.add_ico("Lich_HandR", (0.78, 1.29, 0.22), (0.10, 0.13, 0.09), materials["bone"])
    for index, dx in enumerate((-0.045, 0, 0.045)):
        base.add_curve_tube(f"Lich_FingerL{index}", [(-0.56 + dx, 1.18, 0.09), (-0.56 + dx, 1.08, 0.10)], 0.012, materials["bone_dark"])
        base.add_curve_tube(f"Lich_FingerR{index}", [(0.78 + dx, 1.24, 0.23), (0.78 + dx, 1.16, 0.23)], 0.011, materials["bone_dark"])



def assign_arm_weights(parts: list[bpy.types.Object]) -> None:
    for obj in parts:
        if obj.name not in {"Lich_ArmL", "Lich_ArmR", "Lich_Staff", "Lich_StaffOrb"} and not obj.name.startswith(("Lich_Hand", "Lich_Finger")):
            continue
        for group in obj.vertex_groups:
            group.remove(list(range(len(obj.data.vertices))))
        if obj.name in {"Lich_Staff", "Lich_StaffOrb"}:
            group = obj.vertex_groups.get("weapon.R") or obj.vertex_groups.new(name="weapon.R")
            group.add(list(range(len(obj.data.vertices))), 1.0, "REPLACE")
            continue
        if obj.name.startswith("Lich_Hand") or obj.name.startswith("Lich_Finger"):
            side = "L" if ("HandL" in obj.name or "FingerL" in obj.name) else "R"
            group = obj.vertex_groups.get(f"hand.{side}") or obj.vertex_groups.new(name=f"hand.{side}")
            group.add(list(range(len(obj.data.vertices))), 1.0, "REPLACE")
            continue
        label = "L" if obj.name.endswith("L") else "R"
        upper = obj.vertex_groups.get(f"upper_arm.{label}") or obj.vertex_groups.new(name=f"upper_arm.{label}")
        fore = obj.vertex_groups.get(f"forearm.{label}") or obj.vertex_groups.new(name=f"forearm.{label}")
        hand = obj.vertex_groups.get(f"hand.{label}") or obj.vertex_groups.new(name=f"hand.{label}")
        for vertex in obj.data.vertices:
            if vertex.co.y >= 1.82:
                target = upper
            elif vertex.co.y >= 1.38:
                target = fore
            else:
                target = hand
            target.add([vertex.index], 1.0, "REPLACE")


def assign_detail_weights(parts: list[bpy.types.Object]) -> None:
    # Keep each hard-surface book component with the belt and every facial
    # component with the cranium when the spine/head turn independently.
    head_parts = ("Lich_Skull", "Lich_Cheekbone", "Lich_Jaw", "Lich_Socket",
                  "Lich_EyeGlow", "Lich_Brow", "Lich_Nose", "Lich_Nasal",
                  "Lich_Mouth", "Lich_UpperTeeth", "Lich_LowerTeeth",
                  "Lich_Hood", "Lich_Crown")
    for obj in parts:
        if obj.name.startswith("Lich_Book"):
            bone = "spine"
        elif obj.name.startswith(head_parts):
            bone = "head"
        else:
            continue
        indices = list(range(len(obj.data.vertices)))
        for group in obj.vertex_groups:
            group.remove(indices)
        group = obj.vertex_groups.get(bone) or obj.vertex_groups.new(name=bone)
        group.add(indices, 1.0, "REPLACE")
def add_rig() -> bpy.types.Object:
    parts = [obj for obj in bpy.context.scene.objects if obj.type == "MESH" and obj.get("asset3d_id") == "Lich"]
    data = bpy.data.armatures.new("Lich_RigData")
    rig = bpy.data.objects.new("Lich_Rig", data)
    bpy.context.collection.objects.link(rig)
    rig["asset3d_id"] = "Lich"
    rig["asset3d_rig"] = True
    rig.rotation_euler = BASE_ROT
    bpy.ops.object.select_all(action="DESELECT")
    bpy.context.view_layer.objects.active = rig
    rig.select_set(True)
    bpy.ops.object.mode_set(mode="EDIT")
    bones = data.edit_bones
    root = bones.new("root"); root.head = (0, 0, 0); root.tail = (0, 1, 0)
    spine = bones.new("spine"); spine.head = (0, 1, 0); spine.tail = (0, 2.18, 0); spine.parent = root
    neck = bones.new("neck"); neck.head = (0, 2.18, 0); neck.tail = (0, 2.53, 0); neck.parent = spine
    head = bones.new("head"); head.head = (0, 2.53, 0); head.tail = (0, 3.15, 0); head.parent = neck
    arm_points = {
        "L": ((-0.37, 2.04, 0), (-0.53, 1.76, -0.03), (-0.49, 1.43, 0.05), (-0.50, 1.27, 0.08)),
        "R": ((0.37, 2.04, 0), (0.62, 1.76, 0.10), (0.72, 1.48, 0.20), (0.77, 1.36, 0.22)),
    }
    for label, points in arm_points.items():
        upper = bones.new(f"upper_arm.{label}"); upper.head, upper.tail = points[0], points[1]; upper.parent = spine
        fore = bones.new(f"forearm.{label}"); fore.head, fore.tail = points[1], points[2]; fore.parent = upper; fore.use_connect = True
        hand = bones.new(f"hand.{label}"); hand.head, hand.tail = points[2], points[3]; hand.parent = fore; hand.use_connect = True
        if label == "R":
            weapon = bones.new("weapon.R"); weapon.head = points[2]; weapon.tail = (0.78, 3.35, 0.22); weapon.parent = hand
    bpy.ops.object.mode_set(mode="OBJECT")
    bpy.ops.object.select_all(action="DESELECT")
    rig.select_set(True)
    for obj in parts:
        obj.select_set(True)
    bpy.context.view_layer.objects.active = rig
    bpy.ops.object.parent_set(type="ARMATURE_AUTO")
    assign_arm_weights(parts)
    assign_detail_weights(parts)
    for obj in parts:
        obj.matrix_parent_inverse = Matrix.Identity(4)
    for marker in (bpy.data.objects.get("Lich_front"), bpy.data.objects.get("Lich_back"), bpy.data.objects.get("Lich_pivot")):
        if marker is not None:
            marker.parent = rig
            marker.matrix_parent_inverse = Matrix.Identity(4)
    return rig


def add_turntable(rig: bpy.types.Object) -> bpy.types.Object:
    turntable = bpy.data.objects.new("Lich_Turntable", None)
    bpy.context.collection.objects.link(turntable)
    turntable.empty_display_type = "PLAIN_AXES"
    turntable.empty_display_size = 0.25
    rig.parent = turntable
    rig.matrix_parent_inverse = Matrix.Identity(4)
    return turntable


def reset_pose(rig: bpy.types.Object) -> None:
    for bone in rig.pose.bones:
        bone.location = (0, 0, 0)
        bone.rotation_mode = "XYZ"
        bone.rotation_euler = (0, 0, 0)
        bone.scale = (1, 1, 1)


def pose(anim: str, frame: int) -> tuple[float, float, float]:
    """Return vertical bob, spine roll and head pitch in model-local radians."""
    if anim == "idle":
        return 0.0, -0.015, 0.02
    if anim == "walk":
        swing = 0.28 if frame == 0 else -0.28
        return (0.035 if frame == 0 else 0.0), swing * 0.10, -swing * 0.06
    if anim == "attack":
        if frame == 0:
            return 0.0, 0.12, -0.14
        if frame == 1:
            return -0.015, -0.08, 0.10
        return 0.0, -0.04, 0.04
    if anim == "phase_summon":
        return 0.0, -0.08, -0.16
    if anim == "phase_curse":
        return 0.0, 0.20, 0.14
    if anim == "phase_reinforce":
        return 0.0, -0.12, 0.08
    return -0.02, 0.16, -0.18

def apply_pose(rig: bpy.types.Object, anim: str, frame: int) -> None:
    reset_pose(rig)
    bob, spine_roll, head_pitch = pose(anim, frame)
    rig.location = (0, bob, 0)
    rig.rotation_euler = BASE_ROT
    rig.pose.bones["spine"].rotation_euler.z = spine_roll
    rig.pose.bones["head"].rotation_euler.z = head_pitch
    if anim == "idle":
        arm = {"upper_arm.L": -0.08, "forearm.L": 0.08, "upper_arm.R": 0.10, "forearm.R": -0.16}
    elif anim == "walk":
        sign = 1 if frame == 0 else -1
        arm = {"upper_arm.L": 0.30 * sign, "forearm.L": -0.20 * sign, "upper_arm.R": -0.24 * sign, "forearm.R": -0.12}
    elif anim == "attack":
        if frame == 0:
            arm = {"upper_arm.L": -0.45, "forearm.L": -0.70, "upper_arm.R": 0.55, "forearm.R": -0.72}
        elif frame == 1:
            arm = {"upper_arm.L": -0.20, "forearm.L": -0.30, "upper_arm.R": -0.62, "forearm.R": -0.08}
        else:
            arm = {"upper_arm.L": -0.10, "forearm.L": -0.18, "upper_arm.R": -0.30, "forearm.R": -0.28}
    elif anim == "phase_summon":
        arm = {"upper_arm.L": 0.62, "forearm.L": -0.82, "upper_arm.R": -0.58, "forearm.R": -0.86}
    elif anim == "phase_curse":
        arm = {"upper_arm.L": -0.70, "forearm.L": -0.55, "upper_arm.R": 0.74, "forearm.R": -0.62}
    elif anim == "phase_reinforce":
        arm = {"upper_arm.L": -0.34, "forearm.L": -0.42, "upper_arm.R": 0.42, "forearm.R": -0.46}
    else:
        arm = {"upper_arm.L": 0.52, "forearm.L": -0.78, "upper_arm.R": -0.46, "forearm.R": -0.82}
    for name, angle in arm.items():
        rig.pose.bones[name].rotation_euler.z = angle
    bpy.context.view_layer.update()

def build_action_library(rig: bpy.types.Object) -> None:
    rig.animation_data_create()
    idle_action = None
    clip_specs = [
        ("lich_staff_idle_guard", "idle", 0),
        ("lich_staff_walk_a", "walk", 0),
        ("lich_staff_walk_b", "walk", 1),
        ("lich_staff_cast_windup", "attack", 0),
        ("lich_staff_cast_release", "attack", 1),
        ("lich_staff_cast_recover", "attack", 2),
        ("lich_staff_hit_recoil", "hit", 0),
        ("lich_phase_summon", "phase_summon", 0),
        ("lich_phase_curse", "phase_curse", 0),
        ("lich_phase_reinforce", "phase_reinforce", 0),
    ]
    for name, animation, frame in clip_specs:
        action = bpy.data.actions.new(name)
        action.use_fake_user = True
        rig.animation_data.action = action
        if name == ACTION_PROFILE["default_action"]:
            idle_action = action
        for key_frame in (0, 1):
            apply_pose(rig, animation, frame)
            for bone in rig.pose.bones:
                bone.keyframe_insert(data_path="rotation_euler", frame=key_frame, group=bone.name)
            rig.keyframe_insert(data_path="location", frame=key_frame, group="Root")
            rig.keyframe_insert(data_path="rotation_euler", frame=key_frame, group="Root")
    rig.animation_data.action = idle_action
    bpy.context.scene.frame_set(0)


def render_frames(rig: bpy.types.Object, turntable: bpy.types.Object) -> None:
    scene = bpy.context.scene
    if PROBE:
        directions = DIRECTIONS[:1]
        animations = ANIMATIONS
    else:
        directions = DIRECTIONS
        animations = ANIMATIONS
    # The saved .blend keeps its default idle action; manual atlas poses must
    # not be overwritten by that action when Blender evaluates each render.
    rig.animation_data.action = None
    for animation, count in animations.items():
        for direction, angle in directions:
            target = ROOT / "render/Lich" / animation / direction
            target.mkdir(parents=True, exist_ok=True)
            for frame in range(count):
                turntable.rotation_euler = (0, 0, angle)
                apply_pose(rig, animation, frame)
                scene.render.filepath = str(target / f"{frame:03d}.png")
                bpy.ops.render.render(write_still=True)


def main() -> None:
    bpy.ops.wm.read_factory_settings(use_empty=True)
    base.add_scene()
    materials = {
        "violet": base.material("Lich_Robe", (0.19, 0.055, 0.28, 1), 0.04, 0.58),
        "violet_light": base.material("Lich_Hood", (0.30, 0.10, 0.40, 1), 0.03, 0.56),
        "violet_dark": base.material("Lich_Cape", (0.105, 0.025, 0.16, 1), 0.02, 0.66),
        "bone": base.material("Lich_Bone", (0.64, 0.59, 0.47, 1), 0.03, 0.52),
        "ivory": base.material("Lich_Ivory", (0.85, 0.79, 0.63, 1), 0.02, 0.56),
        "bone_dark": base.material("Lich_BoneDark", (0.20, 0.18, 0.15, 1), 0.02, 0.68),
        "gold": base.material("Lich_Gold", (0.72, 0.43, 0.08, 1), 0.76, 0.30),
        "teal": base.material("Lich_Teal", (0.04, 0.58, 0.56, 1), 0.12, 0.28),
        "black": base.material("Lich_Black", (0.004, 0.006, 0.009, 1), 0.0, 0.82),
    }
    body = base.make_profile_body(materials)
    make_cape(materials["violet_dark"])
    make_hood(materials["violet_dark"], materials["gold"])
    add_skull(materials)
    add_ribs(materials)
    base.add_uv_sphere("Lich_ShoulderMantle", (0, 2.03, -0.015), (0.50, 0.19, 0.31), materials["violet_light"])
    base.add_ico("Lich_CapeClasp", (0, 2.10, 0.34), (0.075, 0.075, 0.035), materials["gold"])
    add_arms(materials)
    for index in range(5):
        base.add_ico(f"Lich_Crown{index}", (-0.27 + index * 0.135, 3.08 + (0.04 if index % 2 == 0 else 0), -0.01), (0.055, 0.18 if index % 2 == 0 else 0.12, 0.055), materials["gold"])
    base.add_curve_tube("Lich_Staff", [(0.80, 0.10, 0.22), (0.78, 1.25, 0.21), (0.80, 2.30, 0.22), (0.78, 3.22, 0.22)], 0.042, materials["gold"])
    base.add_ico("Lich_StaffOrb", (0.78, 3.31, 0.22), (0.145, 0.145, 0.145), materials["teal"])
    base.add_torus("Lich_Belt", (0, 1.24, 0), 0.35, 0.032, materials["gold"], (math.pi / 2, 0, 0))
    add_book(materials)
    for x in (-0.22, 0, 0.22):
        base.add_curve_tube(f"Lich_RobeFold{x}", [(x, 0.22, 0.46), (x * 0.8, 0.70, 0.40), (x * 0.45, 1.18, 0.30)], 0.011, materials["violet_light"])
    base.add_curve_tube("Lich_CapeSeam", [(0, 2.05, -0.33), (0, 1.35, -0.43), (0, 0.35, -0.53)], 0.013, materials["violet_light"])
    base.add_markers()
    rig = add_rig()
    turntable = add_turntable(rig)
    build_action_library(rig)
    blend_path = ROOT / "blender/Lich.blend"
    blend_path.parent.mkdir(parents=True, exist_ok=True)
    bpy.ops.wm.save_as_mainfile(filepath=str(blend_path))
    render_frames(rig, turntable)
    manifest = {
        "version": 1,
        "assets": [{
            "id": "Lich", "kind": "boss", "source": "source/Lich.glb", "render_root": "render/Lich",
            "cell_size": {"width": CELL[0], "height": CELL[1]}, "directions": [n for n, _ in DIRECTIONS],
            "animations": ANIMATIONS, "uvs": True, "materials": True, "rig": True, "pivot": "feet_center",
            "action_profile": ACTION_PROFILE,
            "metadata": {"source_kind": "blender_5.1_skeletal_humanoid", "front_axis": "-Y"},
        }],
    }
    (ROOT / "lich_manifest.json").write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    print("BLENDER_LICH_V5", blend_path)


if __name__ == "__main__":
    main()
