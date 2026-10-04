use crate::model::{mix, Archetype, BossRelic, Game, Intent, Pos, Tile};
use macroquad::prelude::*;
use std::collections::HashMap;

const GOLD: Color = Color::new(0.91, 0.75, 0.35, 1.0);
const SEA: Color = Color::new(0.40, 0.83, 0.77, 1.0);
const INK: Color = Color::new(0.035, 0.043, 0.047, 1.0);
const RED: Color = Color::new(0.96, 0.35, 0.36, 1.0);

// Bound primitives rather than replacing the compositor's camera. In particular,
// macroquad's camera stack does not preserve a caller's viewport.
pub(crate) struct Canvas(pub(crate) Rect);

impl Canvas {
    pub(crate) fn rect(&self, r: Rect, color: Color) {
        let x = r.x.max(self.0.x);
        let y = r.y.max(self.0.y);
        let right = (r.x + r.w).min(self.0.x + self.0.w);
        let bottom = (r.y + r.h).min(self.0.y + self.0.h);
        if right > x && bottom > y {
            draw_rectangle(x, y, right - x, bottom - y, color);
        }
    }

    pub(crate) fn border(&self, r: Rect, width: f32, color: Color) {
        self.rect(Rect::new(r.x, r.y, r.w, width), color);
        self.rect(Rect::new(r.x, r.y + r.h - width, r.w, width), color);
        self.rect(Rect::new(r.x, r.y, width, r.h), color);
        self.rect(Rect::new(r.x + r.w - width, r.y, width, r.h), color);
    }

    pub(crate) fn text(&self, text: &str, x: f32, y: f32, size: u16, color: Color) {
        let x = x.max(self.0.x + 2.0);
        let available = self.0.x + self.0.w - x - 2.0;
        if available <= 0.0 {
            return;
        }
        let mut end = text.len();
        let mut dims = measure_text(text, None, size, 1.0);
        while end > 0 && dims.width > available {
            end = text[..end].char_indices().next_back().map_or(0, |(i, _)| i);
            dims = measure_text(&text[..end], None, size, 1.0);
        }
        if end == 0 || dims.height + 4.0 > self.0.h {
            return;
        }
        let top = y.clamp(self.0.y + 2.0, self.0.y + self.0.h - dims.height - 2.0);
        self.rect(
            Rect::new(x - 2.0, top - 2.0, dims.width + 4.0, dims.height + 4.0),
            INK,
        );
        draw_text(&text[..end], x, top + dims.offset_y, f32::from(size), color);
    }
}

pub(crate) fn terrain_color(tile: Tile) -> Color {
    match tile {
        Tile::Grass => Color::from_rgba(48, 76, 37, 255),
        Tile::Forest => Color::from_rgba(32, 61, 31, 255),
        Tile::DeepForest => Color::from_rgba(21, 43, 28, 255),
        Tile::Road => Color::from_rgba(153, 126, 78, 255),
        Tile::Floor => Color::from_rgba(44, 42, 39, 255),
        Tile::Wall => Color::from_rgba(66, 66, 61, 255),
        Tile::Mountain => Color::from_rgba(113, 114, 105, 255),
        Tile::Rock => Color::from_rgba(50, 55, 55, 255),
        Tile::River => Color::from_rgba(35, 82, 111, 255),
        Tile::Ford => Color::from_rgba(111, 127, 111, 255),
        Tile::Ruins => Color::from_rgba(109, 105, 80, 255),
        Tile::Door | Tile::Chest => Color::from_rgba(151, 111, 46, 255),
        Tile::Up | Tile::Down => Color::from_rgba(186, 174, 129, 255),
        Tile::Shrine => Color::from_rgba(108, 153, 171, 255),
    }
}

fn light(color: Color, strength: f32, remembered: bool) -> Color {
    if remembered {
        let gray = (color.r * 0.30 + color.g * 0.59 + color.b * 0.11) * 0.32;
        Color::new(gray * 0.92, gray, gray * 1.05, color.a)
    } else {
        Color::new(
            color.r * strength,
            color.g * strength,
            color.b * strength,
            color.a,
        )
    }
}

