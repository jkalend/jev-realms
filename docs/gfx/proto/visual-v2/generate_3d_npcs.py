"""PROTOTYPE: true 3D NPC studies for the visual direction board.

This is deliberately outside the production atlas/game code. It builds simple
low-poly 3D meshes, renders them with a tiny software orthographic z-buffer,
and compares the result with the approved 2D and archived 2.5D sprites.

Run from the repository root:
    python docs/gfx/proto/visual-v2/generate_3d_npcs.py
"""

from __future__ import annotations

import math
import random
from pathlib import Path

from PIL import Image, ImageChops, ImageDraw, ImageFilter

import generate as proto

OUT = Path(__file__).resolve().parent
ARCHIVE = OUT / "archive/actors-2d5d"
RNG = random.Random(0x3DCA)

# Low-poly material palette. Values are deliberately graphic rather than
# photorealistic; the point is readable form, silhouette and light direction.
MATERIALS = {
    "skin": (181, 121, 87),
    "skin_light": (211, 151, 111),
    "skin_dark": (116, 72, 56),
    "hair": (73, 45, 34),
    "hair_light": (132, 83, 48),
    "cloth": (91, 104, 111),
    "cloth_light": (137, 148, 148),
    "cloth_dark": (48, 57, 64),
    "blue": (53, 91, 143),
    "blue_light": (91, 137, 181),
    "red": (139, 49, 55),
    "red_light": (190, 75, 68),
    "teal": (43, 123, 126),
    "teal_light": (89, 183, 175),
    "gold": (198, 151, 61),
    "gold_light": (239, 201, 102),
    "steel": (105, 117, 125),
    "steel_light": (181, 193, 193),
    "leather": (83, 52, 37),
    "leather_light": (132, 83, 49),
    "bone": (210, 204, 176),
    "bone_dark": (128, 127, 113),
    "black": (24, 27, 31),
    "void": (12, 14, 18),
    "moss": (63, 84, 48),
}

W, H = 180, 220
SS = 2
BW, BH = W * SS, H * SS


def clamp(v: float) -> int:
    return max(0, min(255, int(round(v))))


def normalize(v: tuple[float, float, float]) -> tuple[float, float, float]:
    length = math.sqrt(sum(x * x for x in v)) or 1.0
    return tuple(x / length for x in v)  # type: ignore[return-value]


def cross(a: tuple[float, float, float], b: tuple[float, float, float]) -> tuple[float, float, float]:
    return (a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0])


def dot(a: tuple[float, float, float], b: tuple[float, float, float]) -> float:
    return sum(x * y for x, y in zip(a, b))


def rotate_y(p: tuple[float, float, float], angle: float) -> tuple[float, float, float]:
    c, s = math.cos(angle), math.sin(angle)
    return (p[0] * c + p[2] * s, p[1], -p[0] * s + p[2] * c)


class Mesh:
    def __init__(self) -> None:
        self.vertices: list[tuple[float, float, float]] = []
        self.faces: list[tuple[int, ...]] = []
        self.materials: list[str] = []

    def vertex(self, p: tuple[float, float, float]) -> int:
        self.vertices.append(p)
        return len(self.vertices) - 1

    def face(self, indices: list[int] | tuple[int, ...], material: str) -> None:
        self.faces.append(tuple(indices))
        self.materials.append(material)


def add_box(mesh: Mesh, center: tuple[float, float, float], size: tuple[float, float, float], material: str, yaw: float = 0.0) -> None:
    cx, cy, cz = center
    sx, sy, sz = (v / 2 for v in size)
    local = [
        (-sx, -sy, -sz), (sx, -sy, -sz), (sx, sy, -sz), (-sx, sy, -sz),
        (-sx, -sy, sz), (sx, -sy, sz), (sx, sy, sz), (-sx, sy, sz),
    ]
    ids = [mesh.vertex((cx + x, cy + y, cz + z)) for x, y, z in (rotate_y(p, yaw) for p in local)]
    for face in ((0, 3, 2, 1), (4, 5, 6, 7), (0, 4, 7, 3), (1, 2, 6, 5), (0, 1, 5, 4), (3, 7, 6, 2)):
        mesh.face([ids[i] for i in face], material)


