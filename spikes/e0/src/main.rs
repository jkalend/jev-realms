//! E0 — Bevy engine spike (timeboxed, THROWAWAY evidence tooling per
//! docs/D2_EVOLUTION.md §7.5). Judges: does the D2 read work with zero new art,
//! and do bevy_light_2d occluder shadows behave on tile walls?
//!
//! Route (a): the sim's own light map (torch radius / night / fog tiers from
//! src/sprites.rs + src/ui.rs) baked per-tile.
//! Route (b): one bevy_light_2d PointLight2d torch with cast shadows against
//! LightOccluder2d wall tiles.
//! Post: built-in Bloom + Vignette (0.19). Captures two screenshots and exits.

use bevy::app::AppExit;
use bevy::post_process::bloom::Bloom;
use bevy::post_process::effect_stack::Vignette;
use bevy::prelude::*;
use bevy::render::view::window::screenshot::{save_to_disk, Screenshot};
use bevy_light_2d::prelude::*;
use laya_realms::model::{Game, Item, Pos, Tile};

/// World units per tile. Camera scale is set so ~26 tiles show across 1280px.
const T: f32 = 32.0;
const VIEW_W: i32 = 26; // tactical zoom: ~24-26 tiles across (D2 density lesson, §7.4-7)
const VIEW_H: i32 = 15;
const CAM_SCALE: f32 = VIEW_W as f32 * T / 1280.0;
/// D25 tilt evidence constants, identical to gfxlab variant_tilt.
const SQUASH: f32 = 0.58;
const SHEAR: f32 = 0.34;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Square,
    Tilt,
}

#[derive(Resource)]
struct Spike {
    game: Game,
    radius: i32,
    frame: u32,
    mode: Mode,
    saved_a: bool,
    saved_b: bool,
}

#[derive(Component)]
struct GridPos {
    gx: i32,   // east of player, tiles
    gy: i32,   // south of player, tiles
    tier: i32, // tiles 0, figures 1, flame 2 (tilt overlap order)
}

#[derive(Component)]
struct Walker;

/// gfxlab/sprites.rs `terrain_color`, verbatim — the game's existing palette.
fn terrain_color(t: Tile) -> (u8, u8, u8) {
    match t {
        Tile::Grass => (48, 76, 37),
        Tile::Forest => (32, 61, 31),
        Tile::DeepForest => (21, 43, 28),
        Tile::Road => (153, 126, 78),
        Tile::Floor => (44, 42, 39),
        Tile::Wall => (66, 66, 61),
        Tile::Mountain => (113, 114, 105),
        Tile::Rock => (50, 55, 55),
        Tile::River => (35, 82, 111),
        Tile::Ford => (111, 127, 111),
        Tile::Ruins => (109, 105, 80),
        Tile::Door | Tile::Chest => (151, 111, 46),
        Tile::Up | Tile::Down => (186, 174, 129),
        Tile::Shrine => (108, 153, 171),
    }
}

/// sprites.rs `light()`: three tiers — full band, falloff rim, remembered gray.
/// An exposure lift brightens for judgment (tier RELATIONSHIPS untouched).
const EXPOSURE: f32 = 2.2;
fn lit(base: (u8, u8, u8), strength: f32, remembered: bool) -> Color {
    let (r, g, b) = (
        base.0 as f32 / 255.0,
        base.1 as f32 / 255.0,
        base.2 as f32 / 255.0,
    );
    let (r, g, b) = if remembered {
        let gray = (r * 0.30 + g * 0.59 + b * 0.11) * 0.32;
        (gray * 0.92, gray, gray * 1.05)
    } else {
        (r * strength, g * strength, b * strength)
    };
    Color::srgb(
        (r * EXPOSURE).min(1.0),
        (g * EXPOSURE).min(1.0),
        (b * EXPOSURE).min(1.0),
    )
}

fn tier(dist: i32, radius: i32, tile: Tile) -> (f32, bool) {
    let strength = if dist * 4 <= radius * 3 { 1.0 } else { 0.55 }
        * if tile == Tile::DeepForest { 0.9 } else { 1.0 };
    (strength, dist > radius)
}

/// Grid offset (gx east, gy south of player) → world translation for a mode.
fn project(mode: Mode, gx: i32, gy: i32) -> (Vec3, f32) {
    let (fx, fy) = (gx as f32, gy as f32);
    match mode {
        Mode::Square => (Vec3::new(fx * T, -fy * T, 0.0), 0.0),
        Mode::Tilt => {
            let sx = fx * T + fy * T * SHEAR;
            let sy = fy * T * SQUASH;
            (Vec3::new(sx, -sy, 0.0), fy * 0.05) // southern rows in front
        }
    }
}

