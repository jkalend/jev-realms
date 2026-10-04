use crate::model::*;
use crate::social;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Gauge, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};

const INK: Color = Color::Gray;
const MUTED: Color = Color::DarkGray;
const GOLD: Color = Color::Yellow;
const SEA: Color = Color::Cyan;

fn panel(title: impl Into<Line<'static>>) -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(MUTED))
        .title(title)
        .title_style(Style::default().fg(GOLD))
}

fn text(frame: &mut Frame, area: Rect, lines: Vec<Line<'_>>) {
    if area.width > 0 && area.height > 0 {
        frame.render_widget(Paragraph::new(lines).style(Style::default().fg(INK)), area);
    }
}

fn line(value: impl Into<String>, color: Color) -> Line<'static> {
    Line::styled(value.into(), Style::default().fg(color))
}

fn inset(area: Rect) -> Rect {
    Rect::new(
        area.x.saturating_add(1),
        area.y.saturating_add(1),
        area.width.saturating_sub(2),
        area.height.saturating_sub(2),
    )
}

fn centered(area: Rect, width: u16, height: u16) -> Rect {
    let width = width.min(area.width);
    let height = height.min(area.height);
    Rect::new(
        area.x + (area.width - width) / 2,
        area.y + (area.height - height) / 2,
        width,
        height,
    )
}

fn ai_label(game: &Game) -> String {
    if game.ai.switched {
        "AI DEGRADED / offline fallback".into()
    } else if game.ai.provider.to_ascii_lowercase().contains("laya") {
        if game.ai.consecutive_failures > 0 {
            format!("Laya live / fallback x{}", game.ai.consecutive_failures)
        } else {
            "Laya live".into()
        }
    } else {
        "AI offline / heuristics".into()
    }
}

pub fn draw(frame: &mut Frame, game: &Game) {
    draw_scene(frame, game, false);
    draw_modal(frame, frame.area(), game);
}

/// Shared layout with view-specific legends; the window replaces the map rectangle.
pub fn draw_scene(frame: &mut Frame, game: &Game, sprite_view: bool) -> Option<Rect> {
    // The GUI reserves two real rows for display controls, never covering HUD/menu cells.
    let area = if sprite_view {
        windowed_area(frame.area())
    } else {
        frame.area()
    };
    frame.render_widget(
        Block::default().style(Style::default().bg(Color::Black).fg(INK)),
        area,
    );
    if area.width == 0 || area.height == 0 {
        return None;
    }
    if area.width < 24 || area.height < 10 {
        frame.render_widget(
            Paragraph::new("LAYA REALMS\nTerminal too small.\nResize: 80x30 minimum\n100x40 recommended\nEsc pause / Q in pause")
                .style(Style::default().fg(GOLD))
                .wrap(Wrap { trim: true }),
            area,
        );
        return None;
    }
    let bands = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(3),
            Constraint::Length(4),
            Constraint::Length(if sprite_view && area.width < 100 {
                2
            } else {
                1
            }),
        ])
        .split(area);
    let title = format!(
        " LAYA REALMS  /  {}  /  {:02}:00 {}  |  {}",
        game.map().name,
        game.hour(),
        if game.night() { "night" } else { "day" },
        ai_label(game)
    );
    text(
        frame,
        bands[0],
        vec![line(
            title,
            if game.ai.switched {
                Color::LightRed
            } else {
                GOLD
            },
        )],
    );
    let map_area;
    let sidebar_threshold = if sprite_view { 80 } else { 86 };
    if bands[1].width >= sidebar_threshold {
        let map_width = if sprite_view {
            bands[1]
                .width
                .saturating_sub(if game.inspector { 40 } else { 36 })
        } else if game.inspector {
            bands[1].width.saturating_sub(42).min(62)
        } else {
            62
        };
        let columns = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(map_width), Constraint::Min(24)])
            .split(bands[1]);
        map_area = columns[0];
        draw_map(frame, columns[0], game, sprite_view);
        if game.inspector {
            draw_inspector(frame, columns[1], game, sprite_view);
        } else {
            draw_sidebar(frame, columns[1], game, sprite_view);
        }
    } else {
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(3),
                Constraint::Length(if bands[1].height > 19 { 8 } else { 4 }),
            ])
            .split(bands[1]);
        map_area = rows[0];
        draw_map(frame, rows[0], game, sprite_view);
        draw_log(frame, rows[1], game, false, sprite_view);
        if game.inspector {
            let overlay = centered(bands[1], 70, bands[1].height);
            frame.render_widget(Clear, overlay);
            draw_inspector(frame, overlay, game, sprite_view);
        }
    }
    draw_hud(frame, bands[2], game);
    let keys = if game.combat.is_some() {
        " Move / Enter attack | F defend X flee C mercy | I items J AI ? help"
    } else {
        " WASD/arrows move | E interact I bag B journal M atlas J AI ? help"
    };
    if sprite_view {
        frame.render_widget(
            Paragraph::new(keys)
                .style(Style::default().fg(MUTED))
                .wrap(Wrap { trim: true }),
            bands[3],
        );
    } else {
        text(frame, bands[3], vec![line(keys, MUTED)]);
    }
    if bands[1].width < sidebar_threshold && game.inspector {
        return None;
    }
    let inner = inset(map_area);
    Some(Rect::new(
        inner.x,
        inner.y,
        inner.width,
        if sprite_view {
            inner.height
        } else {
            inner.height.min(30)
        },
    ))
}

/// Comparison of a gear item against the currently equipped tier, e.g. "+6 ATK".
/// Returns a short label and its color, or None for non-gear.
fn gear_note(game: &Game, item: &Item) -> Option<(String, Color)> {
    if let Item::BossRelic(relic) = item {
        return Some(if game.player.relic == Some(*relic) {
            ("equipped".into(), GOLD)
        } else {
            ("boss relic".into(), Color::LightMagenta)
        });
    }
    let delta = match item {
        Item::Weapon(t) => (i32::from(*t) - i32::from(game.player.weapon)) * 3,
        Item::Armour(t) => (i32::from(*t) - i32::from(game.player.armour)) * 2,
        _ => return None,
    };
    let stat = if matches!(item, Item::Weapon(_)) {
        "ATK"
    } else {
        "DEF"
    };
    Some(if delta > 0 {
        (format!("+{delta} {stat}"), Color::LightGreen)
    } else if delta < 0 {
        (format!("{delta} {stat}"), Color::LightRed)
    } else {
        ("equipped tier".into(), MUTED)
    })
}

fn tile_color(tile: Tile) -> Color {
    match tile {
        Tile::Grass => Color::Rgb(74, 150, 62),
        Tile::Forest => Color::Rgb(38, 116, 48),
        Tile::DeepForest => Color::Rgb(16, 74, 38),
        Tile::Mountain => Color::Rgb(148, 140, 134),
        Tile::Rock => Color::Rgb(102, 96, 92),
        Tile::River => Color::Rgb(42, 106, 196),
        Tile::Ford => Color::Rgb(78, 170, 200),
        Tile::Ruins => Color::Rgb(186, 122, 198),
        Tile::Wall | Tile::Floor => Color::Rgb(112, 108, 104),
        Tile::Road => Color::Rgb(188, 154, 96),
        Tile::Shrine => Color::Rgb(208, 112, 218),
        Tile::Door | Tile::Chest | Tile::Up | Tile::Down => Color::Rgb(232, 198, 92),
    }
}

/// Scales an RGB color by a light factor; ANSI colors pass through unchanged.
fn shade(color: Color, factor: f32) -> Color {
    match color {
        Color::Rgb(r, g, b) => Color::Rgb(
            (r as f32 * factor) as u8,
            (g as f32 * factor) as u8,
            (b as f32 * factor) as u8,
        ),
        other => other,
    }
}

pub fn vision(game: &Game) -> i32 {
    if game.night() {
        if game.tick < game.player.torch_until {
            6
        } else {
            4
        }
    } else {
        12
    }
}

