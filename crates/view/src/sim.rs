//! The snapshot seam. `SimSlot` owns the live `Game` while the app runs; every
//! frame `sync` copies exactly what rendering needs into `WorldView` — the only
//! sim-derived resource the render systems may read. Mutations happen only in
//! the input/clock systems below, through `Action` verbs and sim tick.

use bevy::app::AppExit;
use bevy::prelude::*;
use bevy::render::view::window::screenshot::{save_to_disk, Screenshot};
use laya_realms::model::{Archetype, BossRelic, Build, Class, Game, Intent, Item, MapKind, Modal, Npc, Pos, Tile};
use std::path::PathBuf;

use crate::palette;

#[derive(Resource)]
pub struct SimSlot {
    pub game: Game,
    pub frame: u32,
    /// Wall-clock ms accumulated toward the sim's 4 Hz `tick_world` cadence.
    pub sim_ms: f64,
    pub shot: Option<ShotPlan>,
}

pub struct ShotPlan {
    pub path: PathBuf,
    pub at_frame: u32,
    /// Dev-runner hook: frames at which to press the torch quick-use, then exit.
    pub torch_at: Option<u32>,
    /// (start, end, dir): walk frames so the trail behind goes memory-gray
    /// before the screenshot — the only way a fresh spawn shows all fog tiers.
    pub walk: Option<(u32, u32, (i32, i32))>,
    /// (frame, path): extra capture earlier in the run (pre-staging prelude).
    pub prelude: Option<(u32, PathBuf)>,
    /// (frame, map index): teleport the player next to the boss on that map.
    pub stage_boss: Option<(u32, usize)>,
    /// (frame, class index): run creation flows with this order (D2 boon 0).
    pub create_at: Option<(u32, usize)>,
    /// (frame, tile offset from the player): scripted pointer click — sets
    /// the click-to-move target exactly as pointer::resolve_click would
    /// (headless smoke of the E6 autowalk path).
    pub click: Option<(u32, (i32, i32))>,
    /// (frame, absolute tile): dev staging teleport on the current map (same
    /// privilege as `stage_boss_at` — headless evidence only).
    pub stage: Option<(u32, Pos)>,
    /// (frame, map index, absolute tile): dev teleport to an explicit map.
    /// `stage_boss` only moves when that map has a boss, so terrain that lives
    /// on a bossless map needs its own hook.
    pub stage_map: Option<(u32, usize, Pos)>,
    /// (frame, map index, path): one capture per level. Thirty levels is thirty
    /// Bevy startups if done a run at a time; one run with a list is one startup
    /// and a deterministic set of frames.
    pub levels: Vec<(u32, usize, PathBuf)>,
    /// (frame, case, path): one capture per modal state, same reasoning.
    pub ui_cases: Vec<(u32, &'static str, PathBuf)>,
    pub exit_at: Option<u32>,
    /// Deterministic real-cellar transition assertions, not pre-painted states.
    pub doors_smoke: bool,
    fired: bool,
    prelude_fired: bool,
}

impl ShotPlan {
    /// E4 foundation: natural seed-42 start, torch + walk, one frame.
    pub fn foundation() -> Self {
        Self {
            path: PathBuf::from("docs/gfx/proto/e4-foundation.png"),
            at_frame: 120,
            torch_at: Some(40),
            walk: Some((20, 118, (1, 0))),
            prelude: None,
            stage_boss: None,
            create_at: None,
            exit_at: Some(220),
            click: None,
            stage_map: None,            levels: Vec::new(),
            ui_cases: Vec::new(),
            stage: None,
            fired: false,
            prelude_fired: false,
            doors_smoke: false,
        }
    }

    /// E4-s3: prelude frame in Millbrook (civvies + prop decals), then stage
    /// the Burrow — Floor 2 (map 6) adjacent to the Chief and step INTO him,
    /// which routes through the sim into boss combat (plate proof on stage).
    pub fn actors_s3() -> Self {
        Self {
            path: PathBuf::from("docs/gfx/proto/e4-actors.png"),
            at_frame: 140,
            torch_at: Some(40),
            // Exactly one step into the boss: one combat round (Chief rounds
            // cost ~8 HP — any more and the evidence frame has a dead hero).
            walk: Some((104, 105, (1, 1))),
            prelude: Some((14, PathBuf::from("docs/gfx/proto/e4-foundation.png"))),
            stage_boss: Some((100, 6)),
            create_at: None,
            exit_at: Some(230),
            click: None,
            stage_map: None,            levels: Vec::new(),
            ui_cases: Vec::new(),
            stage: None,
            fired: false,
            prelude_fired: false,
            doors_smoke: false,
        }
    }

