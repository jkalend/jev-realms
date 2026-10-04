"""PROTOTYPE: multi-angle true 3D boss studies.

Reuses the standalone mesh renderer from generate_3d_npcs.py. Each boss is
rendered from four yaw angles so silhouette, equipment and rotation can be
judged before any production atlas or engine change.
"""

from __future__ import annotations

import math
from pathlib import Path

from PIL import Image, ImageDraw

import generate as proto
import generate_3d_npcs as npc

OUT = Path(__file__).resolve().parent

# Additional boss-only materials, kept local to this prototype.
npc.MATERIALS.update({
    "violet": (91, 55, 119),
    "violet_light": (163, 106, 184),
    "rock": (103, 101, 89),
    "rock_light": (139, 132, 108),
    "moss_light": (83, 105, 54),
    "blood_red": (126, 37, 43),
    "bone_blue": (177, 190, 184),
})


def scale_mesh(mesh: npc.Mesh, factor: float) -> npc.Mesh:
    mesh.vertices = [(x * factor, y * factor, z * factor) for x, y, z in mesh.vertices]
    return mesh


def add_crown(mesh: npc.Mesh, y: float, radius: float, material: str = "gold_light") -> None:
    for i in range(5):
        x = -radius + i * radius * 0.5
        height = 0.20 + (0.08 if i % 2 == 0 else 0.0)
        npc.add_box(mesh, (x, y + height / 2, 0), (0.08, height, 0.08), material)


def add_antler(mesh: npc.Mesh, side: int, y: float, z: float, material: str = "bone") -> None:
    root = (side * 0.18, y, z)
    mid = (side * 0.38, y + 0.38, z - 0.02)
    tip = (side * 0.50, y + 0.76, z + 0.02)
    npc.add_beam(mesh, root, mid, 0.07, 0.07, material, 5)
    npc.add_beam(mesh, mid, tip, 0.06, 0.06, material, 5)
    npc.add_beam(mesh, (mid[0], mid[1], mid[2]), (mid[0] + side * 0.22, mid[1] + 0.08, mid[2] - 0.04), 0.045, 0.045, material, 5)
    npc.add_beam(mesh, (mid[0], mid[1], mid[2]), (mid[0] + side * 0.03, mid[1] + 0.33, mid[2] + 0.12), 0.04, 0.04, material, 5)


def add_defined_face(mesh: npc.Mesh, y: float = 2.77, z: float = 0.40,
                     skin: str = "skin", frame: str = "gold_light") -> None:
    """Put a real front face in front of helmets/hoods, plus a back marker."""
    npc.add_ellipsoid(mesh, (0, y, z - 0.01), (0.28, 0.32, 0.075), "black", 8, 4)
    npc.add_ellipsoid(mesh, (0, y, z + 0.055), (0.235, 0.27, 0.065), skin, 8, 4)
    npc.add_box(mesh, (-0.105, y + 0.075, z + 0.12), (0.075, 0.055, 0.035), "black")
    npc.add_box(mesh, (0.105, y + 0.075, z + 0.12), (0.075, 0.055, 0.035), "black")
    npc.add_box(mesh, (0, y + 0.005, z + 0.15), (0.065, 0.12, 0.045), skin)
    npc.add_box(mesh, (0, y - 0.13, z + 0.13), (0.15, 0.035, 0.035), "black")
    npc.add_box(mesh, (-0.25, y + 0.12, z + 0.02), (0.045, 0.32, 0.08), frame)
    npc.add_box(mesh, (0.25, y + 0.12, z + 0.02), (0.045, 0.32, 0.08), frame)


def add_back_marker(mesh: npc.Mesh, y: float = 2.77, z: float = -0.31, material: str = "cloth_dark") -> None:
    npc.add_box(mesh, (0, y, z), (0.065, 0.54, 0.055), material)
    npc.add_box(mesh, (0, y - 0.26, z), (0.20, 0.06, 0.055), material)
    npc.add_ellipsoid(mesh, (0, y + 0.18, z - 0.02), (0.10, 0.16, 0.04), material, 6, 3)


