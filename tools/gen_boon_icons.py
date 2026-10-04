"""Draw the three oathroad gift emblems into assets/ui/boon-icons.png.

Run with Python and Pillow; each transparent 128px tile is addressed by boon index.
"""
from pathlib import Path
from PIL import Image, ImageDraw

SCALE = 4
TILE = 128
GOLD = (207, 171, 103, 255)
BRIGHT = (244, 224, 176, 255)
SEA = (119, 180, 184, 255)
SHADE = (19, 32, 38, 255)
INK = (4, 12, 18, 240)

sheet = Image.new("RGBA", (TILE * 3 * SCALE, TILE * SCALE))


def emblem(index, draw_symbol, tint):
    image = Image.new("RGBA", (TILE * SCALE, TILE * SCALE))
    d = ImageDraw.Draw(image)

    def line(points, color=tint, width=2):
        d.line([(round(x * SCALE), round(y * SCALE)) for x, y in points], fill=color,
               width=width * SCALE, joint="curve")

    def ellipse(box, fill=None, outline=None, width=2):
        d.ellipse(tuple(round(v * SCALE) for v in box), fill=fill, outline=outline,
                  width=width * SCALE)

    def polygon(points, fill=None, outline=None, width=2):
        scaled = [(round(x * SCALE), round(y * SCALE)) for x, y in points]
        d.polygon(scaled, fill=fill)
        if outline:
            d.line(scaled + [scaled[0]], fill=outline, width=width * SCALE, joint="curve")

    # Twin engraved rings and tiny cardinal studs, shared with the talent heraldry.
    ellipse((13, 13, 115, 115), outline=tint, width=2)
    ellipse((19, 19, 109, 109), outline=(*tint[:3], 110), width=1)
    for x, y in ((64, 10), (118, 64), (64, 118), (10, 64)):
        polygon(((x, y - 4), (x + 4, y), (x, y + 4), (x - 4, y)), fill=BRIGHT)
    draw_symbol(line, ellipse, polygon)
    sheet.alpha_composite(image, (index * TILE * SCALE, 0))


def pouch(line, ellipse, polygon):
    # Drawstring, metal clasp and weighted leather bag with stitched side seams.
    polygon(((43, 43), (85, 43), (90, 55), (92, 81), (82, 96), (46, 96), (36, 81), (38, 55)),
            INK, GOLD, 3)
    line(((42, 56), (64, 62), (86, 56)), BRIGHT, 2)
    line(((43, 46), (35, 32), (48, 37), (55, 43)), GOLD, 3)
    line(((85, 46), (93, 32), (80, 37), (73, 43)), GOLD, 3)
    ellipse((55, 37, 73, 52), SHADE, BRIGHT, 2)
    ellipse((51, 67, 77, 89), SHADE, GOLD, 2)
    line(((64, 70), (64, 85)), BRIGHT, 2)
    line(((58, 76), (69, 76), (72, 81), (68, 84), (58, 84)), BRIGHT, 2)
    line(((42, 64), (44, 82), (49, 87)), GOLD, 1)
    line(((86, 64), (84, 82), (79, 87)), GOLD, 1)
    for x, y in ((45, 60), (83, 60), (46, 90), (82, 90)):
        ellipse((x - 1, y - 1, x + 1, y + 1), BRIGHT)


def marches(line, ellipse, polygon):
    # Paired greaves heading along a winding oathroad, with a compass above.
    polygon(((64, 27), (60, 35), (64, 46), (68, 35)), SEA, BRIGHT, 1)
    line(((45, 43), (64, 39), (83, 43)), SEA, 2)
    polygon(((43, 48), (57, 48), (56, 74), (63, 83), (61, 93), (37, 93), (34, 83), (43, 77)),
            INK, SEA, 3)
    polygon(((71, 48), (85, 48), (85, 77), (94, 83), (91, 93), (67, 93), (65, 83), (72, 74)),
            INK, SEA, 3)
    line(((42, 63), (56, 63)), BRIGHT, 2)
    line(((71, 63), (85, 63)), BRIGHT, 2)
    line(((38, 87), (60, 87)), BRIGHT, 2)
    line(((68, 87), (90, 87)), BRIGHT, 2)
    line(((57, 102), (63, 98), (69, 102)), SEA, 2)


def nothing(line, ellipse, polygon):
    # Empty open seal: the path through it continues beyond the circle.
    ellipse((38, 37, 90, 89), outline=GOLD, width=3)
    ellipse((46, 45, 82, 81), outline=(*GOLD[:3], 150), width=1)
    polygon(((63, 28), (68, 36), (63, 43), (58, 36)), SHADE, BRIGHT, 2)
    line(((32, 97), (46, 85), (57, 75), (71, 61), (82, 48), (96, 30)), BRIGHT, 3)
    line(((39, 101), (54, 91), (66, 80), (79, 66), (90, 51)), GOLD, 1)
    for x, y in ((32, 97), (96, 30)):
        ellipse((x - 3, y - 3, x + 3, y + 3), SHADE, BRIGHT, 1)


emblem(0, pouch, GOLD)
emblem(1, marches, SEA)
emblem(2, nothing, GOLD)
out = Path(__file__).resolve().parents[1] / "assets/ui/boon-icons.png"
sheet.resize((TILE * 3, TILE), Image.Resampling.LANCZOS).save(out)
print(out)
