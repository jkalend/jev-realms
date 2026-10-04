//! 3D actor bake kit (D38 actor rung): parametric SDF bodies on simple
//! rigs — humanoid capsules / quadruped barrel+legs — sphere-traced
//! orthographic front view, NW sun + warm fill + 5-tap SDF AO + soft
//! map shadows, 3x supersample downsampled to 64px.
//!
//! Same philosophy as bake3d.rs (terrain): albedos snapped to the painted
//! plates' family colors so in-place key swaps keep the reads; the chunky
//! ±3px silhouette finish is applied by the caller (actors.rs::chunky), so
//! dimension the SDF figure to the same 32-unit grid the painters use.
//! Pure Rust, no assets.

use crate::raster::*;
use image::RgbaImage;

// ---------------------------------------------------------------------------
// World mapping: figure coordinates live in the PAINTED 32-unit grid so
// detail arms translate 1:1: x in [0,32], y in [0,32] (down), z forward.
// Canvas 256x256 supersampled (4x of 64): grid unit = 8px in every axis.

/// v2: 4x supersample (256px canvas -> 64) — the 3x pass left staircase
/// teeth on curves the owner flagged on the batch-2 review.
const SS: i32 = 4;
const S: i32 = 64 * SS;

type V3 = [f32; 3];

