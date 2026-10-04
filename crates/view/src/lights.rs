//! Light layer: the §7.5★-ratified numbers.
//! - Route (a) is the per-tile quad bake in tiles.rs (base, plugin-lag-immune).
//! - Route (b) is this one `PointLight2d` pool at the player while torchlit:
//!   falloff/intensity/ambient straight from the E0 run (falloff 8–10,
//!   intensity 1.2–1.5, ambient 0.08–0.35 — 0.65 washes shadows out).
//! - Post stack: Bloom 0.08 + Vignette 0.35 on the camera (spawned in lib).
//! - Combat bubble: while `game.combat` is live, the vignette pulses red at
//!   the frame edge — the placeholder E4 bubble until s4 chrome replaces it.

use bevy::post_process::effect_stack::Vignette;
use bevy::prelude::*;
use bevy_light_2d::prelude::*;

use crate::camera::MainCam;
use crate::projection::{Proj, T};
use crate::sim::WorldView;


#[derive(Component)]
pub struct TorchFlame;
#[derive(Component)]
pub struct TorchPool;

/// Tracks the torch pool's previous lighting state so the point light is only
/// inserted/removed on transitions, never re-emitted each frame.
#[derive(Resource, Default)]
pub struct TorchState {
    lit: bool,
}

/// The bloom feed: a small ember core riding with the player. It used to be a
/// tile-sized HDR square, which read as a gold box stuck to the character's
/// chest once the terrain got brighter. A procedural radial texture is the
/// shape it really wants, but `mesh2d` samples these as D3 views and a
/// runtime-built 2D image fails its bind group; the point light below carries
/// the actual pool regardless.
pub fn setup(mut commands: Commands) {
    commands.spawn((
        Sprite {
            color: Color::srgba(0.98, 0.55, 0.24, 0.9),
            custom_size: Some(Vec2::splat(T * 0.15)),
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, 3.0),
        Visibility::Hidden,
        TorchFlame,
    ));
    // The pool itself: transform only; `PointLight2d` joins while torchlit.
    commands.spawn((Transform::from_xyz(0.0, 0.0, 3.0), TorchPool));
}

pub fn update(
    view: Res<WorldView>,
    time: Res<Time>,
    proj: Res<Proj>,
    view_options: Res<crate::options::ViewOptions>,
    mut state: ResMut<TorchState>,
    mut commands: Commands,
    mut flame: Query<(&mut Transform, &mut Visibility), (With<TorchFlame>, Without<TorchPool>)>,
    mut pool: Query<(Entity, &mut Transform), With<TorchPool>>,
    mut post: Query<(&mut Light2d, &mut Vignette), With<MainCam>>,
) {
    // Deliberately ABOVE the whole world band: the flame sprite and the light
    // pool are overlays that must never be sorted against terrain, and they
    // carry their own +1 on top of this. Everything that lives on the ground
    // belongs in `Proj::tile_band(pos) + Proj::Z_*` instead (see projection.rs
    // for the layer order); this pair is not part of that ordering.
    let pos = proj.world(view.player, 2.0 + Proj::tile_band(view.player));
    if let Ok((mut t, mut visibility)) = flame.single_mut() {
        t.translation = pos.with_z(pos.z + 1.0);
        *visibility = if view.torchlit {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    if let Ok((entity, mut t)) = pool.single_mut() {
        t.translation = pos.with_z(pos.z + 1.0);
        if view.torchlit != state.lit {
            state.lit = view.torchlit;
            if view.torchlit {
                commands.entity(entity).insert(PointLight2d {
                    radius: 6.0 * T,
                    intensity: 1.5,
                    falloff: 10.0,
                    color: Color::srgb(1.0, 0.72, 0.42),
                    cast_shadows: true,
                    ..default()
                });
            } else {
                commands.entity(entity).remove::<PointLight2d>();
            }
        }
    }
    if let Ok((mut light2d, mut vignette)) = post.single_mut() {
        // Camera-scoped ambient: bright day, deep-blue night; the torch pool
        // rides on top of either.
        light2d.ambient_light = if view.night {
            AmbientLight2d {
                color: Color::srgb(0.55, 0.62, 0.85),
                brightness: if view.torchlit { 0.35 } else { 0.12 },
            }
        } else {
            AmbientLight2d {
                color: Color::srgb(1.0, 1.0, 1.0),
                brightness: 0.85,
            }
        };
        // Combat bubble: vignette stays NEUTRAL — a colored Vignette threshold
        // veils the whole frame (measured flat (47,18,23) everywhere, see E4
        // notes). The red read lives on hud.rs's CombatEdge strips instead.
        vignette.color = Color::BLACK;
        // Options gate the whole vignette and the combat-pulse term
        // separately (E12: flicker-free mode keeps the static frame).
        vignette.intensity = crate::options::vignette_intensity(
            &view_options,
            view.in_combat,
            time.elapsed_secs(),
        );
    }
}
