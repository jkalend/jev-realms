"""Species-specific, opt-in Blender roster studies. Coordinates: Y up, +Z forward."""

from __future__ import annotations

import math


def _loft(api, npc, name, rings, mat, axis="z", segments=12):
    """Closed, rounded cross-section loft; rings give center and two transverse radii."""
    vertices = []
    for row, (cx, cy, cz, ra, rb) in enumerate(rings):
        if axis == "path":
            before = rings[max(0, row - 1)]
            after = rings[min(len(rings) - 1, row + 1)]
            dx, dy, dz = (after[k] - before[k] for k in range(3))
            length = math.sqrt(dx * dx + dy * dy + dz * dz)
            tangent = (dx / length, dy / length, dz / length)
            reference = (0, 0, 1) if abs(tangent[2]) < .85 else (0, 1, 0)
            ux = tangent[1] * reference[2] - tangent[2] * reference[1]
            uy = tangent[2] * reference[0] - tangent[0] * reference[2]
            uz = tangent[0] * reference[1] - tangent[1] * reference[0]
            size = math.sqrt(ux * ux + uy * uy + uz * uz)
            ux, uy, uz = ux / size, uy / size, uz / size
            vx = tangent[1] * uz - tangent[2] * uy
            vy = tangent[2] * ux - tangent[0] * uz
            vz = tangent[0] * uy - tangent[1] * ux
        for i in range(segments):
            angle = math.tau * i / segments
            a, b = ra * math.cos(angle), rb * math.sin(angle)
            vertices.append((cx + a * ux + b * vx, cy + a * uy + b * vy, cz + a * uz + b * vz)
                            if axis == "path" else
                            (cx + a, cy + (b if axis == "z" else 0),
                             cz + (b if axis == "y" else 0)))
    faces = []
    for row in range(len(rings) - 1):
        for i in range(segments):
            j = (i + 1) % segments
            faces.append((row * segments + i, row * segments + j,
                          (row + 1) * segments + j, (row + 1) * segments + i))
    for row in (0, len(rings) - 1):
        vertices.append(tuple(rings[row][:3]))
        center = len(vertices) - 1
        for i in range(segments):
            j = (i + 1) % segments
            faces.append((center, row * segments + j, row * segments + i)
                         if row == 0 else (center, row * segments + i, row * segments + j))
    rounded = any(part in name for part in ("Body", "Chest", "Face", "Skull", "Snout", "Jaw", "Hump", "Neck", "Throat", "Tail", "Bib", "Mane", "Leg", "Shoulder", "Haunch", "Thigh", "Paw", "Foot"))
    obj = api.mesh_object(f"{npc}_{name}", vertices, faces, mat, npc,
                          subsurf=1 if rounded else 0)
    # Organic volumes need continuous normals; horns/claws keep their edged read.
    sharp = any(part in name for part in ("Antler", "Crown", "Shard", "Claw", "Hoof", "Tooth", "Fang"))
    for polygon in obj.data.polygons:
        polygon.use_smooth = not sharp
    return obj


def _ring(api, npc, name, center, outer_x, outer_y, tube, mat, scale):
    """Volumetric elliptical torus facing both front and back, not a billboard."""
    vertices = []
    for i in range(24):
        angle = math.tau * i / 24
        for j in range(8):
            cross = math.tau * j / 8
            spread = tube * math.cos(cross)
            vertices.append(((outer_x + spread) * math.cos(angle) * scale,
                             (center + (outer_y + spread) * math.sin(angle)) * scale,
                             (0.10 + tube * math.sin(cross)) * scale))
    faces = []
    for i in range(24):
        for j in range(8):
            a, b = i * 8 + j, i * 8 + (j + 1) % 8
            c, d = ((i + 1) % 24) * 8 + (j + 1) % 8, ((i + 1) % 24) * 8 + j
            faces.append((a, b, c, d))
    api.mesh_object(f"{npc}_{name}", vertices, faces, mat, npc)


def _scaled(rings, scale):
    return [tuple(value * scale for value in ring) for ring in rings]


def _strand(api, npc, name, points, widths, mat, s):
    """A tapered round strand, unlike the fixed-width curve helper."""
    rings = []
    for (x, y, z), width in zip(points, widths):
        rings.append((x * s, y * s, z * s, width * s, width * s))
    _loft(api, npc, name, rings, mat, axis="path", segments=8)


