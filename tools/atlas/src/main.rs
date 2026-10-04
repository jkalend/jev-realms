//! atlas — the D19 terrain-plate baker (docs/ART_PIPELINE.md).
//!
//!   cargo run -p realm-atlas -- terrain    bake assets/atlas/{terrain/square.png,terrain/iso.png,manifest.json}
//!   cargo run -p realm-atlas -- actors     bake assets/atlas/actors.png (19 Archetypes + 6 class Players) + manifest
//!   cargo run -p realm-atlas -- props      bake assets/atlas/props.png + manifest
//!   cargo run -p realm-atlas -- bake       3D-bake (D38): swap painted plates for lit heightfield renders in place
//!   cargo run -p realm-atlas -- preview    contact sheets into docs/gfx/proto/
//!   cargo run -p realm-atlas -- doors      patch closed/open joinery over current floors; retain other cells
//!
//! Pure-RGBA raster, no GPU. Keys are `format!("{:?}")` of
//! laya_realms::model::{Tile, Archetype} — the manifest.v1 exchange contract.

mod actors;
mod bake3d;
mod bake_actor;
mod chrome;
mod paint;
mod props;
mod raster;

use actors::ActorFigure;
use bake3d::{BakeProp, BakeTile};
use image::{Rgba, RgbaImage};
use laya_realms::model::{Archetype, Class, Tile};
use raster::{blank, hash64};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Every Tile variant in model.rs declaration order. Column order in both
/// terrain sheets. (Declared as a table, not reflection: no new sim API.)
const TILES: [Tile; 16] = [
    Tile::Road,
    Tile::Grass,
    Tile::Forest,
    Tile::DeepForest,
    Tile::Mountain,
    Tile::Rock,
    Tile::River,
    Tile::Ford,
    Tile::Ruins,
    Tile::Wall,
    Tile::Floor,
    Tile::Door,
    Tile::Up,
    Tile::Down,
    Tile::Shrine,
    Tile::Chest,
];

/// Every Archetype variant in model.rs declaration order (27: the 19 of
/// batches 1-2 plus the E5 pack and E7's OathlessCurate). BroodHole keeps
/// its sheet CELL reserved but transparent — its art ships on props.png.
const ARCHETYPES: [Archetype; 27] = [
    Archetype::Commoner,
    Archetype::Vendor,
    Archetype::Guard,
    Archetype::Thief,
    Archetype::Traveller,
    Archetype::Bandit,
    Archetype::Wolf,
    Archetype::Bear,
    Archetype::Rat,
    Archetype::Skeleton,
    Archetype::Chief,
    Archetype::Matriarch,
    Archetype::Lich,
    Archetype::Adjudicator,
    Archetype::Smuggler,
    Archetype::Oracle,
    Archetype::Companion,
    Archetype::Tidemother,
    Archetype::Cragmother,
    Archetype::GnawThane,
    Archetype::Tollmaster,
    Archetype::Mirelight,
    Archetype::PaleStag,
    Archetype::Alchemist,
    Archetype::BroodHole,
    Archetype::FalseGlow,
    Archetype::OathlessCurate,
];

/// Every playable class in model.rs declaration order (None excluded).
const CLASSES: [Class; 6] = [
    Class::Keepwarden,
    Class::Gravebound,
    Class::Redwake,
    Class::Waysworn,
    Class::SigilSworn,
    Class::Fensworn,
];

const TS: u32 = paint::TILE;
/// Square sheet: 8 columns x 2 rows.
const SQ_COLS: u32 = 8;
/// Player plates are build-aware and live on their own sheet: the `actors`
/// states.Player row is the pre-build legacy key set and stays untouched.
const PLAYER_COLS: u32 = 17; // 4x4 weapon/armour matrix plus the bare-body column
const PLAYER_ROWS: u32 = 12; // 6 classes x 2 builds
const RELIC_COLS: u32 = 9;
const RELIC_ROWS: u32 = 2;
/// The nine `model::BossRelic` variants, in enum order.
const RELIC_NAMES: [&str; 9] = [
    "Rallybreaker", "Fangmantle", "Graveglass", "Saltcrown", "Stoneheart",
    "GnawboneCrown", "TollcoinCharm", "WisplightLantern", "Hartshorn",
];
const SQ_ROWS: u32 = 2;
/// Iso sheet: 16 columns x 3 face rows (top, left, right).
const ISO_COLS: u32 = 16;
const FACE_ROWS: u32 = 3;
/// Actors sheet: 8 columns x 5 rows (27 archetype slots + at_bay variant
/// + 6 class players = 34 of 40 cells; BroodHole's slot stays empty).
const ACTOR_COLS: u32 = 8;
const ACTOR_ROWS: u32 = 5;
/// Player class frames live on the last row.
const PLAYER_ROW: u32 = 4;
/// Props sheet: one row — 5 painted decals (incl. the BroodHole maw) plus the
/// 6 overhanging tree sprites, which the view reads by key, not by column.
const PROP_COLS: u32 = 11;

fn debug_name(t: impl std::fmt::Debug) -> String {
    format!("{t:?}")
}

fn seed_of(col: u32, row: u32) -> u64 {
    hash64(col as u64 * 9781 ^ row as u64 * 421 ^ 0x5eed_5eed)
}

// ---------------------------------------------------------------------------
// manifest.v1 (the shared AtlasArt <-> E4View exchange contract)

#[derive(Serialize, Deserialize)]
struct Manifest {
    tile_size: u32,
    sheets: Vec<Sheet>,
}

#[derive(Serialize, Deserialize)]
struct Sheet {
    name: String,
    file: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    cell_size: Option<u32>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    faces: Vec<String>,
    mapping: BTreeMap<String, [u32; 2]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    states: Option<serde_json::Map<String, serde_json::Value>>,
}

fn bake_square_sheet() -> (RgbaImage, BTreeMap<String, [u32; 2]>) {
    let mut sheet = blank(SQ_COLS * TS, SQ_ROWS * TS);
    let mut mapping = BTreeMap::new();
    for (i, t) in TILES.iter().enumerate() {
        let (col, row) = (i as u32 % SQ_COLS, i as u32 / SQ_COLS);
        let tile_img = paint::square_tile(*t, seed_of(col, row));
        blit(&mut sheet, &tile_img, col * TS, row * TS);
        mapping.insert(debug_name(*t), [col, row]);
    }
    (sheet, mapping)
}

fn bake_iso_sheet() -> (RgbaImage, BTreeMap<String, [u32; 2]>) {
    let mut sheet = blank(ISO_COLS * TS, FACE_ROWS * TS);
    let mut mapping = BTreeMap::new();
    for (col, t) in TILES.iter().enumerate() {
        let col = col as u32;
        for (face_row, face) in paint::iso_faces(*t, seed_of(col, 0)).iter().enumerate() {
            blit(&mut sheet, face, col * TS, face_row as u32 * TS);
        }
        // Contract: mapping points at the tile's first face row; remaining
        // faces follow in `faces` order in the rows beneath.
        mapping.insert(debug_name(*t), [col, 0]);
    }
    (sheet, mapping)
}

fn blit(dst: &mut RgbaImage, src: &RgbaImage, ox: u32, oy: u32) {
    for (x, y, p) in src.enumerate_pixels() {
        dst.put_pixel(ox + x, oy + y, *p);
    }
}

fn placeholder() -> RgbaImage {
    // 1x1 fully transparent stand-in so loaders never hit a missing file
    // before the actor/props batches land.
    RgbaImage::from_pixel(1, 1, Rgba([0, 0, 0, 0]))
}

// ---------------------------------------------------------------------------
// Actors + props batches.

/// 27 archetype idle frames + states variants + 6 class-hue hero frames.
/// BroodHole's cell stays reserved-but-empty (the maw is a prop decal).
/// Variant keys live in the sheet's `states` object under their group name
/// ("PaleStag"."at_bay"); players under "Player". See docs/ART_PIPELINE.md.
/// Returns (sheet, mapping, state_groups).
fn bake_actor_sheet() -> (
    RgbaImage,
    BTreeMap<String, [u32; 2]>,
    BTreeMap<String, BTreeMap<String, [u32; 2]>>,
) {
    let mut sheet = blank(ACTOR_COLS * TS, ACTOR_ROWS * TS);
    let mut mapping = BTreeMap::new();
    for (i, a) in ARCHETYPES.iter().enumerate() {
        if *a == Archetype::BroodHole {
            continue; // prop decal, see props.rs
        }
        let (col, row) = (i as u32 % ACTOR_COLS, i as u32 / ACTOR_COLS);
        let plate = actors::actor_plate(actors::accent(*a), &ActorFigure::Npc(*a));
        blit(&mut sheet, &plate, col * TS, row * TS);
        mapping.insert(debug_name(*a), [col, row]);
    }
    // states.PaleStag.at_bay: pose proof-case for the group convention.
    let at_bay = actors::actor_plate(
        actors::accent(Archetype::PaleStag),
        &ActorFigure::NpcState(Archetype::PaleStag, "at_bay"),
    );
    blit(&mut sheet, &at_bay, 4 * TS, 3 * TS);
    let mut variants = BTreeMap::new();
    let mut stag_states = BTreeMap::new();
    stag_states.insert("at_bay".to_string(), [4, 3]);
    variants.insert("PaleStag".to_string(), stag_states);
    let mut players = BTreeMap::new();
    for (i, c) in CLASSES.iter().enumerate() {
        let (col, row) = (1 + i as u32, PLAYER_ROW);
        let plate = actors::actor_plate(
            [235, 200, 90],
            &ActorFigure::Player {
                class_hue: actors::class_hue(*c),
            },
        );
        blit(&mut sheet, &plate, col * TS, row * TS);
        players.insert(debug_name(*c), [col, row]);
    }
    variants.insert("Player".to_string(), players);
    (sheet, mapping, variants)
}

