"""Legacy rounded humanoid Lich proof; superseded by v5.

This pass fixed the primitive-cone construction but retained a round,
cartoonish face and generic idle-like action lanes. The current source is
``create_blender_lich_v5.py``. Its output is preserved under
``lich-blender-5.1-v4-round-rejected/``.
"""

from __future__ import annotations

import json
import math
import sys
from pathlib import Path

import bmesh
import bpy  # type: ignore
from mathutils import Matrix, Vector  # type: ignore
from character_surface import cloth_radius

_TAIL = sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else []
_ROOT_INDEX = _TAIL.index("--output-root") + 1 if "--output-root" in _TAIL else None
ROOT = Path(_TAIL[_ROOT_INDEX]).resolve() if _ROOT_INDEX is not None else Path.cwd()
PROBE = "--probe" in _TAIL
CELL = (128, 160)
BASE_ROT = (math.pi / 2, 0, 0)  # model profile is authored along local Y
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


def material(name: str, color: tuple[float, float, float, float], metallic: float = 0.0, roughness: float = 0.55) -> bpy.types.Material:
    mat = bpy.data.materials.new(name)
    mat.use_nodes = True
    bsdf = mat.node_tree.nodes.get("Principled BSDF")
    bsdf.inputs["Base Color"].default_value = color
    bsdf.inputs["Metallic"].default_value = metallic
    bsdf.inputs["Roughness"].default_value = roughness
    from character_surface import tune_material
    tune_material(bsdf, name, metallic, roughness)
    return mat


def tag(obj: bpy.types.Object, name: str, mat: bpy.types.Material | None = None) -> bpy.types.Object:
    obj.name = name
    obj["asset3d_id"] = "Lich"
    if mat is not None and hasattr(obj.data, "materials"):
        names = {slot.name for slot in obj.data.materials}
        if mat.name not in names:
            obj.data.materials.append(mat)
    return obj


def ensure_uv(obj: bpy.types.Object) -> None:
    if obj.type != "MESH":
        return
    if not obj.data.uv_layers:
        layer = obj.data.uv_layers.new(name="UVMap")
        for poly in obj.data.polygons:
            for loop_index in poly.loop_indices:
                co = obj.data.vertices[obj.data.loops[loop_index].vertex_index].co
                layer.data[loop_index].uv = (co.x * 0.22 + 0.5, co.y * 0.22 + 0.5)
    for poly in obj.data.polygons:
        poly.use_smooth = True


def apply_modifier(obj: bpy.types.Object, modifier: bpy.types.Modifier) -> None:
    bpy.ops.object.select_all(action="DESELECT")
    obj.select_set(True)
    bpy.context.view_layer.objects.active = obj
    bpy.ops.object.modifier_apply(modifier=modifier.name)


def make_profile_body(materials: dict[str, bpy.types.Material]) -> bpy.types.Object:
    # (height along local Y, horizontal radius, depth radius). The narrow
    # waist and deliberate shoulder shelf make the robe read as a body.
    profile = [
        (0.04, 0.64, 0.48),
        (0.16, 0.62, 0.46),
        (0.48, 0.55, 0.40),
        (0.82, 0.45, 0.33),
        (1.10, 0.35, 0.28),
        (1.25, 0.30, 0.25),
        (1.48, 0.35, 0.26),
        (1.78, 0.43, 0.28),
        (2.02, 0.46, 0.28),
        (2.16, 0.37, 0.24),
        (2.25, 0.19, 0.17),
    ]
    segments = 32
    vertices: list[tuple[float, float, float]] = []
    for y, rx, rz in profile:
        for j in range(segments):
            angle = math.tau * j / segments
            fold = cloth_radius(angle, y, profile[0][0], profile[-1][0], .055)
            vertices.append((rx * math.cos(angle) * fold, y, rz * math.sin(angle) * fold))
    faces: list[tuple[int, ...]] = []
    for ring in range(len(profile) - 1):
        for j in range(segments):
            a = ring * segments + j
            b = ring * segments + (j + 1) % segments
            c = (ring + 1) * segments + (j + 1) % segments
            d = (ring + 1) * segments + j
            faces.append((a, b, c, d))
    bottom_center = len(vertices); vertices.append((0, profile[0][0], 0))
    top_center = len(vertices); vertices.append((0, profile[-1][0], 0))
    for j in range(segments):
        faces.append((bottom_center, (j + 1) % segments, j))
        start = (len(profile) - 1) * segments
        faces.append((top_center, start + j, start + (j + 1) % segments))
    mesh = bpy.data.meshes.new("Lich_RobeMesh")
    mesh.from_pydata(vertices, [], faces)
    mesh.validate()
    bm = bmesh.new()
    bm.from_mesh(mesh)
    bmesh.ops.recalc_face_normals(bm, faces=list(bm.faces))
    bm.to_mesh(mesh)
    bm.free()
    obj = bpy.data.objects.new("Lich_Mesh", mesh)
    bpy.context.collection.objects.link(obj)
    obj.data.materials.append(materials["violet"])
    obj.data.materials.append(materials["violet_light"])
    for poly in obj.data.polygons:
        poly.material_index = 1 if poly.center.y > 1.92 else 0
    tag(obj, "Lich_Mesh")
    ensure_uv(obj)
    sub = obj.modifiers.new("RobeSilhouette", "SUBSURF")
    sub.levels = 2
    sub.render_levels = 2
    apply_modifier(obj, sub)
    ensure_uv(obj)
    return obj


