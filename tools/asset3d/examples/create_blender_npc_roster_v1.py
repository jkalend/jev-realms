"""Author the remaining canonical non-player NPC roster in Blender 5.1.

The Lich and Guard are already complete references. This data-driven generator
covers the other 24 runtime Archetype NPCs (player classes and BroodHole are
excluded), giving each a distinct silhouette family, equipment, default idle
and weapon-specific action profile while sharing the Blender/manifest seam.

Run one asset:
    F:/Blender/blender.exe --background --python tools/asset3d/examples/create_blender_npc_roster_v1.py -- \
      --npc Commoner --output-root docs/gfx/proto/visual-v2/npc-roster-v1/Commoner
"""

from __future__ import annotations

import json
import math
import sys
from pathlib import Path

import bmesh
import bpy  # type: ignore
from mathutils import Matrix, Vector  # type: ignore

sys.path.insert(0, str(Path(__file__).resolve().parent))
import create_blender_lich_v4 as base  # type: ignore
from character_surface import cloth_radius

PROFILE_PATH = Path(__file__).with_name("npc_roster_profiles.json")
PROFILES = json.loads(PROFILE_PATH.read_text(encoding="utf-8"))
TAIL = sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else []
ROOT = Path(TAIL[TAIL.index("--output-root") + 1]).resolve()
NPC_NAME = TAIL[TAIL.index("--npc") + 1]
PROBE = "--probe" in TAIL
STUDY_V2 = "--visual-v2" in TAIL
STUDY_V3 = "--visual-v3" in TAIL
STUDY_ANY = STUDY_V2 or STUDY_V3
BASE_ROT = (math.pi / 2, 0, 0)
CELL = (256, 320) if STUDY_ANY else (128, 160)
DIRECTIONS = base.DIRECTIONS
BEAST_FORMS = {"wolf", "bear", "rat", "stag"}
NON_HUMANOID = BEAST_FORMS | {"wisp"}


def rgba(value: str) -> tuple[float, float, float, float]:
    """Hex palette entry -> linear RGBA. Blender base colours are linear, so a raw
    sRGB hex would render washed out and desaturated under the key light."""
    value = value.lstrip("#")
    channels = [int(value[i : i + 2], 16) / 255 for i in (0, 2, 4)]
    linear = [c / 12.92 if c <= 0.04045 else ((c + 0.055) / 1.055) ** 2.4 for c in channels]
    return tuple(linear) + (1.0,)  # type: ignore[return-value]


def make_materials(npc: str, cfg: dict) -> dict[str, bpy.types.Material]:
    p = cfg["palette"]
    cloth = rgba(p["cloth"])
    return {
        "skin": base.material(f"{npc}_Skin", rgba(p["skin"]), 0.0, 0.58),
        "cloth": base.material(f"{npc}_Cloth", cloth, 0.04, 0.56),
        "cloth_shade": base.material(f"{npc}_ClothShade", tuple(c * 0.66 for c in cloth[:3]) + (1.0,), 0.02, 0.70),
        "accent": base.material(f"{npc}_Accent", rgba(p["accent"]), 0.5, 0.36),
        "dark": base.material(f"{npc}_Dark", rgba("#0A0C10"), 0.0, 0.76),
        "bone": base.material(f"{npc}_Bone", rgba("#C9C2A0"), 0.02, 0.54),
        "steel": base.material(f"{npc}_Steel", rgba("#8A97A3"), 0.5, 0.46),
        "glow": base.material(f"{npc}_Glow", rgba("#3FC0B0"), 0.1, 0.28),
    }


def tune_scene() -> None:
    """Roster tone. The Lich/Guard hero scene is lit for one big silhouette; 24 NPCs
    sharing a contact sheet need a darker key range or every read goes high-key.
    EEVEE Next ray tracing supplies the contact occlusion that separates limbs."""
    scene = bpy.context.scene
    for light in scene.objects:
        if light.type == "LIGHT": light.data.energy *= 0.52
    scene.world.color = (0.004, 0.005, 0.010)
    eevee = scene.eevee
    if hasattr(eevee, "use_raytracing"): eevee.use_raytracing = True
    if hasattr(eevee, "use_shadows"): eevee.use_shadows = True
    scene.view_settings.exposure = 0.15


def tag(obj: bpy.types.Object, name: str, npc: str, mat: bpy.types.Material | None = None) -> bpy.types.Object:
    obj.name = name
    obj["asset3d_id"] = npc
    if mat is not None and hasattr(obj.data, "materials") and mat.name not in {slot.name for slot in obj.data.materials}:
        obj.data.materials.append(mat)
    return obj


def mesh_object(name: str, vertices: list[tuple[float, float, float]], faces: list[tuple[int, ...]], mat: bpy.types.Material, npc: str, subsurf: int = 0, thickness: float = 0.0, bevel: float = 0.0) -> bpy.types.Object:
    mesh = bpy.data.meshes.new(f"{name}Mesh")
    mesh.from_pydata(vertices, [], faces)
    mesh.validate()
    bm = bmesh.new(); bm.from_mesh(mesh); bmesh.ops.recalc_face_normals(bm, faces=list(bm.faces)); bm.to_mesh(mesh); bm.free()
    obj = bpy.data.objects.new(name, mesh); bpy.context.collection.objects.link(obj); tag(obj, name, npc, mat)
    if subsurf:
        mod = obj.modifiers.new("SoftSurface", "SUBSURF"); mod.levels = subsurf; mod.render_levels = subsurf; base.apply_modifier(obj, mod)
    if thickness:
        mod = obj.modifiers.new("Thickness", "SOLIDIFY"); mod.thickness = thickness; mod.offset = 0.0; base.apply_modifier(obj, mod)
    if bevel:
        mod = obj.modifiers.new("Bevel", "BEVEL"); mod.width = bevel; mod.segments = 2; base.apply_modifier(obj, mod)
    base.ensure_uv(obj)
    return obj


def add_sphere(name: str, loc: tuple[float, float, float], scale: tuple[float, float, float], mat: bpy.types.Material, npc: str) -> bpy.types.Object:
    bpy.ops.mesh.primitive_uv_sphere_add(segments=18, ring_count=12, location=loc); obj = bpy.context.object; obj.scale = scale
    bpy.ops.object.select_all(action="DESELECT"); obj.select_set(True); bpy.context.view_layer.objects.active = obj; bpy.ops.object.transform_apply(location=False, rotation=False, scale=True)
    tag(obj, name, npc, mat); base.ensure_uv(obj); return obj


def add_ico(name: str, loc: tuple[float, float, float], scale: tuple[float, float, float], mat: bpy.types.Material, npc: str) -> bpy.types.Object:
    bpy.ops.mesh.primitive_ico_sphere_add(subdivisions=2, radius=1, location=loc); obj = bpy.context.object; obj.scale = scale
    bpy.ops.object.select_all(action="DESELECT"); obj.select_set(True); bpy.context.view_layer.objects.active = obj; bpy.ops.object.transform_apply(location=False, rotation=False, scale=True)
    tag(obj, name, npc, mat); base.ensure_uv(obj); return obj


def add_curve(name: str, points: list[tuple[float, float, float]], radius: float, mat: bpy.types.Material, npc: str) -> bpy.types.Object:
    data = bpy.data.curves.new(name, type="CURVE"); data.dimensions = "3D"; data.resolution_u = 12; data.bevel_depth = radius; data.bevel_resolution = 3
    spline = data.splines.new("BEZIER"); spline.bezier_points.add(len(points) - 1)
    for point, co in zip(spline.bezier_points, points): point.co = co; point.handle_left_type = "AUTO"; point.handle_right_type = "AUTO"
    obj = bpy.data.objects.new(name, data); bpy.context.collection.objects.link(obj); data.materials.append(mat)
    bpy.ops.object.select_all(action="DESELECT"); obj.select_set(True); bpy.context.view_layer.objects.active = obj; bpy.ops.object.convert(target="MESH")
    obj = bpy.context.object; tag(obj, name, npc, mat); base.ensure_uv(obj); return obj


