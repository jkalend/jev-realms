//! Projection seam: one enum, one transform per mode.
//! `Square` — the E4 read (D25 evidence A). `Iso` — the D2 diamond flip:
//! `screen_x = (x−y)·T/2`, `screen_y = (x+y)·T/4` (2:1 diamonds, D25-B).
//! Everything render-side (tiles, actors, props, lights, camera anchor)
//! positions through [`Proj::world`], never through a duplicated formula.

use bevy::prelude::*;
use laya_realms::model::Pos;

/// World units per tile (E0 spike constant; px/tile = T / camera scale).
pub const T: f32 = 32.0;

#[derive(Resource, Clone, Copy, PartialEq, Eq, Default)]
pub enum Proj {
    #[default]
    Square,
    Iso,
}

impl Proj {
    /// Tile centre in world coordinates (Iso: the diamond centre = sheet
    /// anchor A). z passes through untouched.
    pub fn world(self, pos: Pos, z: f32) -> Vec3 {
        match self {
            Proj::Square => Vec3::new(pos.x as f32 * T, -(pos.y as f32) * T, z),
            Proj::Iso => Vec3::new(
                (pos.x - pos.y) as f32 * (T * 0.5),
                -(pos.x + pos.y) as f32 * (T * 0.25),
                z,
            ),
        }
    }

    /// Iso face-cell anchor offset: the sheet cells pin their A=(32,20)
    /// anchor to the diamond centre, so the 64px cell centre (32,32) sits
    /// (0, (32−20)px = 12px) above it = +6 world units. In other words the
    /// quad centre lands 6 units BELOW the diamond centre.
    pub const CELL_ANCHOR_DY: f32 = -6.0;

    /// Iso masonry rise in SHEET px, shared with `bake3d::WALL_FACE_H`. The
    /// bake drops the face art down its cell by `ISO_WALL_FACE_DROP_PX` so a
    /// face taller than the cell's own diamond can still fit, and the view
    /// lifts the face sprite back by the same amount; the cap rises by the
    /// full face height. All three must move together or a transparent band
    /// opens between cap and face.
    const ISO_WALL_FACE_PX: f32 = 40.0;
    const ISO_WALL_FACE_DROP_PX: f32 = 24.0;
    /// Wall cap lift in world units (a 64px sheet cell is `T` units).
    pub const ISO_WALL_LIFT: f32 = Self::ISO_WALL_FACE_PX * T / 64.0;
    /// Compensation for the bake's down-shifted face art.
    pub const ISO_WALL_FACE_LIFT: f32 = Self::ISO_WALL_FACE_DROP_PX * T / 64.0;

    /// Sprite z layers, all added on top of [`Proj::tile_band`]. They share ONE
    /// band per tile so a wall can occlude an actor standing behind it: with
    /// terrain at 0.0x and actors parked at a flat 2.0, no terrain ever drew
    /// over a character, which is what let NPCs cut through walls.
    ///
    /// Ordering within a tile, back to front:
    /// top face < wall face < tree < prop < actor < effect.
    /// Between tiles the band's 0.01 step dominates, and every layer here stays
    /// under that step, so an object on a nearer tile always wins.
    pub const Z_WALL_FACE: f32 = 0.010;
    pub const Z_TREE: f32 = 0.015;
    pub const Z_PROP: f32 = 0.016;
    pub const Z_ACTOR: f32 = 0.020;
    /// Chest-pendant overlay on an actor's own tile.
    pub const Z_ACTOR_OVERLAY: f32 = 0.021;
    pub const Z_EFFECT: f32 = 0.025;

    /// Iso z band: southern rows occlude northern ones (`tx+ty` ascending
    /// forward), faces inside one cell separated by an epsilon. Actors/props
    /// add a tile-band offset on top of the terrain base (see callers).
    pub fn tile_band(pos: Pos) -> f32 {
        (pos.x + pos.y) as f32 * 0.01
    }
}

/// Iso world → approximation tile coordinates (the inverse of
/// [`Proj::Iso`]'s transform). Landed now so E6's mouse pick-mapping has the
/// exact inversion E4 validated, not a later re-derivation.
pub fn iso_to_tile(wx: f32, wy: f32) -> Pos {
    let wy = -wy;
    // wx = (tx−ty)·T/2, wy = (tx+ty)·T/4  →  tx−ty = 2wx/T, tx+ty = 4wy/T
    let diff = 2.0 * wx / T;
    let sum = 4.0 * wy / T;
    Pos::new(((sum + diff) * 0.5).round() as i32, ((sum - diff) * 0.5).round() as i32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iso_round_trip() {
        for (x, y) in [(0, 0), (5, 2), (-3, 7), (11, -6), (31, 31)] {
            let p = Pos::new(x, y);
            let w = Proj::Iso.world(p, 0.0);
            assert_eq!(iso_to_tile(w.x, w.y), p, "round trip ({x},{y})");
        }
    }
}