const TREE_KEYS: [&str; 6] = [
    "Tree1", "Tree2", "Tree3", "DeepTree1", "DeepTree2", "DeepTree3",
];

fn bake_prop_sheet() -> (RgbaImage, BTreeMap<String, [u32; 2]>) {
    let plates = props::prop_plates();
    let mut sheet = blank(PROP_COLS * TS, TS);
    let mut mapping = BTreeMap::new();
    for (i, (key, plate)) in plates.into_iter().enumerate() {
        // Free-standing props bake through the SDF rig: a heightfield splat has
        // no vertical geometry, so it cannot express a brazier's legs or a
        // monolith's silhouette at all. Anything without a rig keeps its
        // painted plate.
        let plate = bake3d::bake_prop_rig(&key).unwrap_or(plate);
        blit(&mut sheet, &plate, i as u32 * TS, 0);
        mapping.insert(key.to_string(), [i as u32, 0]);
    }
    for (i, key) in TREE_KEYS.iter().enumerate() {
        let deep = i >= 3;
        let col = plates_len() + i as u32;
        let plate = bake3d::bake_tree(deep, (i % 3) as u64 + 1);
        blit(&mut sheet, &plate, col * TS, 0);
        mapping.insert((*key).to_string(), [col, 0]);
    }
    (sheet, mapping)
}

fn plates_len() -> u32 {
    props::prop_plates().len() as u32
}

/// Patch one sheet entry of the on-disk manifest (other sheets untouched);
/// inserts the sheet if the name is new (chrome lane adds its own family).
fn patch_manifest(
    root: &Path,
    name: &str,
    mapping: BTreeMap<String, [u32; 2]>,
    states: Option<serde_json::Map<String, serde_json::Value>>,
) -> Result<(), String> {
    let path = root.join("manifest.json");
    let raw = std::fs::read_to_string(&path)
        .map_err(|_| "manifest.json missing: run `atlas terrain` first".to_string())?;
    let mut man: Manifest = serde_json::from_str(&raw).map_err(|e| format!("manifest parse: {e}"))?;
    match man.sheets.iter_mut().find(|s| s.name == name) {
        Some(sheet) => {
            sheet.mapping = mapping;
            sheet.states = states;
            if name == "actors" {
                // `atlas actors` explicitly replaces a roster-v2 96px sheet
                // with the painter's 64px legacy sheet.
                sheet.cell_size = None;
            }
        }
        None => man.sheets.push(Sheet {
            name: name.into(),
            file: format!("{name}.png"),
            faces: vec![],
            cell_size: None,
            mapping,
            states,
        }),
    }
    let json = serde_json::to_string_pretty(&man).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| e.to_string())?;
    println!("ATLAS OK {}", path.display());
    Ok(())
}

const CHROME_COLS: u32 = 8;

fn cmd_chrome(root: &Path) -> Result<(), String> {
    let plates = chrome::chrome_plates();
    let rows = plates.len().div_ceil(CHROME_COLS as usize) as u32;
    let mut sheet = blank(CHROME_COLS * TS, rows * TS);
    let mut mapping = BTreeMap::new();
    for (i, (key, plate)) in plates.into_iter().enumerate() {
        let (col, row) = (i as u32 % CHROME_COLS, i as u32 / CHROME_COLS);
        blit(&mut sheet, &plate, col * TS, row * TS);
        mapping.insert(key.to_string(), [col, row]);
    }
    let path = root.join("chrome.png");
    sheet.save(&path).map_err(|e| e.to_string())?;
    println!("ATLAS OK {} ({}x{})", path.display(), sheet.width(), sheet.height());
    patch_manifest(root, "chrome", mapping, None)?;
    chrome_contact()?;
    verify(root)
}

/// Mock consumers proof: pane from the 9-patch + orbs + nameplate + button
/// row + pips + sliver, assembled with only the plates (no game code).
fn chrome_contact() -> Result<(), String> {
    let plates = chrome::chrome_plates();
    let find = |k: &str| plates.iter().find(|(n, _)| *n == k).unwrap().1.clone();
    let mut canvas = blank(8 * 72, 6 * 72 + 16);
    let (w, h) = (canvas.width() as f32, canvas.height() as f32);
    raster::fill_rect(&mut canvas, 0.0, 0.0, w, h, [16, 17, 22]);
    // Orbs: frame first, then the 72% mock fill over the well (consumer
    // draws fills above the inset and under nothing else).
    let orbs = [("OrbFrameRed", HP_MOCK), ("OrbFrameBlue", MANA_MOCK)];
    for (i, (key, fill_c)) in orbs.iter().enumerate() {
        let ox = (4 + i * 2) as u32 * 72 + 4;
        blit_blend(&mut canvas, &find(key), ox, 8);
        fill_disc_mock(&mut canvas, ox, 8, fill_c[0], fill_c[1], 0.72);
    }
    // Nameplate above the pane.
    let np = [("BossNameplateL", 0), ("BossNameplateM", 1), ("BossNameplateR", 2)];
    for (key, i) in np {
        blit_blend(&mut canvas, &find(key), 160 + i * 64, 60);
    }
    // Pane: 4x3 assembly.
    let grid: [[&str; 4]; 3] = [
        ["PanelCornerNW", "PanelEdgeN", "PanelEdgeN", "PanelCornerNE"],
        ["PanelEdgeW", "PanelFill", "PanelFill", "PanelEdgeE"],
        ["PanelCornerSW", "PanelEdgeS", "PanelEdgeS", "PanelCornerSE"],
    ];
    for (gi, grow) in grid.iter().enumerate() {
        for (gj, key) in grow.iter().enumerate() {
            blit_blend(&mut canvas, &find(key), (160 + gj * 64) as u32, (128 + gi * 64) as u32);
        }
    }
    // Menu rows inside the pane interior (one normal, one selected).
    blit_blend(&mut canvas, &find("ButtonNormal"), 168, 200);
    blit_blend(&mut canvas, &find("ButtonSelected"), 232, 200);
    // Sigil pips + XP sliver under the pane (spaced so housings stay discrete).
    for p in 0..3u32 {
        blit_blend(&mut canvas, &find("SigilPip"), 168 + p * 44, 340);
    }
    blit_blend(&mut canvas, &find("XPSliver"), 320, 356);
    let out = PathBuf::from("docs/gfx/proto/atlas-chrome-contact.png");
    let big = upscale2(&canvas);
    big.save(&out).map_err(|e| e.to_string())?;
    println!("ATLAS OK {} ({}x{})", out.display(), big.width(), big.height());
    println!("Chrome legend: orbs (HP/Mana with 72% mock fills) | nameplate | 4x3 pane |");
    println!("               button row (normal + selected alternating) | pips + sliver");
    Ok(())
}

const HP_MOCK: [[u8; 3]; 2] = [[168, 44, 40], [102, 40, 30]];
const MANA_MOCK: [[u8; 3]; 2] = [[64, 92, 178], [36, 52, 105]];

/// Demo-only inset fill: a spherical pool in the orb well (consumers will
/// draw this themselves; the mock just proves the plate composites over it).
fn fill_disc_mock(canvas: &mut RgbaImage, ox: u32, oy: u32, c: [u8; 3], deep: [u8; 3], fill: f32) {
    let _ = deep;
    for y in 0..64i32 {
        for x in 0..64i32 {
            let dx = x as f32 - 32.0;
            let dy = y as f32 - 32.0;
            let d = (dx * dx + dy * dy).sqrt();
            if d < 19.0 && (1.0 - (dy + 19.0) / 38.0) < fill {
                let relief = (1.0 - d * d / (19.0 * 19.0)).sqrt();
                let col = raster::shade(c, 0.55 + 0.75 * relief + 0.30 * (dy / 19.0).clamp(-1.0, 0.0));
                canvas.put_pixel(ox + x as u32, oy + y as u32, image::Rgba([col[0], col[1], col[2], 255]));
            }
        }
    }
}

fn cmd_actors(root: &Path) -> Result<(), String> {
    let (sheet, mapping, variants) = bake_actor_sheet();
    let path = root.join("actors.png");
    sheet.save(&path).map_err(|e| e.to_string())?;
    println!("ATLAS OK {} ({}x{})", path.display(), sheet.width(), sheet.height());
    let mut states = serde_json::Map::new();
    for (group, inner) in variants {
        states.insert(group, serde_json::to_value(inner).map_err(|e| e.to_string())?);
    }
    patch_manifest(root, "actors", mapping, Some(states))?;
    verify(root)
}

fn cmd_props(root: &Path) -> Result<(), String> {
    let (sheet, mapping) = bake_prop_sheet();
    let path = root.join("props.png");
    sheet.save(&path).map_err(|e| e.to_string())?;
    println!("ATLAS OK {} ({}x{})", path.display(), sheet.width(), sheet.height());
    patch_manifest(root, "props", mapping, None)?;
    verify(root)
}

