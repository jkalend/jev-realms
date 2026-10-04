//! Tactical camera: smooth follow on the player + wheel zoom, world-locked.
//! px/tile lives in [16, 64] (D2 density lesson, §7.4-7: more world,
//! not bigger pixels); default 40 ≈ 32 tiles across at 1280px.

use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
use bevy::prelude::*;
use laya_realms::model::Pos;

use crate::input::InputState;
use crate::projection::{Proj, T};
use crate::sim::WorldView;

pub const ZOOM_MIN: f32 = 16.0;
pub const ZOOM_MAX: f32 = 64.0;
pub const ZOOM_DEFAULT: f32 = 48.0;

#[derive(Component)]
pub struct MainCam;

#[derive(Resource)]
pub struct CamZoom {
    pub target: f32,
}

impl Default for CamZoom {
    fn default() -> Self {
        Self {
            target: ZOOM_DEFAULT,
        }
    }
}

/// Last resolved camera pose: world anchor + world-units-per-pixel (the ortho
/// scale). World-space chrome (the HUD) pins itself to this per frame.
/// `anticipate` is the momentum offset (≤0.4 tile) lerped toward the held
/// step direction and back to zero when movement stops.
#[derive(Resource, Default)]
pub struct CamFrame {
    pub pos: Vec2,
    pub scale: f32,
    pub anticipate: Vec2,
}

pub fn follow(
    time: Res<Time>,
    view: Res<WorldView>,
    proj: Res<Proj>,
    input: Option<Res<InputState>>,
    pane: Res<crate::pointer::PointerIns>,
    slot: Res<crate::sim::SimSlot>,
    mut zoom: ResMut<CamZoom>,
    mut frame: ResMut<CamFrame>,
    mut wheel: MessageReader<MouseWheel>,
    mut cams: Query<(&mut Transform, &mut Projection), With<MainCam>>,
) {
    for event in wheel.read() {
        if !matches!(slot.game.modal, laya_realms::model::Modal::None | laya_realms::model::Modal::Pause) {
            continue;
        }
        let step = match event.unit {
            MouseScrollUnit::Line => 1.12_f32.powf(event.y),
            MouseScrollUnit::Pixel => 1.002_f32.powf(event.y),
        };
        zoom.target = (zoom.target * step).clamp(ZOOM_MIN, ZOOM_MAX);
    }
    let Ok((mut transform, mut projection)) = cams.single_mut() else {
        return;
    };
    let dt = time.delta_secs();
    // Momentum offset: anticipate up to 0.4 tile toward the held direction;
    // eased out and cancelled (lerped to zero) when movement stops.
    let anti_target = input
        .and_then(|i| i.held_dir())
        .map(|(dx, dy)| proj.world(Pos::new(dx, dy), 0.0).truncate() * 0.4)
        .unwrap_or(Vec2::ZERO);
    let ka = 1.0 - (-8.0 * dt).exp();
    frame.anticipate = frame.anticipate.lerp(anti_target, ka);
    // pane: E6 edge-pan / two-finger-drag offset rides on the player anchor
    // (temporary: pointer.rs decays it back to zero on its own).
    let target = proj.world(view.player, transform.translation.z)
        + (frame.anticipate + pane.pan).extend(0.0);
    let k = 1.0 - (-10.0 * dt).exp(); // ~63% of the way every 100 ms
    transform.translation = transform.translation.lerp(target, k);
    if let Projection::Orthographic(ortho) = &mut *projection {
        let target_scale = T / zoom.target;
        let s = 1.0 - (-8.0 * dt).exp();
        ortho.scale += (target_scale - ortho.scale) * s;
        frame.scale = ortho.scale;
    }
    frame.pos = transform.translation.truncate();
}
