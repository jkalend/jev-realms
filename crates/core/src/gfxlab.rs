//! PROTOTYPE - design-evidence renderer for docs/D2_EVOLUTION.md.
//!
//! Renders one staged map in the four dimensionality candidates (square
//! top-down, isometric 2:1, compressed-tilt 2.5D, extruded pseudo-3D) plus
//! class/area/boss concept cards, exporting PNG sheets to docs/gfx/proto/.
//! This module is tooling, not a game code path; see examples/gfxlab.rs.

use crate::{
    model::{Archetype, Game, Map, MapKind, Pos, Tile},
    sprites::{self, actor, ActorAnimations, Canvas, Figure, Pose},
    world,
};
use macroquad::prelude::*;

pub const SHEET_W: f32 = 960.0;
pub const SHEET_H: f32 = 640.0;

// Layout: 20x13. Mountains/forest top, road across row 7, river at cols 13-15
// (ford on the road), ruins lower right, walled shrine room lower left.
const LAYOUT: [&str; 13] = [
    "mmmmmffFF,,,,,~~mmmm",
    "mmmf fFFf ,,,,~~~,mm",
    "mffffFf,,,,,,~~~,,,m",
    ",ffFFf,,,,,,,~~,,,,,",
    ",,ff,,,,,,,,~~~,,,,,",
    ",,,,,,,,,,,~~~,,,,,,",
    ",,,,,,,,,,~~~,,,,,,,",
    "rrrrrrrrrrr==rrrrrrr",
    ",,,,,,,,,,,~~,,RRR,,",
    "##D###,,,,,~~,,RRRR,",
    "#,C S#,,,,~~~,,,RR,,",
    "#,   #,,,,~~,,,,R,,,",
    "######,,,,~~,,,,,,,,",
];

fn layout_tile(c: char) -> Tile {
    match c {
        'm' => Tile::Mountain,
        'f' => Tile::Forest,
        'F' => Tile::DeepForest,
        ',' => Tile::Grass,
        'r' => Tile::Road,
        '~' => Tile::River,
        '=' => Tile::Ford,
        'R' => Tile::Ruins,
        '#' | 'D' => Tile::Wall,
        ' ' | 'C' | 'S' => Tile::Floor,
        _ => Tile::Grass,
    }
}

/// Staged world for rendering: synthetic map at index 0 with a cast of NPCs.
fn scene_game(seed: u64) -> Game {
    let mut game = Game::new(seed);
    let mut map = Map::new("Prototype vale", MapKind::Overworld, 20, 13, Tile::Grass);
    for (y, row) in LAYOUT.iter().enumerate() {
        for (x, ch) in row.char_indices() {
            map.tiles[y * 20 + x] = layout_tile(ch);
        }
    }
    map.tiles[9 * 20 + 2] = Tile::Door;
    map.tiles[10 * 20 + 2] = Tile::Chest;
    map.tiles[10 * 20 + 4] = Tile::Shrine;
    map.explored.fill(true);
    game.maps[0] = map;
    game.player.map = 0;
    game.player.pos = Pos::new(10, 7);
    let cast = [
        (Archetype::Guard, 6, 7, 3),
        (Archetype::Vendor, 1, 10, 1),
        (Archetype::Wolf, 7, 2, 2),
        (Archetype::Bandit, 11, 6, 2),
        (Archetype::Skeleton, 18, 8, 3),
    ];
    for (i, (archetype, x, y, level)) in cast.into_iter().enumerate() {
        game.npcs.push(world::make_npc(
            500 + i,
            archetype,
            0,
            Pos::new(x, y),
            level,
            900,
            seed,
        ));
    }
    game
}

fn tile_at(game: &Game, x: i32, y: i32) -> Tile {
    let map = &game.maps[0];
    if x < 0 || y < 0 || x >= map.width || y >= map.height {
        return Tile::Rock;
    }
    map.tiles[(y * map.width + x) as usize]
}

fn diamond(cx: f32, cy: f32, hw: f32, hh: f32, color: Color) {
    draw_triangle(
        Vec2::new(cx, cy - hh),
        Vec2::new(cx - hw, cy),
        Vec2::new(cx + hw, cy),
        color,
    );
    draw_triangle(
        Vec2::new(cx + hw, cy),
        Vec2::new(cx - hw, cy),
        Vec2::new(cx, cy + hh),
        color,
    );
}

fn shade(color: Color, k: f32) -> Color {
    Color::new(
        (color.r * k).min(1.0),
        (color.g * k).min(1.0),
        (color.b * k).min(1.0),
        color.a,
    )
}

/// (a) Today's renderer, untouched: square top-down via sprites::draw_map.
async fn variant_square(game: &Game, path: &str) {
    clear_background(Color::from_rgba(10, 13, 18, 255));
    let mut animations = ActorAnimations::default();
    sprites::draw_map(
        game,
        Rect::new(80.0, 45.0, 800.0, 520.0),
        &mut animations,
        40.0,
    );
    let c = Canvas(Rect::new(0.0, 0.0, SHEET_W, SHEET_H));
    c.text("A - square top-down (current)", 12.0, 18.0, 28, WHITE);
    c.text("real renderer, zero new work", 12.0, 44.0, 18, GRAY);
    capture(path);
    next_frame().await;
}

