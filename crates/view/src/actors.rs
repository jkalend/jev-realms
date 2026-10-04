//! Actor layer: live NPCs + the player, rendered from `actors.png` keyed by
//! `Archetype` / `Player.<Class>` Debug names (E4-s3). Missing keys fall back
//! to the palette quads from the E0 read. Overshoots deliberately simple:
//! one flash channel (hurt = white, gear bump = gold, boss phase = red),
//! visibility + light tint through the same sim light math as terrain.

use bevy::prelude::*;
use laya_realms::model::{Archetype, Build, Class, Pos, Tile};

use crate::atlas::{self, Atlas};
use crate::palette;
use crate::projection::{Proj, T};
use crate::sim::{ActorView, WorldView};

pub const PLAYER_ACTOR_ID: usize = usize::MAX;
/// The relic pendant quad that hangs over the player when one is equipped.
pub const PLAYER_RELIC_ACTOR_ID: usize = usize::MAX - 1;

/// Step smoothing duration: a grid hop glides over ~the sim tick period.
pub const SMOOTH_MS: f32 = 0.110;

#[derive(Component)]
pub struct Actor {
    id: usize,
    /// Last applied visual variant; rewrite image/rect only on change.
    tex: u8, // 0 unset, 1 texture, 2 quad
    /// Plate key currently on the sprite. The player swaps gear by changing its
    /// key, so a bare `tex == 1` would skip the repaint and the character would
    /// keep the old weapon.
    tex_key: Option<String>,
    prev_hp: i32,
    prev_gear: (u8, u8),
    prev_phase: u8,
    flash_until: f32,
    flash_color: Color,
    /// Last SNAPSHOT tile centre; flips advance the glide queue.
    current: Pos,
    /// (last tile centre, glide start secs) while a hop is mid-flight.
    lerp_from: Option<(Pos, f32)>,
    /// Lane currently playing, and its ping-pong frame (0/1). A lane change
    /// restarts at 0 so a strike begins on its first pose.
    action: &'static str,
    frame: u8,
    /// Seconds the current frame has been held, against `anim_last`.
    anim_last: f32,
    /// Facing index into `DIRECTIONS`; survives standing still.
    facing: u8,
    /// Seconds left on a hit reaction, so a one-frame hp drop still shows a
    /// readable recoil instead of a single-frame flicker.
    hit_until: f32,
    /// Seconds left on a strike lane. The sim's `Intent::Attack` lasts one beat
    /// (~85ms), far shorter than a readable swing, so the lane is held open
    /// after the trigger instead of sampled from the instantaneous intent.
    attack_until: f32,
    /// Frames the baked attack lane actually has for this direction (1-4),
    /// probed from the atlas. Short lanes (Bandit, Commoner) are single poses.
    attack_frames: u8,
}

fn archetype_key(a: Archetype) -> &'static str {
    match a {
        Archetype::Commoner => "Commoner",
        Archetype::Vendor => "Vendor",
        Archetype::Guard => "Guard",
        Archetype::Thief => "Thief",
        Archetype::Traveller => "Traveller",
        Archetype::Bandit => "Bandit",
        Archetype::Wolf => "Wolf",
        Archetype::Bear => "Bear",
        Archetype::Rat => "Rat",
        Archetype::Skeleton => "Skeleton",
        Archetype::Chief => "Chief",
        Archetype::Matriarch => "Matriarch",
        Archetype::Lich => "Lich",
        Archetype::Adjudicator => "Adjudicator",
        Archetype::Smuggler => "Smuggler",
        Archetype::Oracle => "Oracle",
        Archetype::Companion => "Companion",
        Archetype::Tidemother => "Tidemother",
        Archetype::Cragmother => "Cragmother",
        // E5/E7: painted plates landed (tools/atlas batch 3) — direct keys.
        Archetype::GnawThane => "GnawThane",
        Archetype::Tollmaster => "Tollmaster",
        Archetype::Mirelight => "Mirelight",
        Archetype::PaleStag => "PaleStag",
        Archetype::Alchemist => "Alchemist",
        // The maw ships on props.png; actor_ref resolves it there.
        Archetype::BroodHole => "BroodHole",
        Archetype::FalseGlow => "FalseGlow",
        // E7: violet-robed schism reader plate.
        Archetype::OathlessCurate => "OathlessCurate",
    }
}