    /// Iso lanes (same staging as the square runners, so panel comparisons
    /// line up one-to-one).
    pub fn iso_foundation() -> Self {
        Self {
            path: PathBuf::from("docs/gfx/proto/e4-iso.png"),
            at_frame: 130,
            torch_at: Some(40),
            walk: Some((50, 100, (1, 0))),
            prelude: None,
            stage_boss: None,
            create_at: None,
            exit_at: Some(220),
            click: None,
            stage_map: None,            levels: Vec::new(),
            ui_cases: Vec::new(),
            stage: None,
            fired: false,
            prelude_fired: false,
            doors_smoke: false,
        }
    }
    /// In-window modal proof: keep the boot Title, capture the pane pixels.
    /// (The invisible-modal regression passed headless tests that asserted
    /// spawn, never visibility — this plan is the honest anchor.)
    pub fn title_modal() -> Self {
        Self {
            path: PathBuf::from("docs/gfx/proto/e6-title.png"),
            at_frame: 90,
            torch_at: None,
            walk: None,
            prelude: None,
            stage_boss: None,
            create_at: None,
            exit_at: Some(150),
            click: None,
            stage_map: None,            levels: Vec::new(),
            ui_cases: Vec::new(),
            stage: None,
            fired: false,
            prelude_fired: false,
            doors_smoke: false,
        }
    }
    pub fn iso_burrow() -> Self {
        Self {
            path: PathBuf::from("docs/gfx/proto/e4-iso-burrow.png"),
            at_frame: 135,
            torch_at: Some(40),
            walk: Some((104, 105, (1, 1))),
            prelude: None,
            stage_boss: Some((100, 6)),
            create_at: Some((14, 4)),
            exit_at: Some(230),
            click: None,
            stage_map: None,            levels: Vec::new(),
            ui_cases: Vec::new(),
            stage: None,
            fired: false,
            prelude_fired: false,
            doors_smoke: false,
        }
    }

    /// Overworld ford: spawn on a walkable crossing beside the visible river,
    /// rather than strand the player on a non-walkable River cell.
    pub fn water() -> Self {
        Self {
            path: PathBuf::from("docs/gfx/proto/e4-water.png"),
            at_frame: 110,
            torch_at: None,
            walk: None,
            prelude: None,
            stage_boss: None,
            create_at: Some((14, 0)),
            exit_at: Some(190),
            click: None,
            stage: None,
            stage_map: Some((40, 0, Pos::new(100, 65))),
            levels: Vec::new(),
            ui_cases: Vec::new(),
            fired: false,
            prelude_fired: false,
            doors_smoke: false,
        }
    }


    /// Level gallery: one capture per map, in a single run. The caller passes
    /// the real map count: a hardcoded one walks off the end of `Game::maps`
    /// and panics the first time the world adds or drops a map.
    pub fn levels(count: usize) -> Self {
        let base = PathBuf::from("docs/gfx/proto/levels");
        let levels = (0..count)
            .map(|map| (30 + map as u32 * 4, map, base.join(format!("{map:02}.png"))))
            .collect();
        Self {
            path: PathBuf::from("docs/gfx/proto/e4-levels-last.png"),
            // No single `at_frame`: the level list owns every capture frame.
            at_frame: 1,
            torch_at: None,
            walk: None,
            prelude: None,
            stage_boss: None,
            create_at: Some((10, 0)),
            exit_at: Some(30 + count as u32 * 4 + 30),
            click: None,
            stage: None,
            stage_map: None,
            levels,
            ui_cases: Vec::new(),
            fired: false,
            prelude_fired: false,
            doors_smoke: false,
        }
    }

    /// UI gallery: one capture per modal state, same single-run reasoning. The
    /// cases are listed in `UI_CASES`; each one stages the state it needs, so a
    /// "talk" shot is a real conversation and not an empty pane.
    pub fn ui_gallery() -> Self {
        let base = PathBuf::from("docs/gfx/proto/ui-cases");
        let ui_cases = UI_CASES
            .iter()
            .enumerate()
            .map(|(index, (name, _))| (30 + index as u32 * UI_CASE_STRIDE, *name, base.join(format!("{name}.png"))))
            .collect();
        Self {
            path: PathBuf::from("docs/gfx/proto/e4-ui-last.png"),
            at_frame: 1,
            torch_at: None,
            walk: None,
            prelude: None,
            stage_boss: None,
            create_at: Some((10, 0)),
            exit_at: Some(30 + UI_CASES.len() as u32 * UI_CASE_STRIDE + 30),
            click: None,
            stage: None,
            // Millbrook has the cast a dialogue pane needs: civilians and a vendor.
            stage_map: Some((12, 1, Pos::new(12, 19))),
            levels: Vec::new(),
            ui_cases,
            fired: false,
            prelude_fired: false,
            doors_smoke: false,
        }
    }

    /// E4-s4 HUD proof: Sigil-Sworn with a channel charge, fighting the Chief
    /// in the Burrow — orbs/meter chip/nameplate up in one frame.
    pub fn hud_s4() -> Self {
        Self {
            path: PathBuf::from("docs/gfx/proto/e4-hud.png"),
            at_frame: 135,
            torch_at: Some(40),
            // Exactly one step INTO the boss: a single combat round, so the
            // frame catches the fight live with the player standing (Chief
            // rounds cost ~8 HP out of 17 when SigilSworn).
            walk: Some((104, 105, (1, 1))),
            prelude: None,
            stage_boss: Some((100, 6)),
            create_at: Some((14, 4)), // order index 4 = SigilSworn
            exit_at: Some(230),
            click: None,
            stage_map: None,            levels: Vec::new(),
            ui_cases: Vec::new(),
            stage: None,
            fired: false,
            prelude_fired: false,
            doors_smoke: false,
        }
    }