def add_frustum(mesh: Mesh, y0: float, y1: float, bottom: tuple[float, float], top: tuple[float, float], material: str, z: float = 0.0) -> None:
    bw, bd = bottom[0] / 2, bottom[1] / 2
    tw, td = top[0] / 2, top[1] / 2
    lower = [mesh.vertex(p) for p in ((-bw, y0, z - bd), (bw, y0, z - bd), (bw, y0, z + bd), (-bw, y0, z + bd))]
    upper = [mesh.vertex(p) for p in ((-tw, y1, z - td), (tw, y1, z - td), (tw, y1, z + td), (-tw, y1, z + td))]
    mesh.face(lower, material)
    mesh.face(list(reversed(upper)), material)
    for i in range(4):
        j = (i + 1) % 4
        mesh.face([lower[i], lower[j], upper[j], upper[i]], material)


def add_beam(mesh: Mesh, a: tuple[float, float, float], b: tuple[float, float, float], width: float, depth: float, material: str, sides: int = 4) -> None:
    axis = normalize((b[0] - a[0], b[1] - a[1], b[2] - a[2]))
    ref = (0.0, 1.0, 0.0) if abs(axis[1]) < 0.9 else (1.0, 0.0, 0.0)
    u = normalize(cross(axis, ref))
    v = normalize(cross(axis, u))
    rings = []
    for center in (a, b):
        ring = []
        for i in range(sides):
            angle = math.tau * i / sides
            offset = tuple(u[k] * math.cos(angle) * width / 2 + v[k] * math.sin(angle) * depth / 2 for k in range(3))
            ring.append(mesh.vertex((center[0] + offset[0], center[1] + offset[1], center[2] + offset[2])))
        rings.append(ring)
    mesh.face(list(reversed(rings[0])), material)
    mesh.face(rings[1], material)
    for i in range(sides):
        j = (i + 1) % sides
        mesh.face([rings[0][i], rings[0][j], rings[1][j], rings[1][i]], material)


def add_ellipsoid(mesh: Mesh, center: tuple[float, float, float], radii: tuple[float, float, float], material: str, segments: int = 10, rings: int = 5) -> None:
    cx, cy, cz = center
    rx, ry, rz = radii
    top = mesh.vertex((cx, cy + ry, cz))
    bottom = mesh.vertex((cx, cy - ry, cz))
    rows: list[list[int]] = []
    for j in range(1, rings):
        theta = math.pi * j / rings
        row = []
        for i in range(segments):
            phi = math.tau * i / segments
            row.append(mesh.vertex((cx + rx * math.sin(theta) * math.cos(phi), cy + ry * math.cos(theta), cz + rz * math.sin(theta) * math.sin(phi))))
        rows.append(row)
    for i in range(segments):
        j = (i + 1) % segments
        mesh.face([top, rows[0][i], rows[0][j]], material)
        mesh.face([bottom, rows[-1][j], rows[-1][i]], material)
    for r in range(len(rows) - 1):
        for i in range(segments):
            j = (i + 1) % segments
            mesh.face([rows[r][i], rows[r + 1][i], rows[r + 1][j], rows[r][j]], material)


def add_cone(mesh: Mesh, center: tuple[float, float, float], radius: float, height: float, material: str, segments: int = 8) -> None:
    cx, cy, cz = center
    bottom = [mesh.vertex((cx + radius * math.cos(math.tau * i / segments), cy, cz + radius * math.sin(math.tau * i / segments))) for i in range(segments)]
    top = mesh.vertex((cx, cy + height, cz))
    for i in range(segments):
        mesh.face([bottom[i], bottom[(i + 1) % segments], top], material)


def add_cape(mesh: Mesh, material: str) -> None:
    front = [mesh.vertex(p) for p in ((-0.48, 2.28, -0.26), (0.48, 2.28, -0.26), (0.40, 1.18, -0.34), (-0.40, 1.18, -0.34))]
    back = [mesh.vertex((x, y, z - 0.07)) for x, y, z in ((-0.48, 2.28, -0.26), (0.48, 2.28, -0.26), (0.40, 1.18, -0.34), (-0.40, 1.18, -0.34))]
    mesh.face(front, material)
    mesh.face(list(reversed(back)), material)
    for i in range(4):
        j = (i + 1) % 4
        mesh.face([front[i], front[j], back[j], back[i]], material)