def add_torus(name: str, loc: tuple[float, float, float], major: float, minor: float, mat: bpy.types.Material, npc: str, rotation: tuple[float, float, float] = (0, 0, 0)) -> bpy.types.Object:
    bpy.ops.mesh.primitive_torus_add(major_radius=major, minor_radius=minor, major_segments=24, minor_segments=8, location=loc, rotation=rotation)
    obj = tag(bpy.context.object, name, npc, mat); base.ensure_uv(obj); return obj


def lathe(name: str, profile: list[tuple[float, float, float]], mat: bpy.types.Material, npc: str, segments: int = 24) -> bpy.types.Object:
    vertices: list[tuple[float, float, float]] = []
    textile = any(part in name for part in ("Torso", "Skirt", "WorkVest"))
    if textile:
        segments = max(32, segments)
    for y, rx, rz in profile:
        for j in range(segments):
            a = math.tau * j / segments
            fold = cloth_radius(a, y, profile[0][0], profile[-1][0]) if textile else 1
            vertices.append((rx * math.cos(a) * fold, y, rz * math.sin(a) * fold))
    faces: list[tuple[int, ...]] = []
    for row in range(len(profile) - 1):
        for j in range(segments): faces.append((row * segments + j, row * segments + (j + 1) % segments, (row + 1) * segments + (j + 1) % segments, (row + 1) * segments + j))
    bottom = len(vertices); vertices.append((0, profile[0][0], 0)); top = len(vertices); vertices.append((0, profile[-1][0], 0))
    for j in range(segments):
        faces.append((bottom, (j + 1) % segments, j)); start = (len(profile) - 1) * segments; faces.append((top, start + j, start + (j + 1) % segments))
    return mesh_object(name, vertices, faces, mat, npc, subsurf=1)


def add_face(npc: str, mats: dict[str, bpy.types.Material], s: float, y: float, z: float) -> None:
    """Face features sized to survive a 128x160 cell: sockets, irises, brow, nose, mouth."""
    for side in (-1, 1):
        add_ico(f"{npc}_EyeSocket{side_label(side)}", (side * 0.085 * s, y + 0.06 * s, z + 0.175 * s), (0.075 * s, 0.055 * s, 0.03 * s), mats["dark"], npc)
        add_ico(f"{npc}_Eye{side_label(side)}", (side * 0.085 * s, y + 0.06 * s, z + 0.20 * s), (0.030 * s, 0.026 * s, 0.022 * s), mats["bone"], npc)
        add_ico(f"{npc}_Cheek{side_label(side)}", (side * 0.165 * s, y - 0.04 * s, z + 0.14 * s), (0.07 * s, 0.07 * s, 0.05 * s), mats["skin"], npc)
        add_ico(f"{npc}_Ear{side_label(side)}", (side * 0.245 * s, y - 0.01 * s, z - 0.02 * s), (0.05 * s, 0.075 * s, 0.05 * s), mats["skin"], npc)
    add_ico(f"{npc}_Brow", (0, y + 0.155 * s, z + 0.175 * s), (0.20 * s, 0.04 * s, 0.035 * s), mats["dark"], npc)
    add_ico(f"{npc}_Nose", (0, y + 0.01 * s, z + 0.215 * s), (0.05 * s, 0.085 * s, 0.055 * s), mats["skin"], npc)
    add_curve(f"{npc}_Mouth", [(-0.09 * s, y - 0.11 * s, z + 0.185 * s), (0, y - 0.13 * s, z + 0.20 * s), (0.09 * s, y - 0.11 * s, z + 0.185 * s)], 0.022 * s, mats["dark"], npc)


def add_head_shell(npc: str, mats: dict[str, bpy.types.Material], s: float, y: float, z: float, mat: bpy.types.Material) -> None:
    """Headgear that sits behind the face plane so the face stays readable."""
    add_sphere(f"{npc}_HoodShell", (0, y + 0.02 * s, z - 0.14 * s), (0.30 * s, 0.28 * s, 0.25 * s), mat, npc)
    add_torus(f"{npc}_HoodBrim", (0, y + 0.04 * s, z + 0.16 * s), 0.22 * s, 0.05 * s, mat, npc)
    add_ico(f"{npc}_HoodPeak", (0, y + 0.10 * s, z - 0.28 * s), (0.13 * s, 0.17 * s, 0.15 * s), mat, npc)
    add_sphere(f"{npc}_Mantle", (0, y - 0.40 * s, z - 0.16 * s), (0.40 * s, 0.17 * s, 0.32 * s), mat, npc)


def add_head(npc: str, cfg: dict, mats: dict[str, bpy.types.Material], s: float) -> None:
    head = cfg["head"]
    y, z = 2.52 * s, 0.20 * s
    if head == "skull":
        add_ico(f"{npc}_Skull", (0, y, z), (0.28 * s, 0.32 * s, 0.24 * s), mats["bone"], npc)
        add_ico(f"{npc}_SkullJaw", (0, 2.34 * s, 0.24 * s), (0.20 * s, 0.15 * s, 0.15 * s), mats["bone"], npc)
        for side in (-1, 1):
            add_ico(f"{npc}_EyeSocket{side_label(side)}", (side * 0.11 * s, y + 0.07 * s, z + 0.18 * s), (0.095 * s, 0.075 * s, 0.03 * s), mats["dark"], npc)
            add_ico(f"{npc}_Eye{side_label(side)}", (side * 0.11 * s, y + 0.07 * s, z + 0.20 * s), (0.032 * s, 0.028 * s, 0.022 * s), mats["accent"], npc)
            add_ico(f"{npc}_Cheek{side_label(side)}", (side * 0.20 * s, y - 0.05 * s, z + 0.12 * s), (0.08 * s, 0.09 * s, 0.06 * s), mats["bone"], npc)
        add_ico(f"{npc}_Brow", (0, y + 0.18 * s, z + 0.19 * s), (0.22 * s, 0.045 * s, 0.035 * s), mats["bone"], npc)
        add_ico(f"{npc}_Nose", (0, y - 0.01 * s, z + 0.23 * s), (0.055 * s, 0.09 * s, 0.05 * s), mats["dark"], npc)
        add_curve(f"{npc}_Mouth", [(-0.10 * s, 2.40 * s, 0.30 * s), (0, 2.38 * s, 0.33 * s), (0.10 * s, 2.40 * s, 0.30 * s)], 0.024 * s, mats["dark"], npc)
        return
    if head == "wisp":
        add_sphere(f"{npc}_GlowHead", (0, y, z), (0.30 * s, 0.34 * s, 0.22 * s), mats["glow"], npc)
        for side in (-1, 1):
            add_ico(f"{npc}_EyeSocket{side_label(side)}", (side * 0.11 * s, y + 0.05 * s, z + 0.16 * s), (0.10 * s, 0.075 * s, 0.03 * s), mats["dark"], npc)
            add_ico(f"{npc}_Eye{side_label(side)}", (side * 0.11 * s, y + 0.05 * s, z + 0.19 * s), (0.036 * s, 0.030 * s, 0.022 * s), mats["bone"], npc)
        add_ico(f"{npc}_GlowMouth", (0, 2.36 * s, 0.30 * s), (0.13 * s, 0.06 * s, 0.03 * s), mats["dark"], npc)
        return
    add_sphere(f"{npc}_Face", (0, y, z), (0.24 * s, 0.27 * s, 0.20 * s), mats["skin"], npc)
    add_face(npc, mats, s, y, z)
    if head == "cap":
        add_sphere(f"{npc}_Cap", (0, y + 0.13 * s, z - 0.09 * s), (0.255 * s, 0.18 * s, 0.22 * s), mats["cloth"], npc)
        add_ico(f"{npc}_CapPeak", (0, y + 0.15 * s, z + 0.14 * s), (0.21 * s, 0.028 * s, 0.11 * s), mats["dark"], npc)
        add_torus(f"{npc}_CapBand", (0, y + 0.04 * s, z - 0.02 * s), 0.245 * s, 0.028 * s, mats["accent"], npc, (math.pi / 2, 0, 0))
    elif head == "hood":
        add_head_shell(npc, mats, s, y, z, mats["cloth"])
    elif head in {"helm", "horned"}:
        add_head_shell(npc, mats, s, y, z, mats["steel"])
        add_torus(f"{npc}_HelmetBrow", (0, y + 0.14 * s, z + 0.20 * s), 0.24 * s, 0.035 * s, mats["steel"], npc)
        add_ico(f"{npc}_NoseGuard", (0, y + 0.01 * s, z + 0.26 * s), (0.045 * s, 0.14 * s, 0.035 * s), mats["accent"], npc)
        for side in (-1, 1):
            add_ico(f"{npc}_CheekGuard{side_label(side)}", (side * 0.20 * s, y - 0.06 * s, z + 0.14 * s), (0.06 * s, 0.13 * s, 0.08 * s), mats["steel"], npc)
        add_ico(f"{npc}_HelmetCrest", (0, y + 0.32 * s, z - 0.10 * s), (0.05 * s, 0.12 * s, 0.24 * s), mats["accent"], npc)
        if head == "horned":
            for side in (-1, 1):
                add_curve(f"{npc}_Horn{side_label(side)}", [(side * 0.20 * s, 2.74 * s, -0.04 * s), (side * 0.42 * s, 3.00 * s, -0.08 * s), (side * 0.32 * s, 3.26 * s, -0.12 * s)], 0.055 * s, mats["bone"], npc)
    elif head == "crown":
        add_head_shell(npc, mats, s, y, z, mats["cloth"])
        add_torus(f"{npc}_CrownRing", (0, 2.80 * s, -0.06 * s), 0.24 * s, 0.035 * s, mats["accent"], npc, (math.pi / 2, 0, 0))
        for i in range(5):
            add_ico(f"{npc}_Crown{i}", (-0.22 * s + i * 0.11 * s, 2.94 * s, -0.06 * s), (0.045 * s, 0.16 * s, 0.045 * s), mats["accent"], npc)
    elif head == "mitre":
        add_head_shell(npc, mats, s, y, z, mats["cloth"])
        add_ico(f"{npc}_Mitre", (0, 2.86 * s, -0.10 * s), (0.21 * s, 0.30 * s, 0.20 * s), mats["cloth"], npc)
        add_torus(f"{npc}_MitreBand", (0, 2.68 * s, -0.06 * s), 0.25 * s, 0.035 * s, mats["accent"], npc, (math.pi / 2, 0, 0))
    elif head == "spines":
        add_sphere(f"{npc}_Hair", (0, y - 0.05 * s, z - 0.10 * s), (0.30 * s, 0.31 * s, 0.28 * s), mats["dark"], npc)
        add_curve(f"{npc}_HairFall", [(0, 2.30 * s, -0.18 * s), (0, 1.95 * s, -0.26 * s), (0, 1.62 * s, -0.24 * s)], 0.13 * s, mats["dark"], npc)
        for i in range(7):
            a = math.tau * i / 7
            add_curve(f"{npc}_Spine{i}", [(0.24 * s * math.cos(a), y + 0.20 * s, -0.10 * s + 0.24 * s * math.sin(a)), (0.20 * s * math.cos(a), y + 0.46 * s, -0.10 * s + 0.20 * s * math.sin(a))], 0.035 * s, mats["bone"], npc)
    elif head == "stone":
        add_ico(f"{npc}_StoneBrow", (0, y + 0.17 * s, z + 0.20 * s), (0.24 * s, 0.07 * s, 0.05 * s), mats["dark"], npc)
        for i in range(4):
            add_ico(f"{npc}_Stone{i}", (-0.21 * s + i * 0.14 * s, y + 0.28 * s - abs(i - 1.5) * 0.03 * s, -0.04 * s), (0.10 * s, 0.11 * s, 0.09 * s), mats["cloth"], npc)
        for side in (-1, 1):
            add_ico(f"{npc}_StoneCheek{side_label(side)}", (side * 0.20 * s, y - 0.07 * s, z + 0.13 * s), (0.08 * s, 0.12 * s, 0.07 * s), mats["cloth"], npc)


