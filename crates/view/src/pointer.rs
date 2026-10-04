//! E6 mouse & touch layer (docs/D2_EVOLUTION.md §7.4): click-to-move autowalk,
//! edge-pan camera, tap-to-inspect, and basic touch gestures.
//!
//! Parity rules (the sim never learns the pointer exists):
//! - Every world mutation goes through the same `Game::action(…)` verbs the
//!   keyboard lane uses: `Action::Move` for autowalk steps (one move per
//!   pacemaker tick, direction rewritten from the current position each tick)
//!   and `Action::Interact` (the TUI `e` key) for clicked portals/chests in
//!   reach. No new sim behavior.
//! - Wheel zoom is NOT here — `camera::follow` already owns it (with
//!   `options::ViewOptions` persistence); this lane only reads the state.
//!
//! Camera anchoring: `camera::follow` lerps onto the player. Edge-pan and
//! two-finger drag add a *temporary* world offset on top (`PointerIns.pan`),
//! which the follow system folds into its anchor; the offset decays back to
//! zero so the player re-centres when the pointer leaves the window edge.

use bevy::input::touch::Touch;
use bevy::prelude::*;
use laya_realms::model::{Action, Game, Modal, Pos, Tile};

use crate::camera::{CamFrame, MainCam};
use crate::{combat_controls, hud};
use crate::input;
use crate::projection::{iso_to_tile, Proj, T};
use crate::sim::{SimSlot, WorldView};

/// Pointer within this many pixels of a window edge edge-pans (CSS px).
pub const EDGE_MARGIN: f32 = 16.0;
/// Edge-pan speed in tiles per second.
pub const EDGE_PAN_RATE: f32 = 6.0;
/// Pan offset decay per second once no pan input is active.
pub const PAN_DECAY: f32 = 4.0;
/// A click lands "on" an actor within this many world units of the actor's
/// tile centre (≈ half a tile).
pub const PICK_RADIUS: f32 = T * 0.5;

#[derive(Resource, Default)]
pub struct PointerIns {
    /// Click-to-move destination; `None` when walking by keyboard/stationary.
    pub target: Option<AutowalkTarget>,
    /// Actor id last tapped (surfaced by the nameplate lane via WorldView).
    pub inspected: Option<usize>,
    /// Temporary camera offset (world units) from edge-pan / two-finger drag.
    pub pan: Vec2,
    /// Two-finger centroid from the previous frame (drag delta), screen px.
    touch_centroid: Option<Vec2>,
    /// Wall time (secs) the last autowalk action was dispatched at — gates the
    /// rhythm so the next step schedules at smoothed completion, not raw
    /// pacemaker blink (capped by the sim pacemaker regardless).
    pub last_step_at: f32,
}

#[derive(Clone, PartialEq, Debug)]
pub struct AutowalkTarget {
    pub target: Pos,
    /// Direction of the last dispatched step (observability/testing).
    pub last_dir: (i32, i32),
}

impl AutowalkTarget {
    pub fn new(target: Pos) -> Self {
        Self {
            target,
            last_dir: (0, 0),
        }
    }
}

/// What a click resolved to (mirrors the resolution order below).
#[derive(Clone, PartialEq, Debug)]
pub enum ClickOutcome {
    /// Clicked a distant or unreachable actor: inspect its nameplate.
    Inspected(usize),
    /// Clicked a portal/chest within reach: fired `Action::Interact`.
    Interacted,
    /// Clicked anywhere else: (re)targeted click-to-move.
    Autowalk(Pos),
    /// One deliberate directional strike at this actor.
    Struck(usize),
    /// One legal adjacent combat reposition.
    Moved(Pos),
    /// Invalid combat ground or a pick blocked by a modal.
    Ignored,
}

/// World-space point → tile coordinate: the exact inverse of [`Proj::world`]'s
/// anchor (iso: the tested `iso_to_tile`; square: the trivial flip).
pub fn pick_tile(proj: Proj, world_x: f32, world_y: f32) -> Pos {
    match proj {
        Proj::Square => Pos::new((world_x / T).round() as i32, (-world_y / T).round() as i32),
        Proj::Iso => iso_to_tile(world_x, world_y),
    }
}