/// Player plate key: `Player.<Class>.<Build>.W<weapon>A<armour>`.
///
/// Equipped gear is part of the plate, not an overlay, so changing
/// `player.weapon` or `player.armour` changes the sprite. The caller falls back
/// through the candidates in `player_plate_keys` when a cell is absent, which is
/// what keeps a save with an unbaked tier from losing its character.
fn class_name(c: Class) -> &'static str {
    match c {
        Class::Keepwarden => "Keepwarden",
        Class::Gravebound => "Gravebound",
        Class::Redwake => "Redwake",
        Class::Waysworn => "Waysworn",
        Class::SigilSworn => "SigilSworn",
        Class::Fensworn => "Fensworn",
        // Unsworn: no authored plate — quad until the class is sworn.
        Class::None => "__unsworn__",
    }
}

/// Key candidates for the player, best first: the exact equipped plate, then
/// the same weapon on the starting plate, then the same plate with the starting
/// weapon, then both starting, then the bare body. A save whose tier predates a
/// re-bake still draws its character instead of falling back to a quad.
pub(crate) fn player_plate_keys(c: Class, b: Build, weapon: u8, armour: u8) -> Vec<String> {
    if matches!(c, Class::None) {
        return vec!["__unsworn__".to_string()];
    }
    let w = weapon.min(3);
    let a = armour.min(3);
    let base = format!("Player.{}.{}", class_name(c), b.key());
    [format!("{base}.W{w}A{a}"), format!("{base}.W{w}A0"), format!("{base}.W0A{a}"), format!("{base}.W0A0"), base]
        .into_iter()
        .fold(Vec::new(), |mut acc: Vec<String>, key| {
            if !acc.contains(&key) {
                acc.push(key);
            }
            acc
        })
}

/// Overlay key for an equipped boss relic. The plate hangs at the chest socket,
/// so it is drawn on a second quad over the body rather than baked into the gear
/// matrix (which would be 12 x 16 x 9 plates).
pub(crate) fn relic_key(relic: laya_realms::model::BossRelic, build: Build) -> String {
    format!("Player.Relic.{}.{}", relic_name(relic), build.key())
}

fn relic_name(relic: laya_realms::model::BossRelic) -> &'static str {
    use laya_realms::model::BossRelic::*;
    match relic {
        Rallybreaker => "Rallybreaker",
        Fangmantle => "Fangmantle",
        Graveglass => "Graveglass",
        Saltcrown => "Saltcrown",
        Stoneheart => "Stoneheart",
        GnawboneCrown => "GnawboneCrown",
        TollcoinCharm => "TollcoinCharm",
        WisplightLantern => "WisplightLantern",
        Hartshorn => "Hartshorn",
    }
}

/// Fallback quad colors: hostiles red-family, civvies muted service hues
/// (the accent rule in docs/ART_PIPELINE.md), companion warm green.
fn fallback_rgb(a: Archetype) -> (u8, u8, u8) {
    if a.boss() {
        (214, 64, 60)
    } else if a.hostile() {
        (198, 96, 84)
    } else if a == Archetype::Companion {
        (118, 186, 140)
    } else {
        (156, 148, 128)
    }
}

/// Tiered light for an actor at `pos`: the same strength curve as the tile
/// under them; outside the vision radius they vanish (the D2 edge-of-dark read).
pub fn tier_tint(view: &WorldView, pos: Pos, tile: Tile) -> Color {
    let dist = pos.distance(view.player);
    let (strength, _remembered) = palette::tier(dist, view.radius, tile);
    palette::lit((255, 255, 255), strength, false)
}