def _add_quad_legs(api, npc: str, s: float, hide, mat_toes, mat_claws,
                   stance: tuple[float, float],
                   front_z: float, rear_z: float,
                   hip_y: tuple[float, float],
                   thick: float,
                   hoof: bool = False,
                   toes: int = 3,
                   bear_claws: bool = False) -> dict[str, tuple]:
    legs = {}
    for front in (True, False):
        for side in (-1, 1):
            label = ("F" if front else "B") + api.side_label(side)
            x = side * stance[0 if front else 1]
            hy = hip_y[0 if front else 1]
            z = front_z if front else rear_z
            foot_z = z + (.08 if front else (-.05 if hoof else -.08))
            knee_z = z + (-.09 if front else -.24) if not hoof else z + (.02 if front else .10)
            hock_z = z + (-.18 if front else .12) if not hoof else z - (.05 if front else .11)
            if bear_claws:
                knee_z = z + .04
                hock_z = foot_z - .04

            # Seamless leg loft: extends smoothly from foot up into the interior torso core
            _loft(api, npc, f"Leg{label}", _scaled([
                (x * 1.08, .055, foot_z + (.10 if not hoof else .06),
                 thick * (1.35 if not hoof else 1.1), thick * (1.55 if not hoof else 1.4)),
                (x * 1.08, .15, foot_z, thick * 1.10, thick * 1.15),
                (x * 1.06, .37 if not hoof else .35, hock_z, thick * .84, thick * .86),
                (x * 1.02, .64 if not hoof else .73, knee_z, thick * 1.16, thick * 1.12),
                (x * .96, hy * .86, z, thick * 1.38, thick * 1.36),
                (x * .84, hy, z, thick * 1.42, thick * 1.42),
                (x * .70, hy * 1.06, z, thick * 1.44, thick * 1.46),
            ], s), hide, axis="y")

            legs[label] = ((x * .84 * s, hy * s, z * s),
                           (x * 1.08 * s, .11 * s, foot_z * s))

            # Muscular anatomical shoulder / haunch blending the limb seamlessly into the torso
            if front:
                _loft(api, npc, f"Shoulder{label}", _scaled([
                    (x * 0.90, hy * 1.06, z - 0.05, thick * 1.35, thick * 1.40),
                    (x * 1.04, hy * 0.88, z + 0.02, thick * 1.28, thick * 1.32),
                    (x * 0.96, hy * 0.68, z + 0.04, thick * 1.08, thick * 1.12),
                ], s), hide, axis="y")
            else:
                _loft(api, npc, f"Haunch{label}", _scaled([
                    (x * 0.86, hy * 1.05, z + 0.06, thick * 1.45, thick * 1.50),
                    (x * 1.05, hy * 0.86, z - 0.02, thick * 1.48, thick * 1.52),
                    (x * 1.00, hy * 0.65, z - 0.06, thick * 1.24, thick * 1.28),
                ], s), hide, axis="y")

            for n in range(toes):
                spread = (n - (toes - 1) / 2) * thick * .68
                _loft(api, npc, f"Leg{label}Toe{n}", _scaled([
                    (x * 1.08 + spread, .095, foot_z + .07, thick * .28, thick * .36),
                    (x * 1.08 + spread * 1.22, .078, foot_z + .19, thick * .22, thick * .26),
                    (x * 1.08 + spread * 1.32, .075, foot_z + .27, .008, .008),
                ], s), mat_toes, axis="z", segments=8)

            if bear_claws and front:
                for n in range(toes):
                    spread = (n - (toes - 1) / 2) * thick * .68
                    _strand(api, npc, f"Leg{label}Claw{n}", [
                        (x * 1.08 + spread * 1.22, .075, foot_z + .21),
                        (x * 1.08 + spread * 1.30, .055, foot_z + .33),
                        (x * 1.08 + spread * 1.35, .018, foot_z + .40),
                    ], [.036, .026, .006], mat_claws, s)

    return legs