// ---------------------------------------------------------------------------
// 3D bake path (D38): swap painted plates for heightfield-rendered ones in
// place. Manifest keys and cell coordinates are untouched.

fn bake_seed(tile: BakeTile) -> u64 {
    hash64(match tile {
        BakeTile::Mountain => 0xba3e_0001,
        BakeTile::DeepForest => 0xba3e_0d2f,
        BakeTile::Forest => 0xba3e_f071,
        BakeTile::Rock => 0xba3e_20c4,
        BakeTile::Wall => 0xba3e_4411,
        BakeTile::River => 0xba3e_21fe,
        BakeTile::Ford => 0xba3e_f0d2,
        BakeTile::Ruins => 0xba3e_2b17,
        BakeTile::Floor => 0xba3e_f100,
    })
}

fn tile_bake_info(tile: BakeTile) -> (Tile, usize) {
    let t = match tile {
        BakeTile::Mountain => Tile::Mountain,
        BakeTile::DeepForest => Tile::DeepForest,
        BakeTile::Forest => Tile::Forest,
        BakeTile::Rock => Tile::Rock,
        BakeTile::Wall => Tile::Wall,
        BakeTile::River => Tile::River,
        BakeTile::Ford => Tile::Ford,
        BakeTile::Ruins => Tile::Ruins,
        BakeTile::Floor => Tile::Floor,
    };
    (t, TILES.iter().position(|x| *x == t).unwrap())
}

/// The D38 actor rung: SDF rig bakes swapped into the actors sheet at the
/// same cells the painters use (players = states.Player row; archetypes =
/// enum-ordered mapping; at_bay = the states.PaleStag slot at [4,3]).
fn cmd_bake_actors(root: &Path) -> Result<(), String> {
    let path = root.join("actors.png");
    let mut sheet = image::open(&path)
        .map_err(|e| format!("{e} (run `atlas actors` first)"))?
        .to_rgba8();
    let mut review_rows: Vec<(String, RgbaImage, RgbaImage)> = Vec::new();
    for (key, rig) in bake_actor::batch() {
        // Cell lookup mirrors bake_actor_sheet layout exactly.
        let (col, row) = if let Some((name, variant)) = key.split_once('.') {
            match name {
                "Player" => {
                    let class = CLASSES
                        .iter()
                        .position(|c| debug_name(*c) == variant)
                        .ok_or_else(|| format!("unknown player class {variant}"))?;
                    (1 + class as u32, PLAYER_ROW)
                }
                "PaleStag" if variant == "at_bay" => (4, 3),
                other => return Err(format!("unknown states group {other}")),
            }
        } else {
            let idx = ARCHETYPES
                .iter()
                .position(|a| debug_name(*a) == key)
                .ok_or_else(|| format!("unknown archetype {key}"))?;
            (idx as u32 % ACTOR_COLS, idx as u32 / ACTOR_COLS)
        };
        let mut baked = actors::chunky(&bake_actor::bake_actor_rig(&rig), 3);
        if bake_actor::is_wisp(key) {
            bake_actor::wisp_fade(&mut baked);
        }
        bake_actor::anchor_shadow(&mut baked);
        // Painted reference cell (rebuilt from the live painters for the
        // review sheet, not from the on-disk sheet, so painting code is the
        // only source of truth for the comparison).
        let painted = match key {
            "PaleStag.at_bay" => actors::actor_plate(
                actors::accent(Archetype::PaleStag),
                &ActorFigure::NpcState(Archetype::PaleStag, "at_bay"),
            ),
            _ if key.starts_with("Player.") => {
                let class = CLASSES
                    .iter()
                    .position(|c| debug_name(*c) == &key[7..])
                    .ok_or_else(|| format!("unknown player class {key}"))?;
                let c = CLASSES[class];
                actors::actor_plate(
                    [235, 200, 90],
                    &ActorFigure::Player {
                        class_hue: actors::class_hue(c),
                    },
                )
            }
            _ => {
                let idx = ARCHETYPES.iter().position(|a| debug_name(*a) == key).unwrap();
                actors::actor_plate(actors::accent(ARCHETYPES[idx]), &ActorFigure::Npc(ARCHETYPES[idx]))
            }
        };
        blit(&mut sheet, &baked, col * TS, row * TS);
        println!("ATLAS BAKE OK {key} -> actors[{col},{row}]");
        review_rows.push((key.to_string(), painted, baked));
    }
    // Hard revert (idempotent): glow-body spectrals stay painted (Main's
    // call over batch-1 review) — re-stamp their cells from painter code.
    for key in bake_actor::PAINTED_ONLY {
        let idx = ARCHETYPES
            .iter()
            .position(|a| debug_name(*a) == key)
            .ok_or_else(|| format!("unknown archetype {key}"))?;
        let (col, row) = (idx as u32 % ACTOR_COLS, idx as u32 / ACTOR_COLS);
        let painted = actors::actor_plate(actors::accent(ARCHETYPES[idx]), &ActorFigure::Npc(ARCHETYPES[idx]));
        blit(&mut sheet, &painted, col * TS, row * TS);
        println!("ATLAS PAINTED {key} -> actors[{col},{row}] (spectral exemption)");
    }
    sheet.save(&path).map_err(|e| e.to_string())?;
    println!("ATLAS OK {}", path.display());
    // Review rows: the batch-2 roster only (heroes + batch-1 E5/E7 subjects
    // reviewed on the rung-2 contact sheet already).
    let batch1: [&str; 6] = [
        "GnawThane",
        "PaleStag",
        "PaleStag.at_bay",
        "Tollmaster",
        "Alchemist",
        "OathlessCurate",
    ];
    let new_rows: Vec<_> = review_rows
        .into_iter()
        .filter(|(k, _, _)| !batch1.contains(&k.as_str()) && !k.starts_with("Player."))
        .collect();
    actor_bake_contact(&new_rows)?;
    verify(root)
}

fn actor_bake_contact(rows: &[(String, RgbaImage, RgbaImage)]) -> Result<(), String> {
    const SLOT: u32 = 140;
    let per_row: u32 = 2;
    let rows_n = rows.len().div_ceil(per_row as usize) as u32;
    let mut shot = blank(per_row * 2 * SLOT + 8, rows_n * SLOT + 8);
    let (w, h) = (shot.width() as f32, shot.height() as f32);
    raster::fill_rect(&mut shot, 0.0, 0.0, w, h, [24, 26, 32]);
    let grass = paint::square_tile(Tile::Grass, 1);
    println!("Actor bake contact: per pair painted | baked-3d, labels below:");
    for (i, (key, painted, baked)) in rows.iter().enumerate() {
        let gr = (i / per_row as usize) as u32;
        let gc = (i % per_row as usize) as u32;
        let ox = gc * 2 * SLOT + 4;
        let oy = gr * SLOT + 4;
        blit_blend(&mut shot, &grass, ox, oy);
        blit_blend(&mut shot, &grass, ox + SLOT, oy);
        blit_blend(&mut shot, painted, ox, oy);
        blit_blend(&mut shot, baked, ox + SLOT, oy);
        println!("  {key}");
    }
    let out = PathBuf::from("docs/gfx/proto/atlas-actors-3d-contact.png");
    let big = upscale2(&shot);
    big.save(&out).map_err(|e| e.to_string())?;
    println!("ATLAS OK {} ({}x{})", out.display(), big.width(), big.height());
    Ok(())
}

// ---------------------------------------------------------------------------
// v3 QA: zoomed-out world contact (D2-route judgment for the de-block wave):
// a fake 6x6 iso map assembled from REAL current faces (baked where the D38
// lane owns the tile, painted for Grass et al. — same mix the renderer draws),
// with 8 baked actors standing on it. This is what the owner judges against.

/// Tile kinds on the world map: painted Grass grounds, baked blocks on top,
/// and the v4 states materials (ridge variants, tall curtain run).
enum WorldTile {
    Grass,
    Baked(BakeTile),
    /// Ridge-family alternate-phase block (states.<Tile>.v2 columns).
    BakedV2(BakeTile),
    /// The 2-cell curtain: upper cell stacked above its anchor row.
    TallWall,
}

