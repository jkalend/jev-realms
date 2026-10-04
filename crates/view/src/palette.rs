//! Colors and the sim-exact light math.
//!
//! Environment palette: `src/sprites.rs` / `gfxlab.rs` `terrain_color` verbatim —
//! the grim environment set D28 ratified (§7.5★ evidence read photographic with
//! these values + EXPOSURE). Accent colors (text, pulse, figures) come from
//! `src/gui.rs` `color(Ink)` — the exact Ink→rgb table, ported as constants.
//!
//! Light: `src/sprites.rs` `light()` bands 1.0/0.55 + memory gray, vision radius
//! from `src/ui.rs` `vision()` / `engine.rs` `reveal()` (12 day / 6 torch / 4 night).

use bevy::prelude::*;
use laya_realms::model::{Game, Tile};

/// E0 spike exposure lift (§7.5★): brightens for screen, tier RELATIONSHIPS untouched.
pub const EXPOSURE: f32 = 2.2;

// --- src/gui.rs `color(Ink)` values, verbatim --------------------------------

/// Ink::Reset | Ink::Black — the dungeon void; also the window clear color.
pub const INK_BLACK: (u8, u8, u8) = (10, 13, 18);
/// Ink::LightYellow — floating text crumbs (ttl-faded).
pub const INK_LIGHT_YELLOW: (u8, u8, u8) = (249, 215, 128);
/// Ink::Red — combat bubble frame pulse (placeholder until E4-s4 chrome).
pub const INK_RED: (u8, u8, u8) = (180, 58, 64);

pub fn srgb8(rgb: (u8, u8, u8)) -> Color {
    Color::srgb_u8(rgb.0, rgb.1, rgb.2)
}

/// `materials_color` for the C-hybrid "lifted" figures: player gold from the spike.
pub const FIGURE_GOLD: (u8, u8, u8) = (235, 200, 90);

/// src/sprites.rs `terrain_color`, verbatim — the game's existing palette.
pub fn terrain_rgb(t: Tile) -> (u8, u8, u8) {
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

/// src/ui.rs `vision()` / engine `reveal()` radius: 12 day, 6 torchlit night,
/// 4 unlit night. Torchlight clears six tiles at night, never the daylight radius.
pub fn vision_radius(game: &Game) -> i32 {
    if game.night() {
        if game.tick < game.player.torch_until { 6 } else { 4 }
    } else {
        12
    }
}

/// src/sprites.rs `light()` band then deep-forest damp; `remembered` when the
/// sim fog tier flips to memory (distance beyond the current radius).
pub fn tier(dist: i32, radius: i32, tile: Tile) -> (f32, bool) {
    let strength = if dist * 4 <= radius * 3 { 1.0 } else { 0.55 }
        * if tile == Tile::DeepForest { 0.9 } else { 1.0 };
    (strength, dist > radius)
}

/// src/sprites.rs `light()` verbatim + the E0 exposure lift. `remembered` is the
/// desaturated gray memory tier; tier 0 is full warmth, rim band is 0.55.
pub fn lit(base: (u8, u8, u8), strength: f32, remembered: bool) -> Color {
    let (r, g, b) = (
        base.0 as f32 / 255.0,
        base.1 as f32 / 255.0,
        base.2 as f32 / 255.0,
    );
    let (r, g, b) = if remembered {
        let gray = (r * 0.30 + g * 0.59 + b * 0.11) * 0.32;
        (gray * 0.92, gray, gray * 1.05)
    } else {
        (r * strength, g * strength, b * strength)
    };
    Color::srgb(
        (r * EXPOSURE).min(1.0),
        (g * EXPOSURE).min(1.0),
        (b * EXPOSURE).min(1.0),
    )
}
