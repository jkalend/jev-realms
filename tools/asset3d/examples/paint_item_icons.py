#!/usr/bin/env python3
"""Paint the item icon plate set for the Laya Realms inventory.

Every icon corresponds to a real `core::model::Item` (including the tiered
`Weapon(u8)` / `Armour(u8)` and the indexed `Key` / `Delivery` / `Glyph`
variants), drawn for the atlas contract: transparent plate, chunky silhouette,
one ink outline, palette drawn from the UI chrome so icons sit beside the
panels instead of fighting them.

Run:
    python tools/asset3d/examples/paint_item_icons.py \
      --output docs/gfx/proto/ui-v1/items/items.png
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path

from PIL import Image, ImageDraw

CELL = 48
COLUMNS = 8
S = 4  # supersample factor: draw at 4x, downsample for antialiased edges

BRASS = (177, 131, 62, 255)
BRASS_HI = (232, 191, 89, 255)
BRASS_LO = (102, 72, 37, 255)
STEEL = (146, 158, 170, 255)
STEEL_LO = (86, 95, 106, 255)
STEEL_HI = (208, 219, 229, 255)
STONE = (66, 66, 61, 255)
STONE_DARK = (44, 42, 39, 255)
INK = (9, 11, 12, 255)
BONE = (214, 205, 172, 255)
SEA = (102, 212, 196, 255)
HP_RED = (168, 44, 40, 255)
MANA_BLUE = (64, 92, 178, 255)
VIOLET = (126, 74, 150, 255)
MOSS = (108, 142, 74, 255)
LEATHER = (128, 88, 56, 255)
CLOTH = (122, 106, 92, 255)


def _mul(value: float) -> int:
    return int(round(value * S))


class Plate:
    """A 48x48 icon drawn on a 4x canvas in 48-space coordinates."""

    def __init__(self) -> None:
        self.img = Image.new("RGBA", (CELL * S, CELL * S), (0, 0, 0, 0))
        self.d = ImageDraw.Draw(self.img)

    def _pts(self, points) -> list[tuple[int, int]]:
        return [(_mul(x), _mul(y)) for x, y in points]

    def poly(self, points, fill, outline=INK, width=1.6) -> None:
        self.d.polygon(self._pts(points), fill=fill, outline=outline, width=_mul(width) or 1)

    def ell(self, box, fill, outline=INK, width=1.6) -> None:
        self.d.ellipse([_mul(v) for v in box], fill=fill, outline=outline, width=_mul(width) or 1)

    def rect(self, box, fill, outline=INK, width=1.6) -> None:
        self.d.rectangle([_mul(v) for v in box], fill=fill, outline=outline, width=_mul(width) or 1)

    def rrect(self, box, radius, fill, outline=INK, width=1.6) -> None:
        self.d.rounded_rectangle(
            [_mul(v) for v in box], radius=_mul(radius), fill=fill, outline=outline, width=_mul(width) or 1
        )

    def line(self, points, fill, width=2.0) -> None:
        self.d.line(self._pts(points), fill=fill, width=_mul(width), joint="curve")

    def arc(self, box, start, end, fill, width=2.0) -> None:
        self.d.arc([_mul(v) for v in box], start, end, fill=fill, width=_mul(width))

    def dot(self, x, y, r, fill, outline=None) -> None:
        self.ell((x - r, y - r, x + r, y + r), fill, outline=outline, width=0.8)

    def out(self) -> Image.Image:
        return self.img.resize((CELL, CELL), Image.Resampling.LANCZOS)


# --------------------------------------------------------------------------- art


def _flask(p: Plate, liquid: tuple, tall: bool, band: bool) -> None:
    neck = 20 if tall else 24
    top = 12 if tall else 15
    body = [(16, top + 2), (32, top + 2), (32, 30), (34, 33), (34, 38), (14, 38), (14, 33), (16, 30)]
    p.poly(body, STONE, width=1.8)
    p.poly([(17, neck), (31, neck), (30, 29), (33, 33), (33, 37), (15, 37), (15, 33), (18, 29)], liquid, outline=None)
    p.poly([(16, top + 2), (32, top + 2), (32, top + 6), (16, top + 6)], liquid, outline=None)
    p.poly([(20, top - 4), (28, top - 4), (28, top + 2), (20, top + 2)], STONE, width=1.6)
    p.rect((19, top - 7, 29, top - 3), LEATHER, width=1.4)
    if band:
        p.rect((17, neck + 1, 31, neck + 4), BRASS_HI, width=1.0)
    p.line([(19, top + 8), (18, top + 15)], (255, 255, 255, 120), width=1.4)


def potion(p: Plate) -> None:
    _flask(p, SEA, tall=False, band=False)


def greater_potion(p: Plate) -> None:
    _flask(p, (86, 240, 214, 255), tall=True, band=True)


def anti_toxin(p: Plate) -> None:
    _flask(p, MOSS, tall=True, band=False)
    p.line([(24, 16), (24, 26)], BRASS_HI, width=2.2)
    p.line([(19, 21), (29, 21)], BRASS_HI, width=2.2)


def mana_tonic(p: Plate) -> None:
    _flask(p, MANA_BLUE, tall=True, band=True)
    for i, r in enumerate((7, 5, 3)):
        p.dot(24, 22 + i * 3, r, (150, 190, 255, 200), outline=None)


def _bundle(p: Plate, tall: bool, accent) -> None:
    h = 34 if tall else 30
    top = 16 if tall else 18
    p.rrect((12, top, 36, top + h), 4, CLOTH, width=1.8)
    p.rect((12, top + 9, 36, top + 13), LEATHER, width=1.2)
    p.rect((22, top, 26, top + h), LEATHER, width=1.2)
    p.line([(15, top + 2), (33, top + 2)], (200, 190, 172, 120), width=1.2)
    p.ell((20, top - 3, 28, top + 4), accent, width=1.4)


def ration(p: Plate) -> None:
    _bundle(p, tall=False, accent=BRASS)


def traveler_ration(p: Plate) -> None:
    _bundle(p, tall=True, accent=SEA)
    p.line([(30, 12), (36, 6), (34, 14)], BONE, width=1.4)


def torch(p: Plate) -> None:
    p.rect((22, 18, 26, 42), LEATHER, width=1.6)
    p.rect((20, 36, 28, 41), BRASS_LO, width=1.4)
    p.poly([(24, 2), (33, 16), (30, 26), (18, 26), (15, 16)], HP_RED, width=1.8)
    p.poly([(24, 7), (30, 17), (27, 24), (21, 24), (18, 17)], (232, 140, 48, 255), outline=None)
    p.poly([(24, 13), (27, 19), (24, 23), (21, 19)], BRASS_HI, outline=None)


def _sword(p: Plate, tier: int) -> None:
    blade_w = 5 + tier
    guard_h = 6 + tier
    p.poly([(22, 4), (26, 4), (26, 30), (22, 30)], STEEL, width=1.6)
    p.poly([(22, 6), (24, 6), (24, 28), (22, 28)], STEEL_HI, outline=None)
    p.poly([(24, 4), (26, 4), (26, 30), (24, 30)], STEEL_LO, outline=None)
    p.rect((24 - blade_w, 30, 24 + blade_w, 30 + guard_h), BRASS, width=1.6)
    if tier >= 1:
        p.rect((24 - blade_w, 30, 24 + blade_w, 32), BRASS_HI, width=0.8)
    p.rect((21, 30 + guard_h, 27, 41), LEATHER, width=1.4)
    p.ell((19, 39, 29, 45), BRASS, width=1.4)
    if tier >= 2:
        p.dot(24, 33 + guard_h / 2, 2.0, SEA, outline=INK)
        p.line([(24, 7), (24, 27)], (255, 255, 255, 90), width=1.0)
    if tier >= 3:
        p.poly([(24 - blade_w, 31), (24 - blade_w - 4, 24), (24 - blade_w + 1, 30)], BRASS_HI, width=1.2)
        p.poly([(24 + blade_w, 31), (24 + blade_w + 4, 24), (24 + blade_w - 1, 30)], BRASS_HI, width=1.2)
        p.line([(19, 8), (24, 5)], STEEL_HI, width=1.0)
        p.line([(29, 8), (24, 5)], STEEL_HI, width=1.0)


def _cuirass(p: Plate, tier: int) -> None:
    """Breastplate: wide pauldrons tapering to a flat waist, neck notch on top."""
    shoulder = 9 + tier
    waist = 6 + tier // 2
    body = [(24 - shoulder, 19), (24 + shoulder, 19), (24 + waist, 31), (24 + waist, 39), (24 - waist, 39), (24 - waist, 31)]
    p.poly(body, STEEL, width=1.8)
    p.poly(
        [
            (24 - shoulder + 1, 20), (24 - waist + 1, 31), (24 - waist + 1, 38),
            (24 - waist + 5, 38), (24 - waist + 5, 31), (24 - shoulder + 5, 20),
        ],
        STEEL_HI,
        outline=None,
    )
    p.line([(24, 21), (24, 37)], STEEL_LO, width=1.2)
    # Neck notch: the plate opens at the throat rather than boxing the head in.
    p.ell((19, 15, 29, 22), (0, 0, 0, 0))
    p.line([(24 - shoulder, 19), (24 - shoulder + 3, 15)], INK, width=1.4)
    p.line([(24 + shoulder, 19), (24 + shoulder - 3, 15)], INK, width=1.4)
    if tier >= 1:
        p.ell((24 - shoulder - 3, 14, 24 - shoulder + 5, 22), STEEL, width=1.6)
        p.ell((24 + shoulder - 5, 14, 24 + shoulder + 3, 22), STEEL, width=1.6)
    if tier >= 2:
        p.rect((24 - waist, 27, 24 + waist, 29), BRASS, width=0.8)
    if tier >= 3:
        p.dot(24, 24, 2.8, SEA)
        p.poly([(21, 15), (27, 15), (24, 6)], BRASS_HI, width=1.4)


def _key(p: Plate, bit: int, ring) -> None:
    p.ell((14, 8, 30, 24), STONE, width=2.2)
    p.ell((18, 12, 26, 20), (0, 0, 0, 0), outline=INK, width=1.0)
    p.rect((22, 22, 26, 42), BRASS, width=1.6)
    if bit == 0:
        p.rect((26, 34, 33, 38), BRASS, width=1.2)
        p.rect((26, 39, 30, 42), BRASS, width=1.2)
    elif bit == 1:
        p.ell((26, 32, 34, 40), BRASS, width=1.2)
    else:
        p.rect((26, 32, 32, 36), BRASS, width=1.2)
        p.rect((26, 37, 35, 41), BRASS, width=1.2)
    p.dot(24, 42, 2.4, ring)


def caravan_goods(p: Plate) -> None:
    p.rrect((10, 22, 38, 40), 3, LEATHER, width=1.8)
    p.rrect((14, 12, 34, 24), 3, LEATHER, width=1.8)
    p.line([(24, 12), (24, 40)], (222, 196, 150, 255), width=1.6)
    p.line([(11, 30), (37, 30)], (222, 196, 150, 255), width=1.6)
    p.rect((22, 28, 26, 33), BRASS_HI, width=1.0)


def contraband(p: Plate) -> None:
    p.rrect((10, 16, 38, 40), 3, CLOTH, width=1.8)
    p.line([(10, 22), (38, 22)], (150, 40, 36, 255), width=1.4)
    p.dot(24, 31, 5.0, HP_RED)
    p.dot(24, 31, 2.2, (120, 22, 20, 255), outline=None)
    p.poly([(19, 26), (29, 26), (24, 21)], LEATHER, width=1.2)


def relic(p: Plate) -> None:
    p.poly([(16, 42), (14, 26), (18, 12), (30, 12), (34, 26), (32, 42)], STONE, width=1.8)
    p.poly([(19, 38), (18, 26), (21, 16), (27, 16), (30, 26), (29, 38)], STONE_DARK, outline=None)
    p.dot(21, 24, 2.0, VIOLET)
    p.dot(27, 24, 2.0, VIOLET)
    p.line([(18, 42), (32, 42)], INK, width=2.0)
    p.line([(20, 14), (28, 10)], (110, 106, 98, 255), width=1.2)


def first_writ(p: Plate) -> None:
    p.poly([(10, 14), (38, 14), (38, 34), (10, 34)], BONE, width=1.8)
    p.poly([(10, 14), (38, 14), (38, 17), (10, 17)], (232, 224, 200, 255), outline=None)
    for y in (21, 25, 29):
        p.line([(14, y), (34, y)], (120, 108, 84, 255), width=1.2)
    p.dot(24, 38, 5.0, HP_RED)
    p.dot(24, 38, 2.0, (120, 22, 20, 255), outline=None)
    p.line([(38, 14), (42, 10)], STONE, width=1.6)


def essence(p: Plate) -> None:
    p.dot(24, 24, 11.0, VIOLET)
    p.dot(24, 24, 7.0, (168, 110, 200, 255), outline=None)
    p.dot(22, 22, 3.0, (232, 210, 245, 255), outline=None)
    for angle, r in ((-1.9, 13), (-1.1, 14), (-0.3, 13)):
        x = 24 + r * (angle + 1.57)
        y = 24 + r * 0.9
        p.dot(x, y, 1.6, (150, 96, 186, 255), outline=None)


def gem_dust(p: Plate) -> None:
    p.poly([(10, 40), (38, 40), (34, 30), (14, 30)], STONE, width=1.6)
    for x, y, r in ((18, 28, 3.0), (24, 24, 3.6), (30, 28, 2.6), (21, 20, 2.2), (28, 18, 2.4)):
        p.poly([(x, y - r), (x + r, y), (x, y + r), (x - r, y)], (120, 190, 220, 255), width=1.0)


def herb_cluster(p: Plate) -> None:
    p.rect((22, 24, 26, 42), MOSS, width=1.4)
    for dy, s in ((0, 1.0), (6, 0.86), (12, 0.7)):
        p.ell((22 - 12 * s - 2, 24 + dy - 3, 22 - 1, 24 + dy + 3), (128, 164, 88, 255), width=1.0)
        p.ell((26, 24 + dy - 3, 26 + 12 * s + 2, 24 + dy + 3), (128, 164, 88, 255), width=1.0)
    p.dot(24, 24, 4.0, (168, 200, 120, 255))


def ore_flake(p: Plate) -> None:
    p.poly([(12, 34), (16, 16), (30, 12), (38, 24), (34, 38), (20, 40)], STONE, width=1.8)
    p.poly([(18, 30), (22, 18), (30, 20), (26, 32)], STEEL_LO, outline=None)
    p.poly([(22, 24), (27, 20), (30, 27), (24, 30)], STEEL, outline=None)
    p.line([(14, 36), (36, 30)], (150, 146, 138, 255), width=1.0)


def glyph_shard(p: Plate) -> None:
    p.poly([(24, 4), (36, 20), (24, 44), (12, 20)], STONE, width=1.8)
    p.poly([(24, 10), (31, 20), (24, 37), (17, 20)], STONE_DARK, outline=None)
    p.line([(24, 14), (24, 32)], SEA, width=1.8)
    p.line([(18, 22), (30, 22)], SEA, width=1.8)
    p.line([(20, 28), (28, 28)], (102, 180, 168, 255), width=1.4)


def delivery(p: Plate, ribbon) -> None:
    p.rrect((11, 15, 37, 41), 3, BONE, width=1.8)
    p.line([(11, 28), (37, 28)], (150, 138, 112, 255), width=1.2)
    p.rect((22, 15, 26, 41), ribbon, width=1.2)
    p.dot(24, 28, 4.0, ribbon)
    p.poly([(22, 15), (26, 15), (24, 9)], ribbon, width=1.0)


# --- boss relics -------------------------------------------------------------


def rallybreaker(p: Plate) -> None:
    p.ell((8, 16, 26, 34), BRASS, width=1.8)
    p.ell((12, 20, 22, 30), (0, 0, 0, 0), outline=BRASS_LO, width=1.4)
    p.poly([(24, 18), (40, 12), (40, 24), (24, 32)], (150, 44, 40, 255), width=1.6)
    p.line([(27, 20), (37, 16)], (198, 74, 68, 255), width=1.0)
    p.rect((20, 30, 26, 42), BRASS_LO, width=1.4)


def fangmantle(p: Plate) -> None:
    p.poly([(8, 18), (40, 18), (36, 38), (12, 38)], (86, 72, 62, 255), width=1.8)
    for i, x in enumerate((12, 19, 26, 33)):
        p.poly([(x, 36), (x + 4, 44), (x + 8, 36)], BONE, width=1.2)
    for x, h in ((13, 10), (24, 14), (35, 10)):
        p.poly([(x, 18), (x + 4, 18 - h), (x + 8, 18)], BONE, width=1.2)
    p.dot(24, 26, 3.0, HP_RED)


def graveglass(p: Plate) -> None:
    p.ell((8, 10, 40, 42), STONE_DARK, width=2.0)
    p.ell((12, 14, 36, 38), (58, 84, 74, 255), width=1.6)
    p.poly([(24, 16), (31, 26), (24, 36), (17, 26)], (110, 200, 186, 255), width=1.0)
    p.line([(14, 18), (22, 30)], (12, 20, 20, 255), width=1.6)
    p.line([(34, 18), (26, 30)], (12, 20, 20, 255), width=1.6)
    p.line([(19, 34), (30, 22)], (12, 20, 20, 255), width=1.2)


def saltcrown(p: Plate) -> None:
    p.poly([(8, 34), (10, 22), (16, 26), (20, 14), (26, 26), (30, 16), (34, 26), (40, 22), (40, 34)], BONE, width=1.8)
    p.rect((9, 33, 39, 39), (176, 190, 196, 255), width=1.4)
    for x in (13, 21, 29, 36):
        p.dot(x, 36, 1.6, (232, 240, 244, 255), outline=None)


def stoneheart(p: Plate) -> None:
    p.poly([(24, 40), (10, 26), (12, 14), (24, 8), (36, 14), (38, 26)], (92, 88, 80, 255), width=1.8)
    p.poly([(24, 34), (15, 25), (16, 17), (24, 13), (32, 17), (33, 25)], (126, 120, 108, 255), outline=None)
    p.line([(24, 12), (21, 22), (26, 30)], INK, width=1.6)
    p.line([(20, 24), (29, 24)], (72, 68, 62, 255), width=1.2)


def gnawbonecrown(p: Plate) -> None:
    p.poly([(8, 32), (10, 24), (15, 30), (19, 16), (25, 30), (30, 18), (34, 30), (39, 24), (40, 32)], BONE, width=1.8)
    p.rect((9, 31, 39, 37), (198, 188, 158, 255), width=1.4)
    for x, h in ((14, 7), (24, 11), (34, 7)):
        p.poly([(x, 30), (x + 2, 30 - h), (x + 4, 30)], (120, 34, 30, 255), width=1.0)


def tollcoincharm(p: Plate) -> None:
    p.ell((9, 14, 33, 38), BRASS, width=1.8)
    p.ell((14, 19, 28, 33), BRASS_LO, width=1.4)
    p.poly([(20, 20), (28, 26), (20, 32)], BRASS_HI, outline=None)
    p.line([(28, 24), (42, 18), (40, 26)], BRASS_HI, width=2.0)
    p.line([(30, 36), (30, 44)], BRASS_LO, width=1.6)
    p.dot(30, 44, 2.4, BRASS_HI)


def wisplightlantern(p: Plate) -> None:
    p.poly([(16, 8), (32, 8), (36, 16), (36, 36), (12, 36), (12, 16)], BRASS_LO, width=1.8)
    p.rect((17, 17, 31, 32), (120, 226, 206, 255), width=1.2)
    p.dot(24, 24, 5.0, (222, 250, 244, 255), outline=None)
    p.poly([(20, 8), (28, 8), (24, 2)], BRASS, width=1.4)
    p.rect((10, 36, 38, 41), BRASS, width=1.4)


def hartshorn(p: Plate) -> None:
    p.ell((14, 22, 34, 42), BONE, width=1.8)
    p.dot(20, 32, 2.4, INK, outline=None)
    p.dot(28, 32, 2.4, INK, outline=None)
    for side in (-1, 1):
        base = 24 + side * 6
        p.line([(base, 24), (base + side * 8, 14), (base + side * 12, 6)], BONE, width=2.4)
        p.line([(base + side * 8, 14), (base + side * 13, 16)], BONE, width=1.8)
    p.line([(18, 40), (30, 40)], (176, 166, 140, 255), width=1.2)


# --- seal glyphs -------------------------------------------------------------

def _glyph_tab(p: Plate) -> None:
    p.poly([(24, 6), (40, 18), (40, 32), (24, 44), (8, 32), (8, 18)], STONE, width=1.8)
    p.poly([(24, 11), (35, 19), (35, 31), (24, 39), (13, 31), (13, 19)], STONE_DARK, outline=None)


def glyph_ash(p: Plate) -> None:
    _glyph_tab(p)
    for dy in (0, 5, 10):
        p.line([(17, 20 + dy), (24, 26 + dy), (31, 20 + dy)], BRASS_HI, width=2.0)


def glyph_fen(p: Plate) -> None:
    _glyph_tab(p)
    for dy in (0, 7, 14):
        p.line([(15, 19 + dy), (20, 23 + dy), (24, 19 + dy), (28, 23 + dy), (33, 19 + dy)], SEA, width=2.0)


def glyph_gate(p: Plate) -> None:
    _glyph_tab(p)
    p.line([(17, 32), (17, 22), (24, 17), (31, 22), (31, 32)], BRASS_HI, width=2.2)
    p.line([(14, 32), (34, 32)], BRASS, width=1.8)


def glyph_seal(p: Plate) -> None:
    _glyph_tab(p)
    p.ell((16, 16, 32, 32), (0, 0, 0, 0), outline=HP_RED, width=2.4)
    p.line([(18, 32), (30, 18)], HP_RED, width=2.0)


def glyph_hart(p: Plate) -> None:
    _glyph_tab(p)
    p.line([(24, 34), (24, 22)], BRASS_HI, width=2.0)
    p.line([(24, 22), (18, 17), (15, 21)], BRASS_HI, width=2.0)
    p.line([(24, 22), (30, 17), (33, 21)], BRASS_HI, width=2.0)
    p.line([(18, 17), (16, 12)], BRASS, width=1.4)
    p.line([(30, 17), (32, 12)], BRASS, width=1.4)


def glyph_crown(p: Plate) -> None:
    _glyph_tab(p)
    p.line([(16, 30), (18, 18), (24, 24), (30, 18), (32, 30)], BRASS_HI, width=2.2)
    p.line([(15, 32), (33, 32)], BRASS, width=1.8)


def _tier(fn, tier: int) -> None:
    def draw(p: Plate) -> None:
        fn(p, tier)
    return draw


ITEMS: list[tuple[str, object]] = [
    ("Potion", potion),
    ("GreaterPotion", greater_potion),
    ("Ration", ration),
    ("TravelerRation", traveler_ration),
    ("Torch", torch),
    ("AntiToxin", anti_toxin),
    ("ManaTonic", mana_tonic),
    ("Essence", essence),
    ("Weapon.Worn", _tier(_sword, 0)),
    ("Weapon.Standard", _tier(_sword, 1)),
    ("Weapon.Fine", _tier(_sword, 2)),
    ("Weapon.Masterwork", _tier(_sword, 3)),
    ("Armour.Worn", _tier(_cuirass, 0)),
    ("Armour.Standard", _tier(_cuirass, 1)),
    ("Armour.Fine", _tier(_cuirass, 2)),
    ("Armour.Masterwork", _tier(_cuirass, 3)),
    ("Key.Burrow", lambda p: _key(p, 0, BRASS_HI)),
    ("Key.CrimsonHollow", lambda p: _key(p, 1, HP_RED)),
    ("Key.Underkeep", lambda p: _key(p, 2, SEA)),
    ("CaravanGoods", caravan_goods),
    ("Contraband", contraband),
    ("Relic", relic),
    ("FirstWrit", first_writ),
    ("GemDust", gem_dust),
    ("HerbCluster", herb_cluster),
    ("OreFlake", ore_flake),
    ("GlyphShard", glyph_shard),
    ("Delivery.Millbrook", lambda p: delivery(p, (150, 90, 60, 255))),
    ("Delivery.Highgate", lambda p: delivery(p, (70, 96, 150, 255))),
    ("Delivery.Saltmarsh", lambda p: delivery(p, (90, 140, 120, 255))),
    ("Glyph.Ash", glyph_ash),
    ("Glyph.Fen", glyph_fen),
    ("Glyph.Gate", glyph_gate),
    ("Glyph.Seal", glyph_seal),
    ("Glyph.Hart", glyph_hart),
    ("Glyph.Crown", glyph_crown),
    ("BossRelic.Rallybreaker", rallybreaker),
    ("BossRelic.Fangmantle", fangmantle),
    ("BossRelic.Graveglass", graveglass),
    ("BossRelic.Saltcrown", saltcrown),
    ("BossRelic.Stoneheart", stoneheart),
    ("BossRelic.GnawboneCrown", gnawbonecrown),
    ("BossRelic.TollcoinCharm", tollcoincharm),
    ("BossRelic.WisplightLantern", wisplightlantern),
    ("BossRelic.Hartshorn", hartshorn),
]

def paint() -> tuple[Image.Image, dict[str, tuple[int, int]]]:
    rows = (len(ITEMS) + COLUMNS - 1) // COLUMNS
    sheet = Image.new("RGBA", (COLUMNS * CELL, rows * CELL), (0, 0, 0, 0))
    cells: dict[str, tuple[int, int]] = {}
    for index, (key, painter) in enumerate(ITEMS):
        col, row = index % COLUMNS, index // COLUMNS
        plate = Plate()
        painter(plate)
        sheet.alpha_composite(plate.out(), (col * CELL, row * CELL))
        cells[key] = (col, row)
    return sheet, cells


def manifest(cells: dict[str, tuple[int, int]]) -> dict:
    return {
        "sheet": "items.png",
        "cell_size": {"width": CELL, "height": CELL},
        "columns": COLUMNS,
        "items": [{"name": key, "col": col, "row": row} for key, (col, row) in cells.items()],
    }


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    sheet, cells = paint()
    args.output.parent.mkdir(parents=True, exist_ok=True)
    sheet.save(args.output)
    manifest_path = args.output.with_suffix(".json")
    manifest_path.write_text(json.dumps(manifest(cells), indent=2) + "\n", encoding="utf-8")
    print("item-icons", args.output, len(ITEMS), "cells", manifest_path)


if __name__ == "__main__":
    main()
