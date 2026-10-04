//! PROTOTYPE driver — renders design-evidence PNG sheets (docs/gfx/proto/).
//! Usage: cargo run --example gfxlab

use macroquad::prelude::*;

fn conf() -> Conf {
    Conf {
        window_title: "gfxlab prototype".into(),
        window_width: laya_realms::gfxlab::SHEET_W as i32,
        window_height: laya_realms::gfxlab::SHEET_H as i32,
        window_resizable: false,
        ..Default::default()
    }
}

#[macroquad::main(conf)]
async fn main() {
    laya_realms::gfxlab::projection_sheets("docs/gfx/proto").await;
    laya_realms::gfxlab::artstyle_sheets("docs/gfx/proto").await;
    laya_realms::gfxlab::concept_cards("docs/gfx/proto").await;
}