    /// E6 pointer smoke: scripted click 8 tiles east — the headless
    /// click-to-move run (autowalk walks it through the real pacemaker). The
    /// player is staged on the open street row first: seed-42 spawns inside
    /// a building and its longest straight greedy lane is 4 tiles. The bin's
    /// exit print shows the player exactly on the picked tile (12,23).
    pub fn pointer_smoke() -> Self {
        Self {
            path: PathBuf::from("docs/gfx/proto/e6-pointer.png"),
            at_frame: 150,
            torch_at: Some(40),
            walk: None,
            prelude: None,
            stage_boss: None,
            create_at: None,
            click: Some((20, (8, 0))),
            stage: Some((19, Pos::new(4, 23))),
            stage_map: None,            levels: Vec::new(),
            ui_cases: Vec::new(),
            exit_at: Some(200),
            fired: false,
            prelude_fired: false,
            doors_smoke: false,
        }
    }

    pub fn doors() -> Self {
        let mut plan = Self::iso_foundation();
        plan.path = "docs/gfx/proto/e4-door-open.png".into();
        plan.at_frame = 100;
        plan.exit_at = Some(440);
        plan.walk = None;
        plan.torch_at = Some(30);
        plan.create_at = Some((14, 0));
        plan.stage_map = Some((20, 4, Pos::new(20, 16)));
        plan.prelude = Some((50, "docs/gfx/proto/e4-door-closed.png".into()));
        plan.doors_smoke = true;
        plan
    }


}

/// Authored environment family for the current map. Floor and masonry resolve
/// actual material states together, rather than tinting a shared grey plate.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Zone {
    /// Millbrook and the other surface settlements: dressed, warm ashlar.
    #[default]
    Town,
    /// First dungeon theming: wet crypt stone.
    Crypt,
    /// Second: cold violet sanctum slate.
    Sanctum,
    /// The Underkeep: the Lich's own pale, drained masonry.
    Underkeep,
    Cave,
    Woodland,
    Arena,
    Dock,
}

impl Zone {
    pub fn of(kind: MapKind) -> Self {
        match kind {
            MapKind::Overworld => Zone::Woodland,
            MapKind::City(_) => Zone::Town,
            MapKind::Cellar => Zone::Crypt,
            MapKind::Arena => Zone::Arena,
            MapKind::Dungeon(0, _) | MapKind::Dungeon(3, 0) => Zone::Cave,
            MapKind::Dungeon(1, _) => Zone::Sanctum,
            MapKind::Dungeon(2, _) => Zone::Underkeep,
            MapKind::Dungeon(3, _) => Zone::Dock,
            MapKind::Dungeon(4, _) => Zone::Crypt,
            MapKind::Dungeon(5, _) => Zone::Town,
            MapKind::Dungeon(6..=7, _) => Zone::Woodland,
            MapKind::Dungeon(_, _) => Zone::Arena,
        }
    }

}

/// Per-frame read-only copy of everything the render layer needs.
#[derive(Resource)]
pub struct WorldView {
    /// Masonry theme of the current map (`Zone::of(map.kind)`).
    pub zone: Zone,
    /// Display name of the current map; the HUD's fallback readout when a class
    /// has no meter yet, so the central inset is never blank.
    pub map_name: String,
    pub player: Pos,
    pub map_w: i32,
    pub map_h: i32,
    pub radius: i32,
    pub night: bool,
    pub torchlit: bool,
    pub in_combat: bool,
    pub hour: u32,
    pub tick: u64,
    pub effects: Vec<(Pos, String, u8)>,
    /// Live NPCs on the player's map (see `ActorView`); player figure data too.
    pub actors: Vec<ActorView>,
    pub player_class: Class,
    /// Chosen body build; part of the player plate key.
    pub player_build: Build,
    /// Equipped boss relic, if any; drives the chest-pendant overlay.
    pub player_relic: Option<BossRelic>,
    pub player_weapon: u8,
    pub player_armour: u8,
    pub player_hp: i32,
    pub player_max_hp: i32,
    pub player_mana: i32,
    pub player_max_mana: i32,
    pub player_stamina: i32,
    pub player_max_stamina: i32,
    pub player_sigils: [bool; 3],
    pub player_xp: u32,
    pub player_level: u32,
    /// src/ui.rs `class_meter()` output verbatim (text + class-hue index),
    /// computed snapshot-side so TUI and window read identically.
    pub class_meter: Option<(String, MeterHue)>,
    /// Portal decals of the player's current map (props sheet decals).
    pub portals: Vec<Pos>,
    /// Boss aggro nameplate plumbing: the boss currently in combat with us.
    pub boss_plate: Option<BossPlateView>,
    /// Any sim modal is open this frame (title included) — the E6 pointer
    /// lane disables click-to-move / edge-pan / tap-inspect while this holds.
    pub modal_open: bool,
    /// Death freezes input the same way a modal does.
    pub player_dead: bool,
    /// Tile of the pointer-inspected actor (pointer.rs → nameplate lane).
    pub inspected: Option<Pos>,
    /// Window grid: columns/rows of cells around the player (see `grid_dims`).
    pub cols: u32,
    pub rows: u32,
    pub left: i32,
    pub top: i32,
    pub cells: Vec<Cell>,
}

