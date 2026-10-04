//! Terrain pass in both projections. `Square`: one quad per cell (E4 v1).
//! `Iso`: one anchored top-face sprite per cell + two lazy side sprites for
//! block tiles (Forest/DeepForest/Mountain/Wall — docs/ART_PIPELINE.md), z
//! banded south-over-north. Route-(a) per-tile light works in SIM resolution
//! and colours whatever the cell draws, so the spike math ports untouched.

use bevy::prelude::*;
use bevy_light_2d::prelude::*;
use laya_realms::model::{Pos, Tile};

use crate::atlas::{self, Atlas, Face};
use crate::palette;
use crate::projection::{Proj, T};
use crate::sim::{grid_dims, WorldView, Zone};

#[derive(Component)]
pub struct CellTag {
    pub col: u32,
    pub row: u32,
    /// Everything that drives this cell's visuals; changed → rewrite, else skip.
    key: CellKey,
    door: Option<Entity>,
}

/// Exposed wall face sibling (Left/Right), anchored with its top face.
#[derive(Component)]
pub struct IsoSide {
    #[allow(dead_code)] // stays tagged for the E6 pick-mapping pass
    face: Face,
    key: SideKey,
}

#[derive(Clone, Copy, PartialEq)]
struct SideKey {
    pos: Pos,
    key_kind: u8, // 0 unbound, 1 textured, 2 hidden
}

#[derive(Clone, Copy, PartialEq)]
struct CellKey {
    pos: Pos,
    showed: bool,
    tile: Tile,
    wall_neighbors: u8,
    near_wall: bool,
    strength_q: u8,
    remembered: bool,
    tex_kind: u8, // 0 fallback quad/diamond, 1 textured
    zone: Zone,
    door_pose: u8,
}

const CELL_VOID: CellKey = CellKey {
    pos: Pos::new(-1, -1),
    showed: false,
    tile: Tile::Grass,
    wall_neighbors: 0,
    near_wall: false,
    strength_q: 0,
    remembered: false,
    tex_kind: 255,
    zone: Zone::Town,
    door_pose: 0,
};

#[derive(Resource, Default)]
pub struct TileGrid {
    cols: u32,
    rows: u32,
    mode: Option<Proj>,
    /// Iso side entities per grid cell: [left, right].
    sides: std::collections::HashMap<(u32, u32), [Option<Entity>; 2]>,
    /// One overhanging tree sprite per woodland cell.
    trees: std::collections::HashMap<(u32, u32), Entity>,
    /// Trees drawn last frame, so cells that stop being woodland can be culled.
    tree_cells: std::collections::HashSet<(u32, u32)>,
}

/// The overhanging canopy drawn above a woodland tile. A 64px iso cell cannot
/// hold a tree — its ground diamond already covers rows 4..36, so a crown
/// raised inside that silhouette is hidden by it — so the volume lives in a
/// sprite on the props sheet, anchored to the tile like any other actor.
#[derive(Component)]
pub struct TreeSpr {
    #[allow(dead_code)]
    pos: Pos,
    key: &'static str,
}

/// Other heightfield families have no extruded tile-sized base.
fn is_block(tile: Tile) -> bool {
    tile == Tile::Wall
}

/// Position hash, avalanche-mixed, per the art-pipeline determinism doctrine
/// (docs/ART_PIPELINE.md).
fn position_hash(pos: Pos) -> u32 {
    let mut h = (pos.x as u32)
        .wrapping_mul(73856093)
        .wrapping_add((pos.y as u32).wrapping_mul(19349663));
    h ^= h >> 13;
    h.wrapping_mul(0x5bd1_e995) ^ (h >> 15)
}

/// Alternate-block swap-in (<Tile>.v2), half the map by position hash.
fn variant_chosen(pos: Pos) -> bool {
    position_hash(pos) & 1 == 1
}

/// States-key of the alternate block art (sheet v2 batch), if a variant was
/// baked for this tile.
fn block_variant_key(tile: Tile) -> Option<&'static str> {
    match tile {
        Tile::Mountain => Some("Mountain.v2"),
        Tile::DeepForest => Some("DeepForest.v2"),
        Tile::Forest => Some("Forest.v2"),
        Tile::Rock => Some("Rock.v2"),
        _ => None,
    }
}

