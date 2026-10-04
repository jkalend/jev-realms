//! Tile painters — the D19 plates for every `model::Tile` variant.
//!
//! All plates are a direct port of the C/C+ de-blocked arm in
//! src/gfxlab.rs (artstyle_deblocked, the D28-ratified target read):
//! grim painterly environment (`grim()` = paint_terrain), deterministic
//! hash grain, edge breakup, organic ridge/canopy silhouettes and
//! rubble-step walls. Square sheet: 64x64 full-bleed. Iso sheet: 64x64
//! cells, three face rows (top/left/right), shared ground-diamond anchor
//! A = (32, 20) so flat tiles and blocks composite identically.

use crate::raster::*;
use image::{Rgba, RgbaImage};
use laya_realms::model::Tile;

pub const TILE: u32 = 64;

/// Iso geometry (2:1). Cell 64x64; diamond anchor y=20 leaves room below
/// for the tallest block skirt (h=28 -> bottom edge lands exactly at 64).
const AX: f32 = 32.0;
const AY: f32 = 20.0;
const HW: f32 = 32.0;
const HH: f32 = 16.0;

/// sprites::terrain_color, verbatim (the game's existing palette).
pub fn terrain_rgb(t: Tile) -> Rgb {
    match t {
        Tile::Grass => [48, 76, 37],
        Tile::Forest => [32, 61, 31],
        Tile::DeepForest => [27, 52, 34],
        Tile::Road => [153, 126, 78],
        Tile::Floor => [44, 42, 39],
        Tile::Wall => [66, 66, 61],
        Tile::Mountain => [113, 114, 105],
        Tile::Rock => [50, 55, 55],
        Tile::River => [35, 82, 111],
        Tile::Ford => [111, 127, 111],
        Tile::Ruins => [109, 105, 80],
        Tile::Door | Tile::Chest => [151, 111, 46],
        Tile::Up | Tile::Down => [186, 174, 129],
        Tile::Shrine => [108, 153, 171],
    }
}

/// Plate base = grim painterly transform of the game palette.
fn base(t: Tile) -> Rgb {
    grim(terrain_rgb(t))
}

fn is_block(t: Tile) -> bool {
    matches!(t, Tile::Forest | Tile::DeepForest | Tile::Mountain | Tile::Wall)
}

/// Block skirt height in face cells (mock: wall 1.6*hh, else 2.2*hh,
/// clamped so the south point stays inside 64px).
fn block_height(t: Tile) -> f32 {
    match t {
        Tile::Wall => 20.0,
        _ => 28.0,
    }
}

// ---------------------------------------------------------------------------
// Shared grain / breakup / tuft ports (gfxlab grain, tufts, edge-dither).

/// Square-tile grain: 7 hashed speckles, 3px, mock's 0.82/0.94/1.06 ladder.
fn grain_sq(img: &mut RgbaImage, seed: u64, c: Rgb) {
    for i in 0..7u64 {
        let px = 2.0 + h01(seed, i * 2) * 59.0;
        let py = 2.0 + h01(seed, i * 2 + 1) * 59.0;
        let k = 0.82 + (i % 3) as f32 * 0.12;
        fill_rect(img, px, py, 3.0, 3.0, shade(c, k));
    }
}

/// Diamond-clipped grain (gfxlab::grain verbatim math, 2px at sheet scale).
fn grain_diamond(img: &mut RgbaImage, seed: u64, c: Rgb) {
    for i in 0..7u64 {
        let hx = h01(seed, i * 3);
        let hy = h01(seed, i * 3 + 1);
        let px = AX - HW + hx * HW * 2.0;
        let along = 1.0 - (hy * 2.0 - 1.0).abs();
        let py = AY - HH * along + hy * HH * 2.0 * along;
        let k = 0.82 + ((i * 37) % 3) as f32 * 0.12;
        fill_rect(img, px, py, 2.0, 2.0, shade(c, k));
    }
}

/// Scatter tufts on vegetated ground (gfxlab's tuft port, doubled).
fn tufts_sq(img: &mut RgbaImage, seed: u64, c: Rgb) {
    for i in 0..3u64 {
        let tx = 4.0 + h01(seed, 40 + i) * 54.0;
        let ty = 4.0 + h01(seed, 50 + i) * 54.0;
        let up = shade(c, 1.3);
        fill_rect(img, tx, ty - 4.0, 2.0, 4.0, up);
        fill_rect(img, tx + 2.0, ty, 3.0, 2.0, up);
    }
}

// Match every repeated ground family to its own shared substrate. Opposite
// sides sample the same periodic field, while distinct materials keep their
// natural boundary (a road crossing grass is a road edge, not a black grid).
pub fn edge_colour(t: Tile) -> Rgb {
    match t {
        Tile::Road => base(Tile::Road),
        Tile::Grass | Tile::Forest | Tile::DeepForest | Tile::Ruins => base(Tile::Grass),
        Tile::Mountain | Tile::Rock => base(Tile::Rock),
        Tile::River | Tile::Ford => shade(grim([34, 80, 95]), 1.16),
        Tile::Floor | Tile::Door | Tile::Chest | Tile::Shrine | Tile::Up | Tile::Down => grim([104, 99, 90]),
        Tile::Wall => grim([92, 92, 86]),
    }
}