#[derive(Clone)]
pub struct ActorView {
    pub id: usize,
    pub pos: Pos,
    pub archetype: Archetype,
    pub name: String,
    pub hp: i32,
    pub max_hp: i32,
    pub band: i32,
    pub phase: u8,
    pub companion: bool,
    /// The NPC's intent is an attack this frame, so the view can play the
    /// strike lane instead of idling through someone hitting it.
    pub attacking: bool,
}

/// src/ui.rs `class_meter` colors, in order: window-side maps these to the
/// gui.rs Ink triples (hud.rs).
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum MeterHue {
    LightYellow,   // Keepwarden
    Ivory225,      // Gravebound VIGIL (rgb 240,235,215)
    RustOrange,    // Redwake (rgb 220,120,60)
    Olive,         // Waysworn (rgb 175,160,95)
    LightCyan,     // SigilSworn
    Violet,        // Fensworn (rgb 170,120,220)
}

/// Verbatim port of src/ui.rs `class_meter(game)`: identical strings and the
/// identity hue per class. Keep in lockstep with the TUI function.
fn class_meter(game: &Game) -> Option<(String, MeterHue)> {
    let p = &game.player;
    let state = &p.class_state;
    let (meter, hue) = match p.class {
        Class::None => return None,
        Class::Keepwarden => (
            format!("  RESOLVE {} — {}", state.resolve, state.stance.name()),
            MeterHue::LightYellow,
        ),
        Class::Gravebound => {
            let tier = if p.hp * 4 < p.max_hp {
                "BLAZING"
            } else if p.hp * 2 < p.max_hp {
                "lit"
            } else {
                "dim"
            };
            (format!("  VIGIL {tier}"), MeterHue::Ivory225)
        }
        Class::Redwake => (
            format!(
                "  MOMENTUM {}{}",
                state.momentum,
                if state.momentum >= 3 { " (exit paid)" } else { "" }
            ),
            MeterHue::RustOrange,
        ),
        Class::Waysworn => (
            format!(
                "  TRAIL {} banked",
                if state.trail_charges > 0 {
                    state.trail_charges.to_string()
                } else {
                    "none".to_string()
                }
            ),
            MeterHue::Olive,
        ),
        Class::SigilSworn => (
            format!("  CHANNEL {} charge", state.channel),
            MeterHue::LightCyan,
        ),
        Class::Fensworn => (
            format!("  BOND double in {}", 3 - state.bond),
            MeterHue::Violet,
        ),
    };
    Some((meter, hue))
}

#[derive(Clone)]
pub struct BossPlateView {
    pub id: usize,
    pub name: String,
    pub hp: i32,
    pub max_hp: i32,
    pub band: i32,
    pub pos: Pos,
}

#[derive(Clone, Copy)]
pub struct Cell {
    pub pos: Pos,
    pub explored: bool,
    pub tile: Tile,
    pub door_open: bool,
    pub door_along_x: bool,
}

impl Default for WorldView {
    fn default() -> Self {
        Self {
            zone: Zone::default(),
            map_name: String::new(),
            player: Pos::new(0, 0),
            map_w: 0,
            map_h: 0,
            radius: 0,
            night: false,
            torchlit: false,
            in_combat: false,
            hour: 0,
            tick: 0,
            effects: Vec::new(),
            actors: Vec::new(),
            player_class: Class::None,
            player_build: Build::Male,
            player_relic: None,
            player_weapon: 0,
            player_armour: 0,
            player_hp: 0,
            player_max_hp: 0,
            player_mana: 0,
            player_max_mana: 0,
            player_stamina: 0,
            player_max_stamina: 0,
            player_sigils: [false; 3],
            player_xp: 0,
            player_level: 0,
            class_meter: None,
            portals: Vec::new(),
            boss_plate: None,
            modal_open: false,
            player_dead: false,
            inspected: None,
            cols: 0,
            rows: 0,
            left: 0,
            top: 0,
            cells: Vec::new(),
        }
    }
}

/// Cells needed around the player so the fog window covers the whole viewport
/// even at full zoom-out (MIN tile size 16px). Three spare rings hide any
/// camera-lerp pop-in. Shared by the snapshot and the tile-grid spawner.
pub fn grid_dims(win_w: f32, win_h: f32) -> (u32, u32) {
    (
        (win_w / 16.0).ceil() as u32 + 6,
        (win_h / 16.0).ceil() as u32 + 6,
    )
}

