use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use laya_realms::{
    input::{active, key, movement_key},
    laya_client::{apply_result, pump, AiService},
    model::*,
    ui,
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{
    env,
    error::Error,
    fs,
    io::{self, IsTerminal},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

type Result<T, E = Box<dyn Error>> = std::result::Result<T, E>;

struct Options {
    seed: u64,
    ai: Option<String>,
    view: String,
    gui_screenshot: Option<String>,
    smoke: bool,
    ai_smoke: usize,
    benchmark: bool,
}
fn options() -> Result<Option<Options>> {
    let mut opts = Options {
        seed: SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos() as u64,
        ai: None,
        // E6: the Bevy window is the default when it is compiled in; the
        // terminal stays one --view tui away (gui is windows-only parity).
        view: if cfg!(feature = "bevy-view") {
            "bevy".into()
        } else {
            "tui".into()
        },
        gui_screenshot: None,
        smoke: false,
        ai_smoke: 0,
        benchmark: false,
    };
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--seed" => {
                opts.seed = args
                    .next()
                    .ok_or("--seed needs an unsigned integer")?
                    .parse()?
            }
            "--ai" => {
                let mode = args.next().ok_or("--ai needs laya or heuristics")?;
                if !matches!(mode.as_str(), "laya" | "heuristics") {
                    return Err("--ai must be laya or heuristics".into());
                }
                opts.ai = Some(mode);
            }
            "--view" => {
                let view = args.next().ok_or("--view needs tui, gui or bevy")?;
                if !matches!(view.as_str(), "tui" | "gui" | "bevy") {
                    return Err("--view must be tui, gui or bevy".into());
                }
                opts.view = view;
            }
            "--gui-screenshot" => {
                opts.gui_screenshot = Some(args.next().ok_or("--gui-screenshot needs a path")?);
            }
            "--smoke" => opts.smoke = true,
            "--ai-smoke" => {
                opts.ai_smoke = args
                    .next()
                    .ok_or("--ai-smoke needs a decision count")?
                    .parse()?;
                if !(1..=50).contains(&opts.ai_smoke) {
                    return Err("--ai-smoke count must be 1..50".into());
                }
            }
            "--benchmark" => opts.benchmark = true,
            "--help" | "-h" => {
                println!("LAYA REALMS — terminal and windowed RPG\n\ncargo run --release -- [--seed N] [--ai laya|heuristics] [--view tui|gui]\n\nAI defaults to Laya when the local Laya gateway (LAYA_GATEWAY_URL) answers /health.\nNo credential is needed for --ai heuristics.\nSave and quit with Esc then S; load with L on the title screen using the same seed.\nSaves: saves/journey-<seed>.json (one slot per seed).\n\n--view tui|gui|bevy  Choose terminal, windowed (macroquad), or Bevy view\n                  (with the bevy-view feature compiled in, bevy is the default)\n--gui-screenshot PATH\n                  Capture the GUI to PNG and exit; requires --view gui\n                  and cannot be combined with headless checks\n--smoke           Run a deterministic world/input/rendering smoke check\n--ai-smoke N      Evaluate N real NPC decisions (1..50), report usage/latency\n--benchmark      Measure 10,000 simulation ticks with the populated world\nHeadless checks run instead of either view (priority: smoke, benchmark, ai-smoke).\n\nIn game: arrows/WASD move, Home/PgUp/End/PgDn diagonals, E interact,\nI inventory, B journal, M map, J decisions, ? help, Esc pause.\nRecommended terminal: 100 columns x 40 rows.\n");
                return Ok(None);
            }
            _ => return Err(format!("Unknown option {arg}; use --help").into()),
        }
    }
    if opts.gui_screenshot.is_some() {
        if opts.view != "gui" {
            return Err("--gui-screenshot requires --view gui".into());
        }
        if opts.smoke || opts.benchmark || opts.ai_smoke > 0 {
            return Err(
                "--gui-screenshot cannot be combined with --smoke, --benchmark, or --ai-smoke"
                    .into(),
            );
        }
    }
    Ok(Some(opts))
}

fn credential() -> Option<String> {
    env::var("LAYA_GATEWAY_TOKEN")
        .ok()
        .filter(|key| !key.trim().is_empty())
        .or_else(|| fs::read_to_string("laya_key.txt").ok())
        .map(|key| key.trim().to_owned())
        .filter(|key| !key.is_empty())
}

// Drop also runs on input/draw errors; the panic hook covers aborting terminal sessions.
struct TerminalGuard;
impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(
            io::stdout(),
            LeaveAlternateScreen,
            event::DisableMouseCapture,
            crossterm::cursor::Show
        );
    }
}