/// Bleed a plate's own edge colour outward one pixel, leaving painted
/// pixels alone.
fn bleed_edge(img: &mut RgbaImage) {
    let (w, h) = (img.width() as i32, img.height() as i32);
    let src = img.clone();
    for y in 0..h {
        for x in 0..w {
            // A texel that is only PARTLY covered is as dangerous as a
            // transparent one. The bilinear kernel scales a texel's colour by
            // its alpha, so the rim of alpha 28/141 texels the supersampler
            // leaves along the diamond edge drew a dark lattice across every
            // ground field — the one artefact that survived flattening the
            // albedo, the height, the seam tint and the sprite bleed. Only a
            // fully opaque texel counts as painted.
            if src.get_pixel(x as u32, y as u32).0[3] == 255 {
                continue;
            }
            let (mut r, mut g, mut b, mut n) = (0u32, 0u32, 0u32, 0u32);
            for (dx, dy) in [(-1i32, 0i32), (1, 0), (0, -1), (0, 1)] {
                let (nx, ny) = (x + dx, y + dy);
                if nx < 0 || ny < 0 || nx >= w || ny >= h {
                    continue;
                }
                let p = src.get_pixel(nx as u32, ny as u32).0;
                if p[3] == 255 {
                    r += p[0] as u32;
                    g += p[1] as u32;
                    b += p[2] as u32;
                    n += 1;
                }
            }
            if n > 0 {
                img.put_pixel(
                    x as u32,
                    y as u32,
                    Rgba([(r / n) as u8, (g / n) as u8, (b / n) as u8, 255]),
                );
            }
        }
    }
}

/// Seam pass: bleed the plate's own colour outward past the diamond edge so
/// bilinear filtering never blends that edge against the black void.
///
/// It deliberately does NOT retint the edge toward a flat base colour: that
/// gave every tile a lighter outline, which read as squares.
///
/// Coverage alone is not enough either. Setting alpha on pixels the
/// heightfield never painted leaves their colour at (0,0,0), so an
/// alpha-only seam painted an opaque BLACK border around every diamond —
/// the dark lattice that survived flattening the ground albedo, the ground
/// height and the seam tint. The bleed has to carry the interior colour
/// outward, not just coverage.
const SEAM_BLEED: usize = 3;

fn seam_pixel(img: &mut RgbaImage, x: u32, y: u32, _material: Rgb) {
    img.get_pixel_mut(x, y).0[3] = 255;
}

pub fn seam_square(img: &mut RgbaImage, t: Tile) {
    let material = edge_colour(t);
    for y in 0..TILE {
        for x in 0..TILE {
            seam_pixel(img, x, y, material);
        }
    }
}

pub fn seam_iso(img: &mut RgbaImage, _t: Tile) {
    for _ in 0..SEAM_BLEED {
        bleed_edge(img);
    }
}

// ---------------------------------------------------------------------------
// Square plates (projection A).

fn sq_organic_blob(img: &mut RgbaImage, cx: f32, cy: f32, r: f32, dark: Rgb, lit: Rgb) {
    fill_circle(img, cx, cy, r, dark);
    fill_circle(img, cx - 1.5, cy - 2.0, r * 0.85, lit);
}