def add_uv_sphere(name: str, loc: tuple[float, float, float], scale: tuple[float, float, float], mat: bpy.types.Material) -> bpy.types.Object:
    bpy.ops.mesh.primitive_uv_sphere_add(segments=24, ring_count=16, location=loc)
    obj = bpy.context.object
    obj.scale = scale
    bpy.ops.object.select_all(action="DESELECT")
    obj.select_set(True)
    bpy.context.view_layer.objects.active = obj
    bpy.ops.object.transform_apply(location=False, rotation=False, scale=True)
    tag(obj, name, mat)
    ensure_uv(obj)
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
    ensure_uv(obj)
    return obj


def add_torus(name: str, loc: tuple[float, float, float], major: float, minor: float, mat: bpy.types.Material, rotation: tuple[float, float, float] = (0, 0, 0)) -> bpy.types.Object:
    bpy.ops.mesh.primitive_torus_add(major_radius=major, minor_radius=minor, major_segments=32, minor_segments=10, location=loc, rotation=rotation)
    obj = tag(bpy.context.object, name, mat)
    ensure_uv(obj)
    return obj


def add_curve_tube(name: str, points: list[tuple[float, float, float]], radius: float, mat: bpy.types.Material) -> bpy.types.Object:
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
    ensure_uv(obj)
    return obj


def add_markers() -> None:
    for name, kind, loc in (("Lich_front", "front", (0, 2.4, 1.4)), ("Lich_back", "back", (0, 2.4, -1.4)), ("Lich_pivot", "pivot", (0, 0, 0))):
        obj = bpy.data.objects.new(name, None)
        bpy.context.collection.objects.link(obj)
        obj.location = loc
        obj.empty_display_type = "PLAIN_AXES"
        obj.empty_display_size = 0.2
        obj["asset3d_id"] = "Lich"
        obj["asset3d_marker"] = kind


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
    spine = bones.new("spine"); spine.head = (0, 1, 0); spine.tail = (0, 2.2, 0); spine.parent = root
    neck = bones.new("neck"); neck.head = (0, 2.2, 0); neck.tail = (0, 2.65, 0); neck.parent = spine
    head = bones.new("head"); head.head = (0, 2.65, 0); head.tail = (0, 3.25, 0); head.parent = neck
    for name, x in (("arm.L", -0.43), ("arm.R", 0.43)):
        bone = bones.new(name); bone.head = (x, 2.05, 0); bone.tail = (x * 1.55, 1.35, 0.08); bone.parent = spine
    for name, x in (("leg.L", -0.28), ("leg.R", 0.28)):
        bone = bones.new(name); bone.head = (x, 1.2, 0); bone.tail = (x, 0.08, 0.05); bone.parent = root
    bpy.ops.object.mode_set(mode="OBJECT")
    bpy.ops.object.select_all(action="DESELECT")
    rig.select_set(True)
    for obj in parts:
        obj.select_set(True)
    bpy.context.view_layer.objects.active = rig
    bpy.ops.object.parent_set(type="ARMATURE_AUTO")
    # Blender's parent operator preserves world transforms. The prototype
    # wants the rig transform to drive every direction, so clear that inverse.
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
    rig.location = (0, 0, 0)
    return turntable