fn cell_at(view: &WorldView, pos: Pos) -> Option<&crate::sim::Cell> {
    let col = pos.x - view.left;
    let row = pos.y - view.top;
    if col < 0 || row < 0 || col >= view.cols as i32 || row >= view.rows as i32 {
        return None;
    }
    view.cells.get((row as u32 * view.cols + col as u32) as usize)
}

fn spawn_scale(archetype: Option<Archetype>) -> f32 {
    match archetype {
        // Presence: bosses stand a tier above, and everyone is drawn a little
        // over one tile so a character's face and weapon read at play size.
        Some(a) if a.boss() => T * 1.75,
        _ => T * 1.3,
    }
}

pub fn render(
    view: Res<WorldView>,
    time: Res<Time>,
    proj: Res<Proj>,
    atlas: Option<Res<Atlas>>,
    images: Res<Assets<Image>>,
    mut commands: Commands,
    mut actors: Query<(Entity, &mut Actor, &mut Sprite, &mut Transform, &mut Visibility)>,
) {
    let now = time.elapsed_secs();
    // Reconcile: drop entities whose npc left the frame (dead, other map).
    for (entity, actor, ..) in actors.iter_mut() {
        // The relic quad only exists while a relic is carried.
        if actor.id == PLAYER_RELIC_ACTOR_ID && view.player_relic.is_none() {
            commands.entity(entity).despawn();
        }
        let alive = if actor.id == PLAYER_ACTOR_ID {
            view.player_hp > 0
        } else {
            view.actors.iter().any(|a| a.id == actor.id)
        };
        if !alive {
            commands.entity(entity).despawn();
        }
    }
    // The player figure: class-variant plate, gear flashes.
    upsert_actor(
        &mut commands,
        &mut actors,
        ActorSlab {
            id: PLAYER_ACTOR_ID,
            pos: view.player,
            key: String::new(),
            arch: None,
            attacking: false,
            alt_keys: player_plate_keys(
                view.player_class,
                view.player_build,
                view.player_weapon,
                view.player_armour,
            ),
            z: Proj::Z_ACTOR_OVERLAY,
            scale: T,
            hp: view.player_hp,
            gear: Some((view.player_weapon, view.player_armour)),
            band_phase: None,
            fallback: palette::FIGURE_GOLD,
        },
        &view,
        &proj,
        &atlas,
        &images,
        now,
    );
    // Equipped boss relic: a second quad on the same tile, a hair in front, that
    // only exists while a relic is carried. Its own id so the two sprites never
    // fight over the same slot.
    if let Some(relic) = view.player_relic {
        upsert_actor(
            &mut commands,
            &mut actors,
            ActorSlab {
                id: PLAYER_RELIC_ACTOR_ID,
                pos: view.player,
                key: String::new(),
                arch: None,
                attacking: false,
                alt_keys: vec![relic_key(relic, view.player_build)],
                z: Proj::Z_ACTOR_OVERLAY + 0.001,
                scale: T,
                hp: view.player_hp,
                gear: None,
                band_phase: None,
                fallback: palette::FIGURE_GOLD,
            },
            &view,
            &proj,
            &atlas,
            &images,
            now,
        );
    }
    for slab_view in &view.actors {
        let a: &ActorView = slab_view;
        upsert_actor(
            &mut commands,
            &mut actors,
            ActorSlab {
                id: a.id,
                pos: a.pos,
                key: archetype_key(a.archetype).to_string(),
                alt_keys: Vec::new(),
                arch: Some(archetype_key(a.archetype)),
                attacking: a.attacking,
                z: Proj::Z_ACTOR,
                scale: spawn_scale(Some(a.archetype)),
                hp: a.hp,
                gear: None,
                band_phase: Some(a.phase),
                fallback: fallback_rgb(a.archetype),
            },
            &view,
            &proj,
            &atlas,
            &images,
            now,
        );
    }
}

/// Direction names, in the order the baked actor sheets pack them. "front"
/// faces the camera, which is +y on a grid that runs down the screen, so a
/// step south reads as walking toward the player.
const DIRECTIONS: [&str; 8] = [
    "front", "front_right", "right", "back_right", "back", "back_left", "left", "front_left",
];

