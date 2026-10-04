//! 3D bake path (D38): procedural heightfield scenes rendered with real
//! lighting — sun NW key + warm fill + baked AO + marched self-shadows,
//! splatted orthographic at 3x and box-downsampled to 64px.
//!
//! Batch 1: Mountain (ridged-fbm conical crag) and DeepForest (metaball
//! canopy mound). Albedos come from the same grim palette as the painterly
//! plates (paint_terrain lineage), so baked plates drop into the same slots
//! with manifest keys unchanged.

use crate::paint;
use crate::raster::*;
use image::RgbaImage;
use laya_realms::model::Tile;

/// Supersample factor: 64px cell rendered at 192, box-downsampled back.
const SS: i32 = 3;
const S: i32 = 64 * SS;

/// Masonry face height in SHEET px, measured up from the ground diamond's
/// lower edge. The face's outer vertex lands at `AY - WALL_FACE_H`, so a face
/// taller than AY = 20 used to be clipped at the cell top and left a notch at
/// every cell boundary in a run. [`WALL_FACE_DROP`] buys the room instead:
/// the art is painted that much lower inside the cell and the view lifts the
/// face sprite back, raising the usable ceiling to `AY + WALL_FACE_DROP`.
pub const WALL_FACE_H: f32 = 40.0;

/// How far below its true position the face art sits inside its cell, in SHEET
/// px. Must equal `Proj::ISO_WALL_FACE_DROP_PX`.
pub const WALL_FACE_DROP: f32 = 24.0;

/// Ground-diamond geometry in supersampled cell coordinates: the shared
/// A=(32,20) anchor is (96,60), half-width 96, half-height 48.
const AX: f32 = 96.0;
const AY: f32 = 60.0;
const HW: f32 = 96.0;
const HH: f32 = 48.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BakeTile {
    Mountain,
    DeepForest,
    Forest,
    Rock,
    Wall,
    /// Flowing channel: a sunken bed with a wave surface and bank foam.
    River,
    /// Crossing: the same channel with a raised bed, so the stones read
    /// through the water and the player can wade.
    Ford,
    /// Collapsed masonry: broken courses, a fallen lintel and rubble drifts.
    Ruins,
    /// Cut flagstones with settled joints and a worn track.
    Floor,
}

fn map_tile(tile: BakeTile) -> Tile {
    match tile {
        BakeTile::Mountain => Tile::Mountain,
        BakeTile::DeepForest => Tile::DeepForest,
        BakeTile::Forest => Tile::Forest,
        BakeTile::Rock => Tile::Rock,
        BakeTile::Wall => Tile::Wall,
        BakeTile::River => Tile::River,
        BakeTile::Ford => Tile::Ford,
        BakeTile::Ruins => Tile::Ruins,
        BakeTile::Floor => Tile::Floor,
    }
}

/// Block-defined prop geometries baked through the same lighting rig
/// (batch 2 stretch): the props sheet keeps its keys, only pixels change.
#[derive(Clone, Copy, PartialEq)]
pub enum BakeProp {
    RelicPedestal,
}

// ---------------------------------------------------------------------------
// Deterministic noise (value noise + fbm + ridged fbm).

fn jitter(ix: i32, iy: i32, seed: u64) -> f32 {
    let v = hash64((ix as u64) << 32 ^ (iy as u64) ^ seed);
    (v >> 40) as f32 / ((1u64 << 24) as f32)
}