def _add_wolf(npc: str, cfg: dict, mats: dict, api) -> dict[str, tuple]:
    s = cfg["scale"]
    hide = mats["skin"]
    pale = mats["accent"]
    dark_fur = mats["cloth_shade"]
    dark = mats["dark"]

    # Athletic lupine body with high withers, deep chest, and sharp abdominal tuck
    body_rings = [
        (0, 1.14, -0.98, 0.18, 0.22),
        (0, 1.18, -0.78, 0.32, 0.30),
        (0, 1.14, -0.36, 0.25, 0.28),
        (0, 1.19,  0.10, 0.34, 0.38),
        (0, 1.25,  0.48, 0.40, 0.45),
        (0, 1.30,  0.74, 0.34, 0.42),
        (0, 1.26,  0.92, 0.26, 0.34),
    ]
    _loft(api, npc, "Body", _scaled(body_rings, s), hide)

    # Ventral pale throat and chest bib
    bib_rings = [
        (0, 0.94,  0.15, 0.22, 0.16),
        (0, 1.00,  0.45, 0.26, 0.20),
        (0, 1.14,  0.72, 0.25, 0.22),
        (0, 1.30,  0.94, 0.22, 0.20),
        (0, 1.46,  1.12, 0.18, 0.18),
    ]
    _loft(api, npc, "ChestBib", _scaled(bib_rings, s), pale)

    # --- FULL-CIRCUMFERENCE THICK VOLUMETRIC WOLF MANE ---
    # 1. Heavy volumetric mane cowl that wraps 360-degrees over the entire neck,
    # withers, throat, and shoulders (widens from head base to broad shoulder cape)
    mane_cowl_rings = [
        (0, 1.62, 1.24, 0.34, 0.30),  # Base of skull / jaw
        (0, 1.54, 1.06, 0.44, 0.40),  # Upper neck
        (0, 1.44, 0.82, 0.52, 0.48),  # Mid neck (very thick volumetric fur)
        (0, 1.34, 0.56, 0.56, 0.52),  # Shoulders / withers mantle
        (0, 1.26, 0.28, 0.50, 0.46),  # Drape over scapula and thoracic arch
    ]
    _loft(api, npc, "NeckMane", _scaled(mane_cowl_rings, s), dark_fur)

    # 2. Raised bristling dorsal mane crest rising high along the nape and withers
    crest_rings = [
        (0, 1.44, 0.18, 0.18, 0.14),
        (0, 1.58, 0.42, 0.26, 0.20),
        (0, 1.68, 0.68, 0.28, 0.22),
        (0, 1.74, 0.94, 0.24, 0.20),
        (0, 1.78, 1.16, 0.18, 0.16),
    ]
    _loft(api, npc, "DorsalMane", _scaled(crest_rings, s), dark_fur)

    # 3. Inner neck core
    neck_rings = [
        (0, 1.24, 0.82, 0.28, 0.32),
        (0, 1.38, 0.98, 0.26, 0.30),
        (0, 1.52, 1.14, 0.22, 0.26),
        (0, 1.66, 1.26, 0.18, 0.20),
    ]
    _loft(api, npc, "ChestNeck", _scaled(neck_rings, s), hide)

    # 4. Multi-directional stylized fur tufts / ruff flanges:
    # A. Lateral flaring ruffs (dramatic wide silhouette)
    for side in (-1, 1):
        letter = api.side_label(side)
        # Upper nape tuft
        _loft(api, npc, f"ManeTuftUpper{letter}", _scaled([
            (side * 0.20, 1.62, 1.14, 0.12, 0.12),
            (side * 0.42, 1.56, 0.98, 0.13, 0.10),
            (side * 0.54, 1.48, 0.82, 0.02, 0.02),
        ], s), dark_fur, axis="path", segments=8)

        # Mid neck broad lateral ruff (flaring out to X = 0.62)
        _loft(api, npc, f"ManeTuftMid{letter}", _scaled([
            (side * 0.26, 1.44, 0.86, 0.14, 0.14),
            (side * 0.48, 1.38, 0.68, 0.15, 0.12),
            (side * 0.62, 1.26, 0.48, 0.02, 0.02),
        ], s), hide, axis="path", segments=8)

        # Scapula / shoulder cape tuft
        _loft(api, npc, f"ManeTuftShoulder{letter}", _scaled([
            (side * 0.28, 1.34, 0.56, 0.14, 0.14),
            (side * 0.50, 1.24, 0.36, 0.13, 0.11),
            (side * 0.58, 1.12, 0.18, 0.02, 0.02),
        ], s), dark_fur, axis="path", segments=8)

        # Flared cheek tufts flanking the jaw
        _loft(api, npc, f"CheekTuft{letter}", _scaled([
            (side * 0.18, 1.60, 1.22, 0.09, 0.12),
            (side * 0.34, 1.54, 1.14, 0.10, 0.11),
            (side * 0.46, 1.46, 1.02, 0.015, 0.02),
        ], s), pale, axis="path", segments=8)

    # B. Ventral / throat hanging ruff tufts (shaggy underside of neck)
    for n, (y_t, z_t) in enumerate([(1.15, 0.95), (1.05, 0.72), (0.95, 0.48)]):
        _loft(api, npc, f"ManeThroatTuft{n}", _scaled([
            (0, y_t + 0.12, z_t + 0.12, 0.16, 0.14),
            (0, y_t,        z_t,        0.18, 0.12),
            (0, y_t - 0.12, z_t - 0.14, 0.02, 0.02),
        ], s), pale if n == 0 else dark_fur, axis="path", segments=8)

    # C. Dorsal hackle spikes along top of mane
    for n, (y_s, z_s) in enumerate([(1.76, 1.05), (1.70, 0.78), (1.60, 0.52)]):
        _loft(api, npc, f"ManeDorsalSpike{n}", _scaled([
            (0, y_s - 0.10, z_s + 0.08, 0.14, 0.12),
            (0, y_s,        z_s,        0.12, 0.10),
            (0, y_s + 0.10, z_s - 0.12, 0.02, 0.02),
        ], s), dark_fur, axis="path", segments=8)

    # Chiseled wolf skull and muzzle
    skull_rings = [
        (0, 1.68, 1.08, 0.20, 0.18),
        (0, 1.72, 1.22, 0.25, 0.20),
        (0, 1.64, 1.36, 0.20, 0.17),
        (0, 1.55, 1.52, 0.14, 0.14),
        (0, 1.48, 1.68, 0.10, 0.11),
        (0, 1.45, 1.76, 0.07, 0.08),
    ]
    _loft(api, npc, "FaceSkull", _scaled(skull_rings, s), hide)
    api.add_ico(f"{npc}_Nose", (0, 1.46 * s, 1.79 * s), (0.075 * s, 0.065 * s, 0.06 * s), dark, npc)

    # Pale lower jaw
    _loft(api, npc, "MuzzleJaw", _scaled([
        (0, 1.42, 1.36, 0.14, 0.08),
        (0, 1.41, 1.55, 0.10, 0.07),
        (0, 1.40, 1.72, 0.06, 0.05),
    ], s), pale)

    # Eyes & ears
    for side in (-1, 1):
        letter = api.side_label(side)
        api.add_ico(f"{npc}_EyeSocket{letter}", (side * 0.165 * s, 1.66 * s, 1.36 * s),
                    (0.045 * s, 0.045 * s, 0.035 * s), dark, npc)
        api.add_sphere(f"{npc}_Eye{letter}", (side * 0.17 * s, 1.665 * s, 1.385 * s),
                       (0.026 * s, 0.030 * s, 0.024 * s), pale, npc)

        # Triangular erect ears with pale inner fur
        _loft(api, npc, f"Ear{letter}", _scaled([
            (side * 0.18, 1.74, 1.15, 0.10, 0.09),
            (side * 0.28, 1.94, 1.10, 0.09, 0.07),
            (side * 0.34, 2.14, 1.05, 0.02, 0.02),
        ], s), hide, axis="y")
        _loft(api, npc, f"EarInner{letter}", _scaled([
            (side * 0.19, 1.76, 1.17, 0.06, 0.05),
            (side * 0.28, 1.93, 1.12, 0.05, 0.04),
            (side * 0.33, 2.10, 1.07, 0.015, 0.015),
        ], s), pale, axis="y")

    # Bushy brush tail
    tail_rings = [
        (0, 1.10, -0.96, 0.13, 0.13),
        (0, 0.94, -1.22, 0.18, 0.17),
        (0, 0.74, -1.46, 0.16, 0.15),
        (0, 0.54, -1.68, 0.12, 0.11),
        (0, 0.40, -1.84, 0.04, 0.04),
    ]
    _loft(api, npc, "Tail", _scaled(tail_rings, s), hide)
    api.add_ico(f"{npc}_TailTip", (0, 0.38 * s, -1.86 * s), (0.05 * s, 0.05 * s, 0.06 * s), dark_fur, npc)

    return _add_quad_legs(api, npc, s, hide, dark, dark,
                          stance=(.36, .38), front_z=.48, rear_z=-.70,
                          hip_y=(1.18, 1.10), thick=.125, toes=4)