fn draw_map(frame: &mut Frame, area: Rect, game: &Game, sprite_view: bool) {
    let map = game.map();
    let title = format!(
        " {}  [{},{}] ",
        map.name, game.player.pos.x, game.player.pos.y
    );
    frame.render_widget(panel(title), area);
    if sprite_view {
        return;
    }
    let inner = inset(area);
    if inner.width == 0 || inner.height == 0 {
        return;
    }
    let width = inner.width.min(60).min(map.width.max(0) as u16);
    let height = inner.height.min(30).min(map.height.max(0) as u16);
    let view = Rect::new(inner.x + (inner.width - width) / 2, inner.y, width, height);
    let left =
        (game.player.pos.x - i32::from(width) / 2).clamp(0, (map.width - i32::from(width)).max(0));
    let top = (game.player.pos.y - i32::from(height) / 2)
        .clamp(0, (map.height - i32::from(height)).max(0));
    let radius = vision(game);
    for y in 0..height {
        for x in 0..width {
            let pos = Pos::new(left + i32::from(x), top + i32::from(y));
            let Some(index) = map.index(pos) else {
                continue;
            };
            if !map.explored.get(index).copied().unwrap_or(false) {
                continue;
            }
            let tile = map.tiles[index];
            let dist = pos.distance(game.player.pos);
            let visible = dist <= radius;
            // Three light bands: full color, a falloff rim, then remembered gray
            // beyond vision. Deep forest drinks what little light it gets.
            let mut style = if visible {
                let band = if dist * 4 <= radius * 3 { 1.0 } else { 0.55 };
                let mut color = shade(tile_color(tile), band);
                if tile == Tile::DeepForest {
                    color = shade(color, 0.9);
                }
                Style::default().fg(color)
            } else {
                Style::default().fg(MUTED).add_modifier(Modifier::DIM)
            };
            let glyph = if tile == Tile::River && visible {
                // Water shimmers as the world ticks.
                if (pos.x + pos.y + game.tick as i32 / 4).rem_euclid(2) == 0 {
                    '≈'
                } else {
                    '~'
                }
            } else if tile == Tile::Door && map.door_open(pos) {
                '/'
            } else {
                tile.glyph()
            };
            if let Some(combat) = &game.combat {
                if visible && pos.distance(combat.center) == 5 {
                    style = Style::default().fg(Color::Red).bg(Color::DarkGray);
                }
            }
            frame.buffer_mut()[(view.x + x, view.y + y)]
                .set_char(glyph)
                .set_style(style);
        }
    }
    let screen = |pos: Pos| -> Option<(u16, u16)> {
        let x = pos.x - left;
        let y = pos.y - top;
        (x >= 0 && y >= 0 && x < i32::from(width) && y < i32::from(height)).then_some((
            view.x.saturating_add(x.max(0) as u16),
            view.y.saturating_add(y.max(0) as u16),
        ))
    };
    for portal in &map.portals {
        if portal.pos.distance(game.player.pos) > radius {
            continue;
        }
        if let Some((x, y)) = screen(portal.pos) {
            frame.buffer_mut()[(x, y)]
                .set_char(map.tile(portal.pos).glyph())
                .set_style(Style::default().fg(GOLD).add_modifier(Modifier::BOLD));
            if portal.pos.distance(game.player.pos) <= 3 && x + 2 < view.right() {
                let label_area = Rect::new(x + 2, y, (view.right() - x - 2).min(24), 1);
                frame.render_widget(
                    Paragraph::new(portal.label.as_str())
                        .style(Style::default().fg(GOLD).bg(Color::Black)),
                    label_area,
                );
            }
        }
    }
    for npc in &game.npcs {
        if npc.alive() && npc.map == game.player.map {
            if let Some((target, _)) = &npc.telegraph {
                // Wind-up cells pulse between two intensities as ticks advance.
                let style = if game.tick.is_multiple_of(2) {
                    Style::default()
                        .fg(Color::White)
                        .bg(Color::LightRed)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::Yellow).bg(Color::Red)
                };
                for dy in -1..=1 {
                    for dx in -1..=1 {
                        if let Some((x, y)) = screen(target.offset(dx, dy)) {
                            frame.buffer_mut()[(x, y)].set_char('!').set_style(style);
                        }
                    }
                }
            }
        }
    }
    for npc in &game.npcs {
        if !npc.alive() || npc.map != game.player.map || npc.pos.distance(game.player.pos) > radius
        {
            continue;
        }
        if let Some((x, y)) = screen(npc.pos) {
            let color = if npc.intent == Intent::Flee {
                GOLD
            } else if npc.archetype.boss() {
                Color::LightMagenta
            } else if npc.archetype.hostile() || npc.intent == Intent::Attack {
                Color::LightRed
            } else {
                match npc.archetype {
                    Archetype::Vendor => GOLD,
                    Archetype::Guard => Color::LightBlue,
                    Archetype::Oracle => Color::LightMagenta,
                    _ => SEA,
                }
            };
            frame.buffer_mut()[(x, y)]
                .set_char(npc.archetype.glyph())
                .set_style(
                    Style::default()
                        .fg(color)
                        .bg(Color::Black)
                        .add_modifier(Modifier::BOLD),
                );
        }
    }
    if let Some((x, y)) = screen(game.player.pos) {
        frame.buffer_mut()[(x, y)].set_char('@').set_style(
            Style::default()
                .fg(Color::White)
                .bg(Color::Black)
                .add_modifier(Modifier::BOLD),
        );
    }
    for effect in game.effects.iter().filter(|effect| effect.ttl > 0) {
        if effect.pos.distance(game.player.pos) > radius {
            continue;
        }
        if let Some((x, y)) = screen(effect.pos) {
            let y = if y > view.y { y - 1 } else { y };
            let width = view.right().saturating_sub(x);
            frame.render_widget(
                Paragraph::new(effect.text.as_str()).style(
                    Style::default()
                        .fg(Color::LightRed)
                        .bg(Color::Black)
                        .add_modifier(Modifier::BOLD),
                ),
                Rect::new(x, y, width, 1),
            );
        }
    }
    if inner.height > height {
        let mut legend = vec![
            line(" @ you  V vendor  G guard  ? oracle", MUTED),
            line(" > descend  < ascend  + door  $ loot", MUTED),
        ];
        if let Some(portal) = map
            .portals
            .iter()
            .min_by_key(|p| p.pos.distance(game.player.pos))
        {
            legend.push(line(
                format!(
                    " Nearest: {} [{},{}]",
                    portal.label, portal.pos.x, portal.pos.y
                ),
                GOLD,
            ));
        }
        text(
            frame,
            Rect::new(
                inner.x,
                inner.y + height,
                inner.width,
                inner.height - height,
            ),
            legend,
        );
    }
}

fn draw_hud(frame: &mut Frame, area: Rect, game: &Game) {
    let p = &game.player;
    if area.height == 0 {
        return;
    }
    let gauges = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(38),
            Constraint::Percentage(32),
            Constraint::Percentage(30),
        ])
        .split(Rect::new(area.x, area.y, area.width, 1));
    let hp_ratio = (f64::from(p.hp.max(0)) / f64::from(p.max_hp.max(1))).clamp(0.0, 1.0);
    let stamina_ratio =
        (f64::from(p.stamina.max(0)) / f64::from(p.max_stamina.max(1))).clamp(0.0, 1.0);
    frame.render_widget(
        Gauge::default()
            .ratio(hp_ratio)
            .label(format!(" HP {}/{} ", p.hp, p.max_hp))
            .gauge_style(
                Style::default()
                    .fg(if hp_ratio <= 0.3 {
                        Color::LightRed
                    } else {
                        Color::Red
                    })
                    .bg(Color::Black),
            ),
        gauges[0],
    );
    frame.render_widget(
        Gauge::default()
            .ratio(stamina_ratio)
            .label(format!(" STAMINA {}/{} ", p.stamina, p.max_stamina))
            .gauge_style(Style::default().fg(Color::Green).bg(Color::Black)),
        gauges[1],
    );
    let mana_ratio = (f64::from(p.mana.max(0)) / f64::from(p.max_mana.max(1))).clamp(0.0, 1.0);
    frame.render_widget(
        Gauge::default()
            .ratio(mana_ratio)
            .label(if p.max_mana > 0 {
                format!(" MANA {}/{} ", p.mana, p.max_mana)
            } else {
                " MANA - study runes ".to_string()
            })
            .gauge_style(
                Style::default()
                    .fg(if p.max_mana > 0 {
                        Color::Blue
                    } else {
                        MUTED
                    })
                    .bg(Color::Black),
            ),
        gauges[2],
    );
    let sigils = p.sigils.iter().filter(|&&s| s).count();
    let xp_text = if p.level >= 10 {
        "MAX".to_string()
    } else {
        format!("{}/{}", p.xp, 25 + p.level * 15)
    };
    let status = if game.turn < p.ward_until {
        " | WARDED"
    } else if p.defending {
        " | DEFENDING"
    } else if game.turn < p.curse_until {
        " | CURSED"
    } else if game.tick < p.torch_until {
        " | TORCH LIT"
    } else {
        ""
    };
    let mut spans = vec![Span::styled(
        format!(
            " Lv {}  XP {}  Gold {}  Sigils {}/3  ATK {} DEF {}{}",
            p.level,
            xp_text,
            p.gold,
            sigils,
            game.attack_power(),
            game.defense_power(),
            status
        ),
        Style::default().fg(GOLD),
    )];
    if let Some((meter, color)) = class_meter(game) {
        spans.push(Span::styled(meter, Style::default().fg(color)));
    }
    text(
        frame,
        Rect::new(
            area.x,
            area.y + 1,
            area.width,
            area.height.saturating_sub(1),
        ),
        vec![
            Line::from(spans),
            line(
                format!(
                    " {} sword / {} armour / relic {} | Rep M {} H {} S {}",
                    Item::tier(p.weapon),
                    Item::tier(p.armour),
                    p.relic.map_or("none", BossRelic::name),
                    p.reputation[0],
                    p.reputation[1],
                    p.reputation[2]
                ),
                INK,
            ),
            line(
                format!(
                    " {} | queued {} | decisions {} | fallback {} | turn {}",
                    ai_label(game),
                    game.ai.pending,
                    game.ai.requests,
                    game.ai.degraded,
                    game.turn
                ),
                if game.ai.switched || game.ai.consecutive_failures > 0 {
                    Color::LightRed
                } else {
                    MUTED
                },
            ),
        ],
    );
}