def add_face_details(mesh: Mesh, y: float = 2.76, z: float = 0.28) -> None:
    add_box(mesh, (-0.115, y + 0.07, z), (0.075, 0.055, 0.035), "black")
    add_box(mesh, (0.115, y + 0.07, z), (0.075, 0.055, 0.035), "black")
    add_box(mesh, (0.0, y - 0.01, z + 0.025), (0.07, 0.13, 0.06), "skin_light")
    add_box(mesh, (0.0, y - 0.16, z + 0.01), (0.16, 0.035, 0.025), "skin_dark")


def humanoid_base(mesh: Mesh, cloth: str, accent: str, skin: str = "skin", armored: bool = False, robe: bool = False) -> None:
    if robe:
        add_cone(mesh, (0, 0.18, 0), 0.56, 2.15, cloth, 8)
        add_ellipsoid(mesh, (0, 0.12, 0.13), (0.25, 0.12, 0.34), "leather")
    else:
        for side in (-1, 1):
            hip = (side * 0.25, 1.28, 0.0)
            knee = (side * 0.30, 0.73, 0.04)
            ankle = (side * 0.32, 0.16, 0.10)
            add_beam(mesh, hip, knee, 0.24, 0.25, cloth)
            add_beam(mesh, knee, ankle, 0.21, 0.23, cloth)
            add_box(mesh, (ankle[0], 0.10, ankle[2] + 0.06), (0.28, 0.18, 0.42), "leather", yaw=side * 0.08)
        add_frustum(mesh, 1.18, 2.34, (0.78, 0.46), (0.86, 0.50), cloth)
    add_box(mesh, (0, 1.26, 0.02), (0.82, 0.12, 0.52), "leather")
    add_box(mesh, (0, 1.26, 0.29), (0.15, 0.13, 0.06), accent)
    add_ellipsoid(mesh, (-0.43, 2.22, 0), (0.25, 0.25, 0.25), cloth if not armored else "steel")
    add_ellipsoid(mesh, (0.43, 2.22, 0), (0.25, 0.25, 0.25), cloth if not armored else "steel")
    for side in (-1, 1):
        shoulder = (side * 0.46, 2.18, 0.0)
        elbow = (side * 0.61, 1.72, 0.10)
        hand = (side * 0.66, 1.34, 0.18)
        add_beam(mesh, shoulder, elbow, 0.22, 0.24, cloth)
        add_beam(mesh, elbow, hand, 0.17, 0.19, skin)
        add_ellipsoid(mesh, hand, (0.13, 0.15, 0.13), skin, 8, 4)
    add_beam(mesh, (0, 2.30, 0), (0, 2.53, 0), 0.22, 0.22, skin, 6)
    add_ellipsoid(mesh, (0, 2.77, 0), (0.34, 0.40, 0.31), skin, 10, 5)
    add_ellipsoid(mesh, (-0.35, 2.77, 0), (0.08, 0.13, 0.10), skin, 8, 4)
    add_ellipsoid(mesh, (0.35, 2.77, 0), (0.08, 0.13, 0.10), skin, 8, 4)
    add_face_details(mesh)
    if armored:
        add_box(mesh, (0, 1.82, 0.28), (0.60, 0.65, 0.08), "steel")
        add_box(mesh, (0, 1.81, 0.34), (0.12, 0.50, 0.04), "gold")


def add_hair(mesh: Mesh, material: str = "hair") -> None:
    add_ellipsoid(mesh, (0, 2.99, -0.04), (0.36, 0.25, 0.32), material, 8, 4)


def add_hood(mesh: Mesh, material: str = "cloth_dark") -> None:
    add_ellipsoid(mesh, (0, 2.82, -0.12), (0.44, 0.48, 0.39), material, 9, 5)
    add_box(mesh, (0, 2.76, 0.30), (0.45, 0.33, 0.06), material)
    add_box(mesh, (0, 2.58, 0.33), (0.38, 0.06, 0.05), "black")