fn add(a: V3, b: V3) -> V3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
fn sub(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn mul(a: V3, k: f32) -> V3 {
    [a[0] * k, a[1] * k, a[2] * k]
}
fn dot(a: V3, b: V3) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn len(a: V3) -> f32 {
    dot(a, a).sqrt()
}
fn norm(a: V3) -> V3 {
    let l = len(a).max(1e-6);
    mul(a, 1.0 / l)
}

/// fig coords (32-unit grid, y down, +z toward the camera) -> world space
/// (y-up canvas, camera at -140 looking +z: rig-front +z = lower world z;
/// fig-top y=0 = upper world y. v1 shipped y- AND z-inverted, cancelling
/// visually only on symmetric parts — caught on the v2 review).
fn wp(x: f32, y: f32, z: f32) -> V3 {
    // Canvas world is y-up; pixel rows run down (py -> wy = 128 - py), so
    // fig-top (y=0) maps to wy +112; fig-foot (y=30) to -128. 8px/fig-unit.
    [x * 8.0 - 128.0, 112.0 - y * 8.0, -z * 8.0]
}

#[derive(Clone, Copy, PartialEq)]
enum Shape {
    Sphere,
    Capsule,
    RoundBox,
}

#[derive(Clone, Copy)]
pub struct Part {
    shape: Shape,
    /// sphere: center; capsule/box endpoint/box-center (fig coords).
    a: V3,
    /// capsule: second endpoint; box: half extents (fig coords).
    b: V3,
    /// radius (sphere/capsule) or corner radius (box).
    r: f32,
    albedo: Rgb,
    /// Emissive materials get a luminance floor and ignore shadow dimming.
    emissive: f32,
}

pub fn sphere(x: f32, y: f32, z: f32, r: f32, albedo: Rgb) -> Part {
    Part { shape: Shape::Sphere, a: [x, y, z], b: [0.0; 3], r, albedo, emissive: 0.0 }
}
pub fn capsule(ax: f32, ay: f32, az: f32, bx: f32, by: f32, bz: f32, r: f32, albedo: Rgb) -> Part {
    Part {
        shape: Shape::Capsule,
        a: [ax, ay, az],
        b: [bx, by, bz],
        r,
        albedo,
        emissive: 0.0,
    }
}
pub fn rbox(x: f32, y: f32, z: f32, hex: [f32; 3], r: f32, albedo: Rgb) -> Part {
    Part { shape: Shape::RoundBox, a: [x, y, z], b: hex, r, albedo, emissive: 0.0 }
}
pub fn emit(mut p: Part, e: f32) -> Part {
    p.emissive = e;
    p
}

fn sdf_part(parts: &[Part], i: usize, p: V3) -> f32 {
    let pt = &parts[i];
    // fig->world on the fly.
    let a = wp(pt.a[0], pt.a[1], pt.a[2]);
    match pt.shape {
        Shape::Sphere => len(sub(p, a)) - pt.r * 8.0,
        Shape::Capsule => {
            let b = wp(pt.b[0], pt.b[1], pt.b[2]);
            let pa = sub(p, a);
            let ba = sub(b, a);
            let h = (dot(pa, ba) / dot(ba, ba).max(1e-6)).clamp(0.0, 1.0);
            len(sub(pa, mul(ba, h))) - pt.r * 8.0
        }
        Shape::RoundBox => {
            let q0 = (p[0] - a[0]).abs() - pt.b[0] * 8.0;
            let q1 = (p[1] - a[1]).abs() - pt.b[1] * 8.0;
            let q2 = (p[2] - a[2]).abs() - pt.b[2] * 8.0;
            let outside = len([q0.max(0.0), q1.max(0.0), q2.max(0.0)]);
            let inside = q0.max(q1).max(q2).min(0.0);
            outside + inside - pt.r * 8.0
        }
    }
}

fn scene(parts: &[Part], p: V3) -> (f32, usize) {
    let mut d = f32::MAX;
    let mut id = 0;
    for (i, _) in parts.iter().enumerate() {
        let di = sdf_part(parts, i, p);
        if di < d {
            d = di;
            id = i;
        }
    }
    (d, id)
}

fn scene_normal(parts: &[Part], p: V3) -> V3 {
    let e = 0.6;
    let (x0, _) = scene(parts, [p[0] + e, p[1] - e, p[2] - e]);
    let (x1, _) = scene(parts, [p[0] - e, p[1] - e, p[2] + e]);
    let (x2, _) = scene(parts, [p[0] - e, p[1] + e, p[2] - e]);
    let (x3, _) = scene(parts, [p[0] + e, p[1] + e, p[2] + e]);
    norm(add(
        add(mul([e, -e, -e], x0), mul([-e, -e, e], x1)),
        add(mul([-e, e, -e], x2), mul([e, e, e], x3)),
    ))
}

/// 5-tap SDF ambient occlusion (directional along the normal).
fn scene_ao(parts: &[Part], p: V3, n: V3) -> f32 {
    let mut occ = 0.0;
    let mut w = 1.0;
    for k in 1..5 {
        let t = k as f32 * 2.2;
        let (d, _) = scene(parts, add(p, mul(n, t)));
        occ += (t - d) * w;
        w *= 0.7;
    }
    (1.0 - occ * 0.6).clamp(0.35, 1.0)
}

const SUN: V3 = [-0.35, 0.72, -0.45]; // toward the light: NW high, frontal
const FILL: V3 = [0.4, -0.3, -0.55]; // warm ground bounce off-frame

/// Penumbra soft shadow map (classic sphere-trace back to the sun).
fn scene_shadow(parts: &[Part], p: V3) -> f32 {
    let l = norm(SUN);
    let mut t = 2.0;
    let mut res = 1.0f32;
    for _ in 0..24 {
        let q = add(p, mul(l, t));
        let (h, _) = scene(parts, q);
        if h < 0.3 {
            return 0.0;
        }
        res = res.min((8.0 * h / t).min(1.0));
        t += (h * 0.6).clamp(0.8, 6.0);
    }
    res
}

/// Bake one rig description into the 64px cell (un-outlined; the caller
/// applies the chunky finish so painted and baked plates share the edge).
pub fn bake_actor_rig(parts: &[Part]) -> RgbaImage {
    let mut img = blank(S as u32, S as u32);
    let cam_z = -140.0f32;
    let dir = norm([0.0, 0.14, 1.0]); // slight downward gaze, sculpted reads
    for py in 0..S {
        for px in 0..S {
            let o = [px as f32 - 96.0, 96.0 - py as f32, cam_z];
            let mut t = 0.0f32;
            let mut hit = false;
            for _ in 0..56 {
                let p = add(o, mul(dir, t));
                let (d, _) = scene(parts, p);
                if d < 0.35 {
                    hit = true;
                    break;
                }
                t += (d * 0.85).max(0.4);
                if t > 260.0 {
                    break;
                }
            }
            if !hit {
                continue;
            }
            let hp = add(o, mul(dir, t));
            let n = scene_normal(parts, hp);
            let (_, part_id) = scene(parts, hp);
            let part = &parts[part_id];
            let ao = scene_ao(parts, hp, n);
            let key = dot(n, norm(SUN)).max(0.0);
            let sh = if key > 0.02 { scene_shadow(parts, hp) } else { 1.0 };
            let fill = dot(n, norm(FILL)).max(0.0);
            // Rim: silhouette glint toward the upper viewer rim.
            let view_n = [hp[0], hp[1] - 40.0, cam_z - hp[2]];
            let rim = (1.0 - dot(n, norm(view_n)).abs()).powi(2).clamp(0.0, 1.0);
            // v2 exposure: actor albedo families were crushing to mud at
            // v1's 0.34 ambient — grim still, but family hues must survive.
            let mut lum = 0.38 * ao
                + 0.78 * key * (0.45 + 0.55 * sh)
                + 0.22 * fill * (0.30 + 0.70 * ao)
                + 0.24 * rim * ao;
            lum = lum.max(part.emissive);
            let al = part.albedo;
            let c = shade(al, lum.min(1.35));
            img.put_pixel(px as u32, py as u32, image::Rgba([c[0], c[1], c[2], 255]));
        }
    }
    downsample3(&img)
}

fn downsample3(src: &RgbaImage) -> RgbaImage {
    let mut out = blank(64, 64);
    for y in 0..64i64 {
        for x in 0..64i64 {
            let (mut r, mut g, mut b, mut a) = (0u32, 0u32, 0u32, 0u32);
            for dy in 0..SS {
                for dx in 0..SS {
                    let p = src.get_pixel((x as i32 * SS + dx) as u32, (y as i32 * SS + dy) as u32).0;
                    r += p[0] as u32;
                    g += p[1] as u32;
                    b += p[2] as u32;
                    a += p[3] as u32;
                }
            }
            let n = (SS * SS) as u32;
            out.put_pixel(
                x as u32,
                y as u32,
                image::Rgba([(r / n) as u8, (g / n) as u8, (b / n) as u8, (a / n) as u8]),
            );
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Body rigs: shared masses on painted-figure coordinates.

const BONE: Rgb = [224, 218, 187];
const STEEL: Rgb = [158, 175, 185];
const LEATHER: Rgb = [145, 102, 67];
const SKIN: Rgb = [225, 179, 132];
const DARK: Rgb = [31, 35, 43];
const GOLD: Rgb = [232, 191, 89];
const WHITE: Rgb = [255, 255, 255];

fn cloth(accent: Rgb) -> Rgb {
    // v2: read the family accent (v1's 0.55/0.36 ladder mudded nearly every
    // civilian into the same brown-grey — owner flagged "albedos muddy").
    shade(accent, 0.66)
}
fn fold(accent: Rgb) -> Rgb {
    shade(accent, 0.40)
}

/// Mid-gear humanoid base v2 (matches painted proportions): torso waist
/// taper with a real belt line, blended shoulder->pauldron joints, and a
/// HEAD with actual volume — brow ridge, nose bump, eye sockets, role
/// hair mass. v1's bare capsule heads and brick bodies are the flagged
/// upstream read; every defect listed here maps to an owner complaint.
pub struct HumanoidOpts {
    pub accent: Rgb,
    pub robe: bool,
    pub broad: bool,
    #[allow(dead_code)]
    pub spectral: bool,
    /// None | "cap" | "hood" | "mitre" | "helm"
    pub headwear: &'static str,
}

pub fn humanoid(opts: &HumanoidOpts) -> Vec<Part> {
    let accent = opts.accent;
    let mut v: Vec<Part> = Vec::with_capacity(24);
    // Torso: chest 4.2 -> waist 3.4 (taper), z-forward belly.
    v.push(capsule(16.0, 17.5, 0.0, 16.0, 13.0, 0.3, 4.0, cloth(accent)));
    v.push(capsule(16.0, 21.0, 0.0, 16.0, 18.0, 0.2, 3.5, fold(accent)));
    // Belt line: dark band + gold buckle (the waist identity marker).
    v.push(rbox(16.0, 21.2, 0.4, [4.0, 1.1, 3.8], 0.3, LEATHER));
    v.push(rbox(16.0, 21.4, 4.0, [1.0, 0.8, 0.4], 0.2, GOLD));
    if opts.robe {
        // Gown: widening skirt instead of legs, tapering from the belt.
        v.push(capsule(16.0, 26.0, 0.0, 16.0, 21.0, 0.0, 5.0, cloth(accent)));
        v.push(capsule(16.0, 29.0, 0.0, 16.0, 25.0, 0.0, 5.8, fold(accent)));
        v.push(capsule(16.0, 24.0, 0.5, 16.0, 28.0, 0.5, 5.5, fold(accent)));
    } else {
        for (lx, lean) in [(13.0, -0.6), (19.0, 0.6)] {
            v.push(capsule(lx, 21.0, 0.0, lx + lean * 1.6, 26.0, 0.6, 1.5, fold(accent)));
            v.push(capsule(lx + lean * 1.6, 26.0, 0.6, lx + lean * 1.8, 29.0, 1.0, 1.3, LEATHER));
            v.push(sphere(lx + lean * 1.8, 29.5, 1.4, 1.6, DARK));
        }
    }
    // Arms: shoulder capsule starts INSIDE the pauldron (the v1 complaint:
    // pauldrons ate the arms — now pauldron sphere is smaller and the arm
    // enters from underneath it).
    for (sx, bend) in [(11.0, -1.0), (21.0, 1.0)] {
        v.push(capsule(sx, 12.2, 0.2, sx + bend * 2.0, 20.0, 1.0, 1.4, cloth(accent)));
        v.push(sphere(sx + bend * 2.0, 20.5, 1.2, 1.3, SKIN));
    }
    if opts.broad {
        // Warlord pauldrons: closer to the body, arms pass under them.
        v.push(sphere(9.8, 11.8, 0.5, 2.4, STEEL));
        v.push(sphere(22.2, 11.8, 0.5, 2.4, STEEL));
    }
    // HEAD with face language: skull sphere + jaw, brow ridge, nose bump,
    // eye sockets — face ques read at 64px instead of a bare capsule.
    v.push(sphere(16.0, 5.0, 1.0, 3.4, SKIN));
    v.push(capsule(16.0, 8.2, 1.4, 16.0, 6.2, 1.6, 2.2, SKIN)); // jaw/chin
    v.push(rbox(16.0, 4.4, 3.8, [2.4, 0.7, 0.7], 0.2, shade(SKIN, 0.82))); // brow ridge
    v.push(sphere(16.1, 6.0, 4.2, 0.75, shade(SKIN, 0.95))); // nose bump
    // v3: sockets must read at 64px — slightly larger, near-ink.
    v.push(sphere(14.9, 5.0, 4.3, 0.62, [5, 6, 8])); // eye socket
    v.push(sphere(17.1, 5.0, 4.3, 0.62, [5, 6, 8])); // eye socket
    v.push(rbox(16.0, 8.0, 3.6, [1.6, 0.5, 0.5], 0.15, shade(SKIN, 0.88))); // mouth line
    match opts.headwear {
        "hood" => {
            // Hood: mass behind/above the skull with open face cavity.
            v.push(sphere(16.0, 3.4, -0.6, 4.0, cloth(accent)));
            v.push(capsule(11.8, 4.0, 0.2, 11.8, 10.0, 0.0, 2.0, cloth(accent)));
            v.push(capsule(20.2, 4.0, 0.2, 20.2, 10.0, 0.0, 2.0, cloth(accent)));
        }
        "cap" => {
            v.push(sphere(16.0, 2.8, 0.2, 3.8, LEATHER));
            v.push(rbox(16.0, 4.4, 2.0, [4.2, 0.6, 2.8], 0.2, shade(LEATHER, 0.85))); // brim
        }
        "mitre" => {
            v.push(rbox(16.0, 1.6, 0.6, [3.2, 2.4, 2.8], 0.6, cloth(accent)));
            v.push(rbox(16.0, 0.4, 0.6, [2.4, 1.4, 2.4], 0.4, cloth(accent)));
            v.push(rbox(16.0, 3.2, 0.8, [3.8, 0.8, 3.0], 0.4, GOLD));
            v.push(rbox(16.0, 2.0, 4.0, [0.6, 2.6, 0.6], 0.15, [255, 255, 255])); // mitre slash
        }
        "helm" => {
            v.push(capsule(16.0, 2.0, 1.0, 16.0, 5.5, 1.0, 3.6, STEEL));
            v.push(capsule(16.0, 1.0, 1.4, 16.0, 4.0, 1.6, 0.8, BONE)); // ridge
            v.push(rbox(16.0, 5.4, 3.6, [2.6, 0.8, 0.6], 0.2, shade(STEEL, 0.7))); // brow band
        }
        _ => {
            // Role hair: mass over/behind skull, sides at the temples.
            v.push(rbox(16.0, 2.6, -0.6, [3.3, 2.0, 2.6], 1.0, DARK));
            v.push(capsule(13.4, 3.0, -0.4, 13.0, 7.0, -0.6, 1.4, DARK));
            v.push(capsule(18.6, 3.0, -0.4, 19.0, 7.0, -0.6, 1.4, DARK));
        }
    }
    v
}

/// Ghost face swap: hollow sockets + glow field (wisp palette).
pub fn ghost_face(v: &mut Vec<Part>, accent: Rgb) {
    // Review v2: glow-face proud of the hood cavity, sockets forward too.
    v.push(emit(sphere(16.0, 5.6, 3.4, 2.2, shade(accent, 0.55)), 0.95));
    v.push(sphere(14.8, 5.2, 5.3, 0.66, [6, 7, 9]));
    v.push(sphere(17.2, 5.2, 5.3, 0.66, [6, 7, 9]));
}

/// Quadruped base: horizontal barrel, four legs, neck up to head, muzzle,
/// ears. Returns parts; extras (antlers/crown/tails) on top.
pub struct QuadOpts {
    pub barrel_r: f32,
    pub low: f32,
    pub fur: Rgb,
    pub dark: Rgb,
}

pub fn quadruped(opts: &QuadOpts) -> Vec<Part> {
    let mut v: Vec<Part> = Vec::with_capacity(16);
    let fur = opts.fur;
    let low = opts.low;
    v.push(capsule(8.0, low, 0.0, 22.0, low + 0.6, 0.0, opts.barrel_r, fur));
    v.push(sphere(8.0, low - 0.4, 0.0, opts.barrel_r * 0.92, shade(fur, 0.85)));
    // Four legs (far pair darker).
    for (lx, far) in [(10.0, true), (13.5, false), (19.0, true), (23.0, false)] {
        let c = if far { shade(fur, 0.75) } else { fur };
        v.push(capsule(lx, low + opts.barrel_r * 0.4, 0.0, lx, 28.5, 0.4, 1.2, c));
        v.push(sphere(lx, 29.2, 0.8, 1.1, opts.dark));
    }
    // Neck + head + muzzle + ears.
    v.push(capsule(21.0, low - 1.0, 0.0, 24.5, low - 6.5, 0.6, 1.8, fur));
    v.push(capsule(23.5, low - 7.0, 0.6, 27.0, low - 5.5, 0.9, 1.5, fur));
    v.push(capsule(27.0, low - 5.5, 0.9, 29.5, low - 4.0, 1.2, 0.9, shade(fur, 0.9)));
    v.push(sphere(29.5, low - 4.0, 1.6, 0.9, DARK)); // nose
    v.push(sphere(23.0, low - 8.0, 0.4, 1.0, shade(fur, 0.8))); // ear far
    v.push(sphere(25.5, low - 8.2, 0.6, 1.0, fur)); // ear
    v.push(sphere(25.0, low - 5.2, 2.6, 0.6, DARK)); // eye
    // Tail: thick-ish rear capsule.
    v.push(capsule(8.5, low - 1.0, 0.0, 4.5, low + 4.0, -0.4, 1.0, shade(fur, 0.8)));
    v
}

// ---------------------------------------------------------------------------
// Rig builders per batch subject.

/// Player hero: mid plate + weapon + class cape (box slab behind the body).
pub fn rig_player(cape: Rgb) -> Vec<Part> {
    let mut v = humanoid(&HumanoidOpts {
        accent: [235, 200, 90],
        robe: false,
        broad: false,
        spectral: false,
        headwear: "none",
    });
    // Cape slab behind, class-hued (matches the painted cape mass).
    v.push(rbox(16.0, 17.0, -2.2, [4.2, 9.5, 1.0], 0.8, cape));
    v.push(capsule(13.0, 8.0, -1.4, 13.0, 25.0, -1.6, 2.0, shade(cape, 0.75)));
    // Chest plate + pauldron-lite + sword.
    v.push(rbox(16.0, 15.0, 2.4, [3.4, 4.0, 1.0], 0.6, STEEL));
    v.push(rbox(16.0, 15.0, 3.0, [1.0, 4.2, 0.5], 0.3, shade(STEEL, 1.2)));
    v.push(sphere(24.6, 20.0, 2.4, 1.2, GOLD)); // pommel
    v.push(capsule(25.5, 12.0, 2.4, 25.5, 21.0, 2.4, 0.7, STEEL)); // blade up
    v.push(capsule(24.2, 19.0, 2.4, 26.8, 19.0, 2.4, 0.45, GOLD)); // guard
    v
}

pub fn rig_gnawthane() -> Vec<Part> {
    let fur = [120, 100, 84];
    let mut v = quadruped(&QuadOpts { barrel_r: 4.6, low: 18.0, fur, dark: shade(fur, 0.5) });
    // Longer body radius reads dire; crown shards over the skull.
    v.push(capsule(21.5, 11.0, 0.6, 21.5, 8.0, 0.6, 0.7, BONE));
    v.push(capsule(24.0, 10.5, 0.6, 24.0, 6.8, 0.6, 0.7, BONE));
    v.push(capsule(26.5, 11.0, 0.6, 26.5, 8.4, 0.6, 0.7, BONE));
    v.push(sphere(25.0, 12.6, 2.6, 0.7, [200, 60, 50])); // red eye
    v.push(rbox(16.0, 15.0, 0.0, [7.0, 2.0, 2.0], 1.0, shade(fur, 0.75))); // mange mantle
    v
}

pub fn rig_pale_stag(at_bay: bool) -> Vec<Part> {
    let fur = [221, 216, 199];
    let lift = if at_bay { 2.6 } else { 0.0 };
    let mut v = quadruped(&QuadOpts { barrel_r: 3.6, low: 18.0 - lift, fur, dark: shade(fur, 0.5) });
    // Ghost antlers: twin forking beams.
    for ax in [22.0, 26.0] {
        v.push(capsule(ax, 9.5 - lift, 0.6, ax - 1.5, 4.5 - lift, 0.4, 0.7, BONE));
        v.push(capsule(ax - 0.8, 7.0 - lift, 0.5, ax - 2.0, 2.0 - lift, 0.3, 0.6, BONE));
        v.push(capsule(ax - 0.2, 5.5 - lift, 0.5, ax + 1.0, 1.5 - lift, 0.3, 0.6, BONE));
    }
    v.push(rbox(15.0, 12.5 - lift, 1.0, [7.0, 1.2, 2.4], 0.8, WHITE)); // mantle sparkle
    v
}

pub fn rig_tollmaster() -> Vec<Part> {
    let mut v = humanoid(&HumanoidOpts {
        accent: [190, 140, 60],
        robe: false,
        broad: true,
        spectral: false,
        headwear: "cap",
    });
    // Coin sash from right hip to left shoulder, hook polearm.
    v.push(capsule(11.0, 11.0, 3.4, 22.0, 19.0, 2.8, 1.0, GOLD));
    v.push(sphere(14.0, 12.2, 4.0, 0.6, WHITE));
    v.push(sphere(17.5, 15.0, 3.8, 0.6, WHITE));
    v.push(sphere(20.5, 17.0, 3.2, 0.6, WHITE));
    v.push(capsule(27.5, 4.0, 1.5, 26.5, 25.0, 1.5, 0.55, LEATHER));
    v.push(capsule(26.5, 5.0, 1.5, 30.0, 6.5, 1.5, 0.65, STEEL)); // hook head
    v.push(capsule(30.0, 6.5, 1.5, 29.5, 9.5, 1.5, 0.65, STEEL)); // hook return
    v
}

/// Ground-contact anchor (v3): every baked actor gets a soft sunk ellipse
/// under the feet so figures stop floating over tiles. Drawn AFTER chunky
/// and only where the plate is empty, so silhouettes stay clean.
pub fn anchor_shadow(img: &mut RgbaImage) {
    for py in 50..64i32 {
        for px in 12..52i32 {
            let dx = (px as f32 - 32.0) / 15.0;
            let dy = (py as f32 - 57.6) / 3.6;
            let d2 = dx * dx + dy * dy;
            if d2 < 1.0 {
                let p = img.get_pixel(px as u32, py as u32);
                if p[3] <= 64 {
                    let a = (105.0 * (1.0 - d2)) as u8;
                    blend_px(img, px, py, [0, 0, 0], a);
                }
            }
        }
    }
}

#[allow(dead_code)]
pub fn rig_wisp(dim: bool) -> Vec<Part> {
    let accent = if dim { [116, 168, 160] } else { [150, 226, 214] };
    let mut v = humanoid(&HumanoidOpts {
        accent,
        robe: true,
        broad: false,
        spectral: true,
        headwear: "hood",
    });
    ghost_face(&mut v, accent);
    // Drift motes.
    let motes: [(f32, f32, f32, Rgb); 4] = if dim {
        [
            (7.0, 10.0, 6.0, accent),
            (26.0, 8.0, 6.0, accent),
            (6.0, 24.0, 5.0, shade(accent, 0.7)),
            (25.0, 26.0, 5.0, shade(accent, 0.7)),
        ]
    } else {
        [
            (6.0, 9.0, 6.0, accent),
            (26.0, 6.0, 6.0, accent),
            (4.5, 22.0, 5.0, accent),
            (27.0, 24.0, 5.0, accent),
        ]
    };
    for (x, y, z, c) in motes {
        v.push(emit(sphere(x, y, z, 0.8, c), 1.0));
    }
    mottle_drape(&mut v, accent, dim);
    v
}

#[allow(dead_code)]
fn mottle_drape(v: &mut Vec<Part>, accent: Rgb, dim: bool) {
    let _ = (v, accent, dim);
}

pub fn rig_alchemist() -> Vec<Part> {
    let mut v = humanoid(&HumanoidOpts {
        accent: [170, 130, 190],
        robe: false,
        broad: false,
        spectral: false,
        headwear: "none",
    });
    // Stained apron + three reagent vials on the belt.
    v.push(rbox(16.0, 17.0, 3.0, [3.2, 5.0, 0.9], 0.5, LEATHER));
    v.push(emit(capsule(13.0, 13.4, 4.0, 13.0, 16.0, 4.0, 0.6, [120, 170, 110]), 0.5));
    v.push(emit(capsule(16.0, 13.4, 4.2, 16.0, 16.0, 4.2, 0.6, [116, 143, 166]), 0.5));
    v.push(emit(capsule(19.0, 13.4, 4.0, 19.0, 16.0, 4.0, 0.6, [180, 140, 90]), 0.5));
    v
}

pub fn rig_oathless_curate() -> Vec<Part> {
    let mut v = humanoid(&HumanoidOpts {
        accent: [120, 90, 150],
        robe: true,
        broad: false,
        spectral: false,
        headwear: "mitre",
    });
    // The page he never stops folding, held out front.
    v.push(rbox(16.0, 15.0, 3.6, [2.6, 3.4, 0.7], 0.3, BONE));
    v.push(rbox(16.0, 15.0, 4.2, [2.0, 0.5, 0.4], 0.2, DARK));
    v.push(rbox(16.0, 17.4, 4.2, [2.0, 0.5, 0.4], 0.2, DARK));
    v
}

/// The initial bake set: (plate key, rig). Painted keywords stay canonical.
// ---------------------------------------------------------------------------
// Batch-2 roster: civilians/military/bosses/quadrupeds on the shared rigs.

fn civilian(accent: Rgb, headwear: &'static str, pack: bool, sack: bool) -> Vec<Part> {
    let mut v = humanoid(&HumanoidOpts {
        accent,
        robe: false,
        broad: false,
        spectral: false,
        headwear,
    });
    if pack {
        v.push(rbox(16.0, 14.0, -2.4, [3.4, 4.2, 1.2], 0.6, LEATHER));
        v.push(rbox(16.0, 11.4, -2.4, [3.8, 0.8, 1.4], 0.3, fold(accent)));
    }
    if sack {
        v.push(rbox(14.0, 15.0, 3.2, [2.4, 3.0, 0.9], 0.5, shade(BONE, 0.75)));
        v.push(capsule(12.0, 12.0, 3.2, 12.0, 17.0, 3.2, 0.5, LEATHER));
    }
    v
}

fn rig_guard() -> Vec<Part> {
    let mut v = humanoid(&HumanoidOpts {
        accent: [135, 161, 240],
        robe: false,
        broad: false,
        spectral: false,
        headwear: "helm",
    });
    // Kite shield on the near arm, spear at rest cant (~45deg): pose
    // variance v3 so the guard reads AT-EASE, not mannequin.
    v.push(rbox(24.5, 17.0, 2.2, [1.8, 4.5, 0.7], 0.4, STEEL));
    v.push(rbox(24.5, 17.0, 2.9, [0.8, 3.4, 0.3], 0.3, shade(STEEL, 1.25)));
    v.push(capsule(22.5, 3.0, 1.8, 29.5, 28.0, 1.8, 0.55, LEATHER));
    v.push(rbox(21.5, 1.6, 1.8, [1.4, 2.4, 0.5], 0.4, STEEL));
    v.push(capsule(22.9, 0.2, 1.8, 21.5, 4.8, 1.8, 0.4, BONE));
    v
}

fn rig_skeleton() -> Vec<Part> {
    let mut v = humanoid(&HumanoidOpts {
        accent: [190, 190, 170],
        robe: false,
        broad: false,
        spectral: false,
        headwear: "none",
    });
    // Bone plates: swap cloth masses toward the ribs; angular skull (the
    // v1 marshmallow complaint — skull needs dome + jaw + big sockets).
    for part in v.iter_mut() {
        part.albedo = match part.shape {
            Shape::Sphere => BONE,
            _ => shade(BONE, 0.82),
        };
    }
    v.push(sphere(16.0, 3.4, 1.0, 3.2, BONE)); // skull dome
    v.push(rbox(16.0, 6.6, 2.8, [2.0, 1.2, 1.0], 0.3, shade(BONE, 0.9))); // jaw block
    v.push(rbox(16.0, 4.2, 4.2, [2.6, 0.8, 0.5], 0.2, shade(BONE, 0.75))); // brow shelf
    v.push(sphere(14.8, 5.0, 4.4, 0.9, DARK)); // socket
    v.push(sphere(17.2, 5.0, 4.4, 0.9, DARK)); // socket
    v.push(rbox(16.0, 13.0, 3.0, [3.0, 0.9, 0.6], 0.2, BONE)); // rib rung
    v.push(rbox(16.0, 15.4, 3.0, [3.0, 0.9, 0.6], 0.2, BONE));
    v.push(rbox(16.0, 17.8, 3.0, [3.0, 0.9, 0.6], 0.2, BONE));
    v
}

fn rig_chief() -> Vec<Part> {
    let mut v = humanoid(&HumanoidOpts {
        accent: [200, 90, 80],
        robe: false,
        broad: true,
        spectral: false,
        headwear: "helm",
    });
    // Horned helm v2: the v1 spikes were lost at 64px — doubled arcs, slab
    // tips, they must read as HORNS first, helm second.
    v.push(capsule(11.5, 3.5, 0.4, 7.0, -3.0, 0.4, 1.1, BONE));
    v.push(capsule(7.5, -2.0, 0.4, 3.5, -5.0, 0.4, 0.9, BONE));
    v.push(sphere(3.0, -5.5, 0.4, 1.2, BONE));
    v.push(capsule(20.5, 3.5, 0.4, 25.0, -3.0, 0.4, 1.1, BONE));
    v.push(capsule(24.5, -2.0, 0.4, 28.5, -5.0, 0.4, 0.9, BONE));
    v.push(sphere(29.0, -5.5, 0.4, 1.2, BONE));
    v.push(rbox(16.0, 4.6, 2.4, [3.4, 0.9, 0.6], 0.4, GOLD));
    v.push(capsule(26.0, 5.0, 1.5, 25.0, 26.0, 1.5, 0.7, LEATHER)); // great club
    v.push(rbox(25.5, 2.6, 1.5, [2.2, 2.6, 1.4], 0.8, shade(BONE, 0.6)));
    v
}

fn rig_lich() -> Vec<Part> {
    let mut v = humanoid(&HumanoidOpts {
        accent: [120, 90, 150],
        robe: true,
        broad: false,
        spectral: false,
        headwear: "none",
    });
    ghost_face(&mut v, [120, 90, 150]);
    // Pronged gold crown + sea-lit staff orb.
    v.push(rbox(16.0, 2.2, 1.2, [4.0, 1.0, 2.6], 0.4, GOLD));
    for x in [12.4, 16.0, 19.6] {
        v.push(capsule(x, 2.0, 1.2, x, -1.0, 1.2, 0.7, GOLD));
    }
    v.push(capsule(27.5, 6.0, 1.5, 26.5, 28.0, 1.5, 0.5, LEATHER));
    v.push(emit(sphere(26.8, 4.2, 2.0, 1.2, [102, 212, 196]), 1.0));
    v
}

fn rig_adjudicator() -> Vec<Part> {
    let mut v = humanoid(&HumanoidOpts {
        accent: [220, 183, 105],
        robe: true,
        broad: true,
        spectral: false,
        headwear: "none",
    });
    // The Judge's gold: plate every mass and crown the dome with the visor slit.
    for part in v.iter_mut() {
        part.albedo = shade(GOLD, 0.9);
    }
    v.push(rbox(16.0, 3.6, 2.4, [4.2, 2.6, 2.2], 0.6, shade(GOLD, 0.75)));
    v.push(rbox(16.0, 4.2, 4.4, [3.2, 0.8, 0.5], 0.3, DARK)); // visor slit
    v.push(rbox(16.0, 8.6, 3.4, [3.4, 1.2, 0.4], 0.3, GOLD));
    // Gavel polearm.
    v.push(capsule(27.5, 6.0, 1.5, 26.5, 29.0, 1.5, 0.6, LEATHER));
    v.push(rbox(27.0, 3.0, 1.5, [2.8, 1.8, 1.6], 0.5, GOLD));
    v
}

fn rig_oracle() -> Vec<Part> {
    let mut v = humanoid(&HumanoidOpts {
        accent: [102, 205, 195],
        robe: true,
        broad: false,
        spectral: false,
        headwear: "hood",
    });
    // Ring-topped staff + scroll bundle at the hip.
    v.push(capsule(27.5, 6.0, 1.5, 26.5, 28.0, 1.5, 0.5, LEATHER));
    v.push(rbox(27.0, 4.2, 1.5, [1.6, 1.6, 0.6], 0.8, GOLD));
    v.push(rbox(13.0, 20.0, 2.6, [2.2, 2.6, 1.0], 0.5, BONE));
    v.push(rbox(13.0, 20.0, 3.2, [2.6, 0.6, 0.4], 0.2, LEATHER));
    v
}

fn rig_companion() -> Vec<Part> {
    let mut v = humanoid(&HumanoidOpts {
        accent: [159, 216, 228],
        robe: false,
        broad: false,
        spectral: false,
        headwear: "none",
    });
    // Sellsword: cape slab + buckler + side sword.
    v.push(rbox(16.0, 16.5, -2.2, [3.6, 8.0, 1.0], 0.8, [159, 216, 228]));
    v.push(rbox(9.0, 17.5, 2.4, [1.6, 2.8, 0.6], 0.3, STEEL));
    v.push(capsule(24.6, 13.0, 2.0, 25.5, 22.0, 2.0, 0.6, STEEL));
    v
}

fn rig_tidemother() -> Vec<Part> {
    let mut v = humanoid(&HumanoidOpts {
        accent: [70, 130, 140],
        robe: true,
        broad: false,
        spectral: false,
        headwear: "hood",
    });
    ghost_face(&mut v, [70, 130, 140]);
    // Kelp drapes off the shoulders, barnacle coronet.
    let kelp = [93, 122, 89];
    v.push(capsule(9.0, 10.0, 1.0, 7.0, 16.0, 1.0, 1.6, kelp));
    v.push(capsule(23.0, 10.0, 1.0, 25.0, 16.0, 1.0, 1.6, kelp));
    v.push(sphere(13.4, 0.8, 1.4, 0.8, WHITE));
    v.push(sphere(16.0, 0.2, 1.4, 0.8, WHITE));
    v.push(sphere(18.6, 0.8, 1.4, 0.8, WHITE));
    v
}

fn rig_quad_small(fur: Rgb) -> Vec<Part> {
    let mut v = quadruped(&QuadOpts { barrel_r: 2.6, low: 21.0, fur, dark: shade(fur, 0.5) });
    // Ratty thin tail, longer than the body.
    v.push(capsule(8.0, 21.0, 0.0, 2.0, 27.0, -0.6, 0.5, shade(fur, 0.7)));
    v
}

fn rig_wolf() -> Vec<Part> {
    quadruped(&QuadOpts { barrel_r: 3.6, low: 19.0, fur: [142, 156, 165], dark: shade([142, 156, 165], 0.5) })
}

fn rig_bear() -> Vec<Part> {
    quadruped(&QuadOpts { barrel_r: 5.2, low: 17.0, fur: [119, 78, 49], dark: shade([119, 78, 49], 0.5) })
}

fn rig_matriarch() -> Vec<Part> {
    let mut v = rig_quad_small([137, 83, 108]);
    // Brood-spines v2: tall and crowned (v1's were spikes lost in the fur).
    for (i, x) in [9.0, 12.0, 15.0, 18.0].into_iter().enumerate() {
        v.push(capsule(x, 21.0 - (i % 2) as f32, 0.2, x, 14.5 - (i % 2) as f32, 0.2, 0.9, BONE));
    }
    v.push(rbox(22.0, 16.0, 2.0, [3.5, 2.5, 1.2], 0.9, shade([137, 83, 108], 0.7)));
    v
}

/// The initial bake set: (plate key, rig). Painted keywords stay canonical.
pub fn batch() -> Vec<(&'static str, Vec<Part>)> {
    let mut out: Vec<(&'static str, Vec<Part>)> = Vec::new();
    for (key, cap) in [
        ("Player.Keepwarden", [232, 181, 88]),
        ("Player.Gravebound", [233, 228, 210]),
        ("Player.Redwake", [216, 110, 70]),
        ("Player.Waysworn", [170, 160, 90]),
        ("Player.SigilSworn", [152, 225, 222]),
        ("Player.Fensworn", [170, 105, 186]),
    ] {
        out.push((key, rig_player(cap)));
    }
    out.push(("GnawThane", rig_gnawthane()));
    out.push(("PaleStag", rig_pale_stag(false)));
    out.push(("PaleStag.at_bay", rig_pale_stag(true)));
    out.push(("Tollmaster", rig_tollmaster()));
    out.push(("Alchemist", rig_alchemist()));
    out.push(("OathlessCurate", rig_oathless_curate()));
    // Batch 2: roster-wide geometry subjects (Main's promotion list).
    out.push(("Commoner", {
        let mut v = civilian([190, 190, 170], "cap", false, true);
        // Home-spun tunic over the torso + a yard tool handle.
        v.push(rbox(16.0, 15.0, 3.0, [3.0, 4.5, 0.8], 0.4, [168, 152, 119]));
        v.push(capsule(26.0, 12.0, 1.8, 25.0, 26.0, 1.8, 0.5, LEATHER));
        v
    }));
    out.push(("Vendor", {
        let mut v = civilian([200, 150, 70], "cap", false, true);
        // Coin apron + gold brow band (the painted Vendor's signatures).
        v.push(rbox(16.0, 15.0, 3.0, [3.2, 4.5, 0.8], 0.4, shade(BONE, 0.9)));
        v.push(rbox(16.0, 3.4, 4.4, [3.0, 0.8, 0.4], 0.2, GOLD));
        v
    }));
    out.push(("Thief", {
        let mut v = civilian([120, 120, 100], "hood", false, false);
        v.push(capsule(24.8, 18.0, 2.0, 25.6, 21.5, 2.0, 0.4, STEEL)); // dirk
        v
    }));
    out.push(("Traveller", {
        let mut v = civilian([170, 160, 90], "cap", false, false);
        // Bedroll lashed to the back + two diagonal straps + walking staff.
        v.push(capsule(11.0, 10.0, -2.6, 21.0, 10.0, -2.6, 2.0, shade(BONE, 0.85)));
        v.push(capsule(12.0, 12.0, -1.6, 20.0, 20.0, 1.2, 0.5, shade(LEATHER, 0.6)));
        v.push(capsule(20.0, 12.0, -1.6, 12.0, 20.0, 1.2, 0.5, shade(LEATHER, 0.6)));
        v.push(capsule(27.0, 8.0, 1.5, 25.5, 29.0, 1.5, 0.55, LEATHER));
        v
    }));
    out.push(("Bandit", {
        let mut v = civilian([200, 90, 80], "hood", false, false);
        // Review v2: the blade carries the bandit read (v1 was generic hood).
        v.push(capsule(24.6, 13.0, 2.0, 25.6, 21.0, 2.0, 0.6, STEEL));
        v.push(capsule(24.0, 20.0, 2.0, 26.0, 19.4, 2.0, 0.4, BONE));
        // Red kerchief at the throat (the painted bandit's accent cue).
        v.push(rbox(16.0, 9.5, 3.6, [2.6, 0.9, 0.5], 0.2, shade([200, 90, 80], 1.25)));
        v
    }));
    out.push(("Smuggler", {
        let mut v = civilian([160, 140, 110], "hood", true, true);
        // Hip pouch on a shoulder strap (painted smuggler's parcel cue).
        v.push(capsule(10.0, 12.0, 4.0, 20.0, 20.0, 3.0, 0.5, shade(LEATHER, 0.6)));
        v.push(rbox(20.5, 20.0, 3.4, [2.4, 2.6, 0.9], 0.5, LEATHER));
        v
    }));
    out.push(("Guard", rig_guard()));
    out.push(("Skeleton", rig_skeleton()));
    out.push(("Chief", rig_chief()));
    out.push(("Lich", rig_lich()));
    out.push(("Adjudicator", rig_adjudicator()));
    out.push(("Oracle", rig_oracle()));
    out.push(("Companion", rig_companion()));
    out.push(("Tidemother", rig_tidemother()));
    out.push(("Rat", rig_quad_small([148, 128, 112])));
    out.push(("Wolf", rig_wolf()));
    out.push(("Bear", rig_bear()));
    out.push(("Matriarch", rig_matriarch()));
    out
}

/// Keys deliberately NOT baked: glow-body spectral actors stay
/// emissive-painted until the rig grows a bloom pass (Main's call after
/// viewing the 3d contact batch 1: baked wisps read as squat robots).
/// cmd_bake_actors paints these cells from the painterly code every run,
/// so reverts survive any order of `actors` / `bake --actors`.
pub const PAINTED_ONLY: [&str; 2] = ["Mirelight", "FalseGlow"];

/// Spectral fade: applied post-bake so wisp tails taper into transparency
/// (painted plates cheat the same trick with alpha rows).
pub fn wisp_fade(img: &mut RgbaImage) {
    for y in 44..64 {
        let t = (y - 44) as f32 / 20.0;
        for x in 0..64 {
            let p = img.get_pixel_mut(x, y as u32);
            if p[3] > 0 {
                let keep = (1.0 - t * t) * 255.0;
                p[3] = ((p[3] as f32).min(keep)) as u8;
            }
        }
    }
}

/// Which baked keys are wisps needing the tail fade.
pub fn is_wisp(key: &str) -> bool {
    matches!(key, "Mirelight" | "FalseGlow")
}