fn draw_sidebar(frame: &mut Frame, area: Rect, game: &Game, sprite_view: bool) {
    let combat_height = if game.combat.is_some() {
        if sprite_view {
            (area.height / 2).clamp(14, 22)
        } else {
            11
        }
    } else if sprite_view {
        (area.height / 3).clamp(9, 14)
    } else {
        7
    };
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(combat_height), Constraint::Min(4)])
        .split(area);
    if game.combat.is_some() {
        draw_combat(frame, sections[0], game, sprite_view);
    } else {
        frame.render_widget(panel(" FIELD NOTES "), sections[0]);
        let mut notes = vec![line(
            if sprite_view {
                "Gold feet: you / E interact"
            } else {
                "@ you / V trade / G watch"
            },
            INK,
        )];
        if sprite_view {
            notes.push(line("Teal cape: you / apron: vendor", MUTED));
            notes.push(line("Pips: red hostile, gold fleeing, violet boss", MUTED));
        }
        if let Some(quest) = game.quests.iter().find(|q| {
            matches!(
                q.stage,
                QuestStage::Ready | QuestStage::Active | QuestStage::Available
            )
        }) {
            notes.push(line(quest.title.clone(), GOLD));
            notes.push(line(quest.description.clone(), INK));
        }
        if let Some(portal) = game
            .map()
            .portals
            .iter()
            .min_by_key(|p| p.pos.distance(game.player.pos))
        {
            notes.push(line(
                format!("{} [{},{}]", portal.label, portal.pos.x, portal.pos.y),
                SEA,
            ));
        }
        if let Some(id) = game.companion {
            if let Some(blade) = game.npcs.get(id) {
                notes.push(line(
                    if blade.alive() {
                        format!("{} {}/{} HP — with you", blade.name, blade.hp, blade.max_hp)
                    } else {
                        format!("{} is down — rises after the fight", blade.name)
                    },
                    Color::LightGreen,
                ));
            }
        }
        frame.render_widget(
            Paragraph::new(notes).wrap(Wrap { trim: true }),
            inset(sections[0]),
        );
    }
    draw_log(frame, sections[1], game, true, sprite_view);
}

fn draw_log(frame: &mut Frame, area: Rect, game: &Game, history: bool, wrap: bool) {
    frame.render_widget(panel(" CHRONICLE / latest first "), area);
    let inner = inset(area);
    let count = if history {
        usize::from(inner.height)
    } else {
        6
    };
    let lines = game
        .log
        .iter()
        .rev()
        .take(count)
        .enumerate()
        .map(|(i, event)| {
            let color = if i == 0 {
                Color::White
            } else if i < 6 {
                INK
            } else {
                MUTED
            };
            Line::from(vec![
                Span::styled(
                    if i < 6 { "> " } else { "  " },
                    Style::default().fg(if i == 0 { GOLD } else { MUTED }),
                ),
                Span::styled(event.as_str(), Style::default().fg(color)),
            ])
        })
        .collect::<Vec<_>>();
    if wrap {
        frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: true }), inner);
    } else {
        text(frame, inner, lines);
    }
}

fn draw_combat(frame: &mut Frame, area: Rect, game: &Game, wrap: bool) {
    let Some(combat) = &game.combat else { return };
    frame.render_widget(
        panel(format!(" ENGAGEMENT / round {} ", combat.round))
            .border_style(Style::default().fg(Color::Red)),
        area,
    );
    let mut order: Vec<(i32, usize, &str)> = combat
        .participants
        .iter()
        .filter_map(|&id| game.npcs.get(id))
        .filter(|npc| npc.alive())
        .map(|npc| (npc.speed, npc.id, npc.name.as_str()))
        .collect();
    order.push((game.player.speed, usize::MAX, "YOU"));
    order.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    let queue = order
        .iter()
        .map(|(_, _, name)| *name)
        .collect::<Vec<_>>()
        .join(" > ");
    let mut lines = vec![
        line("11x11 bubble / speed initiative", MUTED),
        line(queue, SEA),
    ];
    if let Some(id) = game.companion {
        if let Some(blade) = game.npcs.get(id) {
            if blade.alive() && blade.map == game.player.map {
                lines.push(line(
                    format!("{} {}/{} HP (ally)", blade.name, blade.hp, blade.max_hp),
                    Color::LightGreen,
                ));
            }
        }
    }
    for id in &combat.participants {
        let Some(npc) = game.npcs.get(*id).filter(|npc| npc.alive()) else {
            continue;
        };
        lines.push(line(
            format!(
                "{} {}/{} HP{}",
                npc.name,
                npc.hp,
                npc.max_hp,
                if npc.decision_pending {
                    " / judging"
                } else {
                    ""
                }
            ),
            if npc.archetype.boss() {
                Color::LightMagenta
            } else {
                Color::LightRed
            },
        ));
        if npc.archetype.boss() {
            lines.push(line(
                format!(
                    "Phase {}: {}",
                    npc.phase,
                    if npc.tactic.is_empty() {
                        "watching your next move"
                    } else {
                        &npc.tactic
                    }
                ),
                GOLD,
            ));
            if let Some((_, tactic)) = &npc.telegraph {
                let countdown = if npc.cooldown > 0 {
                    "in 2 actions"
                } else {
                    "this round"
                };
                lines.push(line(
                    format!(
                        "WIND-UP: {tactic} resolves {countdown} — leave the red area or defend"
                    ),
                    Color::LightRed,
                ));
            }
        }
        if lines.len() >= usize::from(area.height.saturating_sub(3)) {
            break;
        }
    }
    lines.push(line("F defend / X flee / C mercy", MUTED));
    if wrap {
        frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: true }), inset(area));
    } else {
        text(frame, inset(area), lines);
    }
}