def add_helmet(mesh: Mesh) -> None:
    add_ellipsoid(mesh, (0, 2.96, -0.02), (0.40, 0.30, 0.36), "steel", 8, 4)
    add_box(mesh, (0, 2.72, 0.30), (0.60, 0.08, 0.10), "steel_light")
    add_box(mesh, (0, 3.20, -0.02), (0.09, 0.30, 0.28), "red")
    add_box(mesh, (-0.31, 2.77, 0.10), (0.09, 0.35, 0.30), "steel")
    add_box(mesh, (0.31, 2.77, 0.10), (0.09, 0.35, 0.30), "steel")


def add_spear(mesh: Mesh) -> None:
    add_beam(mesh, (0.78, 0.10, 0.30), (0.78, 3.18, 0.30), 0.07, 0.07, "leather", 6)
    add_beam(mesh, (0.78, 2.92, 0.30), (0.78, 3.48, 0.30), 0.12, 0.08, "steel_light", 4)
    add_box(mesh, (0.78, 3.47, 0.30), (0.16, 0.28, 0.06), "steel_light")


def add_staff(mesh: Mesh) -> None:
    add_beam(mesh, (0.78, 0.10, 0.22), (0.78, 3.20, 0.22), 0.07, 0.07, "leather", 6)
    add_ellipsoid(mesh, (0.78, 3.32, 0.22), (0.19, 0.19, 0.19), "teal_light", 8, 4)
    add_ellipsoid(mesh, (0.78, 3.32, 0.22), (0.08, 0.08, 0.08), "gold_light", 8, 4)


def add_shield(mesh: Mesh) -> None:
    add_box(mesh, (-0.73, 1.65, 0.22), (0.13, 0.82, 0.50), "blue", yaw=-0.12)
    add_box(mesh, (-0.81, 1.65, 0.24), (0.04, 0.56, 0.32), "steel_light", yaw=-0.12)
    add_box(mesh, (-0.84, 1.65, 0.39), (0.03, 0.18, 0.18), "gold_light", yaw=-0.12)


def add_dagger(mesh: Mesh) -> None:
    add_beam(mesh, (0.74, 1.30, 0.27), (0.98, 1.92, 0.30), 0.10, 0.06, "steel_light", 4)
    add_box(mesh, (0.70, 1.25, 0.27), (0.24, 0.07, 0.10), "gold")


def add_hammer(mesh: Mesh) -> None:
    add_beam(mesh, (0.78, 0.20, 0.22), (0.78, 2.30, 0.22), 0.09, 0.09, "leather", 6)
    add_box(mesh, (0.78, 2.40, 0.22), (0.65, 0.28, 0.34), "steel")
    add_box(mesh, (0.78, 2.40, 0.41), (0.22, 0.12, 0.04), "gold_light")


def make_commoner() -> Mesh:
    mesh = Mesh()
    humanoid_base(mesh, "cloth", "gold", "skin")
    add_hair(mesh)
    add_box(mesh, (0.47, 1.36, -0.10), (0.28, 0.38, 0.25), "leather", yaw=-0.18)
    add_box(mesh, (0.47, 1.54, 0.02), (0.30, 0.06, 0.26), "leather_light", yaw=-0.18)
    add_box(mesh, (0, 2.31, 0.18), (0.54, 0.12, 0.34), "red")
    return mesh


def make_guard() -> Mesh:
    mesh = Mesh()
    humanoid_base(mesh, "blue", "gold_light", "skin", armored=True)
    add_helmet(mesh)
    add_cape(mesh, "red")
    add_shield(mesh)
    add_spear(mesh)
    return mesh


def make_bandit() -> Mesh:
    mesh = Mesh()
    humanoid_base(mesh, "red", "gold", "skin")
    add_hood(mesh, "red")
    add_box(mesh, (0, 2.72, 0.37), (0.30, 0.15, 0.05), "black")
    add_dagger(mesh)
    add_box(mesh, (-0.44, 1.48, 0.26), (0.22, 0.26, 0.08), "leather", yaw=0.15)
    return mesh


def make_oracle() -> Mesh:
    mesh = Mesh()
    humanoid_base(mesh, "teal", "gold_light", "skin", robe=True)
    add_hood(mesh, "teal")
    add_staff(mesh)
    add_box(mesh, (-0.34, 1.72, 0.20), (0.22, 0.36, 0.06), "gold", yaw=0.20)
    add_box(mesh, (-0.34, 1.72, 0.24), (0.08, 0.22, 0.03), "teal_light", yaw=0.20)
    return mesh