/// Is the click-to-move over? Pure stop test — the full halt list:
/// modal open (title/pause/…), combat joined, player dead, movement-key
/// intent (a DIRS key just-pressed *or* held), target reached, or a revealed
/// hostile within 1 tile of the player (the D34 safety rule: stop when a
/// revealed hostile enters hostile LOS, here ≈ adjacency).
pub fn autowalk_halt(
    game: &Game,
    view: &WorldView,
    target: &AutowalkTarget,
    ignore_modal: bool,
    dir_key_intent: bool,
) -> bool {
    if !ignore_modal && !matches!(game.modal, Modal::None) {
        return true;
    }
    if game.player.hp <= 0 || game.combat.is_some() || dir_key_intent {
        return true;
    }
    if game.player.pos == target.target {
        return true;
    }
    view.actors
        .iter()
        .any(|a| a.archetype.hostile() && !a.companion && a.pos.distance(game.player.pos) <= 1)
}

/// One autowalk beat: halt, wait out the pacemaker, or dispatch one
/// `Action::Move` step toward the target through the keyboard's verb. The
/// direction is rewritten from the CURRENT player position every tick
/// (greedy Chebyshev step). A step that doesn't move the player and doesn't
/// open combat (wall, mob in the way, …) cancels the walk.
pub fn step_autowalk(
    game: &mut Game,
    target: &mut Option<AutowalkTarget>,
    view: &WorldView,
    ignore_modal: bool,
    dir_key_intent: bool,
) {
    let Some(aw) = target.clone() else { return };
    if autowalk_halt(game, view, &aw, ignore_modal, dir_key_intent) {
        *target = None;
        return;
    }
    // Pacemaker gate: at most one move per tick, same as held keys.
    if game.elapsed_ms < game.move_ready_ms {
        return;
    }
    let pos = game.player.pos;
    let dx = (aw.target.x - pos.x).signum();
    let dy = (aw.target.y - pos.y).signum();
    if (dx, dy) == (0, 0) {
        *target = None;
        return;
    }
    game.action(Action::Move(dx, dy));
    if game.combat.is_some() || game.player.pos == pos {
        // Stepped into a hostile (combat owns input now) or the step found no
        // floor: the walk ends either way.
        *target = None;
        return;
    }
    *target = Some(AutowalkTarget {
        target: aw.target,
        last_dir: (dx, dy),
    });
}

