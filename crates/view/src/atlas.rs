//! ATLAS HOOK (forward-compatible, contract with the tools/atlas lane).
//!
//! Reads `assets/atlas/manifest.json` v1 when it exists (docs/ART_PIPELINE.md).
//! Sheets used by the square projection: terrain-square, actors (+states
//! groups keyed "<Group>.<variant>", e.g. "Player.Keepwarden", "PaleStag.at_bay"),
//! props. Keys are Rust Debug names (`format!("{:?}")`). Manifest absent, sheet
//! unreadable, or key missing → that element falls back to color quads, silently.

use bevy::image::{ImageLoaderSettings, ImageSampler, ImageSamplerDescriptor};
use bevy::prelude::*;
use laya_realms::model::Tile;
use serde::Deserialize;
use std::collections::HashMap;

pub const MANIFEST_PATH: &str = "assets/atlas/manifest.json";
pub const SHEET_TERRAIN_SQUARE: &str = "terrain-square";
pub const SHEET_TERRAIN_ISO: &str = "terrain-iso";
pub const SHEET_ACTORS: &str = "actors";
const SHEET_PLAYERS: &str = "players";
const SHEET_RELICS: &str = "player_relics";
pub const SHEET_PROPS: &str = "props";
pub const SHEET_CHROME: &str = "chrome";

/// The three iso face rows, in manifest `faces` order (doc-fixed ["top",
/// "left", "right"]). Composite sides-then-top at the shared A=(32,20) anchor.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Face {
    Top,
    Left,
    Right,
}
impl Face {
    pub const ALL: [Face; 3] = [Face::Left, Face::Right, Face::Top];
    fn from_name(name: &str) -> Option<Self> {
        match name {
            "top" => Some(Face::Top),
            "left" => Some(Face::Left),
            "right" => Some(Face::Right),
            _ => None,
        }
    }
    fn index(self) -> usize {
        match self {
            Face::Top => 0,
            Face::Left => 1,
            Face::Right => 2,
        }
    }
}

/// Workspace-rooted path fallback (matches `ensure_asset_root` in lib.rs):
/// CWD-relative first (the contract path), then ../.. from this crate.
fn manifest_path() -> std::path::PathBuf {
    let contract = std::path::Path::new(MANIFEST_PATH);
    if contract.exists() {
        return contract.to_path_buf();
    }
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("../../{MANIFEST_PATH}"))
}

#[derive(Debug, Deserialize)]
struct Manifest {
    tile_size: u32,
    sheets: Vec<SheetDecl>,
}

#[derive(Debug, Deserialize)]
struct SheetDecl {
    name: String,
    file: String,
    mapping: Option<HashMap<String, (u32, u32)>>,
    /// Cell edge for this sheet, in pixels. Defaults to the manifest's global
    /// `tile_size`; a sheet declares its own when its plates are authored at a
    /// different resolution (the 3D actor sheet is 96px, because a 64px cell
    /// cannot hold a character that draws larger than one tile).
    cell_size: Option<u32>,
    faces: Option<Vec<String>>,
    states: Option<HashMap<String, HashMap<String, (u32, u32)>>>,
}

/// One loaded sheet: image handle + Debug-name → cell mapping, pre-resolved to
/// texture rects per tile variant so the per-frame pass never formats strings.
pub struct Sheet {
    pub image: Handle<Image>,
    pub tiles: Vec<Option<Rect>>, // indexed by `tile as usize`
    pub states_cells: HashMap<String, Rect>,
}

/// Generic string-keyed sheet (actors incl. "Player.<Class>" flats, props).
pub struct ImageSheet {
    pub image: Handle<Image>,
    pub cells: HashMap<String, Rect>,
}

/// Iso sheet: per-tile three face rects (top/left/right), indexed
/// `tile as usize`, plus flattened `states.<Member>.<Name>` cells
/// ("Wall.tall", "<Tile>.v2") per the manifest states convention.
pub struct IsoSheet {
    pub image: Handle<Image>,
    pub faces: Vec<[Option<Rect>; 3]>, // [tile][Face as usize-order Top,Left,Right]
    pub states_cells: std::collections::HashMap<String, Rect>,
}