fn draw_inspector(frame: &mut Frame, area: Rect, game: &Game, wrap: bool) {
    frame.render_widget(panel(" LAYA LENS / [ ] records / J close "), area);
    let inner = inset(area);
    if inner.height == 0 || inner.width == 0 {
        return;
    }
    let count = game.decisions.len().min(10);
    if count == 0 {
        frame.render_widget(Paragraph::new("No judgments yet.\nApproach an NPC or enter combat.\n\nEvery probability below becomes a deterministic Rust rule; no generated dialogue.").style(Style::default().fg(MUTED)).wrap(Wrap { trim: true }), inner);
        return;
    }
    let selected = game.selected.min(count - 1);
    let summaries = count.min(usize::from((inner.height / 3).max(1)));
    let start = selected.saturating_sub(summaries.saturating_sub(1));
    let mut lines = vec![line(
        format!("Last {} / newest first / {}/{}", count, selected + 1, count),
        MUTED,
    )];
    for (index, record) in game
        .decisions
        .iter()
        .rev()
        .take(10)
        .enumerate()
        .skip(start)
        .take(summaries)
    {
        let result = &record.result;
        lines.push(line(
            format!(
                "{} {} T{} {}ms",
                if index == selected { ">" } else { " " },
                result.request.npc_name,
                result.request.tier,
                result.latency_ms
            ),
            if index == selected { GOLD } else { MUTED },
        ));
    }
    let Some(record) = game.decisions.iter().rev().nth(selected) else {
        return;
    };
    let result = &record.result;
    lines.push(line(
        format!(
            "{} / {} / {}ms",
            result.request.npc_name, result.provider, result.latency_ms
        ),
        SEA,
    ));
    lines.push(line(format!("RULE: {}", record.applied_rule), GOLD));
    if let Some(error) = &result.error {
        lines.push(line(format!("Fallback: {}", error), Color::LightRed));
    }
    for (name, answer) in &result.answers {
        let kind = result
            .request
            .questions
            .get(name)
            .map(|q| q.kind.as_str())
            .unwrap_or("noul");
        let label = answer.choice.as_deref().unwrap_or("-");
        lines.push(line(format!("{} [{}]", name, kind), INK));
        if kind == "choice" {
            lines.push(line(
                format!("  chose {} / conf {:.0}%", label, answer.confidence * 100.0),
                SEA,
            ));
            let mut top: [Option<(&str, f32)>; 3] = [None; 3];
            for (choice, probability) in &answer.probabilities {
                for slot in 0..3 {
                    if top[slot].is_none_or(|(_, p)| *probability > p) {
                        for next in ((slot + 1)..3).rev() {
                            top[next] = top[next - 1];
                        }
                        top[slot] = Some((choice.as_str(), *probability));
                        break;
                    }
                }
            }
            for (choice, probability) in top.into_iter().flatten() {
                lines.push(line(
                    format!("  {} {:.1}%", choice, probability * 100.0),
                    MUTED,
                ));
            }
        } else if kind == "score" {
            lines.push(line(
                format!("  {} / {:.2} normalized", label, answer.value),
                SEA,
            ));
        } else {
            lines.push(line(
                format!(
                    "  P(yes) {:.1}% / conf {:.0}%",
                    answer.value * 100.0,
                    answer.confidence * 100.0
                ),
                SEA,
            ));
        }
    }
    if wrap {
        frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: true }), inner);
    } else {
        text(frame, inner, lines);
    }
}

fn modal(frame: &mut Frame, area: Rect, width: u16, height: u16, title: &str) -> Rect {
    let box_area = centered(area, width, height);
    frame.render_widget(Clear, box_area);
    frame.render_widget(
        panel(format!(" {} ", title))
            .border_style(Style::default().fg(GOLD))
            .style(Style::default().bg(Color::Black)),
        box_area,
    );
    inset(box_area)
}

fn draw_modal(frame: &mut Frame, area: Rect, game: &Game) {
    match game.modal {
        Modal::None => {}
        Modal::Title => draw_title(frame, area, game),
        Modal::Create => draw_create(frame, area, game),
        Modal::Help => draw_help(frame, area),
        Modal::Pause => {
            let inner = modal(frame, area, 58, 12, "CAMPFIRE / paused");
            text(
                frame,
                inner,
                vec![
                    line("", INK),
                    line("  The world waits. Save before leaving the road.", GOLD),
                    line("", INK),
                    line("  Esc / Enter    Return to the road", INK),
                    line("  ?              Controls and field guide", INK),
                    line("  S              Save and quit (not during combat)", INK),
                    line("  Q              Quit without saving", INK),
                    line("", INK),
                    line("  One save per seed. Load it from the title with L.", MUTED),
                ],
            );
        }
        Modal::Inventory => draw_inventory(frame, area, game),
        Modal::Trade(npc) => draw_trade(frame, area, game, npc),
        Modal::Talk(npc) => draw_talk(frame, area, game, npc),
        Modal::Journal => draw_journal(frame, area, game),
        Modal::Atlas => draw_atlas(frame, area, game),
        Modal::Cast => draw_cast(frame, area, game),
        Modal::Forge(id) => draw_forge(frame, area, game, id),
        Modal::Oath => draw_oath(frame, area, game),
        Modal::Talents => draw_talents(frame, area, game),
        Modal::Death => {
            let inner = modal(frame, area, 58, 13, "THE ROAD TAKES ITS TOLL");
            let felled = if game.player.last_hazard.is_empty() {
                String::new()
            } else {
                format!("  Felled by: {}", game.player.last_hazard)
            };
            text(
                frame,
                inner,
                vec![
                    line("", INK),
                    line("  You fell. The world remembers.", Color::LightRed),
                    line(felled, INK),
                    line("", INK),
                    line("  Enter returns you to your last city.", INK),
                    line("  You lose 25% of your gold, not your progress.", MUTED),
                    line("  Fallen enemies stay fallen. Quests endure.", MUTED),
                    line("", INK),
                    line(
                        format!("  Last refuge: {}", city_name(game.player.last_city)),
                        GOLD,
                    ),
                    line("  [ Enter ]  Rise and try another route", SEA),
                ],
            );
        }
        Modal::Victory => {
            let h = &game.player.history;
            let inner = modal(frame, area, 66, 20, "THE FINAL VERDICT");
            text(
                frame,
                inner,
                vec![
                    line("", INK),
                    line("  THREE SIGILS. FOUR LORDS. ONE LAST JUDGMENT.", GOLD),
                    line("", INK),
                    line("  The Adjudicator has fallen. Your choices remain.", INK),
                    line("", INK),
                    line(
                        format!(
                            "  Level {}  /  {} gold  /  {} quests completed",
                            game.player.level, game.player.gold, h.quests
                        ),
                        SEA,
                    ),
                    line(
                        format!(
                            "  {} kills  /  {} mercies  /  {} retreats",
                            h.kills, h.mercy, h.fled
                        ),
                        INK,
                    ),
                    line(
                        format!(
                            "  {} bribes  /  {} thefts  /  {} deaths",
                            h.bribes, h.thefts, h.deaths
                        ),
                        MUTED,
                    ),
                    line(
                        format!(
                            "  {} decisions  /  {} fallback judgments",
                            game.ai.requests, game.ai.degraded
                        ),
                        MUTED,
                    ),
                    line("", INK),
                    // E7 (§6.5): the arbiter's question hangs in the verdict hall —
                    // the Run screen reads its stance, rated by the journey's telemetry.
                    {
                        let sincerity = if game.mercy_oathbreaker {
                            " — it judged your mercy for the oathbreaker sincere"
                        } else {
                            " — it weighed the kneel you kept walking past"
                        };
                        match game.epilogue {
                            Epilogue::Sealed => line(
                                format!("  EPILOGUE: you sealed the cell forever{sincerity}."),
                                GOLD,
                            ),
                            Epilogue::Watch => line(
                                format!("  EPILOGUE: you assumed the Watch{sincerity}."),
                                SEA,
                            ),
                            Epilogue::Unanswered => line(
                                if game.player.inventory.contains(&Item::FirstWrit) {
                                    "  The First Writ hums: stand at the verdict shrine for the arbiter's question."
                                } else {
                                    "  Beneath the Underkeep's depths, a second door waits for whoever seeks it."
                                },
                                MUTED,
                            ),
                        }
                    },
                    line("  [ Enter ]  Keep exploring this world", GOLD),
                    line("  J inspects the decisions behind the journey.", INK),
                    line(
                        "  Esc, then S saves and quits; Q leaves without saving.",
                        MUTED,
                    ),
                ],
            );
        }
    }
}

/// Render menus after the sprite map; untouched cells retain a transparent reset style.
pub fn draw_windowed_modal(frame: &mut Frame, game: &Game) -> Option<Rect> {
    let area = windowed_area(frame.area());
    draw_modal(frame, area, game);
    matches!(game.modal, Modal::Atlas).then(|| atlas_areas(area).1)
}

fn windowed_area(area: Rect) -> Rect {
    Rect::new(
        area.x,
        area.y.saturating_add(2),
        area.width,
        area.height.saturating_sub(2),
    )
}

fn atlas_areas(area: Rect) -> (Rect, Rect) {
    let inner = inset(centered(area, 92, 36));
    let height = inner.height.saturating_sub(12).min(20);
    let width = inner.width.min(86);
    (
        inner,
        Rect::new(inner.x + (inner.width - width) / 2, inner.y, width, height),
    )
}