def make_skeleton() -> Mesh:
    mesh = Mesh()
    for side in (-1, 1):
        add_beam(mesh, (side * 0.22, 1.25, 0), (side * 0.28, 0.68, 0.02), 0.15, 0.15, "bone")
        add_beam(mesh, (side * 0.28, 0.68, 0.02), (side * 0.30, 0.12, 0.08), 0.13, 0.13, "bone")
        add_beam(mesh, (side * 0.36, 2.14, 0), (side * 0.53, 1.68, 0.08), 0.13, 0.13, "bone")
        add_beam(mesh, (side * 0.53, 1.68, 0.08), (side * 0.60, 1.28, 0.16), 0.11, 0.11, "bone")
    add_beam(mesh, (0, 1.18, 0), (0, 2.18, 0), 0.20, 0.20, "bone", 6)
    for y in (1.58, 1.75, 1.92, 2.09):
        add_beam(mesh, (-0.32, y, 0.19), (0.32, y, 0.19), 0.09, 0.09, "bone", 4)
    add_ellipsoid(mesh, (0, 2.60, 0), (0.35, 0.38, 0.30), "bone", 10, 5)
    add_box(mesh, (-0.12, 2.65, 0.28), (0.10, 0.12, 0.05), "black")
    add_box(mesh, (0.12, 2.65, 0.28), (0.10, 0.12, 0.05), "black")
    add_box(mesh, (0, 2.40, 0.28), (0.18, 0.06, 0.04), "bone_dark")
    for side in (-1, 1):
        add_ellipsoid(mesh, (side * 0.35, 2.61, 0), (0.08, 0.13, 0.10), "bone", 8, 4)
    return mesh


def make_wolf() -> Mesh:
    mesh = Mesh()
    fur = "steel"
    add_ellipsoid(mesh, (0, 1.05, 0), (0.82, 0.44, 0.38), fur, 10, 5)
    add_ellipsoid(mesh, (0.82, 1.35, 0), (0.40, 0.38, 0.34), "steel_light", 9, 5)
    add_beam(mesh, (1.08, 1.28, 0), (1.40, 1.15, 0), 0.28, 0.24, "steel_light", 6)
    add_box(mesh, (1.40, 1.15, 0), (0.22, 0.18, 0.18), "black")
    for side in (-1, 1):
        add_cone(mesh, (0.65, 1.73, side * 0.16), 0.14, 0.36, "steel_light", 5)
        add_beam(mesh, (side * 0.48, 0.90, side * 0.18), (side * 0.56, 0.16, side * 0.20), 0.18, 0.18, "steel", 5)
        add_beam(mesh, (side * 0.45, 0.95, side * 0.28), (side * 0.52, 0.20, side * 0.30), 0.12, 0.12, "steel_light", 5)
    add_beam(mesh, (-0.76, 1.08, 0), (-1.24, 1.42, 0), 0.18, 0.18, "steel", 5)
    add_box(mesh, (0.76, 1.48, 0.20), (0.10, 0.10, 0.05), "gold")
    return mesh


def make_tidemother() -> Mesh:
    mesh = Mesh()
    humanoid_base(mesh, "teal", "gold_light", "skin", robe=True)
    add_ellipsoid(mesh, (0, 2.91, -0.08), (0.43, 0.27, 0.35), "teal_light", 8, 4)
    for i in range(5):
        add_box(mesh, (-0.28 + i * 0.14, 3.18, 0), (0.06, 0.25 + (i % 2) * 0.08, 0.06), "gold_light")
    add_cape(mesh, "teal")
    add_staff(mesh)
    for side in (-1, 1):
        add_beam(mesh, (side * 0.40, 1.22, -0.05), (side * 0.54, 0.62, -0.02), 0.10, 0.12, "moss", 5)
    return mesh


def make_tollmaster() -> Mesh:
    mesh = Mesh()
    humanoid_base(mesh, "cloth_dark", "gold_light", "skin", armored=True)
    add_helmet(mesh)
    add_cape(mesh, "red")
    add_hammer(mesh)
    add_box(mesh, (0, 1.83, 0.34), (0.18, 0.60, 0.05), "gold")
    return mesh