// Terrain is drawn on a 32-unit grid. Large, low-contrast shapes carry the
// material; coordinate hashing varies placement without touching game RNG.
fn tile_sprite(
    canvas: &Canvas,
    tile: Tile,
    pos: Pos,
    r: Rect,
    strength: f32,
    remembered: bool,
    tick: u64,
) {
    let pixel = r.w / 32.0;
    let paint = |x: f32, y: f32, w: f32, h: f32, color: Color| {
        canvas.rect(
            Rect::new(r.x + x * pixel, r.y + y * pixel, w * pixel, h * pixel),
            light(color, strength, remembered),
        );
    };
    let hash = mix((pos.x as u32 as u64) | ((pos.y as u32 as u64) << 32));
    let base = match tile {
        Tile::Door | Tile::Up | Tile::Down | Tile::Shrine | Tile::Chest => {
            terrain_color(Tile::Floor)
        }
        Tile::Ruins => terrain_color(Tile::Grass),
        _ => terrain_color(tile),
    };
    paint(0.0, 0.0, 32.0, 32.0, base);
    match tile {
        Tile::Grass => {
            let moss = Color::from_rgba(51, 79, 39, 255);
            paint(3.0, 7.0, 12.0, 6.0, moss);
            paint(19.0, 21.0, 10.0, 5.0, moss);
            for n in 0..3 {
                let x = 3.0 + ((hash >> (n * 12)) % 23) as f32;
                let y = 5.0 + ((hash >> (n * 12 + 5)) % 22) as f32;
                paint(x, y, 5.0, 2.0, Color::from_rgba(39, 66, 32, 255));
                paint(
                    x + 1.0,
                    y - 3.0,
                    2.0,
                    4.0,
                    Color::from_rgba(65, 89, 42, 255),
                );
                paint(
                    x + 3.0,
                    y - 1.0,
                    2.0,
                    3.0,
                    Color::from_rgba(75, 97, 47, 255),
                );
            }
        }
        Tile::Forest | Tile::DeepForest => {
            let deep = tile == Tile::DeepForest;
            let green = if deep {
                Color::from_rgba(34, 66, 46, 255)
            } else {
                Color::from_rgba(53, 91, 43, 255)
            };
            let shade = light(green, 0.70, false);
            let leaf = light(green, 1.15, false);
            paint(5.0, 26.0, 23.0, 4.0, Color::from_rgba(19, 36, 24, 255));
            paint(13.0, 17.0, 7.0, 12.0, Color::from_rgba(63, 47, 32, 255));
            paint(14.0, 18.0, 3.0, 10.0, Color::from_rgba(100, 74, 41, 255));
            paint(10.0, 28.0, 6.0, 2.0, Color::from_rgba(77, 56, 34, 255));
            paint(19.0, 27.0, 4.0, 3.0, Color::from_rgba(63, 47, 32, 255));
            if deep {
                // Tall overlapping cedar tiers, rather than a darker broadleaf.
                for (x, y, w, h) in [
                    (13.0, 1.0, 6.0, 5.0),
                    (9.0, 6.0, 14.0, 6.0),
                    (6.0, 12.0, 20.0, 6.0),
                    (3.0, 18.0, 26.0, 6.0),
                ] {
                    paint(x, y, w, h, shade);
                    paint(x + 1.0, y, w - 4.0, h - 2.0, green);
                    paint(x + 2.0, y + 1.0, w * 0.35, 2.0, leaf);
                }
                paint(23.0, 25.0, 5.0, 2.0, green);
                paint(25.0, 23.0, 2.0, 5.0, green);
            } else {
                // Separate crown lobes and hanging shaded undersides.
                paint(7.0, 4.0, 18.0, 17.0, shade);
                paint(3.0, 10.0, 26.0, 11.0, shade);
                paint(10.0, 2.0, 11.0, 6.0, green);
                paint(5.0, 7.0, 12.0, 11.0, green);
                paint(16.0, 6.0, 10.0, 12.0, green);
                paint(9.0, 15.0, 14.0, 8.0, green);
                paint(8.0, 7.0, 7.0, 3.0, leaf);
                paint(12.0, 4.0, 7.0, 2.0, leaf);
                paint(5.0, 12.0, 5.0, 3.0, leaf);
                paint(17.0, 16.0, 6.0, 2.0, shade);
            }
        }
        Tile::Wall => {
            // A coping slab sits above the darker vertical masonry face.
            paint(0.0, 0.0, 32.0, 7.0, Color::from_rgba(89, 88, 78, 255));
            paint(1.0, 1.0, 30.0, 2.0, Color::from_rgba(106, 103, 89, 255));
            paint(0.0, 7.0, 32.0, 2.0, Color::from_rgba(39, 41, 38, 255));
            for row in 0..3 {
                let y = 10.0 + row as f32 * 7.0;
                let seam = if row % 2 == 0 { 11.0 } else { 21.0 };
                paint(0.0, y + 5.0, 32.0, 2.0, Color::from_rgba(42, 43, 39, 255));
                paint(seam, y, 2.0, 5.0, Color::from_rgba(42, 43, 39, 255));
                paint(1.0, y, seam - 2.0, 1.0, Color::from_rgba(81, 80, 70, 255));
                paint(
                    seam + 3.0,
                    y,
                    28.0 - seam,
                    1.0,
                    Color::from_rgba(77, 77, 68, 255),
                );
            }
            paint(5.0, 3.0, 2.0, 3.0, Color::from_rgba(70, 72, 64, 255));
            paint(24.0, 24.0, 4.0, 2.0, Color::from_rgba(58, 63, 47, 255));
        }
        Tile::Road => {
            let dirt = Color::from_rgba(132, 111, 75, 255);
            paint(2.0, 11.0, 28.0, 3.0, dirt);
            paint(5.0, 26.0, 23.0, 2.0, dirt);
            for n in 0..5 {
                let x = 2.0 + ((hash >> (n * 10)) % 23) as f32;
                let y = 2.0 + ((hash >> (n * 10 + 5)) % 24) as f32;
                let w = 4.0 + ((hash >> (n * 10 + 8)) & 3) as f32;
                paint(x, y + 1.0, w, 4.0, Color::from_rgba(128, 113, 84, 255));
                paint(
                    x + 1.0,
                    y,
                    w - 1.0,
                    3.0,
                    Color::from_rgba(165, 143, 99, 255),
                );
                paint(
                    x + 1.0,
                    y,
                    w - 2.0,
                    1.0,
                    Color::from_rgba(177, 154, 110, 255),
                );
            }
        }
        Tile::Mountain | Tile::Rock => {
            let stone = terrain_color(tile);
            paint(3.0, 26.0, 27.0, 4.0, light(stone, 0.55, false));
            for (x, y, w, h) in [
                (11.0, 3.0, 10.0, 5.0),
                (7.0, 8.0, 18.0, 6.0),
                (4.0, 14.0, 25.0, 7.0),
                (2.0, 21.0, 28.0, 6.0),
            ] {
                paint(x, y, w, h, light(stone, 1.13, false));
                paint(18.0, y, x + w - 18.0, h, light(stone, 0.72, false));
                paint(x + 1.0, y, 17.0 - x, 2.0, light(stone, 1.25, false));
            }
            paint(12.0, 12.0, 2.0, 7.0, light(stone, 0.73, false));
            paint(9.0, 18.0, 5.0, 2.0, light(stone, 0.73, false));
            paint(20.0, 21.0, 7.0, 2.0, light(stone, 0.52, false));
            paint(5.0, 28.0, 5.0, 2.0, light(stone, 1.08, false));
            if tile == Tile::Mountain {
                paint(12.0, 3.0, 8.0, 2.0, Color::from_rgba(185, 186, 168, 255));
                paint(10.0, 7.0, 5.0, 2.0, Color::from_rgba(158, 161, 145, 255));
            } else {
                paint(4.0, 23.0, 5.0, 3.0, Color::from_rgba(53, 66, 52, 255));
            }
        }
        Tile::River | Tile::Ford => {
            paint(0.0, 0.0, 32.0, 32.0, terrain_color(Tile::River));
            paint(4.0, 6.0, 19.0, 9.0, Color::from_rgba(32, 75, 104, 255));
            paint(12.0, 19.0, 20.0, 8.0, Color::from_rgba(30, 72, 103, 255));
            for row in 0..3 {
                let phase = if remembered { 0 } else { tick / 4 };
                let x = 2.0 + ((hash.wrapping_add(phase).wrapping_add(row * 7)) % 18) as f32;
                let y = 5.0 + row as f32 * 10.0;
                paint(x, y, 9.0, 1.0, Color::from_rgba(59, 111, 134, 255));
                paint(
                    x + 3.0,
                    y + 2.0,
                    8.0,
                    1.0,
                    Color::from_rgba(44, 96, 122, 255),
                );
            }
            if tile == Tile::Ford {
                for n in 0..4 {
                    let x = n as f32 * 8.0;
                    let y = 12.0 + (n % 2) as f32 * 3.0;
                    paint(x, y + 4.0, 7.0, 4.0, Color::from_rgba(53, 86, 91, 255));
                    paint(
                        x + 1.0,
                        y + 1.0,
                        6.0,
                        5.0,
                        Color::from_rgba(128, 130, 107, 255),
                    );
                    paint(x + 2.0, y, 4.0, 3.0, Color::from_rgba(161, 154, 121, 255));
                    paint(x, y + 8.0, 7.0, 1.0, Color::from_rgba(74, 129, 143, 255));
                }
            }
        }
        Tile::Floor => {
            // Offset, unequal flagstones, with quiet bevels and occasional cracks.
            for (x, y, w, h) in [
                (1.0, 1.0, 17.0, 12.0),
                (20.0, 1.0, 11.0, 16.0),
                (1.0, 15.0, 10.0, 16.0),
                (13.0, 19.0, 18.0, 12.0),
            ] {
                let shade = if hash & 1 == 0 { 1.15 } else { 1.08 };
                paint(x, y, w, h, light(base, shade, false));
                paint(x, y, w, 1.0, light(base, 1.32, false));
                paint(x, y + h - 1.0, w, 1.0, light(base, 0.75, false));
            }
            if hash & 3 == 0 {
                paint(8.0, 2.0, 1.0, 4.0, light(base, 0.70, false));
                paint(9.0, 6.0, 3.0, 1.0, light(base, 0.70, false));
            }
        }
        Tile::Ruins => {
            let stone = terrain_color(Tile::Ruins);
            paint(3.0, 26.0, 26.0, 4.0, terrain_color(Tile::Rock));
            paint(5.0, 8.0, 7.0, 18.0, light(stone, 0.8, false));
            paint(5.0, 8.0, 3.0, 18.0, stone);
            paint(3.0, 6.0, 11.0, 4.0, light(stone, 1.2, false));
            paint(5.0, 17.0, 7.0, 2.0, terrain_color(Tile::Rock));
            paint(22.0, 16.0, 6.0, 10.0, stone);
            paint(20.0, 14.0, 8.0, 3.0, light(stone, 1.15, false));
            paint(14.0, 23.0, 5.0, 5.0, stone);
            paint(17.0, 21.0, 5.0, 3.0, light(stone, 0.8, false));
            paint(4.0, 23.0, 4.0, 4.0, Color::from_rgba(59, 80, 40, 255));
            paint(24.0, 19.0, 3.0, 5.0, Color::from_rgba(66, 85, 45, 255));
        }
        Tile::Door => {
            paint(3.0, 2.0, 26.0, 30.0, terrain_color(Tile::Wall));
            paint(4.0, 2.0, 24.0, 3.0, Color::from_rgba(109, 104, 85, 255));
            paint(7.0, 6.0, 18.0, 26.0, INK);
            paint(8.0, 7.0, 15.0, 25.0, terrain_color(Tile::Door));
            for x in [8.0, 13.0, 18.0] {
                paint(x, 8.0, 1.0, 23.0, Color::from_rgba(177, 131, 62, 255));
                paint(x + 4.0, 8.0, 1.0, 23.0, Color::from_rgba(102, 72, 37, 255));
            }
            for y in [11.0, 24.0] {
                paint(8.0, y, 15.0, 3.0, Color::from_rgba(60, 59, 49, 255));
                paint(9.0, y, 2.0, 1.0, Color::from_rgba(120, 116, 88, 255));
            }
            paint(19.0, 17.0, 3.0, 5.0, GOLD);
            paint(20.0, 18.0, 1.0, 2.0, INK);
            paint(3.0, 14.0, 4.0, 2.0, Color::from_rgba(43, 45, 40, 255));
        }
        Tile::Up | Tile::Down => {
            paint(3.0, 3.0, 26.0, 27.0, terrain_color(Tile::Wall));
            paint(5.0, 4.0, 22.0, 25.0, INK);
            for n in 0..5 {
                let inset = if tile == Tile::Down { n } else { 4 - n } as f32 * 2.0;
                let y = 5.0 + n as f32 * 5.0;
                let step = light(terrain_color(tile), 1.0 - n as f32 * 0.12, false);
                paint(
                    6.0 + inset,
                    y,
                    20.0 - inset * 2.0,
                    4.0,
                    light(step, 0.65, false),
                );
                paint(6.0 + inset, y, 20.0 - inset * 2.0, 2.0, step);
            }
            paint(3.0, 4.0, 2.0, 25.0, Color::from_rgba(125, 119, 96, 255));
        }
        Tile::Shrine => {
            paint(5.0, 27.0, 23.0, 3.0, INK);
            paint(5.0, 24.0, 22.0, 4.0, terrain_color(Tile::Wall));
            paint(7.0, 23.0, 18.0, 2.0, Color::from_rgba(130, 136, 124, 255));
            paint(11.0, 17.0, 10.0, 6.0, terrain_color(Tile::Ruins));
            paint(12.0, 18.0, 3.0, 4.0, Color::from_rgba(144, 140, 112, 255));
            paint(13.0, 7.0, 6.0, 12.0, light(SEA, 0.65, false));
            paint(15.0, 3.0, 3.0, 15.0, SEA);
            paint(15.0, 4.0, 1.0, 13.0, Color::from_rgba(190, 228, 207, 255));
            paint(7.0, 11.0, 18.0, 3.0, light(SEA, 0.8, false));
            paint(8.0, 11.0, 16.0, 1.0, SEA);
            paint(7.0, 20.0, 2.0, 3.0, GOLD);
            paint(23.0, 20.0, 2.0, 3.0, GOLD);
        }
        Tile::Chest => {
            paint(3.0, 13.0, 27.0, 15.0, INK);
            paint(5.0, 11.0, 22.0, 14.0, terrain_color(Tile::Chest));
            paint(7.0, 8.0, 18.0, 4.0, Color::from_rgba(180, 135, 66, 255));
            paint(5.0, 12.0, 22.0, 3.0, Color::from_rgba(166, 122, 53, 255));
            paint(24.0, 13.0, 3.0, 12.0, Color::from_rgba(102, 75, 36, 255));
            paint(5.0, 17.0, 22.0, 2.0, INK);
            paint(6.0, 22.0, 18.0, 1.0, Color::from_rgba(116, 81, 34, 255));
            for x in [8.0, 22.0] {
                paint(x, 9.0, 2.0, 16.0, light(GOLD, 0.8, false));
                paint(x, 10.0, 2.0, 2.0, GOLD);
                paint(x, 21.0, 2.0, 2.0, GOLD);
            }
            paint(14.0, 16.0, 5.0, 6.0, GOLD);
            paint(16.0, 18.0, 1.0, 3.0, INK);
        }
    }
}