/// Four deterministic water phases without allocating a key per visible cell.
const RIVER_PHASES: [&str; 4] = ["River.v2", "River.v3", "River.v4", "River.v5"];
const FORD_PHASES: [&str; 4] = ["Ford.v2", "Ford.v3", "Ford.v4", "Ford.v5"];

fn flow_variant_key(tile: Tile, pos: Pos) -> Option<&'static str> {
    let phase = (position_hash(pos) % 4) as usize;
    match tile {
        Tile::River => Some(RIVER_PHASES[phase]),
        Tile::Ford => Some(FORD_PHASES[phase]),
        _ => None,
    }
}

/// Static atlas keys: no formatting or allocation in the per-cell draw path.
fn material_key(zone: Zone, tile: Tile, pos: Pos) -> Option<&'static str> {
    let (wall, floors) = match zone {
        Zone::Town => ("Wall.town", ["Floor.town", "Floor.town2"]),
        Zone::Crypt => ("Wall.crypt", ["Floor.crypt", "Floor.crypt2"]),
        Zone::Sanctum => ("Wall.sanctum", ["Floor.sanctum", "Floor.sanctum2"]),
        Zone::Underkeep => ("Wall.underkeep", ["Floor.underkeep", "Floor.underkeep2"]),
        Zone::Cave => ("Wall.cave", ["Floor.cave", "Floor.cave2"]),
        Zone::Woodland => ("Wall.woodland", ["Floor.woodland", "Floor.woodland2"]),
        Zone::Arena => ("Wall.arena", ["Floor.arena", "Floor.arena2"]),
        Zone::Dock => ("Wall.dock", ["Floor.dock", "Floor.dock2"]),
    };
    match tile {
        Tile::Wall => Some(wall),
        Tile::Floor => Some(floors[(position_hash(pos) & 1) as usize]),
        _ => None,
    }
}

/// The same eight material families as `material_key`; resolved without
/// allocating keys during the per-cell pass.
fn door_key(zone: Zone, pos: Pos, open: bool, along_x: bool) -> &'static str {
    macro_rules! poses {
        ($zone:literal) => { [
            concat!("Door.", $zone, "0_closed_x"), concat!("Door.", $zone, "0_open_x"),
            concat!("Door.", $zone, "0_closed_y"), concat!("Door.", $zone, "0_open_y"),
            concat!("Door.", $zone, "1_closed_x"), concat!("Door.", $zone, "1_open_x"),
            concat!("Door.", $zone, "1_closed_y"), concat!("Door.", $zone, "1_open_y"),
        ] };
    }
    let keys = match zone {
        Zone::Town => poses!("town"), Zone::Crypt => poses!("crypt"),
        Zone::Sanctum => poses!("sanctum"), Zone::Underkeep => poses!("underkeep"),
        Zone::Cave => poses!("cave"), Zone::Woodland => poses!("woodland"),
        Zone::Arena => poses!("arena"), Zone::Dock => poses!("dock"),
    };
    keys[(position_hash(pos) as usize & 1) * 4 + usize::from(!along_x) * 2 + usize::from(open)]
}

/// (Re)spawn the window-grid when the viewport size or projection changes.
pub fn ensure(
    mut commands: Commands,
    proj: Res<Proj>,
    windows: Query<&Window>,
    mut grid: ResMut<TileGrid>,
    cells: Query<Entity, Or<(With<CellTag>, With<IsoSide>)>>,
) {
    let (cols, rows) = windows
        .single()
        .map(|w| grid_dims(w.width(), w.height()))
        .unwrap_or((86, 51));
    if grid.cols == cols && grid.rows == rows && grid.mode == Some(*proj) {
        return;
    }
    for entity in cells.iter() {
        commands.entity(entity).despawn();
    }
    grid.cols = cols;
    grid.rows = rows;
    grid.mode = Some(*proj);
    grid.sides.clear();
    for row in 0..rows {
        for col in 0..cols {
            commands.spawn((
                Sprite::from_color(Color::NONE, Vec2::splat(T)),
                Transform::from_xyz(0.0, -100_000.0, 0.0),
                Visibility::Hidden,
                CellTag {
                    col,
                    row,
                    key: CELL_VOID,
                    door: None,
                },
            ));
        }
    }
}