fn world_contact() -> Result<(), String> {
    let layout: [[WorldTile; 6]; 6] = [
        [
            WorldTile::Baked(BakeTile::Mountain),
            WorldTile::BakedV2(BakeTile::Mountain),
            WorldTile::Baked(BakeTile::Mountain),
            WorldTile::Baked(BakeTile::DeepForest),
            WorldTile::Baked(BakeTile::Forest),
            WorldTile::Baked(BakeTile::Mountain),
        ],
        [
            WorldTile::Baked(BakeTile::Mountain),
            WorldTile::Baked(BakeTile::DeepForest),
            WorldTile::Grass,
            WorldTile::Baked(BakeTile::Forest),
            WorldTile::Grass,
            WorldTile::Baked(BakeTile::Mountain),
        ],
        // The vanity defensive line: one straight TALL curtain across.
        [WorldTile::TallWall, WorldTile::TallWall, WorldTile::TallWall,
         WorldTile::TallWall, WorldTile::TallWall, WorldTile::TallWall],
        [WorldTile::Grass, WorldTile::Grass, WorldTile::Grass,
         WorldTile::Grass, WorldTile::Grass, WorldTile::Grass],
        [
            WorldTile::Grass,
            WorldTile::Baked(BakeTile::Forest),
            WorldTile::Grass,
            WorldTile::Grass,
            WorldTile::Baked(BakeTile::Rock),
            WorldTile::Grass,
        ],
        [
            WorldTile::Grass,
            WorldTile::Grass,
            WorldTile::Baked(BakeTile::Rock),
            WorldTile::Grass,
            WorldTile::Grass,
            WorldTile::Grass,
        ],
    ];
    let mut canvas = blank(560, 380);
    let (w, h) = (canvas.width() as f32, canvas.height() as f32);
    raster::fill_rect(&mut canvas, 0.0, 0.0, w, h, [12, 15, 14]);
    // Iso projection: diamonds at 32px half-width, 16px half-height.
    // Logical anchor A=(32,20) of each cell's faces sits at diamond center.
    let anchor = |x: i32, y: i32| ((x - y) * 32 + 280, (x + y) * 16 + 52);
    // Walk faces in y-major painter order: draw each tile's left, right,
    // top as one block, back (sum-low) to front (sum-high).
    for sum in 0..=11 {
        for y in 0..6i32 {
            for x in 0..6i32 {
                if x + y != sum || !(0..6).contains(&x) || !(0..6).contains(&y) {
                    continue;
                }
                let (cx, ay) = anchor(x, y);
                match layout[y as usize][x as usize] {
                    WorldTile::Grass => {
                        let p = paint::iso_faces(Tile::Grass, seed_of(x as u32, y as u32));
                        let faces = [p[0].clone(), p[1].clone(), p[2].clone()];
                        blit_blend(&mut canvas, &faces[1], (cx - 32) as u32, (ay - 20) as u32);
                        blit_blend(&mut canvas, &faces[2], (cx - 32) as u32, (ay - 20) as u32);
                        blit_blend(&mut canvas, &faces[0], (cx - 32) as u32, (ay - 20) as u32);
                    }
                    WorldTile::Baked(bt) => {
                        let faces = bake3d::bake_iso(bt, bake_seed(bt));
                        blit_blend(&mut canvas, &faces[1], (cx - 32) as u32, (ay - 20) as u32);
                        blit_blend(&mut canvas, &faces[2], (cx - 32) as u32, (ay - 20) as u32);
                        blit_blend(&mut canvas, &faces[0], (cx - 32) as u32, (ay - 20) as u32);
                    }
                    WorldTile::BakedV2(bt) => {
                        let faces = bake3d::bake_iso(bt, variant_seed(bt));
                        blit_blend(&mut canvas, &faces[1], (cx - 32) as u32, (ay - 20) as u32);
                        blit_blend(&mut canvas, &faces[2], (cx - 32) as u32, (ay - 20) as u32);
                        blit_blend(&mut canvas, &faces[0], (cx - 32) as u32, (ay - 20) as u32);
                    }
                    WorldTile::TallWall => {
                        // 2-cell curtain: the consumer stacks [c, r+1] above
                        // the anchor cell — same column, one cell-row up.
                        let [upper, lower] = bake3d::bake_wall_tall(bake_seed(BakeTile::Wall));
                        blit_blend(&mut canvas, &lower, (cx - 32) as u32, (ay - 20) as u32);
                        blit_blend(&mut canvas, &upper, (cx - 32) as u32, (ay - 20 - 64) as u32);
                    }
                }
            }
        }
    }
    // 8 baked actors on the grass: fresh rigs with anchor shadows, so this
    // contact reflects the CURRENT plate look after v3.
    let cast: [(usize, i32, i32); 8] = [
        (9, 1, 3),   // Tollmaster
        (12, 2, 3),  // Commoner
        (14, 3, 3),  // Thief
        (18, 4, 3),  // Guard
        (20, 1, 4),  // Chief
        (24, 3, 4),  // Companion
        (13, 4, 4),  // Vendor
        (25, 5, 3),  // Tidemother
    ];
    let rigs = bake_actor::batch();
    for (bi, ax, ay) in cast {
        let (_, rig) = &rigs[bi];
        let mut plate = actors::chunky(&bake_actor::bake_actor_rig(rig), 3);
        bake_actor::anchor_shadow(&mut plate);
        let (cx, ay) = anchor(ax, ay);
        blit_blend(&mut canvas, &plate, (cx - 30) as u32, (ay - 30) as u32);
    }
    let out = PathBuf::from("docs/gfx/proto/atlas-terrain-world-contact.png");
    let big = upscale2(&canvas);
    big.save(&out).map_err(|e| e.to_string())?;
    println!("ATLAS OK {} ({}x{})", out.display(), big.width(), big.height());
    println!("World legend: mountains/flank wall diag, forest mass, rocks,");
    println!("  grass ground, 8 baked actors placed over the ground plane");
    Ok(())
}

// v4 iso states: ridge-family variants + the Wall.tall curtain.
const VARIANT_TILES: [BakeTile; 6] = [
    BakeTile::Mountain,
    BakeTile::DeepForest,
    BakeTile::Forest,
    BakeTile::Rock,
    // Water flows, so its phases are real alternate wave fields rather than
    // recolours. Four of them: two still repeated visibly along a long river.
    BakeTile::River,
    BakeTile::Ford,
];
const VARIANT_BASE_COL: u32 = 16; // variants at cols 16..21, base face rows
// Water's four flow phases per tile: cols 22..25 River, 26..29 Ford.
const FLOW_BASE_COL: u32 = 22;
const FLOW_PHASES: u32 = 4;
const TALL_COL: u32 = 9;          // Wall's enum column
/// states.Wall.tall anchor; the curtain's body cell sits a row below.
const TALL_TOP_ROW: u32 = 3;
/// Material columns follow water phases: wall, floor, alternate floor per family.
const MATERIAL_BASE_COL: u32 = 30;
const ISO_WIDE_W: u32 = (MATERIAL_BASE_COL + 8 * 3) * TS;
const ISO_WIDE_H: u32 = 320;

fn variant_seed(bt: BakeTile) -> u64 {
    hash64(bake_seed(bt) ^ 0x7e57_1a2b)
}

/// Expand the iso sheet to the states layout (30 cols x 5 rows), copying
/// the existing base region — rolling into extend only when a states bake
/// actually runs; rollback (`atlas terrain`) rewinds to the plain grid.
fn expand_iso_canvas(old: &RgbaImage) -> RgbaImage {
    if old.width() >= ISO_WIDE_W && old.height() >= ISO_WIDE_H {
        return old.clone();
    }
    let mut out = blank(ISO_WIDE_W, ISO_WIDE_H);
    let (rw, rh) = (old.width().min(ISO_WIDE_W), old.height().min(ISO_WIDE_H));
    for y in 0..rh {
        for x in 0..rw {
            out.put_pixel(x, y, *old.get_pixel(x, y));
        }
    }
    out
}

/// Write the terrain-iso states entries for this bake's features (other
/// states groups on other sheets untouched; terrain-iso's mapping rows
/// stay as-is — states only add alternate lookup keys).
fn write_iso_states(root: &Path, tiles: &[BakeTile]) -> Result<(), String> {
    let man_path = root.join("manifest.json");
    let mut man: Manifest = serde_json::from_str(
        &std::fs::read_to_string(&man_path).map_err(|e| e.to_string())?,
    )
    .map_err(|e| format!("manifest parse: {e}"))?;
    let sheet = man
        .sheets
        .iter_mut()
        .find(|s| s.name == "terrain-iso")
        .ok_or("manifest has no terrain-iso")?;
    let mut states = sheet.states.take().unwrap_or_default();
    if tiles.iter().any(|bt| VARIANT_TILES.contains(bt)) {
        for (i, bt) in VARIANT_TILES.iter().enumerate() {
            let t = tile_bake_info(*bt).0;
            if matches!(*bt, BakeTile::River | BakeTile::Ford) {
                // Four flow phases per water tile; the view picks one per
                // position hash, so a long river stops repeating.
                let mut phases = serde_json::Map::new();
                for phase in 0..FLOW_PHASES {
                    let col = FLOW_BASE_COL
                        + (if *bt == BakeTile::Ford { FLOW_PHASES } else { 0 })
                        + phase;
                    phases.insert(format!("v{}", phase + 2), serde_json::json!([col, 0]));
                }
                states.insert(debug_name(t), serde_json::Value::Object(phases));
                continue;
            }
            states.insert(
                debug_name(t),
                serde_json::json!({"v2": [VARIANT_BASE_COL + i as u32, 0]}),
            );
        }
    }
    if tiles.contains(&BakeTile::Wall) {
        let wall = states.entry("Wall").or_insert_with(|| serde_json::json!({}));
        wall.as_object_mut().ok_or("Wall states is not an object")?
            .insert("tall".into(), serde_json::json!([TALL_COL, TALL_TOP_ROW]));
    }
    if tiles.contains(&BakeTile::Wall) || tiles.contains(&BakeTile::Floor) {
        for (family, name) in bake3d::MATERIAL_FAMILIES.iter().enumerate() {
            for (group, offset, suffix) in [("Wall", 0, ""), ("Floor", 1, ""), ("Floor", 2, "2")] {
                let member = states.entry(group).or_insert_with(|| serde_json::json!({}));
                member.as_object_mut().ok_or("material states is not an object")?
                    .insert(format!("{name}{suffix}"), serde_json::json!([MATERIAL_BASE_COL + family as u32 * 3 + offset, 0]));
            }
        }
    }
    sheet.states = Some(states);
    let json = serde_json::to_string_pretty(&man).map_err(|e| e.to_string())?;
    std::fs::write(&man_path, json).map_err(|e| e.to_string())?;
    println!("ATLAS OK {} (terrain-iso states updated)", man_path.display());
    Ok(())
}