def _add_bear(npc: str, cfg: dict, mats: dict, api) -> dict[str, tuple]:
    s = cfg["scale"]
    hide = mats["skin"]
    accent = mats["accent"]
    dark = mats["dark"]
    bone = mats["bone"]

    # Massive barrel body with distinct grizzly shoulder hump (y=1.48!)
    body_rings = [
        (0, 1.15, -1.06, 0.42, 0.44),
        (0, 1.20, -0.80, 0.54, 0.50),
        (0, 1.24, -0.32, 0.62, 0.56),
        (0, 1.48,  0.26, 0.68, 0.62),
        (0, 1.36,  0.60, 0.62, 0.54),
        (0, 1.28,  0.88, 0.50, 0.46),
    ]
    _loft(api, npc, "Body", _scaled(body_rings, s), hide)

    # Golden honey chest bib
    bib_rings = [
        (0, 1.02, 0.48, 0.36, 0.22),
        (0, 1.18, 0.74, 0.34, 0.24),
        (0, 1.32, 0.94, 0.28, 0.22),
    ]
    _loft(api, npc, "ChestBib", _scaled(bib_rings, s), accent)

    # Heavy neck
    neck_rings = [
        (0, 1.28, 0.86, 0.48, 0.44),
        (0, 1.40, 1.04, 0.44, 0.40),
        (0, 1.50, 1.20, 0.38, 0.34),
    ]
    _loft(api, npc, "ChestNeck", _scaled(neck_rings, s), hide)

    # Broad cranial vault
    skull_rings = [
        (0, 1.54, 1.08, 0.38, 0.28),
        (0, 1.60, 1.26, 0.42, 0.30),
        (0, 1.52, 1.44, 0.36, 0.26),
    ]
    _loft(api, npc, "FaceSkull", _scaled(skull_rings, s), hide)

    # Broad, blunt honey snout
    snout_rings = [
        (0, 1.46, 1.42, 0.32, 0.24),
        (0, 1.40, 1.60, 0.25, 0.20),
        (0, 1.36, 1.76, 0.18, 0.16),
        (0, 1.34, 1.84, 0.14, 0.13),
    ]
    _loft(api, npc, "FaceSnout", _scaled(snout_rings, s), accent)
    api.add_ico(f"{npc}_Nose", (0, 1.35 * s, 1.88 * s), (0.13 * s, 0.10 * s, 0.08 * s), dark, npc)

    # Heavy lower jaw
    _loft(api, npc, "MuzzleJaw", _scaled([
        (0, 1.30, 1.46, 0.24, 0.12),
        (0, 1.28, 1.68, 0.18, 0.10),
        (0, 1.27, 1.82, 0.12, 0.08),
    ], s), accent)

    # Eyes & cupped ears
    for side in (-1, 1):
        letter = api.side_label(side)
        api.add_sphere(f"{npc}_Eye{letter}", (side * 0.26 * s, 1.54 * s, 1.42 * s),
                       (0.040 * s, 0.042 * s, 0.035 * s), dark, npc)
        api.add_sphere(f"{npc}_Ear{letter}", (side * 0.36 * s, 1.68 * s, 1.18 * s),
                       (0.14 * s, 0.14 * s, 0.09 * s), hide, npc)
        api.add_ico(f"{npc}_EarInner{letter}", (side * 0.36 * s, 1.68 * s, 1.23 * s),
                    (0.08 * s, 0.08 * s, 0.03 * s), accent, npc)

    # Short furry bobtail
    _loft(api, npc, "Tail", _scaled([
        (0, 1.18, -1.06, 0.16, 0.15),
        (0, 1.22, -1.22, 0.12, 0.12),
        (0, 1.26, -1.28, 0.04, 0.04),
    ], s), hide)

    return _add_quad_legs(api, npc, s, hide, dark, bone,
                          stance=(.52, .48), front_z=.56, rear_z=-.72,
                          hip_y=(1.30, 1.12), thick=.24, toes=5, bear_claws=True)