fn paint_square_detail(img: &mut RgbaImage, t: Tile, seed: u64, c: Rgb) {
    match t {
        Tile::Grass => {
            for i in 0..3u64 {
                let x = 4.0 + h01(seed, 200 + i) * 54.0;
                let y = 4.0 + h01(seed, 210 + i) * 54.0;
                fill_rect(img, x, y, 5.0 + (i % 2) as f32 * 3.0, 4.0, shade(c, 0.9));
            }
            tufts_sq(img, seed, c);
        }
        Tile::Forest | Tile::DeepForest => {
            let dark = shade(c, 0.55);
            let n = if t == Tile::DeepForest { 5 } else { 4 };
            for i in 0..n {
                let x = 10.0 + h01(seed, 220 + i as u64) * 44.0;
                let y = 10.0 + h01(seed, 230 + i as u64) * 44.0;
                sq_organic_blob(img, x, y, 9.0 - (i % 3) as f32, dark, c);
            }
            // Canopy glint (canopy()'s top accent).
            fill_circle(img, 32.0 - 3.0, 26.0, 4.5, shade(c, 1.25));
            tufts_sq(img, seed, c);
        }
        Tile::Mountain => {
            // Square crag: low-contrast facets + pale cap + rubble foot.
            // Hard single-triangle reads cheap at 64px; keep the ladder tight.
            let apex_x = 24.0 + h01(seed, 240) * 16.0;
            fill_poly(img, &[[6.0, 54.0], [apex_x, 12.0], [52.0, 56.0]], shade(c, 1.08));
            fill_poly(
                img,
                &[[6.0, 54.0], [apex_x, 12.0], [34.0, 58.0], [8.0, 58.0]],
                shade(c, 0.9),
            );
            fill_poly(
                img,
                &[[apex_x, 12.0], [apex_x - 6.0, 24.0], [apex_x + 7.0, 23.0]],
                shade(c, 1.3),
            );
            for i in 0..6u64 {
                let x = 8.0 + h01(seed, 245 + i) * 44.0;
                let y = 16.0 + h01(seed, 255 + i) * 38.0;
                fill_rect(img, x, y, 2.0, 2.0, shade(c, if i % 2 == 0 { 1.14 } else { 0.84 }));
            }
            for i in 0..5u64 {
                let x = 6.0 + h01(seed, 250 + i) * 52.0;
                fill_rect(img, x, 56.0 + h01(seed, 260 + i) * 5.0, 3.0, 2.0, shade(c, 0.85));
            }
        }
        Tile::Rock => {
            // Boulders as bumps: drop shadow, mass, northwest rim light.
            for i in 0..2u64 {
                let x = 16.0 + h01(seed, 270 + i) * 28.0;
                let y = 20.0 + h01(seed, 280 + i) * 22.0;
                fill_circle(img, x + 3.0, y + 4.0, 9.5, shade(c, 0.6));
                fill_circle(img, x, y, 9.0, shade(c, 1.05));
                fill_circle(img, x - 3.0, y - 3.0, 4.0, shade(c, 1.3));
                fill_circle(img, x + 4.0, y + 5.0, 3.0, shade(c, 0.75));
            }
        }
        Tile::River => {
            // Flow bands, neighbour-agnostic (banks dither via edge flecks).
            for k in 0..3u64 {
                let y = 9.0 + k as f32 * 18.0 + h01(seed, 290 + k) * 5.0;
                let mut x = 2.0 + h01(seed, 300 + k) * 6.0;
                while x < 58.0 {
                    let w = 10.0 + h01(seed, 310 + k * 7 + x as u64) * 8.0;
                    blend_rect(img, x, y, w, 2.0, shade(c, 1.3), 170);
                    x += w + 5.0;
                }
            }
            for i in 0..4u64 {
                let x = 4.0 + h01(seed, 320 + i) * 52.0;
                let y = 4.0 + h01(seed, 330 + i) * 52.0;
                blend_rect(img, x, y, 2.0, 2.0, [204, 224, 226], 160);
            }
        }
        Tile::Ford => {
            // Silt bar + stepping stones (walkable read).
            fill_poly(
                img,
                &[[0.0, 34.0], [TILE as f32, 28.0], [TILE as f32, 42.0], [0.0, 48.0]],
                shade(c, 1.14),
            );
            let stone = grim([150, 144, 130]);
            for i in 0..4u64 {
                let x = 8.0 + i as f32 * 14.0 + h01(seed, 340 + i) * 6.0;
                let y = 32.0 + (i % 2) as f32 * 6.0 - 2.0;
                fill_circle(img, x, y + 6.0, 5.5, stone);
                fill_rect(img, x - 5.0, y + 10.0, 10.0, 2.0, shade(stone, 0.6));
            }
        }
        Tile::Road => {
            for i in 0..6u64 {
                let x = 3.0 + h01(seed, 350 + i) * 54.0;
                let y = 3.0 + h01(seed, 360 + i) * 54.0;
                blend_rect(img, x, y, 4.0 + (i % 2) as f32 * 3.0, 3.0, shade(c, 0.86), 150);
            }
            // Cart-track dabs (subtle; tessellates in both directions).
            for i in 0..4u64 {
                let y = 6.0 + i as f32 * 14.0 + h01(seed, 370 + i) * 6.0;
                blend_rect(img, 21.0, y, 4.0, 3.0, shade(c, 0.78), 110);
                blend_rect(img, 39.0, y, 4.0, 3.0, shade(c, 0.78), 110);
            }
            for i in 0..5u64 {
                let x = 3.0 + h01(seed, 380 + i) * 56.0;
                let y = 3.0 + h01(seed, 390 + i) * 56.0;
                fill_rect(img, x, y, 2.0, 2.0, shade(c, 1.12));
            }
        }
        Tile::Ruins => {
            // sprites.rs ruin layout, doubled and grim-baked.
            let stone = c;
            fill_rect(img, 10.0, 16.0, 14.0, 36.0, shade(stone, 0.8));
            fill_rect(img, 10.0, 16.0, 6.0, 36.0, stone);
            fill_rect(img, 6.0, 12.0, 22.0, 8.0, shade(stone, 1.2));
            fill_rect(img, 10.0, 34.0, 14.0, 4.0, grim(terrain_rgb(Tile::Rock)));
            fill_rect(img, 44.0, 32.0, 12.0, 20.0, stone);
            fill_rect(img, 40.0, 28.0, 16.0, 6.0, shade(stone, 1.15));
            fill_rect(img, 28.0, 46.0, 10.0, 10.0, stone);
            for i in 0..5u64 {
                let x = 4.0 + h01(seed, 400 + i) * 56.0;
                let y = 4.0 + h01(seed, 410 + i) * 56.0;
                fill_rect(img, x, y, 3.0, 3.0, shade(stone, if i % 2 == 0 { 1.1 } else { 0.85 }));
            }
        }
        Tile::Wall => {
            // Rubble-step top, coursed stone with staggered ties.
            for (bx, bw) in [(4.0, 10.0), (24.0, 10.0), (44.0, 10.0)] {
                fill_rect(img, bx, 0.0, bw, 6.0, shade(c, 1.12));
                fill_rect(img, bx, 6.0, bw, 2.0, shade(c, 0.75));
            }
            for row in 0..4u64 {
                let y = 14.0 + row as f32 * 12.0;
                let mut x = 0.0;
                while x < 62.0 {
                    let seg = 8.0 + h01(seed, 420 + row * 8 + x as u64) * 6.0;
                    blend_rect(img, x, y, seg, 1.0, shade(c, 0.55), 190);
                    x += seg + 3.0;
                }
                for i in 0..3u64 {
                    let tx = h01(seed, 430 + row * 4 + i) * 60.0;
                    blend_rect(img, tx, y, 1.0, 12.0, shade(c, 0.6), 150);
                }
            }
        }
        Tile::Floor => {
            for gy in [16.0, 32.0, 48.0] {
                blend_rect(img, 0.0, gy, 64.0, 1.0, shade(c, 0.72), 150);
            }
            for row in 0..4u64 {
                let off = if row % 2 == 0 { 12.0 } else { 30.0 };
                for i in 0..2u64 {
                    let x = (off + i as f32 * 32.0) % 62.0;
                    blend_rect(img, x, row as f32 * 16.0, 1.0, 15.0, shade(c, 0.72), 150);
                }
            }
            for i in 0..6u64 {
                let x = h01(seed, 440 + i) * 60.0;
                let y = h01(seed, 450 + i) * 60.0;
                blend_rect(img, x, y, 4.0, 3.0, shade(c, 1.08), 90);
            }
        }
        Tile::Door => {
            paint_square_detail(img, Tile::Floor, seed, c);
            *img = door_plate(img, false, false, true);
        }
        Tile::Chest => {
            paint_square_detail(img, Tile::Floor, seed, c);
            let chest = grim(terrain_rgb(Tile::Chest));
            fill_rect(img, 6.0, 26.0, 54.0, 28.0, INK);
            fill_rect(img, 10.0, 22.0, 44.0, 28.0, chest);
            fill_rect(img, 14.0, 16.0, 36.0, 8.0, grim([180, 135, 66]));
            fill_rect(img, 10.0, 24.0, 44.0, 6.0, grim([166, 122, 53]));
            fill_rect(img, 48.0, 26.0, 6.0, 22.0, grim([102, 75, 36]));
            fill_rect(img, 29.0, 30.0, 6.0, 10.0, grim([177, 131, 62]));
        }
        Tile::Shrine => {
            paint_square_detail(img, Tile::Floor, seed, c);
            let sea = [102, 212, 196];
            fill_rect(img, 10.0, 52.0, 46.0, 6.0, INK);
            fill_rect(img, 10.0, 48.0, 44.0, 5.0, base(Tile::Wall));
            fill_rect(img, 14.0, 45.0, 36.0, 4.0, [130, 136, 124]);
            fill_rect(img, 22.0, 33.0, 20.0, 13.0, c);
            fill_rect(img, 24.0, 35.0, 6.0, 9.0, [144, 140, 112]);
            fill_rect(img, 26.0, 14.0, 12.0, 21.0, shade(sea, 0.65));
            fill_rect(img, 30.0, 6.0, 6.0, 28.0, grim(sea));
        }
        Tile::Up | Tile::Down => {
            paint_square_detail(img, Tile::Floor, seed, c);
            let wall = base(Tile::Wall);
            let step_c = grim(terrain_rgb(t));
            fill_rect(img, 6.0, 8.0, 52.0, 50.0, wall);
            fill_rect(img, 10.0, 10.0, 44.0, 44.0, INK);
            for n in 0..5u64 {
                let inset = if t == Tile::Down { n } else { 4 - n } as f32 * 4.0;
                let y = 12.0 + n as f32 * 9.0;
                fill_rect(
                    img,
                    12.0 + inset,
                    y,
                    40.0 - inset * 2.0,
                    8.0,
                    shade(step_c, 1.0 - n as f32 * 0.12),
                );
            }
        }
    }
}

