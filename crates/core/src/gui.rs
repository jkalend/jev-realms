//! Windowed presentation: native sprites, shared HUD/menu content and controls.
use crate::{
    input,
    laya_client::{self, AiService},
    model::{Game, Modal},
    sprites, ui,
};
use crossterm::event::{KeyCode as Key, KeyEvent, KeyModifiers};
use macroquad::prelude::*;
use ratatui::{
    backend::TestBackend,
    buffer::Buffer,
    layout::Rect as Cells,
    style::{Color as Ink, Modifier, Style},
    widgets::Paragraph,
    Terminal,
};
use std::{
    path::Path,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

const TILE_SIZES: [f32; 6] = [16.0, 24.0, 32.0, 40.0, 48.0, 64.0];

struct DisplaySettings {
    zoom: usize,
    ui_scale: f32,
    fullscreen: bool,
    windowed_size: Vec2,
    restore_window: bool,
}

impl DisplaySettings {
    fn new() -> Self {
        Self {
            zoom: 2,
            ui_scale: 1.0,
            fullscreen: false,
            windowed_size: vec2(screen_width(), screen_height()),
            restore_window: false,
        }
    }

    // Display keys never enter input::key or alter Game/save state.
    fn keyboard(&mut self) -> bool {
        if self.restore_window {
            resize_window(self.windowed_size);
            self.restore_window = false;
        }
        let control = is_key_down(KeyCode::LeftControl) || is_key_down(KeyCode::RightControl);
        let increase = is_key_pressed(KeyCode::Equal) || is_key_pressed(KeyCode::KpAdd);
        let decrease = is_key_pressed(KeyCode::Minus) || is_key_pressed(KeyCode::KpSubtract);
        let reset = control && (is_key_pressed(KeyCode::Key0) || is_key_pressed(KeyCode::Kp0));
        let fullscreen = is_key_pressed(KeyCode::F11);
        if reset {
            self.zoom = 2;
            self.ui_scale = 1.0;
        } else if increase || decrease {
            if control {
                self.ui_scale =
                    (self.ui_scale + if increase { 0.125 } else { -0.125 }).clamp(0.75, 2.0);
            } else if increase {
                self.zoom = (self.zoom + 1).min(TILE_SIZES.len() - 1);
            } else {
                self.zoom = self.zoom.saturating_sub(1);
            }
        }
        if fullscreen {
            if !self.fullscreen {
                self.windowed_size = vec2(screen_width(), screen_height());
            }
            self.fullscreen = !self.fullscreen;
            set_fullscreen(self.fullscreen);
            // Miniquad processes fullscreen asynchronously. Restore only after it
            // has reinstated the window decorations on the following frame.
            self.restore_window = !self.fullscreen;
        }
        increase || decrease || reset || fullscreen
    }
}

fn resize_window(size: Vec2) {
    #[cfg(target_os = "windows")]
    if desktop::resize_to_fit(size) {
        return;
    }
    request_new_screen_size(size.x, size.y);
}

// Match the OS title bar to the app palette; a DWM-only concern for now.
fn style_titlebar() {
    #[cfg(target_os = "windows")]
    desktop::style_caption(color(Ink::Black), color(Ink::White));
}

// Miniquad exposes no desktop work area. Keep this small Win32 bridge local:
// use physical work/client rectangles, then account for Macroquad's logical DPI.
#[cfg(target_os = "windows")]
mod desktop {
    use super::*;
    use std::ffi::c_void;

    #[repr(C)]
    #[derive(Default)]
    struct NativeRect {
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
    }

    #[repr(C)]
    struct MonitorInfo {
        size: u32,
        monitor: NativeRect,
        work: NativeRect,
        flags: u32,
    }

    #[link(name = "user32")]
    extern "system" {
        fn GetActiveWindow() -> *mut c_void;
        fn GetWindowRect(window: *mut c_void, rect: *mut NativeRect) -> i32;
        fn GetClientRect(window: *mut c_void, rect: *mut NativeRect) -> i32;
        fn MonitorFromWindow(window: *mut c_void, flags: u32) -> *mut c_void;
        fn GetMonitorInfoW(monitor: *mut c_void, info: *mut MonitorInfo) -> i32;
        fn SetWindowPos(
            window: *mut c_void,
            after: *mut c_void,
            x: i32,
            y: i32,
            width: i32,
            height: i32,
            flags: u32,
        ) -> i32;
    }

    pub(super) fn resize_to_fit(size: Vec2) -> bool {
        // SAFETY: called on Macroquad's window thread; structs match Win32 ABI,
        // pointers remain valid for each synchronous call, and failures fall back.
        unsafe {
            let window = GetActiveWindow();
            if window.is_null() {
                return false;
            }
            let mut outer = NativeRect::default();
            let mut client = NativeRect::default();
            let mut info = MonitorInfo {
                size: std::mem::size_of::<MonitorInfo>() as u32,
                monitor: NativeRect::default(),
                work: NativeRect::default(),
                flags: 0,
            };
            if GetWindowRect(window, &mut outer) == 0
                || GetClientRect(window, &mut client) == 0
                || GetMonitorInfoW(MonitorFromWindow(window, 2), &mut info) == 0
            {
                return false;
            }
            let border_x = outer.right - outer.left - (client.right - client.left);
            let border_y = outer.bottom - outer.top - (client.bottom - client.top);
            let work_width = info.work.right - info.work.left;
            let work_height = info.work.bottom - info.work.top;
            let width = ((size.x * screen_dpi_scale()).round() as i32 + border_x)
                .clamp(1, work_width.max(1));
            let height = ((size.y * screen_dpi_scale()).round() as i32 + border_y)
                .clamp(1, work_height.max(1));
            SetWindowPos(
                window,
                std::ptr::null_mut(),
                info.work.left + (work_width - width) / 2,
                info.work.top + (work_height - height) / 2,
                width,
                height,
                0x0004 | 0x0010, // SWP_NOZORDER | SWP_NOACTIVATE
            ) != 0
        }
    }

    #[link(name = "dwmapi")]
    extern "system" {
        fn DwmSetWindowAttribute(
            window: *mut c_void,
            attribute: u32,
            value: *const u32,
            size: u32,
        ) -> i32;
    }

    // Another Miniquad gap: theme the native caption to the dark palette.
    // Dark mode is attribute 20 from Win10 1903 (19 on 1809); exact caption
    // colors require Win11 22000+. Unknown attributes fail harmlessly there.
    pub(super) fn style_caption(background: Color, text: Color) {
        const DWMWA_DARK_1809: u32 = 19;
        const DWMWA_DARK_MODE: u32 = 20;
        const DWMWA_BORDER_COLOR: u32 = 34;
        const DWMWA_CAPTION_COLOR: u32 = 35;
        const DWMWA_TEXT_COLOR: u32 = 36;
        fn set(window: *mut c_void, attribute: u32, value: u32) -> bool {
            // SAFETY: window is the live Macroquad HWND; value outlives the
            // synchronous DWM call, and failure leaves defaults untouched.
            unsafe { DwmSetWindowAttribute(window, attribute, &value, 4) == 0 }
        }
        let colorref = |c: Color| {
            let channel = |v: f32| (v * 255.0).round() as u32;
            (channel(c.b) << 16) | (channel(c.g) << 8) | channel(c.r)
        };
        // SAFETY: GetActiveWindow only reads this thread's active window.
        unsafe {
            let window = GetActiveWindow();
            if window.is_null() {
                return;
            }
            if !set(window, DWMWA_DARK_MODE, 1) {
                set(window, DWMWA_DARK_1809, 1);
            }
            let caption = colorref(background);
            set(window, DWMWA_CAPTION_COLOR, caption);
            set(window, DWMWA_BORDER_COLOR, caption);
            set(window, DWMWA_TEXT_COLOR, colorref(text));
        }
    }
}
/// Runs on the main/window thread, within the caller's live Tokio runtime.
/// Tokio workers service AI requests while Macroquad owns presentation timing.
pub fn launch(
    seed: u64,
    mode: &str,
    key: Option<String>,
    missing: bool,
    screenshot: Option<String>,
) -> Result<(), String> {
    let ai = AiService::new(mode, key, seed)?;
    let mut game = Game::new(seed);
    game.ai.provider = mode.into();
    game.ai.switched = missing;
    if missing {
        game.log("Laya gateway not reachable; using offline decisions.");
    }
    if let Some(path) = screenshot.as_deref() {
        if let Some(parent) = Path::new(path)
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
        {
            std::fs::create_dir_all(parent).map_err(|e| format!("screenshot directory: {e}"))?;
        }
        // Fail before opening the window if the requested destination is unwritable.
        std::fs::File::create(path).map_err(|e| format!("screenshot destination: {e}"))?;
        game.modal = Modal::None;
    }
    let result = Arc::new(Mutex::new(Ok(())));
    let outcome = result.clone();
    macroquad::Window::from_config(
        Conf {
            window_title: "Laya Realms — Sprite View".into(),
            // Bootstrap conservatively; fit the logical target once native DPI
            // and monitor work area are available on the window thread.
            window_width: 960,
            window_height: 600,
            window_resizable: true,
            high_dpi: true,
            ..Default::default()
        },
        async move {
            let value = run(game, ai, screenshot).await;
            *outcome.lock().expect("window result lock") = value;
        },
    );
    let value = result.lock().map_err(|e| e.to_string())?.clone();
    value
}

async fn run(mut game: Game, mut ai: AiService, screenshot: Option<String>) -> Result<(), String> {
    resize_window(vec2(1440.0, 900.0));
    style_titlebar();
    next_frame().await;
    let mut display = DisplaySettings::new();
    let mut scene = Terminal::new(TestBackend::new(116, 44)).map_err(|e| e.to_string())?;
    let mut menus = Terminal::new(TestBackend::new(116, 44)).map_err(|e| e.to_string())?;
    let mut last = Instant::now();
    let mut simulation = Duration::ZERO;
    let mut elapsed = Duration::ZERO;
    let mut repeat_at = 0.0;
    let mut buffered_move = None;
    let mut held_move = None;
    let mut frames = 0;
    let mut actors = sprites::ActorAnimations::default();
    while !game.quit {
        let now = Instant::now();
        let delta = now.duration_since(last).min(Duration::from_millis(100));
        last = now;
        if input::active(&game) {
            elapsed += delta;
            game.elapsed_ms = elapsed.as_millis() as u64;
            simulation += delta;
            while simulation >= Duration::from_millis(250) {
                game.tick_world();
                simulation -= Duration::from_millis(250);
                if !input::active(&game) {
                    simulation = Duration::ZERO;
                    break;
                }
            }
        } else {
            simulation = Duration::ZERO;
        }
        laya_client::pump(&mut game, &mut ai);
        if display.keyboard() {
            buffered_move = None;
            held_move = None;
        } else {
            keyboard(&mut game, &mut repeat_at, &mut buffered_move, &mut held_move);
        }
        render(&game, &mut scene, &mut menus, &mut actors, &display)?;
        if frames == 0 {
            println!("GUI READY: seed={} view=gui", game.seed);
        }
        if frames == 4 {
            if let Some(path) = screenshot.as_deref() {
                let image = get_screen_data();
                image.export_png(path);
                println!(
                    "GUI SCREENSHOT OK: {path} ({}x{})",
                    image.width, image.height
                );
                return Ok(());
            }
        }
        next_frame().await;
        frames += 1;
    }
    Ok(())
}

// Translate hardware keys only. Modal behavior and actions remain in input::key.
const KEYS: &[(KeyCode, Key)] = &[
    (KeyCode::Escape, Key::Esc),
    (KeyCode::Enter, Key::Enter),
    (KeyCode::KpEnter, Key::Enter),
    (KeyCode::Up, Key::Up),
    (KeyCode::Down, Key::Down),
    (KeyCode::Left, Key::Left),
    (KeyCode::Right, Key::Right),
    (KeyCode::Home, Key::Home),
    (KeyCode::End, Key::End),
    (KeyCode::PageUp, Key::PageUp),
    (KeyCode::PageDown, Key::PageDown),
    (KeyCode::Space, Key::Char(' ')),
    (KeyCode::Period, Key::Char('.')),
    (KeyCode::LeftBracket, Key::Char('[')),
    (KeyCode::RightBracket, Key::Char(']')),
    (KeyCode::Slash, Key::Char('/')),
    (KeyCode::A, Key::Char('a')),
    (KeyCode::B, Key::Char('b')),
    (KeyCode::C, Key::Char('c')),
    (KeyCode::D, Key::Char('d')),
    (KeyCode::E, Key::Char('e')),
    (KeyCode::F, Key::Char('f')),
    (KeyCode::H, Key::Char('h')),
    (KeyCode::I, Key::Char('i')),
    (KeyCode::J, Key::Char('j')),
    (KeyCode::L, Key::Char('l')),
    (KeyCode::M, Key::Char('m')),
    (KeyCode::Q, Key::Char('q')),
    (KeyCode::R, Key::Char('r')),
    (KeyCode::S, Key::Char('s')),
    (KeyCode::V, Key::Char('v')),
    (KeyCode::W, Key::Char('w')),
    (KeyCode::X, Key::Char('x')),
    (KeyCode::Y, Key::Char('y')),
    (KeyCode::Z, Key::Char('z')),
    (KeyCode::Key1, Key::Char('1')),
    (KeyCode::Key2, Key::Char('2')),
    (KeyCode::Key3, Key::Char('3')),
    (KeyCode::Key4, Key::Char('4')),
    (KeyCode::Key5, Key::Char('5')),
    (KeyCode::Key6, Key::Char('6')),
    (KeyCode::Key7, Key::Char('7')),
    (KeyCode::Key8, Key::Char('8')),
    (KeyCode::Key9, Key::Char('9')),
    (KeyCode::Kp1, Key::Char('1')),
    (KeyCode::Kp2, Key::Char('2')),
    (KeyCode::Kp3, Key::Char('3')),
    (KeyCode::Kp4, Key::Char('4')),
    (KeyCode::Kp5, Key::Char('5')),
    (KeyCode::Kp6, Key::Char('6')),
    (KeyCode::Kp7, Key::Char('7')),
    (KeyCode::Kp8, Key::Char('8')),
    (KeyCode::Kp9, Key::Char('9')),
];

fn keyboard(
    game: &mut Game,
    repeat_at: &mut f64,
    buffered_move: &mut Option<KeyEvent>,
    held_move: &mut Option<KeyCode>,
) {
    let walking = input::active(game) && game.combat.is_none();
    if !walking {
        *held_move = None;
    }
    if !input::active(game) || game.combat.is_some() {
        *buffered_move = None;
    } else if game.elapsed_ms >= game.move_ready_ms {
        if let Some(event) = buffered_move.take() {
            input::key(game, event);
            if game.combat.is_some() || !input::active(game) {
                *held_move = None;
                return;
            }
        }
    }
    let mut modifiers = KeyModifiers::empty();
    if is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift) {
        modifiers |= KeyModifiers::SHIFT;
    }
    if is_key_down(KeyCode::LeftControl) || is_key_down(KeyCode::RightControl) {
        modifiers |= KeyModifiers::CONTROL;
    }
    let mut pressed = false;
    for &(hardware, mut code) in KEYS {
        if !is_key_pressed(hardware) {
            continue;
        }
        pressed = true;
        if modifiers.contains(KeyModifiers::SHIFT) {
            if let Key::Char(c) = code {
                code = Key::Char(if c == '/' {
                    '?'
                } else {
                    c.to_ascii_uppercase()
                });
            }
        }
        let event = KeyEvent::new(code, modifiers);
        if walking && input::movement_key(code) {
            *held_move = Some(hardware);
        } else {
            *held_move = None;
        }
        if input::active(game)
            && input::movement_key(code)
            && game.combat.is_none()
            && game.elapsed_ms < game.move_ready_ms
        {
            *buffered_move = Some(event);
        } else {
            *buffered_move = None;
            input::key(game, event);
        }
        *repeat_at = get_time() + 0.18;
        break; // one physical press batch never commits multiple rounds
    }
    if !input::active(game) || game.combat.is_some()
        || held_move.is_some_and(|hardware| !is_key_down(hardware))
    {
        *held_move = None;
    }
    // Only a road press arms held walking; modal/combat keys cannot resume it.
    if !pressed
        && input::active(game)
        && game.combat.is_none()
        && modifiers.is_empty()
        && get_time() >= *repeat_at
        && game.elapsed_ms >= game.move_ready_ms
    {
        if let Some(&(_, code)) = KEYS
            .iter()
            .find(|(hardware, code)| input::movement_key(*code) && Some(*hardware) == *held_move)
        {
            input::key(game, KeyEvent::new(code, KeyModifiers::empty()));
            *repeat_at = get_time() + 0.10;
        }
    }
}