MODELS = {
    "Commoner": make_commoner,
    "Guard": make_guard,
    "Bandit": make_bandit,
    "Oracle": make_oracle,
    "Skeleton": make_skeleton,
    "Wolf": make_wolf,
    "Tidemother": make_tidemother,
    "Tollmaster": make_tollmaster,
}

NPC_CELLS = {
    "Commoner": (0, 0),
    "Guard": (2, 0),
    "Bandit": (5, 0),
    "Wolf": (6, 0),
    "Skeleton": (1, 1),
    "Oracle": (7, 1),
    "Tidemother": (1, 2),
    "Tollmaster": (4, 2),
}


def face_normal(points: list[tuple[float, float, float]], indices: tuple[int, ...]) -> tuple[float, float, float]:
    a, b, c = (points[indices[0]], points[indices[1]], points[indices[2]])
    return normalize(cross((b[0] - a[0], b[1] - a[1], b[2] - a[2]), (c[0] - a[0], c[1] - a[1], c[2] - a[2])))


def material_color(material: str, normal: tuple[float, float, float], face_index: int) -> tuple[int, int, int]:
    base = MATERIALS[material]
    light = normalize((-0.42, 0.78, 0.56))
    view = (0.0, 0.0, 1.0)
    n = normal if dot(normal, light) >= 0 else tuple(-x for x in normal)
    diffuse = max(0.0, dot(n, light))
    half = normalize((light[0] + view[0], light[1] + view[1], light[2] + view[2]))
    spec = max(0.0, dot(n, half)) ** (18 if material in {"steel", "steel_light", "gold", "gold_light"} else 8)
    factor = 0.38 + diffuse * 0.80
    if material in {"steel", "steel_light", "gold", "gold_light"}:
        factor += spec * 0.35
    variation = 0.96 + ((face_index * 17) % 7) * 0.012
    return tuple(clamp(channel * factor * variation) for channel in base)  # type: ignore[return-value]


def raster_triangle(p0: tuple[float, float, float], p1: tuple[float, float, float], p2: tuple[float, float, float],
                    color: tuple[int, int, int], zbuf: list[float], pixels: list[tuple[int, int, int, int]]) -> None:
    area = (p1[0] - p0[0]) * (p2[1] - p0[1]) - (p1[1] - p0[1]) * (p2[0] - p0[0])
    if abs(area) < 0.0001:
        return
    min_x = max(0, int(math.floor(min(p0[0], p1[0], p2[0]))))
    max_x = min(BW - 1, int(math.ceil(max(p0[0], p1[0], p2[0]))))
    min_y = max(0, int(math.floor(min(p0[1], p1[1], p2[1]))))
    max_y = min(BH - 1, int(math.ceil(max(p0[1], p1[1], p2[1]))))
    inv_area = 1.0 / area
    for y in range(min_y, max_y + 1):
        for x in range(min_x, max_x + 1):
            w0 = ((p1[0] - x) * (p2[1] - y) - (p1[1] - y) * (p2[0] - x)) * inv_area
            w1 = ((p2[0] - x) * (p0[1] - y) - (p2[1] - y) * (p0[0] - x)) * inv_area
            w2 = 1.0 - w0 - w1
            if w0 < -0.001 or w1 < -0.001 or w2 < -0.001:
                continue
            depth = p0[2] * w0 + p1[2] * w1 + p2[2] * w2
            index = y * BW + x
            if depth <= zbuf[index]:
                continue
            zbuf[index] = depth
            pixels[index] = (*color, 255)