/// None for a sheet until the manifest exists AND its image decodes. Checked
/// per frame, so atlases dropped in while a dev session runs get picked up.
#[derive(Resource, Default)]
pub struct Atlas {
    pub terrain_square: Option<Sheet>,
    pub terrain_iso: Option<IsoSheet>,
    pub actors: Option<ImageSheet>,
    /// Player class/build plates, keyed `Player.<Class>.<Build>`
    /// (docs/PLAYER_PIPELINE.md). Separate from `actors` so a re-bake of the
    /// NPC sheet never has to carry the player with it.
    pub players: Option<ImageSheet>,
    /// Boss-relic pendants, keyed `Player.Relic.<Relic>.<Build>`. A separate
    /// sheet because the gear matrix is already 12 x 16 plates; relics hang on
    /// the player as an overlay rather than multiplying that matrix by nine.
    pub relics: Option<ImageSheet>,
    pub props: Option<ImageSheet>,
    /// 96px standing joinery, separate from the ground and other prop sheets.
    pub doors: Option<ImageSheet>,
    /// D30 chrome plates (docs/ART_PIPELINE.md 'Chrome sheet'); optional —
    /// consumers fall back to flat-drawing without it.
    pub chrome: Option<ImageSheet>,
}

/// Tile Debug names, in enum order (indexed with `as usize`).
// Order must mirror `model::Tile` declaration order.
const TILE_DEBUG_NAMES: [&str; 16] = [
    "Road", "Grass", "Forest", "DeepForest", "Mountain", "Rock", "River", "Ford", "Ruins", "Wall",
    "Floor", "Door", "Up", "Down", "Shrine", "Chest",
];

fn rect_at(col: u32, row: u32, tile_size: u32) -> Rect {
    let (l, t, r, b) = (
        (col * tile_size) as f32,
        (row * tile_size) as f32,
        ((col + 1) * tile_size) as f32,
        ((row + 1) * tile_size) as f32,
    );
    Rect::new(l, t, r, b)
}

