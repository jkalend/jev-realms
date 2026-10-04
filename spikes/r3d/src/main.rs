//! R3D — throwaway Bevy 3D blockout evidence (docs/D2_EVOLUTION.md §7.5 lane).
//! Question: what does "the actual D2 recipe" (pre-rendered 3D plates) vs
//! "full real-time 3D view" read like over the SAME seed-42 sim snapshot,
//! with ZERO art (mesh primitives + the game's existing flat palette only)?
//!
//! Scene: 21x13 tile neighborhood around a torch-lit player at night in the
//! Millbrook city map. Floors = thin quads at palette color; tall tiles
//! (Wall/Rock/Mountain/DeepForest) = cuboid blocks; river semi-emissive;
//! player = capsule + head cylinder; one staged bandit cuboid; one torch
//! column with an HDR emissive tip feeding Bloom. One warm PointLight with
//! shadows carries the night read.
//!
//! Shots (three cameras, one active at a time, screenshots at fixed frames):
//!   r3d-fullview.png  — pitched 3D blockout, the real-time-3D answer.
//!   r3d-plateview.png — tight 4x3 crop look, the pre-rendered-plate answer.
//!   r3d-square.png    — same scene straight overhead (macroquad comparator).

use bevy::app::AppExit;
use bevy::post_process::bloom::Bloom;
use bevy::post_process::effect_stack::Vignette;
use bevy::prelude::*;
use bevy::render::view::window::screenshot::{save_to_disk, Screenshot};
use laya_realms::model::{Game, Item, Pos, Tile};

/// Neighborhood half-extents: 21 wide (x/-east), 13 deep (z/-south).
const GX: i32 = 10;
const GY: i32 = 6;

/// Tile palette, verbatim from gfxlab/sprites.rs `terrain_color`.
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

/// Blockout extrusion height for the occluding tiles (spec band 0.5..=1.4).
fn block_height(t: Tile) -> Option<f32> {
    match t {
        Tile::Wall => Some(0.8),
        Tile::Rock => Some(0.6),
        Tile::Mountain => Some(1.4),
        Tile::DeepForest => Some(1.1),
        _ => None,
    }
}

/// Same Millbrook open-field scout as spikes/e0: walkable spot, clear
/// east-west corridor, a handful of occluders at ring 3..=5 to catch the
/// torch shadow, nothing inside ring 2.
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

/// Live sim snapshot: seed 42, spot scouted at noon so memory exists, then
/// deep night with the starting-kit torch burning (radius 6).
fn build_snapshot() -> Game {
    let mut game = Game::new(42);
    game.player.pos = wall_spot(&game);
    game.hour_ticks = 480 * 4; // 12:00
    game.reveal();
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
    game
}

/// Which of the three camera rigs is live.
#[derive(Component)]
struct Cam(u8);

#[derive(Resource)]
struct Shot {
    frame: u32,
    saved: [bool; 3],
}

fn camera_body(t: Transform, fov: f32, active: bool) -> impl Bundle {
    (
        Camera3d::default(),
        Projection::Perspective(PerspectiveProjection {
            fov,
            ..default()
        }),
        t,
        Camera {
            is_active: active,
            ..default()
        },
        Bloom {
            intensity: 0.06,
            low_frequency_boost: 0.15,
            ..Bloom::NATURAL
        },
        Vignette {
            intensity: 0.3,
            radius: 0.55,
            ..default()
        },
    )
}