def render_mesh(mesh: Mesh, yaw: float = -18.0, pitch: float = 12.0, scale: float = 42.0) -> Image.Image:
    cy, sy = math.cos(math.radians(yaw)), math.sin(math.radians(yaw))
    cp, sp = math.cos(math.radians(pitch)), math.sin(math.radians(pitch))
    center_x, center_y = W * 0.5 * SS, H * 0.86 * SS
    transformed = []
    for x, y, z in mesh.vertices:
        x1 = x * cy + z * sy
        z1 = -x * sy + z * cy
        y2 = y * cp - z1 * sp
        z2 = -y * sp + z1 * cp
        transformed.append((center_x + x1 * scale * SS, center_y - y2 * scale * SS, z2))

    zbuf = [-1e9] * (BW * BH)
    pixels = [(0, 0, 0, 0)] * (BW * BH)
    for face_index, (indices, material) in enumerate(zip(mesh.faces, mesh.materials)):
        normal = face_normal(transformed, indices)
        color = material_color(material, normal, face_index)
        points = [transformed[i] for i in indices]
        for i in range(1, len(points) - 1):
            raster_triangle(points[0], points[i], points[i + 1], color, zbuf, pixels)
    big = Image.new("RGBA", (BW, BH), (0, 0, 0, 0))
    big.putdata(pixels)
    image = big.resize((W, H), Image.Resampling.LANCZOS)

    # Crisp dark silhouette only around the outside; internal mesh edges remain
    # soft material planes instead of black wireframe.
    alpha = image.getchannel("A")
    edge = ImageChops.subtract(alpha.filter(ImageFilter.MaxFilter(5)), alpha)
    outline = Image.new("RGBA", (W, H), (5, 7, 10, 0))
    outline.putalpha(edge.point(lambda a: a * 185 // 255))
    image = Image.alpha_composite(outline, image)

    shadow = Image.new("RGBA", (W, H), (0, 0, 0, 0))
    sd = ImageDraw.Draw(shadow)
    sd.ellipse((W * 0.28, H * 0.86, W * 0.72, H * 0.94), fill=(2, 3, 5, 150))
    shadow = shadow.filter(ImageFilter.GaussianBlur(4.0))
    return proto.trim(Image.alpha_composite(shadow, image), 3)


def render_npc(name: str, yaw: float = -18.0) -> Image.Image:
    return render_mesh(MODELS[name](), yaw=yaw)


def sprite_from_sheet(sheet: Image.Image, col: int, row: int) -> Image.Image:
    return sheet.crop((col * 96, row * 96, col * 96 + 96, row * 96 + 96))


def fit(dst: Image.Image, src: Image.Image, box: tuple[int, int, int, int], nearest: bool = False) -> None:
    x0, y0, x1, y1 = box
    copy = src.copy()
    copy.thumbnail((x1 - x0, y1 - y0), Image.Resampling.NEAREST if nearest else Image.Resampling.LANCZOS)
    px = x0 + (x1 - x0 - copy.width) // 2
    py = y0 + (y1 - y0 - copy.height) // 2
    dst.alpha_composite(copy, (px, py))


def build_contact_sheet() -> Image.Image:
    names = list(MODELS)
    with Image.open(proto.CONTACT_2D) as opened:
        contact = opened.convert("RGBA")
    background = proto.build_grass_background(contact)
    with Image.open(ARCHIVE / "actors-2d5d-sheet.png") as opened:
        sheet_25 = opened.convert("RGBA")

    canvas = Image.new("RGBA", (1900, 1660), (12, 14, 18, 255))
    d = ImageDraw.Draw(canvas)
    d.rectangle((0, 0, 1900, 138), fill=(8, 10, 14, 255))
    proto.draw_text(d, (54, 28), "TRUE 3D NPC STUDY", 38, proto.INK, True)
    proto.draw_text(d, (56, 84), "Low-poly mesh render / directional camera / material and silhouette test", 16, proto.MUTED)
    proto.draw_text(d, (1844, 42), "PROTOTYPE 01", 15, proto.GOLD, True, anchor="ra")

    columns = [(160, 535, "APPROVED 2D"), (650, 1025, "ARCHIVED 2.5D"), (1140, 1515, "TRUE 3D MESH")]
    for x0, x1, label in columns:
        d.rounded_rectangle((x0, 170, x1, 220), 6, fill=(30, 32, 37, 255), outline=(74, 67, 49, 255), width=2)
        proto.draw_text(d, ((x0 + x1) // 2, 195), label, 15, proto.GOLD, True, anchor="mm")

    row_h = 145
    for i, name in enumerate(names):
        col, row = NPC_CELLS[name]
        y0 = 238 + i * row_h
        y1 = y0 + row_h - 10
        d.rounded_rectangle((54, y0, 1846, y1), 7, fill=(22, 24, 29, 255), outline=(45, 46, 49, 255), width=1)
        d.text((72, y0 + 53), name, font=proto.font(15, mono=True), fill=proto.MUTED)
        approved = proto.extract_2d_sprite(contact, background, col, row)
        archived = sprite_from_sheet(sheet_25, col, row)
        new = render_npc(name)
        for src, column in zip((approved, archived, new), columns):
            x0, x1 = column[0], column[1]
            fit(canvas, src, (x0, y0 + 3, x1, y1 - 3), nearest=True)

    y0 = 238 + len(names) * row_h + 8
    d.rectangle((54, y0, 1846, 1626), fill=(16, 19, 24, 255), outline=(67, 61, 46, 255), width=2)
    proto.draw_text(d, (78, y0 + 20), "TURN TABLE / GUARD", 17, proto.GOLD, True)
    for i, yaw in enumerate((-30.0, 0.0, 30.0)):
        x = 430 + i * 420
        fit(canvas, render_npc("Guard", yaw), (x - 155, y0 + 48, x + 155, y0 + 300), nearest=False)
        proto.draw_text(d, (x, y0 + 315), f"YAW {yaw:+.0f}°", 12, proto.MUTED, mono=True, anchor="ma")
    proto.draw_text(d, (78, y0 + 115), "The 3D pass keeps the same low-poly / slightly grim read while making rotation, occlusion and silhouette construction explicit.", 14, proto.MUTED)
    proto.draw_text(d, (78, y0 + 160), "Next issue: replace the capsule-like rig construction with authored NPC meshes while preserving these color and silhouette cues.", 14, (184, 153, 82))
    return canvas.convert("RGB")


def build_world_pass() -> Image.Image:
    # The base is the real-map, actor-free seamless environment. The 3D NPC
    # sprites are then composited at the same projected map coordinates.
    base = proto.make_environment_scene({}, "top").convert("RGBA")
    fixture = proto.load_map_fixture()
    left, top, right, bottom = 4, 9, 21, 26
    tw, th = 78, 45
    player = fixture["player"]["pos"]
    origin = (800, 520 - ((player["x"] - left) + (player["y"] - top)) * th / 2)

    cast = [(player["x"], player["y"], "Guard", True)]
    visible = []
    for npc in fixture["npcs"]:
        x, y = npc["pos"]["x"], npc["pos"]["y"]
        if left <= x <= right and top <= y <= bottom and npc["archetype"] in MODELS:
            visible.append(((x - player["x"]) ** 2 + (y - player["y"]) ** 2, x, y, npc["archetype"]))
    for _, x, y, name in sorted(visible)[:7]:
        cast.append((x, y, name, False))

    for x, y, name, is_player in sorted(cast, key=lambda item: item[0] + item[1]):
        sx, sy = proto.world_xy(x - left, y - top, origin, tw, th)
        sprite = render_npc(name, yaw=-18.0)
        scale = 0.52 if is_player else 0.46
        target = (round(sprite.width * scale), round(sprite.height * scale))
        sprite = sprite.resize(target, Image.Resampling.LANCZOS)
        px = round(sx - target[0] / 2)
        py = round(sy - target[1] + 4)
        if is_player:
            ring = Image.new("RGBA", base.size, (0, 0, 0, 0))
            rd = ImageDraw.Draw(ring)
            rd.ellipse((sx - 17, sy - 5, sx + 17, sy + 7), fill=(231, 192, 102, 105))
            base.alpha_composite(ring)
        base.alpha_composite(sprite, (px, py))

    overlay = Image.new("RGBA", base.size, (0, 0, 0, 0))
    od = ImageDraw.Draw(overlay)
    od.rounded_rectangle((54, 48, 610, 116), radius=5, fill=(13, 15, 19, 224), outline=(105, 82, 44, 225), width=1)
    proto.draw_text2(od, (76, 65), "TRUE 3D NPC PASS", 18, proto.INK + (255,), True)
    proto.draw_text2(od, (76, 94), "SAME REAL MAP / SAME TOP CAMERA / NEW MESH RENDERS", 10, (182, 150, 76, 255), mono=True)
    base.alpha_composite(overlay)
    return base.convert("RGB")


def main() -> None:
    contact = build_contact_sheet()
    contact.save(OUT / "npc-3d-contact.png", quality=95)
    world = build_world_pass()
    world.save(OUT / "npc-3d-world-v1.png", quality=95)
    print("generated true 3D NPC prototype outputs in", OUT)


if __name__ == "__main__":
    main()