#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() -> Result<()> {
    let Some(opts) = options()? else {
        return Ok(());
    };
    if opts.smoke {
        return smoke(opts.seed);
    }
    if opts.benchmark {
        return benchmark(opts.seed);
    }
    let key_value = credential();
    let probed = laya_realms::laya_client::gateway_ready().await;
    let mode = opts
        .ai
        .as_deref()
        .unwrap_or(if probed { "laya" } else { "heuristics" });
    let missing = mode == "laya" && !probed;
    let effective = if missing { "heuristics" } else { mode };
    if opts.ai_smoke > 0 {
        let mut ai = AiService::new(effective, key_value, opts.seed).map_err(io::Error::other)?;
        return ai_smoke(opts.seed, opts.ai_smoke, &mut ai, effective).await;
    }
    if opts.view == "gui" {
        laya_realms::gui::launch(
            opts.seed,
            effective,
            key_value,
            missing,
            opts.gui_screenshot,
        )
        .map_err(io::Error::other)?;
        return Ok(());
    }
    #[cfg(feature = "bevy-view")]
    if opts.view == "bevy" {
        let mut game = Game::new(opts.seed);
        game.ai.provider = effective.into();
        if missing {
            game.ai.switched = true;
            game.log("No local gateway key found; using offline decisions.");
        }
        game.log(format!(
            "World seed: {}. {} decisions. WASD or click to walk; Esc for the campfire.",
            opts.seed, effective
        ));
        // E6: the window owns the full Title → Create flow (no pre-creation
        // bypass): the game boots on its Title modal like the terminal does.
        realms_view::launch(&mut game);
        return Ok(());
    }
    #[cfg(not(feature = "bevy-view"))]
    if opts.view == "bevy" {
        return Err(
            "The bevy window needs its feature: cargo run --release --features bevy-view -- --view bevy"
                .into(),
        );
    }
    let mut ai = AiService::new(effective, key_value, opts.seed).map_err(io::Error::other)?;
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return Err("Interactive play needs a terminal. Run cargo run --release in Windows Terminal/Warp, or use --smoke.".into());
    }
    let mut game = Game::new(opts.seed);
    game.ai.provider = effective.into();
    if missing {
        game.ai.switched = true;
        game.log("No local gateway key found; using offline decisions.");
    }
    game.log(format!(
        "World seed: {}. {} decisions. One save slot per seed.",
        opts.seed, effective
    ));
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen, crossterm::cursor::Show);
        previous(info);
    }));
    enable_raw_mode()?;
    let _guard = TerminalGuard;
    execute!(io::stdout(), EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;
    terminal.hide_cursor()?;
    let mut last = Instant::now();
    let mut simulation = Duration::ZERO;
    let mut frame_at = Instant::now();
    let mut buffered_move = None;
    let mut elapsed = Duration::ZERO;
    let mut dirty = true;
    while !game.quit {
        let now = Instant::now();
        let delta = now.duration_since(last).min(Duration::from_millis(100));
        last = now;
        if active(&game) {
            elapsed += delta;
            game.elapsed_ms = elapsed.as_millis() as u64;
            simulation += delta;
            while simulation >= Duration::from_millis(250) {
                game.tick_world();
                dirty = true;
                simulation -= Duration::from_millis(250);
                if !active(&game) {
                    simulation = Duration::ZERO;
                    break;
                }
            }
        } else {
            simulation = Duration::ZERO;
        }
        let before_ai = (game.ai.requests, game.ai.pending);
        pump(&mut game, &mut ai);
        dirty |= before_ai != (game.ai.requests, game.ai.pending);
        if !active(&game) || game.combat.is_some() {
            buffered_move = None;
        }
        if active(&game) && game.elapsed_ms >= game.move_ready_ms {
            if let Some(event) = buffered_move.take() {
                key(&mut game, event);
                dirty = true;
            }
        }
        // Bound input draining: a held key must never starve the frame or AI queue.
        for _ in 0..16 {
            if !event::poll(Duration::ZERO)? {
                break;
            }
            match event::read()? {
                Event::Key(event) if event.kind != KeyEventKind::Release => {
                    dirty = true;
                    if active(&game)
                        && movement_key(event.code)
                        && game.combat.is_none()
                        && game.elapsed_ms < game.move_ready_ms
                    {
                        buffered_move = Some(event);
                    } else {
                        buffered_move = None;
                        key(&mut game, event);
                    }
                }
                Event::Resize(_, _) => {
                    frame_at = Instant::now();
                    dirty = true;
                }
                _ => {}
            }
        }
        if dirty && Instant::now() >= frame_at {
            terminal.draw(|frame| ui::draw(frame, &game))?;
            dirty = false;
            frame_at = Instant::now() + Duration::from_millis(16);
        }
        std::thread::sleep(Duration::from_millis(4));
    }
    Ok(())
}