/// (b) Isometric 2:1 diamonds with hard-top extrusion and billboarded actors.
async fn variant_iso(game: &Game, path: &str) {
    clear_background(Color::from_rgba(16, 18, 26, 255));
    let c = Canvas(Rect::new(0.0, 0.0, SHEET_W, SHEET_H));
    let (hw, hh, ox, oy) = (26.0f32, 13.0f32, 400.0f32, 90.0f32);
    let tall = |t: Tile| matches!(t, Tile::Wall | Tile::Forest | Tile::DeepForest | Tile::Mountain);
    for pass in 0..2 {
        for y in 0..13 {
            for x in 0..20 {
                let tile = tile_at(game, x, y);
                let cx = ox + (x - y) as f32 * hw;
                let cy = oy + (x + y) as f32 * hh;
                let base = sprites::terrain_color(tile);
                if pass == 0 {
                    // Slight overlap kills triangle-coverage hairlines between diamonds.
                    diamond(cx, cy, hw + 0.8, hh + 0.4, base);
                    match tile {
                        Tile::River | Tile::Ford => {
                            diamond(cx, cy, hw * 0.62, hh * 0.62, shade(base, 1.35))
                        }
                        Tile::Chest => {}
                        _ => {}
                    }
                } else if tall(tile) {
                    let h = if tile == Tile::Wall { hh * 1.6 } else { hh * 2.2 };
                    // Anchored side faces: full quads from top diamond down to ground.
                    let quad = |pts: [Vec2; 4], col: Color| {
                        draw_triangle(pts[0], pts[1], pts[2], col);
                        draw_triangle(pts[2], pts[3], pts[0], col);
                    };
                    quad(
                        [
                            Vec2::new(cx - hw, cy - h),
                            Vec2::new(cx, cy + hh - h),
                            Vec2::new(cx, cy + hh),
                            Vec2::new(cx - hw, cy),
                        ],
                        shade(base, 0.62),
                    );
                    quad(
                        [
                            Vec2::new(cx, cy + hh - h),
                            Vec2::new(cx + hw, cy - h),
                            Vec2::new(cx + hw, cy),
                            Vec2::new(cx, cy + hh),
                        ],
                        shade(base, 0.82),
                    );
                    // Lighter top face last.
                    diamond(cx, cy - h, hw + 0.8, hh + 0.4, shade(base, 1.18));
                } else if matches!(tile, Tile::Door | Tile::Chest | Tile::Shrine) {
                    let col = sprites::terrain_color(tile);
                    c.rect(Rect::new(cx - 5.0, cy - 14.0, 10.0, 14.0), shade(col, 1.2));
                    c.border(Rect::new(cx - 5.0, cy - 14.0, 10.0, 14.0), 1.0, shade(col, 0.6));
                }
            }
        }
    }
    // Actors billboard over the ground, iso-projected at tile centers.
    for npc in game.npcs.iter().filter(|n| n.map == 0 && n.alive()) {
        let cx = ox + (npc.pos.x - npc.pos.y) as f32 * hw;
        let cy = oy + (npc.pos.x + npc.pos.y) as f32 * hh;
        actor(
            &c,
            Rect::new(cx - 16.0, cy - 34.0, 32.0, 34.0),
            accent(npc.archetype),
            Figure::Npc(npc.archetype, npc.phase),
            Pose::default(),
        );
    }
    let px = ox + (game.player.pos.x - game.player.pos.y) as f32 * hw;
    let py = oy + (game.player.pos.x + game.player.pos.y) as f32 * hh;
    actor(
        &c,
        Rect::new(px - 17.0, py - 36.0, 34.0, 36.0),
        Color::from_rgba(235, 200, 90, 255),
        Figure::Player { weapon: 2, armour: 2, relic: None },
        Pose::default(),
    );
    c.text("B - isometric 2:1 (the D2 read)", 12.0, 18.0, 28, WHITE);
    c.text("real depth cue; needs bespoke painter + pick remap", 12.0, 44.0, 18, GRAY);
    capture(path);
    next_frame().await;
}

/// (c) Compressed-tilt "2.5D": square tiles sheared, depth-cue dimming.
async fn variant_tilt(game: &Game, path: &str) {
    clear_background(Color::from_rgba(10, 13, 18, 255));
    let c = Canvas(Rect::new(0.0, 0.0, SHEET_W, SHEET_H));
    let (s, squash, shear, ox, oy) = (46.0f32, 0.58f32, 0.34f32, 250.0f32, 80.0f32);
    let project = |x: i32, y: i32| {
        let py = oy + y as f32 * s * squash;
        let px = ox + x as f32 * s - y as f32 * shear * s * 0.5;
        (px, py)
    };
    for y in 0..13 {
        for x in 0..20 {
            let tile = tile_at(game, x, y);
            let (px, py) = project(x, y);
            let depth = 1.0 - (12 - y) as f32 * 0.045; // farther rows dim
            let col = shade(sprites::terrain_color(tile), depth.max(0.45));
            draw_rectangle(px, py, s + 0.5, s * squash + 0.5, col);
            if matches!(tile, Tile::Wall | Tile::Mountain | Tile::Forest | Tile::DeepForest) {
                let h = s * 0.5;
                draw_rectangle(px, py - h, s + 0.5, h, shade(col, 1.12));
                draw_rectangle(px, py - 2.0, s + 0.5, 2.5, shade(col, 1.35));
            }
        }
    }
    for npc in game.npcs.iter().filter(|n| n.map == 0 && n.alive()) {
        let (px, py) = project(npc.pos.x, npc.pos.y);
        actor(
            &c,
            Rect::new(px + 6.0, py - s * 0.62, s * 0.8, s * 0.86),
            accent(npc.archetype),
            Figure::Npc(npc.archetype, npc.phase),
            Pose::default(),
        );
    }
    let (px, py) = project(game.player.pos.x, game.player.pos.y);
    actor(
        &c,
        Rect::new(px + 5.0, py - s * 0.66, s * 0.84, s * 0.9),
        Color::from_rgba(235, 200, 90, 255),
        Figure::Player { weapon: 2, armour: 2, relic: None },
        Pose::default(),
    );
    c.text("C - compressed-tilt 2.5D", 12.0, 18.0, 28, WHITE);
    c.text("keeps square logic readable; depth is mostly dimming", 12.0, 44.0, 18, GRAY);
    capture(path);
    next_frame().await;
}

/// (d) Extruded pseudo-3D diorama: flat ground, tall blocks with top faces.
async fn variant_extruded(game: &Game, path: &str) {
    clear_background(Color::from_rgba(12, 14, 20, 255));
    let c = Canvas(Rect::new(0.0, 0.0, SHEET_W, SHEET_H));
    let (s, ox, oy, lift) = (44.0f32, 80.0f32, 110.0f32, 20.0f32);
    let tall = |t: Tile| matches!(t, Tile::Wall | Tile::Mountain | Tile::Forest | Tile::DeepForest | Tile::Rock);
    // Ground pass first so extrusions can cover neighbour seams.
    for y in 0..13 {
        for x in 0..20 {
            let tile = tile_at(game, x, y);
            if tall(tile) {
                continue;
            }
            // Lift the ground plane so blocks read against it (diorama, not void).
            draw_rectangle(
                ox + x as f32 * s,
                oy + y as f32 * s,
                s + 0.5,
                s + 0.5,
                shade(sprites::terrain_color(tile), 1.35),
            );
        }
    }
    for y in 0..13 {
        for x in 0..20 {
            let tile = tile_at(game, x, y);
            if !tall(tile) {
                continue;
            }
            let base = sprites::terrain_color(tile);
            let px = ox + x as f32 * s;
            let py = oy + y as f32 * s;
            draw_rectangle(px, py, s + 0.5, s + 0.5, shade(base, 0.78)); // front face
            draw_rectangle(px + 3.0, py - lift, s + 0.5, lift + 8.0, base); // slab body
            draw_rectangle(px + 4.0, py - lift - 2.0, s - 1.5, 6.0, shade(base, 1.28)); // top face
            if matches!(tile, Tile::Forest | Tile::DeepForest) {
                draw_circle(px + s * 0.5, py - lift - 6.0, s * 0.30, shade(base, 1.22));
                draw_circle(px + s * 0.3, py - lift - 2.0, s * 0.20, shade(base, 1.1));
            }
        }
    }
    for npc in game.npcs.iter().filter(|n| n.map == 0 && n.alive()) {
        actor(
            &c,
            Rect::new(
                ox + npc.pos.x as f32 * s + 4.0,
                oy + npc.pos.y as f32 * s - 8.0,
                s * 0.86,
                s * 0.96,
            ),
            accent(npc.archetype),
            Figure::Npc(npc.archetype, npc.phase),
            Pose::default(),
        );
    }
    actor(
        &c,
        Rect::new(
            ox + game.player.pos.x as f32 * s + 3.0,
            oy + game.player.pos.y as f32 * s - 9.0,
            s * 0.9,
            s,
        ),
        Color::from_rgba(235, 200, 90, 255),
        Figure::Player { weapon: 2, armour: 2, relic: None },
        Pose::default(),
    );
    c.text("D - extruded pseudo-3D diorama", 12.0, 18.0, 28, WHITE);
    c.text("strongest depth illusion on square logic; z-order gets fussy at walls", 12.0, 44.0, 18, GRAY);
    capture(path);
    next_frame().await;
}