def make_adjudicator() -> npc.Mesh:
    mesh = npc.Mesh()
    npc.humanoid_base(mesh, "gold", "gold_light", "skin", armored=True)
    npc.add_cape(mesh, "gold")
    npc.add_box(mesh, (-0.56, 2.25, 0.03), (0.48, 0.30, 0.54), "gold_light")
    npc.add_box(mesh, (0.56, 2.25, 0.03), (0.48, 0.30, 0.54), "gold_light")
    npc.add_box(mesh, (0, 2.92, 0.28), (0.52, 0.16, 0.08), "gold_light")
    add_crown(mesh, 3.06, 0.45)
    npc.add_beam(mesh, (0.82, 0.20, 0.22), (0.82, 2.45, 0.22), 0.10, 0.10, "leather", 6)
    npc.add_box(mesh, (0.82, 2.54, 0.22), (0.72, 0.32, 0.40), "gold")
    npc.add_box(mesh, (0.82, 2.54, 0.44), (0.22, 0.12, 0.04), "gold_light")
    add_defined_face(mesh, skin="skin", frame="gold_light")
    add_back_marker(mesh, material="gold")
    return scale_mesh(mesh, 1.12)


def make_lich() -> npc.Mesh:
    mesh = npc.Mesh()
    npc.humanoid_base(mesh, "violet", "gold_light", "bone", robe=True)
    npc.add_hood(mesh, "violet")
    add_crown(mesh, 3.03, 0.48, "gold")
    npc.add_staff(mesh)
    npc.add_box(mesh, (-0.36, 1.82, 0.18), (0.18, 0.42, 0.06), "gold", yaw=0.18)
    npc.add_box(mesh, (0.36, 1.82, 0.18), (0.18, 0.42, 0.06), "gold", yaw=-0.18)
    add_defined_face(mesh, skin="bone", frame="gold")
    add_back_marker(mesh, material="violet")
    return scale_mesh(mesh, 1.04)


def make_cragmother() -> npc.Mesh:
    mesh = npc.Mesh()
    npc.humanoid_base(mesh, "rock", "moss_light", "rock", armored=True)
    for side in (-1, 1):
        npc.add_box(mesh, (side * 0.48, 2.32, 0.02), (0.46, 0.34, 0.50), "rock_light", yaw=side * 0.10)
        npc.add_beam(mesh, (side * 0.28, 2.18, 0.08), (side * 0.76, 1.55, 0.25), 0.25, 0.25, "rock", 5)
    npc.add_ellipsoid(mesh, (0, 2.78, -0.02), (0.38, 0.43, 0.34), "rock_light", 8, 4)
    add_antler(mesh, -1, 2.95, 0.02, "bone_dark")
    add_antler(mesh, 1, 2.95, 0.02, "bone_dark")
    npc.add_box(mesh, (0, 2.73, 0.33), (0.38, 0.16, 0.05), "moss_light")
    add_defined_face(mesh, y=2.78, z=0.40, skin="rock_light", frame="bone_dark")
    add_back_marker(mesh, y=2.78, material="rock")
    return scale_mesh(mesh, 1.16)


def make_gnawthane() -> npc.Mesh:
    mesh = npc.Mesh()
    fur = "leather"
    npc.add_ellipsoid(mesh, (0, 0.95, 0), (0.94, 0.48, 0.44), fur, 10, 5)
    npc.add_ellipsoid(mesh, (0.88, 1.28, 0), (0.44, 0.40, 0.36), "leather_light", 9, 5)
    npc.add_beam(mesh, (1.16, 1.20, 0), (1.48, 1.08, 0), 0.30, 0.24, "leather_light", 6)
    npc.add_box(mesh, (1.48, 1.08, 0), (0.20, 0.18, 0.18), "blood_red")
    for side in (-1, 1):
        npc.add_cone(mesh, (0.68, 1.67, side * 0.17), 0.15, 0.34, "leather_light", 5)
        npc.add_beam(mesh, (side * 0.50, 0.86, side * 0.20), (side * 0.58, 0.16, side * 0.22), 0.20, 0.20, "leather", 5)
        npc.add_beam(mesh, (side * 0.46, 0.93, side * 0.30), (side * 0.53, 0.20, side * 0.32), 0.13, 0.13, "leather_light", 5)
    npc.add_beam(mesh, (-0.82, 1.02, 0), (-1.34, 1.42, 0), 0.18, 0.18, "leather", 5)
    add_crown(mesh, 1.63, 0.54, "bone_dark")
    npc.add_box(mesh, (0.73, 1.42, 0.25), (0.10, 0.10, 0.05), "blood_red")
    npc.add_box(mesh, (0.55, 1.42, 0.25), (0.10, 0.10, 0.05), "blood_red")
    for i in range(3):
        npc.add_beam(mesh, (-0.30 + i * 0.30, 1.46, -0.05), (-0.34 + i * 0.30, 1.74, -0.05), 0.07, 0.07, "bone_dark", 5)
    return scale_mesh(mesh, 1.16)