def _add_rat(npc: str, cfg: dict, mats: dict, api) -> dict[str, tuple]:
    s = cfg["scale"]
    hide = mats["skin"]
    accent = mats["accent"]
    dark = mats["dark"]
    is_boss = cfg["kind"] == "boss"

    # Hunched kyphotic rodent spine with high dorsal arch (y=1.14!)
    body_rings = [
        (0, 0.72, -0.96, 0.16, 0.16),
        (0, 0.94, -0.74, 0.32, 0.30),
        (0, 1.14, -0.34, 0.38, 0.36),
        (0, 0.98,  0.10, 0.34, 0.32),
        (0, 0.82,  0.44, 0.28, 0.26),
        (0, 0.78,  0.68, 0.20, 0.20),
    ]
    _loft(api, npc, "Body", _scaled(body_rings, s), hide)

    # Underbelly countershading
    belly_rings = [
        (0, 0.60, -0.40, 0.20, 0.12),
        (0, 0.62,  0.00, 0.22, 0.14),
        (0, 0.68,  0.36, 0.18, 0.12),
    ]
    _loft(api, npc, "ChestBib", _scaled(belly_rings, s), accent)

    # Low-thrust rodent head
    head_rings = [
        (0, 0.80, 0.64, 0.20, 0.16),
        (0, 0.84, 0.80, 0.24, 0.18),
        (0, 0.80, 0.98, 0.20, 0.16),
        (0, 0.74, 1.16, 0.13, 0.12),
        (0, 0.72, 1.30, 0.06, 0.06),
    ]
    _loft(api, npc, "FaceSkull", _scaled(head_rings, s), hide)
    api.add_ico(f"{npc}_Nose", (0, 0.72 * s, 1.33 * s), (0.05 * s, 0.045 * s, 0.04 * s), accent, npc)

    # Lower jaw
    _loft(api, npc, "MuzzleJaw", _scaled([
        (0, 0.68, 0.84, 0.14, 0.08),
        (0, 0.67, 1.08, 0.10, 0.06),
        (0, 0.66, 1.26, 0.04, 0.03),
    ], s), accent)

    # Whiskers, eyes & rounded pink ears
    for side in (-1, 1):
        letter = api.side_label(side)
        api.add_sphere(f"{npc}_Eye{letter}", (side * 0.14 * s, 0.84 * s, 0.96 * s),
                       (0.038 * s, 0.040 * s, 0.034 * s), dark, npc)

        # Translucent rounded ears
        _loft(api, npc, f"Ear{letter}", _scaled([
            (side * 0.18, 0.88, 0.78, 0.08, 0.06),
            (side * 0.26, 1.02, 0.76, 0.14, 0.06),
            (side * 0.28, 1.16, 0.74, 0.10, 0.04),
            (side * 0.28, 1.22, 0.74, 0.02, 0.02),
        ], s), accent, axis="y")

        # Twitchy whiskers
        for n, delta in enumerate((-0.08, 0.0, 0.08)):
            _strand(api, npc, f"Whisker{n}{letter}", [
                (side * 0.10, 0.74, 1.12),
                (side * 0.32, 0.74 + delta, 1.20),
                (side * 0.50, 0.74 + delta * 1.6, 1.26),
            ], [0.014, 0.009, 0.003], accent, s)

    # Sinuous flexible tail curving near ground and curling up
    tail_rings = [
        (0.00, 0.72, -0.94, 0.080, 0.080),
        (0.03, 0.50, -1.22, 0.065, 0.065),
        (0.08, 0.36, -1.50, 0.050, 0.050),
        (0.14, 0.38, -1.78, 0.035, 0.035),
        (0.18, 0.52, -2.00, 0.012, 0.012),
    ]
    _loft(api, npc, "Tail", _scaled(tail_rings, s), accent, axis="path")

    # Boss GnawThane: crystal crown and dorsal spine shards
    if is_boss:
        for n in range(5):
            x = (n - 2) * 0.12
            _loft(api, npc, f"FaceCrown{n}", _scaled([
                (x, 0.98, 0.78, 0.075, 0.075),
                (x * 1.15, 1.18, 0.74, 0.057, 0.055),
                (x * 1.30, 1.36 + (.10 if n == 2 else 0), 0.70, 0.008, 0.009),
            ], s), accent, axis="y", segments=8)

        # Spiny shards along the arched hunchback
        for n in range(5):
            z_pos = -0.70 + n * 0.22
            y_pos = 0.96 + (0.20 if 1 <= n <= 3 else 0.0)
            _loft(api, npc, f"SpineShard{n}", _scaled([
                (0, y_pos, z_pos, 0.05, 0.05),
                (0, y_pos + 0.16, z_pos - 0.03, 0.035, 0.035),
                (0, y_pos + 0.30, z_pos - 0.06, 0.006, 0.006),
            ], s), accent, axis="y", segments=6)

    return _add_quad_legs(api, npc, s, hide, accent, dark,
                          stance=(.26, .30), front_z=.42, rear_z=-.68,
                          hip_y=(0.78, 0.88), thick=.082, toes=4)