def add_garment(npc: str, cfg: dict, mats: dict[str, bpy.types.Material], s: float) -> None:
    """Silhouette layer per archetype. A 128px cell reads the outline, not the trim."""
    garment = cfg["garment"]
    if garment == "robe":
        lathe(f"{npc}_Skirt", [(0.05 * s, 0.50 * s, 0.44 * s), (0.34 * s, 0.46 * s, 0.40 * s), (0.74 * s, 0.41 * s, 0.35 * s), (1.06 * s, 0.37 * s, 0.31 * s), (1.18 * s, 0.33 * s, 0.28 * s)], mats["cloth"], npc)
        add_torus(f"{npc}_Hem", (0, 0.08 * s, 0), 0.48 * s, 0.035 * s, mats["accent"], npc, (math.pi / 2, 0, 0))
        add_curve(f"{npc}_Sash", [(0, 1.30 * s, 0.30 * s), (0, 1.22 * s, 0.34 * s), (0, 1.12 * s, 0.30 * s)], 0.05 * s, mats["accent"], npc)
    elif garment == "cloak":
        add_sphere(f"{npc}_Cloak", (0, 1.46 * s, -0.28 * s), (0.47 * s, 0.74 * s, 0.24 * s), mats["cloth"], npc)
        add_curve(f"{npc}_CloakHem", [(-0.36 * s, 0.78 * s, -0.28 * s), (0, 0.70 * s, -0.36 * s), (0.36 * s, 0.78 * s, -0.28 * s)], 0.05 * s, mats["accent"], npc)
        add_ico(f"{npc}_Clasp", (0, 2.14 * s, 0.20 * s), (0.08 * s, 0.08 * s, 0.05 * s), mats["accent"], npc)
    elif garment == "apron":
        verts = []
        for y, half, z in ((1.98, 0.22, 0.20), (1.60, 0.28, 0.24), (1.20, 0.32, 0.28), (0.68, 0.34, 0.29)):
            verts += [(-half * s, y * s, z * s), (0, y * s, (z + 0.06) * s), (half * s, y * s, z * s)]
        faces = [(row * 3 + column, row * 3 + column + 1, (row + 1) * 3 + column + 1, (row + 1) * 3 + column) for row in range(3) for column in range(2)]
        mesh_object(f"{npc}_Apron", verts, faces, mats["cloth_shade"], npc, thickness=0.04 * s, bevel=0.015 * s)
        add_ico(f"{npc}_Pouch", (0.30 * s, 1.06 * s, 0.10 * s), (0.13 * s, 0.15 * s, 0.10 * s), mats["dark"], npc)
        add_ico(f"{npc}_Bib", (0, 1.86 * s, 0.28 * s), (0.15 * s, 0.13 * s, 0.04 * s), mats["accent"], npc)
    elif garment == "brigandine":
        lathe(f"{npc}_Cuirass", [(0.94 * s, 0.39 * s, 0.29 * s), (1.22 * s, 0.45 * s, 0.32 * s), (1.46 * s, 0.38 * s, 0.29 * s), (1.74 * s, 0.45 * s, 0.32 * s), (2.02 * s, 0.49 * s, 0.32 * s), (2.16 * s, 0.37 * s, 0.26 * s)], mats["steel"], npc)
        for i, y in enumerate((1.30, 1.62, 1.94)):
            add_curve(f"{npc}_Strap{i}", [(-0.44 * s, y * s, 0.10 * s), (-0.22 * s, (y + 0.04) * s, 0.28 * s), (0, (y + 0.05) * s, 0.33 * s), (0.22 * s, (y + 0.04) * s, 0.28 * s), (0.44 * s, y * s, 0.10 * s)], 0.028 * s, mats["accent"], npc)
        add_torus(f"{npc}_Collar", (0, 2.18 * s, 0), 0.26 * s, 0.055 * s, mats["steel"], npc, (math.pi / 2, 0, 0))
        lathe(f"{npc}_Tasset", [(1.08 * s, 0.40 * s, 0.33 * s), (0.90 * s, 0.43 * s, 0.35 * s), (0.72 * s, 0.39 * s, 0.32 * s)], mats["steel"], npc)
        for side in (-1, 1):
            add_sphere(f"{npc}_Pauldron{side_label(side)}", (side * 0.42 * s, 2.04 * s, 0), (0.21 * s, 0.20 * s, 0.23 * s), mats["steel"], npc)
    elif garment == "vest":
        for side in (-1, 1):
            verts = []
            for y, x, z in ((1.98, 0.08, 0.32), (1.52, 0.24, 0.30), (1.08, 0.32, 0.28)):
                verts += [(side * x * s, y * s, z * s), (side * (x + 0.11) * s, y * s, (z - 0.06) * s)]
            faces = [(row * 2, row * 2 + 1, (row + 1) * 2 + 1, (row + 1) * 2) for row in range(2)]
            mesh_object(f"{npc}_Vest{side_label(side)}", verts, faces, mats["cloth_shade"], npc, thickness=0.045 * s, bevel=0.015 * s)
            add_ico(f"{npc}_Stud{side_label(side)}", (side * 0.30 * s, 1.24 * s, 0.28 * s), (0.07 * s, 0.07 * s, 0.05 * s), mats["accent"], npc)
            add_sphere(f"{npc}_ShoulderPad{side_label(side)}", (side * 0.42 * s, 2.04 * s, 0), (0.22 * s, 0.20 * s, 0.24 * s), mats["cloth_shade"], npc)