/// Full square plate for one tile.
pub fn square_tile(t: Tile, seed: u64) -> RgbaImage {
    let mut img = blank(TILE, TILE);
    // Ruins floors under grass per sprites::tile_sprite; props on Floor.
    let ground = match t {
        Tile::Ruins => base(Tile::Grass),
        Tile::Floor | Tile::Wall | Tile::Door | Tile::Chest | Tile::Shrine | Tile::Up | Tile::Down => edge_colour(t),
        _ => base(t),
    };
    fill_rect(&mut img, 0.0, 0.0, TILE as f32, TILE as f32, ground);
    grain_sq(&mut img, seed, ground);
    paint_square_detail(&mut img, t, seed, ground);
    seam_square(&mut img, t);
    img
}

// ---------------------------------------------------------------------------
// Iso plates (projection B): three face rows per tile, shared anchor A.

/// gfxlab::ridge — organic mountain silhouette, scaled to the face cell.
fn ridge(img: &mut RgbaImage, c: Rgb, seed: u64) {
    let cx = AX;
    let cy = 40.0;
    let s = 20.0;
    let wob = |i: u64| ((hash64(seed + i * 17) % 7) as f32 - 3.0) * s * 0.05;
    let peak_y = cy - s * (1.3 + (seed % 3) as f32 * 0.22);
    let l = [cx - s * 0.9 + wob(1), cy + s * 0.35];
    let r = [cx + s * 0.95 + wob(2), cy + s * 0.38];
    let peak = [cx + wob(3), peak_y];
    fill_poly(img, &[peak, l, [cx + wob(4), cy + s * 0.3]], c);
    fill_poly(img, &[peak, [cx + wob(4), cy + s * 0.3], r], shade(c, 0.8));
    fill_poly(
        img,
        &[
            [cx + wob(3) + s * 0.1, peak_y + s * 0.12],
            [cx - s * 0.28, peak_y + s * 0.5],
            [cx + s * 0.3, peak_y + s * 0.46],
        ],
        shade(c, 1.3),
    );
}