fn face_center(proj: Proj, pos: Pos, band_base: f32) -> Vec3 {
    match proj {
        Proj::Square => Proj::Square.world(pos, band_base),
        Proj::Iso => {
            let d = Proj::Iso.world(pos, band_base);
            Vec3::new(d.x, d.y + Proj::CELL_ANCHOR_DY, d.z)
        }
    }
}

/// Fallback diamond (iso without sheet art): rotated + squashed quad.
fn apply_diamond_fallback(sprite: &mut Sprite, transform: &mut Transform) {
    sprite.image = Handle::default();
    sprite.rect = None;
    sprite.custom_size = Some(Vec2::splat(T * 0.707));
    transform.rotation = Quat::from_rotation_z(std::f32::consts::FRAC_PI_4);
    transform.scale = Vec3::new(1.0, 0.5, 1.0);
}

/// Tree plate for a woodland cell: one of three silhouettes per family, picked
/// by position hash so a forest is not the same stamp repeated.
fn tree_key(tile: Tile, pos: Pos) -> &'static str {
    const FOREST: [&str; 3] = ["Tree1", "Tree2", "Tree3"];
    const DEEP: [&str; 3] = ["DeepTree1", "DeepTree2", "DeepTree3"];
    let set = if tile == Tile::DeepForest { &DEEP } else { &FOREST };
    set[(position_hash(pos) % 3) as usize]
}

/// Drawn height of a tree in world units: 1.55 tiles, so a stand of them
/// stands above the cell it is rooted in.
const TREE_H: f32 = T * 1.55;

/// Per-tree height factor. A stand drawn at one fixed size and tone reads as a
/// stamped hedge rather than a wood, so each tree takes its own size and
/// brightness off the same position hash the silhouette already uses.
fn tree_scale(tile: Tile, pos: Pos) -> f32 {
    const STEPS: [f32; 5] = [0.84, 0.93, 1.0, 1.09, 1.2];
    let base = STEPS[((position_hash(pos) >> 3) % 5) as usize];
    // The deep wood stands a little taller than the open forest.
    if tile == Tile::DeepForest {
        base * 1.06
    } else {
        base
    }
}

/// ±6% around the cell's own tier, so canopies do not flatten into one tone.
fn tree_tint(pos: Pos, base: Color) -> Color {
    let f = 0.94 + ((position_hash(pos) >> 8) % 5) as f32 * 0.03;
    let l = base.to_linear();
    Color::LinearRgba(LinearRgba::new(l.red * f, l.green * f, l.blue * f, l.alpha))
}

/// Cells that stay bare. The tile plate already paints undergrowth, so leaving
/// one in five empty keeps the crowns from sealing the stand into a solid wall
/// and lets the ground read between the trunks.
fn tree_bare(pos: Pos) -> bool {
    position_hash(pos) % 5 == 0
}