/// Find a spot on the starting map: open field around the player (so the
/// route-(a) tiers read), with exactly a few wall tiles at ring 3..=5 to act
/// as clean route-(b) occluders, and a clear east-west walker corridor.
fn wall_spot(game: &Game) -> Pos {
    let map = game.map();
    let mut best = (game.player.pos, 0i32);
    for y in 14..map.height - 14 {
        for x in 14..map.width - 14 {
            let p = Pos::new(x, y);
            if !map.tile(p).walkable() {
                continue;
            }
            let corridor = (-10..=10).all(|dx| map.tile(p.offset(dx, -2)).walkable());
            if !corridor {
                continue;
            }
            let occludes = |t: Tile| matches!(t, Tile::Wall | Tile::Rock | Tile::Mountain);
            let mut ring_walls: i32 = 0;
            let mut open_within_2 = true;
            let mut kinds = [false; 16];
            for dy in -6..=6 {
                for dx in -6..=6 {
                    let t = map.tile(p.offset(dx, dy));
                    kinds[t as usize] = true;
                    let d = dx.abs().max(dy.abs());
                    if d <= 2 && occludes(t) {
                        open_within_2 = false;
                    }
                    if (3..=5).contains(&d) && occludes(t) {
                        ring_walls += 1;
                    }
                }
            }
            // Keep a light footprint: too many walls make a shadow cave.
            if !open_within_2 || !(2..=6).contains(&ring_walls) {
                continue;
            }
            let variety = kinds.iter().filter(|k| **k).count() as i32;
            let score = variety * 10 - (ring_walls - 4).abs();
            if score > best.1 {
                best = (p, score);
            }
        }
    }
    best.0
}

fn build_snapshot() -> Spike {
    let mut game = Game::new(42);
    game.player.pos = wall_spot(&game);
    // Explore at the day radius first so "remembered" memory tiles exist...
    game.hour_ticks = 480 * 4; // 12:00
    game.reveal();
    // ...then drop to deep night and light the sim's own torch: radius 6.
    game.hour_ticks = 480 * 14; // 22:00
    assert!(game.night(), "spike must run at night");
    let torch = game
        .player
        .inventory
        .iter()
        .position(|i| *i == Item::Torch)
        .expect("starting kit has a torch");
    assert!(game.use_item(torch));
    game.reveal();
    assert!(game.tick < game.player.torch_until, "torch must be lit");
    Spike {
        radius: 6,
        game,
        frame: 0,
        mode: Mode::Square,
        saved_a: false,
        saved_b: false,
    }
}

fn setup(mut commands: Commands) {
    let spike = build_snapshot();
    let game = &spike.game;
    let map = game.map();
    let player = game.player.pos;
    let radius = spike.radius;

    commands.spawn((
        Camera2d,
        Projection::Orthographic({
            let mut p = OrthographicProjection::default_2d();
            p.scale = CAM_SCALE;
            p
        }),
        // Route (b): deep blue night ambient; the torch pool rides on top.
        Light2d {
            ambient_light: AmbientLight2d {
                color: Color::srgb(0.55, 0.62, 0.85),
                brightness: 0.35,
            },
        },
        Bloom {
            intensity: 0.08,
            low_frequency_boost: 0.15,
            ..Bloom::NATURAL
        },
        Vignette {
            intensity: 0.35,
            radius: 0.55,
            ..default()
        },
    ));

    // Route (a): per-tile projected light — palette × sim tiers straight from
    // the live Game snapshot. Unexplored stays black (no entity).
    for gy in -VIEW_H / 2..=VIEW_H / 2 + 1 {
        for gx in -VIEW_W / 2 - 1..=VIEW_W / 2 + 1 {
            let pos = player.offset(gx, gy);
            let Some(index) = map.index(pos) else { continue };
            if !map.explored.get(index).copied().unwrap_or(false) {
                continue;
            }
            let tile = map.tiles[index];
            let dist = gx.abs().max(gy.abs());
            let (strength, remembered) = tier(dist, radius, tile);
            let (translation, z) = project(Mode::Square, gx, gy);
            let mut entity = commands.spawn((
                Sprite::from_color(lit(terrain_color(tile), strength, remembered), Vec2::splat(T)),
                Transform::from_translation(translation.with_z(z)),
                GridPos { gx, gy, tier: 0 },
            ));
            // Route (b): tall tiles block the torch and cast its shadow.
            if matches!(tile, Tile::Wall | Tile::Rock | Tile::Mountain) {
                entity.insert(LightOccluder2d {
                    shape: LightOccluder2dShape::Rectangle {
                        half_size: Vec2::splat(T * 0.45),
                    },
                });
            }
        }
    }

    // Player figure + HDR torch flame pixel (feeds bloom).
    let (p, z) = project(Mode::Square, 0, 0);
    commands.spawn((
        Sprite::from_color(Color::srgb_u8(235, 200, 90), Vec2::splat(T * 0.8)),
        Transform::from_translation(p.with_z(z + 1.0)),
        GridPos { gx: 0, gy: 0, tier: 1 },
    ));
    commands.spawn((
        Sprite::from_color(
            Color::LinearRgba(LinearRgba::new(1.35, 0.8, 0.35, 1.0)),
            Vec2::splat(T * 0.35),
        ),
        Transform::from_translation(p.with_z(z + 2.0)),
        GridPos { gx: 0, gy: 0, tier: 2 },
    ));
    commands.spawn((
        PointLight2d {
            radius: 6.0 * T,
            intensity: 1.5,
            falloff: 10.0,
            color: Color::srgb(1.0, 0.72, 0.42),
            cast_shadows: true,
            ..default()
        },
        Transform::from_translation(p.with_z(z + 3.0)),
        GridPos { gx: 0, gy: 0, tier: 2 },
    ));

    // One framed standing stone (render-side only, not sim state) inside the
    // pool, so route (b)'s occluder shadow is unambiguous for the judges.
    let (s, sz) = project(Mode::Square, 3, 0);
    commands.spawn((
        Sprite::from_color(lit(terrain_color(Tile::Rock), 1.0, false), Vec2::splat(T * 0.9)),
        Transform::from_translation(s.with_z(sz + 1.0)),
        GridPos { gx: 3, gy: 0, tier: 1 },
        LightOccluder2d {
            shape: LightOccluder2dShape::Rectangle {
                half_size: Vec2::splat(T * 0.45),
            },
        },
    ));

    // One actor walking a straight east-west path through the torch boundary.
    commands.spawn((
        Sprite::from_color(Color::srgb_u8(200, 90, 80), Vec2::splat(T * 0.8)),
        Transform::from_translation(Vec3::new(-10.0 * T, 2.0 * T, 2.0)),
        GridPos { gx: -10, gy: -2, tier: 1 },
        Walker,
    ));

    commands.insert_resource(spike);
}