/// A stand of trees spread across the whole tile.
///
/// A forest tile repeats hundreds of times on a map. One clump pinned to the
/// centre therefore reads as a single repeated dot — the "dark blob" the old
/// pass produced. Scattering the canopies across the tile footprint keeps each
/// one legible, and the lit upper edge plus the cast shadow stop a tree from
/// reading as a hole punched in the ground.
fn canopy(img: &mut RgbaImage, c: Rgb, seed: u64, trees: u32, deep: bool) {
    let trunk = grim([58, 44, 31]);
    // Understory, so the gaps between trunks read as forest floor and not void.
    for i in 0..trees * 2 {
        let u = h01(seed, 700 + i as u64) * 2.0 - 1.0;
        let v = h01(seed, 760 + i as u64) * 2.0 - 1.0;
        fill_circle(
            img,
            AX + u * HW * 0.72,
            AY + v * HH * 0.72,
            1.4 + h01(seed, 820 + i as u64) * 1.4,
            shade(c, 0.62),
        );
    }
    // Three values, in this order, because a 64px tile only has room for a
    // silhouette plus one highlight: the ground below is darkest, the canopy
    // mass sits clearly above it, and a lit crown catches the light. Painting
    // the canopy darker than the floor is what turned these into black holes.
    for i in 0..trees {
        let u = h01(seed, 300 + i as u64) * 1.4 - 0.7;
        let v = h01(seed, 340 + i as u64) * 0.9 - 0.45;
        let x = AX + u * HW * 0.6;
        let y = AY + v * HH * 0.6;
        let r = 7.0 + h01(seed, 380 + i as u64) * 3.5;
        fill_circle(img, x + r * 0.3, y + r * 0.45, r * 0.85, shade(c, 0.5));
        fill_rect(img, x - r * 0.09, y - r * 0.1, r * 0.18, r * 0.8, trunk);
        fill_circle(img, x, y - r * 0.9, r, shade(c, 0.82));
        fill_circle(img, x - r * 0.08, y - r * 1.02, r * 0.8, shade(c, if deep { 1.2 } else { 1.34 }));
        fill_circle(img, x - r * 0.3, y - r * 1.24, r * 0.36, shade(c, if deep { 1.5 } else { 1.7 }));
    }
}

/// Textured standing joinery over the caller's actual floor material.
/// `along_x` selects the wall run; the open leaf swings off that run, leaving
/// the floor visible through the frame rather than painting a black hole.
pub fn door_plate(floor: &RgbaImage, iso: bool, open: bool, along_x: bool) -> RgbaImage {
    let mut img = floor.clone();
    let slope = if along_x { 0.5 } else { -0.5 };
    let standing = iso && floor.width() == 96;
    // One wall edge spans 32 source pixels, not the full 64px diamond.
    // Keep the midpoint at the standing plate's foot anchor A=(48,64).
    let origin = if standing { [32.0, 64.0 - 16.0 * slope] }
        else if iso { [18.0, 28.0 - 14.0 * slope] } else { [10.0, 56.0] };
    let across = if standing { [32.0 / 28.0, slope * 32.0 / 28.0] }
        else if iso { [1.0, slope] } else { [1.5, 0.0] };
    let up = if standing { [0.0, -1.6] }
        else if iso { [0.0, -1.0] } else { [0.0, -2.0] };
    let point = |u: f32, v: f32| [
        origin[0] + across[0] * u + up[0] * v,
        origin[1] + across[1] * u + up[1] * v,
    ];
    if standing {
        // The lintel has the wall's full tile depth. Its top continues the
        // neighboring caps, rather than leaving a notch behind a thin frame.
        let cx = if along_x { 64.0 } else { 32.0 };
        let cap = [[cx, 0.0], [cx + 32.0, 16.0], [cx, 32.0], [cx - 32.0, 16.0]];
        for (a, b, color) in [(cap[3], cap[2], [78, 74, 62]), (cap[2], cap[1], [61, 58, 49])] {
            fill_poly(&mut img, &[a, b, [b[0], b[1] + 6.4], [a[0], a[1] + 6.4]], color);
        }
        fill_poly(&mut img, &cap, [99, 94, 78]);
    }
    // Threshold and jambs have depth, mortar joints, worn stone edges.
    for (u, v, w, h) in [(0.0, 0.0, 28.0, 2.0), (0.0, 2.0, 4.0, 22.0),
        (24.0, 2.0, 4.0, 22.0), (0.0, 21.0, 28.0, 4.0)] {
        fill_poly(&mut img, &[point(u + 1.5, v), point(u + w + 1.5, v),
            point(u + w + 1.5, v + h), point(u + 1.5, v + h)], [39, 38, 33]);
        fill_poly(&mut img, &[point(u, v), point(u + w, v),
            point(u + w, v + h), point(u, v + h)], [99, 94, 78]);
        for course in (v as i32..(v + h) as i32).step_by(5) {
            fill_poly(&mut img, &[point(u, course as f32), point(u + w, course as f32),
                point(u + w, course as f32 + 0.6), point(u, course as f32 + 0.6)], [51, 49, 42]);
        }
        fill_poly(&mut img, &[point(u, v), point(u + 0.7, v),
            point(u + 0.7, v + h), point(u, v + h)], [133, 124, 98]);
    }
    let hinge = point(4.0, 2.0);
    let leaf_axis = if open {
        if standing {
            // Swing into the passage: +y for an x-run, +x for a y-run.
            if along_x { [-across[0], across[1]] } else { [across[0], -across[1]] }
        } else if iso { [-0.34, 0.43] } else { [-0.35, -0.23] }
    } else { across };
    // Back edge of the thick swung leaf; the front keeps the same timber grain.
    let leaf_point = |u: f32, v: f32| [
        hinge[0] + leaf_axis[0] * u + up[0] * v,
        hinge[1] + leaf_axis[1] * u + up[1] * v,
    ];
    let corners = [leaf_point(0.0, 0.0), leaf_point(20.0, 0.0),
        leaf_point(20.0, 19.0), leaf_point(0.0, 19.0)];
    let back = corners.map(|p| [p[0] + 1.5, p[1] + 0.8]);
    fill_poly(&mut img, &back, [43, 29, 19]);
    let det = leaf_axis[0] * up[1] - leaf_axis[1] * up[0];
    for y in 0..img.height() {
        for x in 0..img.width() {
            let dx = x as f32 + 0.5 - hinge[0];
            let dy = y as f32 + 0.5 - hinge[1];
            let u = (dx * up[1] - dy * up[0]) / det;
            let v = (leaf_axis[0] * dy - leaf_axis[1] * dx) / det;
            if !(0.0..20.0).contains(&u) || !(0.0..19.0).contains(&v) { continue; }
            let plank = (u / 4.0) as u64;
            let grain = (u * 5.0 + (v * 0.55 + plank as f32).sin() * 0.8).sin();
            let knot = ((u - 10.3).powi(2) + ((v - 8.5) * 0.55).powi(2)).sqrt();
            let tone = 0.92 + grain * 0.11 + h01(761, plank) * 0.16
                + if knot < 2.2 { (knot * 8.0).sin() * 0.18 - 0.13 } else { 0.0 };
            let mut c = shade([119, 78, 41], tone);
            if u % 4.0 < 0.45 { c = [48, 34, 23]; }
            // Forged iron straps, hinge plates and a latch on the free edge.
            let strap = (3.0..4.6).contains(&v) || (14.0..15.6).contains(&v);
            if strap { c = if v % 1.0 < 0.3 { [109, 113, 108] } else { [39, 43, 42] }; }
            if strap && (u % 4.0 - 1.6).abs() < 0.48 { c = [148, 143, 117]; }
            if u < 1.3 && ((2.0..6.0).contains(&v) || (13.0..17.0).contains(&v)) { c = [78, 82, 78]; }
            if (16.0..19.0).contains(&u) && (8.0..9.2).contains(&v) { c = [153, 141, 100]; }
            put(&mut img, x as i32, y as i32, c);
        }
    }
    img
}