pub fn setup(asset_server: Res<AssetServer>, mut commands: Commands) {
    let mut atlas = Atlas::default();
    if let Ok(bytes) = std::fs::read(manifest_path()) {
        match serde_json::from_slice::<Manifest>(&bytes) {
            Ok(manifest) if manifest.tile_size > 0 => {
                let ts = manifest.tile_size;
                for decl in &manifest.sheets {
                    // Terrain plates are alpha-masked diamonds that abut
                    // exactly, and a 64px cell renders MAGNIFIED (~84px at the
                    // shipped camera scale: one tile of T=32 world units at
                    // 2.625 px/unit). Under a linear filter the bilinear kernel
                    // spreads each plate's mask across two texels, and the
                    // neighbour sharing that edge spreads its own mask the same
                    // way, so the composite loses coverage and paints a dark
                    // lattice along every diamond edge — what reads as the
                    // ground being "squared". Nearest keeps coverage binary and
                    // the diamonds abut exactly. The plates are near-flat
                    // colour, so there is almost nothing for the magnification
                    // to alias; actors stay on the default linear sampler.
                    let image = if decl.name.starts_with("terrain") {
                        asset_server
                            .load_builder()
                            .with_settings(|s: &mut ImageLoaderSettings| {
                                s.sampler = ImageSampler::Descriptor(
                                    ImageSamplerDescriptor::nearest(),
                                );
                            })
                            .load(format!("atlas/{}", decl.file))
                    } else {
                        asset_server.load(format!("atlas/{}", decl.file))
                    };
                    let cs = decl.cell_size.filter(|c| *c > 0).unwrap_or(ts);
                    match decl.name.as_str() {
                        SHEET_TERRAIN_SQUARE => {
                            let mut tiles = vec![None; TILE_DEBUG_NAMES.len()];
                            if let Some(mapping) = &decl.mapping {
                                for (index, name) in TILE_DEBUG_NAMES.iter().enumerate() {
                                    if let Some(&(col, row)) = mapping.get(*name) {
                                        tiles[index] = Some(rect_at(col, row, ts));
                                    }
                                }
                            }
                            let mut states_cells = HashMap::new();
                            if let Some(states) = &decl.states {
                                for (group, members) in states {
                                    for (name, &(col, row)) in members {
                                        states_cells.insert(format!("{group}.{name}"), rect_at(col, row, ts));
                                    }
                                }
                            }
                            atlas.terrain_square = Some(Sheet { image, tiles, states_cells });
                        }
                        SHEET_ACTORS => {
                            let mut cells = HashMap::new();
                            if let Some(mapping) = &decl.mapping {
                                for (name, &(col, row)) in mapping {
                                    cells.insert(name.clone(), rect_at(col, row, cs));
                                }
                            }
                            // states.Player.<ClassDebugName> → flat "Player.<Class>"
                            // keys; Actor sheets get more state groups later.
                            if let Some(states) = &decl.states {
                                for (group, group_map) in states {
                                    for (name, &(col, row)) in group_map {
                                        cells.insert(
                                            format!("{group}.{name}"),
                                            rect_at(col, row, cs),
                                        );
                                    }
                                }
                            }
                            atlas.actors = Some(ImageSheet { image, cells });
                        }
                        SHEET_PLAYERS => {
                            let mut cells = HashMap::new();
                            if let Some(mapping) = &decl.mapping {
                                for (name, &(col, row)) in mapping {
                                    cells.insert(name.clone(), rect_at(col, row, cs));
                                }
                            }
                            atlas.players = Some(ImageSheet { image, cells });
                        }
                        SHEET_RELICS => {
                            let mut cells = HashMap::new();
                            if let Some(mapping) = &decl.mapping {
                                for (name, &(col, row)) in mapping {
                                    cells.insert(name.clone(), rect_at(col, row, cs));
                                }
                            }
                            atlas.relics = Some(ImageSheet { image, cells });
                        }
                        SHEET_PROPS | "doors" => {
                            let mut cells = HashMap::new();
                            if let Some(mapping) = &decl.mapping {
                                for (name, &(col, row)) in mapping {
                                    cells.insert(name.clone(), rect_at(col, row, cs));
                                }
                            }
                            let sheet = Some(ImageSheet { image, cells });
                            if decl.name == "doors" { atlas.doors = sheet; } else { atlas.props = sheet; }
                        }
                        SHEET_CHROME => {
                            let mut cells = HashMap::new();
                            if let Some(mapping) = &decl.mapping {
                                for (name, &(col, row)) in mapping {
                                    cells.insert(name.clone(), rect_at(col, row, cs));
                                }
                            }
                            atlas.chrome = Some(ImageSheet { image, cells });
                        }
                        SHEET_TERRAIN_ISO => {
                            let mut faces_by_tile = vec![[None; 3]; TILE_DEBUG_NAMES.len()];
                            let face_rows: [u32; 3] = decl
                                .faces
                                .as_deref()
                                .map(|names| {
                                    let mut rows = [u32::MAX; 3];
                                    for (offset, name) in names.iter().enumerate() {
                                        if let Some(face) = Face::from_name(name) {
                                            rows[face.index()] = offset as u32;
                                        }
                                    }
                                    rows
                                })
                                .unwrap_or([u32::MAX; 3]);
                            if face_rows != [u32::MAX; 3] {
                                if let Some(mapping) = &decl.mapping {
                                    for (index, name) in TILE_DEBUG_NAMES.iter().enumerate() {
                                        if let Some(&(col, r)) = mapping.get(*name) {
                                            for face in [Face::Top, Face::Left, Face::Right] {
                                                faces_by_tile[index][face.index()] = Some(rect_at(
                                                    col,
                                                    r + face_rows[face.index()],
                                                    ts,
                                                ));
                                            }
                                        }
                                    }
                                }
                            }
                            let mut states_cells = std::collections::HashMap::new();
                            if let Some(states) = &decl.states {
                                for (group, group_map) in states {
                                    for (name, &(col, row)) in group_map {
                                        states_cells
                                            .insert(format!("{group}.{name}"), rect_at(col, row, ts));
                                    }
                                }
                            }
                            atlas.terrain_iso = Some(IsoSheet {
                                image,
                                faces: faces_by_tile,
                                states_cells,
                            });
                        }
                        _ => {}
                    }
                }
                info!("atlas: manifest loaded");
            }
            _ => warn!("atlas: manifest present but malformed; quads stay"),
        }
    }
    commands.insert_resource(atlas);
}

/// Sheet image + rect for `tile` once decodable, else None → caller draws a quad.
pub fn square_ref<'a>(
    atlas: &'a Atlas,
    images: &Assets<Image>,
    tile: Tile,
) -> Option<(&'a Handle<Image>, Rect)> {
    let sheet = atlas.terrain_square.as_ref()?;
    images.get(&sheet.image)?; // not decoded yet → quads this frame, never an error
    let rect = sheet.tiles.get(tile as usize).copied().flatten()?;
    Some((&sheet.image, rect))
}