fn render(
    game: &Game,
    scene: &mut Terminal<TestBackend>,
    menus: &mut Terminal<TestBackend>,
    actors: &mut sprites::ActorAnimations,
    display: &DisplaySettings,
) -> Result<(), String> {
    // Keep cell dimensions stable across window sizes. Only reduce text when a
    // small window cannot hold the shared menus; never normalize a big display.
    let fit = (screen_width() / 920.0).min(screen_height() / 684.0);
    let scale = display.ui_scale.min(fit.max(0.7));
    let cols = ((screen_width() / (10.0 * scale)) as u16).clamp(1, 320);
    let rows = ((screen_height() / (18.0 * scale)) as u16).clamp(1, 180);
    let grid = Cells::new(0, 0, cols, rows);
    let cell = vec2(10.0 * scale, 18.0 * scale);
    for terminal in [&mut *scene, &mut *menus] {
        if terminal.backend().buffer().area != grid {
            terminal.backend_mut().resize(cols, rows);
            terminal.resize(grid).map_err(|e| e.to_string())?;
        }
    }
    let pixels = |r: Cells| {
        Rect::new(
            r.x as f32 * cell.x,
            r.y as f32 * cell.y,
            r.width as f32 * cell.x,
            r.height as f32 * cell.y,
        )
    };
    let mut map = None;
    scene
        .draw(|frame| {
            map = ui::draw_scene(frame, game, true);
            let status = format!(
                " Map {:.0}px +/- | UI {:.0}% Ctrl+/- | Ctrl+0 reset",
                TILE_SIZES[display.zoom],
                display.ui_scale * 100.0,
            );
            frame.render_widget(
                Paragraph::new(status).style(Style::default().fg(Ink::Yellow)),
                Cells::new(0, 0, cols, 1),
            );
            frame.render_widget(
                Paragraph::new(format!(
                    " F11 {} | numpad +/- | text fit {:.0}%",
                    if display.fullscreen {
                        "windowed"
                    } else {
                        "fullscreen"
                    },
                    scale * 100.0,
                ))
                .style(Style::default().fg(Ink::Gray)),
                Cells::new(0, 1, cols, rows.saturating_sub(1).min(1)),
            );
        })
        .map_err(|e| e.to_string())?;
    clear_background(Color::from_rgba(10, 13, 18, 255));
    draw_buffer(scene.backend().buffer(), cell, false, map);
    if let Some(area) = map {
        sprites::draw_map(game, pixels(area), actors, TILE_SIZES[display.zoom]);
    }
    if !matches!(game.modal, Modal::None) {
        draw_rectangle(
            0.0,
            cell.y * 2.0,
            screen_width(),
            (screen_height() - cell.y * 2.0).max(0.0),
            Color::new(0.0, 0.0, 0.0, 0.48),
        );
        let mut atlas = None;
        menus
            .draw(|frame| {
                atlas = ui::draw_windowed_modal(frame, game);
            })
            .map_err(|e| e.to_string())?;
        draw_buffer(menus.backend().buffer(), cell, true, atlas);
        if let Some(area) = atlas {
            sprites::draw_atlas(game, pixels(area));
        }
    }
    Ok(())
}

