//! Production Bevy view for Laya Realms (docs/D2_EVOLUTION.md §7.4/E4).
//! The simulation never sees this crate: read-only snapshots in, Action verbs out.
//!
//! Architecture: `SimSlot` owns the live `Game`; `sim::sync` copies a per-frame
//! `WorldView` and render systems (tiles/lights/camera/effects) read only that.
//! `input` + `sim::clock` are the only systems that mutate the Game.

pub mod actors;
pub mod atlas;
pub mod backdrop;
pub mod camera;
mod combat_controls;
mod dialogue;
pub mod effects;
pub mod hud;
pub mod input;
pub mod lights;
pub mod modal_details;
pub mod modals;
pub mod nameplate;
pub mod options;
pub mod palette;
pub mod pointer;
pub mod projection;
pub mod props;
pub mod sim;
mod talents;
pub mod tiles;

use bevy::app::AppExit;
use bevy::post_process::bloom::Bloom;
use bevy::post_process::effect_stack::Vignette;
use bevy::prelude::*;
use bevy_light_2d::prelude::*;
use laya_realms::model::Game;

use sim::{ShotPlan, SimSlot, WorldView};

/// Dev-runner configuration for `launch_with`. Interactive runs use `launch`.
pub struct ViewConfig {
    pub title: String,
    pub shot: Option<ShotPlan>,
    pub projection: projection::Proj,
}

impl Default for ViewConfig {
    fn default() -> Self {
        Self {
            title: "Laya Realms".into(),
            shot: None,
            // Owner-ratified default since the iso evidence landed (docs/VISUAL_OPTIONS.md).
            projection: projection::Proj::Iso,
        }
    }
}

/// Run the view on a live game. The Game is borrowed for the app's lifetime and
/// handed back on exit, so callers keep full ownership of their sim state.
pub fn launch(game: &mut Game) {
    launch_with(game, ViewConfig::default());
}

pub fn launch_with(game: &mut Game, config: ViewConfig) {
    ensure_asset_root();
    // Placeholder swap: Game has no Default; seed 0 builds tiny enough to be
    // a cheap husk, and the real game is written back after `App::run` returns.
    let owned = std::mem::replace(game, Game::new(0));
    let mut app = App::new();
    // App::run swaps the whole App out of `self`; the world (and SimSlot) is
    // gone afterwards. Progress flows home through this handoff instead: the
    // exit paths (dev beats, Esc-quit, window close) leave the Game in it.
    let handoff = ExitHandoff(std::sync::Arc::new(parking_lot::Mutex::new(None)));
    let handoff_reader = handoff.clone();
    // Runtime display preferences (saves/view-options.json); the saved zoom
    // seeds the tactical camera so both survive reboots.
    let view_options = options::ViewOptions::load();
    let seeded_zoom = camera::CamZoom {
        target: view_options.zoom,
    };
    app.add_plugins((
        DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: config.title,
                resolution: (1280, 720).into(),
                ..default()
            }),
            ..default()
        }),
        Light2dPlugin,
    ))
    .insert_resource(ClearColor(palette::srgb8(palette::INK_BLACK)))
    .insert_resource(handoff)
    .insert_resource(view_options)
    .insert_resource(config.projection)
    .insert_resource(SimSlot {
        game: owned,
        frame: 0,
        sim_ms: 0.0,
        shot: config.shot,
    })
    .init_resource::<WorldView>()
    .init_resource::<input::InputState>()
    .init_resource::<pointer::PointerIns>()
    .init_resource::<dialogue::ConversationView>()
    .init_resource::<modals::ModalInteraction>()
    .insert_resource(seeded_zoom)
    .init_resource::<camera::CamFrame>()
    .init_resource::<lights::TorchState>()
    .init_resource::<tiles::TileGrid>()

    .add_systems(Startup, (atlas::setup, setup_camera, backdrop::setup, lights::setup, hud::setup))
    .add_systems(
        Update,
        (
            input::modal_keys,
            input::verbs,
            input::movement,
            pointer::clicks,
            modals::clicks,
            modal_details::clicks,
            dialogue::clicks,
            talents::clicks,
            pointer::autowalk,
            sim::clock,
            sim::dev_sequence,
            sim::sync,
        )
            .chain(),
    )
    // The modal pane mirrors this frame's game state (and the frame's keys).
    .add_systems(
        Update,
        (modals::render, modal_details::render.after(modals::render), dialogue::render.after(modals::render), talents::render.after(modals::render))
            .after(sim::sync).after(input::modal_keys),
    )
    // Edge-pan / drag writes PointerIns.pan for camera::follow to fold in.
    .add_systems(
        Update,
        pointer::pan.after(sim::sync).before(camera::follow),
    )
    .add_systems(
        Update,
        (tiles::ensure, tiles::render).chain().after(sim::sync),
    )
    .add_systems(
        Update,
        (
            camera::follow,
            backdrop::update.after(camera::follow),
            lights::update,
            hud::update_fills,
            hud::update_labels,
            hud::update_wayfinder,
            hud::combat_bubble,
            combat_controls::render,
            combat_controls::feedback,
            actors::render,
            props::render,
            nameplate::render,
            nameplate::inspect,
            effects::render,
        )
            .after(sim::sync),
    )
    .add_systems(Update, options::toggles)
    .add_systems(Update, (exit_on_quit, exit_on_window_close));
    app.run();
    let recovered = handoff_reader.0.lock().take();
    if let Some(g) = recovered {
        *game = g;
    }
}

