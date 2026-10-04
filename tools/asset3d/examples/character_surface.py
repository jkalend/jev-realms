"""Deterministic, exportable character surface refinement in authored Y-up space."""
from __future__ import annotations

import math


def folded_panel(rows, columns=9):
    """Subdivide a drape with restrained folds; keep edges and centre anchored."""
    vertices = []
    height = abs(rows[0][1][1] - rows[-1][1][1])
    for row_index, row in enumerate(rows):
        fall = row_index / max(1, len(rows) - 1)
        amplitude = min(.045, height * .032) * (.25 + .75 * fall)
        facing = -1 if row[1][2] < 0 else 1
        for column in range(columns):
            u = column / (columns - 1)
            half = 0 if u <= .5 else 1
            t = u * 2 if half == 0 else (u - .5) * 2
            point = [a + (b - a) * t for a, b in zip(row[half], row[half + 1])]
            point[2] += facing * amplitude * math.sin(4 * math.pi * u) * math.sin(math.pi * u)
            vertices.append(tuple(point))
    faces = [(i * columns + j, i * columns + j + 1,
              (i + 1) * columns + j + 1, (i + 1) * columns + j)
             for i in range(len(rows) - 1) for j in range(columns - 1)]
    return vertices, faces


def cloth_radius(angle, y, low, high, strength=.035):
    """Broad pleats relax below the waist, never alter collar/hem height or pivots."""
    fall = 1 - (y - low) / max(.01, high - low)
    return 1 + strength * (.3 + .7 * fall) * math.cos(8 * angle + .35 * fall)


def tune_material(bsdf, name, metallic, roughness):
    """Keep PBR values glTF-exportable; broad highlights distinguish material classes."""
    name = name.lower()
    textile = any(word in name for word in ('cloth', 'linen', 'violet', 'blue', 'robe', 'hood', 'cape'))
    organic = any(word in name for word in ('skin', 'bone', 'ivory'))
    if textile:
        bsdf.inputs['Metallic'].default_value = 0
        bsdf.inputs['Roughness'].default_value = max(.79, roughness)
        bsdf.inputs['Sheen Weight'].default_value = .16
        bsdf.inputs['Sheen Roughness'].default_value = .8
    elif 'leather' in name:
        bsdf.inputs['Roughness'].default_value = .72
        bsdf.inputs['Specular IOR Level'].default_value = .28
    elif organic:
        bsdf.inputs['Roughness'].default_value = max(.63, roughness)
        bsdf.inputs['Specular IOR Level'].default_value = .26
    elif metallic >= .4:
        bsdf.inputs['Roughness'].default_value = max(.32, roughness)
        bsdf.inputs['Metallic'].default_value = max(.65, metallic)