/// The class signature meter chip on the HUD gauges row (§4.3, D15). Counters
/// only — the class hue is the identity read, not a new economy.
fn class_meter(game: &Game) -> Option<(String, Color)> {
    let p = &game.player;
    let state = &p.class_state;
    let (meter, color) = match p.class {
        Class::None => return None,
        Class::Keepwarden => (
            format!("  RESOLVE {} — {}", state.resolve, state.stance.name()),
            Color::LightYellow,
        ),
        Class::Gravebound => {
            let tier = if p.hp * 4 < p.max_hp {
                "BLAZING"
            } else if p.hp * 2 < p.max_hp {
                "lit"
            } else {
                "dim"
            };
            (format!("  VIGIL {tier}"), Color::Rgb(240, 235, 215))
        }
        Class::Redwake => (
            format!(
                "  MOMENTUM {}{}",
                state.momentum,
                if state.momentum >= 3 { " (exit paid)" } else { "" }
            ),
            Color::Rgb(220, 120, 60),
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
            Color::Rgb(175, 160, 95),
        ),
        Class::SigilSworn => (
            format!("  CHANNEL {} charge", state.channel),
            Color::LightCyan,
        ),
        Class::Fensworn => (
            format!("  BOND double in {}", 3 - state.bond),
            Color::Rgb(170, 120, 220),
        ),
    };
    Some((meter, color))
}

/// Character creation (§3): order pick first, then the origin boon (D2). The six
/// orders present as three callings of two (D33 flavor garnish).
fn draw_create(frame: &mut Frame, area: Rect, game: &Game) {
    let inner = modal(frame, area, 76, 22, "CHOOSE YOUR ORDER");
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(7),
            Constraint::Length(4),
            Constraint::Length(2),
        ])
        .split(inner);
    if game.create_step == 0 {
        text(
            frame,
            rows[0],
            vec![
                line("  The Vigil fell. Its orders endure — swear with one of them.", GOLD),
                line("  Six orders, three callings: Ward holds, Hunt pursues, Speaker reads.", MUTED),
            ],
        );
        let items = (0..6)
            .map(|index| {
                let class = Class::from_index(index);
                ListItem::new(Line::from(vec![
                    Span::styled(
                        format!("  {:<8}", class.calling()),
                        Style::default().fg(MUTED),
                    ),
                    Span::styled(class.name(), Style::default().fg(INK)),
                ]))
            })
            .collect();
        menu(frame, rows[1], items, game.selected);
        let class = Class::from_index(game.selected);
        text(
            frame,
            rows[2],
            vec![
                line(format!("  {}", class.blurb()), INK),
                line(format!("  {}", class.diff()), GOLD),
            ],
        );
        text(
            frame,
            rows[3],
            vec![line("  Enter swear an oath / Esc return", MUTED)],
        );
    } else if game.create_step == 1 {
        let class = Class::from_index(game.create_class);
        text(
            frame,
            rows[0],
            vec![
                line("  Which frame walks the road with you?", GOLD),
                line(
                    format!("  The order is {} ({}).", class.name(), class.calling()),
                    MUTED,
                ),
            ],
        );
        let items = (0..2)
            .map(|index| {
                ListItem::new(Line::styled(
                    format!("  {}", Build::from_index(index).name()),
                    Style::default().fg(INK),
                ))
            })
            .collect();
        menu(frame, rows[1], items, game.selected);
        text(
            frame,
            rows[2],
            vec![line(
                format!("  {}", Build::from_index(game.selected).note()),
                INK,
            )],
        );
        text(
            frame,
            rows[3],
            vec![line("  Enter take this frame / Esc choose another order", MUTED)],
        );
    } else {
        text(
            frame,
            rows[0],
            vec![line("  The Vigil left something behind for you. Take a boon — or take nothing.", GOLD)],
        );
        let items = (0..3)
            .map(|index| {
                ListItem::new(Line::styled(
                    format!("  {}", Boon::from_index(index).name()),
                    Style::default().fg(INK),
                ))
            })
            .collect();
        menu(frame, rows[1], items, game.selected);
        text(
            frame,
            rows[2],
            vec![line(
                format!("  {}", Boon::from_index(game.selected).describe()),
                INK,
            )],
        );
        text(
            frame,
            rows[3],
            vec![line("  Enter take the road / Esc choose your frame", MUTED)],
        );
    }
}

/// The Keepwarden's Bulwark oath pick, opened with F in combat (§4.2).
fn draw_oath(frame: &mut Frame, area: Rect, game: &Game) {
    let inner = modal(frame, area, 64, 12, "SWEAR YOUR OATH");
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(5), Constraint::Length(3)])
        .split(inner);
    let items = [OathStance::Hold, OathStance::Break, OathStance::Breathe]
        .into_iter()
        .map(|stance| ListItem::new(Line::styled(stance.name(), Style::default().fg(INK))))
        .collect();
    menu(frame, rows[0], items, game.selected);
    let stance = [&OathStance::Hold, &OathStance::Break, &OathStance::Breathe][
        game.selected.min(2)
    ];
    text(
        frame,
        rows[1],
        vec![
            line(format!("  {}", stance.describe()), GOLD),
            line("  Enter swear / Esc hold position", MUTED),
        ],
    );
}

/// The talent screen (§5, D9 — shared select-list idiom, both views).
fn draw_talents(frame: &mut Frame, area: Rect, game: &Game) {
    let inner = modal(frame, area, 88, 27, "OATH-TREES");
    let Some(tree) = talent_tree(game.player.class) else {
        text(
            frame,
            inner,
            vec![
                line("", INK),
                line("  The Unsworn keep no lessons. The six orders each teach theirs.", GOLD),
                line("", INK),
                line("  (This journey predates the classes; its pages stay blank.)", MUTED),
            ],
        );
        return;
    };
    let available = game.player.talent_points_available();
    let selected = game.selected.min(11);
    let header = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Min(14), Constraint::Length(4)])
        .split(inner);
    text(
        frame,
        header[0],
        vec![line(
            format!(
                "  {} — {}  ·  {} talent point{} in hand",
                game.player.class.name(),
                game.player.class.calling(),
                available,
                if available == 1 { "" } else { "s" }
            ),
            GOLD,
        )],
    );
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(34),
            Constraint::Percentage(33),
            Constraint::Percentage(33),
        ])
        .split(header[1]);
    for (branch_index, branch) in tree.branches.iter().enumerate() {
        let mut lines = vec![line(format!("  {}", branch.name), SEA)];
        for (tier, node_def) in branch.nodes.iter().enumerate() {
            let index = (branch_index * 4 + tier) as u8;
            let rank = game.player.talent_rank(index);
            let paid = rank >= node_def.cost;
            let tier_level = u32::from(node_def.level);
            let predecessor_met =
                tier == 0 || game.player.talent_rank(index - 1) >= branch.nodes[tier - 1].cost;
            let status = if paid {
                "held".to_string()
            } else if rank > 0 {
                format!("{}/{}", rank, node_def.cost)
            } else if game.player.level < tier_level {
                format!("L{tier_level}")
            } else if !predecessor_met {
                "needs prior".to_string()
            } else if node_def.cost == 2 {
                "2 pts".to_string()
            } else {
                "1 pt".to_string()
            };
            let gated = !paid && (game.player.level < tier_level || !predecessor_met);
            let marker = if selected == index as usize { ">" } else { " " };
            let color = if paid {
                GOLD
            } else if gated {
                MUTED
            } else if selected == index as usize {
                INK
            } else {
                SEA
            };
            lines.push(line(
                format!(
                    " {marker}{:<19} {:<11}",
                    node_def.name,
                    format!("L{:>2} {status}", node_def.level)
                ),
                color,
            ));
        }
        text(frame, cols[branch_index], lines);
    }
    let branch = selected / 4;
    let tier = selected % 4;
    let node_def = &tree.branches[branch].nodes[tier];
    text(
        frame,
        header[2],
        vec![
            line(format!("  {}: {}", node_def.name, node_def.text), INK),
            line(
                "  Points: 1 per level (L2–L10) + 1 per sigil. Enter learn / T or Esc close.",
                MUTED,
            ),
        ],
    );
}

fn draw_title(frame: &mut Frame, area: Rect, game: &Game) {
    let inner = modal(frame, area, 72, 24, "AN ATLAS OF CHOICES");
    let lines = vec![
        line("", INK),
        line("                 J E V   R E A L M S", GOLD),
        line("", INK),
        line(
            "       Explore the realm. Read the minds that shape it.",
            Color::White,
        ),
        line("", INK),
        line("  A living world; every judgment leaves a trace.", INK),
        line(
            "  Three cities. Seven dungeon floors. Four lords to face.",
            MUTED,
        ),
        line("", INK),
        line("  FIRST LIGHT / MILLBROOK", SEA),
        line(
            "  Find the nearby vendor V. Press E to hear of cellar rats.",
            INK,
        ),
        line(
            "  Buy provisions, earn your seals, and claim three Sigils.",
            INK,
        ),
        line(
            "  The Final Trial will judge the traveller you become.",
            MUTED,
        ),
        line("", INK),
        line("  [ Enter ]  Begin journey       [ ? ]  Field guide", GOLD),
        line(
            if std::path::Path::new(&format!("saves/journey-{}.json", game.seed)).exists() {
                "  [ L ]      Continue saved journey  [ Q ]  Leave"
            } else {
                "  [ Q ]      Leave"
            },
            MUTED,
        ),
        line("", INK),
        line(format!("  {} / seed {}", ai_label(game), game.seed), SEA),
        line(
            "  One save per seed. Death costs gold, not the world.",
            MUTED,
        ),
        line("  100x40 recommended / resizable / keyboard only", MUTED),
    ];
    let skip = if inner.height < 18 { 7 } else { 0 };
    text(frame, inner, lines.into_iter().skip(skip).collect());
}