def add_animation(rig: bpy.types.Object) -> None:
    rig.animation_data_create()
    rig.animation_data.action = bpy.data.actions.new("Lich_Idle")
    for frame, bob, sway in ((1, 0, 0), (5, 0.025, 0.018), (9, 0, 0)):
        rig.location = (0, 0, bob)
        rig.rotation_euler = (BASE_ROT[0], sway, 0)
        rig.keyframe_insert(data_path="location", frame=frame, group="Lich")
        rig.keyframe_insert(data_path="rotation_euler", frame=frame, group="Lich")
    bpy.context.scene.frame_start = 1
    bpy.context.scene.frame_end = 9


def add_scene() -> bpy.types.Scene:
    scene = bpy.context.scene
    scene.render.engine = "BLENDER_EEVEE"
    scene.render.resolution_x, scene.render.resolution_y = CELL
    scene.render.resolution_percentage = 100
    scene.render.image_settings.file_format = "PNG"
    scene.render.image_settings.color_mode = "RGBA"
    scene.render.film_transparent = True
    if scene.world is None:
        scene.world = bpy.data.worlds.new("LichWorld")
    scene.world.color = (0.006, 0.008, 0.014)
    for name, loc, energy, color in (("Key", (4.5, -4.5, 7.5), 1250, (1.0, 0.78, 0.58)), ("Fill", (-4, -2, 3.8), 700, (0.34, 0.43, 0.85)), ("Rim", (0, 4.5, 6.5), 1500, (0.48, 0.72, 1.0))):
        data = bpy.data.lights.new(f"Lich_{name}", type="AREA"); data.energy = energy; data.color = color
        light = bpy.data.objects.new(f"Lich_{name}", data); bpy.context.collection.objects.link(light); light.location = loc
        target = Vector((0, 0, 1.8)); light.rotation_euler = (target - light.location).to_track_quat("-Z", "Y").to_euler()
    data = bpy.data.cameras.new("Lich_Camera")
    camera = bpy.data.objects.new("Lich_Camera", data)
    bpy.context.collection.objects.link(camera)
    camera.location = (0, -8, 3.1); data.type = "ORTHO"; data.ortho_scale = 4.8
    target = Vector((0, 0, 1.85)); camera.rotation_euler = (target - camera.location).to_track_quat("-Z", "Y").to_euler(); scene.camera = camera
    return scene


def render_frames(rig: bpy.types.Object, turntable: bpy.types.Object) -> None:
    scene = bpy.context.scene
    directions = DIRECTIONS[:4] if PROBE else DIRECTIONS
    animations = {"idle": 1} if PROBE else ANIMATIONS
    for animation, count in animations.items():
        for direction, angle in directions:
            target = ROOT / "render/Lich" / animation / direction
            target.mkdir(parents=True, exist_ok=True)
            for frame in range(count):
                turntable.rotation_euler = (0, 0, angle)
                rig.rotation_euler = (BASE_ROT[0], 0, 0)
                bpy.context.view_layer.update()
                rig.location = (0, 0, 0)
                rig.scale = (1, 1, 1)
                if animation == "walk":
                    rig.location.x = -0.08 if frame == 0 else 0.08
                    rig.rotation_euler.y = -0.035 if frame == 0 else 0.035
                elif animation == "attack":
                    rig.scale = (1 + frame * 0.012,) * 3
                elif animation == "hit":
                    rig.rotation_euler.y = -0.08
                    rig.location.x = -0.06
                scene.render.filepath = str(target / f"{frame:03d}.png")
                bpy.ops.render.render(write_still=True)