/// Sim driver: wall time → `elapsed_ms` (the movement pacemaker), 4 Hz world
/// ticks, and the synchronous heuristic drain of the NPC decision outbox —
/// the same `heuristic` fallback the terminal sim loop uses (src/main.rs).
pub fn clock(time: Res<Time>, slot: ResMut<SimSlot>) {
    let slot = slot.into_inner();
    slot.frame += 1;
    if slot.game.player.hp <= 0 {
        return; // death freezes the world, same as the terminal Death modal
    }
    let delta = time.delta_secs_f64().min(0.1) * 1000.0;
    slot.game.elapsed_ms = slot.game.elapsed_ms.saturating_add(delta as u64);
    slot.sim_ms += delta;
    while slot.sim_ms >= 250.0 {
        slot.sim_ms -= 250.0;
        slot.game.tick_world();
    }
    for request in std::mem::take(&mut slot.game.outbox) {
        let decision = laya_realms::laya_client::heuristic(&request);
        slot.game.apply_decision(&decision);
    }
}

/// Build this frame's read-only snapshot.
pub fn sync(
    slot: Res<SimSlot>,
    windows: Query<&Window>,
    ins: Res<crate::pointer::PointerIns>,
    mut view: ResMut<WorldView>,
) {
    let game = &slot.game;
    let map = game.map();
    let (cols, rows) = windows
        .single()
        .map(|w| grid_dims(w.width(), w.height()))
        .unwrap_or((86, 51));
    let left = game.player.pos.x - cols as i32 / 2;
    let top = game.player.pos.y - rows as i32 / 2;
    view.player = game.player.pos;
    view.zone = Zone::of(map.kind);
    if view.map_name != map.name {
        view.map_name = map.name.clone();
    }
    view.map_w = map.width;
    view.map_h = map.height;
    view.radius = palette::vision_radius(game);
    view.night = game.night();
    view.torchlit = game.tick < game.player.torch_until;
    view.in_combat = game.combat.is_some();
    view.modal_open = !matches!(game.modal, Modal::None);
    view.player_dead = game.player.hp <= 0;
    view.hour = game.hour();
    view.tick = game.tick;
    view.cols = cols;
    view.rows = rows;
    view.left = left;
    view.top = top;
    view.effects.clear();
    view.effects.extend(
        game.effects
            .iter()
            .map(|e| (e.pos, e.text.clone(), e.ttl)),
    );
    view.player_class = game.player.class;
    view.player_weapon = game.player.weapon;
    view.player_armour = game.player.armour;
    view.player_hp = game.player.hp;
    view.player_max_hp = game.player.max_hp;
    view.player_mana = game.player.mana;
    view.player_max_mana = game.player.max_mana;
    view.player_stamina = game.player.stamina;
    view.player_max_stamina = game.player.max_stamina;
    view.player_sigils = game.player.sigils;
    view.player_xp = game.player.xp;
    view.player_level = game.player.level;
    view.class_meter = class_meter(game);
    view.actors.clear();
    view.actors.extend(
        game
            .npcs
            .iter()
            .filter(|n| n.alive() && n.map == game.player.map)
            .map(|n| ActorView {
                id: n.id,
                pos: n.pos,
                archetype: n.archetype,
                name: n.name.clone(),
                hp: n.hp,
                max_hp: n.max_hp,
                band: n.last_hp_band,
                phase: n.phase,
                companion: game.companion == Some(n.id),
                attacking: n.intent == Intent::Attack,
            }),
    );
    view.inspected = ins
        .inspected
        .and_then(|id| view.actors.iter().find(|a| a.id == id))
        .map(|a| a.pos);
    view.portals.clear();
    view.portals.extend(map.portals.iter().map(|p| p.pos));
    view.boss_plate = game
        .combat
        .as_ref()
        .and_then(|combat| {
            combat
                .participants
                .iter()
                .filter_map(|&id| game.npcs.get(id))
                .find(|n| n.archetype.boss())
        })
        .or_else(|| {
            // Boss aggro without combat yet (s4 plate rule): intent is Attack.
            game.npcs
                .iter()
                .find(|n| n.archetype.boss() && n.map == game.player.map && n.alive() && matches!(n.intent, Intent::Attack))
        })
        .map(|boss| BossPlateView {
            id: boss.id,
            name: boss.name.clone(),
            hp: boss.hp,
            max_hp: boss.max_hp,
            band: boss.last_hp_band,
            pos: boss.pos,
        });
    view.cells.clear();
    for row in 0..rows {
        for col in 0..cols {
            let pos = Pos::new(left + col as i32, top + row as i32);
            let (explored, tile) = match map.index(pos) {
                Some(index) => (
                    map.explored.get(index).copied().unwrap_or(false),
                    map.tiles.get(index).copied().unwrap_or(Tile::Grass),
                ),
                None => (false, Tile::Grass),
            };
            let door_along_x = if tile == Tile::Door {
                let horizontal = [pos.offset(-1, 0), pos.offset(1, 0)]
                    .into_iter().filter(|p| map.tile(*p) == Tile::Wall).count();
                let vertical = [pos.offset(0, -1), pos.offset(0, 1)]
                    .into_iter().filter(|p| map.tile(*p) == Tile::Wall).count();
                horizontal >= vertical
            } else { true };
            view.cells.push(Cell {
                pos, explored, tile,
                door_open: tile == Tile::Door && map.door_open(pos),
                door_along_x,
            });
        }
    }
}

