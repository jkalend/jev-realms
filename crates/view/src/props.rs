//! Prop decals (v1): portals → `TorchBrazier`, Shrine tiles → `RuneStone`,
//! Chest tiles → `RelicPedestal`. v1 mapping is deliberate; `Potion` is
//! unwired until loot/stash surfaces exist (E6+). Props sit on the terrain
//! grid (z 1), tinted by the same sim light tiers; missing keys fall back to
//! small palette quads.

use bevy::prelude::*;
use laya_realms::model::{Pos, Tile};

use crate::atlas::{self, Atlas};
use crate::palette;
use crate::projection::{Proj, T};
use crate::sim::WorldView;

#[derive(Component)]
pub struct Prop {
    pos: Pos,
    key: &'static str,
    kind_tex: u8, // 0 unset, 1 texture, 2 quad
}

const PROPS: &[(&str, (u8, u8, u8))] = &[
    ("TorchBrazier", (232, 146, 64)),
    ("RuneStone", (148, 190, 208)),
    ("RelicPedestal", (198, 176, 118)),
];

fn fallback_rgb(key: &str) -> (u8, u8, u8) {
    PROPS
        .iter()
        .find(|(k, _)| *k == key)
        .map(|(_, c)| *c)
        .unwrap_or((200, 200, 200))
}

pub fn render(
    view: Res<WorldView>,
    proj: Res<Proj>,
    atlas: Option<Res<Atlas>>,
    images: Res<Assets<Image>>,
    mut commands: Commands,
    mut props: Query<(Entity, &mut Prop, &mut Sprite, &mut Transform, &mut Visibility)>,
) {
    // Wanted set: portals (braziers) + shrine/chest cells, explored only.
    let mut wanted: Vec<(Pos, &str)> = Vec::new();
    for &pos in &view.portals {
        wanted.push((pos, "TorchBrazier"));
    }
    for cell in view.cells.iter() {
        if !cell.explored {
            continue;
        }
        let key = match cell.tile {
            Tile::Shrine => "RuneStone",
            Tile::Chest => "RelicPedestal",
            _ => continue,
        };
        wanted.push((cell.pos, key));
    }
    for (entity, prop, ..) in props.iter_mut() {
        if !wanted.iter().any(|(pos, key)| *pos == prop.pos && *key == prop.key) {
            commands.entity(entity).despawn();
        }
    }
    for (pos, key) in wanted {
        if let Some((_, mut tag, mut sprite, mut transform, mut visibility)) =
            props.iter_mut().find(|(_, p, ..)| p.pos == pos && p.key == key)
        {
            let dist = pos.distance(view.player);
            if dist > view.radius {
                *visibility = Visibility::Hidden;
                continue;
            }
            *visibility = Visibility::Visible;
            let z = match *proj {
                Proj::Iso => Proj::Z_PROP + Proj::tile_band(pos),
                Proj::Square => Proj::Z_PROP,
            };
            transform.translation = proj.world(pos, z);
            let tex = atlas.as_deref().and_then(|a| atlas::prop_ref(a, &images, key));
            // (Re)bind the texture variant only on change; tint every frame.
            match (tex, tag.kind_tex) {
                (Some((image, rect)), current) if current != 1 => {
                    tag.kind_tex = 1;
                    sprite.image = image.clone();
                    sprite.rect = Some(rect);
                    sprite.custom_size = Some(Vec2::splat(T * 0.9));
                }
                (None, current) if current != 2 => {
                    tag.kind_tex = 2;
                    sprite.image = Handle::default();
                    sprite.rect = None;
                    sprite.custom_size = Some(Vec2::splat(T * 0.45));
                }
                _ => {}
            }
            sprite.color = if tag.kind_tex == 1 {
                crate::actors::tier_tint(&view, pos, Tile::Grass)
            } else {
                let (s, _) = palette::tier(dist, view.radius, Tile::Grass);
                palette::lit(fallback_rgb(key), s, false)
            };
            continue;
        }
        commands.spawn((
            Sprite::from_color(Color::NONE, Vec2::splat(T * 0.45)),
            Transform::from_xyz(0.0, -100_000.0, 1.0),
            Visibility::Hidden,
            Prop {
                pos,
                key,
                kind_tex: 0,
            },
        ));
    }
}