fn smooth(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

fn noise2(x: f32, y: f32, seed: u64) -> f32 {
    let (ix, iy) = (x.floor() as i32, y.floor() as i32);
    let (fx, fy) = (smooth(x.fract()), smooth(y.fract()));
    let a = jitter(ix, iy, seed);
    let b = jitter(ix + 1, iy, seed);
    let c = jitter(ix, iy + 1, seed);
    let d = jitter(ix + 1, iy + 1, seed);
    a + (b - a) * fx + (c - a) * fy + (a - b - c + d) * fx * fy
}

fn fbm(x: f32, y: f32, seed: u64, oct: u32) -> f32 {
    let (mut v, mut amp, mut f, mut norm) = (0.0, 0.5, 1.0, 0.0);
    for i in 0..oct {
        v += amp * noise2(x * f, y * f, seed ^ (i as u64 * 7919));
        norm += amp;
        amp *= 0.5;
        f *= 2.1;
    }
    v / norm
}

fn ridged(x: f32, y: f32, seed: u64) -> f32 {
    1.0 - (fbm(x, y, seed, 4) * 2.0 - 1.0).abs()
}

// ---------------------------------------------------------------------------
// Heightfields (world domain u,v in 0..64, h in world px above ground).


fn height(tile: BakeTile, u: f32, v: f32, seed: u64) -> f32 {
    match tile {
        BakeTile::Mountain => {
            let cx = 30.0 + h01(seed, 1) * 5.0;
            let cy = 29.0 + h01(seed, 2) * 3.0;
            let dx = (u - cx) / 26.0;
            let dy = (v - cy) / 23.0;
            let d = (dx * dx + dy * dy).sqrt();
            let ground = 2.0 + fbm(u * 0.16, v * 0.16, seed ^ 7, 3) * 1.8;
            if d >= 1.05 {
                ground
            } else {
                // Review v2: sharper peak, real ridge amplitude (v1's detail
                // was capped ~2px and downsampled into a smooth egg).
                let cone = (1.0 - d.powf(1.3)).powf(0.9) * 34.0;
                let r = ridged(u * 0.085, v * 0.085, seed);
                let detail = (r - 0.45) * 16.0 * (cone * 0.12).min(1.2);
                // Talus apron: slight lift in a ring near the base.
                let talus = ((1.0 - d).max(0.0) * 4.0).min(2.5);
                // A scree boulder shoulder for asymmetry.
                let bx = cx + 12.0 - h01(seed, 5) * 6.0;
                let by = cy + 10.0 - h01(seed, 6) * 5.0;
                let bd = (((u - bx) / 7.0).powi(2) + ((v - by) / 6.0).powi(2)).sqrt();
                let boulder = (1.0 - bd).max(0.0) * 8.0;
                let mnt = (cone + detail).max(talus).max(boulder);
                ground * 0.35 + mnt
            }
        }
        BakeTile::DeepForest | BakeTile::Forest => {
            // The tile is forest FLOOR, not a canopy: the trees are overhanging
            // sprites (see `bake_tree`). A 64px iso cell physically cannot
            // hold one — its ground diamond already covers rows 4..36, and a
            // crown raised inside that silhouette is hidden by it. What stays
            // here is the lit ground they stand on: needle litter, low mounds
            // and a few stumps for relief.
            let (mounds, ground_n) = if tile == BakeTile::DeepForest {
                (5u64, 2.0)
            } else {
                (4u64, 1.6)
            };
            let ground = ground_n + fbm(u * 0.2, v * 0.2, seed ^ 13, 3) * 1.2;
            let mut top = ground;
            for i in 0..mounds {
                let bx = 12.0 + h01(seed, 20 + i as u64) * 40.0;
                let by = 12.0 + h01(seed, 30 + i as u64) * 40.0;
                let rr = 3.0 + h01(seed, 40 + i as u64) * 3.0;
                let d = (((u - bx).powi(2) + (v - by).powi(2)).sqrt() / rr).min(1.5);
                if d < 1.0 {
                    top = top.max(ground + 3.4 * (1.0 - d * d));
                }
            }
            top
        }
        BakeTile::Rock => {
            // Faceted boulder outcrop: truncated pyramids with crisp edges,
            // plus a low slab ground. Angles do the reading, not texture.
            let ground = 1.5 + fbm(u * 0.14, v * 0.14, seed ^ 17, 3) * 1.4;
            let mut top = ground;
            // Review v3: three BIG outcrops own the read (v2's five small
            // boulders dissolved into dark grit at 64px), plus two satellites.
            for i in 0..5u64 {
                let big = i < 3;
                let cx = if big { 16.0 + h01(seed, 60 + i) * 32.0 } else { 22.0 + h01(seed, 60 + i) * 20.0 };
                let cy = if big { 16.0 + h01(seed, 70 + i) * 32.0 } else { 22.0 + h01(seed, 70 + i) * 20.0 };
                let rx = (if big { 12.0 } else { 6.0 }) + h01(seed, 80 + i) * 5.0;
                let ry = (if big { 10.0 } else { 5.0 }) + h01(seed, 90 + i) * 4.0;
                let bh = (if big { 14.0 } else { 8.0 }) + h01(seed, 100 + i) * 7.0;
                let m = ((u - cx).abs() / rx).max((v - cy).abs() / ry);
                // Linear-taper prism capped at 70% -> truncated pyramid.
                let b = (bh * (1.0 - m * 1.35)).min(bh * 0.72);
                top = top.max(b);
            }
            // Grit over everything.
            top + fbm(u * 0.5, v * 0.5, seed ^ 19, 2) * 1.2
        }
        BakeTile::Ruins => {
            // A room that came apart: surviving wall stubs of different
            // heights, a fallen lintel lying across one corner, and rubble
            // drifts that soften the edges. The height does the storytelling;
            // the albedo only dresses it.
            let ground = 1.2 + fbm(u * 0.18, v * 0.18, seed ^ 61, 3) * 1.3;
            let mut top = ground;
            // Surviving wall stubs on the north and west edges.
            for (run, along) in [(7.0f32, 0.0f32), (0.0f32, 7.0f32)] {
                if ((u - run) * (v - along)).abs() < 26.0 {
                    let course = ((u * 0.16).max(v * 0.16) * 1.4).sin();
                    let h = (7.0 + course * 3.2) * ((1.0 - fbm(u * 0.3, v * 0.3, seed ^ 67, 2)) * 0.5 + 0.5);
                    top = top.max(h.max(ground));
                }
            }
            // The fallen lintel: a long low slab bridging two stubs.
            let lx = (u - 30.0) / 22.0;
            let ly = (v - 16.0) / 5.0;
            if lx * lx + ly * ly < 1.0 {
                top = top.max(5.0 + (1.0 - (lx * lx + ly * ly).sqrt()) * 3.0);
            }
            // Rubble drifts: low, wide, deterministic.
            for i in 0..7u64 {
                let bx = 12.0 + h01(seed, 80 + i) * 40.0;
                let by = 12.0 + h01(seed, 90 + i) * 40.0;
                let d = (((u - bx) / 8.0).powi(2) + ((v - by) / 7.0).powi(2)).sqrt();
                if d < 1.0 {
                    top = top.max(ground + (1.0 - d).max(0.0) * 5.5);
                }
            }
            top
        }
        BakeTile::Floor => {
            // Dead flat, and that is the point. A residual fbm here tilts the
            // normals a fraction of a degree, the shared rig shades that tilt,
            // and neighbouring cells disagree at their shared edge — which is
            // the faint diamond lattice still visible across the ground. No
            // height variation means no normal variation means no seams.
            0.4
        }
        BakeTile::River | BakeTile::Ford => {
            // A gently moving *surface*, not a high-relief bank roof. The
            // coastline belongs to adjacent land cells, not a white cap
            // repeated down the centre of every water tile.
            let flow = (u * 0.14 + v * 0.035).sin() * 0.12;
            let ripple = fbm(u * 0.085, v * 0.20, seed ^ 47, 2) * 0.13;
            if tile == BakeTile::Ford {
                0.65 + flow + ripple + fbm(u * 0.19, v * 0.13, seed ^ 41, 2) * 0.32
            } else {
                0.55 + flow + ripple
            }
        }
        BakeTile::Wall => {
            // Flat on purpose, and the cap is LIFTED by the view instead (see
            // `Proj::ISO_WALL_LIFT`). Any height written here raises the cap
            // inside its own cell, which tears it away from the top edge of
            // the baked masonry faces and leaves a transparent band between
            // them. Vertical normals also keep every cap in a run identical,
            // so the roof union has nothing to seam on.
            0.0
        }
    }
}

/// Central-difference normal of any heightfield (world px gradient).
fn normal_of(hf: &dyn Fn(f32, f32) -> f32, u: f32, v: f32) -> (f32, f32, f32) {
    let e = 0.6;
    let hx = (hf(u + e, v) - hf(u - e, v)) / (2.0 * e);
    let hy = (hf(u, v + e) - hf(u, v - e)) / (2.0 * e);
    let inv = 1.0 / (hx * hx + hy * hy + 1.0).sqrt();
    (-hx * inv, -hy * inv, inv)
}

fn normal(tile: BakeTile, u: f32, v: f32, seed: u64) -> (f32, f32, f32) {
    normal_of(&|a, b| height(tile, a, b, seed), u, v)
}

// ---------------------------------------------------------------------------
// Lighting rig: grim-baked, sun NW high, warm fill, sky bounce.

const SUN: (f32, f32, f32) = (-0.52, -0.48, 0.74); // from NW above
const FILL: (f32, f32, f32) = (0.55, 0.55, 0.30); // weak warm SE bounce

/// Marched self-shadow: walk toward the sun over the heightfield.
fn shadowed_of(hf: &dyn Fn(f32, f32) -> f32, u: f32, v: f32, h: f32) -> bool {
    let (sx, sy, sz) = SUN;
    let mut k = 1u32;
    while k <= 14 {
        let t = k as f32 * 2.2;
        if hf(u + sx * t, v + sy * t) > h + sz * t + 0.6 {
            return true;
        }
        k += 1;
    }
    false
}

/// Cheap directional AO from local height discontinuity.
fn ao_of(hf: &dyn Fn(f32, f32) -> f32, u: f32, v: f32, h: f32, seed_off: u64) -> f32 {
    let mut occ = 0.0;
    for i in 0..6u64 {
        let ang = i as f32 * std::f32::consts::FRAC_PI_3 + h01(seed_off, 90) * std::f32::consts::TAU;
        let (dx, dy) = (ang.cos() * 2.6, ang.sin() * 2.6);
        let hn = hf(u + dx, v + dy);
        occ += (hn - h - 1.6).max(0.0) * 0.6;
    }
    (1.0 / (1.0 + 0.45 * occ)).clamp(0.45, 1.0)
}

/// Material albedo (grim palette snapped) with mottle + strata.
/// v3 adds a MACRO blotch field: low-frequency organic variation keyed to
/// one fixed phase per tile family (never the bake seed), so like-tiles
/// share their large blotches at shared borders — the grid's readable
/// repetition, not the pattern. Yes, the same pattern repeats per family;
/// that's the honest plate-only stand-in until per-coordinate variants
/// land as a future batch (documented limit).
fn albedo(tile: BakeTile, u: f32, v: f32, h: f32, n: (f32, f32, f32), seed: u64) -> Rgb {
    let (off_u, off_v) = match tile {
        BakeTile::Mountain => (0.0, 0.0),
        BakeTile::DeepForest => (40.0, 10.0),
        BakeTile::Forest => (80.0, 50.0),
        BakeTile::Rock => (120.0, 90.0),
        BakeTile::Wall => (160.0, 30.0),
        BakeTile::River => (200.0, 70.0),
        BakeTile::Ford => (240.0, 100.0),
        BakeTile::Ruins => (280.0, 130.0),
        BakeTile::Floor => (320.0, 160.0),
    };
    // v4: macro phase follows the BAKE SEED (per variant), not one global
    // constant — Mountain.v2 / DeepForest.v2 / Forest.v2 / Rock.v2 batches
    // are bake-seed permutations of the same silhouette family; the
    // blotch field must permute with them or variants share mottle.
    let macro_k = 0.90
        + fbm((u + off_u) * 0.055, (v + off_v) * 0.055, seed ^ 0x6c6f7455, 3) * 0.20;
    let mottle = 0.86 + fbm(u * 0.35, v * 0.35, seed ^ 31, 3) * 0.26;
    let mottle = mottle * macro_k;
    match tile {
        BakeTile::Mountain => {
            let rock = grim([113, 114, 105]);
            // Stony ground vs crag mass; strata banding on steep faces.
            let ground_k = if h < 4.5 { 0.80 } else { 1.15 };
            let slope = (n.2 * 255.0).min(255.0) / 255.0;
            let mut k = ground_k * mottle;
            if slope < 0.72 {
                let band = ((h * 0.55 + fbm(u * 0.3, v * 0.3, seed ^ 47, 2) * 3.0) * 0.9).sin();
                k *= 0.92 + band * 0.08;
                // Glint on the NW-facing facets.
                k *= if n.1 < 0.0 { 1.10 } else { 0.97 };
            }
            shade(rock, k)
        }
        BakeTile::DeepForest | BakeTile::Forest => {
            // Value order is the whole read: needle litter dark, canopy mid,
            // lit tips brightest. The previous pass had it inverted — a bright
            // green floor under near-black crowns — so a forest tile read as a
            // green lozenge punched full of holes.
            let leaf = if tile == BakeTile::DeepForest {
                grim([74, 112, 56])
            } else {
                grim([96, 138, 66])
            };
            let litter = if tile == BakeTile::DeepForest {
                grim([32, 43, 29])
            } else {
                grim([43, 54, 34])
            };
            if h < 3.0 {
                // Bare needle floor between the trunks.
                shade(litter, 0.88 + (mottle - 0.86) * 1.4)
            } else {
                // Canopy tips catch light; the skirt under a crown stays dark
                // so the crown has a shaded side instead of a flat blob.
                let tip = ((h - 3.0) / 9.0).clamp(0.0, 1.0);
                let base_k = if tile == BakeTile::DeepForest { 0.80 } else { 0.88 };
                let k = base_k + tip * 0.72 + (mottle - 0.86);
                shade(leaf, k.clamp(0.78, 1.45))
            }
        }
        BakeTile::River | BakeTile::Ford => {
            let deep = grim([31, 74, 94]);
            let shallow = grim([42, 87, 100]);
            let edge = u.min(v).min(64.0 - u).min(64.0 - v);
            let interior = smooth((edge / 9.0).clamp(0.0, 1.0));
            let bank = (fbm(u * 0.038, v * 0.038, seed ^ 59, 3) - 0.40).max(0.0) * interior * 0.7;
            // A low-contrast bank tint, never a white lip on every plate.
            // Ripples run along u, with separate deterministic phases for
            // neighbouring river tiles but the same dark teal boundary.
            let flow = fbm(u * 0.055, v * 0.24, seed ^ 53, 2) - 0.5;
            let glint = (1.0 - n.2).clamp(0.0, 1.0) * 4.0;
            let k = 0.98 + flow * 0.14 * interior;
            let mut c = [
                deep[0] as f32 * (1.0 - bank) + shallow[0] as f32 * bank,
                deep[1] as f32 * (1.0 - bank) + shallow[1] as f32 * bank,
                deep[2] as f32 * (1.0 - bank) + shallow[2] as f32 * bank,
            ];
            for component in &mut c {
                *component = *component * k + glint;
            }
            // Short curved crests carry the surface read at game scale. Each
            // seed scatters broken strokes, never a band across the whole tile.
            let mut crest = 0.0f32;
            let mut trough = 0.0f32;
            for i in 0..7u64 {
                let cx = 10.0 + h01(seed, 310 + i) * 44.0;
                let cy = 10.0 + h01(seed, 330 + i) * 44.0;
                let length = 6.0 + h01(seed, 350 + i) * 8.0;
                let dx = (u - cx) / length;
                if dx.abs() >= 1.0 { continue; }
                let bend = dx * dx * 1.5 + dx * 0.8;
                let dy = v - cy - bend;
                let along = (1.0 - dx * dx).powi(2);
                crest = crest.max(along * (1.0 - (dy / 1.1).abs()).max(0.0));
                trough = trough.max(along * (1.0 - ((dy - 1.5) / 1.3).abs()).max(0.0));
            }
            for (component, lift) in c.iter_mut().zip([14.0, 20.0, 22.0]) {
                *component += interior * (crest * lift - trough * 3.5);
            }
            if tile == BakeTile::Ford {
                // Submerged gravel distinguishes a walkable ford from the
                // river, without a raised pale slab or a per-cell foam crown.
                let bed = fbm(u * 0.18, v * 0.18, seed ^ 41, 3);
                let pebbles = ((bed - 0.61) * 1.5).clamp(0.0, 0.24);
                let gravel = grim([107, 108, 94]);
                for (component, stone) in c.iter_mut().zip(gravel) {
                    *component = *component * (1.0 - pebbles) + stone as f32 * pebbles;
                }
            }
            [c[0].clamp(0.0, 255.0) as u8, c[1].clamp(0.0, 255.0) as u8, c[2].clamp(0.0, 255.0) as u8]
        }
        BakeTile::Ruins => {
            let stone = grim([96, 94, 86]);
            let mortar = grim([64, 62, 57]);
            // Dressed faces catch light; joints and rubble read as mortar-dark.
            let k = if h < 3.0 {
                0.78 * mottle
            } else {
                let course = ((h * 0.7) + fbm(u * 0.3, v * 0.3, seed ^ 73, 2) * 2.0).sin();
                (0.98 + course * 0.10) * mottle
            };
            let edge = (1.0 - (n.2 * 1.1).min(1.0)).max(0.0);
            let c = shade(if h < 2.4 { mortar } else { stone }, k * (0.9 + edge * 0.16));
            // Moss in the sheltered rubble.
            let moss = (fbm(u * 0.16, v * 0.16, seed ^ 79, 3) - 0.56).max(0.0) * 2.6;
            if h < 4.5 && moss > 0.0 {
                let moss_c = grim([58, 84, 52]);
                [
                    (c[0] as f32 * (1.0 - moss) + moss_c[0] as f32 * moss).min(255.0) as u8,
                    (c[1] as f32 * (1.0 - moss) + moss_c[1] as f32 * moss).min(255.0) as u8,
                    (c[2] as f32 * (1.0 - moss) + moss_c[2] as f32 * moss).min(255.0) as u8,
                ]
            } else {
                c
            }
        }
        BakeTile::Floor => {
            // Warm worn stone, not per-slab elevation gates or black joints.
            // The city has large Floor fields; they must read as one plane, and
            // a value clearly ABOVE the wall cap — the approved study has a lit
            // ground, not a black plane with light walls on it.
            // Uniform on purpose. Any per-cell noise (fbm or a baked mottle)
            // makes neighbouring cells disagree at their shared edge, and that
            // disagreement is the faint grid that was still visible across the
            // ground. Every Floor cell is now the same value, so the joins are
            // mathematically seamless.
            let stone = grim([104, 99, 90]);
            shade(stone, 1.0)
        }
        BakeTile::Rock => {
            let slate = grim([50, 55, 55]);
            // Wet-slate ground vs lit boulder faces; west facets glint.
            let ground_k = if h < 3.0 { 0.80 } else { 1.20 };
            let mut k = ground_k * mottle;
            let slope_n = n.2;
            if slope_n < 0.8 {
                k *= if n.0 < 0.0 { 1.12 } else { 0.9 };
            }
            shade(slate, k.clamp(0.5, 1.35))
        }
        BakeTile::Wall => {
            // The cap reads ABOVE the floor: the reference draws a wall as a
            // lit block, and a cap darker than the ground is what made every
            // wall in the game look like a recessed ribbon instead of
            // masonry. Bright enough to separate from the floor, low enough
            // to stay inside `palette::lit`'s headroom — an albedo past ~116
            // clips to white through EXPOSURE 2.2.
            let cap = grim([136, 126, 104]);
            // `mottle` is a function of (u,v) and the bake seed alone, never
            // of the cell, so it is IDENTICAL in every wall cell: a run unions
            // into one roof with nothing to seam on. A per-cell value would
            // print the tile grid onto the wall tops.
            shade(cap, (1.0 + (mottle - 1.0) * 0.10).clamp(0.97, 1.03))
        }
    }
}

/// Full shade: ao * (ambient + sun key (shadowed) + warm fill + sky bounce),
/// clamped so the plate stays grim (max ~1.32x albedo).
fn shade_of(
    tile: BakeTile,
    hf: &dyn Fn(f32, f32) -> f32,
    albedo: Rgb,
    n: (f32, f32, f32),
    u: f32,
    v: f32,
    h: f32,
    seed: u64,
) -> Rgb {
    let ao = ao_of(hf, u, v, h, seed);
    let key = (n.0 * SUN.0 + n.1 * SUN.1 + n.2 * SUN.2).max(0.0);
    let vis = if key > 0.02 && shadowed_of(hf, u, v, h) {
        0.34
    } else {
        1.0
    };
    let fill = (n.0 * FILL.0 + n.1 * FILL.1 + n.2 * FILL.2).max(0.0);
    // Review v2: raised ambient/key floors (v1 underexposed on the grim bg).
    let mut lum = (0.38 * ao + 0.85 * key * vis * (0.55 + 0.45 * ao) + 0.18 * fill + 0.12 * ao * n.2)
        .min(1.32);
    // Foliage is not a lit rock face. A crown is mostly steep skirt, and the
    // shared rig drives those pixels to ~0.2x albedo, which is how a green
    // forest baked out as a black mass. Compress the range for the canopy
    // families so the shaded side stays a readable dark green.
    if matches!(tile, BakeTile::Forest | BakeTile::DeepForest) {
        lum = 0.62 + lum * 0.52;
    }
    shade(albedo, lum.min(1.32))
}

fn shade_sample(tile: BakeTile, u: f32, v: f32, seed: u64) -> (Rgb, (f32, f32, f32), f32) {
    let h = height(tile, u, v, seed);
    let n = normal(tile, u, v, seed);
    let al = albedo(tile, u, v, h, n, seed);
    let hf = |a: f32, b: f32| height(tile, a, b, seed);
    (shade_of(tile, &hf, al, n, u, v, h, seed), n, h)
}

// ---------------------------------------------------------------------------
// Splat renderer.

struct Splat {
    depth: Vec<f32>,
    col: Vec<(f32, f32, f32)>,
    alpha: Vec<bool>,
    norm: Vec<(f32, f32, f32)>,
    w: i32,
    h: i32,
}

impl Splat {
    fn new(w: i32, h: i32) -> Self {
        let n = (w * h) as usize;
        Self {
            depth: vec![f32::MAX; n],
            col: vec![(0.0, 0.0, 0.0); n],
            alpha: vec![false; n],
            norm: vec![(0.0, 0.0, 0.0); n],
            w,
            h,
        }
    }
    fn put(&mut self, x: i32, y: i32, depth: f32, c: Rgb, n: (f32, f32, f32)) {
        if x < 0 || y < 0 || x >= self.w || y >= self.h {
            return;
        }
        let i = (y * self.w + x) as usize;
        if depth < self.depth[i] {
            self.depth[i] = depth;
            self.col[i] = (c[0] as f32, c[1] as f32, c[2] as f32);
            self.alpha[i] = true;
            self.norm[i] = n;
        }
    }
    fn disc(&mut self, cx: f32, cy: f32, depth: f32, c: Rgb, n: (f32, f32, f32), r: i32) {
        for dy in -r..=r {
            for dx in -r..=r {
                if dx * dx + dy * dy <= r * r {
                    self.put((cx + dx as f32) as i32, (cy + dy as f32) as i32, depth, c, n);
                }
            }
        }
    }
}

/// Square top-down render: camera straight down, full-bleed opaque tile.
pub fn bake_square(tile: BakeTile, seed: u64) -> RgbaImage {
    if matches!(tile, BakeTile::Wall | BakeTile::Floor) {
        return bake_material_square(tile == BakeTile::Wall, 0, seed);
    }
    let mut sp = Splat::new(S, S);
    // Sample the heightfield at ~0.35 world-px spacing (dense coverage).
    let steps = 190;
    for iy in 0..steps {
        for ix in 0..steps {
            let u = (ix as f32 + 0.5) / steps as f32 * 64.0;
            let v = (iy as f32 + 0.5) / steps as f32 * 64.0;
            let (c, n, h) = shade_sample(tile, u, v, seed);
            let sx = u * SS as f32;
            let sy = v * SS as f32;
            sp.disc(sx, sy, -h, c, n, 2);
        }
    }
    let mut out = downsample(&sp);
    clutter_square(tile, seed, &mut out);
    paint::seam_square(&mut out, map_tile(tile));
    out
}

/// The two exposed masonry faces in supersampled cell coordinates. The shared
/// A=(32,20) ground anchor is (96,60) here, half-width 96, half-height 48.
/// Each face is the ground diamond's own lower edge lifted by
/// [`WALL_FACE_H`] — the wall occupies its own cell and never covers the tile
/// to its south.
fn wall_face_polys() -> ([[f32; 2]; 4], [[f32; 2]; 4]) {
    let lift = WALL_FACE_H * SS as f32;
    let drop = WALL_FACE_DROP * SS as f32;
    (
        // South/screen-left: ground edge from the left vertex to the bottom.
        [
            [AX - HW, AY - lift + drop],
            [AX, AY + HH - lift + drop],
            [AX, AY + HH + drop],
            [AX - HW, AY + drop],
        ],
        // East/screen-right: ground edge from the bottom vertex to the right.
        [
            [AX, AY + HH - lift + drop],
            [AX + HW, AY - lift + drop],
            [AX + HW, AY + drop],
            [AX, AY + HH + drop],
        ],
    )
}

/// Material families are atlas states, not runtime recolours. Keep the geometry
/// shared: neither weathering nor joints may change the cap/face anchors.
pub const MATERIAL_FAMILIES: [&str; 8] = [
    "town", "crypt", "sanctum", "underkeep", "cave", "woodland", "arena", "dock",
];

fn material_base(family: usize, cap: bool) -> Rgb {
    let rgb = match family {
        0 => [112, 104, 87],
        1 => [71, 86, 80],
        2 => [83, 77, 94],
        3 => [106, 105, 94],
        4 => [93, 79, 60],
        5 => [68, 78, 53],
        6 => [95, 91, 86],
        _ => [104, 85, 63],
    };
    shade(grim(rgb), if cap { 1.25 } else { 1.0 })
}

fn material_surface(family: usize, u: f32, v: f32, cap: bool, phase: u64) -> Rgb {
    let base = if cap && matches!(family, 4 | 5) {
        grim(if family == 4 { [121, 112, 93] } else { [100, 109, 85] })
    } else {
        material_base(family, cap)
    };
    let edge = u.min(v).min(64.0 - u).min(64.0 - v);
    let detail = smooth((edge / 7.0).clamp(0.0, 1.0));
    let grain = fbm(u * 0.21, v * 0.21, phase ^ 0x71, 3) - 0.5;
    let broad = fbm(u * 0.045, v * 0.045, phase ^ 0x99, 2) - 0.5;
    let (w, h) = if cap && family != 7 {
        // A heavy coping slab, not the room's little floor pavers (or mud).
        (32.0, 32.0)
    } else {
        match family {
            0 | 1 => (16.0, 16.0),
            2 | 6 => (32.0, 32.0),
            3 => (32.0, 16.0),
            7 => (8.0, 64.0),
            _ => (16.0, 16.0),
        }
    };
    let row = (v / h).floor();
    let warp = if family == 1 { (v * std::f32::consts::TAU / 16.0).sin() * 1.4 } else { 0.0 };
    let x = (u + row * w * 0.5 + warp).rem_euclid(w);
    let y = v.rem_euclid(h);
    let joint = x.min(w - x).min(y.min(h - y));
    let stone = jitter(((u + row * w * 0.5) / w).floor() as i32, row as i32, 0x9182);
    let mut k = 1.0 + detail * (grain * 0.18 + broad * 0.15);
    if !cap && (family == 4 || family == 5) {
        // Packed earth / leaf mould have no slab grid. Gravel, roots and damp
        // patches are restrained inside the footprint, never raised obstacles.
        k += detail * broad * 0.25;
        let root = (v - 28.0 - (u * 0.12).sin() * 8.0).abs();
        if family == 5 && root < 0.7 { k *= 0.79; }
        if grain > 0.21 { k += detail * 0.18; }
    } else {
        k += (stone - 0.5) * 0.10 * detail;
        if joint < 0.55 { k *= 0.79; }
        else if joint < 1.1 { k *= 1.06; }
        if !cap && family == 2 {
            let inlay = ((u + v).rem_euclid(32.0) - 16.0).abs();
            if inlay < 1.15 { k *= 1.32; }
        }
        if !cap && family == 6 && x > 3.0 && y > 3.0 && (x < 4.3 || y < 4.3) { k *= 1.32; }
        if family == 7 { k += detail * (u * 2.7 + (v * 0.08).sin()).sin() * 0.08; }
        if family == 3 && detail > 0.7 {
            let crack = (v - 20.0 - u * 0.29 - (u * 0.6).sin() * 0.9).abs();
            if crack < 0.35 { k *= 0.72; }
        }
    }
    shade(base, k)
}

fn masonry(family: usize, s: f32, t: f32, lit: f32) -> Rgb {
    let z = t * WALL_FACE_H;
    if family == 7 {
        // Timber retaining wall: long beams, wood grain, iron strap and bolts.
        let beam = z.rem_euclid(10.0);
        let strap = (s.rem_euclid(32.0) - 16.0).abs();
        let grain = (s * 0.9 + (z * 0.7).sin() * 1.2).sin() * 0.055;
        if strap < 1.2 {
            return shade(grim([62, 65, 61]), lit * if beam < 1.4 { 1.45 } else { 0.85 });
        }
        return shade(material_base(7, true), lit * (1.0 + grain) * if beam < 0.6 { 0.72 } else { 1.0 });
    }
    let (width, rows) = match family {
        1 => (16.0, 4.0),
        2 => (32.0, 2.0),
        3 => (32.0, 2.0),
        4 | 5 => (16.0, 3.0),
        _ => (32.0, 3.0),
    };
    let course = WALL_FACE_H / rows;
    let row = (z / course).floor();
    let rough = family == 4 || family == 5 || family == 1;
    let warp = if rough {
        (s * std::f32::consts::TAU / 16.0).sin() * if family == 1 { 0.7 } else { 2.1 }
    } else { 0.0 };
    let y = (z + warp).rem_euclid(course);
    let side_warp = if matches!(family, 4 | 5) { (z * 0.42).sin() * 1.7 } else { 0.0 };
    let x = (s + row * width * 0.5 + side_warp).rem_euclid(width);
    let joint = y.min(course - y).min(x.min(width - x));
    let block = jitter(((s + row * width * 0.5) / width).floor() as i32, row as i32, family as u64 + 991);
    let grain = fbm(s * 0.32, z * 0.29, family as u64 + 814, 3) - 0.5;
    let mut k = lit * (0.91 + block * 0.19 + grain * if rough { 0.30 } else { 0.17 });
    // Hairline mortar and a small bevel, not the old wide horizontal stripes.
    if joint < 0.42 { k *= 0.73; }
    else if joint < 0.9 { k *= 1.09; }
    if z < 2.3 { k = lit * (1.12 + grain * 0.12); }
    if z > 34.0 { k *= 1.0 - (z - 34.0) * 0.025; }
    if rough && z > 24.0 { k *= 0.94 + grain * 0.18; }
    let base = if family == 4 { grim([121, 112, 93]) } else { material_base(family, true) };
    let c = shade(base, k);
    if matches!(family, 1 | 5) && z > 23.0 && grain < 0.0 {
        let moss = grim([49, 72, 43]);
        let amount = ((z - 23.0) / 17.0 * -grain * 1.6).min(0.28);
        return [
            (c[0] as f32 * (1.0 - amount) + moss[0] as f32 * amount) as u8,
            (c[1] as f32 * (1.0 - amount) + moss[1] as f32 * amount) as u8,
            (c[2] as f32 * (1.0 - amount) + moss[2] as f32 * amount) as u8,
        ];
    }
    c
}

/// Bake material states through the exact same diamond and dropped face quads
/// as the ordinary wall. Surface texture is flat: navigation and joins stay
/// on the shared plane, with no clipping or relief-driven normal seams.
pub fn bake_material_iso(wall: bool, family: usize, phase: u64) -> [RgbaImage; 3] {
    let mut top = Splat::new(S, S);
    let mut left = Splat::new(S, S);
    let mut right = Splat::new(S, S);
    let (south_f, east_f) = wall_face_polys();
    let lift = WALL_FACE_H * SS as f32;
    let drop = WALL_FACE_DROP * SS as f32;
    for y in 0..S {
        for x in 0..S {
            let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
            let u = (px - AX) / 3.0 + (py - 12.0) / 1.5;
            let v = (py - 12.0) / 1.5 - (px - AX) / 3.0;
            if (0.0..64.0).contains(&u) && (0.0..64.0).contains(&v) {
                let c = material_surface(family, u, v, wall, phase);
                top.put(x, y, 0.0, c, (0.0, 0.0, 1.0));
            }
            if wall && in_poly(px, py, &south_f) {
                let t = (py - (AY + px * 0.5 - lift + drop)) / lift;
                left.put(x, y, 0.0, masonry(family, px / HW * 64.0, t, 0.78), (0.0, 0.0, 1.0));
            }
            if wall && in_poly(px, py, &east_f) {
                let t = (py - (AY + HH - (px - AX) * 0.5 - lift + drop)) / lift;
                right.put(x, y, 0.0, masonry(family, (px - AX) / HW * 64.0, t, 0.54), (0.0, 0.0, 1.0));
            }
        }
    }
    let mut faces = [downsample(&top), downsample(&left), downsample(&right)];
    paint::seam_iso(&mut faces[0], if wall { Tile::Wall } else { Tile::Floor });
    faces
}

pub fn bake_material_square(wall: bool, family: usize, phase: u64) -> RgbaImage {
    let mut sp = Splat::new(S, S);
    for y in 0..S {
        for x in 0..S {
            let c = material_surface(family, (x as f32 + 0.5) / 3.0, (y as f32 + 0.5) / 3.0, wall, phase);
            sp.put(x, y, 0.0, c, (0.0, 0.0, 1.0));
        }
    }
    downsample(&sp)
}

/// Iso heightfield surface (shared A=(32,20) ground diamond) plus only the
/// short exposed masonry faces. Other families keep their relief in Top.
pub fn bake_iso(tile: BakeTile, seed: u64) -> [RgbaImage; 3] {
    if matches!(tile, BakeTile::Wall | BakeTile::Floor) {
        return bake_material_iso(tile == BakeTile::Wall, 0, seed);
    }
    let mut sp = Splat::new(S, S);
    let steps = 240;
    // Height scale: keep the summit inside the cell top (peak y >= 1px).
    let hscale = match tile {
        BakeTile::Mountain => 0.55,
        // 1.0, not 0.72: the crown amplitude in `height()` is already solved
        // against the iso silhouette constraint, so a scale here just eats the
        // difference and every tree bakes flat inside its own diamond.
        BakeTile::DeepForest => 1.0,
        BakeTile::Forest => 1.0,
        BakeTile::Rock => 0.6,
        // Wall heights are authored for iso already (12+4 fits under cell top).
        BakeTile::Wall => 1.0,
        // Water is a shallow channel: the wave field is gentle, and a tall
        // scale would push the banks out of the cell and hide the surface.
        BakeTile::River => 0.85,
        BakeTile::Ford => 0.85,
        // Ruins needs its stubs inside the cell; Floor is almost flat, so a
        // low scale keeps the settled joints from turning into a relief map.
        BakeTile::Ruins => 0.95,
        BakeTile::Floor => 0.30,
    };
    for iy in 0..steps {
        for ix in 0..steps {
            let u = (ix as f32 + 0.5) / steps as f32 * 64.0;
            let v = (iy as f32 + 0.5) / steps as f32 * 64.0;
            let (c, n, h_raw) = shade_sample(tile, u, v, seed);
            // Non-wall relief returns to the ground plane at every footprint
            // boundary. Raised rock/tree/water detail stays inside the cell.
            let h = if tile == BakeTile::Wall {
                h_raw * hscale
            } else {
                let edge = u.min(v).min(64.0 - u).min(64.0 - v);
                let taper = (edge / 9.0).clamp(0.0, 1.0);
                h_raw * hscale * taper * taper * (3.0 - 2.0 * taper)
            };
            let sx = 96.0 + (u - v) * 1.5;
            let sy = 12.0 + (u + v) * 0.75 - h * SS as f32;
            let depth = (u + v) - 2.0 * h;
            sp.disc(sx, sy, depth, c, n, 2);
        }
    }
    // Non-masonry relief owns only a top plate; extruded material faces are
    // rendered by bake_material_iso above.
    let mut faces = [downsample(&sp), blank(64, 64), blank(64, 64)];
    paint::seam_iso(&mut faces[0], map_tile(tile));
    faces
}


fn in_poly(px: f32, py: f32, pts: &[[f32; 2]]) -> bool {
    let mut inside = false;
    let n = pts.len();
    for i in 0..n {
        let [ax, ay] = pts[i];
        let [bx, by] = pts[(i + 1) % n];
        if (ay > py) != (by > py) && px < (bx - ax) * (py - ay) / (by - ay) + ax {
            inside = !inside;
        }
    }
    inside
}

// Square-view surface litter; iso surfaces use their heightfield detail
// without free-floating decals outside the ground footprint.

fn stones(img: &mut RgbaImage, seed: u64, count: u64, y0: f32, y1: f32, dark: Rgb, light: Rgb) {
    for i in 0..count {
        let x = 3.0 + h01(seed, 400 + i) * 58.0;
        let y = y0 + h01(seed, 500 + i) * (y1 - y0);
        let w = 2.0 + (h01(seed, 600 + i) * 3.0).floor();
        fill_rect(img, x, y, w, w * 0.7, if i % 2 == 0 { dark } else { light });
    }
}


fn tufts(img: &mut RgbaImage, seed: u64, count: u64, c: Rgb) {
    for i in 0..count {
        let x = 8.0 + h01(seed, 900 + i) * 48.0;
        let y = 18.0 + h01(seed, 950 + i) * 20.0;
        let up = shade(c, 1.3);
        fill_rect(img, x, y, 2.0, 3.0, up);
        fill_rect(img, x + 2.0, y + 2.0, 3.0, 2.0, up);
    }
}


/// Square-view clutter: same families, concentrated at the foot band.
pub fn clutter_square(tile: BakeTile, seed: u64, img: &mut RgbaImage) {
    match tile {
        BakeTile::Ruins => {
            let stone = grim([96, 94, 86]);
            stones(img, seed ^ 41, 16, 38.0, 62.0, shade(stone, 0.55), shade(stone, 1.05));
        }
        BakeTile::Floor => {}
        BakeTile::River | BakeTile::Ford => {}
        BakeTile::Mountain | BakeTile::Rock => {
            let rock = grim([113, 114, 105]);
            stones(img, seed ^ 21, 12, 40.0, 62.0, shade(rock, 0.6), shade(rock, 1.2));
        }
        BakeTile::DeepForest | BakeTile::Forest => {
            let leaf = if tile == BakeTile::DeepForest { grim([21, 43, 28]) } else { grim([32, 61, 31]) };
            tufts(img, seed ^ 22, 6, leaf);
        }
        BakeTile::Wall => {
            let stone = grim([66, 66, 61]);
            stones(img, seed ^ 23, 12, 42.0, 62.0, shade(stone, 0.55), shade(stone, 1.1));
        }
    }
}

// ---------------------------------------------------------------------------
// Wall.tall (rung v4): the 2-cell curtain. One 192x384-super buffer renders
// a 40px masonry face anchored to the standard iso ground convention; split
// at 192: the UPPER cell holds the parapet cap + crown of the face, the
// LOWER cell holds the face body + foot/skirt with the ground anchor
// EXACTLY at (32,20) again. Composite consumers stack [col, r+1] over
// the tall anchor cell (see manifest.v1 states notes in ART_PIPELINE.md).

/// Taller curtain heighttable: same diagonal run, face joint 36/merlon 40.
fn tall_wall_height(u: f32, v: f32, seed: u64) -> f32 {
    let ground = 2.0 + fbm(u * 0.2, v * 0.2, seed ^ 23, 2) * 1.2;
    const RUN: f32 = 72.0;
    if (u + v - RUN).abs() >= 6.0 {
        ground
    } else {
        let merlon = ((u / 7.0) as i32 % 2 == 0) && (u % 7.0) < 4.6;
        let mut h: f32 = if merlon { 40.0 } else { 36.0 };
        if (u % 8.0) < 2.6 {
            h = (h + 2.5).min(42.0);
        }
        let rob = fbm(u * 0.6, 3.3, seed ^ 37, 2);
        if rob > 0.60 && (u + v - RUN).abs() > 3.0 {
            h *= 0.74 + rob * 0.18;
        }
        h.max(ground)
    }
}

/// Masonry face texture for a wall splat drop (coursing + ties + slits).
fn masonry_tex(seed: u64, t: i32, x: i32) -> f32 {
    let course = (t as f32 / 7.0 + noise2(x as f32 * 0.05, 0.0, seed ^ 79) * 2.0) as i32;
    let tie = ((x as f32 + course as f32 * 3.0) % 14.0) < 1.4;
    let slit = (x % 30) < 4 && t > 18 && t < 42;
    if slit {
        0.42
    } else if (t % 7) < 1 || tie {
        0.62
    } else {
        1.0
    }
}

// ---------------------------------------------------------------------------
// Prop SDF lane.
//
// A small purpose-built ray-marcher for free-standing props, authored directly
// in PLATE coordinates: `x`/`y` are the 64x64 cell (y grows downward, like the
// image), `z` is depth with the viewer at -z, and a prop's base sits on
// `PROP_GROUND`. There is no hidden transform, so a part placed at (32, 58)
// stands on the tile anchor by construction.
//
// `bake_actor`'s rig lane is deliberately NOT reused: its camera offsets the
// origin by 96 while its buffer is `64 * SS` = 256, so its own figure
// coordinates do not land where its rigs assume, and props authored that way
// came out mis-scaled and clipped. Authoring against the plate removes the
// dependency on that transform entirely.

/// A prop's ground line in plate px. `crates/view/src/props.rs` centres the
/// plate on the tile's own ground point (it applies no `CELL_ANCHOR_DY`), so the
/// plate centre IS the tile centre: props are authored standing on `y = 32` and
/// occupy the UPPER half of the plate, giving them about half a tile of height.
/// Authoring them against the plate's bottom row instead sank them a third of a
/// tile below the tile they stood on and let them swallow their neighbours.
const PROP_GROUND: f32 = 32.0;
/// Plate centre line.
const PROP_CX: f32 = 32.0;

type P3 = (f32, f32, f32);

const P_IRON: Rgb = [92, 88, 84];
const P_IRON_DARK: Rgb = [58, 56, 55];
const P_GRANITE: Rgb = [124, 126, 122];
const P_GRANITE_DARK: Rgb = [86, 89, 86];
const P_EMBER: Rgb = [226, 128, 52];
const P_FLAME: Rgb = [246, 196, 92];
const P_RUNE: Rgb = [150, 226, 214];
const P_GLASS: Rgb = [108, 178, 150];
const P_CORK: Rgb = [122, 92, 54];
const P_GOLD: Rgb = [198, 162, 84];
const P_MOSS: Rgb = [58, 84, 52];

#[derive(Clone, Copy, PartialEq)]
enum PShape {
    Ball,
    Rod,
    Slab,
}

#[derive(Clone, Copy)]
pub(crate) struct PPart {
    shape: PShape,
    a: P3,
    b: P3,
    r: f32,
    albedo: Rgb,
    /// 0 = matte, >0 lifts the part's luminance floor and ignores shading.
    glow: f32,
}

fn ball(x: f32, y: f32, z: f32, r: f32, albedo: Rgb) -> PPart {
    PPart { shape: PShape::Ball, a: (x, y, z), b: (0.0, 0.0, 0.0), r, albedo, glow: 0.0 }
}

fn rod(a: P3, b: P3, r: f32, albedo: Rgb) -> PPart {
    PPart { shape: PShape::Rod, a, b, r, albedo, glow: 0.0 }
}

fn slab(x: f32, y: f32, z: f32, half: P3, r: f32, albedo: Rgb) -> PPart {
    PPart { shape: PShape::Slab, a: (x, y, z), b: half, r, albedo, glow: 0.0 }
}

fn glow(mut p: PPart, g: f32) -> PPart {
    p.glow = g;
    p
}

fn p_sub(a: P3, b: P3) -> P3 {
    (a.0 - b.0, a.1 - b.1, a.2 - b.2)
}

fn p_len(a: P3) -> f32 {
    (a.0 * a.0 + a.1 * a.1 + a.2 * a.2).sqrt()
}

fn seg_dist(p: P3, a: P3, b: P3) -> f32 {
    let ab = p_sub(b, a);
    let ap = p_sub(p, a);
    let denom = ab.0 * ab.0 + ab.1 * ab.1 + ab.2 * ab.2;
    let t = if denom <= 1e-6 {
        0.0
    } else {
        ((ap.0 * ab.0 + ap.1 * ab.1 + ap.2 * ab.2) / denom).clamp(0.0, 1.0)
    };
    p_len((ap.0 - ab.0 * t, ap.1 - ab.1 * t, ap.2 - ab.2 * t))
}

fn part_dist(p: P3, q: &PPart) -> f32 {
    match q.shape {
        PShape::Ball => p_len(p_sub(p, q.a)) - q.r,
        PShape::Rod => seg_dist(p, q.a, q.b) - q.r,
        PShape::Slab => {
            let d = (
                (p.0 - q.a.0).abs() - q.b.0,
                (p.1 - q.a.1).abs() - q.b.1,
                (p.2 - q.a.2).abs() - q.b.2,
            );
            let out = (d.0.max(0.0), d.1.max(0.0), d.2.max(0.0));
            p_len(out) + d.0.max(d.1).max(d.2).min(0.0) - q.r
        }
    }
}

fn prop_scene(parts: &[PPart], p: P3) -> (f32, usize) {
    let mut best = f32::MAX;
    let mut id = 0;
    for (i, q) in parts.iter().enumerate() {
        let d = part_dist(p, q);
        if d < best {
            best = d;
            id = i;
        }
    }
    (best, id)
}

fn prop_normal(parts: &[PPart], p: P3) -> P3 {
    let e = 0.35;
    let d = |dx: f32, dy: f32, dz: f32| prop_scene(parts, (p.0 + dx, p.1 + dy, p.2 + dz)).0;
    let n = (
        d(e, 0.0, 0.0) - d(-e, 0.0, 0.0),
        d(0.0, e, 0.0) - d(0.0, -e, 0.0),
        d(0.0, 0.0, e) - d(0.0, 0.0, -e),
    );
    let l = p_len(n).max(1e-5);
    (n.0 / l, n.1 / l, n.2 / l)
}

/// SDF parts for a free-standing prop, in plate coordinates. `None` means no
/// rig, and the caller keeps the painted plate.
pub fn prop_rig(key: &str) -> Option<Vec<PPart>> {
    let g = PROP_GROUND;
    let cx = PROP_CX;
    let v = match key {
        // A coal bowl on three legs, with a live flame. Everything is measured
        // in plate px and the plate ends at y = 0, so the whole prop has to fit
        // inside the 32px above its ground line. `glow` is a luminance FLOOR,
        // not a boost: an emissive part needs a value above 1 to read as lit.
        "TorchBrazier" => {
            let mut v = Vec::new();
            for (dx, dz) in [(-7.0f32, 5.0f32), (7.0, 5.0), (0.0, -7.0)] {
                v.push(rod(
                    (cx + dx * 1.5, g, dz * 1.5),
                    (cx + dx * 0.55, g - 12.0, dz * 0.55),
                    1.6,
                    P_IRON_DARK,
                ));
            }
            v.push(slab(cx, 16.0, 0.0, (9.0, 4.0, 9.0), 4.0, P_IRON));
            v.push(slab(cx, 11.0, 0.0, (10.5, 1.5, 10.5), 1.5, shade(P_IRON, 1.14)));
            for (dx, dz) in [(-4.0f32, -2.0f32), (3.5, 2.5), (0.0, 4.0)] {
                v.push(glow(ball(cx + dx, 9.0, dz, 2.6, P_EMBER), 0.95));
            }
            for (i, (y, r)) in [(7.0f32, 4.6f32), (4.6, 3.4), (2.8, 2.2), (1.6, 1.3)]
                .into_iter()
                .enumerate()
            {
                let c = if i < 2 { P_EMBER } else { P_FLAME };
                v.push(glow(ball(cx, y, 0.0, r, c), if i < 2 { 1.05 } else { 1.2 }));
            }
            v
        }
        // A standing monolith with a lit rune on its face and moss at the foot.
        "RuneStone" => {
            let mut v = vec![
                slab(cx, g - 3.0, 0.0, (9.0, 2.5, 6.0), 2.0, P_GRANITE_DARK),
                slab(cx, g - 7.0, 0.0, (7.0, 1.5, 4.5), 1.5, P_GRANITE),
                slab(cx, 17.0, 1.0, (4.5, 7.0, 2.5), 3.0, P_GRANITE),
            ];
            // Rune: a vertical stroke with two arms, proud of the face.
            //
            // NOTE the rounded-box convention below: this SDF gives a part an
            // OUTER extent of `half + r`, so the slab's flat front face sits at
            // `centre_z - (half_z + r)` = -4.5, not at its `half_z`. Authoring
            // the rune against `half_z` buried it inside the stone.
            let f = 5.2;
            v.push(glow(rod((cx, 10.5, -f), (cx, 23.5, -f), 0.85, P_RUNE), 1.15));
            v.push(glow(rod((cx, 23.5, -f), (cx - 3.0, 18.5, -f), 0.85, P_RUNE), 1.15));
            v.push(glow(rod((cx, 15.5, -f), (cx + 3.0, 20.0, -f), 0.85, P_RUNE), 1.15));
            for (dx, dz) in [(-8.0f32, 3.0f32), (7.5, -2.5), (-3.0, -5.5)] {
                v.push(ball(cx + dx, g - 3.0, dz, 2.8, P_MOSS));
            }
            v
        }
        // A stepped plinth carrying a relic.
        "RelicPedestal" => vec![
            slab(cx, g - 3.0, 0.0, (10.0, 2.5, 7.0), 2.0, P_GRANITE_DARK),
            slab(cx, g - 8.0, 0.0, (7.0, 2.0, 5.0), 1.5, P_GRANITE),
            slab(cx, g - 13.0, 0.0, (5.0, 3.5, 4.0), 2.0, shade(P_GRANITE, 0.9)),
            slab(cx, 12.0, 0.0, (7.5, 2.0, 6.0), 1.5, P_GRANITE),
            glow(ball(cx, 6.0, 0.0, 4.0, P_GOLD), 0.95),
            glow(ball(cx, 4.5, -1.0, 1.9, shade(P_RUNE, 1.1)), 1.25),
        ],
        // A small round flask: body, neck, cork, one glint.
        "Potion" => vec![
            ball(cx, 22.0, 0.0, 7.0, P_GLASS),
            rod((cx, 17.0, 0.0), (cx, 11.0, 0.0), 2.6, P_GLASS),
            slab(cx, 8.0, 0.0, (2.5, 2.0, 2.5), 1.0, P_CORK),
            glow(ball(cx - 2.5, 20.0, -4.5, 1.8, shade(P_GLASS, 1.5)), 1.30),
        ],
        _ => return None,
    };
    Some(v)
}

/// Ray-march a prop rig into a 64px plate and drop the same ground shadow the
/// actor lane uses, so props and figures sit on the tile identically.
pub fn bake_prop_rig(key: &str) -> Option<RgbaImage> {
    let parts = prop_rig(key)?;
    let mut img = RgbaImage::from_pixel(64, 64, image::Rgba([0, 0, 0, 0]));
    // Sun from the upper left and slightly in front (negative y is up, and the
    // viewer is at -z), matching the terrain rig's north-west key.
    let sun = {
        let v = (-0.46f32, -0.72f32, -0.52f32);
        let l = p_len(v);
        (v.0 / l, v.1 / l, v.2 / l)
    };
    let fill = {
        let v = (0.5f32, 0.34f32, -0.5f32);
        let l = p_len(v);
        (v.0 / l, v.1 / l, v.2 / l)
    };
    for py in 0..64i32 {
        for px in 0..64i32 {
            let o = (px as f32 + 0.5, py as f32 + 0.5, -90.0);
            let (mut t, mut hit) = (0.0f32, false);
            let mut point = o;
            let mut id = 0usize;
            for _ in 0..96 {
                let p = (o.0, o.1, o.2 + t);
                let (d, i) = prop_scene(&parts, p);
                if d < 0.3 {
                    hit = true;
                    point = p;
                    id = i;
                    break;
                }
                t += d.max(0.3);
                if t > 190.0 {
                    break;
                }
            }
            if !hit {
                continue;
            }
            let n = prop_normal(&parts, point);
            let key_l = (n.0 * sun.0 + n.1 * sun.1 + n.2 * sun.2).max(0.0);
            let fill_l = (n.0 * fill.0 + n.1 * fill.1 + n.2 * fill.2).max(0.0);
            // Rim: graze light where the surface turns away from the viewer.
            let rim = (1.0 - (-n.2).abs()).powi(3) * 0.30;
            let lum = (0.42 + 0.74 * key_l + 0.20 * fill_l + rim).min(1.30);
            let part = &parts[id];
            let c = shade(part.albedo, lum.max(part.glow));
            img.put_pixel(px as u32, py as u32, image::Rgba([c[0], c[1], c[2], 255]));
        }
    }
    crate::bake_actor::anchor_shadow(&mut img);
    Some(img)
}

pub fn bake_wall_tall(seed: u64) -> [RgbaImage; 2] {
    let (w_img, h_img) = (S, S + S); // 192 x 384
    let mut sp = Splat::new(w_img, h_img);
    let steps = 240;
    // Buffer ground line: face heights live at gy - h*3 (40px face top at
    // 168 = inside the UPPER cell band), split at the cell boundary 192.
    let gy = 288.0;
    for iy in 0..steps {
        for ix in 0..steps {
            let u = (ix as f32 + 0.5) / steps as f32 * 64.0;
            let v = (iy as f32 + 0.5) / steps as f32 * 64.0;
            let h = tall_wall_height(u, v, seed);
            let n = normal_of(&|a, b| tall_wall_height(a, b, seed), u, v);
            let al = albedo(BakeTile::Wall, u, v, h, n, seed);
            let c = shade_of(BakeTile::Wall, &|a, b| tall_wall_height(a, b, seed), al, n, u, v, h, seed);
            let sx = 96.0 + (u - v) * 1.5;
            let sy = gy - (64.0 - (u + v)) * 0.75 - h * SS as f32;
            let depth = (u + v) - 2.0 * h;
            sp.disc(sx, sy, depth, c, n, 2);
        }
    }
    // South face skirt: extend down 40 super px from the silhouette edge,
    // coursing/slits/buttress texture, erode near the robbing.
    for x in 0..w_img {
        let mut y_bottom = -1;
        for y in (0..h_img).rev() {
            let i = (y * w_img + x) as usize;
            if sp.alpha[i] {
                y_bottom = y;
                break;
            }
        }
        if y_bottom < 0 {
            continue;
        }
        let ei = (y_bottom * w_img + x) as usize;
        let (nx, _, _) = sp.norm[ei];
        let side_k = if nx < 0.0 { 0.62 } else { 1.12 };
        for t in 1..=88i32 {
            let y = y_bottom + t;
            if y >= h_img {
                break;
            }
            let i = (y * w_img + x) as usize;
            if sp.alpha[i] {
                continue;
            }
            let tf = t as f32 / 88.0;
            let k = side_k * (1.0 - tf * 0.16) * masonry_tex(seed, t, x);
            let dropout = noise2(x as f32 * 0.25, t as f32 * 0.13, seed ^ 93) * 0.72;
            if dropout < tf * 0.55 {
                continue;
            }
            let c = shade(shade(grim([66, 66, 61]), 1.22), k);
            sp.put(x, y, sp.depth[ei] + 0.01, c, sp.norm[ei]);
        }
    }
    // Split at the CELL boundary: upper = [0..192] (parapet cap + face
    // crown), lower = [192..384] (face body + ground diamond anchor), each
    // 64 world px; gy chosen so the lower anchor lands at ~(32, 20). (v4
    // bug fix: splitting at 64 super-px left the upper cell EMPTY and the
    // curtain invisible in composited views.)
    let upper = crop_rows(&sp, 0, 192);
    let lower = crop_rows(&sp, 192, h_img);
    [upper, lower]
}

/// Crop a super-resolution band out of a bake buffer and box-downsample it
/// to the 64px plate cell.
fn crop_rows(sp: &Splat, y0: i32, y1: i32) -> RgbaImage {
    let mut cell = blank(64, 64);
    for py in 0..64i32 {
        for px in 0..64i32 {
            let (mut r, mut g, mut b, mut a) = (0f32, 0f32, 0f32, 0f32);
            for dy in 0..SS {
                for dx in 0..SS {
                    let sy = y0 + py * SS + dy;
                    let sx = px * SS + dx;
                    if sy >= y1 {
                        continue;
                    }
                    let i = (sy * sp.w + sx) as usize;
                    if sp.alpha[i] {
                        r += sp.col[i].0;
                        g += sp.col[i].1;
                        b += sp.col[i].2;
                        a += 1.0;
                    }
                }
            }
            if a > 0.0 {
                let n = (SS * SS) as f32;
                cell.put_pixel(
                    px as u32,
                    py as u32,
                    image::Rgba([(r / a) as u8, (g / a) as u8, (b / a) as u8, (255.0 * a / n) as u8]),
                );
            }
        }
    }
    cell
}

// ---------------------------------------------------------------------------
// Prop bakes (batch-2 stretch): block-defined geometry, oblique projection
// anchored so a ~52px-tall object sits inside the 64px cell.

fn prop_height(prop: BakeProp, u: f32, v: f32) -> f32 {
    match prop {
        BakeProp::RelicPedestal => {
            // Review v3: orb owns the prop (v2 crushed it into a ziggurat).
            // Lower shaft (12), bigger hemisphere (r 9.5), felt ring beneath.
            let inbox = |x0: f32, y0: f32, x1: f32, y1: f32| {
                u >= x0 && u < x1 && v >= y0 && v < y1
            };
            let plinth: f32 = if inbox(24.0, 24.0, 40.0, 40.0) {
                12.0
            } else if inbox(18.0, 18.0, 46.0, 46.0) {
                7.0
            } else if inbox(12.0, 12.0, 52.0, 52.0) {
                4.0
            } else {
                0.0
            };
            let d = ((u - 32.0) * (u - 32.0) + (v - 32.0) * (v - 32.0)).sqrt();
            let orb = if d < 9.5 {
                12.0 + (90.25 - d * d).sqrt()
            } else {
                0.0
            };
            plinth.max(orb)
        }
    }
}

fn prop_albedo(prop: BakeProp, u: f32, v: f32, h: f32) -> Rgb {
    match prop {
        BakeProp::RelicPedestal => {
            let gold = [232, 191, 89];
            let sea = [102, 212, 196];
            let stone = grim([113, 114, 105]);
            let felt = [137, 83, 108];
            let d = ((u - 32.0) * (u - 32.0) + (v - 32.0) * (v - 32.0)).sqrt();
            if h > 12.5 && d < 9.5 {
                // Relic orb: gold shell, sea-glass core, NW specular chip.
                if d < 4.2 {
                    shade(sea, 0.95)
                } else if d < 5.4 && u < 30.0 && v < 31.0 {
                    [255, 255, 255]
                } else {
                    shade(gold, 0.95 + (9.5 - d) * 0.04)
                }
            } else if h > 11.5 && d < 12.5 {
                felt // cushion ring under the orb
            } else if h > 6.5 {
                shade(stone, 1.08) // shaft
            } else {
                shade(stone, 0.92) // base step
            }
        }
    }
}

/// Bake one prop: oblique splat, same lighting rig, transparent margins.
pub fn bake_prop(prop: BakeProp) -> RgbaImage {
    let seed = hash64(0xba3e_90d0 + prop as u64);
    let hf = |a: f32, b: f32| prop_height(prop, a, b);
    let mut sp = Splat::new(S, S);
    let steps = 190;
    for iy in 0..steps {
        for ix in 0..steps {
            let u = (ix as f32 + 0.5) / steps as f32 * 64.0;
            let v = (iy as f32 + 0.5) / steps as f32 * 64.0;
            let h = hf(u, v);
            if h <= 0.0 {
                continue;
            }
            let n = normal_of(&hf, u, v);
            let al = prop_albedo(prop, u, v, h);
            let c = shade_of(BakeTile::Floor, &hf, al, n, u, v, h, seed);
            // Prop-space oblique anchor (see batch-2 notes): south edge of
            // the object lands inside the cell, summit clear of the top.
            let sx = 96.0 + (u - v) * 1.5;
            let sy = 128.0 + (u + v) * 0.75 - h * SS as f32;
            sp.disc(sx, sy, (u + v) - 2.0 * h, c, n, 2);
        }
    }
    downsample(&sp)
}

/// Box 3x3 downsample (alpha-weighted) back to the 64px cell.
fn downsample(sp: &Splat) -> RgbaImage {
    let mut out = blank(64, 64);
    for y in 0..64i32 {
        for x in 0..64i32 {
            let (mut r, mut g, mut b, mut a) = (0.0f32, 0.0, 0.0, 0.0);
            for dy in 0..SS {
                for dx in 0..SS {
                    let i = ((y * SS + dy) * S + (x * SS + dx)) as usize;
                    if sp.alpha[i] {
                        let c = sp.col[i];
                        r += c.0;
                        g += c.1;
                        b += c.2;
                        a += 1.0;
                    }
                }
            }
            let n = (SS * SS) as f32;
            if a > 0.0 {
                out.put_pixel(
                    x as u32,
                    y as u32,
                    image::Rgba([
                        (r / a) as u8,
                        (g / a) as u8,
                        (b / a) as u8,
                        (255.0 * a / n) as u8,
                    ]),
                );
            }
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Tree sprites.
//
// A 64px iso cell cannot hold a tree: the ground diamond already occupies rows
// 4..36, and a crown raised inside that silhouette is hidden by it — measured
// at a 7px maximum rise, which is why a forest baked as flat dark smudges.
// So the canopy is a separate overhanging sprite drawn above its tile, the
// same way actors and props already are. Bottom-anchored at row 63: the view
// lifts the sprite by half its drawn height so the trunk foot lands on the
// tile's anchor point.

/// One tree: a trunk, a root flare, and 4-6 overlapping canopy lobes with a lit
/// north-west shoulder and a shaded south-east skirt.
pub fn bake_tree(deep: bool, variant: u64) -> RgbaImage {
    let mut img = RgbaImage::from_pixel(64, 64, image::Rgba([0, 0, 0, 0]));
    let seed = 0x7ee5_0000 ^ variant.wrapping_mul(0x9e37_79b9);
    let leaf = if deep { grim([74, 112, 56]) } else { grim([96, 138, 66]) };
    let bark = if deep { grim([54, 44, 34]) } else { grim([68, 55, 40]) };
    let base_x = 32.0 + (h01(seed, 1) - 0.5) * 5.0;
    let lean = (h01(seed, 2) - 0.5) * 3.0;

    // Trunk with a root flare, tapering into the canopy.
    let trunk_h = 26.0 + h01(seed, 3) * 8.0;
    for y in 0..64i32 {
        let t = (63 - y) as f32 / 63.0;
        if t > trunk_h / 64.0 {
            continue;
        }
        let widen = 1.0 + (1.0 - t / (trunk_h / 64.0)).max(0.0) * 1.4;
        let half = (2.6 * widen).max(1.0);
        let cx = base_x + lean * t;
        for x in 0..64i32 {
            if (x as f32 + 0.5 - cx).abs() <= half {
                // Bark grain: two lit and two dark stripes down the trunk.
                let k = match (x as i32) % 3 {
                    0 => 1.18,
                    1 => 0.86,
                    _ => 1.0,
                };
                let c = shade(bark, k * (1.0 - (1.0 - t) * 0.25));
                img.put_pixel(x as u32, y as u32, image::Rgba([c[0], c[1], c[2], 255]));
            }
        }
    }

    // Canopy: rasterize the lobe union to a coverage mask FIRST, then shade the
    // mask as a single volume.
    //
    // The previous pass shaded every lobe with its own radial ramp and
    // max-blended them, so each lobe printed its own bright centre and dark rim
    // over its neighbours and a tree read as a pile of flat discs rather than a
    // canopy. Coverage also supplies the shape terms the volume needs: lit along
    // the north-west shoulder, falling into shade on the underside, dark again
    // along the silhouette rim.
    let lobes = (5 + (h01(seed, 4) * 3.0) as usize).min(8);
    let spread = if deep { 15.0 } else { 18.0 };
    let mid_y = 63.0 - trunk_h * 0.92;
    let mut cov = [[0u8; 64]; 64];
    for i in 0..lobes {
        let ang = std::f32::consts::TAU * (i as f32 / lobes as f32)
            + h01(seed, 10 + i as u64) * 0.9;
        let rad = 0.45 + h01(seed, 20 + i as u64) * 0.55;
        let r = (if deep { 9.5 } else { 11.5 }) * (0.74 + h01(seed, 30 + i as u64) * 0.46);
        let cx = (base_x + lean * 0.5 + ang.cos() * spread * rad).clamp(r + 2.0, 62.0 - r);
        let cy = (mid_y - 7.0 + ang.sin() * spread * 0.60 * rad).max(r + 3.0);
        for y in 0..64i32 {
            for x in 0..64i32 {
                let dx = x as f32 + 0.5 - cx;
                let dy = y as f32 + 0.5 - cy;
                if dx * dx + dy * dy <= r * r {
                    cov[y as usize][x as usize] = cov[y as usize][x as usize].saturating_add(1);
                }
            }
        }
    }
    // Crown lobe: the light-catching top of the mass.
    let top = 15.0 + h01(seed, 5) * 5.0;
    for y in 0..64i32 {
        for x in 0..64i32 {
            let dx = x as f32 + 0.5 - (base_x + lean * 0.35);
            let dy = y as f32 + 0.5 - top;
            if dx * dx * 1.15 + dy * dy <= 144.0 {
                cov[y as usize][x as usize] = cov[y as usize][x as usize].saturating_add(1);
            }
        }
    }
    let (mut y0, mut y1) = (63.0f32, 0.0f32);
    for (y, row) in cov.iter().enumerate() {
        if row.iter().any(|c| *c > 0) {
            y0 = y0.min(y as f32);
            y1 = y1.max(y as f32);
        }
    }
    let span = (y1 - y0).max(1.0);
    let mass_x = base_x + lean * 0.4;
    for y in 0..64i32 {
        for x in 0..64i32 {
            let n = cov[y as usize][x as usize];
            if n == 0 {
                continue;
            }
            // 0 at the crown top, 1 at the underside; -1 west .. +1 east.
            let vy = (y as f32 - y0) / span;
            let vx = (x as f32 + 0.5 - mass_x) / spread;
            let dome = (1.0 - (vx * 0.9).powi(2)).max(0.0);
            let mut k = 0.92
                + (1.0 - vy) * (1.0 - vx.max(0.0)) * 0.34
                - vy * 0.30
                - vx.max(0.0) * 0.22
                + dome * (1.0 - vy) * 0.10
                + (n.min(4) as f32) * 0.045;
            // Silhouette rim: darken where the mass ends so the canopy has a
            // defined edge instead of dissolving into the tile behind it.
            let rim = [
                (1i32, 0i32),
                (-1, 0),
                (0, 1),
                (0, -1),
                (1, 1),
                (1, -1),
                (-1, 1),
                (-1, -1),
            ]
            .iter()
            .any(|(dx, dy)| {
                let (nx, ny) = (x + dx, y + dy);
                nx < 0 || ny < 0 || nx >= 64 || ny >= 64 || cov[ny as usize][nx as usize] == 0
            });
            if rim {
                k *= 0.78;
            }
            // Leaf break-up, from smooth noise so it reads as foliage and not
            // as per-lobe banding.
            k += (noise2(x as f32 * 0.30, y as f32 * 0.30, seed ^ 5) - 0.5) * 0.14;
            let c = shade(leaf, k.clamp(0.52, 1.48));
            img.put_pixel(x as u32, y as u32, image::Rgba([c[0], c[1], c[2], 255]));
        }
    }
    // Root shadow so the foot is not a cut line on the tile.
    for x in 0..64i32 {
        for y in 58..64i32 {
            let dx = (x as f32 + 0.5 - base_x).abs();
            let dy = (y as f32 - 61.0).abs();
            if dx * 0.55 + dy > 9.0 {
                continue;
            }
            let p = img.get_pixel_mut(x as u32, y as u32);
            if p.0[3] > 0 {
                p.0[3] = p.0[3].saturating_sub(40);
            }
        }
    }
    img
}