def make_pale_stag() -> npc.Mesh:
    mesh = npc.Mesh()
    npc.add_ellipsoid(mesh, (0, 1.08, 0), (0.82, 0.43, 0.38), "bone", 10, 5)
    npc.add_ellipsoid(mesh, (0.76, 1.48, 0), (0.33, 0.34, 0.30), "bone_blue", 9, 5)
    npc.add_beam(mesh, (1.00, 1.42, 0), (1.26, 1.32, 0), 0.22, 0.20, "bone_blue", 6)
    npc.add_box(mesh, (1.27, 1.32, 0), (0.16, 0.15, 0.15), "black")
    for side in (-1, 1):
        npc.add_beam(mesh, (side * 0.45, 0.94, side * 0.18), (side * 0.52, 0.15, side * 0.20), 0.16, 0.16, "bone", 5)
        npc.add_beam(mesh, (side * 0.42, 1.00, side * 0.28), (side * 0.48, 0.20, side * 0.30), 0.11, 0.11, "bone_blue", 5)
    add_antler(mesh, -1, 1.73, 0.0, "bone_dark")
    add_antler(mesh, 1, 1.73, 0.0, "bone_dark")
    npc.add_box(mesh, (0.66, 1.60, 0.26), (0.08, 0.08, 0.05), "black")
    npc.add_box(mesh, (0.86, 1.60, 0.26), (0.08, 0.08, 0.05), "black")
    npc.add_beam(mesh, (-0.72, 1.04, -0.08), (-1.08, 1.28, -0.08), 0.10, 0.10, "bone", 5)
    return scale_mesh(mesh, 1.12)


def make_mirelight() -> npc.Mesh:
    mesh = npc.Mesh()
    npc.add_cone(mesh, (0, 1.12, 0), 0.58, 1.60, "violet", 8)
    npc.add_ellipsoid(mesh, (0, 2.70, -0.06), (0.43, 0.46, 0.36), "violet", 9, 5)
    npc.add_ellipsoid(mesh, (0, 2.70, 0.26), (0.22, 0.24, 0.10), "teal_light", 8, 4)
    npc.add_box(mesh, (0, 2.68, 0.37), (0.26, 0.12, 0.04), "black")
    for side in (-1, 1):
        npc.add_cone(mesh, (side * 0.34, 1.20, 0), 0.18, 0.82, "teal", 6)
    npc.add_ellipsoid(mesh, (0.72, 2.20, 0.10), (0.14, 0.14, 0.14), "teal_light", 8, 4)
    add_defined_face(mesh, y=2.70, z=0.40, skin="teal_light", frame="violet")
    add_back_marker(mesh, y=2.70, material="violet")
    return scale_mesh(mesh, 1.10)


def make_chief() -> npc.Mesh:
    mesh = npc.Mesh()
    npc.humanoid_base(mesh, "red", "gold_light", "skin", armored=True)
    npc.add_helmet(mesh)
    npc.add_cape(mesh, "red")
    npc.add_hammer(mesh)
    add_defined_face(mesh, skin="skin", frame="gold_light")
    add_back_marker(mesh, material="red")
    return scale_mesh(mesh, 1.08)


def make_matriarch() -> npc.Mesh:
    mesh = npc.make_wolf()
    for i in range(5):
        npc.add_beam(mesh, (-0.50 + i * 0.25, 1.46, 0.0), (-0.56 + i * 0.25, 1.76, 0.0), 0.10, 0.10, "bone_dark", 5)
    for side in (-1, 1):
        npc.add_box(mesh, (0.76 + side * 0.13, 1.60, 0.22), (0.08, 0.08, 0.05), "blood_red")
    npc.add_beam(mesh, (-0.72, 1.05, -0.08), (-1.08, 1.30, -0.08), 0.10, 0.10, "bone", 5)
    return scale_mesh(mesh, 1.16)


def make_tidemother_boss() -> npc.Mesh:
    mesh = npc.make_tidemother()
    add_defined_face(mesh, skin="skin", frame="gold_light")
    add_back_marker(mesh, material="teal")
    return scale_mesh(mesh, 1.10)


def make_tollmaster_boss() -> npc.Mesh:
    mesh = npc.make_tollmaster()
    add_defined_face(mesh, skin="skin", frame="gold_light")
    add_back_marker(mesh, material="red")
    return scale_mesh(mesh, 1.10)