def add_humanoid_body(npc: str, cfg: dict, mats: dict[str, bpy.types.Material]) -> tuple[dict[str, tuple], dict[str, tuple]]:
    s = cfg["scale"]; form = cfg["form"]
    torso_profile = [(0.86 * s, 0.34 * s, 0.25 * s), (1.12 * s, 0.40 * s, 0.27 * s), (1.36 * s, 0.32 * s, 0.24 * s), (1.68 * s, 0.40 * s, 0.27 * s), (1.98 * s, 0.44 * s, 0.27 * s), (2.14 * s, 0.34 * s, 0.22 * s), (2.25 * s, 0.16 * s, 0.15 * s)]
    lathe(f"{npc}_Torso", torso_profile, mats["bone"] if form == "undead" else mats["cloth"], npc)
    add_sphere(f"{npc}_Chest", (0, 1.68 * s, 0.27 * s), (0.30 * s, 0.34 * s, 0.05 * s), mats["accent"], npc)
    add_torus(f"{npc}_Belt", (0, 1.12 * s, 0), 0.34 * s, 0.03 * s, mats["accent"], npc, (math.pi / 2, 0, 0))
    add_garment(npc, cfg, mats, s)
    add_head(npc, cfg, mats, s)
    arms = {"L": ((-0.38 * s, 2.02 * s, 0), (-0.56 * s, 1.72 * s, 0.02 * s), (-0.50 * s, 1.38 * s, 0.08 * s), (-0.50 * s, 1.22 * s, 0.10 * s)), "R": ((0.38 * s, 2.02 * s, 0), (0.58 * s, 1.72 * s, 0.02 * s), (0.62 * s, 1.40 * s, 0.08 * s), (0.64 * s, 1.24 * s, 0.10 * s))}
    for label, pts in arms.items():
        add_curve(f"{npc}_UpperArm{label}", pts[:2], 0.12 * s, mats["cloth"], npc); add_ico(f"{npc}_Elbow{label}", pts[1], (0.12 * s, 0.12 * s, 0.11 * s), mats["steel"], npc); add_curve(f"{npc}_Forearm{label}", pts[1:3], 0.095 * s, mats["cloth"], npc); add_ico(f"{npc}_Hand{label}", pts[3], (0.10 * s, 0.12 * s, 0.09 * s), mats["skin"], npc)
    legs = {"L": ((-0.20 * s, 1.05 * s, 0), (-0.25 * s, 0.58 * s, 0.02 * s), (-0.27 * s, 0.10 * s, 0.05 * s)), "R": ((0.20 * s, 1.05 * s, 0), (0.25 * s, 0.58 * s, 0.02 * s), (0.27 * s, 0.10 * s, 0.05 * s))}
    for label, pts in legs.items():
        add_curve(f"{npc}_Thigh{label}", pts[:2], 0.14 * s, mats["cloth"], npc); add_ico(f"{npc}_Knee{label}", pts[1], (0.13 * s, 0.12 * s, 0.12 * s), mats["steel"], npc); add_curve(f"{npc}_Shin{label}", pts[1:], 0.10 * s, mats["steel"], npc); add_sphere(f"{npc}_Boot{label}", (pts[2][0], 0.08 * s, 0.08 * s), (0.15 * s, 0.10 * s, 0.18 * s), mats["dark"], npc)
    if form == "undead":
        for i, y in enumerate((1.42, 1.58, 1.74)): add_curve(f"{npc}_Rib{i}", [(-0.22 * s, y * s, 0.30 * s), (0, (y - 0.05) * s, 0.33 * s), (0.22 * s, y * s, 0.30 * s)], 0.022 * s, mats["bone"], npc)
    return arms, legs


def add_weapon(npc: str, cfg: dict, mats: dict[str, bpy.types.Material], s: float) -> None:
    weapon = cfg["weapon"]; x = 0.64 * s
    if weapon in {"dagger", "knife", "sword"}:
        length = {"dagger": 0.32, "knife": 0.42, "sword": 0.78}[weapon] * s
        add_curve(f"{npc}_WeaponShaft", [(x, 0.85 * s, 0.10 * s), (x, 1.65 * s, 0.10 * s)], 0.028 * s, mats["dark"], npc)
        verts = [(x - 0.07 * s, 1.65 * s, 0.10 * s), (x, 1.65 * s + length, 0.10 * s), (x + 0.07 * s, 1.65 * s, 0.10 * s)]
        mesh_object(f"{npc}_WeaponBlade", verts, [(0, 1, 2)], mats["steel"], npc, thickness=0.035 * s)
        add_ico(f"{npc}_WeaponGuard", (x, 1.63 * s, 0.10 * s), (0.14 * s, 0.035 * s, 0.05 * s), mats["accent"], npc)
    elif weapon == "greataxe":
        add_curve(f"{npc}_AxeShaft", [(x, 0.18 * s, 0.10 * s), (x, 2.25 * s, 0.10 * s)], 0.05 * s, mats["dark"], npc)
        add_ico(f"{npc}_AxeHead", (x, 2.10 * s, 0.10 * s), (0.28 * s, 0.30 * s, 0.10 * s), mats["steel"], npc)
    elif weapon == "gavel":
        add_curve(f"{npc}_GavelShaft", [(x, 0.28 * s, 0.10 * s), (x, 2.22 * s, 0.10 * s)], 0.055 * s, mats["dark"], npc)
        add_ico(f"{npc}_GavelHead", (x, 2.22 * s, 0.10 * s), (0.30 * s, 0.22 * s, 0.22 * s), mats["accent"], npc)
    elif weapon in {"staff", "ringstaff", "kelp"}:
        add_curve(f"{npc}_Staff", [(x, 0.12 * s, 0.10 * s), (x, 2.35 * s, 0.10 * s)], 0.035 * s, mats["dark"], npc)
        add_ico(f"{npc}_StaffTop", (x, 2.48 * s, 0.10 * s), (0.13 * s, 0.13 * s, 0.13 * s), mats["glow"] if weapon != "kelp" else mats["accent"], npc)
        if weapon == "ringstaff": add_torus(f"{npc}_StaffRing", (x, 2.48 * s, 0.10 * s), 0.20 * s, 0.025 * s, mats["accent"], npc, (math.pi / 2, 0, 0))
    elif weapon == "flail":
        add_curve(f"{npc}_FlailShaft", [(x, 0.35 * s, 0.10 * s), (x, 2.05 * s, 0.10 * s)], 0.05 * s, mats["dark"], npc); add_curve(f"{npc}_FlailChain", [(x, 2.05 * s, 0.10 * s), (x + 0.30 * s, 2.45 * s, 0.10 * s)], 0.025 * s, mats["steel"], npc); add_ico(f"{npc}_FlailWeight", (x + 0.32 * s, 2.50 * s, 0.10 * s), (0.16 * s, 0.16 * s, 0.16 * s), mats["steel"], npc)
    elif weapon == "orb": add_ico(f"{npc}_Orb", (x, 1.85 * s, 0.14 * s), (0.16 * s, 0.16 * s, 0.16 * s), mats["glow"], npc)
    elif weapon == "vials":
        for i, dx in enumerate((-0.12, 0, 0.12)): add_ico(f"{npc}_Vial{i}", (dx * s, 1.15 * s, 0.30 * s), (0.045 * s, 0.10 * s, 0.045 * s), mats["glow"] if i == 1 else mats["accent"], npc)
    elif weapon == "page": add_ico(f"{npc}_Page", (x, 1.60 * s, 0.18 * s), (0.16 * s, 0.22 * s, 0.02 * s), mats["bone"], npc)
    elif weapon == "pack": add_ico(f"{npc}_Pack", (-0.20 * s, 1.55 * s, -0.28 * s), (0.28 * s, 0.34 * s, 0.18 * s), mats["dark"], npc)
    elif weapon in {"claws", "fists"}:
        for side in (-1, 1):
            for i in range(3): add_ico(f"{npc}_Claw{side_label(side)}{i}", (side * 0.52 * s + (i - 1) * 0.05 * s, 1.30 * s, 0.10 * s), (0.035 * s, 0.11 * s, 0.035 * s), mats["bone"] if cfg["form"] != "wisp" else mats["glow"], npc)
    if cfg["weapon"] == "swordshield":
        verts = [(-0.58 * s, 1.90 * s, 0.30 * s), (-0.34 * s, 1.90 * s, 0.30 * s), (-0.30 * s, 1.10 * s, 0.30 * s), (-0.62 * s, 1.10 * s, 0.30 * s)]
        mesh_object(f"{npc}_Shield", verts, [(0, 1, 2, 3)], mats["steel"], npc, thickness=0.06 * s, bevel=0.02 * s)