fn smoke(seed: u64) -> Result<()> {
    use laya_realms::laya_client::heuristic;
    use ratatui::backend::TestBackend;
    let mut game = Game::new(seed);
    let start = game.player.pos;
    game.modal = Modal::None;
    for dx in [-1, 1] {
        game.elapsed_ms += 1000;
        game.action(Action::Move(dx, 0));
    }
    assert!(game.map().tile(game.player.pos).walkable());
    let request_npc = game
        .npcs
        .iter()
        .position(|npc| npc.alive() && npc.map == game.player.map)
        .ok_or("no town NPC")?;
    game.request_decision(request_npc, 2);
    for request in std::mem::take(&mut game.outbox) {
        let result = heuristic(&request);
        assert!(!result.answers.is_empty());
        let rule = game.apply_decision(&result);
        game.decisions.push_back(DecisionRecord {
            result,
            applied_rule: rule,
        });
    }
    game.inspector = true;
    for size in [(120, 44), (80, 30), (40, 15), (24, 10), (10, 5)] {
        let mut terminal = Terminal::new(TestBackend::new(size.0, size.1))?;
        let vendor = game
            .npcs
            .iter()
            .position(|n| n.archetype == Archetype::Vendor)
            .ok_or("no vendor")?;
        let smith = game
            .npcs
            .iter()
            .position(|n| n.archetype == Archetype::Vendor && n.name.contains("Blacksmith"))
            .ok_or("no smith")?;
        for modal in [
            Modal::None,
            Modal::Title,
            Modal::Create,
            Modal::Help,
            Modal::Pause,
            Modal::Inventory,
            Modal::Journal,
            Modal::Atlas,
            Modal::Trade(vendor),
            Modal::Talk(vendor),
            Modal::Cast,
            Modal::Forge(smith),
            Modal::Oath,
            Modal::Talents,
            Modal::Death,
            Modal::Victory,
        ] {
            game.modal = modal;
            terminal.draw(|frame| ui::draw(frame, &game))?;
        }
        // The zoomed local chart has its own sampling path through the atlas.
        game.modal = Modal::Atlas;
        game.atlas_zoom = true;
        terminal.draw(|frame| ui::draw(frame, &game))?;
        game.atlas_zoom = false;
    }
    game.modal = Modal::None;
    let before = game.player.pos;
    key(
        &mut game,
        KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE),
    );
    assert!(matches!(game.modal, Modal::Inventory));
    key(&mut game, KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    assert!(active(&game));
    assert_eq!(before, game.player.pos);
    println!("SMOKE OK seed={seed}: {} maps, {} NPCs, movement {:?}->{:?}, decisions, input, modal/resize render paths",game.maps.len(),game.npcs.len(),start,game.player.pos);
    Ok(())
}
fn benchmark(seed: u64) -> Result<()> {
    let mut game = Game::new(seed);
    game.modal = Modal::None;
    let start = Instant::now();
    for _ in 0..10_000 {
        game.elapsed_ms += 250;
        game.tick_world();
        for request in std::mem::take(&mut game.outbox) {
            let result = laya_realms::laya_client::heuristic(&request);
            game.apply_decision(&result);
        }
    }
    let elapsed = start.elapsed();
    println!("SIMULATION seed={seed} NPCs={} ticks=10000 elapsed_ms={:.2} mean_tick_us={:.2} single_core_at_4hz={:.3}%",game.npcs.len(),elapsed.as_secs_f64()*1000.0,elapsed.as_secs_f64()*100.0,elapsed.as_secs_f64()/2500.0*100.0);
    Ok(())
}
async fn ai_smoke(seed: u64, count: usize, ai: &mut AiService, mode: &str) -> Result<()> {
    let mut game = Game::new(seed);
    game.ai.provider = mode.into();
    game.modal = Modal::None;
    let mut ids: Vec<_> = game
        .npcs
        .iter()
        .filter(|n| {
            matches!(
                n.archetype,
                Archetype::Bandit | Archetype::Vendor | Archetype::Lich | Archetype::Adjudicator
            )
        })
        .map(|n| n.id)
        .collect();
    ids.sort_by_key(|id| (!game.npcs[*id].archetype.boss(), *id));
    let mut latencies = Vec::new();
    let mut live = 0;
    let mut failures = 0;
    let mut tokens = 0;
    for id in ids.iter().copied().cycle().take(count) {
        game.player.map = game.npcs[id].map;
        game.player.pos = game.npcs[id].pos.offset(1, 0);
        game.request_decision(
            id,
            if game.npcs[id].archetype.boss() {
                3
            } else if game.npcs[id].archetype.hostile() {
                1
            } else {
                2
            },
        );
        let Some(request) = game.outbox.pop() else {
            continue;
        };
        if !ai.submit(request) {
            return Err("AI smoke queue unexpectedly full".into());
        }
        let deadline = Instant::now() + Duration::from_secs(15);
        let result = loop {
            if let Some(result) = ai.drain().into_iter().next() {
                break result;
            }
            if Instant::now() > deadline {
                return Err("AI worker did not complete within 15 seconds".into());
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        };
        tokens += result.input_tokens;
        latencies.push(result.latency_ms);
        if result.provider == "laya" {
            live += 1;
        }
        if result.error.is_some() {
            failures += 1;
        }
        println!(
            "npc={} provider={} questions={} latency_ms={} error={}",
            result.request.npc_name,
            result.provider,
            result.answers.len(),
            result.latency_ms,
            result.error.as_deref().unwrap_or("none")
        );
        apply_result(&mut game, ai, result);
        if failures >= 5 {
            assert!(
                game.ai.switched && ai.is_heuristic(),
                "five failures must switch the service and HUD together"
            );
            println!(
                "Verified: persistent HUD and service switched to heuristics after five failures."
            );
            break;
        }
    }
    latencies.sort_unstable();
    let p95 = latencies
        .get((latencies.len() * 95).div_ceil(100).saturating_sub(1))
        .copied()
        .unwrap_or(0);
    println!("AI REPORT decisions={} live={} fallback={} p95_ms={} input_tokens={} estimated_input_cost_usd={:.8} tier1_budget_met={}",latencies.len(),live,failures,p95,tokens,tokens as f64*laya_realms::laya_client::INPUT_PRICE_PER_MILLION/1_000_000.0,live>0 && p95<=150);
    if latencies.is_empty() {
        return Err("No decisions exercised".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use laya_realms::persist::{save_game, save_path, try_load};
    fn press(game: &mut Game, code: KeyCode) {
        key(game, KeyEvent::new(code, KeyModifiers::NONE));
    }

    #[test]
    fn save_round_trip_restores_the_journey() {
        let seed = 9_999_999_999;
        let mut game = Game::new(seed);
        game.modal = Modal::None;
        game.player.gold = 123;
        game.player.level = 4;
        game.quests[0].stage = QuestStage::Ready;
        game.player.inventory.push(Item::Key(0));
        game.player.spells = vec![Spell::Spark];
        game.player.max_mana = 4;
        game.player.relic = Some(BossRelic::Graveglass);
        assert!(save_game(&game).is_ok());
        let mut fresh = Game::new(seed);
        try_load(&mut fresh);
        assert_eq!(fresh.player.gold, 123);
        assert_eq!(fresh.player.level, 4);
        assert_eq!(fresh.quests[0].stage, QuestStage::Ready);
        assert!(fresh.player.inventory.contains(&Item::Key(0)));
        assert_eq!(fresh.player.spells, vec![Spell::Spark]);
        assert_eq!(fresh.player.relic, Some(BossRelic::Graveglass));
        assert_eq!(fresh.npcs.len(), game.npcs.len());
        assert!(active(&fresh));
        assert_eq!(fresh.move_ready_ms, 0, "movement gate rebase on load");
        let mut legacy = serde_json::to_value(&game).unwrap();
        legacy["player"].as_object_mut().unwrap().remove("relic");
        assert_eq!(
            serde_json::from_value::<Game>(legacy).unwrap().player.relic,
            None,
            "pre-relic saves should default to an empty slot"
        );
        let _ = fs::remove_file(save_path(seed));
    }

    #[test]
    fn keyboard_reaches_vendor_work_and_death_confirmation_respawns() {
        let mut game = Game::new(42);
        // Title -> creation (order, body build, boon) -> the road.
        press(&mut game, KeyCode::Enter);
        press(&mut game, KeyCode::Enter);
        press(&mut game, KeyCode::Enter);
        press(&mut game, KeyCode::Enter);
        assert!(active(&game));
        press(&mut game, KeyCode::Char('e'));
        for request in std::mem::take(&mut game.outbox) {
            game.apply_decision(&laya_realms::laya_client::heuristic(&request));
        }
        press(&mut game, KeyCode::Char('q'));
        press(&mut game, KeyCode::Enter);
        assert_eq!(game.quests[0].stage, QuestStage::Active);
        game.player.hp = 0;
        game.player.gold = 100;
        game.modal = Modal::Death;
        press(&mut game, KeyCode::Enter);
        assert!(active(&game));
        assert_eq!(game.player.hp, game.player.max_hp);
        assert_eq!(game.player.gold, 75);
        assert_eq!(game.quests[0].stage, QuestStage::Active);
    }
}