/// Mouse click / single-finger tap → pick tile → verbs.
pub(crate) fn clicks(
    mut buttons: ResMut<ButtonInput<MouseButton>>,
    touches: Res<Touches>,
    windows: Query<&Window>,
    cams: Query<(&Camera, &GlobalTransform), With<MainCam>>,
    proj: Res<Proj>,
    view: Res<WorldView>,
    mut ins: ResMut<PointerIns>,
    mut slot: ResMut<SimSlot>,
    mut kb: ResMut<input::InputState>,
    feedback: Query<(&Transform, &Sprite, &Visibility), With<crate::combat_controls::FeedbackPlate>>,
) {
    // Single-finger touch mirrors the primary button for every other system.
    let mut tap_points: Vec<Vec2> = Vec::new();
    for touch in touches.iter_just_pressed() {
        if touches.iter().count() <= 1 {
            tap_points.push(touch.position());
            buttons.press(MouseButton::Left);
        }
    }
    if touches.iter().count() == 0 {
        for _ in touches.iter_just_released().chain(touches.iter_just_canceled()) {
            buttons.release(MouseButton::Left);
        }
    }

    // While any modal is open (title included) the pointer lane is disabled;
    // modal interactions are their own surface. Checked FIRST (before the
    // window/camera queries) so a click can never leak under a modal.
    if kb.frame_consumed {
        ins.target = None;
        return;
    }
    if view.modal_open || view.player_dead || !matches!(slot.game.modal, Modal::None) {
        ins.target = None;
        return;
    }
    let Ok(window) = windows.single() else { return };
    let Ok((camera, cam_tf)) = cams.single() else { return };

    // Taps resolve at the finger position; mouse clicks at the cursor.
    let mut picks: Vec<Vec2> = tap_points;
    if buttons.just_pressed(MouseButton::Left) && picks.is_empty() {
        if let Some(cursor) = window.cursor_position() {
            picks.push(cursor);
        }
    }
    for screen in picks {
        if slot.game.combat.is_some() {
            if let Some(index) = combat_controls::hit_test(screen, window.width(), window.height()) {
                ins.target = None;
                kb.frame_consumed = true;
                combat_controls::activate(&mut slot.game, index);
                break;
            }
        }
        // Both the rail's decorative metal and its keys intercept world picks.
        // Mouse and touch share this path, so a tap cannot walk through a key.
        if let Some(key) = hud::hit_test(screen, window.width(), window.height()) {
            ins.target = None;
            kb.frame_consumed = true;
            if let Some(index) = key {
                hud::open_button(&mut slot.game, index);
            }
            break;
        }
        if !matches!(slot.game.modal, Modal::None) {
            break; // another pick in this same touch frame cannot leak behind it
        }
        let Ok(world) = camera.viewport_to_world_2d(cam_tf, screen) else {
            continue;
        };
        if feedback.iter().any(|(transform, sprite, visibility)| {
            *visibility == Visibility::Visible && sprite.custom_size.is_some_and(|size| {
                (world - transform.translation.truncate()).abs()
                    .cmple(size * transform.scale.truncate() * 0.5).all()
            })
        }) {
            ins.target = None;
            kb.frame_consumed = true;
            break;
        }
        let tile = pick_tile(*proj, world.x, world.y);
        resolve_click(&mut slot.game, &view, *proj, &mut ins, tile, world);
        kb.frame_consumed = true;
        break; // one click/tap frame can commit at most one tactical action
    }
}

/// Resolve the nearest visible, live actor before ground. Adjacent hostiles
/// receive directional strikes; friendly interaction keeps the keyboard's
/// contextual priority. Combat ground accepts only one legal adjacent step.
pub fn resolve_click(
    game: &mut Game,
    view: &WorldView,
    proj: Proj,
    ins: &mut PointerIns,
    tile: Pos,
    world: Vec2,
) -> ClickOutcome {
    if !matches!(game.modal, Modal::None) || game.player.hp <= 0 {
        ins.target = None;
        return ClickOutcome::Ignored;
    }
    let picked = view.actors.iter()
        .filter_map(|a| {
            let npc = game.npcs.get(a.id)?;
            let explored = game.map().index(a.pos)
                .is_some_and(|i| game.map().explored[i]);
            if a.pos == game.player.pos || !npc.alive() || npc.map != game.player.map
                || npc.pos != a.pos || a.pos.distance(view.player) > view.radius || !explored
            {
                return None;
            }
            let distance = proj.world(a.pos, 0.0).truncate().distance_squared(world);
            (distance <= PICK_RADIUS * PICK_RADIUS).then_some((a, distance))
        })
        .min_by(|(a, da), (b, db)| da.total_cmp(db).then_with(|| a.id.cmp(&b.id)))
        .map(|(a, _)| a);
    if let Some(actor) = picked {
        let id = actor.id;
        ins.inspected = Some(id);
        ins.target = None;
        if !actor.companion
            && laya_realms::engine::hostile(&game.npcs[id])
            && game.can_melee(game.player.pos, actor.pos)
        {
            let dx = actor.pos.x - game.player.pos.x;
            let dy = actor.pos.y - game.player.pos.y;
            game.action(Action::Move(dx, dy));
            return ClickOutcome::Struck(id);
        }
        // Interact chooses the nearest ordinary NPC, not an arbitrary actor.
        // Only dispatch when it will choose the clicked friendly; portals and
        // truce wolves retain their higher contextual priority.
        let friendly = game.npcs.iter()
            .filter(|n| n.alive() && n.map == game.player.map
                && !n.archetype.hostile() && n.pos.distance(game.player.pos) <= 1)
            .min_by_key(|n| n.pos.distance(game.player.pos))
            .map(|n| n.id);
        if game.combat.is_none() && friendly == Some(id)
            && game.can_melee(game.player.pos, actor.pos)
            && !game.map().portals.iter().any(|p| p.pos.distance(game.player.pos) <= 1)
            && !game.npcs.iter().any(|n| n.alive() && n.map == game.player.map
                && n.archetype.hostile() && n.pos.distance(game.player.pos) <= 1)
        {
            game.action(Action::Interact);
            return ClickOutcome::Interacted;
        }
        return ClickOutcome::Inspected(id);
    }
    ins.inspected = None;
    ins.target = None;
    if game.combat.is_some() {
        if game.can_melee(game.player.pos, tile) && game.npc_at(tile).is_none() {
            let dx = tile.x - game.player.pos.x;
            let dy = tile.y - game.player.pos.y;
            game.action(Action::Move(dx, dy));
            return ClickOutcome::Moved(tile);
        }
        return ClickOutcome::Ignored;
    }
    let interactable = view.portals.contains(&tile)
        || view.cells.iter().any(|c| c.explored && c.pos == tile && c.tile == Tile::Chest);
    if interactable && tile.distance(game.player.pos) <= 1 {
        game.action(Action::Interact);
        return ClickOutcome::Interacted;
    }
    ins.target = Some(AutowalkTarget::new(tile));
    ClickOutcome::Autowalk(tile)
}

