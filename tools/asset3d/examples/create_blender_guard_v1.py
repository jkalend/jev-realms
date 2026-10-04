"""Author a standalone Guard asset in Blender 5.1.

The Guard follows the canonical 2D read: steel helmet and nose guard, blue
armored tunic, leather/gold belt, kite shield and long spear. The body is a
custom lathed torso with separate jointed limbs; shield and spear have their own
bones so the action profile is weapon-specific rather than a generic idle sheet.

Run:
    F:/Blender/blender.exe --background --python tools/asset3d/examples/create_blender_guard_v1.py -- \
      --output-root docs/gfx/proto/visual-v2/guard-blender-5.1
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
from character_surface import cloth_radius

ROOT = base.ROOT
PROBE = base.PROBE
CELL = base.CELL
BASE_ROT = (math.pi / 2, 0, 0)
DIRECTIONS = base.DIRECTIONS
ANIMATIONS = {"idle": 1, "walk": 2, "attack": 3, "hit": 1}
ACTION_PROFILE = {
    "id": "npc_guard_spear",
    "default_action": "guard_spear_idle_guard",
    "weapon": "spear",
    "clips": {
        "idle": ["guard_spear_idle_guard"],
        "walk": ["guard_spear_walk_a", "guard_spear_walk_b"],
        "attack": [
            "guard_spear_thrust_windup",
            "guard_spear_thrust",
            "guard_spear_recover",
            "guard_shield_bash",
        ],
        "hit": ["guard_shield_hit_block"],
    },
}

ARM_POINTS = {
    "L": ((-0.40, 2.04, 0.0), (-0.59, 1.76, 0.02), (-0.54, 1.45, 0.10), (-0.55, 1.30, 0.14)),
    "R": ((0.40, 2.04, 0.0), (0.62, 1.77, 0.02), (0.75, 1.48, 0.11), (0.83, 1.30, 0.19)),
}
LEG_POINTS = {
    "L": ((-0.22, 1.12, 0.0), (-0.27, 0.62, 0.02), (-0.30, 0.12, 0.06)),
    "R": ((0.22, 1.12, 0.0), (0.27, 0.62, 0.02), (0.30, 0.12, 0.06)),
}


def tag(obj: bpy.types.Object, name: str, mat: bpy.types.Material | None = None) -> bpy.types.Object:
    obj.name = name
    obj["asset3d_id"] = "Guard"
    if mat is not None and hasattr(obj.data, "materials"):
        if mat.name not in {slot.name for slot in obj.data.materials}:
            obj.data.materials.append(mat)
    return obj


def mesh_object(name: str, vertices: list[tuple[float, float, float]], faces: list[tuple[int, ...]], mat: bpy.types.Material, subsurf: int = 0, thickness: float = 0.0, bevel: float = 0.0) -> bpy.types.Object:
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
    tag(obj, name, mat)
    if subsurf:
        mod = obj.modifiers.new("TailoredSurface", "SUBSURF")
        mod.levels = subsurf
        mod.render_levels = subsurf
        base.apply_modifier(obj, mod)
    if thickness:
        mod = obj.modifiers.new("ArmorThickness", "SOLIDIFY")
        mod.thickness = thickness
        mod.offset = 0.0
        base.apply_modifier(obj, mod)
    if bevel:
        mod = obj.modifiers.new("ArmorBevel", "BEVEL")
        mod.width = bevel
        mod.segments = 2
        base.apply_modifier(obj, mod)
    base.ensure_uv(obj)
    return obj


def add_sphere(name: str, loc: tuple[float, float, float], scale: tuple[float, float, float], mat: bpy.types.Material) -> bpy.types.Object:
    bpy.ops.mesh.primitive_uv_sphere_add(segments=20, ring_count=12, location=loc)
    obj = bpy.context.object
    obj.scale = scale
    bpy.ops.object.select_all(action="DESELECT")
    obj.select_set(True)
    bpy.context.view_layer.objects.active = obj
    bpy.ops.object.transform_apply(location=False, rotation=False, scale=True)
    tag(obj, name, mat)
    base.ensure_uv(obj)
    return obj


def add_ico(name: str, loc: tuple[float, float, float], scale: tuple[float, float, float], mat: bpy.types.Material) -> bpy.types.Object:
    bpy.ops.mesh.primitive_ico_sphere_add(subdivisions=2, radius=1, location=loc)
    obj = bpy.context.object
    obj.scale = scale
    bpy.ops.object.select_all(action="DESELECT")
    obj.select_set(True)
    bpy.context.view_layer.objects.active = obj
    bpy.ops.object.transform_apply(location=False, rotation=False, scale=True)
    tag(obj, name, mat)
    base.ensure_uv(obj)
    return obj


def add_curve(name: str, points: list[tuple[float, float, float]], radius: float, mat: bpy.types.Material) -> bpy.types.Object:
    data = bpy.data.curves.new(name, type="CURVE")
    data.dimensions = "3D"
    data.resolution_u = 16
    data.bevel_depth = radius
    data.bevel_resolution = 4
    spline = data.splines.new("BEZIER")
    spline.bezier_points.add(len(points) - 1)
    for point, co in zip(spline.bezier_points, points):
        point.co = co
        point.handle_left_type = "AUTO"
        point.handle_right_type = "AUTO"
    obj = bpy.data.objects.new(name, data)
    bpy.context.collection.objects.link(obj)
    data.materials.append(mat)
    bpy.ops.object.select_all(action="DESELECT")
    obj.select_set(True)
    bpy.context.view_layer.objects.active = obj
    bpy.ops.object.convert(target="MESH")
    obj = bpy.context.object
    tag(obj, name, mat)
    base.ensure_uv(obj)
    return obj


def add_torus(name: str, loc: tuple[float, float, float], major: float, minor: float, mat: bpy.types.Material, rotation: tuple[float, float, float] = (0, 0, 0)) -> bpy.types.Object:
    bpy.ops.mesh.primitive_torus_add(major_radius=major, minor_radius=minor, major_segments=28, minor_segments=10, location=loc, rotation=rotation)
    obj = tag(bpy.context.object, name, mat)
    base.ensure_uv(obj)
    return obj


def make_torso(materials: dict[str, bpy.types.Material]) -> bpy.types.Object:
    profile = [
        (0.90, 0.40, 0.27), (1.12, 0.42, 0.28), (1.34, 0.35, 0.25),
        (1.58, 0.42, 0.28), (1.88, 0.47, 0.29), (2.08, 0.45, 0.28),
        (2.20, 0.34, 0.23), (2.28, 0.17, 0.16),
    ]
    segments = 28
    vertices: list[tuple[float, float, float]] = []
    for y, rx, rz in profile:
        for j in range(segments):
            a = math.tau * j / segments
            fold = cloth_radius(a, y, profile[0][0], profile[-1][0], .025)
            vertices.append((rx * math.cos(a) * fold, y, rz * math.sin(a) * fold))
    faces: list[tuple[int, ...]] = []
    for row in range(len(profile) - 1):
        for j in range(segments):
            a = row * segments + j
            b = row * segments + (j + 1) % segments
            c = (row + 1) * segments + (j + 1) % segments
            d = (row + 1) * segments + j
            faces.append((a, b, c, d))
    bottom = len(vertices); vertices.append((0, profile[0][0], 0))
    top = len(vertices); vertices.append((0, profile[-1][0], 0))
    for j in range(segments):
        faces.append((bottom, (j + 1) % segments, j))
        start = (len(profile) - 1) * segments
        faces.append((top, start + j, start + (j + 1) % segments))
    obj = mesh_object("Guard_Torso", vertices, faces, materials["blue"], subsurf=1)
    for poly in obj.data.polygons:
        poly.material_index = 1 if poly.center.y > 1.78 and poly.center.z < 0 else 0
        poly.use_smooth = True
    obj.data.materials.append(materials["steel"])
    base.ensure_uv(obj)
    return obj


def make_helmet(materials: dict[str, bpy.types.Material]) -> None:
    # Exposed skin below the open-faced steel cap makes the head read as human.
    add_sphere("Guard_Neck", (0, 2.32, 0), (0.14, 0.15, 0.14), materials["skin"])
    add_sphere("Guard_Face", (0, 2.51, 0.02), (0.235, 0.26, 0.225), materials["skin"])
    segments = 16
    vertices: list[tuple[float, float, float]] = []
    for height, rx, rz in ((2.65, 0.285, 0.265), (2.77, 0.27, 0.245), (2.87, 0.19, 0.175), (2.93, 0.055, 0.055)):
        for j in range(segments):
            a = math.tau * j / segments
            z = rz * math.sin(a)
            vertices.append((rx * math.cos(a), height - (0.055 if z < 0 else 0), z))
    faces = [
        ((row + 1) * segments + j, (row + 1) * segments + (j + 1) % segments,
         row * segments + (j + 1) % segments, row * segments + j)
        for row in range(3) for j in range(segments)
    ]
    faces.append(tuple(reversed(range(3 * segments, 4 * segments))))
    mesh_object("Guard_Helmet", vertices, faces, materials["steel"], thickness=0.025, bevel=0.012)
    add_curve("Guard_HelmetCrest", [(0, 2.69, 0.27), (0, 2.91, 0.08), (0, 2.79, -0.20)],
              0.025, materials["gold"])
    mesh_object("Guard_HelmetNape",
                [(-0.23, 2.61, -0.21), (0, 2.61, -0.27), (0.23, 2.61, -0.21),
                 (-0.17, 2.37, -0.14), (0, 2.37, -0.20), (0.17, 2.37, -0.14)],
                [(0, 1, 4, 3), (1, 2, 5, 4)], materials["steel"],
                thickness=0.025, bevel=0.008)
    add_curve("Guard_Brow", [(-0.23, 2.67, 0.19), (0, 2.68, 0.28), (0.23, 2.67, 0.19)],
              0.035, materials["steel_light"])
    for side in (-1, 1):
        cheek = [
            (side * 0.245, 2.65, 0.13), (side * 0.26, 2.64, -0.07),
            (side * 0.225, 2.38, -0.035), (side * 0.22, 2.41, 0.14),
            (side * 0.20, 2.52, 0.22),
        ]
        mesh_object(f"Guard_Cheek{side}", cheek, [(0, 1, 2, 3), (0, 3, 4)],
                    materials["steel"], thickness=0.022, bevel=0.009)
        add_sphere(f"Guard_Eye{side}", (side * 0.09, 2.55, 0.247),
                   (0.038, 0.022, 0.019), materials["dark"])
    add_curve("Guard_NoseGuard", [(0, 2.68, 0.29), (0, 2.58, 0.29)],
              0.027, materials["steel_light"])
    add_sphere("Guard_Nose", (0, 2.48, 0.252), (0.042, 0.062, 0.04), materials["skin"])
    add_curve("Guard_Mouth", [(-0.055, 2.39, 0.231), (0, 2.39, 0.243),
                              (0.055, 2.39, 0.231)], 0.009, materials["dark"])


def make_armor(materials: dict[str, bpy.types.Material]) -> None:
    # The front follows the torso's oval cross-section rather than hovering
    # over it as a convex chest medallion.
    rows = [
        (2.07, 0.32, 0.19, 0.275),
        (1.92, 0.39, 0.19, 0.30),
        (1.72, 0.36, 0.19, 0.305),
        (1.48, 0.31, 0.18, 0.27),
    ]
    vertices = []
    for y, half_width, edge_z, center_z in rows:
        vertices.extend((half_width * t, y, edge_z + (center_z - edge_z) * (1 - t * t))
                        for t in (-1, -0.5, 0, 0.5, 1))
    faces = [
        ((row + 1) * 5 + col, (row + 1) * 5 + col + 1, row * 5 + col + 1, row * 5 + col)
        for row in range(3) for col in range(4)
    ]
    mesh_object("Guard_Cuirass", vertices, faces, materials["steel"], thickness=0.035, bevel=0.012)
    add_curve("Guard_CuirassHem", [(-0.34, 1.48, 0.22), (0, 1.47, 0.30), (0.34, 1.48, 0.22)],
              0.018, materials["gold"])
    add_curve("Guard_CuirassRidge", [(0, 2.06, .289), (0, 1.75, .32), (0, 1.48, .29)],
              .012, materials["steel_light"])
    for side in (-1, 1):
        add_curve(f"Guard_CuirassSide{side}", [(side * .37, 1.94, .22),
                  (side * .34, 1.72, .23), (side * .30, 1.48, .20)],
                  .012, materials["steel_light"])
    # Matching rear yoke and central seam clarify front versus back on turns.
    back = [
        (-0.31, 2.04, -0.22), (0, 2.10, -0.275), (0.31, 2.04, -0.22),
        (-0.36, 1.56, -0.22), (0, 1.51, -0.30), (0.36, 1.56, -0.22),
    ]
    mesh_object("Guard_Backplate", back, [(0, 1, 4, 3), (1, 2, 5, 4)],
                materials["steel"], thickness=0.025, bevel=0.01)
    add_curve("Guard_BackSeam", [(0, 1.99, -0.295), (0, 1.60, -0.32)],
              0.016, materials["steel_light"])
    segments = 20
    belt_vertices = [
        (rx * math.cos(math.tau * j / segments), y, rz * math.sin(math.tau * j / segments))
        for y, rx, rz in ((1.09, 0.43, 0.295), (1.20, 0.43, 0.295))
        for j in range(segments)
    ]
    belt_faces = [(segments + j, segments + (j + 1) % segments, (j + 1) % segments, j)
                  for j in range(segments)]
    mesh_object("Guard_Belt", belt_vertices, belt_faces, materials["leather"], thickness=0.018)
    add_torus("Guard_BeltBuckle", (0, 1.145, 0.315), 0.065, 0.014, materials["gold"])
    add_sphere("Guard_BeltPin", (0, 1.145, 0.337), (0.031, 0.013, 0.015), materials["gold"])


def make_shield(materials: dict[str, bpy.types.Material]) -> tuple[bpy.types.Object, bpy.types.Object]:
    center_x = -0.62
    rows = [(1.92, 0.17), (1.72, 0.24), (1.44, 0.25), (1.18, 0.19), (1.04, 0.06)]
    columns = 5
    vertices: list[tuple[float, float, float]] = []
    for y, half in rows:
        for column in range(columns):
            t = -1.0 + 2.0 * column / (columns - 1)
            x = center_x + t * half
            z = 0.30 + 0.10 * (1.0 - t * t)
            vertices.append((x, y, z))
    faces: list[tuple[int, ...]] = []
    for row in range(len(rows) - 1):
        for column in range(columns - 1):
            a = row * columns + column
            faces.append((a, a + 1, a + columns + 1, a + columns))
    shield = mesh_object("Guard_Shield", vertices, [tuple(reversed(face)) for face in faces],
                         materials["steel"], subsurf=1, thickness=0.06, bevel=0.018)
    rim = [
        (center_x - 0.24, 1.44, 0.39), (center_x - 0.19, 1.90, 0.38),
        (center_x, 1.98, 0.38), (center_x + 0.19, 1.90, 0.38),
        (center_x + 0.25, 1.44, 0.39), (center_x + 0.19, 1.14, 0.36),
        (center_x, 1.02, 0.33), (center_x - 0.19, 1.14, 0.36),
    ]
    rim_obj = add_curve("Guard_ShieldRim", rim, 0.028, materials["gold"])
    boss = add_sphere("Guard_ShieldBoss", (center_x, 1.48, 0.43), (0.11, 0.11, 0.06), materials["gold"])
    add_curve("Guard_ShieldGrip", [(center_x, 1.30, 0.21), (center_x, 1.53, 0.26)],
              0.033, materials["leather"])
    return shield, rim_obj if boss else shield


def make_spear(materials: dict[str, bpy.types.Material]) -> tuple[bpy.types.Object, bpy.types.Object, bpy.types.Object]:
    shaft = add_curve("Guard_SpearShaft", [(0.86, 0.10, 0.22), (0.84, 1.55, 0.20), (0.86, 3.05, 0.18)], 0.038, materials["leather"])
    collar = add_torus("Guard_SpearCollar", (0.86, 2.92, 0.18), 0.06, 0.018, materials["gold"], (math.pi / 2, 0, 0))
    vertices = [(0.86, 2.98, 0.18), (0.74, 3.22, 0.18), (0.86, 3.48, 0.18), (0.98, 3.22, 0.18)]
    blade = mesh_object("Guard_SpearBlade", vertices, [(2, 1, 0), (3, 2, 0)],
                        materials["steel_light"], thickness=0.045, bevel=0.012)
    return shaft, collar, blade


def add_limbs(materials: dict[str, bpy.types.Material]) -> None:
    for label, points in ARM_POINTS.items():
        shoulder, elbow, wrist, hand = points
        add_curve(f"Guard_UpperArm{label}", [shoulder, elbow], 0.14, materials["blue"])
        add_ico(f"Guard_Pauldron{label}", (shoulder[0] * 1.12, 2.01, 0.005),
                (0.21, 0.165, 0.19), materials["steel"])
        add_ico(f"Guard_Elbow{label}", elbow, (0.14, 0.13, 0.13), materials["steel"])
        add_curve(f"Guard_Forearm{label}", [elbow, wrist], 0.11, materials["blue"])
        add_ico(f"Guard_Glove{label}", hand, (0.11, 0.13, 0.10), materials["leather"])
    for label, points in LEG_POINTS.items():
        hip, knee, ankle = points
        add_curve(f"Guard_Thigh{label}", [hip, knee], 0.16, materials["blue"])
        add_ico(f"Guard_Knee{label}", knee, (0.15, 0.14, 0.14), materials["steel"])
        add_curve(f"Guard_Shin{label}", [knee, ankle], 0.12, materials["steel"])
        add_sphere(f"Guard_Boot{label}", (ankle[0], 0.10, ankle[2] + 0.03), (0.17, 0.12, 0.22), materials["leather"])


def add_markers() -> None:
    for name, kind, loc in (("Guard_front", "front", (0, 2.4, 1.4)), ("Guard_back", "back", (0, 2.4, -1.4)), ("Guard_pivot", "pivot", (0, 0, 0))):
        obj = bpy.data.objects.new(name, None)
        bpy.context.collection.objects.link(obj)
        obj.location = loc
        obj.empty_display_type = "PLAIN_AXES"
        obj.empty_display_size = 0.2
        obj["asset3d_id"] = "Guard"
        obj["asset3d_marker"] = kind


def add_rig() -> bpy.types.Object:
    parts = [obj for obj in bpy.context.scene.objects if obj.type == "MESH" and obj.get("asset3d_id") == "Guard"]
    data = bpy.data.armatures.new("Guard_RigData")
    rig = bpy.data.objects.new("Guard_Rig", data)
    bpy.context.collection.objects.link(rig)
    rig["asset3d_id"] = "Guard"
    rig["asset3d_rig"] = True
    rig.rotation_euler = BASE_ROT
    bpy.ops.object.select_all(action="DESELECT")
    bpy.context.view_layer.objects.active = rig
    rig.select_set(True)
    bpy.ops.object.mode_set(mode="EDIT")
    bones = data.edit_bones
    root = bones.new("root"); root.head = (0, 0, 0); root.tail = (0, 1, 0)
    spine = bones.new("spine"); spine.head = (0, 0.9, 0); spine.tail = (0, 2.1, 0); spine.parent = root
    neck = bones.new("neck"); neck.head = (0, 2.1, 0); neck.tail = (0, 2.45, 0); neck.parent = spine
    head = bones.new("head"); head.head = (0, 2.45, 0); head.tail = (0, 2.95, 0); head.parent = neck
    for label, points in ARM_POINTS.items():
        shoulder, elbow, wrist, hand = points
        upper = bones.new(f"upper_arm.{label}"); upper.head, upper.tail = shoulder, elbow; upper.parent = spine
        fore = bones.new(f"forearm.{label}"); fore.head, fore.tail = elbow, wrist; fore.parent = upper; fore.use_connect = True
        hand_bone = bones.new(f"hand.{label}"); hand_bone.head, hand_bone.tail = wrist, hand; hand_bone.parent = fore; hand_bone.use_connect = True
    for label, points in LEG_POINTS.items():
        hip, knee, ankle = points
        thigh = bones.new(f"thigh.{label}"); thigh.head, thigh.tail = hip, knee; thigh.parent = root
        shin = bones.new(f"shin.{label}"); shin.head, shin.tail = knee, ankle; shin.parent = thigh; shin.use_connect = True
    weapon = bones.new("weapon.R"); weapon.head = ARM_POINTS["R"][3]; weapon.tail = (0.86, 3.3, 0.18); weapon.parent = bones["hand.R"]
    shield = bones.new("shield.L"); shield.head = ARM_POINTS["L"][3]; shield.tail = (-0.62, 1.5, 0.4); shield.parent = bones["hand.L"]
    bpy.ops.object.mode_set(mode="OBJECT")
    bpy.ops.object.select_all(action="DESELECT")
    rig.select_set(True)
    for obj in parts:
        obj.select_set(True)
    bpy.context.view_layer.objects.active = rig
    bpy.ops.object.parent_set(type="ARMATURE_AUTO")
    for obj in parts:
        obj.matrix_parent_inverse = Matrix.Identity(4)
    # Explicit weights keep the authored equipment attached to its action bones.
    for obj in parts:
        name = obj.name
        for group in obj.vertex_groups:
            group.remove(list(range(len(obj.data.vertices))))
        if name.startswith(("Guard_UpperArm", "Guard_Pauldron")):
            group_name = "upper_arm.L" if name.endswith("L") else "upper_arm.R"
        elif name.startswith("Guard_Elbow"):
            group_name = "upper_arm.L" if name.endswith("L") else "upper_arm.R"
        elif name.startswith("Guard_Forearm"):
            group_name = "forearm.L" if name.endswith("L") else "forearm.R"
        elif name.startswith("Guard_Glove"):
            group_name = "hand.L" if name.endswith("L") else "hand.R"
        elif name.startswith("Guard_Thigh") or name.startswith("Guard_Knee"):
            group_name = "thigh.L" if name.endswith("L") else "thigh.R"
        elif name.startswith("Guard_Shin") or name.startswith("Guard_Boot"):
            group_name = "shin.L" if name.endswith("L") else "shin.R"
        elif name.startswith("Guard_Spear"):
            group_name = "weapon.R"
        elif name.startswith("Guard_Shield"):
            group_name = "shield.L"
        elif name.startswith(("Guard_Helmet", "Guard_Face", "Guard_Eye", "Guard_Cheek", "Guard_NoseGuard", "Guard_Brow", "Guard_Nose", "Guard_Mouth")):
            group_name = "head"
        elif name == "Guard_Neck":
            group_name = "neck"
        else:
            group_name = "spine"
        group = obj.vertex_groups.get(group_name) or obj.vertex_groups.new(name=group_name)
        group.add(list(range(len(obj.data.vertices))), 1.0, "REPLACE")
    for marker in (bpy.data.objects.get("Guard_front"), bpy.data.objects.get("Guard_back"), bpy.data.objects.get("Guard_pivot")):
        if marker is not None:
            marker.parent = rig
            marker.matrix_parent_inverse = Matrix.Identity(4)
    return rig


def add_turntable(rig: bpy.types.Object) -> bpy.types.Object:
    turntable = bpy.data.objects.new("Guard_Turntable", None)
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


def apply_pose(rig: bpy.types.Object, anim: str, frame: int) -> None:
    reset_pose(rig)
    if anim == "idle":
        bob, spine_roll, head_pitch = 0.0, -0.02, 0.0
        arms = {
            "upper_arm.L": (0.10, 0.08, -0.18), "forearm.L": (0.18, 0.02, 0.22),
            "upper_arm.R": (0.02, 0.04, 0.10), "forearm.R": (0.16, 0.0, -0.16),
            "thigh.L": (0.0, 0.0, -0.02), "shin.L": (0.0, 0.0, 0.03),
            "thigh.R": (0.0, 0.0, 0.02), "shin.R": (0.0, 0.0, -0.03),
        }
    elif anim == "walk":
        sign = 1 if frame == 0 else -1
        bob, spine_roll, head_pitch = (0.025 if frame == 0 else 0.0), 0.02 * sign, -0.02 * sign
        arms = {
            "upper_arm.L": (0.04, 0.03, -0.14), "forearm.L": (0.16, 0.0, 0.16),
            "upper_arm.R": (0.0, 0.03, 0.10), "forearm.R": (0.16, 0.0, -0.12),
            "thigh.L": (0.0, 0.0, 0.28 * sign), "shin.L": (0.0, 0.0, -0.18 * sign),
            "thigh.R": (0.0, 0.0, -0.28 * sign), "shin.R": (0.0, 0.0, 0.18 * sign),
        }
    elif anim == "attack":
        if frame == 0:
            bob, spine_roll, head_pitch = 0.0, 0.10, -0.06
            arms = {"upper_arm.L": (0.20, 0.10, -0.44), "forearm.L": (0.34, 0.0, 0.38), "upper_arm.R": (0.16, -0.08, 0.48), "forearm.R": (0.30, 0.0, -0.48), "thigh.L": (0.0, 0.0, -0.08), "thigh.R": (0.0, 0.0, 0.08)}
        elif frame == 1:
            bob, spine_roll, head_pitch = -0.01, -0.06, 0.08
            arms = {"upper_arm.L": (0.16, 0.14, -0.22), "forearm.L": (0.28, 0.0, 0.24), "upper_arm.R": (0.06, 0.16, -0.42), "forearm.R": (0.10, 0.0, -0.12), "thigh.L": (0.0, 0.0, 0.16), "thigh.R": (0.0, 0.0, -0.16)}
        else:
            bob, spine_roll, head_pitch = 0.0, -0.03, 0.03
            arms = {"upper_arm.L": (0.10, 0.10, -0.18), "forearm.L": (0.22, 0.0, 0.20), "upper_arm.R": (0.04, 0.10, -0.20), "forearm.R": (0.14, 0.0, -0.20), "thigh.L": (0.0, 0.0, 0.05), "thigh.R": (0.0, 0.0, -0.05)}
    elif anim == "bash":
        bob, spine_roll, head_pitch = 0.0, -0.08, 0.06
        arms = {"upper_arm.L": (0.22, 0.20, -0.18), "forearm.L": (0.12, 0.0, 0.18), "upper_arm.R": (0.10, 0.08, 0.18), "forearm.R": (0.22, 0.0, -0.18), "thigh.L": (0.0, 0.0, 0.10), "thigh.R": (0.0, 0.0, -0.10)}
    else:
        bob, spine_roll, head_pitch = -0.015, 0.14, -0.12
        arms = {"upper_arm.L": (0.20, 0.10, 0.48), "forearm.L": (0.36, 0.0, 0.36), "upper_arm.R": (0.10, 0.06, -0.34), "forearm.R": (0.30, 0.0, -0.44), "thigh.L": (0.0, 0.0, 0.10), "thigh.R": (0.0, 0.0, -0.10)}
    rig.location = (0, bob, 0)
    rig.rotation_euler = BASE_ROT
    rig.pose.bones["spine"].rotation_euler.z = spine_roll
    rig.pose.bones["head"].rotation_euler.z = head_pitch
    for name, angles in arms.items():
        rig.pose.bones[name].rotation_euler = angles
    bpy.context.view_layer.update()


def build_action_library(rig: bpy.types.Object) -> None:
    rig.animation_data_create()
    idle_action = None
    clips = [
        ("guard_spear_idle_guard", "idle", 0),
        ("guard_spear_walk_a", "walk", 0),
        ("guard_spear_walk_b", "walk", 1),
        ("guard_spear_thrust_windup", "attack", 0),
        ("guard_spear_thrust", "attack", 1),
        ("guard_spear_recover", "attack", 2),
        ("guard_shield_bash", "bash", 0),
        ("guard_shield_hit_block", "hit", 0),
    ]
    for name, anim, frame in clips:
        action = bpy.data.actions.new(name)
        action.use_fake_user = True
        rig.animation_data.action = action
        if name == ACTION_PROFILE["default_action"]:
            idle_action = action
        for key_frame in (0, 1):
            apply_pose(rig, anim, frame)
            for bone in rig.pose.bones:
                bone.keyframe_insert(data_path="rotation_euler", frame=key_frame, group=bone.name)
            rig.keyframe_insert(data_path="location", frame=key_frame, group="Root")
            rig.keyframe_insert(data_path="rotation_euler", frame=key_frame, group="Root")
    rig.animation_data.action = idle_action
    bpy.context.scene.frame_set(0)


def render_frames(rig: bpy.types.Object, turntable: bpy.types.Object) -> None:
    scene = bpy.context.scene
    directions = DIRECTIONS[:1] if PROBE else DIRECTIONS
    # Action evaluation otherwise overwrites the manual pose on each render.
    saved_action = rig.animation_data.action
    rig.animation_data.action = None
    try:
        for animation, count in ANIMATIONS.items():
            for direction, angle in directions:
                target = ROOT / "render/Guard" / animation / direction
                target.mkdir(parents=True, exist_ok=True)
                for frame in range(count):
                    turntable.rotation_euler = (0, 0, angle)
                    apply_pose(rig, animation, frame)
                    scene.render.filepath = str(target / f"{frame:03d}.png")
                    bpy.ops.render.render(write_still=True)
    finally:
        rig.animation_data.action = saved_action


def main() -> None:
    bpy.ops.wm.read_factory_settings(use_empty=True)
    base.add_scene()
    materials = {
        "blue": base.material("Guard_Blue", (0.16, 0.28, 0.48, 1), 0.18, 0.46),
        "steel": base.material("Guard_Steel", (0.46, 0.55, 0.62, 1), 0.68, 0.30),
        "steel_light": base.material("Guard_SteelLight", (0.68, 0.76, 0.80, 1), 0.72, 0.24),
        "leather": base.material("Guard_Leather", (0.24, 0.12, 0.06, 1), 0.05, 0.62),
        "gold": base.material("Guard_Gold", (0.78, 0.52, 0.12, 1), 0.72, 0.28),
        "skin": base.material("Guard_Skin", (0.62, 0.40, 0.26, 1), 0.0, 0.58),
        "dark": base.material("Guard_Dark", (0.035, 0.045, 0.06, 1), 0.0, 0.72),
    }
    make_torso(materials)
    make_helmet(materials)
    add_limbs(materials)
    make_armor(materials)
    shield, _ = make_shield(materials)
    make_spear(materials)
    add_markers()
    rig = add_rig()
    turntable = add_turntable(rig)
    build_action_library(rig)
    blend_path = ROOT / "blender/Guard.blend"
    blend_path.parent.mkdir(parents=True, exist_ok=True)
    bpy.ops.wm.save_as_mainfile(filepath=str(blend_path))
    render_frames(rig, turntable)
    manifest = {
        "version": 1,
        "assets": [{
            "id": "Guard", "kind": "actor", "source": "source/Guard.glb", "render_root": "render/Guard",
            "cell_size": {"width": CELL[0], "height": CELL[1]}, "directions": [n for n, _ in DIRECTIONS],
            "animations": ANIMATIONS, "action_profile": ACTION_PROFILE, "uvs": True, "materials": True,
            "rig": True, "pivot": "feet_center",
            "metadata": {"source_kind": "blender_5.1_guard_spear", "front_axis": "-Y"},
        }],
    }
    (ROOT / "guard_manifest.json").write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    print("BLENDER_GUARD_V1", blend_path)


if __name__ == "__main__":
    main()
