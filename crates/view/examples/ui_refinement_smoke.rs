use bevy::prelude::*;
use laya_realms::model::{Game, Modal};
use realms_view::{camera::CamZoom, input, options::{self, ViewOptions}, sim::{ShotPlan, SimSlot}, launch_with, ViewConfig};

fn main() {
    let case = std::env::args().nth(1).unwrap_or_else(|| "settings".into());
    if case == "settings" {
        let mut game = Game::new(42);
        game.modal = Modal::None;
        let mut app = App::new();
        app.init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<input::InputState>()
            .init_resource::<ViewOptions>()
            .init_resource::<CamZoom>()
            .insert_resource(SimSlot { game, frame: 0, sim_ms: 0.0, shot: None })
            .add_systems(Update, (input::modal_keys, options::toggles).chain());
        app.world_mut().resource_mut::<ButtonInput<KeyCode>>().press(KeyCode::KeyB);
        app.update();
        let opts = app.world().resource::<ViewOptions>();
        println!("JOURNAL_STATE modal={:?} bloom={} vignette={} flicker={}",
            app.world().resource::<SimSlot>().game.modal, opts.bloom, opts.vignette, opts.flicker);
        assert!(matches!(app.world().resource::<SimSlot>().game.modal, Modal::Journal));
        assert!(opts.bloom, "opening the journal must not toggle bloom");
        return;
    }
    let mut game = Game::new(42);
    game.modal = Modal::None;
    let mut shot = ShotPlan::ui_gallery();
    shot.path = "docs/gfx/proto/interaction/small-last.png".into();
    for (_, name, path) in &mut shot.ui_cases {
        *path = format!("docs/gfx/proto/interaction/small-{name}.png").into();
    }
    launch_with(&mut game, ViewConfig { title: "UI refinement smoke".into(), shot: Some(shot), ..Default::default() });
    println!("SMALL_MENU_GALLERY_OK");
}