/// Standing prop box (gfxlab's Door/Chest/Shrine billboard, doubled).
fn prop_box(img: &mut RgbaImage, c: Rgb, seed: u64) {
    let _ = seed;
    let x = AX - 9.0;
    let y = AY + 6.0 - 24.0; // bottom edge sits on the diamond centre
    fill_rect(img, x, y, 18.0, 24.0, shade(c, 1.2));
    for (dx, dy) in [(0, 0), (17, 0), (0, 23), (17, 23)] {
        put(img, x as i32 + dx, y as i32 + dy, shade(c, 0.6));
    }
    blend_rect(img, x, y, 18.0, 2.0, shade(c, 1.35), 160);
}

/// Iso top-face plate. Flat tiles: crisp grim diamond (D25 flat-top read).
/// Blocks: silhouette art rising from the ground-diamond anchor.
fn iso_top(t: Tile, seed: u64) -> RgbaImage {
    let mut img = blank(TILE, TILE);
    let c = if t == Tile::Floor { edge_colour(t) } else { base(t) };
    match t {
        Tile::Forest | Tile::DeepForest => {
            // Forest floor shows between the trunks, so the ground diamond is
            // lifted rather than sunk toward black.
            // The floor under a canopy is the darkest thing on the tile.
            diamond(&mut img, AX, AY, HW, HH, shade(c, 0.46));
            canopy(&mut img, c, seed, if t == Tile::DeepForest { 3 } else { 2 }, t == Tile::DeepForest);
        }
        Tile::Mountain => {
            diamond(&mut img, AX, AY, HW, HH, shade(c, 0.8));
            ridge(&mut img, c, seed);
        }
        Tile::Wall => {
            // A plain shared stone cap; exposed masonry comes from the short
            // side faces, not a tooth or an etched cross on every wall cell.
            diamond(&mut img, AX, AY, HW, HH, edge_colour(Tile::Wall));
        }
        Tile::Door => {
            let floor = edge_colour(Tile::Floor);
            diamond(&mut img, AX, AY, HW, HH, floor);
            grain_diamond(&mut img, seed, floor);
            img = door_plate(&img, true, false, true);
        }
        Tile::Chest | Tile::Shrine => {
            let floor = edge_colour(Tile::Floor);
            diamond(&mut img, AX, AY, HW, HH, floor);
            grain_diamond(&mut img, seed, floor);
            prop_box(&mut img, c, seed);
            if t == Tile::Shrine {
                blend_rect(&mut img, AX - 4.0, AY - 22.0, 8.0, 4.0, [180, 226, 214], 140);
            }
        }
        Tile::Up | Tile::Down => {
            let floor = edge_colour(Tile::Floor);
            diamond(&mut img, AX, AY, HW, HH, floor);
            let step_c = grim(terrain_rgb(t));
            for n in 0..3u64 {
                let (k, scale) = if t == Tile::Down {
                    (1.0 - n as f32 * 0.14, 0.3 + n as f32 * 0.24)
                } else {
                    (0.72 + n as f32 * 0.14, 0.78 - n as f32 * 0.24)
                };
                diamond(
                    &mut img,
                    AX,
                    AY,
                    HW * scale,
                    HH * scale,
                    shade(step_c, k),
                );
            }
        }
        _ => {
            diamond(&mut img, AX, AY, HW, HH, c);
            if !matches!(t, Tile::Grass | Tile::Road) {
                grain_diamond(&mut img, seed, c);
            }
            match t {
                Tile::River | Tile::Ford => {
                    // Mock's inner channel diamond (hw*0.62). The lift is small
                    // on purpose: a pale base plus a strong inner diamond pushed
                    // the ford to a blown-out white band in play.
                    diamond(&mut img, AX, AY, HW * 0.62, HH * 0.62, shade(c, if t == Tile::Ford { 1.12 } else { 1.3 }));
                    if t == Tile::Ford {
                        // A ford is a crossing, so the bed is the subject: broad
                        // wet stones with a lit crown, stepping across the flow.
                        let stone = grim([138, 133, 121]);
                        for i in 0..5u64 {
                            let dx = -16.0 + i as f32 * 8.0 + h01(seed, 520 + i) * 5.0;
                            let dy = (h01(seed, 524 + i) - 0.5) * HH * 0.7;
                            let r = 3.2 + h01(seed, 528 + i) * 2.2;
                            fill_circle(&mut img, AX + dx, AY + dy + 1.2, r, shade(stone, 0.68));
                            fill_circle(&mut img, AX + dx, AY + dy, r * 0.86, stone);
                            fill_circle(&mut img, AX + dx - r * 0.24, AY + dy - r * 0.3, r * 0.4, shade(stone, 1.22));
                        }
                        // Wake lines where the current breaks over the stones.
                        for i in 0..3u64 {
                            let dx = -14.0 + i as f32 * 12.0;
                            blend_rect(&mut img, AX + dx, AY - 1.0, 7.0, 1.0, shade(c, 1.4), 90);
                        }
                    }
                }
                Tile::Grass => {
                    // Broad tonal patches first: a flat diamond reads as a
                    // repeating stamp however many specks sit on top of it.
                    for i in 0..5u64 {
                        let px = AX - HW * 0.8 + h01(seed, 525 + i) * HW * 1.6;
                        let py = AY - HH * 0.7 + h01(seed, 535 + i) * HH * 1.4;
                        let k = if i % 2 == 0 { 0.82 } else { 1.16 };
                        blend_circle(&mut img, px, py, 5.0 + h01(seed, 545 + i) * 7.0, shade(c, k), 110);
                    }
                    // Tufts, then a scatter of bare earth so the sward has grit.
                    for i in 0..3u64 {
                        let tx = AX - HW * 0.78 + h01(seed, 530 + i) * HW * 1.56;
                        let ty = AY - HH * 0.6 + h01(seed, 540 + i) * HH * 1.2;
                        let up = shade(c, 1.12);
                        blend_rect(&mut img, tx, ty - 2.0, 1.0, 2.0, up, 150);
                        blend_rect(&mut img, tx + 1.6, ty - 1.0, 1.0, 1.0, up, 100);
                    }
                    for i in 0..2u64 {
                        let tx = AX - HW * 0.7 + h01(seed, 560 + i) * HW * 1.4;
                        let ty = AY - HH * 0.5 + h01(seed, 565 + i) * HH;
                        blend_circle(&mut img, tx, ty, 1.3 + h01(seed, 570 + i), shade(c, 0.78), 105);
                    }
                }
                Tile::Ruins => {
                    let stone = c;
                    for i in 0..4u64 {
                        let dx = -14.0 + h01(seed, 550 + i) * 28.0;
                        let dy = -6.0 + h01(seed, 560 + i) * 12.0;
                        fill_rect(&mut img, AX + dx, AY + dy, 4.0, 3.0, shade(stone, 1.1));
                    }
                }
                Tile::Road => {
                    // Broad compacted track and embedded rounded gravel, not
                    // isolated square specks across a sand-coloured plate.
                    blend_circle(&mut img, AX, AY, 12.0, shade(c, 0.90), 90);
                    for i in 0..7u64 {
                        let dx = -18.0 + h01(seed, 570 + i) * 36.0;
                        let dy = -7.0 + h01(seed, 580 + i) * 14.0;
                        let r = 0.6 + h01(seed, 590 + i) * 0.8;
                        blend_circle(&mut img, AX + dx, AY + dy, r, shade(c, 0.81), 150);
                        blend_circle(&mut img, AX + dx - 0.3, AY + dy - 0.3, r * 0.5, shade(c, 1.1), 90);
                    }
                }
                Tile::Rock => {
                    for i in 0..2u64 {
                        let dx = -10.0 + h01(seed, 590 + i) * 18.0;
                        let dy = -5.0 + h01(seed, 600 + i) * 10.0;
                        fill_circle(&mut img, AX + dx, AY + dy, 4.5, shade(c, 1.15));
                        fill_circle(&mut img, AX + dx + 1.0, AY + dy + 2.0, 3.5, shade(c, 0.75));
                    }
                }
                _ => {}
            }
        }
    }
    seam_iso(&mut img, t);
    img
}

