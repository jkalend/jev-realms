//! Floating text: `game.effects` (pos, text, ttl) rendered as world-space
//! `Text2d`, reconciled against the snapshot each frame and faded by ttl.
//! Sim ticks decay ttl (4 → 0, ~1 s at 4 Hz); alpha tracks it directly.
//! Bundled default font (FiraMono subset) — the gothic display face is E4-s4.

use bevy::prelude::*;
use laya_realms::model::Pos;

use crate::palette;
use crate::projection::Proj;
use crate::sim::WorldView;

const TTL_FULL: f32 = 4.0; // engine.rs pushes ttl: 4

#[derive(Component)]
pub struct Floating {
    pos: Pos,
    text: String,
}

pub fn render(
    view: Res<WorldView>,
    proj: Res<Proj>,
    mut commands: Commands,
    mut texts: Query<(Entity, &mut Text2d, &mut TextColor, &mut Transform, &Floating)>,
) {
    // Despawn what's gone, re-anchor what survives.
    for (entity, _text, _color, _transform, tag) in texts.iter_mut() {
        if !view
            .effects
            .iter()
            .any(|(pos, text, _)| *pos == tag.pos && *text == tag.text)
        {
            commands.entity(entity).despawn();
        }
    }
    for (pos, text, ttl) in &view.effects {
        let drift_up = (TTL_FULL - *ttl as f32).max(0.0) * 6.0;
        let alpha = (*ttl as f32 / TTL_FULL).clamp(0.0, 1.0);
        if let Some((_, _, mut color, mut transform, _)) = texts
            .iter_mut()
            .find(|(_, _, _, _, tag)| tag.pos == *pos && tag.text == *text)
        {
            transform.translation = proj.world(*pos, 20.0) + Vec3::new(0.0, drift_up, 0.0);
            let (r, g, b) = palette::INK_LIGHT_YELLOW;
            color.0 = Color::srgba_u8(r, g, b, (alpha * 255.0) as u8);
        } else {
            let (r, g, b) = palette::INK_LIGHT_YELLOW;
            commands.spawn((
                Text2d::new(text.clone()),
                TextFont::from_font_size(14.0),
                TextLayout::justify(Justify::Center),
                TextColor(Color::srgba_u8(r, g, b, (alpha * 255.0) as u8)),
                Transform::from_translation(proj.world(*pos, 20.0) + Vec3::new(0.0, drift_up, 0.0)),
                Floating {
                    pos: *pos,
                    text: text.clone(),
                },
            ));
        }
    }
}