/// Ridge variants side-by-side (baked block v1 | baked block v2): the
/// cheap seamlessness proof — adjacent-permutation blotch phases.
fn variant_contact() -> Result<(), String> {
    const SLOT: u32 = 140;
    const PER_COL: usize = 4;
    // One row per variant; the sheet grows with VARIANT_TILES rather than
    // assuming four, which is what silently overflowed when water joined.
    let rows = VARIANT_TILES.len().div_ceil(PER_COL) as u32;
    let mut shot = blank(PER_COL as u32 * SLOT, rows * SLOT);
    let (w, h) = (shot.width() as f32, shot.height() as f32);
    raster::fill_rect(&mut shot, 0.0, 0.0, w, h, [24, 26, 32]);
    println!("Variant contact: col0 v1 | col1 v2 | col2 v2-adjacent | col3 v2-adjacent (seam test)");
    for (i, bt) in VARIANT_TILES.iter().enumerate() {
        let oy = (i / PER_COL) as u32 * SLOT + 4;
        let v1 = bake3d::bake_iso(*bt, bake_seed(*bt));
        let v2 = bake3d::bake_iso(*bt, variant_seed(*bt));
        let composite = |faces: &[RgbaImage; 3]| {
            let mut b = blank(TS, TS);
            blit_blend(&mut b, &faces[1], 0, 0);
            blit_blend(&mut b, &faces[2], 0, 0);
            blit_blend(&mut b, &faces[0], 0, 0);
            b
        };
        blit_blend(&mut shot, &composite(&v1), 4, oy);
        for c in 0..3u32 {
            blit_blend(&mut shot, &composite(&v2), SLOT + 4 + c * SLOT, oy);
        }
        println!("  slot {i}: {bt:?} (col 0 base, cols 1-3 phase-variant)");
    }
    let out = PathBuf::from("docs/gfx/proto/atlas-baked-variants-contact.png");
    let big = upscale2(&shot);
    big.save(&out).map_err(|e| e.to_string())?;
    println!("ATLAS OK {} ({}x{})", out.display(), big.width(), big.height());
    Ok(())
}

fn cmd_bake(root: &Path, tiles: Vec<BakeTile>, props_l: Vec<BakeProp>) -> Result<(), String> {
    if !tiles.is_empty() {
        let sq_path = root.join("terrain/square.png");
        let iso_path = root.join("terrain/iso.png");
        let mut square = image::open(&sq_path)
            .map_err(|e| format!("{e} (run `atlas terrain` first)"))?
            .to_rgba8();
        let mut iso = image::open(&iso_path)
            .map_err(|e| format!("{e} (run `atlas terrain` first)"))?
            .to_rgba8();
        for bt in tiles.iter().copied() {
            let (t, idx) = tile_bake_info(bt);
            let seed = bake_seed(bt);
            // Square cell (8x2 grid, enum order).
            let (col, row) = (idx as u32 % SQ_COLS, idx as u32 / SQ_COLS);
            let sq = bake3d::bake_square(bt, seed);
            blit(&mut square, &sq, col * TS, row * TS);
            // Iso column, three face rows.
            let faces = bake3d::bake_iso(bt, seed);
            for (fr, face) in faces.iter().enumerate() {
                blit(&mut iso, face, idx as u32 * TS, fr as u32 * TS);
            }
            println!("ATLAS BAKE OK {t:?} -> square[{col},{row}] + iso[{idx},0..2]");
        }
        // v4 states extension: ridge variants (one extra column per tile,
        // v2 rows in base face order) + Wall.tall (2-cell curtain pair).
        let wants_variants = tiles.iter().any(|bt| VARIANT_TILES.contains(bt));
        let wants_tall = tiles.contains(&BakeTile::Wall);
        let wants_materials = tiles.contains(&BakeTile::Wall) || tiles.contains(&BakeTile::Floor);
        if wants_variants || wants_tall || wants_materials {
            let mut iso_wide = expand_iso_canvas(&iso);
            if wants_variants {
                for (i, bt) in VARIANT_TILES.iter().enumerate() {
                    let seed = variant_seed(*bt);
                    if matches!(*bt, BakeTile::River | BakeTile::Ford) {
                        // Water's phases are baked by the flow block below; the
                        // states map is written there.
                        continue;
                    }
                    // Only variants of tiles in this bake list are batch
                    // refresh-critical; regenerate all four whenever any is,
                    // so the states set stays internally consistent.
                    let faces = bake3d::bake_iso(*bt, seed);
                    for (fr, face) in faces.iter().enumerate() {
                        blit(&mut iso_wide, face, (VARIANT_BASE_COL + i as u32) * TS, fr as u32 * TS);
                    }
                    println!("ATLAS BAKE OK {bt:?}.v2 -> iso[{},0..2]", VARIANT_BASE_COL + i as u32);
                }
            }
            // Water flow phases: each is a distinct wave field, keyed v2..v5 so
            // the view can pick one per position hash instead of coin-flipping
            // between two and letting a long river visibly repeat.
            if tiles.contains(&BakeTile::River) || tiles.contains(&BakeTile::Ford) {
                for phase in 0..FLOW_PHASES {
                    for bt in [BakeTile::River, BakeTile::Ford] {
                        if !tiles.contains(&bt) {
                            continue;
                        }
                        let idx = FLOW_BASE_COL
                            + (if bt == BakeTile::Ford { FLOW_PHASES } else { 0 })
                            + phase;
                        let seed = variant_seed(bt) ^ (0x9e37_79b9 * (phase as u64 + 1));
                        let faces = bake3d::bake_iso(bt, seed);
                        for (fr, face) in faces.iter().enumerate() {
                            blit(&mut iso_wide, face, idx * TS, fr as u32 * TS);
                        }
                        println!("ATLAS BAKE OK {bt:?}.v{} -> iso[{idx},0..2]", phase + 2);
                    }
                }
            }
            if wants_tall {
                let [upper, lower] = bake3d::bake_wall_tall(bake_seed(BakeTile::Wall));
                blit(&mut iso_wide, &upper, TALL_COL * TS, 3 * TS);
                blit(&mut iso_wide, &lower, TALL_COL * TS, 4 * TS);
                println!("ATLAS BAKE OK Wall.tall -> iso[{TALL_COL},3..4] (2-cell curtain)");
            }
            if wants_materials {
                for (family, name) in bake3d::MATERIAL_FAMILIES.iter().enumerate() {
                    for offset in 0..3 {
                        let faces = bake3d::bake_material_iso(offset == 0, family, 0x5eed + offset as u64);
                        let col = MATERIAL_BASE_COL + family as u32 * 3 + offset;
                        for (fr, face) in faces.iter().enumerate() {
                            blit(&mut iso_wide, face, col * TS, fr as u32 * TS);
                        }
                    }
                    println!("ATLAS BAKE OK material {name} -> iso[{},0..2] wall/floor/floor2", MATERIAL_BASE_COL + family as u32 * 3);
                }
            }
            iso_wide.save(&iso_path).map_err(|e| e.to_string())?;
            write_iso_states(root, &tiles)?;
        }
        square.save(&sq_path).map_err(|e| e.to_string())?;
        if !(wants_variants || wants_tall || wants_materials) {
            // States-expanded writes already saved the wide sheet inside the
            // block above; saving the narrow image again would clobber it.
            iso.save(&iso_path).map_err(|e| e.to_string())?;
        }
        println!("ATLAS OK {} + {}", sq_path.display(), iso_path.display());
    }
    if !props_l.is_empty() {
        let props_path = root.join("props.png");
        let mut props_img = image::open(&props_path)
            .map_err(|e| format!("{e} (run `atlas props` first)"))?
            .to_rgba8();
        for bp in props_l.iter().copied() {
            let key = match bp {
                BakeProp::RelicPedestal => "RelicPedestal",
            };
            let col = props::prop_plates()
                .iter()
                .position(|(k, _)| *k == key)
                .ok_or_else(|| format!("prop key {key} not in painter roster"))?
                as u32;
            let plate = actors::chunky(&bake3d::bake_prop(bp), 3);
            blit(&mut props_img, &plate, col * TS, 0);
            println!("ATLAS BAKE OK {key} -> props[{col},0]");
        }
        props_img.save(&props_path).map_err(|e| e.to_string())?;
        println!("ATLAS OK {}", props_path.display());
    }
    bake_contact(&tiles[..], &props_l[..])?;
    if !tiles.is_empty() {
        // Ridge-variant rows (baked v1 | baked v2 pairs) whenever a states
        // bake ran, plus the world-map judgment view.
        if tiles.iter().any(|bt| VARIANT_TILES.contains(bt)) {
            variant_contact()?;
        }
        world_contact()?;
    }
    verify(root)
}