def _add_stag(npc: str, cfg: dict, mats: dict, api) -> dict[str, tuple]:
    s = cfg["scale"]
    hide = mats["skin"]
    pale = mats["accent"]
    dark = mats["dark"]
    bone = mats["bone"]

    # Regal high cervid torso with deep chest
    body_rings = [
        (0, 1.22, -0.96, 0.22, 0.26),
        (0, 1.26, -0.74, 0.32, 0.30),
        (0, 1.18, -0.26, 0.28, 0.30),
        (0, 1.24,  0.22, 0.36, 0.40),
        (0, 1.34,  0.54, 0.38, 0.44),
        (0, 1.38,  0.78, 0.26, 0.34),
    ]
    _loft(api, npc, "Body", _scaled(body_rings, s), hide)

    # Snowy white rump patch
    api.add_ico(f"{npc}_TailRump", (0, 1.26 * s, -0.95 * s), (0.24 * s, 0.24 * s, 0.16 * s), pale, npc)

    # High proud neck
    neck_rings = [
        (0, 1.38, 0.78, 0.26, 0.34),
        (0, 1.64, 0.88, 0.22, 0.28),
        (0, 1.90, 0.98, 0.18, 0.22),
        (0, 2.06, 1.04, 0.16, 0.18),
    ]
    _loft(api, npc, "ChestNeck", _scaled(neck_rings, s), hide)

    # Chiseled skull
    skull_rings = [
        (0, 2.06, 0.92, 0.16, 0.16),
        (0, 2.12, 1.06, 0.19, 0.18),
        (0, 2.02, 1.22, 0.15, 0.14),
        (0, 1.92, 1.40, 0.10, 0.10),
        (0, 1.86, 1.50, 0.065, 0.065),
    ]
    _loft(api, npc, "FaceSkull", _scaled(skull_rings, s), hide)
    api.add_ico(f"{npc}_Nose", (0, 1.86 * s, 1.52 * s), (0.065 * s, 0.05 * s, 0.05 * s), dark, npc)

    # Eyes & alert leaf ears
    for side in (-1, 1):
        letter = api.side_label(side)
        api.add_sphere(f"{npc}_Eye{letter}", (side * 0.145 * s, 2.08 * s, 1.10 * s),
                       (0.042 * s, 0.046 * s, 0.038 * s), dark, npc)

        # Leaf ears swept back and outward
        _loft(api, npc, f"Ear{letter}", _scaled([
            (side * 0.16, 2.14, 0.98, 0.08, 0.06),
            (side * 0.32, 2.26, 0.82, 0.12, 0.05),
            (side * 0.42, 2.36, 0.68, 0.02, 0.02),
        ], s), hide, axis="path", segments=8)

        # Branching imperial antler rack
        beam = [
            (side * 0.16, 2.20, 0.96),
            (side * 0.36, 2.38, 0.80),
            (side * 0.56, 2.62, 0.62),
            (side * 0.72, 2.88, 0.42),
            (side * 0.82, 3.12, 0.22),
        ]
        _strand(api, npc, f"AntlerBeam{letter}", beam, [.085, .080, .064, .042, .008], bone, s)

        # Brow tine (forward thrust above brow)
        _strand(api, npc, f"AntlerBrow{letter}", [
            beam[0],
            (side * 0.22, 2.34, 1.14),
            (side * 0.26, 2.50, 1.28),
        ], [.055, .030, .006], bone, s)

        # Bez tine (mid-beam upward tine)
        _strand(api, npc, f"AntlerBez{letter}", [
            beam[2],
            (side * 0.62, 2.82, 0.75),
            (side * 0.68, 3.04, 0.85),
        ], [.048, .026, .006], bone, s)

        # Crown fork at top
        _strand(api, npc, f"AntlerCrown{letter}", [
            beam[3],
            (side * 0.66, 3.10, 0.36),
            (side * 0.60, 3.28, 0.30),
        ], [.040, .022, .005], bone, s)

    # Short deer tail
    _loft(api, npc, "Tail", _scaled([
        (0, 1.22, -0.98, 0.10, 0.10),
        (0, 1.28, -1.14, 0.08, 0.08),
        (0, 1.34, -1.22, 0.02, 0.02),
    ], s), pale)

    return _add_quad_legs(api, npc, s, hide, dark, dark,
                          stance=(.30, .32), front_z=.52, rear_z=-.70,
                          hip_y=(1.30, 1.20), thick=.082, hoof=True, toes=2)