pub fn render(
    view: Res<WorldView>,
    proj: Res<Proj>,
    atlas: Option<Res<Atlas>>,
    images: Res<Assets<Image>>,
    mut commands: Commands,
    mut cells: Query<
        (Entity, &mut CellTag, &mut Sprite, &mut Transform, &mut Visibility),
        Without<IsoSide>,
    >,
    mut sides: Query<
        (Entity, &mut IsoSide, &mut Sprite, &mut Transform, &mut Visibility),
        Without<CellTag>,
    >,
    // Disjoint from both cell queries: a tree sprite carries neither tag.
    mut trees: Query<
        (&mut TreeSpr, &mut Sprite, &mut Transform, &mut Visibility),
        (Without<CellTag>, Without<IsoSide>),
    >,
    mut grid: ResMut<TileGrid>,
) {
    for (entity, mut tag, mut sprite, mut transform, mut visibility) in cells.iter_mut() {
        let index = (tag.row * view.cols + tag.col) as usize;
        let Some(cell) = view.cells.get(index) else { continue };
        let dist = cell
            .pos
            .x
            .abs_diff(view.player.x)
            .max(cell.pos.y.abs_diff(view.player.y)) as i32;
        let (strength, remembered) = palette::tier(dist, view.radius, cell.tile);
        // Iso alternate blocks (sheet v2 batch): coordinate-hashed swap-in per
        // map coordinate to break repeated macro mottle. Deterministic, no RNG.
        // Water resolves on all four of its phases rather than a coin flip, so a
        // long river does not repeat the same band pattern every two tiles.
        let variant_key = if *proj == Proj::Iso {
            material_key(view.zone, if cell.tile == Tile::Door { Tile::Floor } else { cell.tile }, cell.pos)
                .or_else(|| flow_variant_key(cell.tile, cell.pos))
                .or_else(|| variant_chosen(cell.pos).then(|| block_variant_key(cell.tile)).flatten())
                .filter(|key| atlas.as_deref().is_some_and(|a| atlas::iso_state_ref(a, &images, key).is_some()))
        } else {
            None
        };
        // Face Left borders the cell at y+1; Face Right borders x+1.
        // Shared masonry is interior to a run, not two visible cube sides.
        let wall_neighbors = if *proj == Proj::Iso && cell.tile == Tile::Wall {
            let south_y = if tag.row + 1 < view.rows {
                view.cells.get(index + view.cols as usize)
            } else {
                None
            };
            let south_x = if tag.col + 1 < view.cols {
                view.cells.get(index + 1)
            } else {
                None
            };
            u8::from(south_y.is_some_and(|next| next.explored && next.tile == Tile::Wall))
                | (u8::from(south_x.is_some_and(|next| next.explored && next.tile == Tile::Wall)) << 1)
        } else {
            0
        };
        let near_wall = *proj == Proj::Iso && cell.tile == Tile::Wall && dist <= 2;
        let door = (*proj == Proj::Square && cell.tile == Tile::Door)
            .then(|| door_key(view.zone, cell.pos, cell.door_open, cell.door_along_x));
        let tex = atlas.as_deref().and_then(|a| match *proj {
            Proj::Square => door.and_then(|key| atlas::square_state_ref(a, &images, key))
                .or_else(|| atlas::square_ref(a, &images, cell.tile)),
            Proj::Iso => {
                if let Some(v2) = variant_key {
                    atlas::iso_state_face_ref(a, &images, v2, Face::Top)
                } else {
                    atlas::iso_ref(a, &images, if cell.tile == Tile::Door { Tile::Floor } else { cell.tile }, Face::Top)
                }
            }
        });
        let tex_kind = tex.is_some() as u8;
        let door_prop = if *proj == Proj::Iso && cell.tile == Tile::Door {
            let pose = match (cell.door_open, cell.door_along_x) {
                (false, true) => "closed_x", (true, true) => "open_x",
                (false, false) => "closed_y", (true, false) => "open_y",
            };
            atlas.as_deref().and_then(|a| atlas::door_ref(a, &images, pose))
        } else { None };
        let key = CellKey {
            pos: cell.pos,
            showed: cell.explored,
            tile: cell.tile,
            wall_neighbors,
            near_wall,
            strength_q: (strength * 100.0).round() as u8,
            remembered,
            tex_kind,
            zone: view.zone,
            door_pose: u8::from(cell.door_open) | (u8::from(cell.door_along_x) << 1)
                | (u8::from(door_prop.is_some()) << 2),
        };
        if key == tag.key {
            continue;
        }
        tag.key = key;
        if *proj != Proj::Iso || cell.tile != Tile::Door || !cell.explored {
            if let Some(door_ent) = tag.door.take() {
                commands.entity(door_ent).despawn();
            }
        }
        if !cell.explored {
            *visibility = Visibility::Hidden;
            commands.entity(entity).remove::<LightOccluder2d>();
            update_sides(
                &mut grid,
                &mut sides,
                &mut commands,
                atlas.as_deref(),
                &images,
                (tag.col, tag.row),
                None,
            );
            continue;
        }
        *visibility = Visibility::Visible;
        let band = Proj::tile_band(cell.pos);
        let z = match *proj {
            Proj::Square => 0.0,
            Proj::Iso => band,
        };
        transform.translation = face_center(*proj, cell.pos, z);
        // Masonry stands on its cell. The cap rides `ISO_WALL_LIFT` above the
        // diamond it would otherwise occupy and the baked faces fill the gap
        // down to the ground edge, so a wall reads as a lit block instead of a
        // recessed ribbon. Faces already carry the same rise.
        if *proj == Proj::Iso && cell.tile == Tile::Wall {
            transform.translation.y += Proj::ISO_WALL_LIFT;
        }
        // Ground tint: plate = unlit albedo → light-only white tint; fallback
        let mut color = if tex_kind == 1 {
            palette::lit((255, 255, 255), strength, remembered)
        } else {
            palette::lit(palette::terrain_rgb(cell.tile), strength, remembered)
        };
        let unfaded = color;
        if near_wall {
            // Enough to see the hero through a parapet, not so much that the
            // nearest wall reads as missing.
            color.set_alpha(0.72);
        }
        sprite.color = color;
        if let Some((image, rect)) = door_prop {
            // Door feet A=(48,64) on a 96px plate; T*1.5 renders its 40px
            // rise at the same height as masonry. Child keeps normal tile z.
            let door_sprite = Sprite {
                image: image.clone(), rect: Some(rect), color: unfaded,
                custom_size: Some(Vec2::splat(T * 1.5)), ..default()
            };
            // The leaf belongs on the exposed facade, not the centre of the
            // thick wall footprint. X-runs meet the south (+y/2) edge; Y-runs
            // meet the east (+x/2) edge. Project that half-cell displacement
            // while retaining the ordinary prop depth within this tile band.
            let facade_x = if cell.door_along_x { -T * 0.25 } else { T * 0.25 };
            let door_transform = Transform::from_xyz(facade_x, 14.0 - T * 0.125, Proj::Z_PROP);
            if let Some(child) = tag.door {
                commands.entity(child).insert((door_sprite, door_transform));
            } else {
                let child = commands.spawn((door_sprite, door_transform, Visibility::Inherited)).id();
                commands.entity(entity).add_child(child);
                tag.door = Some(child);
            }
        }
        match tex {
            Some((image, rect)) => {
                sprite.image = image.clone();
                sprite.rect = Some(rect);
                // Exactly T, in both projections. Any bleed here (`T + 1.0`
                // for a while) breaks iso tiling: neighbouring diamonds are
                // 16 units apart, so a 33-unit diamond no longer shares an
                // edge with its neighbours and the two antialiased boundaries
                // cross in a sub-pixel sliver — the lattice that survived
                // every attempt to flatten the ground. Fringe coverage is the
                // bake's job (`paint::seam_iso` bleeds the plate's own
                // colour), not the sprite's.
                sprite.custom_size = Some(Vec2::splat(T));
                // A reflected canopy doubles the two baked forest silhouettes
                // without changing their level ground boundary or tile anchor.
                sprite.flip_x = *proj == Proj::Iso
                    && matches!(cell.tile, Tile::Forest | Tile::DeepForest)
                    && position_hash(cell.pos) & 2 != 0;
                transform.rotation = Quat::IDENTITY;
                transform.scale = Vec3::new(1.0, 1.0, 1.0);
            }
            None => {
                sprite.flip_x = false;
                if *proj == Proj::Iso {
                    apply_diamond_fallback(&mut sprite, &mut transform);
                } else {
                    sprite.image = Handle::default();
                    sprite.rect = None;
                    sprite.custom_size = Some(Vec2::splat(T));
                    transform.rotation = Quat::IDENTITY;
                    transform.scale = Vec3::new(1.0, 1.0, 1.0);
                }
            }
        }
        // Route (b) occluders: tall tiles block the torch pool (square; iso
        // approximation shares the same footprint for v1).
        let occludes = (matches!(cell.tile, Tile::Wall | Tile::Rock | Tile::Mountain)
            || (cell.tile == Tile::Door && !cell.door_open))
            && !remembered
            && strength > 0.3;
        if occludes {
            commands
                .entity(entity)
                .insert(LightOccluder2d {
                    shape: LightOccluder2dShape::Rectangle {
                        half_size: Vec2::splat(T * 0.45),
                    },
                });
        } else {
            commands.entity(entity).remove::<LightOccluder2d>();
        }
        // Only exposed masonry faces are extruded in iso.
        update_sides(
            &mut grid,
            &mut sides,
            &mut commands,
            atlas.as_deref(),
            &images,
            (tag.col, tag.row),
            if *proj == Proj::Iso && cell.explored && is_block(cell.tile) {
                Some(SideState {
                    pos: cell.pos,
                    tile: cell.tile,
                    // The face keeps the unfaded tier even where the cap above
                    // it is faded for the player's benefit: washing the face
                    // out too is what made the room's own wall disappear.
                    tint: unfaded,
                    band,
                    neighbors: wall_neighbors,
                    variant_key,
                })
            } else {
                None
            },
        );
        // Woodland carries its volume in an overhanging sprite rather than in
        // the cell: see `TreeSpr`. Drawn at the cell's own band so the tiles
        // behind a stand of trees draw first and the ones in front clip it.
        // Visibility matches the ground exactly — gating on the sight radius
        // left the far half of a revealed forest as bare green carpet.
        let woodland = *proj == Proj::Iso
            && cell.explored
            && matches!(cell.tile, Tile::Forest | Tile::DeepForest)
            && !tree_bare(cell.pos);
        if woodland {
            grid.tree_cells.insert((tag.col, tag.row));
            upsert_tree(
                &mut grid,
                &mut commands,
                &mut trees,
                atlas.as_deref(),
                &images,
                (tag.col, tag.row),
                cell.pos,
                cell.tile,
                band,
                unfaded,
            );
        }
    }
    // Cells that stopped being woodland drop their canopy.
    let live: Vec<(u32, u32)> = grid.tree_cells.iter().copied().collect();
    for coord in live {
        let still = view
            .cells
            .get((coord.1 * view.cols + coord.0) as usize)
            .is_some_and(|c| {
                c.explored
                    && matches!(c.tile, Tile::Forest | Tile::DeepForest)
                    && !tree_bare(c.pos)
            });
        if !still {
            if let Some(entity) = grid.trees.remove(&coord) {
                commands.entity(entity).despawn();
            }
            grid.tree_cells.remove(&coord);
        }
    }
}

