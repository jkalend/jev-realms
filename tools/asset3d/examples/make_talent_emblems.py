#!/usr/bin/env python3
"""Paint the 18 order/branch seals used by the live oath-tree diagram.

Run from the repository root: python tools/asset3d/examples/make_talent_emblems.py
The 3-column x 6-row sheet follows Class::from_index order; each 64px cell
is transparent outside its medallion so Bevy can tint locked/unlocked nodes.
"""
from pathlib import Path
from PIL import Image, ImageDraw

SIZE, SCALE = 64, 4
PALE = (227, 217, 185, 255)
ORDERS = [
    ("shield", "blade", "beacon"),
    ("tomb", "censer", "barrow"),
    ("daggers", "hook", "wave"),
    ("bridle", "scales", "compass"),
    ("lightning", "quill", "eye"),
    ("reed", "mire", "antler"),
]
ACCENTS = [(153, 184, 198), (173, 154, 178), (204, 126, 107),
           (187, 160, 114), (166, 137, 213), (150, 181, 118)]


def seal(motif: str, accent: tuple[int, int, int]) -> Image.Image:
    im = Image.new("RGBA", (SIZE * SCALE, SIZE * SCALE))
    d = ImageDraw.Draw(im)
    def box(a, color, width=0):
        xy = tuple(round(v * SCALE) for v in a)
        d.ellipse(xy, outline=color if width else None,
                  fill=None if width else color, width=width * SCALE)
    def path(points, color=PALE, width=2, close=False):
        p = [(round(x*SCALE), round(y*SCALE)) for x, y in points]
        d.line(p + ([p[0]] if close else []), fill=color, width=width*SCALE, joint="curve")
    def poly(points, color=PALE):
        d.polygon([(round(x*SCALE), round(y*SCALE)) for x, y in points], fill=color)

    base = (15, 19, 26, 242)
    metal = (*accent, 255)
    box((3, 3, 61, 61), (13, 17, 23, 185))
    box((5, 5, 59, 59), metal, 2)
    box((9, 9, 55, 55), (82, 75, 61, 255), 1)
    box((11, 11, 53, 53), base)
    poly([(32, 2), (35, 7), (32, 12), (29, 7)], metal)
    poly([(32, 52), (35, 57), (32, 62), (29, 57)], metal)
    for x in (5, 57):
        box((x-1, 31, x+1, 33), metal)

    if motif == "shield":
        path([(32, 15), (46, 20), (44, 36), (32, 49), (20, 36), (18, 20)], PALE, 3, True)
        path([(32, 20), (32, 43)], metal, 2)
        path([(24, 30), (40, 30)], metal, 2)
    elif motif == "blade":
        poly([(24, 46), (39, 18), (46, 14), (42, 23), (27, 49)], PALE)
        path([(18, 39), (32, 47)], metal, 3)
        path([(21, 48), (25, 52)], PALE, 3)
    elif motif == "beacon":
        path([(21, 47), (43, 47), (40, 24), (24, 24)], PALE, 2, True)
        path([(25, 30), (39, 30), (39, 40), (25, 40)], metal, 2, True)
        poly([(29, 27), (32, 14), (35, 27)], PALE)
        path([(32, 46), (32, 39)], PALE, 2)
    elif motif == "tomb":
        path([(19, 49), (19, 30), (22, 22), (32, 17), (42, 22), (45, 30), (45, 49)], PALE, 3)
        path([(26, 34), (38, 34), (32, 25), (32, 43)], metal, 2)
        path([(15, 49), (49, 49)], PALE, 2)
    elif motif == "censer":
        path([(18, 31), (22, 43), (42, 43), (46, 31)], PALE, 3)
        path([(20, 31), (44, 31)], metal, 2)
        path([(28, 25), (26, 19), (30, 14)], PALE, 2)
        path([(35, 25), (38, 18), (35, 14)], PALE, 2)
        path([(32, 43), (32, 50)], metal, 2)
    elif motif == "barrow":
        path([(15, 42), (23, 33), (31, 29), (42, 35), (49, 43)], PALE, 3)
        path([(15, 44), (49, 44)], metal, 2)
        box((27, 17, 37, 27), PALE, 2)
        path([(32, 19), (32, 25)], metal, 1)
    elif motif == "daggers":
        for mirror in (False, True):
            m = (lambda x: 64-x) if mirror else (lambda x: x)
            path([(m(18), 46), (m(29), 19), (m(27), 16)], PALE, 3)
            path([(m(14), 34), (m(27), 38)], metal, 2)
    elif motif == "hook":
        path([(22, 17), (39, 17), (42, 20), (42, 38), (39, 44), (31, 46), (26, 42)], PALE, 3)
        path([(22, 17), (22, 48)], metal, 3)
        path([(18, 47), (27, 47)], PALE, 2)
    elif motif == "wave":
        path([(15, 38), (22, 27), (28, 31), (36, 22), (44, 27), (48, 33)], PALE, 3)
        path([(15, 46), (24, 42), (33, 45), (43, 39), (50, 41)], metal, 3)
        box((36, 14, 40, 18), PALE)
    elif motif == "bridle":
        path([(21, 19), (20, 38), (26, 47), (38, 47), (44, 38), (43, 19)], PALE, 3)
        path([(21, 30), (43, 30)], metal, 2)
        box((27, 34, 37, 43), PALE, 2)
    elif motif == "scales":
        path([(32, 16), (32, 47), (22, 47), (42, 47)], PALE, 3)
        path([(17, 27), (47, 27)], metal, 3)
        path([(20, 27), (17, 41), (23, 41), (20, 27)], PALE, 2)
        path([(44, 27), (41, 41), (47, 41), (44, 27)], PALE, 2)
    elif motif == "compass":
        box((15, 15, 49, 49), PALE, 2)
        poly([(32, 18), (36, 32), (32, 46), (28, 32)], metal)
        box((30, 30, 34, 34), PALE)
    elif motif == "lightning":
        poly([(34, 13), (21, 34), (30, 34), (27, 51), (44, 27), (34, 27)], PALE)
        path([(34, 13), (21, 34), (30, 34)], metal, 2)
    elif motif == "quill":
        path([(20, 47), (40, 17), (47, 15), (45, 27), (22, 48)], PALE, 3)
        path([(22, 46), (38, 30)], metal, 2)
        path([(18, 50), (43, 50)], PALE, 2)
    elif motif == "eye":
        path([(15, 32), (24, 23), (32, 20), (40, 23), (49, 32), (40, 41), (32, 44), (24, 41), (15, 32)], PALE, 3)
        box((26, 26, 38, 38), metal)
        box((30, 30, 34, 34), base)
    elif motif == "reed":
        path([(26, 49), (27, 20), (18, 15)], PALE, 2)
        path([(37, 49), (36, 22), (45, 16)], PALE, 2)
        poly([(21, 17), (15, 15), (23, 27)], metal)
        poly([(42, 18), (49, 16), (41, 28)], metal)
        path([(17, 45), (47, 45)], PALE, 2)
    elif motif == "mire":
        path([(14, 38), (23, 35), (31, 39), (40, 34), (50, 39)], PALE, 3)
        path([(18, 46), (28, 43), (37, 46), (46, 44)], metal, 2)
        box((27, 18, 37, 28), PALE, 2)
        path([(32, 28), (32, 38)], PALE, 2)
    else:  # antler
        path([(32, 49), (32, 27), (23, 20), (18, 14)], PALE, 3)
        path([(32, 27), (41, 20), (46, 14)], PALE, 3)
        path([(24, 21), (19, 27), (17, 21)], metal, 2)
        path([(40, 21), (45, 27), (47, 21)], metal, 2)
        path([(28, 46), (36, 46)], PALE, 2)
    return im.resize((SIZE, SIZE), Image.Resampling.LANCZOS)


def main() -> None:
    sheet = Image.new("RGBA", (SIZE * 3, SIZE * len(ORDERS)))
    for row, (motifs, accent) in enumerate(zip(ORDERS, ACCENTS)):
        for col, motif in enumerate(motifs):
            sheet.alpha_composite(seal(motif, accent), (col * SIZE, row * SIZE))
    out = Path(__file__).resolve().parents[3] / "assets/ui/talent-emblems.png"
    out.parent.mkdir(parents=True, exist_ok=True)
    sheet.save(out)
    print(out)


if __name__ == "__main__":
    main()
