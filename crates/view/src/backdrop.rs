//! Environmental atmospheric backdrop system.
//!
//! Replaces the unlit pitch-black void outside the map boundaries with rich
//! zone-specific skies, cavern abysses, and celestial horizons, while synchronizing
//! Bevy's ClearColor so no raw ink black ever leaks.

use bevy::prelude::*;

use crate::camera::CamFrame;
use crate::sim::{WorldView, Zone};

#[derive(Component)]
pub struct BackdropSprite;

#[derive(Resource)]
pub struct BackdropAssets {
    pub town_day: Handle<Image>,
    pub town_night: Handle<Image>,
    pub cave: Handle<Image>,
    pub crypt: Handle<Image>,
    pub underkeep: Handle<Image>,
    pub arena: Handle<Image>,
    pub dock: Handle<Image>,
    pub sanctum: Handle<Image>,
}

pub fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let assets = BackdropAssets {
        town_day: asset_server.load("backdrops/town_day.png"),
        town_night: asset_server.load("backdrops/town_night.png"),
        cave: asset_server.load("backdrops/cave.png"),
        crypt: asset_server.load("backdrops/crypt.png"),
        underkeep: asset_server.load("backdrops/underkeep.png"),
        arena: asset_server.load("backdrops/arena.png"),
        dock: asset_server.load("backdrops/dock.png"),
        sanctum: asset_server.load("backdrops/sanctum.png"),
    };

    commands.spawn((
        Sprite {
            image: assets.town_day.clone(),
            custom_size: Some(Vec2::splat(2048.0)),
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, -50.0),
        Visibility::Visible,
        BackdropSprite,
    ));

    commands.insert_resource(assets);
}

pub fn update(
    view: Res<WorldView>,
    frame: Res<CamFrame>,
    assets: Option<Res<BackdropAssets>>,
    mut clear_color: ResMut<ClearColor>,
    mut query: Query<(&mut Sprite, &mut Transform), With<BackdropSprite>>,
) {
    let Some(assets) = assets else { return };
    let Ok((mut sprite, mut transform)) = query.single_mut() else { return };

    // Pin backdrop to camera position with generous margins to completely cover any aspect ratio
    transform.translation = frame.pos.extend(-50.0);
    let view_width = 2400.0 * frame.scale.max(0.4);
    let view_height = 1350.0 * frame.scale.max(0.4);
    sprite.custom_size = Some(Vec2::new(view_width, view_height));

    let (handle, horizon_color) = match view.zone {
        Zone::Town | Zone::Woodland => {
            if view.night {
                (&assets.town_night, Color::srgb(0.04, 0.05, 0.09))
            } else {
                (&assets.town_day, Color::srgb(0.24, 0.20, 0.18))
            }
        }
        Zone::Cave => (&assets.cave, Color::srgb(0.06, 0.04, 0.04)),
        Zone::Crypt => (&assets.crypt, Color::srgb(0.03, 0.06, 0.06)),
        Zone::Underkeep => (&assets.underkeep, Color::srgb(0.03, 0.05, 0.07)),
        Zone::Arena => (&assets.arena, Color::srgb(0.04, 0.03, 0.07)),
        Zone::Dock => (&assets.dock, Color::srgb(0.04, 0.07, 0.09)),
        Zone::Sanctum => (&assets.sanctum, Color::srgb(0.05, 0.03, 0.08)),
    };

    if sprite.image != *handle {
        sprite.image = handle.clone();
    }
    clear_color.0 = horizon_color;
}