fn accent(archetype: Archetype) -> Color {
    match archetype {
        Archetype::Guard => Color::from_rgba(135, 161, 240, 255),
        Archetype::Wolf | Archetype::Bandit | Archetype::Skeleton => {
            Color::from_rgba(200, 90, 80, 255)
        }
        _ => Color::from_rgba(190, 190, 170, 255),
    }
}

fn capture(path: &str) {
    let image = get_screen_data();
    image.export_png(path);
    println!("GFXLAB OK {path} ({}x{})", image.width, image.height);
}

/// Renders all four dimensionality variants into docs/gfx/proto/.
pub async fn projection_sheets(dir: &str) {
    let game = scene_game(42);
    std::fs::create_dir_all(dir).unwrap_or_default();
    variant_square(&game, &format!("{dir}/proj-a-square.png")).await;
    variant_iso(&game, &format!("{dir}/proj-b-iso.png")).await;
    variant_tilt(&game, &format!("{dir}/proj-c-tilt.png")).await;
    variant_extruded(&game, &format!("{dir}/proj-d-extruded.png")).await;
}

// ---------------------------------------------------------------------------
// Concept cards - classes, areas, bosses (docs/D2_EVOLUTION.md v0.5)

const PARCH: Color = Color::from_rgba(26, 24, 21, 255);
const LINE: Color = Color::from_rgba(180, 150, 90, 255);
const DIM: Color = Color::from_rgba(150, 145, 132, 255);

struct ClassSpec {
    name: &'static str,
    order: &'static str,
    hue: Color,
    weapon: u8,
    armour: u8,
    bias: &'static str,
    signature: &'static str,
    meter: &'static str,
    extra: Option<Archetype>,
}

const CLASSES: [ClassSpec; 6] = [
    ClassSpec {
        name: "KEEPWARDEN",
        order: "the garrison that would not leave",
        hue: Color::from_rgba(232, 181, 88, 255),
        weapon: 3,
        armour: 3,
        bias: "+4 HP / -2 mana",
        signature: "Bulwark oaths - stances on Defend",
        meter: "Resolve, amber gold",
        extra: None,
    },
    ClassSpec {
        name: "SIGIL-SWORN",
        order: "those who read the script wrong on purpose",
        hue: Color::from_rgba(152, 225, 222, 255),
        weapon: 1,
        armour: 0,
        bias: "Spark + 4 mana / -3 HP, -1 ATK",
        signature: "Channel - consecutive casts charge",
        meter: "Channel, ice cyan",
        extra: None,
    },
    ClassSpec {
        name: "FENSWORN",
        order: "the pack knows your name",
        hue: Color::from_rgba(170, 105, 186, 255),
        weapon: 0,
        armour: 1,
        bias: "cheap loyal blade / gear -1 tier",
        signature: "Bond - companion fights as kin",
        meter: "Bond, viridian violet",
        extra: Some(Archetype::Wolf),
    },
    ClassSpec {
        name: "REDWAKE",
        order: "the tide collects",
        hue: Color::from_rgba(216, 110, 70, 255),
        weapon: 2,
        armour: 1,
        bias: "+2 SPD, +1 stamina / -1 DEF",
        signature: "Momentum - kill and flee feed the strike",
        meter: "Momentum, rust orange",
        extra: None,
    },
    ClassSpec {
        name: "GRAVEBOUND",
        order: "they kneel so the dead stay down",
        hue: Color::from_rgba(233, 228, 210, 255),
        weapon: 0,
        armour: 2,
        bias: "+2 HP / -1 SPD",
        signature: "Last Vigil - wounded, therefore dangerous",
        meter: "Vigil, pale ivory",
        extra: None,
    },
    ClassSpec {
        name: "WAYSWORN",
        order: "the road pays its own",
        hue: Color::from_rgba(170, 160, 90, 255),
        weapon: 1,
        armour: 0,
        bias: "+1 SPD, +2 stamina / -2 HP",
        signature: "Open Road - kills free the feet",
        meter: "Trail, olive tan",
        extra: None,
    },
];

fn panel_frame(c: &Canvas) {
    c.rect(Rect::new(0.0, 0.0, 320.0, 460.0), PARCH);
    c.border(Rect::new(2.0, 2.0, 316.0, 456.0), 2.0, shade(LINE, 0.8));
}

fn glow(c: &Canvas, x: f32, y: f32, r: f32, hue: Color) {
    let mut col = hue;
    col.a = 0.10;
    c.rect(Rect::new(x - r, y - r, r * 2.0, r * 2.0), col);
    col.a = 0.16;
    c.rect(Rect::new(x - r * 0.6, y - r * 0.6, r * 1.2, r * 1.2), col);
}

async fn class_card(spec: &ClassSpec, path: &str) {
    clear_background(Color::from_rgba(6, 7, 9, 255));
    let c = Canvas(Rect::new(0.0, 0.0, 320.0, 460.0));
    panel_frame(&c);
    c.text(spec.name, 16.0, 16.0, 21, LINE);
    c.text(spec.order, 16.0, 40.0, 14, DIM);
    glow(&c, 160.0, 190.0, 100.0, spec.hue);
    actor(
        &c,
        Rect::new(104.0, 90.0, 112.0, 170.0),
        spec.hue,
        Figure::Player {
            weapon: spec.weapon,
            armour: spec.armour,
            relic: None,
        },
        Pose::default(),
    );
    if let Some(extra) = spec.extra {
        actor(
            &c,
            Rect::new(200.0, 190.0, 48.0, 60.0),
            spec.hue,
            Figure::Npc(extra, 0),
            Pose::default(),
        );
    }
    c.rect(Rect::new(16.0, 300.0, 288.0, enumerate_bar(spec.hue)), shade(spec.hue, 0.9));
    c.text("BIAS", 16.0, 318.0, 13, DIM);
    c.text(spec.bias, 96.0, 318.0, 13, WHITE);
    c.text("SIGNATURE", 16.0, 342.0, 13, DIM);
    c.text(spec.signature, 96.0, 342.0, 13, WHITE);
    c.text("METER", 16.0, 366.0, 13, DIM);
    c.text(spec.meter, 96.0, 366.0, 13, WHITE);
    let image = get_screen_data();
    image.export_png(path);
    println!("GFXLAB OK {path}");
    next_frame().await;
}