/// Where the Game is parked when the app exits (App::run consumes the world).
#[derive(Resource, Clone)]
pub struct ExitHandoff(pub std::sync::Arc<parking_lot::Mutex<Option<Game>>>);

/// The repo keeps `assets/` at the workspace root, but bevy anchors its asset
/// root to the crate manifest dir (crates/view under `cargo run -p`). Point it
/// at the workspace unless the caller overrode it.
fn ensure_asset_root() {
    if std::env::var_os("BEVY_ASSET_ROOT").is_some() {
        return;
    }
    let workspace = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .map(std::path::Path::to_path_buf);
    if let Some(root) = workspace {
        if root.join("assets").is_dir() {
            std::env::set_var("BEVY_ASSET_ROOT", root);
        }
    }
}

fn setup_camera(mut commands: Commands, view_options: Res<options::ViewOptions>) {
    let mut ortho = OrthographicProjection::default_2d();
    ortho.scale = projection::T / camera::ZOOM_DEFAULT;
    commands.spawn((
        Camera2d,
        Projection::Orthographic(ortho),
        // (No bevy_ui camera here: the Light2d composite suppresses the UI
        // pass — chrome is world-space, see hud.rs/modals.rs.)
        // Post stack: the E0-approved frame (§7.5★) until E4-s4 grades it;
        // intensity gates come from persisted runtime options.
        Bloom {
            intensity: options::bloom_intensity(view_options.bloom),
            low_frequency_boost: 0.15,
            ..Bloom::NATURAL
        },
        Vignette {
            intensity: 0.35,
            radius: 0.55,
            ..default()
        },
        // Camera-scoped ambient; lights::update retunes it for day/night.
        Light2d {
            ambient_light: AmbientLight2d {
                color: Color::srgb(1.0, 1.0, 1.0),
                brightness: 0.85,
            },
        },
        camera::MainCam,
    ));
}

/// Park the live Game when the sim asks to quit (Esc → Pause → Q once E6
/// brings modals; `game.quit` today).
fn exit_on_quit(
    mut slot: ResMut<SimSlot>,
    handoff: Res<ExitHandoff>,
    mut exit: MessageWriter<AppExit>,
) {
    if slot.game.quit {
        *handoff.0.lock() = Some(std::mem::replace(&mut slot.game, Game::new(0)));
        exit.write(AppExit::Success);
    }
}

/// Same handoff when the window is closed from the title bar. Consumes the
/// close request so progress is never lost to an OS close.
fn exit_on_window_close(
    mut slot: ResMut<SimSlot>,
    handoff: Res<ExitHandoff>,
    mut close: MessageReader<bevy::window::WindowCloseRequested>,
    mut exit: MessageWriter<AppExit>,
) {
    for _ in close.read() {
        *handoff.0.lock() = Some(std::mem::replace(&mut slot.game, Game::new(0)));
        exit.write(AppExit::Success);
    }
}