/// Dev-runner beats: press the torch, capture the screenshot, exit cleanly.
pub fn dev_sequence(
    slot: ResMut<SimSlot>,
    mut ins: ResMut<crate::pointer::PointerIns>,
    handoff: Res<crate::ExitHandoff>,
    mut commands: Commands,
    mut exit: MessageWriter<AppExit>,
) {
    let frame = slot.frame;
    let slot = slot.into_inner();
    let Some(shot) = &mut slot.shot else { return };
    if let Some((stage_at, pos)) = shot.stage {
        if stage_at == frame {
            slot.game.player.pos = pos;
            slot.game.reveal();
        }
    }
    if let Some((click_at, (dx, dy))) = shot.click {
        // Scripted pointer event: same resolution as pointer::resolve_click's
        // walk branch (the pick math itself is unit-tested in pointer.rs).
        if click_at == frame {
            ins.target = Some(crate::pointer::AutowalkTarget::new(
                slot.game.player.pos.offset(dx, dy),
            ));
        }
    }
    if let Some((create_at, order)) = shot.create_at {
        if create_at == frame {
            let game = &mut slot.game;
            game.apply_creation(
                Class::from_index(order),
                laya_realms::model::Build::Male,
                laya_realms::model::Boon::from_index(0),
            );
        }
        // Sigil-Sworn proof: the engine's break_channel() resets a staged charge
        // on ANY move, so inject the charge right before the shot, after the
        // last walk beat resolves.
        if order == 4 && frame == shot.at_frame.saturating_sub(12) {
            let game = &mut slot.game;
            if game.player.class_state.channel == 0 {
                game.player.class_state.channel = 1;
            }
        }
    }
    if shot.torch_at == Some(frame) {
        let game = &mut slot.game;
        let torch = game
            .player
            .inventory
            .iter()
            .position(|i| *i == Item::Torch);
        if let Some(index) = torch {
            game.action(laya_realms::model::Action::Use(index));
        }
    }
    // Gallery captures. State is staged SETTLE frames before the capture: a
    // modal pane does not exist on the frame the modal is set, and a teleport
    // does not respawn the tile grid until the next one. Capturing on the same
    // frame gives a shot of the world with the pane missing.
    const SETTLE: u32 = 3;
    for (capture_at, map, path) in shot.levels.clone() {
        if frame + SETTLE == capture_at {
            let game = &mut slot.game;
            game.modal = Modal::None;
            game.player.map = map;
            if let Some(m) = game.maps.get_mut(map) {
                game.player.pos = Pos::new(m.width / 2, m.height / 2);
                // Gallery evidence is the map, not the fog ring: explore the
                // whole thing so the capture shows the level it documents.
                m.explored.fill(true);
                game.reveal();
            }
        }
        if frame == capture_at {
            capture(&mut commands, &path);
        }
    }
    for (capture_at, case, path) in shot.ui_cases.clone() {
        if frame + SETTLE == capture_at {
            let game = &mut slot.game;
            if let Some((_, build)) = UI_CASES.iter().find(|(name, _)| *name == case) {
                game.selected = 0;
                game.modal = build(game);
            }
        }
        if frame == capture_at {
            capture(&mut commands, &path);
        }
        // Release the modal only once the capture has been read back: clearing
        // it on the capture frame ships a bare world shot for every case.
        if frame == capture_at + UI_CASE_RELEASE {
            slot.game.modal = Modal::None;
        }
    }
    if let Some((stage_at, map, pos)) = shot.stage_map {
        if stage_at == frame {
            let game = &mut slot.game;
            stage_map_at(game, map, pos);
        }
    }
    if let Some((stage_at, map)) = shot.stage_boss {
        if stage_at == frame {
            stage_boss_at(&mut slot.game, map);
        }
    }
    if shot.doors_smoke {
        use laya_realms::model::Action;
        let door = Pos::new(20, 14);
        let game = &mut slot.game;
        match frame {
            20 => {
                // Keep the generated map untouched; remove combat distractions
                // from this deterministic joinery/passage evidence run.
                for npc in &mut game.npcs {
                    if npc.map == 4 { npc.map = 0; }
                }
                game.combat = None;
                game.maps[4].set_door_open(door, false);
            }
            70 | 115 => {
                game.move_ready_ms = 0;
                game.action(Action::Move(0, -1));
                assert_eq!(game.player.pos, door, "door passage must remain a single step");
                assert!(game.map().door_open(door), "passage opens the real leaf");
                game.action(Action::Interact);
                assert!(game.map().door_open(door), "occupied door cannot close on the player");
            }
            80 | 120 => {
                game.move_ready_ms = 0;
                game.action(Action::Move(0, -1));
                assert_ne!(game.player.pos, door);
            }
            65 | 108 | 96..=98 => {
                game.move_ready_ms = 0;
                game.action(Action::Move(0, if (96..=98).contains(&frame) { 1 } else { -1 }));
            }
            90 | 110 => {
                game.action(Action::Interact);
                assert!(!game.map().door_open(door), "adjacent E closes the unoccupied threshold");
            }
            95 => {
                game.action(Action::Interact);
                assert!(game.map().door_open(door), "adjacent E reopens the door");
            }
            125 => eprintln!("DOOR SMOKE OK: bump opens, occupied close refused, E closes/reopens, return passage opens; pos=(20,13@map4)"),
            130 => {
                // The other real cellar doorway runs along y. View both poses
                // from its east approach, without changing the generated walls.
                game.player.pos = Pos::new(14, 6);
                game.maps[4].set_door_open(Pos::new(12, 6), false);
                game.reveal();
            }
            150 | 210 => {
                let path = match (shot.path.ends_with("e4-door-square-open.png"), frame == 210) {
                    (false, false) => "docs/gfx/proto/e4-door-y-closed.png",
                    (false, true) => "docs/gfx/proto/e4-door-y-open.png",
                    (true, false) => "docs/gfx/proto/e4-door-square-y-closed.png",
                    (true, true) => "docs/gfx/proto/e4-door-square-y-open.png",
                };
                capture(&mut commands, std::path::Path::new(path));
            }
            170 | 175 | 180 | 185 => {
                game.move_ready_ms = 0;
                game.action(Action::Move(if frame <= 175 { -1 } else { 1 }, 0));
                if frame == 175 {
                    assert_eq!(game.player.pos, Pos::new(12, 6));
                    assert!(game.map().door_open(Pos::new(12, 6)));
                }
                if frame == 185 {
                    assert_eq!(game.player.pos, Pos::new(14, 6));
                    eprintln!("DOOR Y SMOKE OK: real passage opens perpendicular doorway");
                }
            }
            240 | 340 => {
                // Thin, authored town walls expose both jamb-to-wall joints;
                // the cellar's long rock-cut corridors obscure the lower leaf.
                let (door, stand) = if frame == 240 {
                    (Pos::new(10, 25), Pos::new(10, 27))
                } else {
                    (Pos::new(14, 9), Pos::new(16, 9))
                };
                game.player.map = 1;
                game.player.pos = stand;
                for npc in &mut game.npcs {
                    if npc.map == 1 { npc.map = 0; }
                }
                game.combat = None;
                game.maps[1].set_door_open(door, false);
                game.reveal();
            }
            260 | 320 | 360 | 420 => {
                let square = shot.path.ends_with("e4-door-square-open.png");
                let path = match (square, frame) {
                    (false, 260) => "docs/gfx/proto/e4-door-wall-x-closed.png",
                    (false, 320) => "docs/gfx/proto/e4-door-wall-x-open.png",
                    (false, 360) => "docs/gfx/proto/e4-door-wall-y-closed.png",
                    (false, _) => "docs/gfx/proto/e4-door-wall-y-open.png",
                    (true, 260) => "docs/gfx/proto/e4-door-square-wall-x-closed.png",
                    (true, 320) => "docs/gfx/proto/e4-door-square-wall-x-open.png",
                    (true, 360) => "docs/gfx/proto/e4-door-square-wall-y-closed.png",
                    (true, _) => "docs/gfx/proto/e4-door-square-wall-y-open.png",
                };
                capture(&mut commands, std::path::Path::new(path));
            }
            280 | 285 | 290 | 295 | 380 | 385 | 390 | 395 => {
                let x_wall = frame < 300;
                let forward = frame % 100 <= 85;
                let step = if forward { -1 } else { 1 };
                game.move_ready_ms = 0;
                game.action(Action::Move(if x_wall { 0 } else { step }, if x_wall { step } else { 0 }));
                let door = if x_wall { Pos::new(10, 25) } else { Pos::new(14, 9) };
                if frame % 100 == 85 {
                    assert_eq!(game.player.pos, door);
                    assert!(game.map().door_open(door));
                    eprintln!("DOOR WALL SMOKE OK: wall-connected {} doorway opens through passage", if x_wall { "x" } else { "y" });
                }
            }
            _ => {}
        }
    }
    if let Some((start, end, (dx, dy))) = shot.walk {
        // Through the real verb: the engine pacemaker (85 ms × terrain cost)
        // still gates cadence, exactly like held keys do.
        if (start..=end).contains(&frame) {
            slot.game.action(laya_realms::model::Action::Move(dx, dy));
        }
    }
    if !shot.prelude_fired {
        if let Some((prelude_at, prelude_path)) = &shot.prelude {
            if *prelude_at == frame {
                shot.prelude_fired = true;
                capture(&mut commands, prelude_path);
            }
        }
    }
    if !shot.fired && frame >= shot.at_frame {
        shot.fired = true;
        capture(&mut commands, &shot.path);
    }
    if shot.fired && shot.exit_at == Some(frame) {
        *handoff.0.lock() = Some(std::mem::replace(&mut slot.game, Game::new(0)));
        exit.write(AppExit::Success);
    }
}