fn enumerate_bar(hue: Color) -> f32 {
    let _ = hue;
    3.0
}

struct BossSpec {
    name: &'static str,
    epithet: &'static str,
    hue: Color,
    figure: BossFigure,
    signature: &'static str,
    act: &'static str,
    hp: &'static str,
}

enum BossFigure {
    Npc(Archetype),
    Tidemother,
    Stag,
    Wisp,
}

const BOSSES: [BossSpec; 11] = [
    BossSpec {
        name: "RED JACK, THE CHIEF",
        epithet: "the warren's morale",
        hue: Color::from_rgba(200, 90, 80, 255),
        figure: BossFigure::Npc(Archetype::Chief),
        signature: "Rally and sacrifice",
        act: "I · Burrow",
        hp: "64 HP",
    },
    BossSpec {
        name: "GNAW-THANE",
        epithet: "the rat-king below (side)",
        hue: Color::from_rgba(160, 140, 110, 255),
        figure: BossFigure::Npc(Archetype::Rat),
        signature: "Plague Tide - biting waves",
        act: "I · Fen barrow",
        hp: "40 HP",
    },
    BossSpec {
        name: "ASHFANG, THE MATRIARCH",
        epithet: "the pack remembers",
        hue: Color::from_rgba(150, 110, 80, 255),
        figure: BossFigure::Npc(Archetype::Matriarch),
        signature: "Pack tactics + howl summons",
        act: "II · Crimson Hollow",
        hp: "86 HP",
    },
    BossSpec {
        name: "TOLLMASTER GRUDGE",
        epithet: "the ford remembers every coin (side)",
        hue: Color::from_rgba(180, 120, 70, 255),
        figure: BossFigure::Npc(Archetype::Bandit),
        signature: "Bridge Tax - shoves and caltrops",
        act: "II · Ford camp",
        hp: "70 HP",
    },
    BossSpec {
        name: "CRAGMOTHER",
        epithet: "the mountain awake",
        hue: Color::from_rgba(140, 100, 70, 255),
        figure: BossFigure::Npc(Archetype::Bear),
        signature: "Avalanche Slam - 3x3 wind-up",
        act: "III · Ridge den",
        hp: "112 HP",
    },
    BossSpec {
        name: "MIRELIGHT",
        epithet: "three false lights (side)",
        hue: Color::from_rgba(120, 220, 200, 255),
        figure: BossFigure::Wisp,
        signature: "Strike the tell, not the glow",
        act: "III · Deep fen, night",
        hp: "78 HP",
    },
    BossSpec {
        name: "THE PALE STAG",
        epithet: "the hunt that hunts you",
        hue: Color::from_rgba(220, 220, 210, 255),
        figure: BossFigure::Stag,
        signature: "Break - it flees and mends",
        act: "III · Bounty escalation",
        hp: "124 HP",
    },
    BossSpec {
        name: "THE TIDEMOTHER",
        epithet: "the drowned matriarch",
        hue: Color::from_rgba(70, 130, 140, 255),
        figure: BossFigure::Tidemother,
        signature: "Undertow - fourth-round pull",
        act: "IV · Saltmarsh docks",
        hp: "96 HP",
    },
    BossSpec {
        name: "VAEL, THE LAST CASTELLAN",
        epithet: "the post is held",
        hue: Color::from_rgba(140, 160, 200, 255),
        figure: BossFigure::Npc(Archetype::Lich),
        signature: "Pressure, summons, curse, retreat",
        act: "V · Underkeep",
        hp: "108 HP",
    },
    BossSpec {
        name: "THE OATHLESS CURATE",
        epithet: "he read the covenant's price",
        hue: Color::from_rgba(120, 90, 150, 255),
        figure: BossFigure::Npc(Archetype::Oracle),
        signature: "Unwrit - your sigils fall one by one",
        act: "V · Underkeep depths",
        hp: "150 HP",
    },
    BossSpec {
        name: "THE ADJUDICATOR",
        epithet: "the last judgment of the Vigil",
        hue: Color::from_rgba(220, 183, 105, 255),
        figure: BossFigure::Npc(Archetype::Adjudicator),
        signature: "It rates your run and adapts",
        act: "VI · Final Trial",
        hp: "168 HP",
    },
];

fn sketch_tidemother(c: &Canvas, r: Rect) {
    let p = r.w / 32.0;
    let teal = Color::from_rgba(50, 110, 120, 255);
    let deep = Color::from_rgba(28, 60, 68, 255);
    let bone = Color::from_rgba(210, 205, 185, 255);
    let gold = Color::from_rgba(200, 170, 80, 255);
    // tentacle skirt
    for (i, dx) in [5.0f32, 9.0, 13.0, 17.0, 21.0, 25.0].into_iter().enumerate() {
        let sway = if i % 2 == 0 { -1.5 } else { 1.5 };
        c.rect(Rect::new(r.x + dx * p + sway, r.y + 22.0 * p, 2.5 * p, 8.0 * p), deep);
    }
    // robe mass and shoulders
    c.rect(Rect::new(r.x + 9.0 * p, r.y + 10.0 * p, 14.0 * p, 14.0 * p), teal);
    c.rect(Rect::new(r.x + 7.0 * p, r.y + 10.0 * p, 18.0 * p, 4.0 * p), deep);
    // head + drowned veil
    c.rect(Rect::new(r.x + 12.0 * p, r.y + 5.0 * p, 8.0 * p, 7.0 * p), bone);
    c.rect(Rect::new(r.x + 12.0 * p, r.y + 9.0 * p, 8.0 * p, 3.0 * p), deep);
    // salt-crown
    c.rect(Rect::new(r.x + 11.0 * p, r.y + 4.0 * p, 10.0 * p, 1.5 * p), gold);
    c.rect(Rect::new(r.x + 12.5 * p, r.y + 2.0 * p, 2.0 * p, 2.5 * p), gold);
    c.rect(Rect::new(r.x + 17.5 * p, r.y + 2.0 * p, 2.0 * p, 2.5 * p), gold);
    // pale eyes
    c.rect(Rect::new(r.x + 13.5 * p, r.y + 7.0 * p, 1.5 * p, 1.2 * p), Color::from_rgba(160, 230, 230, 255));
    c.rect(Rect::new(r.x + 17.0 * p, r.y + 7.0 * p, 1.5 * p, 1.2 * p), Color::from_rgba(160, 230, 230, 255));
}