/// Iso side face (0 = left, 1 = right). Blocks only; flat tiles are
/// ground diamonds with no depth, so their side cells stay empty.
fn iso_side(t: Tile, left: bool, seed: u64) -> RgbaImage {
    let mut img = blank(TILE, TILE);
    if !is_block(t) {
        return img;
    }
    let c = base(t);
    let h = block_height(t);
    let (k, pts): (f32, [[f32; 2]; 4]) = if left {
        (
            0.62,
            [
                [AX - HW, AY],
                [AX, AY + HH],
                [AX, AY + HH + h],
                [AX - HW, AY + h],
            ],
        )
    } else {
        (
            0.82,
            [
                [AX, AY + HH],
                [AX + HW, AY],
                [AX + HW, AY + h],
                [AX, AY + HH + h],
            ],
        )
    };
    let face_c = match t {
        // Forest skirts read as shadowed undergrowth, darker than the crown.
        Tile::Forest | Tile::DeepForest => shade(c, 0.55),
        _ => c,
    };
    quad(&mut img, pts, shade(face_c, k));
    match t {
        Tile::Forest | Tile::DeepForest => {
            // Leaf lobes straddling the top edge blend crown into skirt.
            for i in 0..3u64 {
                let u = 0.15 + h01(seed, 620 + i + (left as u64) * 8) * 0.7;
                let (ex, ey) = if left {
                    (AX - HW + u * HW, AY + u * HH)
                } else {
                    (AX + u * HW, AY + HH - u * HH)
                };
                fill_circle(&mut img, ex, ey, 5.0, face_c);
                fill_circle(&mut img, ex - 1.0, ey - 1.5, 4.0, c);
            }
        }
        Tile::Mountain => {
            // Strata lines + scree at the foot.
            for d in 0..2u64 {
                let off = h * (0.35 + d as f32 * 0.3);
                let mut u = 0.0;
                while u < 0.9 {
                    let seg = 0.12 + h01(seed, 640 + d * 8 + (left as u64) + (u * 50.0) as u64) * 0.1;
                    let (x0, y0) = if left {
                        (AX - HW + u * HW, AY + u * HH + off)
                    } else {
                        (AX + u * HW, AY + HH - u * HH + off)
                    };
                    let (x1, y1) = if left {
                        (AX - HW + (u + seg) * HW, AY + (u + seg) * HH + off)
                    } else {
                        (AX + (u + seg) * HW, AY + HH - (u + seg) * HH + off)
                    };
                    // Dashed line approximation: dotted pixels along the segment.
                    let steps = 6;
                    for spt in 0..steps {
                        let tt = spt as f32 / steps as f32;
                        blend_px(
                            &mut img,
                            (x0 + (x1 - x0) * tt) as i32,
                            (y0 + (y1 - y0) * tt) as i32,
                            shade(c, 0.5),
                            170,
                        );
                    }
                    u += seg + 0.08;
                }
            }
            for i in 0..4u64 {
                let u = 0.1 + h01(seed, 660 + i + (left as u64) * 8) * 0.8;
                let (ex, ey) = if left {
                    (AX - HW + u * HW, AY + u * HH + h)
                } else {
                    (AX + u * HW, AY + HH - u * HH + h)
                };
                fill_rect(&mut img, ex - 1.0, ey - 3.0, 3.0, 2.0, shade(c, 0.55));
            }
        }
        Tile::Wall => {
            // Coursed masonry with staggered ties.
            for d in 0..2u64 {
                let off = 7.0 + d as f32 * 7.0;
                let mut u = 0.0;
                while u < 0.92 {
                    let seg = 0.14 + h01(seed, 680 + d * 8 + (left as u64) + (u * 40.0) as u64) * 0.1;
                    let (x0, y0) = if left {
                        (AX - HW + u * HW, AY + u * HH + off)
                    } else {
                        (AX + u * HW, AY + HH - u * HH + off)
                    };
                    let (x1, y1) = if left {
                        (AX - HW + (u + seg) * HW, AY + (u + seg) * HH + off)
                    } else {
                        (AX + (u + seg) * HW, AY + HH - (u + seg) * HH + off)
                    };
                    let steps = 8;
                    for spt in 0..steps {
                        let tt = spt as f32 / steps as f32;
                        blend_px(
                            &mut img,
                            (x0 + (x1 - x0) * tt) as i32,
                            (y0 + (y1 - y0) * tt) as i32,
                            shade(c, 0.55),
                            190,
                        );
                    }
                    u += seg + 0.06;
                }
            }
            for i in 0..3u64 {
                let u = 0.12 + h01(seed, 700 + i + (left as u64) * 8) * 0.75;
                let (ex, ey) = if left {
                    (AX - HW + u * HW, AY + u * HH)
                } else {
                    (AX + u * HW, AY + HH - u * HH)
                };
                blend_rect(&mut img, ex, ey + 7.0, 1.5, 7.0, shade(c, 0.6), 150);
            }
        }
        _ => {}
    }
    img
}

/// The three iso face plates for one tile: top, left, right.
pub fn iso_faces(t: Tile, seed: u64) -> [RgbaImage; 3] {
    [
        iso_top(t, seed),
        iso_side(t, true, seed),
        iso_side(t, false, seed),
    ]
}