BOSSES = [
    ("Adjudicator", make_adjudicator, 48.0, (5, 1)),
    ("Lich", make_lich, 46.0, (4, 1)),
    ("Cragmother", make_cragmother, 45.0, (2, 2)),
    ("Tidemother", make_tidemother_boss, 47.0, (1, 2)),
    ("Gnaw-Thane", make_gnawthane, 47.0, (3, 2)),
    ("Pale Stag", make_pale_stag, 47.0, (6, 2)),
    ("Mirelight", make_mirelight, 49.0, (5, 2)),
    ("Tollmaster", make_tollmaster_boss, 47.0, (4, 2)),
    ("Chief", make_chief, 47.0, (2, 1)),
    ("Matriarch", make_matriarch, 47.0, (3, 1)),
]


def fit(dst: Image.Image, src: Image.Image, box: tuple[int, int, int, int], nearest: bool = False) -> None:
    x0, y0, x1, y1 = box
    copy = src.copy()
    copy.thumbnail((x1 - x0, y1 - y0), Image.Resampling.NEAREST if nearest else Image.Resampling.LANCZOS)
    dst.alpha_composite(copy, (x0 + (x1 - x0 - copy.width) // 2, y0 + (y1 - y0 - copy.height) // 2))


def build_sheet() -> Image.Image:
    with Image.open(proto.CONTACT_2D) as opened:
        contact = opened.convert("RGBA")
    background = proto.build_grass_background(contact)
    canvas = Image.new("RGBA", (2100, 1880), (11, 13, 17, 255))
    d = ImageDraw.Draw(canvas)
    d.rectangle((0, 0, 2100, 140), fill=(8, 10, 14, 255))
    proto.draw_text(d, (54, 28), "BOSS TURN TABLE / TRUE 3D", 39, proto.INK, True)
    proto.draw_text(d, (56, 86), "Four yaw angles per boss / low-poly mesh blockout / production-safe visual study", 16, proto.MUTED)
    proto.draw_text(d, (2044, 42), "ISSUE 02", 15, proto.GOLD, True, anchor="ra")

    angles = (-35.0, 0.0, 35.0, 180.0)
    columns = [(170, 500, "2D ID REFERENCE"), (580, 910, "FRONT 3/4"), (980, 1310, "FRONT"), (1380, 1710, "SIDE"), (1780, 2040, "BACK 180°")]
    for x0, x1, label in columns:
        d.rounded_rectangle((x0, 170, x1, 220), 6, fill=(30, 32, 37, 255), outline=(74, 67, 49, 255), width=2)
        proto.draw_text(d, ((x0 + x1) // 2, 195), label, 13, proto.GOLD, True, anchor="mm")

    row_h = 151
    for i, (name, builder, scale, (col, row)) in enumerate(BOSSES):
        y0 = 240 + i * row_h
        y1 = y0 + row_h - 9
        d.rounded_rectangle((54, y0, 2046, y1), 7, fill=(22, 24, 29, 255), outline=(45, 46, 49, 255), width=1)
        d.text((72, y0 + 60), name, font=proto.font(14, mono=True), fill=proto.MUTED)
        reference = proto.extract_2d_sprite(contact, background, col, row)
        fit(canvas, reference, (170, y0 + 4, 500, y1 - 4), nearest=True)
        mesh = builder()
        for angle, (x0, x1, _) in zip(angles, columns[1:]):
            rendered = npc.render_mesh(mesh, yaw=angle, pitch=10.0, scale=scale)
            fit(canvas, rendered, (x0, y0 + 4, x1, y1 - 4), nearest=False)

    y0 = 240 + len(BOSSES) * row_h + 8
    d.rectangle((54, y0, 2046, 1846), fill=(16, 19, 24, 255), outline=(67, 61, 46, 255), width=2)
    proto.draw_text(d, (78, y0 + 20), "READ THE ROTATION", 17, proto.GOLD, True)
    proto.draw_text(d, (78, y0 + 58), "The purpose of this sheet is silhouette and construction: crown, staff, antlers, cloak, weapon side and shoulder mass must remain readable from every facing.", 14, proto.MUTED)
    proto.draw_text(d, (78, y0 + 94), "Next 3D issue: replace these blockout meshes with authored forms and directional animation frames; keep the 2.5D archive as fallback.", 14, (184, 153, 82))
    return canvas.convert("RGB")


def main() -> None:
    build_sheet().save(OUT / "boss-3d-turntable.png", quality=95)
    print("generated boss turntable in", OUT)


if __name__ == "__main__":
    main()