fn sketch_stag(c: &Canvas, r: Rect) {
    let p = r.w / 32.0;
    let hide = Color::from_rgba(215, 212, 200, 255);
    let shade_c = Color::from_rgba(165, 162, 152, 255);
    let antler = Color::from_rgba(190, 170, 130, 255);
    // legs
    for dx in [8.0f32, 13.0, 22.0, 27.0] {
        c.rect(Rect::new(r.x + dx * p, r.y + 18.0 * p, 2.0 * p, 10.0 * p), shade_c);
    }
    // body, chest, neck
    c.rect(Rect::new(r.x + 6.0 * p, r.y + 13.0 * p, 20.0 * p, 6.0 * p), hide);
    c.rect(Rect::new(r.x + 12.0 * p, r.y + 11.0 * p, 10.0 * p, 4.0 * p), shade_c);
    c.rect(Rect::new(r.x + 21.0 * p, r.y + 8.0 * p, 4.0 * p, 7.0 * p), hide);
    // head and muzzle
    c.rect(Rect::new(r.x + 20.0 * p, r.y + 5.0 * p, 8.0 * p, 5.0 * p), hide);
    c.rect(Rect::new(r.x + 25.0 * p, r.y + 8.0 * p, 4.0 * p, 2.5 * p), shade_c);
    // antlers (stepped forks)
    c.rect(Rect::new(r.x + 21.5 * p, r.y + 2.0 * p, 1.4 * p, 3.4 * p), antler);
    c.rect(Rect::new(r.x + 19.5 * p, r.y + 0.5 * p, 1.4 * p, 2.4 * p), antler);
    c.rect(Rect::new(r.x + 25.5 * p, r.y + 2.0 * p, 1.4 * p, 3.4 * p), antler);
    c.rect(Rect::new(r.x + 27.5 * p, r.y + 0.5 * p, 1.4 * p, 2.4 * p), antler);
    c.rect(Rect::new(r.x + 19.5 * p, r.y + 2.4 * p, 3.4 * p, 1.2 * p), antler);
    c.rect(Rect::new(r.x + 25.5 * p, r.y + 2.4 * p, 3.4 * p, 1.2 * p), antler);
    // eye
    c.rect(Rect::new(r.x + 22.5 * p, r.y + 6.5 * p, 1.6 * p, 1.6 * p), Color::from_rgba(30, 34, 40, 255));
}

fn sketch_wisp(_c: &Canvas, r: Rect) {
    let cx = r.x + r.w / 2.0;
    let cy = r.y + r.h / 2.0;
    let glow_c = Color::from_rgba(90, 200, 180, 120);
    draw_circle(cx, cy, r.w * 0.42, glow_c);
    draw_circle(cx, cy, r.w * 0.26, Color::from_rgba(140, 235, 215, 180));
    draw_circle(cx, cy, r.w * 0.12, Color::from_rgba(235, 255, 250, 255));
}

async fn boss_card(spec: &BossSpec, path: &str) {
    clear_background(Color::from_rgba(6, 7, 9, 255));
    let c = Canvas(Rect::new(0.0, 0.0, 320.0, 460.0));
    panel_frame(&c);
    c.text(spec.name, 16.0, 16.0, 21, LINE);
    c.text(spec.epithet, 16.0, 40.0, 13, DIM);
    glow(&c, 160.0, 190.0, 105.0, spec.hue);
    match spec.figure {
        BossFigure::Npc(archetype) => actor(
            &c,
            Rect::new(90.0, 70.0, 140.0, 210.0),
            spec.hue,
            Figure::Npc(archetype, 1),
            Pose::default(),
        ),
        BossFigure::Tidemother => sketch_tidemother(&c, Rect::new(90.0, 70.0, 140.0, 210.0)),
        BossFigure::Stag => sketch_stag(&c, Rect::new(90.0, 70.0, 140.0, 210.0)),
        BossFigure::Wisp => sketch_wisp(&c, Rect::new(90.0, 70.0, 140.0, 210.0)),
    }
    c.rect(Rect::new(16.0, 300.0, 288.0, 3.0), shade(spec.hue, 0.9));
    c.text("SIGNATURE", 16.0, 318.0, 13, DIM);
    c.text(spec.signature, 16.0, 336.0, 13, WHITE);
    c.text("ACT", 16.0, 364.0, 13, DIM);
    c.text(spec.act, 64.0, 364.0, 13, WHITE);
    c.text("HP", 240.0, 364.0, 13, DIM);
    c.text(spec.hp, 268.0, 364.0, 13, WHITE);
    let image = get_screen_data();
    image.export_png(path);
    println!("GFXLAB OK {path}");
    next_frame().await;
}

async fn area_card(game: &Game, index: usize, path: &str) {
    let mut shot = game.clone();
    let map_name = shot.maps[index].name.replace('—', "-");
    for m in shot.maps.iter_mut() {
        m.explored.fill(true);
    }
    shot.modal = crate::model::Modal::None;
    shot.player.map = index;
    let map = &shot.maps[index];
    // Land on a walkable tile near the center, not blind (rivers exist).
    let (cx, cy) = (map.width / 2, map.height / 2);
    let mut spot = Pos::new(cx, cy);
    'scan: for radius in 0..12 {
        for dy in -radius..=radius {
            for dx in -radius..=radius {
                let p = Pos::new(cx + dx, cy + dy);
                if map.tile(p).walkable() {
                    spot = p;
                    break 'scan;
                }
            }
        }
    }
    shot.player.pos = spot;
    for npc in shot.npcs.iter_mut().filter(|n| n.map == index) {
        if npc.pos.distance(shot.player.pos) > 10 {
            for (dx, dy) in [(3, 3), (0, 4), (4, 0), (-4, 0), (0, -4)] {
                let p = Pos::new(shot.player.pos.x + dx, shot.player.pos.y + dy);
                if shot.maps[index].tile(p).walkable() {
                    npc.pos = p;
                    break;
                }
            }
        }
    }
    clear_background(Color::from_rgba(6, 7, 9, 255));
    let c = Canvas(Rect::new(0.0, 0.0, 640.0, 440.0));
    c.rect(Rect::new(0.0, 0.0, 640.0, 440.0), PARCH);
    let mut animations = ActorAnimations::default();
    sprites::draw_map(
        &shot,
        Rect::new(8.0, 34.0, 624.0, 368.0),
        &mut animations,
        28.0,
    );
    c.border(Rect::new(8.0, 34.0, 624.0, 368.0), 2.0, shade(LINE, 0.8));
    c.text(&map_name, 10.0, 12.0, 22, LINE);
    c.text(&format!("map #{index} · concept card"), 430.0, 420.0, 12, DIM);
    let image = get_screen_data();
    image.export_png(path);
    println!("GFXLAB OK {path}");
    next_frame().await;
}

// ---------------------------------------------------------------------------
// Art-style bake-off (docs/D2_EVOLUTION.md D28): same staged iso scene, three
// style arms. Arm A fidelity ceiling: painterly approximated in code.