fn draw_buffer(buffer: &Buffer, cell_size: Vec2, overlay: bool, skip: Option<Cells>) {
    let font_size = (cell_size.y * 1.2).max(1.0) as u16;
    let advance = measure_text("M", None, font_size, 1.0).width.max(1.0);
    for (index, cell) in buffer.content.iter().enumerate() {
        let col = (index % buffer.area.width as usize) as u16;
        let row = (index / buffer.area.width as usize) as u16;
        if skip.is_some_and(|r| col >= r.x && col < r.right() && row >= r.y && row < r.bottom()) {
            continue;
        }
        let symbol = cell.symbol();
        if overlay && cell.bg == Ink::Reset && symbol == " " {
            continue;
        }
        let x = col as f32 * cell_size.x;
        let y = row as f32 * cell_size.y;
        let reversed = cell.modifier.contains(Modifier::REVERSED);
        let (fg, bg) = if reversed {
            (cell.bg, cell.fg)
        } else {
            (cell.fg, cell.bg)
        };
        if !overlay || bg != Ink::Reset {
            draw_rectangle(x, y, cell_size.x, cell_size.y, color(bg));
        }
        if symbol.trim().is_empty() {
            continue;
        }
        let ink = color(fg);
        if border(symbol, Rect::new(x, y, cell_size.x, cell_size.y), ink) {
            continue;
        }
        // Ratatui gauges use block glyphs: draw solid geometry instead of relying
        // on the window's built-in font containing the Unicode block range.
        if symbol == "█" {
            draw_rectangle(x, y + 2.0, cell_size.x, cell_size.y - 4.0, ink);
            continue;
        }
        let symbol = match symbol {
            "♣" => "t",
            "♠" => "T",
            "▲" => "^",
            "≈" => "~",
            "—" => "-",
            other => other,
        };
        draw_text_ex(
            symbol,
            x,
            y + cell_size.y * 0.80,
            TextParams {
                font_size,
                font_scale_aspect: cell_size.x / advance,
                color: ink,
                ..Default::default()
            },
        );
    }
}