/// Facing index for a tile delta, or None when the actor did not move.
fn facing_from(dx: i32, dy: i32) -> Option<u8> {
    Some(match (dx.signum(), dy.signum()) {
        (0, 1) => 0,
        (1, 1) => 1,
        (1, 0) => 2,
        (1, -1) => 3,
        (0, -1) => 4,
        (-1, -1) => 5,
        (-1, 0) => 6,
        (-1, 1) => 7,
        _ => return None,
    })
}

/// Seconds one frame of a lane holds. Walking is brisker than idling, so a
/// patrol reads as purposeful rather than as a statue with a twitch. A strike
/// is brisker still — it is a swing, not a pose.
fn lane_period(action: &str) -> f32 {
    match action {
        "walk" => 0.17,
        "attack" => 0.11,
        _ => 0.45,
    }
}

/// How long a strike lane stays open once an attack intent is seen, so the
/// whole swing plays even though the intent itself lasts a single sim beat.
const ATTACK_LANE_SECS: f32 = 0.34;

struct ActorSlab {
    id: usize,
    pos: Pos,
    key: String,
    /// Fallback keys, best first. Empty for every actor but the player.
    alt_keys: Vec<String>,
    /// Set for NPCs, whose plate is keyed `<Arch>.<action>.<dir>.<frame>`.
    /// The player and its relic overlay keep the flat gear-key chain instead.
    arch: Option<&'static str>,
    /// The NPC's intent is an attack this frame: play the strike lane.
    attacking: bool,
    z: f32,
    scale: f32,
    hp: i32,
    gear: Option<(u8, u8)>,
    band_phase: Option<u8>,
    fallback: (u8, u8, u8),
}

/// The key chain to try for this frame: the animated NPC plate first, then
/// coarser fallbacks. A lane with fewer frames than we asked for still resolves
/// through `.000`, and the flat `<Arch>` alias is the last resort, so a
/// character is never dropped to a colour quad just because it was mid-step.
fn plate_keys(slab: &ActorSlab, t: &mut Actor, step: (i32, i32), now: f32) -> (String, Vec<String>) {
    let Some(arch) = slab.arch else {
        return (slab.key.clone(), slab.alt_keys.clone());
    };
    let (dx, dy) = step;
    if let Some(facing) = facing_from(dx, dy) {
        t.facing = facing;
    }
    if slab.hp < t.prev_hp {
        t.hit_until = now + 0.35;
    }
    // Rising edge: an attack intent opens the strike lane, and the lane then
    // holds itself open for the whole swing. Sampling `slab.attacking` every
    // frame only ever caught the one frame the intent happened to be set on,
    // which is why every blow in game read as a static pose.
    if slab.attacking && now >= t.attack_until {
        t.attack_until = now + ATTACK_LANE_SECS;
    }
    let striking = now < t.attack_until;
    let action = if striking {
        "attack"
    } else if dx != 0 || dy != 0 {
        "walk"
    } else if now < t.hit_until {
        "hit"
    } else {
        "idle"
    };
    if action != t.action {
        t.action = action;
        t.frame = 0;
        t.anim_last = now;
    } else if now - t.anim_last >= lane_period(action) {
        t.anim_last = now;
        // Strike lanes step forward through every baked frame and hold the
        // last one; idle and walk keep their two-frame ping-pong.
        t.frame = if action == "attack" {
            let last = t.attack_frames.saturating_sub(1);
            if t.frame < last { t.frame + 1 } else { t.frame }
        } else {
            (t.frame + 1) % 2
        };
    }
    let dir = DIRECTIONS[t.facing as usize];
    // A one-frame lane (Bandit, Commoner) must not ask for a frame it never baked.
    let frame = t.frame.min(t.attack_frames.saturating_sub(1));
    (
        format!("{arch}.{action}.{dir}.{frame:03}"),
        vec![
            format!("{arch}.{action}.{dir}.000"),
            arch.to_string(),
        ],
    )
}