/// 45° yaw / 38° pitch offset from a target at the given distance.
fn orbit(target: Vec3, dist: f32, yaw_deg: f32, pitch_deg: f32) -> Transform {
    let yaw = yaw_deg.to_radians();
    let pitch = pitch_deg.to_radians();
    let dir = Vec3::new(
        pitch.cos() * yaw.sin(),
        pitch.sin(),
        pitch.cos() * yaw.cos(),
    );
    Transform::from_translation(target + dir * dist).looking_at(target, Vec3::Y)
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let game = build_snapshot();
    let map = game.map();
    let player = game.player.pos;

    // --- tile neighborhood -------------------------------------------------
    let floor_mesh = meshes.add(Cuboid::new(1.0, 0.02, 1.0));
    let block_mesh = meshes.add(Cuboid::new(1.0, 1.0, 1.0));
    let mut mats: [Option<Handle<StandardMaterial>>; 16] = Default::default();
    let mut nearest_wall: Option<(i32, Vec2)> = None;
    for gy in -GY..=GY {
        for gx in -GX..=GX {
            let pos = player.offset(gx, gy);
            let Some(index) = map.index(pos) else { continue };
            let tile = map.tiles[index];
            let (r, g, b) = terrain_color(tile);
            let mat = mats[tile as usize].get_or_insert_with(|| {
                let mut m = StandardMaterial {
                    base_color: Color::srgb_u8(r, g, b),
                    perceptual_roughness: 1.0,
                    metallic: 0.0,
                    ..default()
                };
                if tile == Tile::River {
                    m.emissive = LinearRgba::rgb(0.02, 0.06, 0.11);
                }
                materials.add(m)
            });
            let raised = if tile == Tile::Road { 0.01 } else { 0.0 };
            let center = Vec3::new(gx as f32, raised, gy as f32);
            commands.spawn((
                Mesh3d(floor_mesh.clone()),
                MeshMaterial3d(mat.clone()),
                Transform::from_translation(center),
            ));
            if let Some(h) = block_height(tile) {
                commands.spawn((
                    Mesh3d(block_mesh.clone()),
                    MeshMaterial3d(mat.clone()),
                    Transform::from_translation(center + Vec3::Y * (0.01 + h / 2.0))
                        .with_scale(Vec3::new(0.96, h, 0.96)),
                ));
                let d = gx.abs().max(gy.abs());
                if (3..=5).contains(&d)
                    && nearest_wall.is_none_or(|(bd, _)| d < bd)
                {
                    nearest_wall = Some((d, Vec2::new(gx as f32, gy as f32)));
                }
            }
        }
    }

    // --- figures -----------------------------------------------------------
    // Player: capsule body + head cylinder, e0's gold figure tint.
    let gold = materials.add(StandardMaterial {
        base_color: Color::srgb_u8(235, 200, 90),
        perceptual_roughness: 0.9,
        ..default()
    });
    let skin = materials.add(StandardMaterial {
        base_color: Color::srgb_u8(214, 170, 130),
        perceptual_roughness: 0.9,
        ..default()
    });
    commands.spawn((
        Mesh3d(meshes.add(Capsule3d::new(0.16, 0.42))),
        MeshMaterial3d(gold),
        Transform::from_xyz(0.0, 0.38, 0.0),
    ));
    commands.spawn((
        Mesh3d(meshes.add(Cylinder::new(0.13, 0.12))),
        MeshMaterial3d(skin),
        Transform::from_xyz(0.0, 0.82, 0.0),
    ));
    // One staged bandit cuboid on the nearest open neighbor tile east-ish.
    let bandit_at = [(2, 1), (1, 2), (2, -1), (1, -2), (-2, 1)]
        .into_iter()
        .find(|(dx, dy)| map.tile(player.offset(*dx, *dy)).walkable())
        .unwrap_or((2, 1));
    commands.spawn((
        Mesh3d(block_mesh.clone()),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb_u8(200, 90, 80),
            perceptual_roughness: 0.9,
            ..default()
        })),
        Transform::from_xyz(bandit_at.0 as f32, 0.56, bandit_at.1 as f32)
            .with_scale(Vec3::new(0.5, 1.1, 0.5)),
    ));
    // Torch column at the player's tile edge: thin pole + HDR emissive tip.
    let torch_tip = Vec3::new(0.45, 0.88, 0.36);
    commands.spawn((
        Mesh3d(meshes.add(Cylinder::new(0.04, 0.8))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb_u8(92, 64, 38),
            perceptual_roughness: 1.0,
            ..default()
        })),
        Transform::from_xyz(torch_tip.x, 0.42, torch_tip.z),
    ));
    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(0.09))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::BLACK,
            emissive: LinearRgba::rgb(6.0, 3.0, 1.2),
            ..default()
        })),
        Transform::from_translation(torch_tip),
    ));
    // THE light: one warm shadowed point light at the torch.
    commands.spawn((
        PointLight {
            color: Color::srgb(1.0, 0.72, 0.42),
            intensity: 42000.0,
            range: 20.0,
            radius: 0.12,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_translation(torch_tip + Vec3::Y * 0.25),
    ));
    // Night fill: VERY faint cool, almost straight down so walls read as mass.
    commands.spawn((
        DirectionalLight {
            color: Color::srgb(0.62, 0.72, 1.0),
            illuminance: 2600.0,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_rotation_x(-1.45)),
    ));
    commands.insert_resource(GlobalAmbientLight {
        color: Color::srgb(0.30, 0.32, 0.42),
        brightness: 200.0,
        ..default()
    });

    // --- cameras -----------------------------------------------------------
    // 1: full blockout view — frames the 21x13 neighborhood at yaw 45/pitch 38.
    commands.spawn((
        camera_body(orbit(Vec3::new(0.0, 0.4, 0.0), 30.0, 45.0, 38.0), 0.45, true),
        Cam(0),
    ));
    // 2: plate crop — tight on ~4x3 tiles holding torch + player + ring walls:
    // midpoint between the player and the nearest ring wall,
    // pulled toward the pool so torch + player + bandit + wall row all fit
    // in one ~4x3-tile frame — the "baked plate" composition.
    let plate_center = nearest_wall
        .map(|(_, w)| w * 0.35)
        .unwrap_or(Vec2::new(1.5, 1.0));
    commands.spawn((
        camera_body(
            orbit(
                Vec3::new(plate_center.x + 0.5, 0.45, plate_center.y + 0.2),
                9.0,
                45.0,
                38.0,
            ),
            0.45,
            false,
        ),
        Cam(1),
    ));
    // 3: honest square comparator — same scene, straight overhead, narrow FOV
    // from far up so the projection reads flat like the macroquad view.
    commands.spawn((
        camera_body(
            Transform::from_translation(Vec3::new(0.0, 80.0, 0.02))
                .looking_at(Vec3::ZERO, Vec3::NEG_Z),
            0.17,
            false,
        ),
        Cam(2),
    ));

    commands.insert_resource(Shot {
        frame: 0,
        saved: [false; 3],
    });
}