// Neighbors are supplied only after the caller has checked exploration. Unknown
// ground must not contribute a shoreline or a road edge through the fog.
fn tile_edges(
    canvas: &Canvas,
    tile: Tile,
    r: Rect,
    strength: f32,
    remembered: bool,
    neighbors: [Option<Tile>; 4],
) {
    let water = matches!(tile, Tile::River | Tile::Ford);
    if !water && tile != Tile::Road {
        return;
    }
    let p = r.w / 32.0;
    for (side, neighbor) in neighbors.into_iter().enumerate() {
        let Some(neighbor) = neighbor else { continue };
        let edge = if water {
            !matches!(neighbor, Tile::River | Tile::Ford)
        } else {
            matches!(
                neighbor,
                Tile::Grass | Tile::Forest | Tile::DeepForest | Tile::Ruins
            )
        };
        if !edge {
            continue;
        }
        for segment in 0..4 {
            let offset = segment as f32 * 8.0;
            let depth = if segment % 2 == 0 { 2.0 } else { 3.0 };
            let (x, y, w, h) = match side {
                0 => (offset, 0.0, 8.0, depth),
                1 => (32.0 - depth, offset, depth, 8.0),
                2 => (offset, 32.0 - depth, 8.0, depth),
                _ => (0.0, offset, depth, 8.0),
            };
            let color = if water {
                Color::from_rgba(79, 108, 103, 255)
            } else {
                Color::from_rgba(91, 103, 55, 255)
            };
            canvas.rect(
                Rect::new(r.x + x * p, r.y + y * p, w * p, h * p),
                light(color, strength, remembered),
            );
        }
    }
}

/// Presentation history only: neither animation nor facing is serialized.
#[derive(Default)]
pub(crate) struct ActorAnimations {
    location: Option<(u64, usize)>,
    tick: u64,
    actors: HashMap<Option<usize>, Motion>,
}

struct Motion {
    pos: Pos,
    hp: i32,
    left: bool,
    moved_at: Option<u64>,
    hurt_at: Option<u64>,
}

#[derive(Clone, Copy, Default)]
pub(crate) struct Pose {
    left: bool,
    stride: f32,
    flash: bool,
}

impl ActorAnimations {
    fn sync(&mut self, game: &Game) {
        let location = (game.seed, game.player.map);
        if self.location != Some(location) || game.tick < self.tick {
            self.actors.clear();
            self.location = Some(location);
        }
        self.tick = game.tick;
    }