def add_wisp_body(npc: str, cfg: dict, mats: dict[str, bpy.types.Material]) -> None:
    s = cfg["scale"]; profile = [(0.55 * s, 0.05 * s, 0.05 * s), (0.95 * s, 0.20 * s, 0.16 * s), (1.45 * s, 0.32 * s, 0.24 * s), (1.95 * s, 0.36 * s, 0.26 * s), (2.35 * s, 0.20 * s, 0.16 * s)]
    lathe(f"{npc}_Tail", profile, mats["glow"], npc); add_head(npc, cfg, mats, s)
    for side in (-1, 1): add_curve(f"{npc}_WispArm{side_label(side)}", [(side * 0.22 * s, 1.80 * s, 0), (side * 0.52 * s, 1.55 * s, 0.04 * s), (side * 0.62 * s, 1.25 * s, 0.06 * s)], 0.06 * s, mats["glow"], npc)
    add_ico(f"{npc}_WispCore", (0, 1.55 * s, 0.20 * s), (0.14 * s, 0.14 * s, 0.08 * s), mats["accent"], npc)


def add_beast_body(npc: str, cfg: dict, mats: dict[str, bpy.types.Material]) -> dict[str, tuple]:
    s = cfg["scale"]; form = cfg["form"]
    # Quadrupeds extend along local Z (front/back), not the humanoid vertical axis.
    add_sphere(f"{npc}_Body", (0, 1.02 * s, -0.20 * s), (0.46 * s, 0.38 * s, 0.86 * s), mats["skin"], npc)
    add_sphere(f"{npc}_Chest", (0, 1.16 * s, 0.42 * s), (0.50 * s, 0.44 * s, 0.42 * s), mats["skin"], npc)
    add_curve(f"{npc}_Neck", [(0, 1.28 * s, 0.52 * s), (0, 1.56 * s, 0.70 * s)], 0.20 * s, mats["skin"], npc)
    add_sphere(f"{npc}_BeastHead", (0, 1.66 * s, 0.80 * s), (0.30 * s, 0.31 * s, 0.38 * s), mats["skin"], npc)
    add_sphere(f"{npc}_Muzzle", (0, 1.55 * s, 1.10 * s), (0.22 * s, 0.17 * s, 0.26 * s), mats["skin"] if form != "rat" else mats["accent"], npc)
    for side in (-1, 1):
        add_ico(f"{npc}_Ear{side_label(side)}", (side * 0.23 * s, 1.92 * s, 0.72 * s), (0.09 * s, 0.17 * s, 0.08 * s), mats["skin"], npc)
        add_ico(f"{npc}_Eye{side_label(side)}", (side * 0.13 * s, 1.74 * s, 1.05 * s), (0.05 * s, 0.035 * s, 0.025 * s), mats["dark"], npc)
    legs = {
        "FL": ((-0.32 * s, 1.00 * s, 0.48 * s), (-0.40 * s, 0.12 * s, 0.54 * s)),
        "FR": ((0.32 * s, 1.00 * s, 0.48 * s), (0.40 * s, 0.12 * s, 0.54 * s)),
        "BL": ((-0.32 * s, 0.92 * s, -0.62 * s), (-0.44 * s, 0.12 * s, -0.68 * s)),
        "BR": ((0.32 * s, 0.92 * s, -0.62 * s), (0.44 * s, 0.12 * s, -0.68 * s)),
    }
    for label, pts in legs.items():
        add_curve(f"{npc}_Leg{label}", pts, 0.12 * s, mats["skin"] if form != "bear" else mats["dark"], npc)
        add_ico(f"{npc}_Foot{label}", (pts[1][0], 0.10 * s, pts[1][2] + 0.06 * s), (0.14 * s, 0.10 * s, 0.18 * s), mats["dark"], npc)
    add_curve(f"{npc}_Tail", [(0, 1.02 * s, -0.95 * s), (0.10 * s, 1.18 * s, -1.24 * s), (0.24 * s, 1.38 * s, -1.38 * s)], 0.07 * s, mats["skin"], npc)
    if form == "stag":
        for side in (-1, 1):
            add_curve(f"{npc}_Antler{side_label(side)}", [(side * 0.16 * s, 1.96 * s, 0.70 * s), (side * 0.40 * s, 2.42 * s, 0.64 * s), (side * 0.56 * s, 2.84 * s, 0.58 * s)], 0.055 * s, mats["bone"], npc)
    if form == "rat":
        for i in range(5):
            add_ico(f"{npc}_Shard{i}", (-0.22 * s + i * 0.11 * s, 1.98 * s, 0.70 * s), (0.045 * s, 0.16 * s, 0.045 * s), mats["accent"], npc)
    if cfg["weapon"] in {"claws", "fists"}:
        for label, pts in (("F", legs["FL"]), ("B", legs["FR"])):
            for i in range(3):
                add_ico(f"{npc}_Claw{label}{i}", (pts[1][0] + (i - 1) * 0.05 * s, 0.26 * s, pts[1][2] + 0.12 * s), (0.035 * s, 0.10 * s, 0.035 * s), mats["bone"], npc)
    return legs


def add_markers(npc: str) -> None:
    for name, kind, loc in ((f"{npc}_front", "front", (0, 2.4, 1.4)), (f"{npc}_back", "back", (0, 2.4, -1.4)), (f"{npc}_pivot", "pivot", (0, 0, 0))):
        obj = bpy.data.objects.new(name, None); bpy.context.collection.objects.link(obj); obj.location = loc; obj.empty_display_type = "PLAIN_AXES"; obj.empty_display_size = 0.2; obj["asset3d_id"] = npc; obj["asset3d_marker"] = kind

def side_label(side: int) -> str:
    """`-1` / `1` as `L` / `R`.

    A part named with the raw int (`Pauldron-1`) does not end in "L", so
    `bone_group` routes it to the RIGHT limb and the left-hand ornament rides the
    right arm. Every side-named part must use this.
    """
    return "L" if side < 0 else "R"


# Object-name prefix -> deform bone. Every mesh is rigid-weighted to one bone, so
# an unmapped part silently rides the spine instead of following the head.
HEAD_PARTS = ("Face", "Eye", "Cheek", "Ear", "Brow", "Nose", "Mouth", "Hood", "Helmet",
              "Cap", "Mantle", "Horn", "Crown", "Mitre", "Skull", "Glow", "Antler",
              "Stone", "Hair", "Spine", "Shard", "Muzzle", "Jaw", "Whisker", "CheekTuft",
              "Beard", "Mask", "Veil", "Goggles", "Respirator", "Halo", "Hat", "Coif",
              "CrownHalo", "SolarRay", "SpiderEye", "Mandible", "ThirdEye", "Earring",
              "Infula", "Liripipe", "Braid", "CowlTail", "BandanaKnot", "HatRibbon", "Chaperon")