def add_beast_body(npc: str, cfg: dict, mats: dict, api) -> dict[str, tuple]:
    form = cfg["form"]
    if form == "wolf":
        return _add_wolf(npc, cfg, mats, api)
    elif form == "bear":
        return _add_bear(npc, cfg, mats, api)
    elif form == "rat":
        return _add_rat(npc, cfg, mats, api)
    elif form == "stag":
        return _add_stag(npc, cfg, mats, api)
    else:
        # Fallback for any other beast
        return _add_wolf(npc, cfg, mats, api)


def add_wisp_body(npc: str, cfg: dict, mats: dict, api) -> None:
    s = cfg["scale"]
    boss = cfg["kind"] == "boss"
    # Alpha is on the shell, not the empty center; render/export remain 3-D meshes.
    shell = api.base.material(f"{npc}_SpectralShell", api.rgba(cfg["palette"]["cloth"]), 0.0, .49)
    shell.node_tree.nodes.get("Principled BSDF").inputs["Alpha"].default_value = .82
    shell.surface_render_method = "DITHERED"
    halo = api.base.material(f"{npc}_SpectralRim", api.rgba(cfg["palette"]["accent"]), 0.0, .28)
    shader = halo.node_tree.nodes.get("Principled BSDF")
    shader.inputs["Emission Color"].default_value = api.rgba(cfg["palette"]["accent"])
    shader.inputs["Emission Strength"].default_value = .48 if boss else .25
    _ring(api, npc, "BodyHollow", 1.61, .34 if boss else .28, .39, .10, shell, s)
    _ring(api, npc, "BodyCoreRim", 1.61, .19 if boss else .16, .19, .037, halo, s)
    api.add_ico(f"{npc}_BodyEmber", (0, 1.61 * s, .10 * s),
                ((.055 if boss else .038) * s, .07 * s, .055 * s), halo, npc)
    _loft(api, npc, "Tail", _scaled([
        (0, .39, -.22, .008, .01),
        (.07, .65, -.19, .11, .10),
        (.02, 1.01, -.12, .22, .17),
        (0, 1.28, -.06, .28, .19),
        (0, 1.36, -.04, .26, .17),
    ], s), shell, axis="y")
    _loft(api, npc, "ChestCowl", _scaled([
        (0, 1.87, -.09, .28, .19),
        (0, 2.04, -.08, .34 if boss else .28, .21),
        (0, 2.23, -.02, .22, .17),
        (0, 2.32, .01, .11, .12),
    ], s), shell, axis="y")
    _loft(api, npc, "FaceMask", _scaled([
        (0, 2.28, .0, .21, .18),
        (0, 2.43, .02, .29 if boss else .24, .22),
        (0, 2.59, .07, .22, .18),
        (0, 2.75 if boss else 2.69, .05, .025, .03),
    ], s), halo if boss else shell, axis="y")
    for side in (-1, 1):
        letter = api.side_label(side)
        _loft(api, npc, f"WispArm{letter}", _scaled([
            (side * .26, 2.04, -.04, .11, .10),
            (side * .42, 1.87, .01, .13, .13),
            (side * .55, 1.58, .14, .08, .085),
            (side * .62, 1.39, .20, .014, .021),
        ], s), shell, axis="y")
        api.add_ico(f"{npc}_Eye{letter}",
                    (side * .105 * s, 2.49 * s, .225 * s),
                    (.065 * s, .047 * s, .045 * s), mats["dark"], npc)
        api.add_ico(f"{npc}_EyeSpark{letter}",
                    (side * .105 * s, 2.49 * s, .252 * s),
                    (.022 * s, .024 * s, .018 * s), halo, npc)
        if boss:
            _strand(api, npc, f"GlowCrown{letter}",
                    [(side * .19, 2.61, .00), (side * .31, 2.80, -.03),
                     (side * .42, 3.02, -.09)], [.065, .045, .005], halo, s)
        else:
            _strand(api, npc, f"GlowDroop{letter}",
                    [(side * .18, 2.64, -.06), (side * .34, 2.51, -.12),
                     (side * .39, 2.35, -.16)], [.07, .05, .006], shell, s)
        for n in range(2 if boss else 3):
            _strand(api, npc, f"TailMist{n}{letter}",
                    [(side * (.14 + .075 * n), 1.22, -.09),
                     (side * (.22 + .085 * n), .77 - .075 * n, -.13 - .1 * n),
                     (side * (.28 + .105 * n), .31 - .055 * n, -.27 - .13 * n)],
                    [.065, .05, .004], shell if n else halo, s)
    if boss:
        # Crown and double wake identify Mirelight even in silhouette from behind.
        _strand(api, npc, "GlowCrownCenter", [(0, 2.68, -.06), (0, 2.94, -.13),
                                               (0, 3.14, -.20)], [.073, .045, .005], halo, s)
        _loft(api, npc, "TailWake", _scaled([(0, .93, -.17, .14, .11),
                                             (0, .52, -.32, .10, .09),
                                             (0, .19, -.49, .008, .008)], s), halo, axis="y")
    else:
        # The impostor retains a small dark split through its back as well as its face.
        api.add_ico(f"{npc}_FaceBackMark", (0, 2.43 * s, -.18 * s),
                    (.083 * s, .18 * s, .025 * s), mats["dark"], npc)