/// Painted vs baked side-by-side for the honest review pass.
fn bake_contact(tiles: &[BakeTile], props_l: &[BakeProp]) -> Result<(), String> {
    const SLOT: u32 = 140;
    let tile_rows = tiles.len();
    let rows = (tile_rows + props_l.len()) as u32;
    let mut shot = blank(4 * SLOT, rows.max(1) * SLOT);
    let (w, h) = (shot.width() as f32, shot.height() as f32);
    raster::fill_rect(&mut shot, 0.0, 0.0, w, h, [24, 26, 32]);
    println!("Bake contact legend per tile row: painted-square | baked-square | painted-iso | baked-iso");
    println!("                                 prop row: painted-prop | baked-prop");
    for (y0, bt) in tiles.iter().enumerate() {
        let (t, idx) = tile_bake_info(*bt);
        let oy = y0 as u32 * SLOT + 4;
        let seed = bake_seed(*bt);
        // Painted references from the live painter code.
        let painted_sq = paint::square_tile(t, seed_of(idx as u32, 0));
        let painted_iso = paint::iso_faces(t, seed_of(idx as u32, 0));
        // Fresh bakes.
        let baked_sq = bake3d::bake_square(*bt, seed);
        let baked_iso = bake3d::bake_iso(*bt, seed);
        let mut painted_block = blank(TS, TS);
        blit_blend(&mut painted_block, &painted_iso[1], 0, 0);
        blit_blend(&mut painted_block, &painted_iso[2], 0, 0);
        blit_blend(&mut painted_block, &painted_iso[0], 0, 0);
        let mut baked_block = blank(TS, TS);
        blit_blend(&mut baked_block, &baked_iso[1], 0, 0);
        blit_blend(&mut baked_block, &baked_iso[2], 0, 0);
        blit_blend(&mut baked_block, &baked_iso[0], 0, 0);
        blit_blend(&mut shot, &painted_sq, 4, oy);
        blit_blend(&mut shot, &baked_sq, SLOT + 4, oy);
        blit_blend(&mut shot, &painted_block, 2 * SLOT + 4, oy);
        blit_blend(&mut shot, &baked_block, 3 * SLOT + 4, oy);
        println!("  row {y0}: {t:?}");
    }
    for (j, bp) in props_l.iter().enumerate() {
        let y0 = tile_rows + j;
        let oy = y0 as u32 * SLOT + 4;
        let key = match bp {
            BakeProp::RelicPedestal => "RelicPedestal",
        };
        let painted = props::prop_plates()
            .into_iter()
            .find(|(k, _)| *k == key)
            .unwrap()
            .1;
        let baked = actors::chunky(&bake3d::bake_prop(*bp), 3);
        blit_blend(&mut shot, &painted, 4, oy);
        blit_blend(&mut shot, &baked, SLOT + 4, oy);
        println!("  row {y0}: {key} (prop)");
    }
    let out = PathBuf::from("docs/gfx/proto/atlas-baked-terrain-contact.png");
    let big = upscale2(&shot);
    big.save(&out).map_err(|e| e.to_string())?;
    println!("ATLAS OK {} ({}x{})", out.display(), big.width(), big.height());
    Ok(())
}

/// `bake [--tiles a,b] [--props p,q]` — a bare comma list means tiles.
fn parse_bake_args(rest: &[String]) -> Result<(Vec<BakeTile>, Vec<BakeProp>), String> {
    let mut tiles: Vec<BakeTile> = Vec::new();
    let mut props_l: Vec<BakeProp> = Vec::new();
    let mut tile_names: Vec<String> = Vec::new();
    let mut prop_names: Vec<String> = Vec::new();
    let mut i = 0;
    if rest.is_empty() {
        tile_names.push("Mountain,DeepForest".into());
    }
    while i < rest.len() {
        match rest[i].as_str() {
            "--tiles" => {
                i += 1;
                tile_names.push(rest.get(i).ok_or("--tiles needs a list")?.clone());
            }
            "--props" => {
                i += 1;
                prop_names.push(rest.get(i).ok_or("--props needs a list")?.clone());
            }
            bare if !bare.starts_with("--") => tile_names.push(bare.to_string()),
            other => return Err(format!("unknown bake flag {other}")),
        }
        i += 1;
    }
    for group in tile_names {
        for n in group.split(',') {
            tiles.push(match n.trim() {
                "Mountain" => BakeTile::Mountain,
                "DeepForest" => BakeTile::DeepForest,
                "Forest" => BakeTile::Forest,
                "Rock" => BakeTile::Rock,
                "Wall" => BakeTile::Wall,
                "River" => BakeTile::River,
                "Ford" => BakeTile::Ford,
                "Ruins" => BakeTile::Ruins,
                "Floor" => BakeTile::Floor,
                other => return Err(format!("unknown bake tile {other}")),
            });
        }
    }
    for group in prop_names {
        for n in group.split(',') {
            props_l.push(match n.trim() {
                "RelicPedestal" => BakeProp::RelicPedestal,
                other => return Err(format!("unknown bake prop {other}")),
            });
        }
    }
    tiles.dedup();
    props_l.dedup();
    Ok((tiles, props_l))
}

/// ROLLBACK-SAFE rebuild: refresh the two terrain sheets as painted
/// (clearing iso states extensions: variants/tall rows) while PRESERVING
/// every other sheet in the manifest (actors/props/chrome mappings live
/// with their own batches — `atlas terrain` must never clobber them).
/// If the manifest doesn't exist yet, construct a fresh v1 from scratch.
fn cmd_terrain(root: &Path) -> Result<(), String> {
    let terrain_dir = root.join("terrain");
    std::fs::create_dir_all(&terrain_dir).map_err(|e| e.to_string())?;

    let (square, sq_map) = bake_square_sheet();
    let sq_path = terrain_dir.join("square.png");
    square.save(&sq_path).map_err(|e| e.to_string())?;
    println!("ATLAS OK {} ({}x{})", sq_path.display(), square.width(), square.height());

    let (iso, iso_map) = bake_iso_sheet();
    let iso_path = terrain_dir.join("iso.png");
    iso.save(&iso_path).map_err(|e| e.to_string())?;
    println!("ATLAS OK {} ({}x{})", iso_path.display(), iso.width(), iso.height());

    // Actors/props are follow-up batches; emit transparent placeholders so
    // the renderer's colour-quad fallback never errors on a missing file.
    for stub in ["actors.png", "props.png"] {
        let p = root.join(stub);
        if !p.exists() {
            placeholder().save(&p).map_err(|e| e.to_string())?;
            println!("ATLAS OK {} (1x1 placeholder)", p.display());
        }
    }

    let man_path = root.join("manifest.json");
    let mut manifest: Manifest = std::fs::read_to_string(&man_path)
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_else(|| Manifest {
            tile_size: TS,
            sheets: vec![
                Sheet {
                    name: "terrain-square".into(),
                    file: "terrain/square.png".into(),
                    cell_size: None,
                    faces: vec![],
                    mapping: Default::default(),
                    states: None,
                },
                Sheet {
                    name: "terrain-iso".into(),
                    file: "terrain/iso.png".into(),
                    cell_size: None,
                    faces: vec!["top".into(), "left".into(), "right".into()],
                    mapping: Default::default(),
                    states: None,
                },
                Sheet {
                    name: "actors".into(),
                    file: "actors.png".into(),
                    cell_size: None,
                    faces: vec![],
                    mapping: Default::default(),
                    states: Some(Default::default()),
                },
                Sheet {
                    name: "props".into(),
                    file: "props.png".into(),
                    cell_size: None,
                    faces: vec![],
                    mapping: Default::default(),
                    states: None,
                },
            ],
        });
    for sheet in manifest.sheets.iter_mut() {
        match sheet.name.as_str() {
            "terrain-square" => {
                sheet.faces = vec![];
                sheet.mapping = sq_map.clone();
                sheet.states = None;
            }
            "terrain-iso" => {
                sheet.faces = vec!["top".into(), "left".into(), "right".into()];
                sheet.mapping = iso_map.clone();
                // Rollback: variants/tall rows cleared with the painted pass.
                sheet.states = None;
            }
            _ => {} // actors/props/chrome keep their own batches' mappings
        }
    }
    manifest.tile_size = TS;
    let json = serde_json::to_string_pretty(&manifest).map_err(|e| e.to_string())?;
    std::fs::write(&man_path, &json).map_err(|e| e.to_string())?;
    println!("ATLAS OK {}", man_path.display());

    verify(root)
}