fn capture(commands: &mut Commands, path: &std::path::Path) {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let path = std::env::current_dir()
        .map(|dir| dir.join(path))
        .unwrap_or_else(|_| path.to_path_buf());
    commands
        .spawn(Screenshot::primary_window())
        .observe(save_to_disk(path));
}

/// Teleport next to the boss on `map` (dev evidence staging only — the sim
/// never calls this), scan west for a walkable stand, then re-reveal.

/// The level gallery walks every map the game actually built; the runner
/// passes `Game::maps.len()` so the plan can never index past `Game::maps`.

/// Frames between UI gallery cases. A case sets its modal SETTLE frames before
/// its capture and the modal is only released a few frames after it, because
/// the screenshot readback lands on a later rendered frame than the command.
const UI_CASE_STRIDE: u32 = 8;
const UI_CASE_RELEASE: u32 = 3;

/// Every modal state the UI gallery shows, and the state each one needs. A
/// capture of "talk" against an empty map is not evidence of a dialogue pane.

pub const UI_CASES: [(&str, fn(&mut Game) -> Modal); 14] = [
    ("title", |_| Modal::Title),
    ("create-class", |_| Modal::Create),
    ("create-build", |g| {
        g.create_step = 1;
        g.create_class = 0;
        g.selected = 0;
        Modal::Create
    }),
    ("create-boon", |g| {
        g.create_step = 2;
        g.create_class = 0;
        g.create_build = 1;
        g.selected = 0;
        Modal::Create
    }),
    ("help", |_| Modal::Help),
    ("pause", |_| Modal::Pause),
    ("inventory", |g| {
        stock_satchel(g);
        Modal::Inventory
    }),
    ("journal", |g| {
        give_quest_state(g);
        Modal::Journal
    }),
    ("atlas", |g| {
        give_quest_state(g);
        Modal::Atlas
    }),
    ("talents", |g| {
        g.player.level = 6;
        g.player.talent_points = 7;
        g.player.talents = vec![0, 1, 4, 5, 8];
        g.selected = 2;
        Modal::Talents
    }),
    ("talk", |g| Modal::Talk(nearest(g, |n| n.archetype != Archetype::BroodHole).unwrap_or(0))),
    ("trade", |g| Modal::Trade(nearest(g, |n| n.archetype == Archetype::Vendor).unwrap_or(0))),
    ("forge", |g| Modal::Forge(nearest(g, |n| n.archetype != Archetype::BroodHole).unwrap_or(0))),
    ("oath", |g| {
        g.modal = Modal::Create;
        g.create_step = 2;
        Modal::Oath
    }),
];