def main() -> None:
    bpy.ops.wm.read_factory_settings(use_empty=True)
    add_scene()
    materials = {
        "violet": material("Lich_Robe", (0.25, 0.07, 0.34, 1), 0.05, 0.5),
        "violet_light": material("Lich_Hood", (0.43, 0.16, 0.53, 1), 0.04, 0.52),
        "bone": material("Lich_Bone", (0.76, 0.70, 0.56, 1), 0.08, 0.46),
        "gold": material("Lich_Gold", (0.78, 0.48, 0.10, 1), 0.78, 0.28),
        "teal": material("Lich_Teal", (0.08, 0.64, 0.62, 1), 0.18, 0.3),
        "black": material("Lich_Black", (0.008, 0.012, 0.018, 1), 0, 0.7),
    }
    body = make_profile_body(materials)
    add_uv_sphere("Lich_Mantle", (0, 1.98, -0.02), (0.52, 0.22, 0.34), materials["violet_light"])
    add_uv_sphere("Lich_Hood", (0, 2.60, -0.12), (0.40, 0.45, 0.31), materials["violet_light"])
    add_uv_sphere("Lich_Face", (0, 2.62, 0.22), (0.25, 0.28, 0.09), materials["bone"])
    add_uv_sphere("Lich_CheekL", (-0.14, 2.55, 0.28), (0.07, 0.08, 0.035), materials["bone"])
    add_uv_sphere("Lich_CheekR", (0.14, 2.55, 0.28), (0.07, 0.08, 0.035), materials["bone"])
    add_uv_sphere("Lich_EyeSocketL", (-0.10, 2.72, 0.30), (0.085, 0.06, 0.035), materials["black"])
    add_uv_sphere("Lich_EyeSocketR", (0.10, 2.72, 0.30), (0.085, 0.06, 0.035), materials["black"])
    add_uv_sphere("Lich_EyeL", (-0.10, 2.72, 0.335), (0.038, 0.038, 0.025), materials["teal"])
    add_uv_sphere("Lich_EyeR", (0.10, 2.72, 0.335), (0.038, 0.038, 0.025), materials["teal"])
    add_uv_sphere("Lich_Nose", (0, 2.60, 0.35), (0.05, 0.10, 0.045), materials["bone"])
    add_curve_tube("Lich_Mouth", [(-0.09, 2.49, 0.31), (0, 2.47, 0.33), (0.09, 2.49, 0.31)], 0.012, materials["black"])
    add_torus("Lich_HoodRim", (0, 2.34, -0.01), 0.32, 0.055, materials["violet_light"], (math.pi / 2, 0, 0))
    add_torus("Lich_Belt", (0, 1.24, 0), 0.35, 0.035, materials["gold"], (math.pi / 2, 0, 0))
    for side in (-1, 1):
        label = "L" if side < 0 else "R"
        add_curve_tube(f"Lich_Arm{label}", [(side * 0.40, 2.03, 0), (side * 0.67, 1.78, 0.03), (side * 0.70, 1.52, 0.08), (side * 0.60, 1.27, 0.12)], 0.13, materials["violet"])
        add_uv_sphere(f"Lich_Shoulder{label}", (side * 0.40, 2.04, 0), (0.23, 0.20, 0.23), materials["violet_light"])
        add_uv_sphere(f"Lich_Hand{label}", (side * 0.60, 1.25, 0.12), (0.12, 0.15, 0.11), materials["bone"])
    for i in range(5):
        add_ico(f"Lich_Crown{i}", (-0.28 + i * 0.14, 3.10 + (0.05 if i % 2 == 0 else 0), 0), (0.07, 0.18 if i % 2 == 0 else 0.13, 0.07), materials["gold"])
    add_curve_tube("Lich_Staff", [(0.82, 0.10, 0.22), (0.78, 1.25, 0.20), (0.84, 2.25, 0.23), (0.78, 3.18, 0.22)], 0.045, materials["gold"])
    add_uv_sphere("Lich_StaffOrb", (0.78, 3.30, 0.22), (0.16, 0.16, 0.16), materials["teal"])
    for x in (-0.22, 0, 0.22):
        add_curve_tube(f"Lich_RobeFold{x}", [(x, 0.22, 0.46), (x * 0.8, 0.70, 0.40), (x * 0.45, 1.18, 0.30)], 0.012, materials["violet_light"])
    add_markers()
    rig = add_rig()
    turntable = add_turntable(rig)
    add_animation(rig)
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
            "metadata": {"source_kind": "blender_5.1_custom_humanoid", "front_axis": "-Y"},
        }],
    }
    (ROOT / "lich_manifest.json").write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    print("BLENDER_LICH_V4", blend_path)


if __name__ == "__main__":
    main()