    fn pose(&mut self, id: Option<usize>, pos: Pos, hp: i32, now: u64) -> Pose {
        let motion = self.actors.entry(id).or_insert(Motion {
            pos,
            hp,
            left: false,
            moved_at: None,
            hurt_at: None,
        });
        if pos != motion.pos {
            if pos.x != motion.pos.x {
                motion.left = pos.x < motion.pos.x;
            }
            // Teleports are not walking; never animate across a room or a load.
            motion.moved_at = (pos.distance(motion.pos) == 1).then_some(now);
            motion.pos = pos;
        }
        if hp < motion.hp {
            motion.hurt_at = Some(now);
        }
        motion.hp = hp;
        Pose {
            left: motion.left,
            stride: match motion.moved_at.and_then(|at| now.checked_sub(at)) {
                Some(0..80) => 1.0,
                Some(80..160) => -1.0,
                _ => 0.0,
            },
            flash: motion
                .hurt_at
                .and_then(|at| now.checked_sub(at))
                .is_some_and(|age| age < 120),
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) enum Figure {
    Npc(Archetype, u8),
    Player {
        weapon: u8,
        armour: u8,
        relic: Option<BossRelic>,
    },
}

fn gear_color(tier: u8) -> Color {
    match tier {
        0 => Color::from_rgba(145, 102, 67, 255),
        1 => Color::from_rgba(158, 175, 185, 255),
        2 => Color::from_rgba(159, 216, 228, 255),
        _ => GOLD,
    }
}

// Thirty-two authored units leave room for joints, faces and equipment
// construction; broad color masses remain readable at half-size.
pub(crate) fn actor(canvas: &Canvas, r: Rect, accent: Color, figure: Figure, pose: Pose) {
    let p = r.w / 32.0;
    let paint = |x: f32, y: f32, w: f32, h: f32, mut color: Color| {
        let x = if pose.left { 32.0 - x - w } else { x };
        // Apply damage last, including highlights and phase details.
        if pose.flash {
            color.r += (1.0 - color.r) * 0.7;
            color.g += (1.0 - color.g) * 0.7;
            color.b += (1.0 - color.b) * 0.7;
        }
        canvas.rect(Rect::new(r.x + x * p, r.y + y * p, w * p, h * p), color);
    };
    let skin = Color::from_rgba(225, 179, 132, 255);
    let skin_shadow = Color::from_rgba(166, 115, 86, 255);
    let bone = Color::from_rgba(224, 218, 187, 255);
    let steel = gear_color(1);
    let leather = gear_color(0);
    let dark = Color::from_rgba(31, 35, 43, 255);
    let cloth = light(accent, 0.48, false);
    let fold = light(accent, 0.32, false);
    let stride = pose.stride * 1.5;
    canvas.rect(
        Rect::new(r.x + 4.0 * p, r.y + 27.0 * p, 24.0 * p, 3.0 * p),
        Color::new(0.0, 0.0, 0.0, 0.30),
    );
    // This mark conveys allegiance/intent, not an equipment or buff badge.
    paint(13.0, 30.0, 6.0, 2.0, accent);
    let (kind, phase) = match figure {
        Figure::Npc(kind, phase) => (kind, phase),
        Figure::Player { .. } => (Archetype::Companion, 0),
    };
    match kind {
        Archetype::Rat | Archetype::Matriarch | Archetype::Wolf | Archetype::Bear => {
            let (body, back, belly, head) = match kind {
                Archetype::Rat => (Color::from_rgba(148, 128, 112, 255), 19.0, 26.0, 18.0),
                Archetype::Matriarch => (Color::from_rgba(137, 83, 108, 255), 11.0, 25.0, 13.0),
                Archetype::Wolf => (Color::from_rgba(142, 156, 165, 255), 16.0, 24.0, 11.0),
                _ => (Color::from_rgba(119, 78, 49, 255), 9.0, 25.0, 10.0),
            };
            let fur = light(body, 0.66, false);
            let ridge = light(body, 1.18, false);
            let rat = matches!(kind, Archetype::Rat | Archetype::Matriarch);
            // Far legs, shoulder and bent near hock give the animals four limbs.
            paint(10.0, belly - 2.0 - stride, 3.0, 5.0, fur);
            paint(21.0, belly - 2.0 + stride, 3.0, 5.0, fur);
            paint(6.0, back + 2.0, 19.0, belly - back - 1.0, dark);
            paint(7.0, back, 15.0, belly - back - 1.0, body);
            paint(9.0, back - 1.0, 9.0, 3.0, body);
            paint(8.0, back + 1.0, 9.0, 2.0, ridge);
            paint(7.0, belly - 4.0, 13.0, 3.0, fur);
            paint(7.0, belly - 4.0 + stride, 5.0, 4.0, body);
            paint(8.0, belly + stride, 3.0, 3.0, fur);
            paint(
                8.0,
                belly + 2.0 + stride,
                5.0,
                1.0,
                if rat { skin } else { fur },
            );
            paint(20.0, belly - 5.0 - stride, 4.0, 6.0, fur);
            paint(
                21.0,
                belly + 1.0 - stride,
                5.0,
                2.0,
                if rat { skin } else { body },
            );
            // Neck, cheek plane, brow, projecting muzzle and dark nose.
            paint(19.0, head + 3.0, 7.0, belly - head - 4.0, fur);
            paint(21.0, head, 7.0, 8.0, body);
            paint(22.0, head, 5.0, 2.0, ridge);
            paint(
                24.0,
                head + 5.0,
                6.0,
                3.0,
                if rat { skin_shadow } else { ridge },
            );
            paint(29.0, head + 5.0, 2.0, 2.0, INK);
            paint(
                25.0,
                head + 3.0,
                2.0,
                2.0,
                if phase >= 2 { RED } else { INK },
            );
            if rat {
                paint(2.0, 25.0, 5.0, 2.0, skin_shadow);
                paint(1.0, 22.0, 2.0, 4.0, skin_shadow);
                paint(2.0, 21.0, 3.0, 2.0, skin);
                paint(20.0, head - 3.0, 5.0, 5.0, fur);
                paint(21.0, head - 2.0, 3.0, 3.0, skin_shadow);
                paint(22.0, head - 2.0, 2.0, 2.0, skin);
                paint(27.0, head + 8.0, 2.0, 2.0, bone);
                paint(25.0, head + 9.0, 5.0, 1.0, fur);
                paint(11.0, back + 4.0, 4.0, 2.0, fur);
                if kind == Archetype::Matriarch {
                    // Broken bone spines and a ragged mane identify the brood boss.
                    paint(8.0, 8.0, 3.0, 5.0, bone);
                    paint(13.0, 5.0, 3.0, 7.0, bone);
                    paint(18.0, 8.0, 2.0, 5.0, bone);
                    paint(8.0, 12.0, 13.0, 3.0, fur);
                    paint(10.0, 15.0, 3.0, 4.0, ridge);
                    paint(15.0, 14.0, 3.0, 3.0, ridge);
                    paint(28.0, 21.0, 2.0, 4.0, bone);
                }
            } else if kind == Archetype::Wolf {
                paint(2.0, 14.0, 6.0, 4.0, fur);
                paint(1.0, 12.0, 3.0, 3.0, body);
                paint(20.0, 6.0, 3.0, 7.0, fur);
                paint(21.0, 8.0, 2.0, 3.0, skin_shadow);
                paint(25.0, 8.0, 2.0, 4.0, fur);
                paint(19.0, 18.0, 5.0, 3.0, ridge);
                paint(20.0, 21.0, 3.0, 3.0, ridge);
                paint(27.0, 18.0, 2.0, 2.0, bone);
                paint(11.0, 18.0, 8.0, 2.0, fur);
            } else {
                paint(8.0, 8.0, 10.0, 5.0, body);
                paint(9.0, 9.0, 6.0, 2.0, ridge);
                paint(20.0, 7.0, 4.0, 4.0, fur);
                paint(25.0, 7.0, 4.0, 4.0, fur);
                paint(21.0, 8.0, 2.0, 2.0, leather);
                paint(26.0, 8.0, 2.0, 2.0, leather);
                paint(24.0, 15.0, 5.0, 4.0, leather);
                paint(29.0, 15.0, 2.0, 2.0, INK);
                paint(17.0, 15.0, 3.0, 7.0, fur);
                paint(10.0, 20.0, 4.0, 2.0, ridge);
                paint(9.0, 28.0 + stride, 4.0, 1.0, bone);
                paint(22.0, 27.0 - stride, 4.0, 1.0, bone);
            }
        }
        _ => {
            let undead = matches!(kind, Archetype::Skeleton | Archetype::Lich);
            let face = if undead { bone } else { skin };
            let face_shade = if undead {
                light(bone, 0.68, false)
            } else {
                skin_shadow
            };
            let robe = matches!(
                kind,
                Archetype::Oracle | Archetype::Lich | Archetype::Adjudicator
            );
            let broad = matches!(kind, Archetype::Chief | Archetype::Adjudicator);
            // Cape is behind the body and occupies its own left-hand silhouette.
            if kind == Archetype::Companion {
                let cape = if matches!(figure, Figure::Player { .. }) {
                    SEA
                } else {
                    accent
                };
                paint(5.0, 10.0, 7.0, 15.0, light(cape, 0.60, false));
                paint(4.0, 21.0, 7.0, 6.0, light(cape, 0.60, false));
                paint(6.0, 12.0, 3.0, 13.0, cape);
                paint(4.0, 26.0, 4.0, 2.0, cape);
            }
            // Separate trouser, knee and boot planes keep the stride readable.
            for (x, step) in [(10.0, stride), (18.0, -stride)] {
                paint(x, 22.0 + step, 4.0, 5.0, fold);
                paint(x, 25.0 + step, 4.0, 3.0, leather);
                paint(x, 28.0 + step, 6.0, 2.0, dark);
                paint(x, 25.0 + step, 4.0, 1.0, light(leather, 1.2, false));
            }
            paint(8.0, 12.0, 16.0, 12.0, dark);
            paint(10.0, 12.0, 12.0, 11.0, cloth);
            paint(10.0, 14.0, 3.0, 8.0, light(accent, 0.75, false));
            paint(18.0, 15.0, 3.0, 7.0, fold);
            paint(7.0, 14.0, 3.0, 5.0, cloth);
            paint(7.0, 19.0, 3.0, 3.0, fold);
            paint(8.0, 21.0, 3.0, 3.0, face_shade);
            paint(22.0, 14.0, 3.0, 5.0, cloth);
            paint(23.0, 19.0, 3.0, 4.0, face_shade);
            paint(24.0, 20.0, 2.0, 2.0, face);
            paint(10.0, 22.0, 12.0, 2.0, leather);
            paint(16.0, 22.0, 3.0, 2.0, GOLD);
            // Hair/hood outline, shaded cheek, ear, nose and brow.
            paint(11.0, 3.0, 12.0, 10.0, dark);
            paint(13.0, 4.0, 8.0, 8.0, face_shade);
            paint(15.0, 4.0, 7.0, 6.0, face);
            paint(12.0, 7.0, 2.0, 3.0, face);
            paint(22.0, 8.0, 2.0, 2.0, face);
            paint(19.0, 6.0, 3.0, 1.0, face_shade);
            paint(20.0, 7.0, 2.0, 2.0, INK);
            paint(18.0, 11.0, 4.0, 1.0, face_shade);
            if robe {
                paint(9.0, 20.0, 15.0, 7.0, cloth);
                paint(7.0, 27.0, 19.0, 2.0, dark);
                paint(10.0, 20.0, 2.0, 8.0, light(accent, 0.75, false));
                paint(15.0, 22.0, 2.0, 6.0, fold);
                paint(21.0, 20.0, 2.0, 8.0, fold);
                paint(9.0, 27.0, 15.0, 1.0, leather);
            }
            if broad {
                paint(4.0, 12.0, 23.0, 5.0, light(steel, 0.7, false));
                paint(5.0, 12.0, 7.0, 2.0, steel);
                paint(20.0, 12.0, 6.0, 2.0, steel);
                paint(10.0, 15.0, 13.0, 8.0, light(steel, 0.65, false));
                paint(12.0, 16.0, 4.0, 6.0, steel);
                paint(17.0, 17.0, 4.0, 2.0, steel);
            }
            match kind {
                Archetype::Commoner => {
                    paint(9.0, 4.0, 15.0, 3.0, leather);
                    paint(13.0, 1.0, 8.0, 4.0, leather);
                    paint(13.0, 3.0, 8.0, 2.0, light(leather, 0.65, false));
                    paint(13.0, 14.0, 8.0, 9.0, Color::from_rgba(168, 152, 119, 255));
                    paint(14.0, 14.0, 2.0, 8.0, bone);
                    paint(16.0, 19.0, 4.0, 2.0, light(leather, 0.8, false));
                    paint(25.0, 17.0, 2.0, 12.0, leather);
                    paint(24.0, 15.0, 4.0, 3.0, steel);
                }
                Archetype::Vendor => {
                    paint(2.0, 13.0, 7.0, 13.0, light(leather, 0.65, false));
                    paint(2.0, 13.0, 6.0, 3.0, leather);
                    paint(4.0, 14.0, 2.0, 10.0, GOLD);
                    paint(9.0, 4.0, 15.0, 3.0, GOLD);
                    paint(13.0, 1.0, 8.0, 4.0, cloth);
                    paint(14.0, 1.0, 6.0, 1.0, accent);
                    paint(14.0, 14.0, 8.0, 10.0, bone);
                    paint(18.0, 14.0, 2.0, 10.0, light(bone, 0.75, false));
                    paint(15.0, 19.0, 6.0, 4.0, leather);
                    paint(17.0, 19.0, 2.0, 2.0, GOLD);
                }
                Archetype::Traveller => {
                    paint(2.0, 10.0, 7.0, 15.0, light(leather, 0.7, false));
                    paint(1.0, 9.0, 8.0, 4.0, bone);
                    paint(2.0, 10.0, 6.0, 1.0, light(bone, 0.7, false));
                    paint(4.0, 13.0, 2.0, 11.0, leather);
                    paint(9.0, 4.0, 15.0, 2.0, leather);
                    paint(13.0, 1.0, 8.0, 4.0, leather);
                    paint(14.0, 3.0, 7.0, 1.0, GOLD);
                    paint(12.0, 13.0, 2.0, 9.0, leather);
                    paint(14.0, 21.0, 7.0, 4.0, leather);
                    paint(27.0, 9.0, 2.0, 21.0, leather);
                    paint(27.0, 9.0, 3.0, 2.0, bone);
                }
                Archetype::Thief | Archetype::Bandit | Archetype::Smuggler => {
                    paint(10.0, 2.0, 13.0, 4.0, cloth);
                    paint(9.0, 5.0, 4.0, 9.0, cloth);
                    paint(11.0, 3.0, 8.0, 2.0, light(accent, 0.65, false));
                    paint(14.0, 10.0, 9.0, 3.0, dark);
                    paint(13.0, 14.0, 3.0, 4.0, leather);
                    paint(16.0, 17.0, 3.0, 4.0, leather);
                    if kind == Archetype::Bandit {
                        paint(9.0, 14.0, 13.0, 3.0, leather);
                        paint(10.0, 14.0, 3.0, 1.0, steel);
                        paint(19.0, 14.0, 2.0, 1.0, steel);
                        paint(27.0, 11.0, 3.0, 12.0, steel);
                        paint(28.0, 10.0, 2.0, 3.0, bone);
                        paint(26.0, 22.0, 6.0, 2.0, leather);
                        paint(28.0, 24.0, 2.0, 4.0, leather);
                    } else if kind == Archetype::Smuggler {
                        paint(1.0, 18.0, 8.0, 9.0, leather);
                        paint(2.0, 16.0, 6.0, 3.0, light(leather, 1.2, false));
                        paint(4.0, 17.0, 2.0, 10.0, bone);
                        paint(1.0, 23.0, 8.0, 2.0, light(leather, 0.65, false));
                        paint(19.0, 23.0, 4.0, 4.0, leather);
                    } else {
                        paint(7.0, 14.0, 3.0, 12.0, dark);
                        paint(25.0, 20.0, 5.0, 2.0, steel);
                        paint(29.0, 19.0, 2.0, 2.0, bone);
                        paint(24.0, 19.0, 2.0, 4.0, leather);
                        paint(13.0, 23.0, 3.0, 4.0, dark);
                    }
                }
                Archetype::Guard => {
                    paint(11.0, 1.0, 11.0, 5.0, steel);
                    paint(13.0, 1.0, 3.0, 4.0, bone);
                    paint(10.0, 5.0, 14.0, 2.0, light(steel, 0.7, false));
                    paint(11.0, 7.0, 3.0, 5.0, steel);
                    paint(12.0, 14.0, 10.0, 7.0, steel);
                    paint(13.0, 15.0, 3.0, 5.0, light(steel, 1.2, false));
                    paint(3.0, 16.0, 9.0, 10.0, steel);
                    paint(5.0, 17.0, 5.0, 7.0, cloth);
                    paint(7.0, 17.0, 2.0, 7.0, accent);
                    paint(5.0, 24.0, 5.0, 3.0, steel);
                    paint(28.0, 7.0, 2.0, 23.0, leather);
                    paint(27.0, 3.0, 4.0, 5.0, steel);
                    paint(28.0, 1.0, 2.0, 6.0, bone);
                }
                Archetype::Skeleton => {
                    paint(11.0, 3.0, 12.0, 8.0, bone);
                    paint(11.0, 4.0, 2.0, 6.0, light(bone, 0.65, false));
                    paint(13.0, 6.0, 3.0, 3.0, INK);
                    paint(19.0, 6.0, 3.0, 3.0, INK);
                    paint(17.0, 9.0, 2.0, 2.0, INK);
                    paint(14.0, 11.0, 7.0, 2.0, bone);
                    paint(16.0, 11.0, 1.0, 2.0, INK);
                    paint(19.0, 11.0, 1.0, 2.0, INK);
                    paint(10.0, 14.0, 12.0, 9.0, dark);
                    for y in [14.0, 17.0, 20.0] {
                        paint(12.0, y, 8.0, 2.0, bone);
                        paint(11.0, y + 1.0, 2.0, 1.0, light(bone, 0.65, false));
                    }
                    paint(16.0, 14.0, 2.0, 9.0, bone);
                    paint(12.0, 23.0, 9.0, 2.0, bone);
                    paint(7.0, 15.0, 2.0, 5.0, bone);
                    paint(8.0, 20.0, 2.0, 4.0, bone);
                    paint(23.0, 15.0, 2.0, 5.0, bone);
                    paint(11.0, 25.0 + stride, 2.0, 4.0, bone);
                    paint(19.0, 25.0 - stride, 2.0, 4.0, bone);
                }
                Archetype::Chief => {
                    paint(7.0, 1.0, 3.0, 7.0, bone);
                    paint(5.0, 0.0, 3.0, 3.0, bone);
                    paint(24.0, 1.0, 3.0, 7.0, bone);
                    paint(26.0, 0.0, 3.0, 3.0, bone);
                    paint(10.0, 3.0, 14.0, 3.0, steel);
                    paint(14.0, 3.0, 5.0, 2.0, GOLD);
                    paint(13.0, 10.0, 10.0, 5.0, leather);
                    paint(15.0, 12.0, 6.0, 4.0, light(leather, 0.7, false));
                    paint(5.0, 14.0, 5.0, 3.0, bone);
                    paint(7.0, 16.0, 3.0, 3.0, bone);
                    paint(11.0, 22.0, 12.0, 3.0, leather);
                    paint(16.0, 22.0, 4.0, 3.0, GOLD);
                    paint(27.0, 10.0, 2.0, 20.0, leather);
                    paint(25.0, 8.0, 7.0, 7.0, steel);
                    paint(30.0, 7.0, 2.0, 9.0, bone);
                    paint(25.0, 9.0, 2.0, 4.0, light(steel, 0.65, false));
                }
                Archetype::Oracle | Archetype::Lich => {
                    paint(10.0, 2.0, 13.0, 3.0, cloth);
                    paint(9.0, 4.0, 4.0, 10.0, cloth);
                    paint(10.0, 5.0, 2.0, 7.0, light(accent, 0.7, false));
                    paint(27.0, 9.0, 2.0, 21.0, leather);
                    paint(25.0, 5.0, 6.0, 6.0, GOLD);
                    paint(
                        27.0,
                        6.0,
                        2.0,
                        4.0,
                        if kind == Archetype::Lich { SEA } else { bone },
                    );
                    paint(12.0, 15.0, 2.0, 11.0, light(accent, 0.8, false));
                    paint(18.0, 16.0, 2.0, 10.0, light(accent, 0.7, false));
                    if kind == Archetype::Lich {
                        for x in [11.0, 16.0, 21.0] {
                            paint(x, 0.0, 2.0, 5.0, GOLD);
                        }
                        paint(11.0, 4.0, 12.0, 2.0, GOLD);
                        paint(14.0, 7.0, 3.0, 2.0, INK);
                        paint(19.0, 7.0, 3.0, 2.0, INK);
                        paint(15.0, 7.0, 2.0, 2.0, SEA);
                        paint(20.0, 7.0, 2.0, 2.0, SEA);
                        paint(15.0, 11.0, 6.0, 2.0, dark);
                        paint(7.0, 13.0, 5.0, 3.0, bone);
                        paint(21.0, 13.0, 4.0, 3.0, bone);
                        paint(15.0, 18.0, 3.0, 4.0, GOLD);
                        paint(8.0, 28.0, 3.0, 2.0, cloth);
                        paint(20.0, 28.0, 4.0, 2.0, cloth);
                    } else {
                        paint(13.0, 7.0, 10.0, 2.0, GOLD);
                        paint(14.0, 8.0, 7.0, 1.0, light(GOLD, 0.7, false));
                        paint(14.0, 14.0, 6.0, 2.0, bone);
                        paint(3.0, 18.0, 7.0, 6.0, leather);
                        paint(4.0, 18.0, 5.0, 4.0, bone);
                        paint(6.0, 18.0, 1.0, 4.0, light(leather, 0.6, false));
                    }
                }
                Archetype::Adjudicator => {
                    paint(10.0, 1.0, 14.0, 11.0, light(GOLD, 0.65, false));
                    paint(11.0, 2.0, 11.0, 8.0, GOLD);
                    paint(13.0, 2.0, 3.0, 4.0, bone);
                    paint(13.0, 7.0, 9.0, 2.0, INK);
                    paint(17.0, 9.0, 3.0, 4.0, GOLD);
                    paint(18.0, 9.0, 1.0, 3.0, bone);
                    paint(5.0, 12.0, 7.0, 2.0, GOLD);
                    paint(21.0, 12.0, 6.0, 2.0, GOLD);
                    paint(13.0, 16.0, 8.0, 2.0, GOLD);
                    paint(16.0, 15.0, 2.0, 8.0, GOLD);
                    paint(10.0, 25.0, 13.0, 2.0, GOLD);
                    paint(27.0, 13.0, 2.0, 17.0, leather);
                    paint(23.0, 8.0, 9.0, 6.0, light(GOLD, 0.65, false));
                    paint(23.0, 8.0, 9.0, 2.0, GOLD);
                    paint(24.0, 10.0, 2.0, 3.0, bone);
                    paint(30.0, 10.0, 2.0, 3.0, GOLD);
                }
                Archetype::Companion => {
                    paint(11.0, 3.0, 11.0, 3.0, leather);
                    paint(12.0, 3.0, 5.0, 1.0, light(leather, 1.25, false));
                    if matches!(figure, Figure::Npc(..)) {
                        paint(13.0, 14.0, 8.0, 7.0, steel);
                        paint(14.0, 15.0, 2.0, 5.0, bone);
                        paint(18.0, 19.0, 3.0, 2.0, light(steel, 0.65, false));
                        paint(27.0, 15.0, 2.0, 10.0, steel);
                        paint(26.0, 24.0, 5.0, 2.0, leather);
                    }
                }
                Archetype::Rat | Archetype::Matriarch | Archetype::Wolf | Archetype::Bear | Archetype::Cragmother => {}
                // E5 proxies on the humanoid base until the bake lane ships actors
                // (ART-TODO: Actor.GnawThane/Tollmaster/Mirelight/PaleStag plates).
                Archetype::GnawThane => {
                    // Gnawed bone crown, whisker tufts, hunched warren-king mantle.
                    let fen_fur = Color::from_rgba(148, 128, 112, 255);
                    paint(12.0, 1.0, 2.0, 3.0, bone);
                    paint(15.0, 0.0, 2.0, 4.0, bone);
                    paint(18.0, 1.0, 2.0, 3.0, bone);
                    paint(11.0, 4.0, 1.0, 3.0, bone);
                    paint(20.0, 4.0, 1.0, 3.0, bone);
                    paint(9.0, 9.0, 3.0, 1.0, fen_fur);
                    paint(20.0, 9.0, 3.0, 1.0, fen_fur);
                    paint(8.0, 16.0, 16.0, 4.0, light(fen_fur, 0.55, false));
                    paint(27.0, 14.0, 3.0, 8.0, fen_fur);
                }
                Archetype::Tollmaster => {
                    // Coin sash, toll-hook staff, warlord's flat cap.
                    paint(10.0, 13.0, 12.0, 2.0, GOLD);
                    paint(14.0, 13.0, 1.0, 1.0, WHITE);
                    paint(17.0, 13.0, 1.0, 1.0, WHITE);
                    paint(11.0, 2.0, 12.0, 4.0, leather);
                    paint(9.0, 4.0, 16.0, 2.0, light(leather, 0.7, false));
                    paint(27.0, 8.0, 2.0, 17.0, steel);
                    paint(27.0, 8.0, 5.0, 2.0, steel);
                    paint(30.0, 8.0, 2.0, 4.0, INK);
                }
                Archetype::Mirelight => {
                    // A hollow face under a veil of drowned light; motes drift off.
                    let glow = Color::from_rgba(150, 226, 214, 255);
                    paint(13.0, 5.0, 6.0, 5.0, light(glow, 0.5, false));
                    paint(15.0, 6.0, 2.0, 2.0, INK);
                    paint(19.0, 6.0, 2.0, 2.0, INK);
                    paint(3.0, 12.0, 2.0, 2.0, glow);
                    paint(28.0, 9.0, 2.0, 2.0, glow);
                    paint(24.0, 24.0, 2.0, 2.0, glow);
                    paint(6.0, 24.0, 2.0, 2.0, glow);
                    paint(9.0, 19.0, 14.0, 3.0, light(glow, 0.35, false));
                }
                Archetype::PaleStag => {
                    // Pale antlers and a ghost-bright mantle (quadruped plate pending).
                    paint(11.0, 0.0, 2.0, 5.0, bone);
                    paint(19.0, 0.0, 2.0, 5.0, bone);
                    paint(9.0, 1.0, 2.0, 2.0, bone);
                    paint(21.0, 1.0, 2.0, 2.0, bone);
                    paint(10.0, 3.0, 2.0, 1.0, WHITE);
                    paint(20.0, 3.0, 2.0, 1.0, WHITE);
                    paint(9.0, 15.0, 14.0, 3.0, WHITE);
                }
                Archetype::Alchemist => {
                    // Stained apron and a belt of vials.
                    paint(11.0, 14.0, 10.0, 12.0, gear_color(0));
                    paint(12.0, 14.0, 2.0, 3.0, Color::from_rgba(120, 170, 110, 255));
                    paint(15.0, 14.0, 2.0, 3.0, Color::from_rgba(116, 143, 166, 255));
                    paint(18.0, 14.0, 2.0, 3.0, Color::from_rgba(180, 140, 90, 255));
                    paint(11.0, 14.0, 10.0, 1.0, leather);
                }
                Archetype::BroodHole => {
                    // A gnawed pit mouth eating the silhouette down to the floor.
                    paint(6.0, 8.0, 20.0, 22.0, INK);
                    paint(4.0, 22.0, 24.0, 8.0, dark);
                    paint(6.0, 23.0, 20.0, 6.0, Color::from_rgba(46, 36, 28, 255));
                    paint(8.0, 24.0, 16.0, 4.0, INK);
                    paint(5.0, 21.0, 3.0, 2.0, bone);
                    paint(24.0, 21.0, 3.0, 2.0, bone);
                }
                Archetype::FalseGlow => {
                    // A light pretending to be her: dimmer, thinner, same veil shape.
                    let glow = Color::from_rgba(116, 168, 160, 255);
                    paint(13.0, 6.0, 6.0, 5.0, light(glow, 0.45, false));
                    paint(4.0, 13.0, 1.0, 1.0, glow);
                    paint(27.0, 10.0, 1.0, 1.0, glow);
                    paint(10.0, 20.0, 12.0, 3.0, light(glow, 0.3, false));
                }
                // E7 proxy (ART-TODO: Actor.OathlessCurate plate): the curator's
                // torn violet mitre and the page he never stops folding.
                Archetype::OathlessCurate => {
                    let curate_violet = Color::from_rgba(120, 90, 150, 255);
                    paint(11.0, 1.0, 12.0, 4.0, curate_violet);
                    paint(13.0, 0.0, 8.0, 3.0, light(curate_violet, 0.6, false));
                    paint(14.0, 14.0, 6.0, 10.0, bone);
                    paint(15.0, 15.0, 4.0, 1.0, INK);
                    paint(15.0, 18.0, 4.0, 1.0, INK);
                    paint(15.0, 21.0, 4.0, 1.0, INK);
                    paint(9.0, 12.0, 4.0, 8.0, curate_violet);
                    paint(21.0, 12.0, 4.0, 8.0, curate_violet);
                }
                // The drowned matriarch: kelp shawl, barnacle coronet, trawl-net shoulder.
                Archetype::Tidemother => {
                    paint(11.0, 2.0, 11.0, 3.0, Color::from_rgba(93, 122, 89, 255));
                    paint(13.0, 1.0, 2.0, 2.0, WHITE);
                    paint(18.0, 1.0, 2.0, 2.0, WHITE);
                    paint(12.0, 6.0, 2.0, 2.0, INK);
                    paint(19.0, 6.0, 2.0, 2.0, INK);
                    paint(15.0, 6.0, 2.0, 2.0, Color::from_rgba(116, 143, 166, 255));
                    paint(8.0, 12.0, 4.0, 8.0, Color::from_rgba(93, 122, 89, 255));
                    paint(20.0, 12.0, 4.0, 8.0, Color::from_rgba(93, 122, 89, 255));
                    paint(9.0, 19.0, 14.0, 3.0, Color::from_rgba(70, 96, 84, 255));
                }
            }
            if let Figure::Player {
                weapon,
                armour,
                relic,
            } = figure
            {
                let plate = gear_color(armour);
                let plate_shadow = light(plate, 0.65, false);
                paint(11.0, 13.0, 12.0, 10.0, plate_shadow);
                paint(12.0, 14.0, 5.0, 7.0, plate);
                paint(18.0, 14.0, 4.0, 6.0, plate);
                paint(12.0, 14.0, 2.0, 5.0, light(plate, 1.2, false));
                paint(12.0, 21.0, 10.0, 2.0, leather);
                paint(16.0, 21.0, 3.0, 2.0, GOLD);
                if armour == 0 {
                    // Stitched leather vest with crossed shoulder straps.
                    paint(13.0, 13.0, 2.0, 4.0, light(leather, 0.6, false));
                    paint(19.0, 13.0, 2.0, 4.0, light(leather, 0.6, false));
                    paint(15.0, 17.0, 5.0, 2.0, light(leather, 0.6, false));
                } else {
                    paint(8.0, 12.0, 6.0, 4.0, plate_shadow);
                    paint(20.0, 12.0, 6.0, 4.0, plate_shadow);
                    paint(9.0, 12.0, 5.0, 2.0, plate);
                    paint(20.0, 12.0, 5.0, 2.0, plate);
                    paint(12.0, 23.0, 4.0, 2.0, plate);
                    paint(19.0, 23.0, 4.0, 2.0, plate);
                    paint(23.0, 18.0, 3.0, 2.0, plate);
                }
                if armour >= 2 {
                    paint(11.0, 1.0, 12.0, 5.0, plate_shadow);
                    paint(12.0, 1.0, 9.0, 3.0, plate);
                    paint(14.0, 1.0, 2.0, 3.0, bone);
                    paint(11.0, 6.0, 3.0, 5.0, plate);
                    paint(21.0, 5.0, 3.0, 2.0, plate);
                }
                if armour >= 3 {
                    paint(14.0, 0.0, 5.0, 2.0, SEA);
                    paint(16.0, 15.0, 3.0, 4.0, SEA);
                    paint(9.0, 14.0, 4.0, 1.0, bone);
                    paint(21.0, 14.0, 4.0, 1.0, bone);
                }
                // Weapon and armour grades are independent, including geometry.
                let metal = gear_color(weapon);
                let tip = 14.0 - f32::from(weapon.min(3)) * 2.0;
                paint(27.0, tip + 2.0, 3.0, 22.0 - tip, metal);
                paint(28.0, tip, 2.0, 4.0, metal);
                paint(
                    27.0,
                    tip + 3.0,
                    1.0,
                    19.0 - tip,
                    if weapon == 0 { steel } else { bone },
                );
                paint(26.0, 23.0, 6.0, 2.0, metal);
                paint(28.0, 25.0, 2.0, 4.0, leather);
                paint(28.0, 29.0, 2.0, 1.0, metal);
                if weapon >= 2 {
                    paint(28.0, 16.0, 2.0, 3.0, SEA);
                    paint(26.0, 22.0, 2.0, 2.0, metal);
                    paint(30.0, 22.0, 2.0, 2.0, metal);
                }
                if weapon >= 3 {
                    paint(28.0, 10.0, 2.0, 2.0, bone);
                    paint(29.0, 25.0, 1.0, 2.0, GOLD);
                }
                match relic {
                    Some(BossRelic::Rallybreaker) => {
                        paint(25.0, 21.0, 3.0, 3.0, RED);
                        paint(26.0, 22.0, 1.0, 1.0, GOLD);
                    }
                    Some(BossRelic::Fangmantle) => {
                        paint(7.0, 10.0, 3.0, 5.0, bone);
                        paint(23.0, 10.0, 3.0, 5.0, bone);
                        paint(8.0, 13.0, 2.0, 3.0, Color::from_rgba(137, 83, 108, 255));
                    }
                    Some(BossRelic::Graveglass) => {
                        paint(15.0, 0.0, 4.0, 2.0, SEA);
                        paint(16.0, 1.0, 2.0, 2.0, WHITE);
                    }
                    // A salt-crust diadem of barnacles at the brow.
                    Some(BossRelic::Saltcrown) => {
                        paint(14.0, 2.0, 5.0, 1.0, Color::from_rgba(116, 143, 166, 255));
                        paint(13.0, 1.0, 1.0, 1.0, WHITE);
                        paint(19.0, 1.0, 1.0, 1.0, WHITE);
                    }
                    // A grey ridge-plate pauldron on the left shoulder.
                    Some(BossRelic::Stoneheart) => {
                        paint(6.0, 11.0, 3.0, 3.0, Color::from_rgba(148, 148, 150, 255));
                        paint(7.0, 12.0, 2.0, 1.0, WHITE);
                    }
                    // E5 relic marks (§6).
                    Some(BossRelic::GnawboneCrown) => {
                        paint(12.0, 1.0, 2.0, 2.0, bone);
                        paint(15.0, 0.0, 2.0, 3.0, bone);
                        paint(18.0, 1.0, 2.0, 2.0, bone);
                    }
                    Some(BossRelic::TollcoinCharm) => {
                        paint(15.0, 22.0, 3.0, 3.0, GOLD);
                        paint(16.0, 23.0, 1.0, 1.0, WHITE);
                    }
                    Some(BossRelic::WisplightLantern) => {
                        paint(23.0, 19.0, 3.0, 4.0, Color::from_rgba(150, 226, 214, 255));
                        paint(24.0, 18.0, 1.0, 1.0, GOLD);
                    }
                    Some(BossRelic::Hartshorn) => {
                        paint(8.0, 4.0, 2.0, 5.0, bone);
                        paint(7.0, 4.0, 1.0, 2.0, WHITE);
                    }
                    None => {}
                }
                // Wide gold feet, not a selection box resembling a window.
                paint(9.0, 30.0, 15.0, 2.0, GOLD);
            }
        }
    }
    if kind.boss() && phase >= 2 {
        // Phase wounds and awakened eyes are placed after every material detail.
        paint(14.0, 16.0, 2.0, 4.0, RED);
        paint(16.0, 20.0, 2.0, 5.0, RED);
        if kind != Archetype::Matriarch {
            paint(20.0, 7.0, 2.0, 2.0, RED);
        }
        if phase >= 3 {
            paint(8.0, 13.0, 4.0, 2.0, GOLD);
            paint(21.0, 15.0, 3.0, 2.0, GOLD);
            paint(12.0, 23.0, 2.0, 4.0, GOLD);
            if kind == Archetype::Matriarch {
                paint(13.0, 5.0, 3.0, 3.0, GOLD);
            }
        }
    }
}

/// Public snapshot of the viewport mapping so hit detection stays aligned with drawing.
#[derive(Clone, Copy, Debug)]
pub struct MapView {
    /// Pixel size of one drawn tile after clamping to the viewport.
    pub size: f32,
    /// Number of drawn tile columns.
    pub width: i32,
    /// Number of drawn tile rows.
    pub height: i32,
    /// Leftmost world tile included in the viewport.
    pub left: i32,
    /// Topmost world tile included in the viewport.
    pub top: i32,
    /// Pixel origin of the top-left drawn tile.
    pub origin: Vec2,
}

impl MapView {
    fn new(game: &Game, area: Rect, tile_size: f32) -> Option<Self> {
        let map = game.map();
        if !area.x.is_finite()
            || !area.y.is_finite()
            || !area.w.is_finite()
            || !area.h.is_finite()
            || !tile_size.is_finite()
            || tile_size < 1.0
            || area.w < 1.0
            || area.h < 1.0
            || map.width <= 0
            || map.height <= 0
        {
            return None;
        }
        let size = tile_size.min(area.w).min(area.h);
        let width = ((area.w / size).floor() as i32).min(map.width).max(1);
        let height = ((area.h / size).floor() as i32).min(map.height).max(1);
        let left = (game.player.pos.x - width / 2).clamp(0, map.width - width);
        let top = (game.player.pos.y - height / 2).clamp(0, map.height - height);
        let origin = vec2(
            area.x + (area.w - width as f32 * size) / 2.0,
            area.y + (area.h - height as f32 * size) / 2.0,
        );
        Some(Self {
            size,
            width,
            height,
            left,
            top,
            origin,
        })
    }

    fn screen(&self, pos: Pos) -> Option<Rect> {
        (pos.x >= self.left
            && pos.y >= self.top
            && pos.x < self.left + self.width
            && pos.y < self.top + self.height)
            .then_some(Rect::new(
                self.origin.x + (pos.x - self.left) as f32 * self.size,
                self.origin.y + (pos.y - self.top) as f32 * self.size,
                self.size,
                self.size,
            ))
    }
}
/// Exposes the viewport mapping used by the GUI map so hit detection follows the drawing.
pub fn map_view(game: &Game, area: Rect, tile_size: f32) -> Option<(MapView, Rect)> {
    let view = MapView::new(game, area, tile_size)?;
    Some((view, area))
}

pub(crate) fn draw_map(game: &Game, area: Rect, animations: &mut ActorAnimations, tile_size: f32) {
    let Some(view) = MapView::new(game, area, tile_size) else {
        return;
    };
    let canvas = Canvas(area);
    animations.sync(game);
    canvas.rect(area, INK);
    let size = view.size;
    let screen = |pos| view.screen(pos);
    let map = game.map();
    let radius = crate::ui::vision(game);
    for y in 0..view.height {
        for x in 0..view.width {
            let pos = Pos::new(view.left + x, view.top + y);
            let Some(index) = map.index(pos) else {
                continue;
            };
            if !map.explored.get(index).copied().unwrap_or(false) {
                continue;
            }
            let distance = pos.distance(game.player.pos);
            let tile = map.tiles[index];
            let strength = if distance * 4 <= radius * 3 {
                1.0
            } else {
                0.55
            } * if tile == Tile::DeepForest { 0.9 } else { 1.0 };
            let r = Rect::new(
                view.origin.x + x as f32 * size,
                view.origin.y + y as f32 * size,
                size,
                size,
            );
            tile_sprite(
                &canvas,
                tile,
                pos,
                r,
                strength,
                distance > radius,
                game.tick,
            );
            if matches!(tile, Tile::River | Tile::Ford | Tile::Road) {
                let neighbors = [(0, -1), (1, 0), (0, 1), (-1, 0)].map(|(dx, dy)| {
                    let index = map.index(pos.offset(dx, dy))?;
                    if map.explored.get(index).copied().unwrap_or(false) {
                        map.tiles.get(index).copied()
                    } else {
                        None
                    }
                });
                tile_edges(&canvas, tile, r, strength, distance > radius, neighbors);
            }
            if game
                .combat
                .as_ref()
                .is_some_and(|combat| distance <= radius && pos.distance(combat.center) == 5)
            {
                canvas.border(r, 1.0, RED);
            }
        }
    }
    for npc in game
        .npcs
        .iter()
        .filter(|npc| npc.alive() && npc.map == game.player.map)
    {
        if let Some((target, _)) = &npc.telegraph {
            let pulse = 0.22 + 0.20 * ((get_time() * 6.0).sin() as f32 * 0.5 + 0.5);
            for y in -1..=1 {
                for x in -1..=1 {
                    if let Some(r) = screen(target.offset(x, y)) {
                        canvas.rect(r, Color::new(1.0, 0.08, 0.06, pulse));
                        canvas.border(r, 1.0, RED);
                        canvas.text("!", r.x + 6.0, r.y + 3.0, 14, GOLD);
                    }
                }
            }
        }
    }
    for npc in game.npcs.iter().filter(|npc| {
        npc.alive() && npc.map == game.player.map && npc.pos.distance(game.player.pos) <= radius
    }) {
        let Some(r) = screen(npc.pos) else { continue };
        let color = if npc.intent == Intent::Flee {
            GOLD
        } else if npc.archetype.boss() {
            Color::from_rgba(215, 124, 233, 255)
        } else if npc.archetype.hostile() || npc.intent == Intent::Attack {
            RED
        } else {
            match npc.archetype {
                Archetype::Guard => Color::from_rgba(135, 161, 240, 255),
                _ => SEA,
            }
        };
        actor(
            &canvas,
            r,
            color,
            Figure::Npc(npc.archetype, npc.phase),
            animations.pose(Some(npc.id), npc.pos, npc.hp, game.elapsed_ms),
        );
        if npc.hp < npc.max_hp {
            canvas.rect(Rect::new(r.x + 2.0, r.y + size - 3.0, size - 4.0, 3.0), INK);
            canvas.rect(
                Rect::new(
                    r.x + 2.0,
                    r.y + size - 3.0,
                    (size - 4.0) * (npc.hp as f32 / npc.max_hp.max(1) as f32).clamp(0.0, 1.0),
                    2.0,
                ),
                color,
            );
        }
    }
    if let Some(r) = screen(game.player.pos) {
        actor(
            &canvas,
            r,
            WHITE,
            Figure::Player {
                weapon: game.player.weapon,
                armour: game.player.armour,
                relic: game.player.relic,
            },
            animations.pose(None, game.player.pos, game.player.hp, game.elapsed_ms),
        );
    }
    for portal in &map.portals {
        if portal.pos.distance(game.player.pos) > radius {
            continue;
        }
        if let Some(r) = screen(portal.pos) {
            canvas.border(r, 1.0, GOLD);
            if portal.pos.distance(game.player.pos) <= 3 {
                let label_width = measure_text(&portal.label, None, 13, 1.0).width;
                let x = (r.x + size + 3.0)
                    .min(area.x + area.w - label_width - 4.0)
                    .max(area.x + 2.0);
                canvas.text(&portal.label, x, r.y - 13.0, 13, GOLD);
            }
        }
    }
    for effect in game
        .effects
        .iter()
        .filter(|effect| effect.ttl > 0 && effect.pos.distance(game.player.pos) <= radius)
    {
        if let Some(r) = screen(effect.pos) {
            canvas.text(
                &effect.text,
                r.x,
                r.y - 10.0 - f32::from(6u8.saturating_sub(effect.ttl)) * 2.0,
                15,
                RED,
            );
        }
    }
}

pub(crate) fn draw_atlas(game: &Game, area: Rect) {
    let Some(world) = game.maps.first() else {
        return;
    };
    if area.w < 1.0 || area.h < 1.0 || world.width <= 0 || world.height <= 0 {
        return;
    }
    let canvas = Canvas(area);
    canvas.rect(area, INK);
    // Like the terminal atlas, the realm is a chart: terrain and landmarks are
    // public even before exploration. Local zoom is a crop of that same chart.
    let location = if game.player.map == 0 {
        Some(game.player.pos)
    } else {
        world
            .portals
            .iter()
            .find(|portal| portal.destination == game.player.map)
            .map(|portal| portal.pos)
    };
    let (width, height) = if game.atlas_zoom {
        (
            (area.w / 5.0).floor().max(1.0) as i32,
            (area.h / 5.0).floor().max(1.0) as i32,
        )
    } else {
        (world.width, world.height)
    };
    let width = width.min(world.width);
    let height = height.min(world.height);
    let center = location.unwrap_or(Pos::new(world.width / 2, world.height / 2));
    let left = if game.atlas_zoom {
        (center.x - width / 2).clamp(0, world.width - width)
    } else {
        0
    };
    let top = if game.atlas_zoom {
        (center.y - height / 2).clamp(0, world.height - height)
    } else {
        0
    };
    let size = (area.w / width as f32).min(area.h / height as f32);
    let origin = vec2(
        area.x + (area.w - width as f32 * size) / 2.0,
        area.y + (area.h - height as f32 * size) / 2.0,
    );
    for y in 0..height {
        for x in 0..width {
            let tile = world.tile(Pos::new(left + x, top + y));
            if tile != Tile::Rock {
                canvas.rect(
                    Rect::new(
                        origin.x + x as f32 * size,
                        origin.y + y as f32 * size,
                        size,
                        size,
                    ),
                    light(terrain_color(tile), 0.85, false),
                );
            }
        }
    }
    let point = |pos: Pos| {
        (pos.x >= left && pos.x < left + width && pos.y >= top && pos.y < top + height).then_some(
            vec2(
                origin.x + (pos.x - left) as f32 * size + size / 2.0,
                origin.y + (pos.y - top) as f32 * size + size / 2.0,
            ),
        )
    };
    for portal in &world.portals {
        if let Some(p) = point(portal.pos) {
            let marker = size.max(5.0);
            canvas.rect(
                Rect::new(p.x - marker / 2.0, p.y - marker / 2.0, marker, marker),
                GOLD,
            );
            let mut badge = [0u8; 4];
            let glyph = if (1..=3).contains(&portal.destination) {
                char::from(b'0' + portal.destination as u8)
            } else {
                '*'
            };
            canvas.text(
                glyph.encode_utf8(&mut badge),
                p.x + marker / 2.0 + 1.0,
                p.y - 5.0,
                13,
                GOLD,
            );
        }
    }
    if let Some(p) = location.and_then(point) {
        let marker = size.max(7.0);
        canvas.rect(
            Rect::new(
                p.x - marker / 2.0 - 1.0,
                p.y - marker / 2.0 - 1.0,
                marker + 2.0,
                marker + 2.0,
            ),
            INK,
        );
        canvas.rect(
            Rect::new(p.x - marker / 2.0, p.y - marker / 2.0, marker, marker),
            WHITE,
        );
        canvas.border(
            Rect::new(
                p.x - marker / 2.0 - 2.0,
                p.y - marker / 2.0 - 2.0,
                marker + 4.0,
                marker + 4.0,
            ),
            1.0,
            GOLD,
        );
    }
}