fn upsert_tree(
    grid: &mut TileGrid,
    commands: &mut Commands,
    trees: &mut Query<
        (&mut TreeSpr, &mut Sprite, &mut Transform, &mut Visibility),
        (Without<CellTag>, Without<IsoSide>),
    >,
    atlas: Option<&Atlas>,
    images: &Assets<Image>,
    coord: (u32, u32),
    pos: Pos,
    tile: Tile,
    band: f32,
    tint: Color,
) {
    let key = tree_key(tile, pos);
    let Some((image, rect)) = atlas.and_then(|a| atlas::prop_ref(a, images, key)) else {
        return;
    };
    // Bottom-anchored on the tile: the sprite is centre-anchored, so lift it
    // by half its drawn height and the trunk foot lands on the anchor point.
    // `Iso::world`'s second argument is z, NOT a y offset — feeding it
    // `CELL_ANCHOR_DY` (a y correction belonging in `face_center`) parked every
    // tree at z = band - 5.985, six units under the lowest terrain band, so the
    // map drew over them and only the crowns poked out past its edge.
    let h = TREE_H * tree_scale(tile, pos);
    let tint = tree_tint(pos, tint);
    let translation = Vec3::new(0.0, h * 0.5, 0.0) + Proj::Iso.world(pos, band + Proj::Z_TREE);
    let rect = Some(rect);
    if let Some(entity) = grid.trees.remove(&coord) {
        if let Ok((mut tag, mut sprite, mut transform, mut visibility)) = trees.get_mut(entity) {
            if tag.key != key {
                tag.key = key;
                sprite.image = image.clone();
                sprite.rect = rect;
            }
            *visibility = Visibility::Visible;
            sprite.custom_size = Some(Vec2::splat(h));
            sprite.color = tint;
            transform.translation = translation;
        }
        grid.trees.insert(coord, entity);
        return;
    }
    let entity = commands
        .spawn((
            Sprite {
                image: image.clone(),
                rect,
                color: tint,
                custom_size: Some(Vec2::splat(h)),
                ..default()
            },
            Transform::from_translation(translation),
            Visibility::Visible,
            TreeSpr { pos, key },
        ))
        .id();
    grid.trees.insert(coord, entity);
 }