/// Autowalk scheduler: runs after keyboard input, one pacemaker step/frame.
pub fn autowalk(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    kb: Res<input::InputState>,
    view: Res<WorldView>,
    mut ins: ResMut<PointerIns>,
    mut slot: ResMut<SimSlot>,
) {
    let dir_key_intent =
        kb.held_move() || input::DIRS.iter().any(|(key, _, _)| keys.just_pressed(*key));
    if ins.target.is_none() {
        return;
    }
    // Rhythm gate: the next step schedules off the PREVIOUS step's smoothed
    // landing (~110 ms glide), not raw pacemaker boundaries; the sim pacemaker
    // inside step_autowalk still caps cadence regardless.
    if time.elapsed_secs() - ins.last_step_at < crate::actors::SMOOTH_MS {
        return;
    }
    let pos_before = slot.game.player.pos;
    step_autowalk(&mut slot.game, &mut ins.target, &view, false, dir_key_intent);
    if slot.game.player.pos != pos_before {
        ins.last_step_at = time.elapsed_secs();
    }
}

/// Edge-pan + two-finger-drag camera offset. Writes only `PointerIns.pan`;
/// `camera::follow` folds it into its anchor. Disabled while a modal is open
/// or the window is unfocused; the pan-adjusted anchor is clamped to the
/// projected world bounds (with a two-tile margin).
pub fn pan(
    time: Res<Time>,
    windows: Query<&Window>,
    proj: Res<Proj>,
    view: Res<WorldView>,
    touches: Res<Touches>,
    mut ins: ResMut<PointerIns>,
    frame: Res<CamFrame>,
) {
    let dt = time.delta_secs();
    let Ok(window) = windows.single() else { return };
    let mut push = Vec2::ZERO; // world-units this frame

    // Edge-pan: pointer near a window edge, no modal (title = disabled).
    if window.focused && !view.modal_open {
        if let Some(cursor) = window.cursor_position() {
            let mut dir = Vec2::ZERO;
            if cursor.x < EDGE_MARGIN {
                dir.x -= 1.0;
            }
            if cursor.x > window.width() - EDGE_MARGIN {
                dir.x += 1.0;
            }
            if cursor.y < EDGE_MARGIN {
                dir.y += 1.0; // screen y down → world y up
            }
            if cursor.y > window.height() - EDGE_MARGIN {
                dir.y -= 1.0;
            }
            push += dir * EDGE_PAN_RATE * T * dt;
        }
    }

    // Two-finger drag: centroid delta → world pan (drag moves the world).
    let fingers: Vec<&Touch> = touches.iter().collect();
    if fingers.len() == 2 {
        let c = (fingers[0].position() + fingers[1].position()) * 0.5;
        if let Some(prev) = ins.touch_centroid {
            push += (c - prev) * Vec2::new(-1.0, 1.0) * frame.scale;
        }
        ins.touch_centroid = Some(c);
    } else {
        ins.touch_centroid = None;
    }

    if push == Vec2::ZERO {
        // Nothing held: ease the offset back so the player re-centres.
        ins.pan *= (-PAN_DECAY * dt).exp();
        if ins.pan.length() < 0.5 {
            ins.pan = Vec2::ZERO;
        }
    } else {
        ins.pan += push;
    }

    // Clamp: the pan-adjusted anchor never leaves the projected world bounds.
    if view.map_w > 0 && view.map_h > 0 {
        let (min, max) = world_bounds(*proj, &view);
        let anchor = frame.pos + ins.pan;
        ins.pan.x = anchor.x.clamp(min.x, max.x) - frame.pos.x;
        ins.pan.y = anchor.y.clamp(min.y, max.y) - frame.pos.y;
    }
}