WEAPON_PARTS = ("Weapon", "Axe", "Gavel", "Staff", "Flail", "Orb", "Page", "Vial", "Claw", "Shield", "Tome", "Book", "Scepter", "Maul", "Cutlass", "Lantern", "Rune")
LEG_PARTS = ("Thigh", "Shin", "Boot", "Knee")


def bone_group(npc: str, cfg: dict, name: str) -> str:
    def starts(*prefixes: str) -> bool:
        return any(name.startswith(f"{npc}_{prefix}") for prefix in prefixes)
    side = "L" if name.endswith("L") else "R"
    if starts("UpperArm", "Elbow", "Pauldron", "ShoulderPad", "SpikedPauldron"): return f"upper_arm.{side}"
    if starts("Forearm", "Hand", "Gauntlet"): return f"forearm.{side}"
    if starts(*LEG_PARTS): return f"thigh.{side}"
    if starts("Shield"): return "forearm.L"
    if starts(*WEAPON_PARTS): return "weapon.R"
    if starts(*HEAD_PARTS): return "head"
    if cfg["form"] in BEAST_FORMS:
        if starts("Ruff", "Throat", "Neck", "ChestNeck", "ManeTuftUpper"): return "neck"
        if starts("Hump", "Bib", "ChestBib", "DorsalMane", "Tail", "Rump", "SpineShard", "Mane", "Shoulder", "Haunch"): return "spine"
    if cfg["form"] in BEAST_FORMS and starts("Leg", "Foot"):
        return "leg." + ("FL" if "FL" in name else "FR" if "FR" in name else "BL" if "BL" in name else "BR")
    return "spine"



def add_rig(npc: str, cfg: dict, arms: dict[str, tuple], legs: dict[str, tuple], mats: dict) -> bpy.types.Object:
    parts = [obj for obj in bpy.context.scene.objects if obj.type == "MESH" and obj.get("asset3d_id") == npc]
    data = bpy.data.armatures.new(f"{npc}_RigData"); rig = bpy.data.objects.new(f"{npc}_Rig", data); bpy.context.collection.objects.link(rig); rig["asset3d_id"] = npc; rig["asset3d_rig"] = True; rig.rotation_euler = BASE_ROT
    bpy.ops.object.select_all(action="DESELECT"); bpy.context.view_layer.objects.active = rig; rig.select_set(True); bpy.ops.object.mode_set(mode="EDIT")
    bones = data.edit_bones; s = cfg["scale"]
    root = bones.new("root"); root.head = (0, 0, 0); root.tail = (0, 0.8 * s, 0)
    spine = bones.new("spine"); spine.head = (0, 0.8 * s, 0); spine.tail = (0, 2.0 * s, 0); spine.parent = root
    neck = bones.new("neck"); neck.head = (0, 2.0 * s, 0); neck.tail = (0, 2.35 * s, 0); neck.parent = spine
    head = bones.new("head"); head.head = (0, 2.35 * s, 0); head.tail = (0, 2.9 * s, 0); head.parent = neck
    if cfg["form"] not in NON_HUMANOID:
        for label, pts in arms.items():
            upper = bones.new(f"upper_arm.{label}"); upper.head, upper.tail = pts[0], pts[1]; upper.parent = spine
            fore = bones.new(f"forearm.{label}"); fore.head, fore.tail = pts[1], pts[2]; fore.parent = upper; fore.use_connect = True
            hand = bones.new(f"hand.{label}"); hand.head, hand.tail = pts[2], pts[3]; fore.parent = upper; hand.parent = fore; hand.use_connect = True
    if STUDY_ANY and cfg["form"] not in NON_HUMANOID:
        for label, pts in legs.items():
            thigh = bones.new(f"thigh.{label}"); thigh.head, thigh.tail = pts[0], pts[1]; thigh.parent = root
            shin = bones.new(f"shin.{label}"); shin.head, shin.tail = pts[1], pts[2]; shin.parent = thigh; shin.use_connect = True
    if cfg["form"] in BEAST_FORMS:
        for label, pts in legs.items():
            leg = bones.new(f"leg.{label}"); leg.head, leg.tail = pts[0], pts[1]; leg.parent = root
    if cfg["weapon"] not in {"unarmed", "fists", "claws"}: weapon = bones.new("weapon.R"); weapon.head = (0.64 * s, 1.24 * s, 0.10 * s); weapon.tail = (0.64 * s, 2.60 * s, 0.10 * s); weapon.parent = bones.get("hand.R") or spine
    bpy.ops.object.mode_set(mode="OBJECT"); bpy.ops.object.select_all(action="DESELECT"); rig.select_set(True)
    for obj in parts: obj.select_set(True)
    bpy.context.view_layer.objects.active = rig; bpy.ops.object.parent_set(type="ARMATURE_AUTO")
    if STUDY_ANY:
        for obj in parts: obj.matrix_parent_inverse = Matrix.Identity(4)
    for obj in parts:
        for group in obj.vertex_groups: group.remove(list(range(len(obj.data.vertices))))
        name = obj.name
        if STUDY_ANY and (name.startswith(f"{npc}_ArmSleeve") or name.startswith(f"{npc}_Legging")):
            label = name[-1]
            arm = name.startswith(f"{npc}_ArmSleeve")
            joint = (arms if arm else legs)[label][1][1]
            upper_name, lower_name = ("upper_arm", "forearm") if arm else ("thigh", "shin")
            upper = obj.vertex_groups.get(f"{upper_name}.{label}") or obj.vertex_groups.new(name=f"{upper_name}.{label}")
            lower = obj.vertex_groups.get(f"{lower_name}.{label}") or obj.vertex_groups.new(name=f"{lower_name}.{label}")
            blend = 0.09 * cfg["scale"]
            for vertex in obj.data.vertices:
                upper_weight = max(0.0, min(1.0, (vertex.co.y - joint + blend) / (2 * blend)))
                if upper_weight > 0: upper.add([vertex.index], upper_weight, "REPLACE")
                if upper_weight < 1: lower.add([vertex.index], 1 - upper_weight, "REPLACE")
        elif STUDY_ANY and cfg["form"] in BEAST_FORMS and any(name == f"{npc}_Leg{lbl}" for lbl in ("FL", "FR", "BL", "BR")):
            label = name[len(f"{npc}_Leg"):]
            leg_name = f"leg.{label}"
            leg_grp = obj.vertex_groups.get(leg_name) or obj.vertex_groups.new(name=leg_name)
            spine_grp = obj.vertex_groups.get("spine") or obj.vertex_groups.new(name="spine")
            pts = legs.get(label)
            if pts:
                hip_y = pts[0][1]
                blend_bottom = hip_y * 0.70
                blend_top = hip_y * 0.98
                for vertex in obj.data.vertices:
                    vy = vertex.co.y
                    if vy <= blend_bottom:
                        w_spine = 0.0
                    elif vy >= blend_top:
                        w_spine = 0.82
                    else:
                        t = (vy - blend_bottom) / (blend_top - blend_bottom)
                        w_spine = 0.82 * (t * t * (3.0 - 2.0 * t))
                    w_leg = 1.0 - w_spine
                    if w_leg > 0: leg_grp.add([vertex.index], w_leg, "REPLACE")
                    if w_spine > 0: spine_grp.add([vertex.index], w_spine, "REPLACE")
            else:
                leg_grp.add(list(range(len(obj.data.vertices))), 1.0, "REPLACE")
        elif STUDY_ANY and cfg["form"] in BEAST_FORMS and (name == f"{npc}_Neck" or name == f"{npc}_ChestNeck" or name == f"{npc}_NeckMane"):
            neck_grp = obj.vertex_groups.get("neck") or obj.vertex_groups.new(name="neck")
            spine_grp = obj.vertex_groups.get("spine") or obj.vertex_groups.new(name="spine")
            verts_y = [v.co.y for v in obj.data.vertices]
            min_y = min(verts_y) if verts_y else 0
            max_y = max(verts_y) if verts_y else 1
            span = max(0.001, max_y - min_y)
            for vertex in obj.data.vertices:
                t = (vertex.co.y - min_y) / span
                w_neck = max(0.0, min(1.0, (t - 0.15) / 0.50))
                w_spine = 1.0 - w_neck
                if w_neck > 0: neck_grp.add([vertex.index], w_neck, "REPLACE")
                if w_spine > 0: spine_grp.add([vertex.index], w_spine, "REPLACE")
        else:
            group_name = bone_group(npc, cfg, name)
            if STUDY_ANY and name.startswith(f"{npc}_Boot"):
                group_name = f"shin.{name[-1]}"
            if STUDY_ANY and name == f"{npc}_SpineRear":
                group_name = "spine"
            group = obj.vertex_groups.get(group_name) or obj.vertex_groups.new(name=group_name)
            group.add(list(range(len(obj.data.vertices))), 1.0, "REPLACE")
    for marker in (bpy.data.objects.get(f"{npc}_front"), bpy.data.objects.get(f"{npc}_back"), bpy.data.objects.get(f"{npc}_pivot")):
        if marker is not None: marker.parent = rig; marker.matrix_parent_inverse = Matrix.Identity(4)
    return rig