fn border(symbol: &str, r: Rect, ink: Color) -> bool {
    let (left, right, up, down) = match symbol {
        "─" => (true, true, false, false),
        "│" => (false, false, true, true),
        "┌" => (false, true, false, true),
        "┐" => (true, false, false, true),
        "└" => (false, true, true, false),
        "┘" => (true, false, true, false),
        "├" => (false, true, true, true),
        "┤" => (true, false, true, true),
        "┬" => (true, true, false, true),
        "┴" => (true, true, true, false),
        "┼" => (true, true, true, true),
        _ => return false,
    };
    let (cx, cy) = (r.x + r.w / 2.0, r.y + r.h / 2.0);
    if left {
        draw_line(r.x, cy, cx, cy, 1.0, ink);
    }
    if right {
        draw_line(cx, cy, r.x + r.w, cy, 1.0, ink);
    }
    if up {
        draw_line(cx, r.y, cx, cy, 1.0, ink);
    }
    if down {
        draw_line(cx, cy, cx, r.y + r.h, 1.0, ink);
    }
    true
}

fn color(ink: Ink) -> Color {
    let (r, g, b) = match ink {
        Ink::Reset | Ink::Black => (10, 13, 18),
        Ink::Red => (180, 58, 64),
        Ink::Green => (87, 160, 108),
        Ink::Yellow => (220, 183, 105),
        Ink::Blue => (82, 128, 196),
        Ink::Magenta => (170, 105, 186),
        Ink::Cyan => (107, 184, 184),
        Ink::Gray => (193, 200, 207),
        Ink::DarkGray => (111, 124, 138),
        Ink::White => (241, 239, 225),
        Ink::LightRed => (244, 114, 114),
        Ink::LightGreen => (148, 219, 132),
        Ink::LightYellow => (249, 215, 128),
        Ink::LightBlue => (137, 186, 238),
        Ink::LightMagenta => (223, 155, 233),
        Ink::LightCyan => (152, 225, 222),
        Ink::Rgb(r, g, b) => (r, g, b),
        Ink::Indexed(n) => {
            if n < 16 {
                return color(
                    [
                        Ink::Black,
                        Ink::Red,
                        Ink::Green,
                        Ink::Yellow,
                        Ink::Blue,
                        Ink::Magenta,
                        Ink::Cyan,
                        Ink::Gray,
                        Ink::DarkGray,
                        Ink::LightRed,
                        Ink::LightGreen,
                        Ink::LightYellow,
                        Ink::LightBlue,
                        Ink::LightMagenta,
                        Ink::LightCyan,
                        Ink::White,
                    ][n as usize],
                );
            }
            if n >= 232 {
                let v = 8 + (n - 232) * 10;
                (v, v, v)
            } else {
                let n = n - 16;
                let component = |v: u8| if v == 0 { 0 } else { 55 + 40 * v };
                (component(n / 36), component(n / 6 % 6), component(n % 6))
            }
        }
    };
    Color::from_rgba(r, g, b, 255)
}