fn draw_help(frame: &mut Frame, area: Rect) {
    let inner = modal(frame, area, 78, 30, "FIELD GUIDE / Esc closes");
    let lines = vec![
        line("THE ROAD", GOLD),
        line("Arrows / WASD   Move; bump a hostile to attack", INK),
        line(
            "Home/PgUp/End/PgDn or numpad 1..9   Eight-way movement",
            INK,
        ),
        line(
            "E interact   Space wait   R rest   Enter / Shift+A attack",
            INK,
        ),
        line("F defend   X flee   C offer mercy   I use/equip items", INK),
        line("Combat: one fresh press per action; time waits for you.", INK),
        line("", INK),
        line("THE SATCHEL", GOLD),
        line(
            "I inventory   B journal   M atlas   J decision inspector   Y runes",
            INK,
        ),
        line(
            "Gear lists show +ATK/-ATK or +DEF/-DEF against your equipped tier.",
            INK,
        ),
        line(
            "Sellswords join for 50g; smiths forge pairs into the next tier.",
            INK,
        ),
        line(
            "Guard captains post seeded bounties: culls and parcel runs.",
            INK,
        ),
        line(
            "Esc->S saves & quits; L on the title restores (one slot/seed).",
            INK,
        ),
        line(
            "Menus: arrows / W,S select; Enter confirms; Esc closes",
            INK,
        ),
        line("Trade: H haggle   Inventory: V sells beside a vendor", INK),
        line(
            "Atlas: 1 Millbrook, 2 Highgate, 3 Saltmarsh (visited only)",
            INK,
        ),
        line("Inspector: [ and ] browse the last ten decisions", INK),
        line("? help   Esc pause   Q quits only from pause/title", INK),
        line("", INK),
        line("THE LONG WAY HOME", GOLD),
        line(
            "Ask Millbrook's vendor about rats. Help Highgate's caravan.",
            INK,
        ),
        line(
            "Choose a side in Saltmarsh. Buy dungeon seals and better gear.",
            INK,
        ),
        line("Burrow (2 floors), Crimson Hollow (2), Underkeep (3):", INK),
        line(
            "defeat each lord, take the three Sigils, enter the Final Trial.",
            INK,
        ),
        line("", INK),
        line("READING THE MAP", GOLD),
        line(
            "@ you  V vendor  G guard  ? oracle  b bandit  w wolf  s undead",
            INK,
        ),
        line(
            "♣ forest  ♠ deep forest  ▲ mountain  ≈ river  - ford  & ruins",
            INK,
        ),
        line(
            "< up  > down  + door  $ loot  O shrine / red rim: combat",
            INK,
        ),
        line(
            "Dim tiles are remembered. A lit torch reaches six at night.",
            MUTED,
        ),
        line(
            "Laya live or seeded offline judgments never block your turn.",
            MUTED,
        ),
        line(
            "Save: Esc then S. Death returns you to a city; lose 25% gold.",
            MUTED,
        ),
    ];
    text(frame, inner, lines);
}

fn menu(frame: &mut Frame, area: Rect, rows: Vec<ListItem<'_>>, selected: usize) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let selected = if rows.is_empty() {
        None
    } else {
        Some(selected.min(rows.len() - 1))
    };
    let list = List::new(rows).highlight_style(
        Style::default()
            .fg(Color::Black)
            .bg(GOLD)
            .add_modifier(Modifier::BOLD),
    );
    let mut state = ListState::default().with_selected(selected);
    frame.render_stateful_widget(list, area, &mut state);
}

fn menu_sections(inner: Rect, top: u16, bottom: u16) -> std::rc::Rc<[Rect]> {
    Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(top),
            Constraint::Min(1),
            Constraint::Length(bottom),
        ])
        .split(inner)
}

fn draw_inventory(frame: &mut Frame, area: Rect, game: &Game) {
    let inner = modal(frame, area, 68, 28, "SATCHEL / inventory");
    let rows = menu_sections(inner, 2, 4);
    text(
        frame,
        rows[0],
        vec![
            line(
                format!(
                    "{}/20 slots / {} gold",
                    game.player.inventory.len(),
                    game.player.gold
                ),
                GOLD,
            ),
            line("Arrows/W,S select / Enter use or equip / Esc close", MUTED),
        ],
    );
    if game.player.inventory.is_empty() {
        text(
            frame,
            rows[1],
            vec![line("Your satchel is empty. Find a city vendor V.", MUTED)],
        );
    } else {
        let items = game
            .player
            .inventory
            .iter()
            .map(|item| match gear_note(game, item) {
                Some((note, color)) => ListItem::new(Line::from(vec![
                    Span::styled(item.name(), Style::default().fg(INK)),
                    Span::styled(format!("  [{note}]"), Style::default().fg(color)),
                ])),
                None => ListItem::new(Line::from(vec![Span::styled(
                    item.name(),
                    Style::default().fg(INK),
                )])),
            })
            .collect();
        menu(frame, rows[1], items, game.selected);
    }
    let selected = game.player.inventory.get(game.selected);
    let advice = match selected {
        Some(Item::Potion) => "Enter: drink to restore 18 HP; consumes a combat turn.",
        Some(Item::Ration) => "Enter: eat to restore HP and stamina.",
        Some(Item::Torch) => "Enter: light for six-tile night vision.",
        Some(Item::Weapon(_)) | Some(Item::Armour(_)) => {
            "Enter: equip. Better gear can turn a losing fight."
        }
        Some(Item::BossRelic(relic)) => relic.describe(),
        Some(Item::Key(_)) => "A permanent dungeon permission; opens its entrance.",
        Some(Item::CaravanGoods) => "Return the goods in Highgate, or consider the fence.",
        Some(Item::Contraband) => "A Saltmarsh choice: report to the watch, or smuggle.",
        Some(Item::Relic) => "Ask the oracle to identify this relic.",
        Some(Item::Delivery(_)) => "A bounty parcel: deliver it to the named city's guard captain.",
        Some(Item::AntiToxin) => "Enter: purge fester and steel yourself against new bites.",
        Some(Item::ManaTonic) => "Enter: drink to recover 4 mana mid-fight.",
        Some(Item::Essence) => "Enter: unswear your gifts — every point returns to your hand.",
        Some(Item::FirstWrit) => "Enter: the unwinding — or carry it past the Trial for the arbiter's question.",
        Some(Item::GreaterPotion) => "Enter: drink to restore 32 HP; consumes a combat turn.",
        Some(Item::TravelerRation) => "Enter: eat to restore 12 HP and all stamina.",
        Some(Item::GemDust) | Some(Item::HerbCluster) | Some(Item::OreFlake) | Some(Item::GlyphShard) => {
            "Alchemist's matter: try a transmute at any still."
        }
        Some(Item::Glyph(_)) => "A seal-glyph fragment: a smith's socket-punch (75g) writes it into plain gear.",
        None => "Restock before descending into the dungeons.",
    };
    let sell = selected.map(social::sell_price).unwrap_or(0);
    text(
        frame,
        rows[2],
        vec![
            line(advice, SEA),
            line(
                if sell > 0 {
                    format!("V: sell beside a vendor for {} gold", sell)
                } else {
                    "This item is not offered for sale.".into()
                },
                MUTED,
            ),
            line(
                "Equipment and consumables use your turn during combat.",
                MUTED,
            ),
        ],
    );
}