def add_turntable(rig: bpy.types.Object) -> bpy.types.Object:
    turntable = bpy.data.objects.new("NPC_Turntable", None); bpy.context.collection.objects.link(turntable); turntable.empty_display_type = "PLAIN_AXES"; rig.parent = turntable; rig.matrix_parent_inverse = Matrix.Identity(4); return turntable


def reset_pose(rig: bpy.types.Object) -> None:
    for bone in rig.pose.bones: bone.location = (0, 0, 0); bone.rotation_mode = "XYZ"; bone.rotation_euler = (0, 0, 0); bone.scale = (1, 1, 1)


def apply_pose(rig: bpy.types.Object, cfg: dict, anim: str, frame: int) -> None:
    """Pose the rig for one frame of an animation track. `frame` indexes the track."""
    reset_pose(rig)
    form = cfg["form"]
    s = cfg["scale"]
    bob = forward = spine_roll = neck_roll = head_roll = 0.0
    weapon = (0.0, 0.0, 0.0)
    if anim == "idle":
        # Every form has to breathe: a quadruped or wisp has no arm bones, so an
        # arm-only idle is a frozen pose.
        breath = 1.0 if frame == 0 else -1.0
        bob, spine_roll, neck_roll = 0.04 * breath, 0.05 * breath, 0.07 * breath
        arms = {"upper_arm.L": (0.05, 0.0, -0.10 + 0.05 * breath), "forearm.L": (0.12, 0.0, 0.12), "upper_arm.R": (0.0, 0.0, 0.10 + 0.05 * breath), "forearm.R": (0.12, 0.0, -0.14)}
        legs = {"thigh.L": 0.0, "shin.L": 0.0, "thigh.R": 0.0, "shin.R": 0.0}
        if form in BEAST_FORMS:
            legs = {"leg.FL": 0.10 * breath, "leg.FR": -0.09 * breath, "leg.BL": -0.09 * breath, "leg.BR": 0.10 * breath}
    elif anim == "walk":
        sign = 1 if frame == 0 else -1
        bob, spine_roll = 0.05 * sign, 0.03 * sign
        arms = {"upper_arm.L": (0.0, 0.0, 0.30 * sign), "forearm.L": (0.18, 0.0, -0.18 * sign), "upper_arm.R": (0.0, 0.0, -0.26 * sign), "forearm.R": (0.18, 0.0, -0.12)}
        legs = {"thigh.L": 0.34 * sign, "shin.L": -0.20 * sign, "thigh.R": -0.34 * sign, "shin.R": 0.20 * sign}
        if form in BEAST_FORMS:
            legs = {"leg.FL": 0.26 * sign, "leg.FR": -0.26 * sign, "leg.BL": -0.26 * sign, "leg.BR": 0.26 * sign}
    elif anim == "attack":
        if frame == 0:
            spine_roll, neck_roll, forward = 0.22, 0.16, -0.10 * s
            arms = {"upper_arm.L": (-0.55, 0.0, -0.62), "forearm.L": (0.42, 0.0, 0.52), "upper_arm.R": (-0.55, 0.0, 0.66), "forearm.R": (0.42, 0.0, -0.58)}
            weapon = (0.0, 0.0, 0.85)
        elif frame == 1:
            spine_roll, neck_roll, forward = -0.26, -0.20, 0.20 * s
            arms = {"upper_arm.L": (0.34, 0.0, -0.16), "forearm.L": (0.10, 0.0, 0.14), "upper_arm.R": (0.34, 0.0, 0.20), "forearm.R": (0.10, 0.0, -0.18)}
            weapon = (0.0, 0.0, -0.95)
        elif frame == 2:
            spine_roll, neck_roll, forward = 0.20, 0.14, 0.05 * s
            arms = {"upper_arm.L": (-0.40, 0.0, -0.46), "forearm.L": (0.30, 0.0, 0.40), "upper_arm.R": (-0.40, 0.0, 0.50), "forearm.R": (0.30, 0.0, -0.44)}
            weapon = (0.0, 0.0, 0.45)
        else:
            spine_roll, neck_roll, forward = 0.06, 0.04, 0.0
            arms = {"upper_arm.L": (0.10, 0.0, -0.14), "forearm.L": (0.14, 0.0, 0.16), "upper_arm.R": (0.10, 0.0, 0.16), "forearm.R": (0.14, 0.0, -0.18)}
            weapon = (0.0, 0.0, -0.10)
        legs = {"thigh.L": 0.0, "shin.L": 0.0, "thigh.R": 0.0, "shin.R": 0.0}
        if form in BEAST_FORMS:
            if frame == 0:
                legs = {"leg.FL": 0.18, "leg.FR": 0.14, "leg.BL": -0.20, "leg.BR": -0.18}
            elif frame == 1:
                legs = {"leg.FL": -0.32, "leg.FR": -0.28, "leg.BL": 0.30, "leg.BR": 0.26}
            elif frame == 2:
                legs = {"leg.FL": -0.12, "leg.FR": 0.16, "leg.BL": 0.14, "leg.BR": -0.10}
            else:
                legs = {"leg.FL": 0.05, "leg.FR": -0.05, "leg.BL": -0.05, "leg.BR": 0.05}
    elif anim == "phase":
        # A phase action is a two-frame build: the coil tightens, then holds deeper.
        build = 1.0 if frame == 0 else 1.18
        spine_roll, neck_roll = 0.26 * build, 0.14 * build
        arms = {"upper_arm.L": (-0.30 * build, 0.0, 0.70 * build), "forearm.L": (0.36 * build, 0.0, 0.40 * build), "upper_arm.R": (-0.30 * build, 0.0, -0.70 * build), "forearm.R": (0.36 * build, 0.0, -0.40 * build)}
        weapon = (0.0, 0.0, 0.55 * build)
        legs = {"thigh.L": 0.0, "shin.L": 0.0, "thigh.R": 0.0, "shin.R": 0.0}
        if form in BEAST_FORMS:
            legs = {"leg.FL": -0.15 * build, "leg.FR": -0.15 * build, "leg.BL": 0.20 * build, "leg.BR": 0.20 * build}
    else:
        settle = 1.0 if frame == 0 else -1.0
        spine_roll, neck_roll, head_roll, forward = 0.24 * settle, 0.12, -0.20 * settle, -0.12 * s
        arms = {"upper_arm.L": (0.0, 0.0, 0.52 * settle), "forearm.L": (0.30, 0.0, 0.36), "upper_arm.R": (0.0, 0.0, -0.48 * settle), "forearm.R": (0.30, 0.0, -0.36)}
        weapon = (0.0, 0.0, 0.30 * settle)
        legs = {"thigh.L": 0.16 * settle, "shin.L": -0.12, "thigh.R": -0.16 * settle, "shin.R": 0.12}
        if form in BEAST_FORMS:
            legs = {"leg.FL": 0.24 * settle, "leg.FR": 0.20 * settle, "leg.BL": -0.16 * settle, "leg.BR": -0.18 * settle}
    # A wisp floats, so it reads the bob far more strongly than a planted body.
    rig.location = (0, bob * (0.9 if form == "wisp" else 0.35) + forward * 0.15, forward)
    rig.rotation_euler = BASE_ROT
    for bone_name, angle in (("spine", spine_roll), ("neck", neck_roll), ("head", head_roll)):
        if bone_name in rig.pose.bones: rig.pose.bones[bone_name].rotation_euler.z = angle
    for name, angles in arms.items():
        if name in rig.pose.bones: rig.pose.bones[name].rotation_euler = angles
    for name, angle in legs.items():
        if name in rig.pose.bones: rig.pose.bones[name].rotation_euler.z = angle
    if "weapon.R" in rig.pose.bones: rig.pose.bones["weapon.R"].rotation_euler = weapon
    bpy.context.view_layer.update()