/// Self-check: parse the manifest back and re-decode every sheet it names.
/// Actors/props may still be placeholders (1x1, empty mapping) before their
/// batches run; once populated they must be complete.
fn verify(root: &Path) -> Result<(), String> {
    let raw = std::fs::read_to_string(root.join("manifest.json")).map_err(|e| e.to_string())?;
    let man: Manifest = serde_json::from_str(&raw).map_err(|e| format!("manifest parse: {e}"))?;
    if man.tile_size != TS {
        return Err(format!("tile_size {} != {TS}", man.tile_size));
    }
    let mut seen = BTreeMap::new();
    for sheet in &man.sheets {
        let (w, h) = match sheet.name.as_str() {
            "terrain-square" if sheet.states.as_ref().is_some_and(|s| s.contains_key("Door")) =>
                (12 * TS, SQ_ROWS * TS),
            "terrain-square" => (SQ_COLS * TS, SQ_ROWS * TS),
            "terrain-iso" => {
                // States-expanded sheets (ridge variants / Wall.tall) grow
                // to the wide grid; plain rollback sheets stay 16x3.
                if sheet
                    .states
                    .as_ref()
                    .is_some_and(|s| !s.is_empty())
                {
                    (ISO_WIDE_W, ISO_WIDE_H)
                } else {
                    (ISO_COLS * TS, FACE_ROWS * TS)
                }
            }
            "doors" => (4 * 96, 96),
            "actors" if sheet.mapping.is_empty() => (1, 1),
            "actors" if sheet.cell_size.is_some() => {
                // A roster-v2 actors sheet is a packed, larger-cell animation
                // atlas, not the original 8x5 legacy sprite grid.
                let cell = sheet.cell_size.unwrap();
                let cols = sheet.mapping.values().map(|p| p[0] + 1).max().unwrap_or(0);
                let rows = sheet.mapping.values().map(|p| p[1] + 1).max().unwrap_or(0);
                (cols * cell, rows * cell)
            }
            "actors" => (ACTOR_COLS * TS, ACTOR_ROWS * TS),
            "players" if sheet.mapping.is_empty() => (1, 1),
            "players" => (PLAYER_COLS * TS, PLAYER_ROWS * TS),
            // Relic pendants: 9 relics x 2 builds, one transparent cell each.
            "player_relics" => (RELIC_COLS * TS, RELIC_ROWS * TS),
            "props" if sheet.mapping.is_empty() => (1, 1),
            "props" => (PROP_COLS * TS, TS),
            // Chrome lane is optional until `atlas chrome` runs; when present
            // it must be the full 8-col family at 3 rows.
            "chrome" => (CHROME_COLS * TS, 3 * TS),
            other => return Err(format!("unknown sheet {other}")),
        };
        let img = image::open(root.join(&sheet.file))
            .map_err(|e| format!("{}: {e}", sheet.file))?;
        if img.width() != w || img.height() != h {
            return Err(format!(
                "{}: dims {}x{} != {w}x{h}",
                sheet.file,
                img.width(),
                img.height()
            ));
        }
        if sheet.name.starts_with("terrain") {
            for t in TILES {
                if !sheet.mapping.contains_key(&debug_name(t)) {
                    return Err(format!("{}: missing {}", sheet.name, debug_name(t)));
                }
            }
            // Plain key must ALWAYS stay present alongside states rows
            // (backward compat: consumers that ignore states must not lose
            // the tile). Extra states keys are tolerated as additive lookups.
            if sheet.name == "terrain-iso" {
                if let Some(states) = sheet.states.as_ref().filter(|s| !s.is_empty()) {
                    if states.contains_key("Wall") && !sheet.mapping.contains_key("Wall") {
                        return Err("terrain-iso: states rows for Wall but plain Wall key missing".into());
                    }
                }
            }
        }
        if sheet.name == "actors" && !sheet.mapping.is_empty() {
            for a in ARCHETYPES {
                if a == Archetype::BroodHole {
                    continue; // prop decal
                }
                if !sheet.mapping.contains_key(&debug_name(a)) {
                    return Err(format!("actors: missing {}", debug_name(a)));
                }
            }
            if sheet.cell_size.is_none() {
                let states = sheet.states.as_ref().ok_or("actors: states missing")?;
                let players = states
                    .get("Player")
                    .and_then(|v| v.as_object())
                    .ok_or("actors: states.Player missing")?;
                for c in CLASSES {
                    if !players.contains_key(&debug_name(c)) {
                        return Err(format!("actors: states.Player missing {}", debug_name(c)));
                    }
                }
                let stag = states
                    .get("PaleStag")
                    .and_then(|v| v.as_object())
                    .ok_or("actors: states.PaleStag missing")?;
                if !stag.contains_key("at_bay") {
                    return Err("actors: states.PaleStag missing at_bay".into());
                }
            }
        }
        if sheet.name == "player_relics" && !sheet.mapping.is_empty() {
            for relic in RELIC_NAMES {
                for build in ["Male", "Female"] {
                    let key = format!("Player.Relic.{relic}.{build}");
                    if !sheet.mapping.contains_key(&key) {
                        return Err(format!("player_relics: missing {key}"));
                    }
                }
            }
        }
        if sheet.name == "players" && !sheet.mapping.is_empty() {
            // One bare plate and the full 4x4 weapon/armour matrix per class per
            // build. A missing build, or a missing corner of the matrix, is the
            // exact bug this sheet exists to prevent.
            for c in CLASSES {
                for build in ["Male", "Female"] {
                    let base = format!("Player.{}.{build}", debug_name(c));
                    if !sheet.mapping.contains_key(&base) {
                        return Err(format!("players: missing {base}"));
                    }
                    for w in 0..4 {
                        for a in 0..4 {
                            let key = format!("{base}.W{w}A{a}");
                            if !sheet.mapping.contains_key(&key) {
                                return Err(format!("players: missing {key}"));
                            }
                        }
                    }
                }
            }
        }
        seen.insert(sheet.name.clone(), sheet.mapping.len());
    }
    for name in ["terrain-square", "terrain-iso", "actors", "props", "players", "player_relics"] {
        if !seen.contains_key(name) {
            return Err(format!("manifest missing sheet {name}"));
        }
    }
    println!(
        "ATLAS VERIFY OK manifest.json: {} sheets, {} tiles x2 terrain, {} actors, {} props",
        man.sheets.len(),
        TILES.len(),
        seen.get("actors").unwrap_or(&0),
        seen.get("props").unwrap_or(&0)
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// preview: contact sheet of every tile in both projections.

fn upscale2(src: &RgbaImage) -> RgbaImage {
    RgbaImage::from_fn(src.width() * 2, src.height() * 2, |x, y| {
        *src.get_pixel(x / 2, y / 2)
    })
}

fn cmd_preview(out: &Path) -> Result<(), String> {
    const COLS: usize = 4;
    const SLOT: u32 = 140; // 64 square + 4 gap + 64 iso-top + padding
    const ROW_H: u32 = 72;
    let rows = TILES.len().div_ceil(COLS) as u32;
    let mut shot = blank(COLS as u32 * SLOT, rows * ROW_H);
    // Neutral backdrop (gfxlab sheet bg family).
    let (w, h) = (shot.width() as f32, shot.height() as f32);
    raster::fill_rect(&mut shot, 0.0, 0.0, w, h, [24, 26, 32]);
    let mut labels: Vec<String> = Vec::new();
    for (i, t) in TILES.iter().enumerate() {
        let (c, r) = (i % COLS, i / COLS);
        let (ox, oy) = (c as u32 * SLOT + 4, r as u32 * ROW_H + 4);
        let square = paint::square_tile(*t, seed_of(i as u32, 0));
        let [top, left, right] = paint::iso_faces(*t, seed_of(i as u32, 0));
        // Composite the iso block exactly as E4 will: sides under top.
        let mut block = blank(TS, TS);
        blit_blend(&mut block, &left, 0, 0);
        blit_blend(&mut block, &right, 0, 0);
        blit_blend(&mut block, &top, 0, 0);
        blit_blend(&mut shot, &square, ox, oy);
        blit_blend(&mut shot, &block, ox + TS + 4, oy);
        labels.push(debug_name(*t));
    }
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let big = upscale2(&shot);
    big.save(out).map_err(|e| e.to_string())?;
    println!("ATLAS OK {} ({}x{})", out.display(), big.width(), big.height());
    println!("Contact legend (row-major, square | iso-block), left -> right:");
    for chunk in labels.chunks(COLS) {
        println!("  {}", chunk.join("  "));
    }
    Ok(())
}

/// Src-over alpha paste (blank sheets have transparent margins).
fn blit_blend(dst: &mut RgbaImage, src: &RgbaImage, ox: u32, oy: u32) {
    for (x, y, p) in src.enumerate_pixels() {
        let a = p[3] as u32;
        if a == 0 {
            continue;
        }
        let d = dst.get_pixel_mut(ox + x, oy + y);
        let dd = d.0;
        let oa = a + dd[3] as u32 * (255 - a) / 255;
        let mix = |s: u8, dc: u8| -> u8 {
            ((s as u32 * a * 255 + dc as u32 * dd[3] as u32 * (255 - a)) / (oa.max(1) * 255)) as u8
        };
        *d = Rgba([mix(p[0], dd[0]), mix(p[1], dd[1]), mix(p[2], dd[2]), oa.min(255) as u8]);
    }
}

/// Actors + props contact: every archetype and class hero over a grim Grass
/// plate (the env they must read against), props over Floor.
fn cmd_preview_actors(out: &Path) -> Result<(), String> {
    const SLOT: u32 = 72;
    let grass = paint::square_tile(Tile::Grass, 1);
    let floor = paint::square_tile(Tile::Floor, 1);
    let mut shot = blank(ACTOR_COLS * SLOT, 6 * SLOT);
    let (w, h) = (shot.width() as f32, shot.height() as f32);
    raster::fill_rect(&mut shot, 0.0, 0.0, w, h, [24, 26, 32]);
    let (sheet, mapping, variants) = bake_actor_sheet();
    let mut labels: Vec<String> = Vec::new();
    for (key, [col, row]) in mapping.iter() {
        let ox = col * SLOT + 4;
        let oy = row * SLOT + 4;
        blit_blend(&mut shot, &grass, ox, oy);
        blit_blend(&mut shot, &RgbaImage::from_fn(TS, TS, |x, y| {
            *sheet.get_pixel(col * TS + x, row * TS + y)
        }), ox, oy);
        labels.push(key.clone());
    }
    // States variants sit on plain neutral (proof of the group mechanic).
    for (group, inner) in &variants {
        for (name, [col, row]) in inner {
            if group == "Player" {
                continue;
            }
            let ox = col * SLOT + 4;
            let oy = row * SLOT + 4;
            blit_blend(&mut shot, &grass, ox, oy);
            blit_blend(&mut shot, &RgbaImage::from_fn(TS, TS, |x, y| {
                *sheet.get_pixel(col * TS + x, row * TS + y)
            }), ox, oy);
            labels.push(format!("{group}.{name}"));
        }
    }
    let (prop_sheet, prop_map) = bake_prop_sheet();
    let mut prop_labels: Vec<String> = Vec::new();
    for (key, [col, _]) in &prop_map {
        let ox = col * SLOT + 4;
        let oy = 5 * SLOT + 4;
        blit_blend(&mut shot, &floor, ox, oy);
        blit_blend(&mut shot, &RgbaImage::from_fn(TS, TS, |x, y| {
            *prop_sheet.get_pixel(col * TS + x, y)
        }), ox, oy);
        prop_labels.push(key.clone());
    }
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let big = upscale2(&shot);
    big.save(out).map_err(|e| e.to_string())?;
    println!("ATLAS OK {} ({}x{})", out.display(), big.width(), big.height());
    println!("Actor legend (row-major over grass; players row 4 cols 1-6):");
    println!("  row0: Commoner Vendor Guard Thief Traveller Bandit Wolf Bear");
    println!("  row1: Rat Skeleton Chief Matriarch Lich Adjudicator Smuggler Oracle");
    println!("  row2: Companion Tidemother Cragmother GnawThane Tollmaster Mirelight PaleStag Alchemist");
    println!("  row3: (BroodHole=prop) FalseGlow OathlessCurate <PaleStag.at_bay>");
    println!("  row4: {} {} {} {} {} {} (players)", class_name(0), class_name(1), class_name(2), class_name(3), class_name(4), class_name(5));
    println!("  row5 (props): {}", prop_labels.join(" "));
    let _ = labels;
    Ok(())
}

fn class_name(i: usize) -> String {
    debug_name(CLASSES[i])
}

/// E5/E7 deliverables contact: the new bosses/NPCs over grass, the at_bay
/// states pair side-by-side, and the BroodHole prop decal on Floor.
fn cmd_preview_e5e7(out: &Path) -> Result<(), String> {
    const SLOT: u32 = 72;
    let grass = paint::square_tile(Tile::Grass, 1);
    let floor = paint::square_tile(Tile::Floor, 1);
    let mut shot = blank(4 * SLOT, 3 * SLOT);
    let (w, h) = (shot.width() as f32, shot.height() as f32);
    raster::fill_rect(&mut shot, 0.0, 0.0, w, h, [24, 26, 32]);
    // Row 0: the four humanoid/civvy reads. Row 1: quadrupeds + at_bay pair.
    // Row 2: BroodHole prop on Floor.
    let layout: [(usize, usize, Archetype); 7] = [
        (0, 0, Archetype::Tollmaster),
        (1, 0, Archetype::Alchemist),
        (2, 0, Archetype::Mirelight),
        (3, 0, Archetype::OathlessCurate),
        (0, 1, Archetype::GnawThane),
        (1, 1, Archetype::PaleStag),
        (3, 1, Archetype::FalseGlow),
    ];
    for (cx, cy, a) in layout {
        let plate = actors::actor_plate(actors::accent(a), &ActorFigure::Npc(a));
        let ox = cx as u32 * SLOT + 4;
        let oy = cy as u32 * SLOT + 4;
        blit_blend(&mut shot, &grass, ox, oy);
        blit_blend(&mut shot, &plate, ox, oy);
    }
    // The proof-case pair: grazing vs at_bay stag, side by side.
    let bayed = actors::actor_plate(
        actors::accent(Archetype::PaleStag),
        &ActorFigure::NpcState(Archetype::PaleStag, "at_bay"),
    );
    blit_blend(&mut shot, &grass, 2 * SLOT + 4, SLOT + 4);
    blit_blend(&mut shot, &bayed, 2 * SLOT + 4, SLOT + 4);
    // Prop maw.
    let maw = props::prop_plates()
        .into_iter()
        .find(|(k, _)| *k == "BroodHole")
        .unwrap()
        .1;
    blit_blend(&mut shot, &floor, 4, 2 * SLOT + 4);
    blit_blend(&mut shot, &maw, 4, 2 * SLOT + 4);
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let big = upscale2(&shot);
    big.save(out).map_err(|e| e.to_string())?;
    println!("ATLAS OK {} ({}x{})", out.display(), big.width(), big.height());
    println!("E5/E7 legend: row0: Tollmaster Alchemist Mirelight OathlessCurate");
    println!("              row1: GnawThane | PaleStag grazing | PaleStag.at_bay | FalseGlow");
    println!("              row2: BroodHole (prop, on Floor)");
    Ok(())
}

/// Patch only door art, retaining other production terrain/actor/prop cells.
fn cmd_doors(root: &Path) -> Result<(), String> {
    let path = root.join("manifest.json");
    let mut man: Manifest = serde_json::from_str(
        &std::fs::read_to_string(&path).map_err(|e| e.to_string())?,
    ).map_err(|e| e.to_string())?;
    let square = man.sheets.iter_mut().find(|s| s.name == "terrain-square").ok_or("missing square sheet")?;
    let old = image::open(root.join(&square.file)).map_err(|e| e.to_string())?.to_rgba8();
    let [col, row] = square.mapping["Floor"];
    let floor = image::imageops::crop_imm(&old, col * TS, row * TS, TS, TS).to_image();
    let mut img = blank(old.width().max(12 * TS), old.height());
    blit(&mut img, &old, 0, 0);
    let mut states = serde_json::Map::new();
    let mut door_img = blank(4 * 96, 96);
    let mut door_mapping = BTreeMap::new();
    for (pose, (open, along_x)) in [(false, true), (true, true), (false, false), (true, false)].into_iter().enumerate() {
        let key = ["closed_x", "open_x", "closed_y", "open_y"][pose];
        let plate = paint::door_plate(&floor, false, open, along_x);
        blit(&mut img, &plate, (8 + pose as u32) * TS, 0);
        if pose == 0 {
            let [x, y] = square.mapping["Door"];
            blit(&mut img, &plate, x * TS, y * TS);
        }
        for zone in bake3d::MATERIAL_FAMILIES {
            for phase in 0..2 {
                states.insert(format!("{zone}{phase}_{key}"), serde_json::json!([8 + pose, 0]));
            }
        }
        let standing = paint::door_plate(&blank(96, 96), true, open, along_x);
        blit(&mut door_img, &standing, pose as u32 * 96, 0);
        door_mapping.insert(key.into(), [pose as u32, 0]);
    }
    square.states.get_or_insert_with(serde_json::Map::new).insert("Door".into(), serde_json::Value::Object(states));
    img.save(root.join(&square.file)).map_err(|e| e.to_string())?;
    // Ground is now the renderer's existing Floor.<zone> phase, unchanged.
    if let Some(iso) = man.sheets.iter_mut().find(|s| s.name == "terrain-iso") {
        if iso.states.as_mut().is_some_and(|states| states.remove("Door").is_some()) {
            let old = image::open(root.join(&iso.file)).map_err(|e| e.to_string())?.to_rgba8();
            image::imageops::crop_imm(&old, 0, 0, ISO_WIDE_W, old.height()).to_image()
                .save(root.join(&iso.file)).map_err(|e| e.to_string())?;
        }
    }
    door_img.save(root.join("doors.png")).map_err(|e| e.to_string())?;
    man.sheets.retain(|s| s.name != "doors");
    man.sheets.push(Sheet {
        name: "doors".into(), file: "doors.png".into(), cell_size: Some(96),
        faces: Vec::new(), mapping: door_mapping, states: None,
    });
    std::fs::write(&path, serde_json::to_string_pretty(&man).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    println!("ATLAS OK doors: 96px standing closed/open, two wall orientations; original ground and other sheets retained");
    Ok(())
}


fn main() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        Some("terrain") => cmd_terrain(Path::new("assets/atlas")),
        Some("actors") => cmd_actors(Path::new("assets/atlas")),
        Some("props") => cmd_props(Path::new("assets/atlas")),
        Some("chrome") => cmd_chrome(Path::new("assets/atlas")),
        Some("doors") => cmd_doors(Path::new("assets/atlas")),
        Some("bake") => {
            let rest: Vec<String> = args.collect();
            if rest.iter().any(|a| a == "--actors") {
                cmd_bake_actors(Path::new("assets/atlas"))
            } else {
                let (tiles, props_l) = parse_bake_args(&rest)?;
                cmd_bake(Path::new("assets/atlas"), tiles, props_l)
            }
        }
        Some("preview") => {
            cmd_preview(PathBuf::from("docs/gfx/proto/atlas-terrain-contact.png").as_path())?;
            cmd_preview_actors(PathBuf::from("docs/gfx/proto/atlas-actors-contact.png").as_path())?;
            cmd_preview_e5e7(PathBuf::from("docs/gfx/proto/atlas-actors-e5e7-contact.png").as_path())
        }
        other => Err(format!(
            "usage: atlas <terrain|actors|props|chrome|doors|bake|preview> (got {})",
            other.unwrap_or("<none>")
        )),
    }
}