/// A satchel worth photographing: the real starting kit plus a gear ladder, so
/// the detail pane and the equipped column both have something to show.
fn stock_satchel(game: &mut Game) {
    let p = &mut game.player;
    if p.inventory.iter().any(|i| matches!(i, Item::GlyphShard)) {
        return;
    }
    p.inventory = vec![
        Item::Potion,
        Item::Potion,
        Item::GreaterPotion,
        Item::Ration,
        Item::Torch,
        Item::Weapon(3),
        Item::Armour(2),
        Item::GlyphShard,
        Item::GemDust,
        Item::OreFlake,
        Item::Relic,
        Item::Key(0),
    ];
    p.gold = 340;
    p.weapon = 2;
    p.armour = 2;
    p.relic = Some(BossRelic::Stoneheart);
}

fn give_quest_state(game: &mut Game) {
    stock_satchel(game);
    let p = &mut game.player;
    p.reputation = [70, 50, 35];
    p.keys = [true, false, false];
    p.sigils = [true, false, false];
    p.level = 4;
    p.gold = 340;
}

fn nearest(game: &Game, want: impl Fn(&Npc) -> bool) -> Option<usize> {
    game.npcs
        .iter()
        .position(|n| n.map == game.player.map && n.alive() && want(n))
}

fn stage_map_at(game: &mut Game, map: usize, pos: Pos) {
    if map >= game.maps.len() {
        return;
    }
    game.player.map = map;
    game.player.pos = pos;
    game.reveal();
    game.log("dev: staged to an explicit map and tile.");
}

fn stage_boss_at(game: &mut Game, map: usize) {
    let Some(boss) = game
        .npcs
        .iter()
        .find(|n| n.map == map && n.archetype.boss() && n.alive())
    else {
        return;
    };
    let target = boss.pos;
    let walkable = |p: Pos| {
        game.maps
            .get(map)
            .and_then(|m| m.index(p).map(|i| m.tiles.get(i).copied().unwrap_or(Tile::Rock)))
            .is_some_and(|t| t.walkable())
    };
    let taken = |p: Pos| game.npcs.iter().any(|n| n.alive() && n.map == map && n.pos == p);
    // Adjacency first, but SOUTH of the boss: in the isometric the lower band
    // draws in front, so a player staged north of the boss ended up hidden
    // behind it — the capture showed the back of a helmet where the Lich was.
    const SPOTS: [(i32, i32); 10] = [
        (1, 1), (0, 1), (1, 0), (2, 1), (1, 2), (2, 2), (0, 2), (2, 0), (-1, 1), (1, -1),
    ];
    let chosen = SPOTS
        .iter()
        .map(|(dx, dy)| target.offset(*dx, *dy))
        .find(|p| walkable(*p) && !taken(*p));
    if let Some(p) = chosen {
        game.player.map = map;
        game.player.pos = p;
        game.reveal();
        game.log("dev: staged next to the boss for the E4-s3 proof frame.");
    }
}