struct SideState {
    pos: Pos,
    tile: Tile,
    tint: Color,
    band: f32,
    neighbors: u8,
    variant_key: Option<&'static str>,
}

/// Exposed wall faces only. A neighbouring wall consumes the corresponding
/// side, while adjacent walkable cells retain the short masonry silhouette.
fn update_sides(
    grid: &mut TileGrid,
    sides: &mut Query<
        (Entity, &mut IsoSide, &mut Sprite, &mut Transform, &mut Visibility),
        Without<CellTag>,
    >,
    commands: &mut Commands,
    atlas: Option<&Atlas>,
    images: &Assets<Image>,
    coord: (u32, u32),
    state: Option<SideState>,
) {
    let mut slots = grid.sides.remove(&coord).unwrap_or([None, None]);
    for (face_slot, face) in [Face::Left, Face::Right].into_iter().enumerate() {
        let rect = state
            .as_ref()
            .filter(|s| s.neighbors & (1 << face_slot) == 0)
            .and_then(|s| atlas.and_then(|a| match s.variant_key {
                Some(key) => atlas::iso_state_face_ref(a, images, key, face),
                None => atlas::iso_ref(a, images, s.tile, face),
            }))
            .map(|(image, rect)| (image.clone(), rect));
        upsert_side(
            commands,
            sides,
            slots.get_mut(face_slot).expect("slot exists"),
            rect,
            SideSet {
                state: state.as_ref(),
                z_up: if face_slot == 0 { 0.001 } else { 0.002 },
                face,
            },
        );
    }
    if slots.iter().any(Option::is_some) {
        grid.sides.insert(coord, slots);
    }
}