#[derive(Clone, Copy, PartialEq)]
enum Arm {
    Painterly,
    Chunky,
    Hybrid,
}

fn rl_terrain(tile: Tile) -> Color {
    match tile {
        Tile::Grass => Color::from_rgba(84, 132, 62, 255),
        Tile::Forest => Color::from_rgba(48, 104, 52, 255),
        Tile::DeepForest => Color::from_rgba(30, 74, 46, 255),
        Tile::Road => Color::from_rgba(196, 160, 100, 255),
        Tile::River => Color::from_rgba(66, 132, 178, 255),
        Tile::Ford => Color::from_rgba(140, 160, 140, 255),
        Tile::Mountain => Color::from_rgba(150, 148, 132, 255),
        Tile::Wall => Color::from_rgba(96, 94, 88, 255),
        Tile::Floor => Color::from_rgba(74, 70, 64, 255),
        Tile::Ruins => Color::from_rgba(140, 130, 100, 255),
        _ => sprites::terrain_color(tile),
    }
}

fn paint_terrain(tile: Tile) -> Color {
    let base = sprites::terrain_color(tile);
    // Grim painterly: desaturate ~40%, deepen ~15%.
    let sat = 0.60;
    let gray = base.r * 0.30 + base.g * 0.59 + base.b * 0.11;
    shade(
        Color::new(
            gray + (base.r - gray) * sat,
            gray + (base.g - gray) * sat,
            gray + (base.b - gray) * sat,
            1.0,
        ),
        0.85,
    )
}

fn env_color(arm: Arm, tile: Tile) -> Color {
    match arm {
        Arm::Painterly | Arm::Hybrid => paint_terrain(tile),
        Arm::Chunky => rl_terrain(tile),
    }
}

/// Sparse deterministic speckle for the painterly arm (grain, not noise field).
fn grain(x: i32, y: i32, cx: f32, cy: f32, hw: f32, hh: f32, base: Color) {
    for i in 0..7u32 {
        let hx = ((x as u32 * 31 + y as u32 * 17 + i * 13) % 100) as f32 / 100.0;
        let hy = ((x as u32 * 13 + y as u32 * 29 + i * 7) % 100) as f32 / 100.0;
        let px = cx - hw + hx * hw * 2.0;
        let along = 1.0 - (hy * 2.0 - 1.0).abs();
        let py = cy - hh * along + hy * hh * 2.0 * along;
        let k = 0.82 + ((i * 37) % 3) as f32 * 0.12;
        draw_rectangle(px, py, vhw(hw, hh), vhw(hw, hh), shade(base, k));
    }
}

fn vhw(hw: f32, hh: f32) -> f32 {
    ((hw + hh) / 14.0).max(1.0)
}

fn draw_actor_styled(c: &Canvas, r: Rect, accent: Color, figure: Figure, pose: Pose, arm: Arm) {
    let env_arm = if arm == Arm::Hybrid { Arm::Chunky } else { arm };
    match env_arm {
        Arm::Chunky => {
            // RL read: near-black ink outline passes, then crisp saturated fill,
            // plus the Blizzard unit brightness-lift on hybrid.
            let lift = if arm == Arm::Hybrid { 1.22 } else { 1.08 };
            let ink = Color::from_rgba(14, 12, 16, 255);
            for (dx, dy) in [(-1.4f32, 0.0), (1.4, 0.0), (0.0, -1.4), (0.0, 1.4)] {
                actor(c, Rect::new(r.x + dx, r.y + dy, r.w, r.h), ink, figure, pose);
            }
            actor(c, r, shade(accent, lift), figure, pose);
        }
        Arm::Painterly => {
            let gray = accent.r * 0.3 + accent.g * 0.59 + accent.b * 0.11;
            let muted = shade(
                Color::new(
                    gray + (accent.r - gray) * 0.55,
                    gray + (accent.g - gray) * 0.55,
                    gray + (accent.b - gray) * 0.55,
                    1.0,
                ),
                0.9,
            );
            actor(c, r, muted, figure, pose);
            // Painter-grain over the figure: hashed sparse dark/light flecks.
            let seed = (r.x as u32 * 7).wrapping_add(r.y as u32 * 13);
            for i in 0..9u32 {
                let hx = ((seed + i * 41) % 100) as f32 / 100.0;
                let hy = ((seed + i * 23) % 100) as f32 / 100.0;
                let k = 0.8 + ((seed / 7 + i) % 3) as f32 * 0.14;
                c.rect(
                    Rect::new(r.x + hx * r.w, r.y + hy * r.h, 1.6, 1.6),
                    shade(muted, k),
                );
            }
        }
        Arm::Hybrid => unreachable!(),
    }
}

