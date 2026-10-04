//! realmscape — development runner for the E4 Bevy view.
//!
//! Opens a headed window on the REAL seed-42 game (through the production lib
//! path), bypasses the title screen (creation UI is E6), presses the torch
//! quick-use at frame 40, captures docs/gfx/proto/e4-foundation.png at frame
//! ~120 via Bevy Screenshot, and exits cleanly. WASD/arrows + Home/End/PgUp/
//! PgDn to walk, F/X/C/Space verbs, Enter/Shift+A attack, T torch, wheel zoom;
//! pointer: click-to-move, adjacent strikes/talk, inspect, touch tap/drag.

use laya_realms::model::{Game, Modal};
use realms_view::sim::ShotPlan;
use realms_view::{launch_with, ViewConfig};

fn main() {
    // Plan selector: `realmscape hud` → e4-hud.png runbook (SigilSworn staged);
    // `realmscape foundation` → e4-foundation.png; default → e4-actors.png.
    let plan_arg = std::env::args().nth(1).unwrap_or_default();
    let mut game = Game::new(42);
    let (plan, proj) = match plan_arg.as_str() {
        "hud" => (ShotPlan::hud_s4(), realms_view::projection::Proj::Square),
        "foundation" => (ShotPlan::foundation(), realms_view::projection::Proj::Square),
        "iso" => (ShotPlan::iso_foundation(), realms_view::projection::Proj::Iso),
        "doors" => (ShotPlan::doors(), realms_view::projection::Proj::Iso),
        "doors-square" => {
            let mut shot = ShotPlan::doors();
            shot.path = "docs/gfx/proto/e4-door-square-open.png".into();
            shot.prelude = Some((50, "docs/gfx/proto/e4-door-square-closed.png".into()));
            (shot, realms_view::projection::Proj::Square)
        }
        "city-live" => {
            let mut shot = ShotPlan::iso_foundation();
            shot.walk = None;
            shot.at_frame = u32::MAX;
            shot.exit_at = None;
            (shot, realms_view::projection::Proj::Iso)
        }
        "iso-burrow" => (ShotPlan::iso_burrow(), realms_view::projection::Proj::Iso),
        // Stage the real Underkeep boss; the live variant remains playable
        // until the window is closed rather than taking a shot and exiting.
        "lich" | "lich-live" => {
            let mut shot = ShotPlan::iso_burrow();
            shot.path = "docs/gfx/proto/e4-iso-lich.png".into();
            shot.stage_boss = Some((100, 11));
            shot.walk = None;
            if plan_arg == "lich-live" {
                shot.at_frame = u32::MAX;
                shot.exit_at = None;
            }
            (shot, realms_view::projection::Proj::Iso)
        }
        "title" => (ShotPlan::title_modal(), realms_view::projection::Proj::Iso),
        // E6 headless pointer smoke: scripted click 8 tiles east of spawn;
        // the exit print must show the player on the picked tile.
        "pointer" => (ShotPlan::pointer_smoke(), realms_view::projection::Proj::Iso),
        // Overworld river on the actual map; live mode keeps control after
        // staging the player rather than exiting after its screenshot frame.
        "water" | "water-live" => {
            let mut shot = ShotPlan::water();
            if plan_arg == "water-live" {
                shot.at_frame = u32::MAX;
                shot.exit_at = None;
            }
            (shot, realms_view::projection::Proj::Iso)
        }
        // Galleries: one run each, many captures (see `ShotPlan::levels`).
        // The gallery walks the maps the game actually built.
        "levels" => (ShotPlan::levels(game.maps.len()), realms_view::projection::Proj::Iso),
        "ui" => (ShotPlan::ui_gallery(), realms_view::projection::Proj::Iso),
        _ => (ShotPlan::actors_s3(), realms_view::projection::Proj::Square),
    };
    if plan_arg != "title" && matches!(game.modal, Modal::Title) {
        // E4 foundation has no modal UI; the title/create flow is E6. Skip it
        // (but the "title" plan keeps it — it IS the modal visibility probe).
        game.modal = Modal::None;
    }
    launch_with(
        &mut game,
        ViewConfig {
            title: format!("realmscape — Laya Realms E4 dev (seed 42, {plan_arg:?})"),
            shot: Some(plan),
            projection: proj,
        },
    );
    eprintln!(
        "realmscape exit: pos=({},{}@map{}) hp={}/{} mana={}/{} class={:?} cell={} night={} map_name={}",
        game.player.pos.x,
        game.player.pos.y,
        game.player.map,
        game.player.hp,
        game.player.max_hp,
        game.player.mana,
        game.player.max_mana,
        game.player.class,
        game.player.class_state.channel,
        game.night(),
        game.map().name,
    );
}