fn draw_trade(frame: &mut Frame, area: Rect, game: &Game, id: usize) {
    let Some(npc) = game.npcs.get(id) else { return };
    let inner = modal(frame, area, 70, 28, &format!("{} / provisions", npc.name));
    let rows = menu_sections(inner, 3, 4);
    let city = match game.maps.get(npc.map).map(|map| map.kind) {
        Some(MapKind::City(city)) => city,
        _ => game.player.last_city,
    };
    let haggle = match game.haggle {
        Some((vendor, discount)) if vendor == id => format!("Negotiated reduction: {}%", discount),
        _ if npc.decision_pending => "Judgment pending; trading stays responsive.".into(),
        _ => "H: ask for a better price".into(),
    };
    text(
        frame,
        rows[0],
        vec![
            line(
                format!(
                    "{} gold / {}/20 satchel slots",
                    game.player.gold,
                    game.player.inventory.len()
                ),
                GOLD,
            ),
            line(
                "Enter buy / Q work & talk / H haggle / I bag / Esc leave",
                MUTED,
            ),
            line(haggle, SEA),
        ],
    );
    let items = social::stock(city)
        .into_iter()
        .map(|(item, base)| {
            let price = social::buy_price(game, id, base);
            let owned_key = matches!(item, Item::Key(key) if game.player.keys.get(key).copied().unwrap_or(false));
            let mut spans = vec![Span::styled(
                format!("{:>4}g  {}", price, item.name()),
                Style::default().fg(if price > game.player.gold { MUTED } else { INK }),
            )];
            if owned_key {
                spans.push(Span::styled("  [on keyring]", Style::default().fg(MUTED)));
            } else if let Some((note, color)) = gear_note(game, &item) {
                spans.push(Span::styled(format!("  [{note}]"), Style::default().fg(color)));
            }
            ListItem::new(Line::from(spans))
        })
        .collect();
    menu(frame, rows[1], items, game.selected);
    text(
        frame,
        rows[2],
        vec![
            line(
                "Listed prices include your reputation and negotiation.",
                MUTED,
            ),
            line("Stock replenishes; your satchel holds 20 items.", MUTED),
            line("To sell: Esc, I inventory, select an item, then V.", SEA),
            line(
                "Dungeon seals unlock entry; they do not grant a Sigil.",
                MUTED,
            ),
        ],
    );
}

fn draw_talk(frame: &mut Frame, area: Rect, game: &Game, id: usize) {
    let Some(npc) = game.npcs.get(id) else { return };
    let inner = modal(frame, area, 72, 23, &npc.name);
    let rows = menu_sections(inner, 5, 3);
    let relation = social::conversation_regard(npc);
    text(
        frame,
        rows[0],
        vec![
            line(
                format!("{} · {} regard", social::conversation_role(npc), relation),
                SEA,
            ),
            line(
                if npc.decision_pending {
                    "A judgment is pending. The world does not wait for it."
                } else {
                    "They remember your actions. Choose a response."
                },
                MUTED,
            ),
            line("", INK),
            line("Arrows/W,S select / Enter speak / Esc leave", GOLD),
        ],
    );
    let choices = social::talk_options(game, id)
        .into_iter()
        .map(|choice| {
            let mut lines = Vec::new();
            let mut row = String::new();
            let width = usize::from(rows[1].width.saturating_sub(2)).max(1);
            for word in choice.split_whitespace() {
                if !row.is_empty() && row.chars().count() + word.chars().count() + 1 > width {
                    lines.push(Line::raw(std::mem::take(&mut row)));
                }
                if !row.is_empty() { row.push(' '); }
                row.push_str(word);
            }
            lines.push(Line::raw(row));
            ListItem::new(lines).style(Style::default().fg(INK))
        })
        .collect();
    menu(frame, rows[1], choices, game.selected);
    let latest = game
        .log
        .back()
        .map(String::as_str)
        .unwrap_or("J opens the ledger of observable NPC judgments.");
    frame.render_widget(
        Paragraph::new(latest)
            .style(Style::default().fg(MUTED))
            .wrap(Wrap { trim: true }),
        rows[2],
    );
}

fn stage(stage: QuestStage) -> (&'static str, Color) {
    match stage {
        QuestStage::Locked => ("LOCKED", MUTED),
        QuestStage::Available => ("AVAILABLE", SEA),
        QuestStage::Active => ("ACTIVE", GOLD),
        QuestStage::Ready => ("RETURN", Color::LightGreen),
        QuestStage::Complete => ("COMPLETE", Color::Green),
    }
}

fn draw_journal(frame: &mut Frame, area: Rect, game: &Game) {
    let inner = modal(frame, area, 78, 28, "THE SECOND WATCH / quest journal");
    let sections = menu_sections(inner, 2, 7);
    let (act_n, act_name) = social::act_of(game.player.map);
    text(
        frame,
        sections[0],
        vec![
            line(
                format!("Act {act_n} — {act_name}   ·   Arrows/W,S select / Esc closes", act_n = roman(act_n), act_name = act_name),
                GOLD,
            ),
            line(
                "Progress survives death. Save from the pause menu before quitting.",
                MUTED,
            ),
        ],
    );
    let quests = game
        .quests
        .iter()
        .enumerate()
        .map(|(index, quest)| {
            let (status, color) = stage(quest.stage);
            ListItem::new(vec![
                line(format!("{}. {}", index + 1, quest.title), color),
                line(
                    format!("   {}  {}/{}", status, quest.progress, quest.goal),
                    MUTED,
                ),
                Line::raw(""),
            ])
        })
        .collect();
    menu(frame, sections[1], quests, game.selected);
    if let Some(quest) = game
        .quests
        .get(game.selected.min(game.quests.len().saturating_sub(1)))
    {
        let (status, color) = stage(quest.stage);
        let mut detail = vec![
            line(format!("{} / {}", quest.title, status), color),
            line(quest.description.clone(), INK),
            Line::raw(""),
            line(
                "Three lords hold Sigils. The fourth waits in the Final Trial; a fifth stirs beneath it.",
                MUTED,
            ),
        ];
        // E7 (§6.5): the orders' trials ride the journal beside the threads.
        let trial_rows: Vec<(String, Color)> = game
            .trials
            .iter()
            .enumerate()
            .filter(|(_, t)| !matches!(t.stage, TrialStage::Locked))
            .map(|(i, t)| {
                let name = [
                    "The Manual Pages",
                    "Read Wrong on Purpose",
                    "The Old Truce",
                    "The Charter of the Tide",
                    "The Last Watch's Name",
                    "The Oathroad",
                ][i];
                let (tag, color) = match t.stage {
                    TrialStage::Offered => ("OFFERED", SEA),
                    TrialStage::Active => ("ACTIVE", GOLD),
                    TrialStage::Done => ("MASTERED", Color::Green),
                    TrialStage::Locked => unreachable!(),
                };
                (format!("  {} — {tag}", name, tag = tag), color)
            })
            .collect();
        if !trial_rows.is_empty() {
            detail.push(Line::raw(""));
            detail.push(line("THE ORDERS' TRIALS", GOLD));
            for (row, color) in trial_rows {
                detail.push(line(row, color));
            }
        }
        // E7 (D32): the codex remembers every recipe the still taught.
        if !game.codex.is_empty() {
            detail.push(Line::raw(""));
            detail.push(line("CODEX — learned transmutes", GOLD));
            for recipe in &game.codex {
                detail.push(line(format!("  · {recipe}"), INK));
            }
        }
        frame.render_widget(
            Paragraph::new(detail).wrap(Wrap { trim: true }),
            sections[2],
        );
    }
}

fn roman(n: u8) -> &'static str {
    ["—", "I", "II", "III", "IV", "V", "VI"]
        .get(n as usize)
        .copied()
        .unwrap_or("·")
}

fn draw_cast(frame: &mut Frame, area: Rect, game: &Game) {
    let inner = modal(frame, area, 66, 22, "RUNIC ARTS / Y closes");
    let rows = menu_sections(inner, 3, 4);
    text(
        frame,
        rows[0],
        vec![
            line(
                format!(
                    "Mana {}/{} — casting in battle consumes your action's turn.",
                    game.player.mana, game.player.max_mana
                ),
                GOLD,
            ),
            line("Arrows/W,S select / Enter cast / Esc close", MUTED),
            line("", INK),
        ],
    );
    if game.player.spells.is_empty() {
        text(
            frame,
            rows[1],
            vec![
                line("No runes learned yet.", INK),
                line(
                    "Study with any city's shrine oracle (near ? by the shrine).",
                    MUTED,
                ),
            ],
        );
    } else {
        let items = game
            .player
            .spells
            .iter()
            .map(|spell| {
                let cost = game.spell_cost(*spell);
                ListItem::new(format!("{:<14} {} mana", spell.name(), cost))
                    .style(Style::default().fg(if cost > game.player.mana { MUTED } else { INK }))
            })
            .collect();
        menu(frame, rows[1], items, game.selected);
    }
    let advice: String = match game.player.spells.get(game.selected) {
        Some(spell) => spell.describe().into(),
        None => "Sparks and wards need a live enemy; mend works anywhere.".into(),
    };
    text(
        frame,
        rows[2],
        vec![
            line(advice, SEA),
            line("Mana returns slowly over time and fully at an inn.", MUTED),
        ],
    );
}