/// Frame count of the baked attack lane for this archetype and direction.
/// Probed, not assumed: the roster ships between one and four strike frames.
fn attack_lane_len(
    arch: &str,
    dir: &str,
    atlas: &Option<Res<Atlas>>,
    images: &Assets<Image>,
) -> u8 {
    let count = atlas
        .as_deref()
        .map(|a| {
            (0..4)
                .take_while(|frame| {
                    atlas::actor_ref(a, images, &format!("{arch}.attack.{dir}.{frame:03}")).is_some()
                })
                .count()
        })
        .unwrap_or(1);
    count.max(1) as u8
}

fn upsert_actor(
    commands: &mut Commands,
    actors: &mut Query<(Entity, &mut Actor, &mut Sprite, &mut Transform, &mut Visibility)>,
    slab: ActorSlab,
    view: &WorldView,
    proj: &Proj,
    atlas: &Option<Res<Atlas>>,
    images: &Assets<Image>,
    now: f32,
) {
    // Fog rule: unexplored cells are dark — actors don't show through them.
    let cell = cell_at(view, slab.pos);
    let dist = slab.pos.distance(view.player);
    let visible = slab.id == PLAYER_ACTOR_ID
        || (dist <= view.radius && cell.is_some_and(|c| c.explored));
    let tile = cell.map(|c| c.tile).unwrap_or(Tile::Grass);
    let stale = |t: &mut Actor, s: &mut Sprite, tf: &mut Transform, vis: &mut Visibility| {
        // Sampled before the glide update rewrites `t.current`, or every actor
        // would read as standing still.
        let step = (slab.pos.x - t.current.x, slab.pos.y - t.current.y);
        // flashes: hurt (hp drop) → white; gear bump → gold; boss phase → red
        if slab.hp < t.prev_hp {
            t.flash_until = now + 0.3;
            t.flash_color = Color::srgb(2.6, 2.6, 2.8);
        } else if slab.gear.is_some_and(|g| g > t.prev_gear) {
            t.flash_until = now + 0.3;
            t.flash_color = Color::srgb(2.6, 2.2, 1.2);
        } else if slab.band_phase.is_some_and(|p| p != t.prev_phase) && t.prev_hp > 0 {
            t.flash_until = now + 0.4;
            t.flash_color = Color::srgb(2.4, 0.9, 0.7);
        }
        t.prev_hp = slab.hp;
        if let Some(g) = slab.gear {
            t.prev_gear = g;
        }
        if let Some(p) = slab.band_phase {
            t.prev_phase = p;
        }
        // Probe the strike lane once per actor: the roster ships 1-4 frames per
        // archetype, and a lane shorter than the one being asked for falls back
        // to its own `.000` rather than dropping the character to a quad.
        if t.attack_frames == 0 {
            if let Some(arch) = slab.arch {
                let dir = DIRECTIONS[t.facing as usize];
                t.attack_frames = attack_lane_len(arch, dir, atlas, images);
            }
        }
        if !visible {
            *vis = Visibility::Hidden;
            return;
        }
        *vis = Visibility::Visible;
        // Step smoothing: a snapshot tile change kicks a glide from the old
        // centre; mid-flight position eases quadratic, landing is EXACT.
        if slab.pos != t.current {
            t.lerp_from = Some((t.current, now));
            t.current = slab.pos;
        }
        let z = match proj {
            Proj::Iso => slab.z + Proj::tile_band(slab.pos),
            Proj::Square => slab.z,
        };
        let target = proj.world(t.current, z);
        tf.translation = match t.lerp_from {
            Some((from, t0)) => {
                let k = ((now - t0) / SMOOTH_MS).clamp(0.0, 1.0);
                let k = 1.0 - (1.0 - k) * (1.0 - k); // ease-out quadratic
                if k >= 1.0 {
                    t.lerp_from = None;
                    target
                } else {
                    let start = proj.world(from, z);
                    start + (target - start) * k
                }
            }
            None => target,
        };
        let tier = tier_tint(view, slab.pos, tile);
        s.color = if now < t.flash_until {
            t.flash_color
        } else {
            match t.tex {
                1 => tier,
                _ => palette::lit(
                    slab.fallback,
                    if dist * 4 <= view.radius * 3 { 1.0 } else { 0.55 },
                    false,
                ),
            }
        };
        let (primary, rest) = plate_keys(&slab, t, step, now);
        let mut tex_key = primary.clone();
        let tex = atlas.as_deref().and_then(|a| {
            for key in std::iter::once(&primary).chain(rest.iter()) {
                if let Some(found) = atlas::actor_ref(a, images, key) {
                    tex_key = key.clone();
                    return Some(found);
                }
            }
            None
        });
        match (tex, t.tex) {
            (Some((image, rect)), current) if current != 1 || t.tex_key.as_deref() != Some(tex_key.as_str()) => {
                t.tex = 1;
                t.tex_key = Some(tex_key);
                s.image = image.clone();
                s.rect = Some(rect);
                s.custom_size = Some(Vec2::splat(slab.scale));
                s.color = if now < t.flash_until {
                    t.flash_color
                } else {
                    tier
                };
            }
            (None, current) if current != 2 => {
                t.tex = 2;
                s.image = Handle::default();
                s.rect = None;
                s.custom_size = Some(Vec2::splat(T * 0.8));
            }
            _ => {}
        }
    };
    if let Some((_, mut tag, mut sprite, mut transform, mut visibility)) =
        actors.iter_mut().find(|(_, a, ..)| a.id == slab.id)
    {
        stale(&mut tag, &mut sprite, &mut transform, &mut visibility);
        return;
    }
    commands.spawn((
        Sprite::from_color(Color::NONE, Vec2::splat(T * 0.8)),
        Transform::from_xyz(0.0, -100_000.0, slab.z),
        Visibility::Hidden,
        Actor {
            id: slab.id,
            tex: 0,
            tex_key: None,
            prev_hp: slab.hp,
            prev_gear: slab.gear.unwrap_or((0, 0)),
            prev_phase: 0,
            flash_until: 0.0,
            flash_color: Color::WHITE,
            current: slab.pos,
            lerp_from: None,
            action: "idle",
            frame: 0,
            anim_last: now,
            facing: 0,
            hit_until: 0.0,
            attack_until: 0.0,
            attack_frames: 0,
        },
    ));
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The player plate key carries class, build AND equipped gear, so equipping
    /// a different blade has to produce a different key - that is the whole
    /// mechanism behind "the player shows their gear".
    #[test]
    fn player_plate_key_tracks_equipped_gear() {
        let base = player_plate_keys(Class::Keepwarden, Build::Male, 0, 0);
        let armed = player_plate_keys(Class::Keepwarden, Build::Male, 3, 2);
        assert_eq!(base[0], "Player.Keepwarden.Male.W0A0");
        assert_eq!(armed[0], "Player.Keepwarden.Male.W3A2");
        assert_ne!(base[0], armed[0], "changing gear must change the plate");
    }

    /// A tier beyond the baked matrix, or a save written before a re-bake, must
    /// still resolve to a character instead of falling through to a flat quad.
    #[test]
    fn player_plate_keys_degrade_instead_of_disappearing() {
        let keys = player_plate_keys(Class::Redwake, Build::Female, 7, 9);
        assert_eq!(keys[0], "Player.Redwake.Female.W3A3", "tiers clamp to the baked matrix");
        assert!(keys.contains(&"Player.Redwake.Female".to_string()), "bare body is the last resort");
        assert!(keys.last().is_some_and(|k| k == "Player.Redwake.Female"));
    }

    /// Every candidate must be distinct; a duplicate would resolve the wrong cell.
    #[test]
    fn player_plate_keys_are_distinct_and_start_with_the_exact_gear() {
        for weapon in 0..4u8 {
            for armour in 0..4u8 {
                let keys = player_plate_keys(Class::Fensworn, Build::Male, weapon, armour);
                let mut sorted = keys.clone();
                sorted.sort();
                sorted.dedup();
                assert_eq!(sorted.len(), keys.len(), "duplicate key in {keys:?}");
                assert_eq!(keys[0], format!("Player.Fensworn.Male.W{weapon}A{armour}"));
            }
        }
    }

    #[test]
    fn unsworn_has_no_plate() {
        assert_eq!(player_plate_keys(Class::None, Build::Male, 0, 0), vec!["__unsworn__".to_string()]);
    }

    /// A baked multi-frame strike has to actually play: the lane opens on the
    /// attack intent, steps through every frame the atlas has, and holds the
    /// last one. Pinning attack to frame 0 is what made every blow a still pose.
    #[test]
    fn strike_lane_plays_every_baked_frame_then_holds() {
        let mut slab = ActorSlab {
            id: 1,
            pos: Pos::new(4, 4),
            key: String::new(),
            arch: Some("Chief"),
            attacking: true,
            alt_keys: Vec::new(),
            z: Proj::Z_ACTOR,
            scale: T,
            hp: 10,
            gear: None,
            band_phase: None,
            fallback: (200, 200, 200),
        };
        let mut actor = Actor {
            id: 1,
            tex: 0,
            tex_key: None,
            prev_hp: 10,
            prev_gear: (0, 0),
            prev_phase: 0,
            flash_until: 0.0,
            flash_color: Color::WHITE,
            current: Pos::new(4, 4),
            lerp_from: None,
            action: "idle",
            frame: 0,
            anim_last: 0.0,
            facing: 0,
            hit_until: 0.0,
            attack_until: 0.0,
            attack_frames: 3,
        };
        // t = 0: the intent opens the lane on frame 0.
        let (first, _) = plate_keys(&slab, &mut actor, (0, 0), 0.0);
        assert_eq!(first, "Chief.attack.front.000");
        // One lane period later it is on frame 1, then frame 2.
        let t1 = lane_period("attack");
        let (second, _) = plate_keys(&slab, &mut actor, (0, 0), t1);
        assert_eq!(second, "Chief.attack.front.001");
        let (third, _) = plate_keys(&slab, &mut actor, (0, 0), t1 * 2.0);
        assert_eq!(third, "Chief.attack.front.002");
        // Past the last baked frame it holds, it does not fall back to 0.
        let (held, _) = plate_keys(&slab, &mut actor, (0, 0), t1 * 3.0);
        assert_eq!(held, "Chief.attack.front.002", "the swing must not restart");
        // The lane outlives the instantaneous intent, so a one-beat attack
        // still shows its whole swing.
        assert!(actor.attack_until > t1 * 2.0);
        // And it closes: after the window the actor is idling again.
        slab.attacking = false;
        let after = actor.attack_until + 0.01;
        let (idle, _) = plate_keys(&slab, &mut actor, (0, 0), after);
        assert_eq!(idle, "Chief.idle.front.000");
    }

    /// A one-frame lane (Bandit, Commoner) must never ask for a frame that was
    /// not baked; the key chain falls back to its own `.000` instead.
    #[test]
    fn single_frame_strike_stays_on_frame_zero() {
        let slab = ActorSlab {
            id: 2,
            pos: Pos::new(1, 1),
            key: String::new(),
            arch: Some("Bandit"),
            attacking: true,
            alt_keys: Vec::new(),
            z: Proj::Z_ACTOR,
            scale: T,
            hp: 5,
            gear: None,
            band_phase: None,
            fallback: (200, 200, 200),
        };
        let mut actor = Actor {
            id: 2,
            tex: 0,
            tex_key: None,
            prev_hp: 5,
            prev_gear: (0, 0),
            prev_phase: 0,
            flash_until: 0.0,
            flash_color: Color::WHITE,
            current: Pos::new(1, 1),
            lerp_from: None,
            action: "idle",
            frame: 2,
            anim_last: 0.0,
            facing: 0,
            hit_until: 0.0,
            attack_until: 0.0,
            attack_frames: 1,
        };
        let (key, _) = plate_keys(&slab, &mut actor, (0, 0), 1.0);
        assert_eq!(key, "Bandit.attack.front.000");
    }
 }