# Pose-track length per animation. Clips are cut from these tracks, so a rendered
# frame and the matching GLB clip always show the same pose.
TRACK_LENGTH = {"idle": 2, "walk": 2, "attack": 4, "hit": 2, "phase": 2}


def render_counts(cfg: dict) -> dict[str, int]:
    """Frames rendered per animation row. Attack renders one frame per declared beat."""
    counts = {name: TRACK_LENGTH[name] for name in ("idle", "walk", "hit")}
    counts["attack"] = max(1, min(len(cfg["clips"]["attack"]), TRACK_LENGTH["attack"]))
    return counts


def clip_frames(action: str, index: int, count: int) -> list[int]:
    """Track frames a single declared clip animates through. Each declared beat
    advances to the next and the last wraps, so no clip is a frozen pose."""
    if action == "attack" and count > 1:
        return [index, (index + 1) % count]
    length = TRACK_LENGTH[action]
    return [index % length, (index + 1) % length]


def build_action_library(rig: bpy.types.Object, cfg: dict) -> None:
    profile = action_profile(cfg)
    rig.animation_data_create()
    clips = [(name, action, index, len(names)) for action, names in profile["clips"].items() for index, name in enumerate(names)]
    clips += [(name, "phase", 0, 1) for name in profile.get("boss", {}).get("phase_actions", [])]
    default_action = None
    for name, anim, index, count in clips:
        action = bpy.data.actions.new(name)
        action.use_fake_user = True
        rig.animation_data.action = action
        if name == profile["default_action"]: default_action = action
        poses = []
        for key_frame, source in enumerate(clip_frames(anim, index, count)):
            apply_pose(rig, cfg, anim, source)
            poses.append(tuple(tuple(round(v, 4) for v in bone.rotation_euler) for bone in rig.pose.bones))
            for bone in rig.pose.bones: bone.keyframe_insert(data_path="rotation_euler", frame=key_frame, group=bone.name)
            rig.keyframe_insert(data_path="location", frame=key_frame, group="Root")
            rig.keyframe_insert(data_path="rotation_euler", frame=key_frame, group="Root")
        # A clip whose keys are identical is a frozen pose wearing a clip name.
        if len(poses) > 1 and poses[0] == poses[-1]:
            raise SystemExit(f"clip {name} ({anim} beat {index} of {count}) is a frozen pose")
    rig.animation_data.action = default_action
    bpy.context.scene.frame_set(0)




def action_profile(cfg: dict) -> dict:
    profile_id = ("boss_" if cfg["kind"] == "boss" else "npc_") + NPC_NAME.lower() + "_" + cfg["weapon"]
    profile = {"id": profile_id, "weapon": cfg["weapon"], "default_action": cfg["clips"]["idle"][0], "clips": cfg["clips"]}
    if "boss" in cfg: profile["boss"] = cfg["boss"]
    return profile


def render_frames(rig: bpy.types.Object, turntable: bpy.types.Object, cfg: dict) -> None:
    scene = bpy.context.scene
    directions = DIRECTIONS[:1] if PROBE else DIRECTIONS
    # An active action re-evaluates the pose bones on every depsgraph update and
    # would overwrite the hand-applied pose, so render from a detached rig.
    rig.animation_data.action = None
    for animation, count in render_counts(cfg).items():
        for direction, angle in directions:
            target = ROOT / "render" / NPC_NAME / animation / direction
            target.mkdir(parents=True, exist_ok=True)
            for frame in range(count):
                turntable.rotation_euler = (0, 0, angle)
                apply_pose(rig, cfg, animation, frame)
                scene.render.filepath = str(target / f"{frame:03d}.png")
                bpy.ops.render.render(write_still=True)


def main() -> None:
    if NPC_NAME not in PROFILES: raise SystemExit(f"unknown NPC: {NPC_NAME}")
    cfg = PROFILES[NPC_NAME]; bpy.ops.wm.read_factory_settings(use_empty=True); base.add_scene(); tune_scene(); mats = make_materials(NPC_NAME, cfg)
    if STUDY_ANY:
        scene = bpy.context.scene
        scene.render.resolution_x, scene.render.resolution_y = CELL
        camera = scene.camera
        camera.data.ortho_scale = max(
            {"wolf": 5.0, "stag": 4.6}.get(cfg["form"], 4.2),
            (4.3 if cfg["form"] == "rat" else 3.9) * cfg["scale"],
        )
        camera.location = (0, -8, 4.7)
        target = Vector((0, 0, 1.55 * cfg["scale"]))
        camera.rotation_euler = (target - camera.location).to_track_quat("-Z", "Y").to_euler()
    arms = {}; legs = {}
    if STUDY_ANY and cfg["form"] in NON_HUMANOID:
        import npc_roster_visual_v2_creatures
        if cfg["form"] == "wisp":
            npc_roster_visual_v2_creatures.add_wisp_body(NPC_NAME, cfg, mats, sys.modules[__name__])
        else:
            legs = npc_roster_visual_v2_creatures.add_beast_body(NPC_NAME, cfg, mats, sys.modules[__name__])
    elif cfg["form"] == "wisp": add_wisp_body(NPC_NAME, cfg, mats)
    elif cfg["form"] in BEAST_FORMS: legs = add_beast_body(NPC_NAME, cfg, mats)
    else:
        if STUDY_V3:
            import npc_roster_visual_v3
            arms, legs = npc_roster_visual_v3.add_humanoid_body(NPC_NAME, cfg, mats, sys.modules[__name__])
        elif STUDY_V2:
            import npc_roster_visual_v2
            arms, legs = npc_roster_visual_v2.add_humanoid_body(NPC_NAME, cfg, mats, sys.modules[__name__])
        else:
            arms, legs = add_humanoid_body(NPC_NAME, cfg, mats)
    if cfg["form"] not in NON_HUMANOID:
        if STUDY_V3:
            npc_roster_visual_v3.add_weapon(NPC_NAME, cfg, mats, cfg["scale"], sys.modules[__name__])
        elif STUDY_V2:
            npc_roster_visual_v2.add_weapon(NPC_NAME, cfg, mats, cfg["scale"], sys.modules[__name__])
        else:
            add_weapon(NPC_NAME, cfg, mats, cfg["scale"])
    add_markers(NPC_NAME); rig = add_rig(NPC_NAME, cfg, arms, legs, mats); turntable = add_turntable(rig); build_action_library(rig, cfg)
    blend_path = ROOT / "blender" / f"{NPC_NAME}.blend"; blend_path.parent.mkdir(parents=True, exist_ok=True); bpy.ops.wm.save_as_mainfile(filepath=str(blend_path)); render_frames(rig, turntable, cfg)
    manifest = {"version": 1, "assets": [{"id": NPC_NAME, "kind": cfg["kind"], "source": f"source/{NPC_NAME}.glb", "render_root": f"render/{NPC_NAME}", "cell_size": {"width": CELL[0], "height": CELL[1]}, "directions": [n for n, _ in DIRECTIONS], "animations": render_counts(cfg), "action_profile": action_profile(cfg), "uvs": True, "materials": True, "rig": True, "pivot": "feet_center", "metadata": {"source_kind": "blender_5.1_roster_visual_v3" if STUDY_V3 else ("blender_5.1_roster_visual_v2" if STUDY_V2 else "blender_5.1_roster_v1"), "front_axis": "-Y"}}]}
    (ROOT / f"{NPC_NAME.lower()}_manifest.json").write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8"); print("NPC_ROSTER_VISUAL_V3" if STUDY_V3 else ("NPC_ROSTER_VISUAL_V2" if STUDY_V2 else "NPC_ROSTER_V1"), NPC_NAME, blend_path)


if __name__ == "__main__":
    main()