fn draw_forge(frame: &mut Frame, area: Rect, game: &Game, id: usize) {
    let Some(npc) = game.npcs.get(id) else { return };
    let inner = modal(frame, area, 70, 24, &format!("{} / forge", npc.name));
    let rows = menu_sections(inner, 3, 5);
    text(
        frame,
        rows[0],
        vec![
            line(
                format!(
                    "{} gold. Trade two matching pieces plus coin for the next tier.",
                    game.player.gold
                ),
                GOLD,
            ),
            line("Arrows/W,S select / Enter forge / Esc leave", MUTED),
            line("", INK),
        ],
    );
    let options = game.forge_options();
    if options.is_empty() {
        text(
            frame,
            rows[1],
            vec![
                line("Nothing to forge.", INK),
                line(
                    "Bring two swords or two armours of equal tier (worn, standard or fine).",
                    MUTED,
                ),
            ],
        );
    } else {
        let items = options
            .iter()
            .map(|(item, cost)| {
                let (parts, tier) = match item {
                    Item::Weapon(t) => ("swords", t - 1),
                    Item::Armour(t) => ("armours", t - 1),
                    _ => unreachable!("forge yields only gear"),
                };
                ListItem::new(format!(
                    "{:>4}g  {}  (2x {} {})",
                    cost,
                    item.name(),
                    Item::tier(tier).to_lowercase(),
                    parts
                ))
                .style(Style::default().fg(if *cost > game.player.gold {
                    MUTED
                } else {
                    INK
                }))
            })
            .collect();
        menu(frame, rows[1], items, game.selected);
    }
    text(
        frame,
        rows[2],
        vec![
            line(
                "Forged gear lands in your pack; equip it from inventory (I).",
                MUTED,
            ),
            line(
                "Duplicate loot is a resource — salvage it into masterwork steel.",
                SEA,
            ),
            line(
                "The smith remembers fair trade, not haggling, at the anvil.",
                MUTED,
            ),
        ],
    );
}

use social::city_name;

/// Braille dot bits for a 2x4 sub-cell grid: [row][column].
const BRAILLE: [[u8; 2]; 4] = [[0x01, 0x08], [0x02, 0x10], [0x04, 0x20], [0x40, 0x80]];

/// Which terrain kind wins a braille cell's shared foreground color.
fn tile_rank(tile: Tile) -> u8 {
    match tile {
        Tile::Ford => 0,
        Tile::River => 1,
        Tile::Road => 2,
        Tile::Ruins => 3,
        Tile::Shrine => 4,
        Tile::Mountain => 5,
        Tile::DeepForest => 6,
        Tile::Forest => 7,
        _ => 8,
    }
}

fn draw_atlas(frame: &mut Frame, area: Rect, game: &Game) {
    let title = if game.atlas_zoom {
        "REALM ATLAS / local chart (Z: world)"
    } else {
        "REALM ATLAS / charted lands (Z: local)"
    };
    modal(frame, area, 92, 36, title);
    let (inner, miniature) = atlas_areas(area);
    if inner.width == 0 || inner.height == 0 {
        return;
    }
    let Some(world) = game.maps.first() else {
        return;
    };
    let miniature_height = miniature.height;
    let miniature_width = miniature.width;
    // Braille dots carry 2x4 sub-pixels per cell; local view maps them 1:1 with tiles.
    let dots_w = i32::from(miniature_width) * 2;
    let dots_h = i32::from(miniature_height) * 4;
    let (origin_x, origin_y, view_w, view_h) = if game.atlas_zoom {
        let vw = dots_w.min(world.width);
        let vh = dots_h.min(world.height);
        (
            (game.player.pos.x - vw / 2).clamp(0, (world.width - vw).max(0)),
            (game.player.pos.y - vh / 2).clamp(0, (world.height - vh).max(0)),
            vw,
            vh,
        )
    } else {
        (0, 0, world.width, world.height)
    };
    if miniature.width > 0 && miniature.height > 0 {
        for cy in 0..miniature.height {
            for cx in 0..miniature.width {
                let mut bits = 0u8;
                let mut best: Option<(u8, Color)> = None;
                for dy in 0..4 {
                    for dx in 0..2 {
                        let wx = origin_x + ((i32::from(cx) * 2 + dx) * view_w) / dots_w;
                        let wy = origin_y + ((i32::from(cy) * 4 + dy) * view_h) / dots_h;
                        let tile = world.tile(Pos::new(wx, wy));
                        if matches!(tile, Tile::Rock) {
                            continue;
                        }
                        bits |= BRAILLE[dy as usize][dx as usize];
                        let candidate = (tile_rank(tile), tile_color(tile));
                        if best.is_none_or(|current| candidate.0 < current.0) {
                            best = Some(candidate);
                        }
                    }
                }
                frame.buffer_mut()[(miniature.x + cx, miniature.y + cy)]
                    .set_char(char::from_u32(0x2800 + u32::from(bits)).unwrap_or(' '))
                    .set_style(Style::default().fg(best.map_or(MUTED, |(_, c)| shade(c, 0.8))));
            }
        }
        // Overlays are cell-resolution markers over the dot map.
        let point = |pos: Pos| -> Option<(u16, u16)> {
            if pos.x < origin_x
                || pos.y < origin_y
                || pos.x >= origin_x + view_w
                || pos.y >= origin_y + view_h
            {
                return None;
            }
            let dx = (pos.x - origin_x) * dots_w / view_w;
            let dy = (pos.y - origin_y) * dots_h / view_h;
            Some((
                miniature.x + (dx / 2).min(i32::from(miniature_width) - 1) as u16,
                miniature.y + (dy / 4).min(i32::from(miniature_height) - 1) as u16,
            ))
        };
        for portal in &world.portals {
            let Some((x, y)) = point(portal.pos) else {
                continue;
            };
            let glyph = if (1..=3).contains(&portal.destination) {
                char::from(b'0' + portal.destination as u8)
            } else {
                '*'
            };
            frame.buffer_mut()[(x, y)]
                .set_char(glyph)
                .set_style(Style::default().fg(GOLD).add_modifier(Modifier::BOLD));
        }
        let location = if game.player.map == 0 {
            Some(game.player.pos)
        } else {
            world
                .portals
                .iter()
                .find(|portal| portal.destination == game.player.map)
                .map(|p| p.pos)
        };
        if let Some((x, y)) = location.and_then(point) {
            frame.buffer_mut()[(x, y)].set_char('@').set_style(
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            );
        }
    }
    let details = Rect::new(
        inner.x,
        inner.y + miniature_height,
        inner.width,
        inner.height - miniature_height,
    );
    let mut lines = vec![
        line(
            "1/2/3: travel to a discovered city / Z toggles world-local view / Esc closes",
            GOLD,
        ),
        line(
            "Travel is deliberate, never automatic on cursor movement.",
            MUTED,
        ),
    ];
    for city in 0..3 {
        let pos = world
            .portals
            .iter()
            .find(|p| p.destination == city + 1)
            .map(|p| format!("[{},{}]", p.pos.x, p.pos.y))
            .unwrap_or_default();
        lines.push(line(
            format!(
                "{} {} {} / {}",
                city + 1,
                city_name(city),
                pos,
                if game.visited[city] {
                    "discovered"
                } else {
                    "unvisited"
                }
            ),
            if game.visited[city] { SEA } else { MUTED },
        ));
    }
    for portal in world
        .portals
        .iter()
        .filter(|p| !(1..=3).contains(&p.destination))
    {
        let access = match portal.requirement {
            Some(3) => {
                if game.player.sigils.iter().all(|s| *s) {
                    "three Sigils held"
                } else {
                    "needs all three Sigils"
                }
            }
            Some(key) => {
                if game.player.keys.get(key).copied().unwrap_or(false) {
                    "seal held"
                } else {
                    "seal required"
                }
            }
            None => "open road",
        };
        lines.push(line(
            format!(
                "* {} [{},{}] / {}",
                portal.label, portal.pos.x, portal.pos.y, access
            ),
            GOLD,
        ));
    }
    lines.push(line(
        "♣/♠ woods  ▲ ridge  ≈ river  - ford  . road  * landmark",
        MUTED,
    ));
    text(frame, details, lines);
}