/// Projected world-bounds of the map, expanded by a two-tile margin.
fn world_bounds(proj: Proj, view: &WorldView) -> (Vec2, Vec2) {
    let (w, h) = (view.map_w, view.map_h);
    let corners = [
        Pos::new(0, 0),
        Pos::new(w, 0),
        Pos::new(0, h),
        Pos::new(w, h),
    ];
    let mut min = Vec2::splat(f32::MAX);
    let mut max = Vec2::splat(f32::MIN);
    for c in corners {
        let p = proj.world(c, 0.0).truncate();
        min = min.min(p);
        max = max.max(p);
    }
    (min - Vec2::splat(T * 2.0), max + Vec2::splat(T * 2.0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::ActorView;
    use laya_realms::model::{Archetype, Intent, Modal};

    /// Realmscape parity: the real seed-42 game as a "headless SimSlot",
    /// title bypassed exactly like the bin does.
    fn headless_game() -> Game {
        let mut game = Game::new(42);
        game.modal = Modal::None;
        game
    }

    /// Minimal snapshot hand-roll: player pos + live actors, like sim::sync.
    fn view_for(game: &Game, _proj: Proj) -> WorldView {
        let mut view = WorldView::default();
        view.player = game.player.pos;
        view.map_w = game.map().width;
        view.map_h = game.map().height;
        view.radius = crate::palette::vision_radius(game);
        view.actors = game
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
            })
            .collect();
        view
    }

    #[test]
    fn pick_inverts_projection_anchors() {
        for proj in [Proj::Square, Proj::Iso] {
            for (x, y) in [(0, 0), (5, 2), (-3, 7), (11, -6), (31, 31)] {
                let w = proj.world(Pos::new(x, y), 0.0);
                assert_eq!(pick_tile(proj, w.x, w.y), Pos::new(x, y));
            }
        }
    }

    #[test]
    fn autowalk_reaches_target_eight_tiles_out() {
        let mut game = headless_game();
        stage_on_street(&mut game);
        let mut view = view_for(&game, Proj::Iso);
        let goal = game.player.pos.offset(8, 0);
        let mut target = Some(AutowalkTarget::new(goal));
        for _ in 0..512 {
            view.player = game.player.pos;
            step_autowalk(&mut game, &mut target, &view, true, false);
            // Wall-clock pacemaker, advanced like sim::clock does.
            game.elapsed_ms += 100;
            if target.is_none() {
                break;
            }
        }
        assert_eq!(game.player.pos, goal, "click-to-move walked 8 tiles");
        assert!(target.is_none(), "autowalk cleared on target reached");
    }

    #[test]
    fn autowalk_halts_on_modal() {
        let mut game = headless_game();
        let view = view_for(&game, Proj::Iso);
        let goal = game.player.pos.offset(3, 0);
        let mut target = Some(AutowalkTarget::new(goal));
        game.modal = Modal::Pause;
        step_autowalk(&mut game, &mut target, &view, false, false);
        assert!(target.is_none(), "a modal opening cancels the walk");
        assert_ne!(game.player.pos, goal, "no step dispatched under a modal");
    }

    #[test]
    fn autowalk_halts_on_tile_reached() {
        let mut game = headless_game();
        let view = view_for(&game, Proj::Iso);
        let here = game.player.pos;
        let mut target = Some(AutowalkTarget::new(here));
        step_autowalk(&mut game, &mut target, &view, true, false);
        assert!(target.is_none(), "standing on the target cancels the walk");
    }

    #[test]
    fn autowalk_halts_on_unit_lost_to_hostile_los() {
        let mut game = headless_game();
        let mut view = view_for(&game, Proj::Iso);
        // A revealed hostile exactly 1 tile away: the D34 safety rule says
        // click-to-move hands the unit back to the player.
        let near = game.player.pos.offset(1, 0);
        view.actors.push(ActorView {
            id: usize::MAX,
            pos: near,
            archetype: Archetype::Wolf,
            name: "wolf".into(),
            hp: 8,
            max_hp: 8,
            band: 4,
            phase: 0,
            companion: false,
            attacking: false,
        });
        let origin = game.player.pos;
        let goal = origin.offset(-4, 0);
        let mut target = Some(AutowalkTarget::new(goal));
        step_autowalk(&mut game, &mut target, &view, true, false);
        assert!(target.is_none(), "hostile within 1 tile halts the walk");
        assert_eq!(game.player.pos, origin, "no step dispatched");
    }

    #[test]
    fn autowalk_halts_on_movement_key() {
        let mut game = headless_game();
        let view = view_for(&game, Proj::Iso);
        let goal = game.player.pos.offset(4, 0);
        let mut target = Some(AutowalkTarget::new(goal));
        step_autowalk(&mut game, &mut target, &view, true, true /* W just pressed */);
        assert!(target.is_none(), "keyboard move intent cancels the walk");
    }

    /// Stage on the open Millbrook street row for long-lane walk tests.
    /// Seed-42 spawns INSIDE a building (longest straight lane from spawn is
    /// 4 tiles north), so genuinely-open-ground proofs stand the player on
    /// the south street first — same fixture style as `stage_boss_at`.
    fn stage_on_street(game: &mut Game) {
        game.player.pos = Pos::new(4, 23);
        debug_assert!(game.map().tile(Pos::new(12, 23)).walkable());
    }

    #[test]
    fn autowalk_halts_on_blocked_step() {
        let mut game = headless_game();
        stage_on_street(&mut game);
        let view = view_for(&game, Proj::Iso);
        // Map corner: unreachable Rock. The walk must cancel itself on the
        // first blocked step instead of pinning against the wall — and it
        // must keep walking while steps DO land (no early bail-out).
        let goal = Pos::new(0, 0);
        let mut target = Some(AutowalkTarget::new(goal));
        let mut landed = 0;
        for _ in 0..64 {
            let before = game.player.pos;
            step_autowalk(&mut game, &mut target, &view, true, false);
            game.elapsed_ms += 100;
            if game.player.pos != before {
                landed += 1;
            }
            if target.is_none() {
                break;
            }
        }
        assert!(landed >= 2, "walked at least two free steps before the wall");
        assert!(target.is_none(), "blocked step cancelled the walk");
        assert_ne!(game.player.pos, goal, "never teleported into the rock");
    }

    #[test]
    fn clicks_do_not_leak_under_a_modal() {
        // Drives the real clicks system: modal_open in the snapshot, a left
        // press waiting — no autowalk target may be set, no actor inspected.
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<ButtonInput<MouseButton>>()
            .init_resource::<Touches>()
            .init_resource::<PointerIns>()
            .init_resource::<input::InputState>()
            .insert_resource(WorldView {
                modal_open: true,
                ..WorldView::default()
            })
            .insert_resource(SimSlot {
                game: headless_game(),
                frame: 0,
                sim_ms: 0.0,
                shot: None,
            })
            .insert_resource(Proj::Iso)
            .add_systems(Update, clicks);
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Left);
        app.update();
        let ins = app.world().resource::<PointerIns>();
        assert!(ins.target.is_none(), "no autowalk under a modal");
        assert!(ins.inspected.is_none(), "no inspection under a modal");
    }

    #[test]
    fn click_adjacent_portal_routes_through_interact_verb() {
        let mut game = headless_game();
        let portal = game.player.pos.offset(0, 1);
        let mut view = view_for(&game, Proj::Square);
        view.portals.push(portal);
        let mut ins = PointerIns::default();
        ins.target = Some(AutowalkTarget::new(portal.offset(2, 0))); // walking: click overrides
        let world = Proj::Square.world(portal, 0.0).truncate();
        let outcome = resolve_click(&mut game, &view, Proj::Square, &mut ins, portal, world);
        assert_eq!(outcome, ClickOutcome::Interacted);
        assert!(ins.target.is_none(), "the interact verb ends the walk");
    }

    #[test]
    fn click_actor_marks_inspection() {
        let mut game = click_game(Archetype::BroodHole, false);
        game.npcs[0].pos = game.player.pos.offset(3, 0);
        game.reveal();
        let view = view_for(&game, Proj::Square);
        let actor = view.actors[0].clone();
        let world = Proj::Square.world(actor.pos, 0.0).truncate();
        let mut ins = PointerIns::default();
        ins.target = Some(AutowalkTarget::new(Pos::new(9, 9)));
        let outcome = resolve_click(&mut game, &view, Proj::Square, &mut ins, actor.pos, world);
        assert_eq!(outcome, ClickOutcome::Inspected(actor.id));
        assert_eq!(ins.inspected, Some(actor.id));
        assert!(ins.target.is_none(), "inspecting stops the walk");
    }

    #[test]
    fn click_ground_starts_autowalk() {
        let mut game = headless_game();
        let view = view_for(&game, Proj::Square);
        let goal = Pos::new(-40, -40); // far from any actor
        let world = Proj::Square.world(goal, 0.0).truncate();
        let mut ins = PointerIns::default();
        let outcome = resolve_click(&mut game, &view, Proj::Square, &mut ins, goal, world);
        assert_eq!(outcome, ClickOutcome::Autowalk(goal));
        assert_eq!(ins.target.as_ref().map(|t| t.target), Some(goal));
    }

    fn click_game(kind: Archetype, combat: bool) -> Game {
        use laya_realms::model::{Map, MapKind};
        let mut game = headless_game();
        game.maps[0] = Map::new("Click arena", MapKind::Arena, 30, 30, Tile::Floor);
        game.player.map = 0;
        game.player.pos = Pos::new(10, 10);
        game.npcs = vec![laya_realms::world::make_npc(
            0, kind, 0, Pos::new(11, 10), 1, 1, 42,
        )];
        game.npcs[0].hp = 500;
        game.npcs[0].max_hp = 500;
        if combat {
            game.npcs[0].intent = Intent::Attack;
            game.refresh_combat();
        }
        game.reveal();
        game
    }

    #[test]
    fn overlapping_iso_picks_strike_the_nearest_actor_not_first_actor() {
        let mut game = click_game(Archetype::BroodHole, true);
        let mut second = laya_realms::world::make_npc(
            1, Archetype::BroodHole, 0, Pos::new(11, 11), 1, 1, 42,
        );
        second.hp = 500;
        second.max_hp = 500;
        second.intent = Intent::Attack;
        game.npcs.push(second);
        game.refresh_combat();
        game.reveal();
        let view = view_for(&game, Proj::Iso);
        let first = Proj::Iso.world(game.npcs[0].pos, 0.0).truncate();
        let second = Proj::Iso.world(game.npcs[1].pos, 0.0).truncate();
        let world = second.lerp(first, 0.4);
        let tile = pick_tile(Proj::Iso, world.x, world.y);
        let mut ins = PointerIns::default();
        let outcome = resolve_click(&mut game, &view, Proj::Iso, &mut ins, tile, world);
        assert_eq!(outcome, ClickOutcome::Struck(1));
        assert_eq!(game.npcs[0].hp, 500);
        assert!(game.npcs[1].hp < 500);
        assert_eq!(game.turn, 1);
        assert!(ins.target.is_none());
    }

    #[test]
    fn combat_ground_repositions_one_round_but_invalid_clicks_cost_nothing() {
        let mut game = click_game(Archetype::BroodHole, true);
        game.player.stamina = 3;
        game.player.defending = true;
        game.maps[0].set(Pos::new(9, 10), Tile::Wall);
        let view = view_for(&game, Proj::Square);
        let mut ins = PointerIns::default();
        for tile in [Pos::new(9, 10), Pos::new(9, 9), Pos::new(20, 20), Pos::new(10, 10)] {
            ins.target = Some(AutowalkTarget::new(Pos::new(20, 20)));
            let world = Proj::Square.world(tile, 0.0).truncate();
            assert_eq!(
                resolve_click(&mut game, &view, Proj::Square, &mut ins, tile, world),
                ClickOutcome::Ignored,
            );
            assert!(ins.target.is_none());
        }
        assert_eq!(game.turn, 0);
        assert_eq!(game.player.stamina, 3);
        assert!(game.player.defending);
        let tile = Pos::new(10, 11);
        let world = Proj::Square.world(tile, 0.0).truncate();
        assert_eq!(
            resolve_click(&mut game, &view, Proj::Square, &mut ins, tile, world),
            ClickOutcome::Moved(tile),
        );
        assert_eq!(game.player.pos, tile);
        assert_eq!(game.turn, 1);
        assert_eq!(game.player.stamina, 4);
        assert!(ins.target.is_none());
    }

    #[test]
    fn hidden_or_stale_actor_cannot_be_struck_by_a_pick() {
        let mut game = click_game(Archetype::BroodHole, true);
        let tile = game.npcs[0].pos;
        let world = Proj::Square.world(tile, 0.0).truncate();
        let mut view = view_for(&game, Proj::Square);
        let mut ins = PointerIns::default();
        let index = game.map().index(tile).unwrap();
        game.maps[0].explored[index] = false;
        assert_eq!(
            resolve_click(&mut game, &view, Proj::Square, &mut ins, tile, world),
            ClickOutcome::Ignored,
        );
        game.maps[0].explored[index] = true;
        view.actors[0].id = usize::MAX;
        assert_eq!(
            resolve_click(&mut game, &view, Proj::Square, &mut ins, tile, world),
            ClickOutcome::Ignored,
        );
        assert_eq!(game.npcs[0].hp, 500);
        assert_eq!(game.turn, 0);
    }

    #[test]
    fn nearby_friendly_click_opens_conversation_without_walking() {
        let mut game = click_game(Archetype::Commoner, false);
        let view = view_for(&game, Proj::Square);
        let tile = game.npcs[0].pos;
        let world = Proj::Square.world(tile, 0.0).truncate();
        let mut ins = PointerIns::default();
        let origin = game.player.pos;
        assert_eq!(
            resolve_click(&mut game, &view, Proj::Square, &mut ins, tile, world),
            ClickOutcome::Interacted,
        );
        assert!(matches!(game.modal, Modal::Talk(0)));
        assert_eq!(game.player.pos, origin);
        assert_eq!(game.turn, 0);
        assert!(ins.target.is_none());
    }
}