struct SideSet<'a> {
    state: Option<&'a SideState>,
    z_up: f32,
    face: Face,
}

fn upsert_side(
    commands: &mut Commands,
    sides: &mut Query<
        (Entity, &mut IsoSide, &mut Sprite, &mut Transform, &mut Visibility),
        Without<CellTag>,
    >,
    slot: &mut Option<Entity>,
    rect: Option<(Handle<Image>, Rect)>,
    set: SideSet,
) {
    let Some((image, rect)) = rect else {
        if let Some(old) = slot.take() {
            commands.entity(old).despawn();
        }
        return;
    };
    let Some(s) = set.state else {
        if let Some(old) = slot.take() {
            commands.entity(old).despawn();
        }
        return;
    };
    // A wall's south face hangs INTO the cell to its south, so it has to paint
    // after that cell's ground. Banding it on the wall's own row put it under
    // the floor tile and hid every wall face in the game.
    let z = (s.band + Proj::Z_WALL_FACE) + set.z_up;
    let d = Proj::Iso.world(s.pos, z);
    // The bake paints the face art `WALL_FACE_DROP` lower in its cell than it
    // belongs, which is what lets a 40px face fit a 64px cell; lift the sprite
    // back so the face's top edge meets the raised cap.
    let translation = Vec3::new(
        d.x,
        d.y + Proj::CELL_ANCHOR_DY + Proj::ISO_WALL_FACE_LIFT,
        d.z,
    );
    let side_key = SideKey {
        pos: s.pos,
        key_kind: 1,
    };
    if let Some(entity) = slot.take() {
        if let Ok((_, mut tag, mut sprite, mut transform, mut visibility)) =
            sides.get_mut(entity)
        {
            if tag.key != side_key {
                tag.key = side_key;
            }
            *visibility = Visibility::Visible;
            sprite.image = image;
            sprite.rect = Some(rect);
            sprite.custom_size = Some(Vec2::splat(T));
            sprite.color = s.tint;
            transform.translation = translation;
        }
        *slot = Some(entity);
        return;
    }
    *slot = Some(
        commands
            .spawn((
                Sprite {
                    image,
                    rect: Some(rect),
                    color: s.tint,
                    custom_size: Some(Vec2::splat(T)),
                    ..default()
                },
                Transform::from_translation(translation),
                IsoSide { face: set.face, key: side_key },
            ))
            .id(),
    );
}