async fn artstyle_scene(game: &Game, arm: Arm, title: &str, note: &str, path: &str) {
    let bg = match arm {
        Arm::Painterly => Color::from_rgba(14, 14, 17, 255),
        _ => Color::from_rgba(20, 22, 30, 255),
    };
    clear_background(bg);
    let c = Canvas(Rect::new(0.0, 0.0, SHEET_W, SHEET_H));
    let (hw, hh, ox, oy) = (26.0f32, 13.0f32, 400.0f32, 90.0f32);
    let tall = |t: Tile| matches!(t, Tile::Wall | Tile::Forest | Tile::DeepForest | Tile::Mountain);
    let ink = Color::from_rgba(14, 12, 16, 255);
    for pass in 0..2 {
        for y in 0..13 {
            for x in 0..20 {
                let tile = tile_at(game, x, y);
                let cx = ox + (x - y) as f32 * hw;
                let cy = oy + (x + y) as f32 * hh;
                let base = env_color(arm, tile);
                if pass == 0 {
                    diamond(cx, cy, hw + 0.8, hh + 0.4, base);
                    if arm != Arm::Chunky {
                        grain(x, y, cx, cy, hw, hh, base);
                    }
                    if matches!(tile, Tile::River | Tile::Ford) {
                        diamond(cx, cy, hw * 0.62, hh * 0.62, shade(base, 1.3));
                    }
                    if arm != Arm::Painterly {
                        // RL ink edges on the two south-facing diamond borders.
                        draw_line(cx - hw, cy, cx, cy + hh, 1.2, ink);
                        draw_line(cx + hw, cy, cx, cy + hh, 1.2, ink);
                    }
                } else if tall(tile) {
                    let h = if tile == Tile::Wall { hh * 1.6 } else { hh * 2.2 };
                    let quad = |pts: [Vec2; 4], col: Color| {
                        draw_triangle(pts[0], pts[1], pts[2], col);
                        draw_triangle(pts[2], pts[3], pts[0], col);
                    };
                    quad(
                        [Vec2::new(cx - hw, cy - h), Vec2::new(cx, cy + hh - h), Vec2::new(cx, cy + hh), Vec2::new(cx - hw, cy)],
                        shade(base, if arm == Arm::Chunky { 0.55 } else { 0.62 }),
                    );
                    quad(
                        [Vec2::new(cx, cy + hh - h), Vec2::new(cx + hw, cy - h), Vec2::new(cx + hw, cy), Vec2::new(cx, cy + hh)],
                        shade(base, 0.82),
                    );
                    diamond(cx, cy - h, hw + 0.8, hh + 0.4, shade(base, if arm == Arm::Chunky { 1.32 } else { 1.18 }));
                    if arm != Arm::Painterly {
                        draw_line(cx - hw, cy - h, cx, cy + hh - h, 1.2, ink);
                        draw_line(cx + hw, cy - h, cx, cy + hh - h, 1.2, ink);
                    }
                } else if matches!(tile, Tile::Door | Tile::Chest | Tile::Shrine) {
                    let col = env_color(arm, tile);
                    c.rect(Rect::new(cx - 5.0, cy - 14.0, 10.0, 14.0), shade(col, 1.2));
                    if arm != Arm::Painterly {
                        c.border(Rect::new(cx - 5.0, cy - 14.0, 10.0, 14.0), 1.5, ink);
                    }
                }
            }
        }
    }
    for npc in game.npcs.iter().filter(|n| n.map == 0 && n.alive()) {
        let cx = ox + (npc.pos.x - npc.pos.y) as f32 * hw;
        let cy = oy + (npc.pos.x + npc.pos.y) as f32 * hh;
        draw_actor_styled(
            &c,
            Rect::new(cx - 16.0, cy - 34.0, 32.0, 34.0),
            accent(npc.archetype),
            Figure::Npc(npc.archetype, npc.phase),
            Pose::default(),
            arm,
        );
    }
    let px = ox + (game.player.pos.x - game.player.pos.y) as f32 * hw;
    let py = oy + (game.player.pos.x + game.player.pos.y) as f32 * hh;
    draw_actor_styled(
        &c,
        Rect::new(px - 17.0, py - 36.0, 34.0, 36.0),
        Color::from_rgba(235, 200, 90, 255),
        Figure::Player { weapon: 2, armour: 2, relic: None },
        Pose::default(),
        arm,
    );
    c.text(title, 12.0, 18.0, 28, WHITE);
    c.text(note, 12.0, 44.0, 18, GRAY);
    let image = get_screen_data();
    image.export_png(path);
    println!("GFXLAB OK {path}");
    next_frame().await;
}

// Organic ridge silhouette for tall blocks (mountains/rock), replacing cuboids.
fn ridge(cx: f32, cy: f32, s: f32, base: Color, seed: u32) {
    let wob = |i: u32| (((seed + i * 17) % 7) as f32 - 3.0) * s * 0.05;
    let peak_y = cy - s * (1.3 + (seed % 3) as f32 * 0.22);
    let l = Vec2::new(cx - s * 0.9 + wob(1), cy + s * 0.35);
    let r = Vec2::new(cx + s * 0.95 + wob(2), cy + s * 0.38);
    let peak = Vec2::new(cx + wob(3), peak_y);
    draw_triangle(peak, l, Vec2::new(cx + wob(4), cy + s * 0.3), base);
    draw_triangle(peak, Vec2::new(cx + wob(4), cy + s * 0.3), r, shade(base, 0.8));
    draw_triangle(
        Vec2::new(cx + wob(3) + s * 0.1, peak_y + s * 0.12),
        Vec2::new(cx - s * 0.28, peak_y + s * 0.5),
        Vec2::new(cx + s * 0.3, peak_y + s * 0.46),
        shade(base, 1.3),
    );
}

fn canopy(cx: f32, cy: f32, s: f32, base: Color, seed: u32) {
    let dark = shade(base, 0.55);
    let trunk = Color::from_rgba(70, 52, 36, 255);
    draw_rectangle(cx - s * 0.06, cy - s * 0.2, s * 0.12, s * 0.5, trunk);
    let blobs: [(f32, f32, f32); 4] = [(-0.28, -0.5, 0.3), (0.2, -0.62, 0.34), (-0.02, -0.34, 0.3), (0.05, -0.78, 0.22)];
    for (i, (dx, dy, r)) in blobs.into_iter().enumerate() {
        let jx = (((seed + i as u32 * 11) % 5) as f32 - 2.0) * s * 0.02;
        draw_circle(cx + dx * s + jx, cy + dy * s, r * s, dark);
    }
    for (i, (dx, dy, r)) in blobs.into_iter().enumerate() {
        let jx = (((seed + i as u32 * 11) % 5) as f32 - 2.0) * s * 0.02;
        draw_circle(cx + dx * s + jx, cy + dy * s - 1.0, r * s * 0.88, base);
    }
    draw_circle(
        cx - 0.1 * s,
        cy - 0.66 * s,
        0.16 * s,
        shade(base, 1.25),
    );
}