fn shots_game(
    mut shot: ResMut<Shot>,
    mut cams: Query<(&Cam, &mut Camera)>,
    mut commands: Commands,
    mut exit: MessageWriter<AppExit>,
) {
    shot.frame += 1;
    let frame = shot.frame;
    let paths = [
        "docs/gfx/proto/r3d-fullview.png",
        "docs/gfx/proto/r3d-plateview.png",
        "docs/gfx/proto/r3d-square.png",
    ];
    // Camera hand-offs one frame before each capture so the switch is settled.
    match frame {
        55 => cams.iter_mut().for_each(|(c, mut cam)| cam.is_active = c.0 == 1),
        85 => cams.iter_mut().for_each(|(c, mut cam)| cam.is_active = c.0 == 2),
        _ => {}
    }
    for (i, at) in [40u32, 70u32, 100u32].into_iter().enumerate() {
        if frame == at && !shot.saved[i] {
            shot.saved[i] = true;
            let path = std::env::current_dir().unwrap().join(paths[i]);
            commands
                .spawn(Screenshot::primary_window())
                .observe(save_to_disk(path));
        }
    }
    if frame >= 150 {
        exit.write(AppExit::Success);
    }
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "laya-realms R3D spike".into(),
                resolution: (1280, 720).into(),
                ..default()
            }),
            ..default()
        }))
        .insert_resource(ClearColor(Color::srgb_u8(10, 13, 18)))
        .add_systems(Startup, setup)
        .add_systems(Update, shots_game)
        .run();
}