pub fn square_state_ref<'a>(
    atlas: &'a Atlas,
    images: &Assets<Image>,
    key: &str,
) -> Option<(&'a Handle<Image>, Rect)> {
    let sheet = atlas.terrain_square.as_ref()?;
    images.get(&sheet.image)?;
    Some((&sheet.image, sheet.states_cells.get(key).copied()?))
}

/// Iso face cell (image + rect) for (tile, face), else None → diamond fallback.
pub fn iso_ref<'a>(
    atlas: &'a Atlas,
    images: &Assets<Image>,
    tile: Tile,
    face: Face,
) -> Option<(&'a Handle<Image>, Rect)> {
    let sheet = atlas.terrain_iso.as_ref()?;
    images.get(&sheet.image)?;
    let rect = sheet
        .faces
        .get(tile as usize)
        .and_then(|f| f.get(face.index()))
        .copied()
        .flatten()?;
    Some((&sheet.image, rect))
}

/// Iso states cell ("Wall.tall", "Mountain.v2"), else None → plain tile art.
pub fn iso_state_ref<'a>(
    atlas: &'a Atlas,
    images: &Assets<Image>,
    key: &str,
) -> Option<(&'a Handle<Image>, Rect)> {
    let sheet = atlas.terrain_iso.as_ref()?;
    images.get(&sheet.image)?;
    let rect = sheet.states_cells.get(key).copied()?;
    Some((&sheet.image, rect))
}

/// Face-shifted states cell: variant blocks ("<Tile>.v2") pack faces as rows
/// top/left/right under the key cell in base face order.
pub fn iso_state_face_ref<'a>(
    atlas: &'a Atlas,
    images: &Assets<Image>,
    key: &str,
    face: Face,
) -> Option<(&'a Handle<Image>, Rect)> {
    let (image, rect) = iso_state_ref(atlas, images, key)?;
    let h = rect.height();
    let mut shifted = rect;
    shifted.min.y += h * face.index() as f32;
    shifted.max.y += h * face.index() as f32;
    Some((image, shifted))
}

pub fn door_ref<'a>(
    atlas: &'a Atlas,
    images: &Assets<Image>,
    key: &str,
) -> Option<(&'a Handle<Image>, Rect)> {
    let sheet = atlas.doors.as_ref()?;
    images.get(&sheet.image)?;
    Some((&sheet.image, sheet.cells.get(key).copied()?))
}

/// Chrome plate cell ("OrbFrameRed", "PanelFill"…), else None → flat chrome.
pub fn chrome_ref<'a>(
    atlas: &'a Atlas,
    images: &Assets<Image>,
    key: &str,
) -> Option<(&'a Handle<Image>, Rect)> {
    let sheet = atlas.chrome.as_ref()?;
    images.get(&sheet.image)?;
    let rect = sheet.cells.get(key).copied()?;
    Some((&sheet.image, rect))
}

/// Actor cell by Debug name ("Chief", "Player.Keepwarden"…), else None → quad.
/// Terrain-decal archetypes (BroodHole's maw) live on the props sheet: their
/// key resolves there too, so spawners never branch per key.
pub fn actor_ref<'a>(
    atlas: &'a Atlas,
    images: &Assets<Image>,
    key: &str,
) -> Option<(&'a Handle<Image>, Rect)> {
    for sheet in [atlas.actors.as_ref(), atlas.players.as_ref(), atlas.relics.as_ref()].into_iter().flatten() {
        if !images.get(&sheet.image).is_some() {
            continue;
        }
        if let Some(rect) = sheet.cells.get(key).copied() {
            return Some((&sheet.image, rect));
        }
    }
    let props = atlas.props.as_ref()?;
    images.get(&props.image)?;
    let rect = props.cells.get(key).copied()?;
    Some((&props.image, rect))
}

/// Prop cell by plate name ("TorchBrazier"…), else None → quad decal.
pub fn prop_ref<'a>(
    atlas: &'a Atlas,
    images: &Assets<Image>,
    key: &str,
) -> Option<(&'a Handle<Image>, Rect)> {
    let sheet = atlas.props.as_ref()?;
    images.get(&sheet.image)?;
    let rect = sheet.cells.get(key).copied()?;
    Some((&sheet.image, rect))
}