/// Hybrid arm with the de-blocking pass: organic silhouettes, edge-dither
/// blending between differing neighbor tiles, scatter tufts. (§7.4-6 note)
async fn artstyle_deblocked(game: &Game, path: &str) {
    clear_background(Color::from_rgba(14, 14, 17, 255));
    let c = Canvas(Rect::new(0.0, 0.0, SHEET_W, SHEET_H));
    let (hw, hh, ox, oy) = (26.0f32, 13.0f32, 400.0f32, 90.0f32);
    let tall = |t: Tile| matches!(t, Tile::Wall | Tile::Forest | Tile::DeepForest | Tile::Mountain);
    for pass in 0..2 {
        for y in 0..13 {
            for x in 0..20 {
                let tile = tile_at(game, x, y);
                let cx = ox + (x - y) as f32 * hw;
                let cy = oy + (x + y) as f32 * hh;
                let base = paint_terrain(tile);
                if pass == 0 {
                    diamond(cx, cy, hw + 0.8, hh + 0.4, base);
                    grain(x, y, cx, cy, hw, hh, base);
                    if matches!(tile, Tile::River | Tile::Ford) {
                        diamond(cx, cy, hw * 0.62, hh * 0.62, shade(base, 1.3));
                    }
                    // Edge-dither blend: speckle the neighbor's color along the
                    // shared edge where the four diagonal neighbors differ.
                    let neighbors = [(1, 0, hw * 0.72, 0.0), (-1, 0, -hw * 0.72, 0.0), (0, 1, 0.0, hh * 0.72), (0, -1, 0.0, -hh * 0.72)];
                    for (k, (dx, dy, ex, ey)) in neighbors.into_iter().enumerate() {
                        let other = tile_at(game, x + dx, y + dy);
                        if other != tile {
                            let oc = paint_terrain(other);
                            for i in 0..4u32 {
                                let t0 = (((x as u32 * 13 + y as u32 * 7 + i * 5 + k as u32) % 11) as f32 / 11.0 - 0.5) * 1.6;
                                let dot = vhw(hw, hh);
                                draw_rectangle(
                                    cx + ex + if dx == 0 { t0 * hw * 0.4 } else { 0.0 },
                                    cy + ey + if dy == 0 { t0 * hh * 0.4 } else { 0.0 },
                                    dot,
                                    dot,
                                    oc,
                                );
                            }
                        }
                    }
                    // Scatter tufts on vegetated ground.
                    if matches!(tile, Tile::Grass | Tile::Forest | Tile::DeepForest) {
                        for i in 0..3u32 {
                            let tx = cx - hw * 0.5 + ((x as u32 * 19 + y as u32 * 5 + i * 23) % 100) as f32 / 100.0 * hw;
                            let ty = cy - hh * 0.5 + ((x as u32 * 7 + y as u32 * 31 + i * 11) % 100) as f32 / 100.0 * hh;
                            let up = shade(base, 1.3);
                            draw_rectangle(tx, ty - 2.0, 1.2, 2.0, up);
                            draw_rectangle(tx + 1.2, ty, 1.6, 1.0, up);
                        }
                    }
                } else if tall(tile) {
                    let seed = x as u32 * 977 + y as u32 * 421;
                    match tile {
                        Tile::Mountain => ridge(cx, cy - hh * 0.4, hw * 1.5, base, seed),
                        Tile::Forest | Tile::DeepForest => canopy(cx, cy, hw * 1.7, base, seed),
                        Tile::Wall => {
                            // Boxy keep, but rubble-stepped top line.
                            let h = hh * 1.6;
                            let quad = |pts: [Vec2; 4], col: Color| {
                                draw_triangle(pts[0], pts[1], pts[2], col);
                                draw_triangle(pts[2], pts[3], pts[0], col);
                            };
                            quad(
                                [Vec2::new(cx - hw, cy - h), Vec2::new(cx, cy + hh - h), Vec2::new(cx, cy + hh), Vec2::new(cx - hw, cy)],
                                shade(base, 0.62),
                            );
                            quad(
                                [Vec2::new(cx, cy + hh - h), Vec2::new(cx + hw, cy - h), Vec2::new(cx + hw, cy), Vec2::new(cx, cy + hh)],
                                shade(base, 0.82),
                            );
                            diamond(cx, cy - h, hw + 0.8, hh + 0.4, shade(base, 1.18));
                            for i in 0..4u32 {
                                let rx = cx - hw + ((seed + i * 29) % 100) as f32 / 100.0 * hw * 2.0;
                                let step = 1.5 + ((seed + i * 7) % 3) as f32;
                                draw_rectangle(rx, cy - h - step, 3.0, step, shade(base, 1.05));
                            }
                        }
                        _ => {}
                    }
                } else if matches!(tile, Tile::Door | Tile::Chest | Tile::Shrine) {
                    let col = paint_terrain(tile);
                    c.rect(Rect::new(cx - 5.0, cy - 14.0, 10.0, 14.0), shade(col, 1.2));
                }
            }
        }
    }
    for npc in game.npcs.iter().filter(|n| n.map == 0 && n.alive()) {
        let cx = ox + (npc.pos.x - npc.pos.y) as f32 * hw;
        let cy = oy + (npc.pos.x + npc.pos.y) as f32 * hh;
        draw_actor_styled(
            &c,
            Rect::new(cx - 16.0, cy - 34.0, 32.0, 34.0),
            accent(npc.archetype),
            Figure::Npc(npc.archetype, npc.phase),
            Pose::default(),
            Arm::Hybrid,
        );
    }
    let px = ox + (game.player.pos.x - game.player.pos.y) as f32 * hw;
    let py = oy + (game.player.pos.x + game.player.pos.y) as f32 * hh;
    draw_actor_styled(
        &c,
        Rect::new(px - 17.0, py - 36.0, 34.0, 36.0),
        Color::from_rgba(235, 200, 90, 255),
        Figure::Player { weapon: 2, armour: 2, relic: None },
        Pose::default(),
        Arm::Hybrid,
    );
    c.text("C+ - hybrid, de-blocked (target read)", 12.0, 18.0, 28, WHITE);
    c.text("organic silhouettes, edge blends, scatter -- still ZERO baked art", 12.0, 44.0, 18, GRAY);
    let image = get_screen_data();
    image.export_png(path);
    println!("GFXLAB OK {path}");
    next_frame().await;
}

/// Renders the three art-style arms of the same scene (D28).
pub async fn artstyle_sheets(dir: &str) {
    let game = scene_game(42);
    std::fs::create_dir_all(dir).unwrap_or_default();
    artstyle_scene(
        &game,
        Arm::Painterly,
        "A - grim painterly (D2 realism, code-approx)",
        "CEILING NOTE: painterly approximated procedurally; real plates go further",
        &format!("{dir}/artstyle-a-painterly.png"),
    )
    .await;
    artstyle_scene(
        &game,
        Arm::Chunky,
        "B - chunky cartoon (Rogue Legacy read)",
        "ink outlines, flat tones, saturated discipline",
        &format!("{dir}/artstyle-b-chunky.png"),
    )
    .await;
    artstyle_scene(
        &game,
        Arm::Hybrid,
        "C - hybrid: grim env + chunky lifted actors",
        "environment from A, actors from B (+22% read lift)",
        &format!("{dir}/artstyle-c-hybrid.png"),
    )
    .await;
    artstyle_deblocked(&game, &format!("{dir}/artstyle-c-plus-deblocked.png")).await;
}

/// Renders every concept card into docs/gfx/proto/{classes,bosses,areas}/.
pub async fn concept_cards(dir: &str) {
    let game = Game::new(42);
    // Portrait cards: shrink the logical window so exports are card-sized.
    request_new_screen_size(320.0, 460.0);
    next_frame().await;
    for (i, spec) in CLASSES.iter().enumerate() {
        std::fs::create_dir_all(format!("{dir}/classes")).unwrap_or_default();
        class_card(spec, &format!("{dir}/classes/{}-{}.png", i + 1, spec.name.to_lowercase())).await;
    }
    for (i, spec) in BOSSES.iter().enumerate() {
        std::fs::create_dir_all(format!("{dir}/bosses")).unwrap_or_default();
        boss_card(
            spec,
            &format!(
                "{dir}/bosses/{:02}-{}.png",
                i + 1,
                spec.name.to_lowercase().replace(' ', "-").replace(',', "")
            ),
        )
        .await;
    }
    std::fs::create_dir_all(format!("{dir}/areas")).unwrap_or_default();
    request_new_screen_size(640.0, 440.0);
    next_frame().await;
    for index in 0..game.maps.len() {
        let slug = game.maps[index]
            .name
            .to_lowercase()
            .replace(' ', "-")
            .replace(['—', ','], "");
        area_card(&game, index, &format!("{dir}/areas/{:02}-{slug}.png", index + 1)).await;
    }
}