fn drive(
    mut spike: ResMut<Spike>,
    mut commands: Commands,
    mut query: Query<(&GridPos, &mut Transform, Option<&Walker>, &mut Sprite)>,
    mut exit: MessageWriter<AppExit>,
) {
    spike.frame += 1;
    let frame = spike.frame;

    // Walker: drifts east across the lit/unlit boundary, tinted per its tile.
    {
        let walk = (((frame / 4) % 60) as f32 - 31.0) / 4.0; // -7.75..=7.25
        let radius = spike.radius;
        for (g, mut transform, walker, mut sprite) in query.iter_mut() {
            if walker.is_none() {
                continue;
            }
            let dist = (walk.abs() as i32).max(g.gy.abs());
            let (strength, remembered) = tier(dist, radius, Tile::Grass);
            sprite.color = lit((200, 90, 80), strength, remembered);
            let (base, zb) = project(spike.mode, 0, g.gy);
            transform.translation = Vec3::new(base.x + walk * T, base.y, zb + g.tier as f32);
        }
    }

    match frame {
        // Screenshot A: square projection, routes a+b+post.
        30 if !spike.saved_a => {
            spike.saved_a = true;
            let path = std::env::current_dir()
                .unwrap()
                .join("docs/gfx/proto/e0-a-lgttorch.png");
            commands
                .spawn(Screenshot::primary_window())
                .observe(save_to_disk(path));
        }
        // Switch to the D25 tilt/diorama variant of the same scene.
        50 if spike.mode == Mode::Square => {
            spike.mode = Mode::Tilt;
            for (g, mut transform, _, _) in query.iter_mut() {
                let (t, z) = project(Mode::Tilt, g.gx, g.gy);
                transform.translation = t.with_z(z + g.tier as f32);
            }
        }
        // Screenshot B: tilt/diorama projection of the same scene.
        80 if !spike.saved_b => {
            spike.saved_b = true;
            let path = std::env::current_dir()
                .unwrap()
                .join("docs/gfx/proto/e0-b-diorama.png");
            commands
                .spawn(Screenshot::primary_window())
                .observe(save_to_disk(path));
        }
        130.. => {
            exit.write(AppExit::Success);
        }
        _ => {}
    }
}

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title: "laya-realms E0 spike".into(),
                    resolution: (1280, 720).into(),
                    ..default()
                }),
                ..default()
            }),
            Light2dPlugin,
        ))
        .insert_resource(ClearColor(Color::srgb_u8(10, 13, 18)))
        .add_systems(Startup, setup)
        .add_systems(Update, drive)
        .run();
}
