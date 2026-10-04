# Roadmap — Laya Realms

The Rust terminal game is implemented. The checklist below records delivered systems; verification limits are stated separately rather than treating the original estimates as measured facts. See the [README](../README.md) for running and controls.

## M0 — Walking skeleton

- [x] Cargo project, ratatui/crossterm terminal lifecycle and error restoration
- [x] Separate 4 Hz simulation and input/render loop; movement buffering and terrain costs
- [x] Scrolling 60×30 viewport, eight-way movement, HUD, event log and help
- [x] Resizable layouts and title, pause, inventory, journal and atlas screens

**Evidence:** release executable launched and driven through a real Windows pseudo-terminal; `--smoke` exercises eleven screens at five sizes, including tiny windows.

## M1 — The world

- [x] Seeded 200×160 overworld, connected roads, two fords and northern ridge
- [x] Millbrook, Highgate and Saltmarsh interiors, day/night schedules and gates
- [x] Tutorial cellar, seven themed dungeon floors and Final Trial arena
- [x] More than 200 NPCs, local pathfinding, wandering, patrols and schedule following
- [x] Seeded personalities and capped, persistent-within-session NPC memories

**Evidence:** flood-fill regression checks portal connectivity, arrivals, actors and landmarks over seeds 0, 1, 42 and `u64::MAX`. Seed 42 starts with 258 NPCs across 13 maps.

## M2 — First live decisions

- [x] Async gateway client, per-NPC batching, three-second attempt timeout, one retry
- [x] `DecisionProvider`, live/heuristic providers, `--ai` and `--seed`
- [x] Typed question banks, strict answer validation and normalized score handling
- [x] Tactical combat: initiative, flanking, defend, fleeing, mercy and pack morale
- [x] Decision inspector, JSONL replay and latency/input-token instrumentation

**Protocol correction:** Vercel `/v1/evaluate` uses `boolean`/`probability` and camelCase usage fields; TypeSafe's direct endpoint uses `noul`/`noul`. The client translates at the wire boundary. See [Vercel's evaluation reference](https://vercel.com/docs/ai-gateway/modalities/evaluation).

**Cadence decisions:** use event triggers and a bounded eight-request service rather than adding random timing jitter. Keep personality axes as structured state, not repeated instructions. Live responses validate that representation; a controlled personality-prose comparison has not been performed.

## M3 — The social city

- [x] Guards, suspicion, bribery, night-gate permits and road patrols
- [x] City-tier stock, equipment swaps, consumables, buy/sell and negotiated prices
- [x] Night theft, recorded restitution, witnesses, rumours and reputation
- [x] Rat catch and the missing caravan, including honest return/fence outcomes
- [x] Quest journal, permanent keyring, discovered-city caravan travel

**Playtest correction:** caravan recovery accepts killed, spared, or sufficiently routed wolves. Killing every wolf is not required after choosing mercy. A regression covers both mercy and routing, while nearby retreating wolves still block collection.

## M4 — Dungeon delving

- [x] Dungeon aggression, regrouping, local leashes and pack awareness
- [x] Chief rally/sacrifice; Matriarch pack targeting and finite summons
- [x] Treasure, relics, wayshrines, gear tiers, XP/levels 1–10 and respawn
- [x] Branching smuggling and three-Sigil quest progression

**Balance:** enemies are concentrated around quest sites and dungeon rooms; overworld fights are avoidable. Boss wind-ups provide two player actions to escape marked areas. Summons are capped per encounter. City quests grant dungeon keys, with purchase alternatives; progression does not require farming respawns.

## M5 — The showcase fights

- [x] Lich pressure/summon/curse/retreat, health phases and five-turn reevaluation
- [x] Adjudicator mercy/greed/courage ratings and recent-action adaptation
- [x] Three distinct Sigils gate the arena; final boss defeat completes the campaign

**History decision:** fixed deed counters plus the last eight combat actions, within the per-NPC snapshot. This preserves recent tactical signals without sending the entire session log. A telegraphed attack retains its original target/tactic even if a new AI result arrives.

## M6 — Hardening and measurements

- [x] Per-decision fallback, five-consecutive-failure session switch and persistent HUD warning
- [x] Title/death/victory screens, complete controls, journal and inspector navigation
- [x] End-to-end action-driven campaign playthrough without stat boosts or teleporting
- [x] Live gateway smoke and observed rate-limit fallback
- [x] Deterministic heuristic tests, protocol rejection tests, combat/quest regressions
- [x] Populated-world simulation timing and current run instructions

## M7 — Post-review fixes and clarity

A full code read after M6 found no crashers or soft-locks, but sharpened several inconsistencies between what the game does and what the UI says:

- [x] **Torch fog fix:** lighting a torch at night cleared the fog of war at the daylight radius (12) while the map only drew six tiles. Fog clearing now mirrors the drawn vision radius exactly (day 12 / torch night 6 / night 4). Regression: `torch_clears_fog_only_within_its_own_radius_at_night`.
- [x] **DEFENDING indicator fix:** the HUD status could never be observed, because the engine cleared the brace flag at the end of every combat round, before any draw. The flag is now reassigned at the start of each combat action and cleared only on combat end, death, flee, or respawn. Regression: `braced_status_survives_until_the_next_action`.
- [x] **Gear comparison markers:** shop and inventory lists show each weapon/armour's `+ATK/-ATK` or `+DEF/-DEF` delta against the equipped tier, color-coded; seals already on the keyring read "on keyring".
- [x] **HUD XP progress:** shows `XP current/needed` (MAX at level 10).
- [x] **Boss wind-up countdown:** the combat sidebar states whether a telegraphed 3×3 attack resolves "in 2 actions" or "this round".
- [x] **Death attribution:** the death screen names what felled you (last damage source).
- [x] **Keyring seal purchase fix:** a full 20-slot pack blocked buying a dungeon seal even though seals never occupy a slot. Regression: `full_pack_never_blocks_a_keyring_seal_purchase`.
- [x] **Gate permit pinning fix:** a paid bribe was matched against one specific guard; guards moving between the bribe and the gate voided the permit (and could invite a second bribe). Permits now match the gate destination.
- [x] **Critical memory retention:** the 12-event NPC memory window could evict `player_spared_me` or `stole:` records, quietly un-sparing a caravan wolf or underpaying theft restitution. Ordinary events are now evicted first; the cap itself is unchanged.
- [x] Torch duration reworded ("two hours of night, about four real minutes"); help screen documents the gear markers.

## M8 — v2 feature set

Five systems the original design deferred, built under the same determinism and visibility rules:

- [x] **Saves:** the whole `Game` (world, NPCs, memories, quests, bounties, companion, runes) serializes to `saves/journey-<seed>.json` from the pause menu; title `L` restores it. Async decision plumbing is dropped on load by design. Regression: `save_round_trip_restores_the_journey`.
- [x] **Runes (magic):** Spark/Mend/Ward taught by oracles per judgment, tuition 25/45/70, mana gauge + trickle regen, casting is a real combat turn validated like items. Regressions: `study_grant_uses_the_oracles_judgment_and_sets_mana`, `spark_consumes_mana_and_strikes_past_armour`, `mend_and_ward_work_through_the_action_channel`.
- [x] **Party:** one sellsword per city (50g); follows across maps, strikes post-initiative, anchors flanks, soaks melee meant for the player, revives at half HP after combat. Regressions: `companion_hires_follows_and_dismisses`, `companion_fights_soaks_blows_and_rises_after_combat`.
- [x] **Forging:** smiths convert 2× tier gear + 25/70/150 gold into the next tier. Regression: `forge_trades_two_matching_pieces_and_coin_for_the_next_tier`.
- [x] **Bounties:** guard captains hand out seeded cull/parcel contracts (≤3 open) with level-scaled pay; progress tracks kills, delivery settles at the destination captain. Regressions: `cull_bounty_tracks_kills_and_pays_the_posted_reward`, `parcel_delivery_settles_at_the_destination_captain`.
- [x] **Terminal graphics pass:** single-width Unicode terrain textures, true-color palette, three-band light falloff, water shimmer, pulsing boss telegraphs, and a braille-resolution atlas with Z world/local zoom. Style candidates A/B/C/D are documented with real frame renders in [gfx/](gfx/index.html).
- [x] **Load-gate fix:** `move_ready_ms` persisted as an absolute wall-clock value, freezing movement for up to a minute after loading a save. Rebased on load; regression lives in `save_round_trip_restores_the_journey`.

### Observations

- A live 50-request smoke attempt produced **30 valid Jev results**, then **five HTTP 429 fallbacks**. It intentionally stopped at 35 after verifying that both the service and HUD switched to heuristics.
- Overall p95: **641 ms**. This exceeds the draft's 150 ms tier-1 aspiration. Network latency does not block input, rendering, or simulation.
- Reported input usage: **24,163 tokens**; input-only estimate **$0.001015** at $0.042/million. This is not a verified invoice or hourly cost measurement.
- Populated-city benchmark: approximately **81 ms / 10,000 simulation ticks** with 258 NPCs (about **8 µs/tick**). This excludes terminal rendering/network and is not a worst-case whole-process CPU guarantee.
- Live terminal CPU sample: **1.04% of one core over 10.52 seconds** while standing in populated Millbrook with NPC movement and live Jev enabled. This short Windows sample is not a worst-case CPU guarantee.
- Real terminal checks cover title, starting play, movement, trading, quest acceptance, journal and clean exit. Headless rendering exercises every modal and resize case.
- Full action-driven campaign completions: **seed 1 — 2,646 actions / 285 combat rounds / 77 kills**; **seed 42 — 2,643 actions / 291 combat rounds / 79 kills**. Both finished all five quests, reached level 10, and won with zero deaths, using earned equipment and normal movement.
- Replaying the same **30 live request states** through the heuristic provider compared **111 questions**: **21** differed in selected choice, score band, or boolean side; mean absolute normalized-value difference **0.1458**. This shows observable provider differences, not that either provider is better balanced.
- Final release checks: **27 tests pass**, strict Clippy is clean, and headless rendering passes **65 modal/size combinations**. Fresh live boss requests for the Lich and Adjudicator both returned valid six-question answers (**1,190 ms / 409 ms**, no fallback). The real terminal also opened inventory while a social judgment was pending, received the answer without blocking input, and exited cleanly.

### Not claimed

A five-minute demo GIF, measured human campaign duration, a statistically controlled live-versus-offline gameplay study, and achievement of the original gateway latency target have not been produced. These are presentation/measurement work, not missing campaign paths. The game remains fully playable when gateway access or rate limits prevent live decisions.

## M9 — Windowed sprite view C

- [x] `--view tui` preserves A as the default; `--view gui` launches a resizable native macroquad window.
- [x] Procedural pixel terrain and actors, square-tile camera, exploration fog, distance lighting, water shimmer, combat boundary, pulsing telegraphs, NPC health bars, and floating combat text.
- [x] Graphical atlas with world/local zoom; interior city locations map to their world portal.
- [x] Shared keyboard action routing, HUD, every modal, decision pump/failure handling, and save/load. Held movement follows terrain cooldown; combat stays one move per press.
- [x] `--gui-screenshot PATH.png` captures actual starting gameplay and exits. [Native capture](gfx/C-live.png); the older C mock-up remains for comparison.

**Verification:** release tests (27), strict Clippy, and terminal smoke (70 modal/resize paths). Real Windows GUI interaction covered title, merchant purchase, quest acceptance, inventory, journal, world/local atlas, inspector, runes, resize, diagonal/held movement, and save/load. A GUI save with the purchased item and active quest also loaded in a real terminal session. Window close/save exits were clean. This is not a separate full-campaign graphical playthrough or a new live-Jev measurement.

## M10 — Port to self-hosted Laya

The decision layer moved from the hosted TypeSafe Jev API (Vercel AI Gateway, closed) to [Laya](https://huggingface.co/convaiinnovations/laya) — open Apache-2.0 weights, served by a local sidecar (`laya_gateway.py`) that speaks the same `/v1/evaluate` dialect. The Rust client (`src/laya_client.rs`) keeps its strict validation, retry, and fallback policy; only the endpoint, provider label, and mode name changed (`--ai laya`).

- [x] `laya_gateway.py` sidecar: `/v1/evaluate` + `/health`, boolean⇄noul translation, `noul`/`choice`/`score` passed through, distribution re-normalization to exactly 1.0, auto-routing between the English and multilingual checkpoints
- [x] Client port: `LayaProvider` (5s timeout), keyless local auth, `LAYA_GATEWAY_URL` override, startup health probe for the default mode
- [x] Rename across the crate (`laya-realms`, `LayaProvider`, HUD/inspector labels); replay logs, inspector, and failure policy unchanged in behaviour
- [x] Cost model: input tokens still reported; estimated price per million now **$0** (self-hosted)

**Verification:** `cargo test` — 27/27 pass. `--ai-smoke 12` against the local gateway (RTX 4090): **12/12 live decisions, p95 148 ms**, zero fallbacks, 14,907 input tokens, estimated cost $0.00000000. The tier-1 150 ms p95 budget is met on GPU; CPU-class hosts should expect ~200–500 ms per decision and heavier sessions may prefer `--ai heuristics`. Historical hosted-gateway measurements above are preserved as records of the old provider.

## M11 — Windowed actor design

- [x] Distinct 20-unit silhouettes for all 17 NPC archetypes; letter badges removed.
- [x] Player cape and wide gold foot marker; independent equipped weapon/armour tiers change materials, blade length, plates, helmet and trim.
- [x] Actual boss phases drive wounds, awakened eyes and phase-three highlights.
- [x] View-local two-frame walking, persistent horizontal facing, HP-loss flashes and translucent clipped shadows. No changes to simulation RNG or save schema.
- [x] Sprite-specific legends; terminal presentation unchanged. [Actor contact sheet](gfx/C-actors.png), [current gameplay](gfx/C-live.png), and [remaining visual waves](VISUAL_ROADMAP.md).

**Verification:** 27 tests pass, strict all-target Clippy and formatting checks pass, and terminal smoke covers 70 modal/resize paths. A temporary native-window harness exercised every archetype, all four equipment tiers and mixed tiers, boss phases 1–3, movement/facing/flash timing, teleport/map/load resets, clipping, and serialized-game equality across rendering. Actual GUI starting gameplay was captured separately. Both windows exited automatically; the temporary harness was removed. No new full-campaign playthrough or renderer-performance measurement is claimed.

## M12 — Higher-resolution detail and viewport seams

- [x] 1440×900 logical startup fit for the native window; full-screen/restore remembers a fitting windowed rectangle. World zoom 16/24/32/40/48/64 and text scale move independently, with readable limits at small sizes.
- [x] Terrain is re-authored at 32 units with material cues: cobbles, grass tufts, tree crowns/trunks, masonry with top faces, stairs, water depth/ripples, and exploration-aware road/bank edges.
- [x] Actors are re-authored at 32 units with segmented limbs, faces, construction, equipped gear, distinct bosses and powered phase markers. Players show independent weapon and armour grades without pretending mixed gear creates combined levels.
- [x] The map projection math (`sprites::map_view`) is shared with rendering, enabling accurate pointer-to-tile mapping downstream; scene/menu render surfaces (`ui::draw_scene`, `ui::draw_windowed_modal`) are public seams instead of duplicated compositions.
**Verification:** 27 tests pass, strict all-target Clippy and formatting checks pass, release build succeeds, and headless smoke covers 13 maps and 70 modal/resize paths. A temporary native-window probe covered 174 size/scale/modal/zoom combinations plus 40 overworld frames (~0.28 ms median, ~0.79 ms p95 measured render cost, excluding swap/GPU wait). All probe artifacts are deleted.

## M13 — Boss relics

- [x] The Chief drops Rallybreaker (+2 damage after defending), the Matriarch drops Fangmantle (+1 DEF while at least two enemies are adjacent), and the Lich drops Graveglass (-1 rune mana cost, minimum 1).
- [x] One relic equips at a time through the existing inventory action; swapping stows the previous relic, relics cannot be sold, and full packs never destroy a unique drop.
- [x] The shared HUD and inventory expose the equipped relic and its exact passive. The windowed player sprite gains a distinct weapon seal, fang shoulders, or graveglass circlet from the authoritative equipped field.
- [x] Existing saves load with an empty relic slot via a serde default; both renderers and the combat engine use the same saved value.

**Verification:** regression `boss_relics_drop_and_apply_their_three_passives`, full test suite, strict Clippy, and formatting checks.

## M14 — Character creation and the six orders

- [x] Title → creation (two beats: order, then origin boon) replacing the single fixed loadout. Six classes in three callings (Ward: Keepwarden, Gravebound · Hunt: Redwake, Waysworn · Speaker: Sigil-Sworn, Fensworn), each with a starting-stat diff and an exclusive signature per docs/D2_EVOLUTION.md §4.2; boons per D2 (tithe pouch / drilled marches / take nothing).
- [x] Signatures implemented in the engine: Bulwark oath stance-pick on F (Hold recoil/round, Break trades defense for +2 next strike, Breathe pays +1 extra stamina; resolve builds 0–3), Sigil-Sworn consecutive-rune channel (+1+2 bolt), Fensworn bond (25g hire, +25% partner HP once, every third strike doubles, whole revival), Redwake momentum (kill/flee banks up to 3, strike spends +2 each, 3 buys a guaranteed exit per D23), Gravebound Last Vigil (+2 ATK <50% HP, +4 <25%), Waysworn Open Road (road/ford tolls halved; post-kill next move toll-free with +1 stamina refund).
- [x] Class-tinted meter chip on the HUD gauges row per §4.3 (D15); text-only counters, no new spending economy; identical content in terminal and windowed views.
- [x] Save schema: `Player.class`/`class_state` and creation fields are serde-defaulted; pre-class saves load as `Class::None` and play exactly as M13.

**Verification:** 34 lib tests (8 new class regressions: creation sheet matrix, Gravebound thresholds, Redwake bank/spend, Keepwarden recoil+resolve with brace-expiry, Sigil-Sworn channel build/bolt damage/movement break, Waysworn tolls+trail, Fensworn hire/re-hire/whole-rise and third-round double-strike), CLI smoke on 70 modal/resize render paths (14 paths × 5 sizes) including creation and oath screens, and text-buffer visual dumps of all three new screens.

## M15 — Talent trees (the oath-trees)

- [x] Six class trees, three branches × four nodes each (72 talents), tier-gated at player level 1/3/6/9 with predecessor chains, 2-point branch capstones, and the §5 mini-synergy rule (each branch's third node strengthens per node 1–2 held).
- [x] Point economy: 1 per level-up (L2–L10) + 1 per sigil claim = 12; ledger is `Player.talent_points` (earned) minus `Player.talents` (one entry per point), serde-defaulted; M14 saves serialize with 0 points.
- [x] Every node is wired to a live engine seam on payment: braced-defense multipliers (Plate Drills/Judgment/Overwatch), stamina economy (Second Wind/Steady Bit/Swampwise), lethal intercepts (Last Bastion/Methuselah Vow/Plunder the Ossuary/Bogside Vigil), spell scaffolding (Conductor/Static Pact/Overcharge/Tempest/Illuminator/Wordsmith/Plumb Line/Still Water/Slow Burn/Cascade/Redact/Seal-Reader), movement terrain (Sedgeskin/Pathfinder/Paupers' Saint/Surveyor), marketplace economics (Gutter-Law/Fence's Friend/Rights of Way/Oathpath Discount/Caravan Code/Quartering/Forager), combat tempo (Ebb/Red Tide/Rip Line/Second Tide/Rip Current/Crow-Broker/Hollow Court/Requiem/Bell Toll/Ash-Writhe/Road-Sworn/Track Stand/Waylaid Reading/Hollow Caller), bond upgrades (Lean Bite/Thick Coat/Bloodline/Packlord/Camouflage/Snare Craft/Skinning/Lure).
- [x] `Modal::Talents` (T key): three branch columns, per-node status (held / x-of-2 / level gate / needs prior / cost), point balance footer, Enter learns; identical content in both views.
- [x] One free Oracle respec (D8) through the existing talk menu with exact rollback of stat-delta talents; flags guard reuse.

**Verification:** 42 lib tests (8 new: income ledger 9+sigil+boss-levels, spend gates incl 5-point branch arithmetic, respec delta rollback, Tempest free cast incl gate bypass, Conductor 3-charge channel, Last Bastion lethal intercept, Overwatch adjacency bite, Packlord 2-cadence), CLI smoke on 75 modal/resize render paths (15 paths × 5 sizes) including the Oath-Trees screen.

## M16 — Bosses I (the arena pair)

- [x] **The Tidemother** (≈96 HP, L8): rises on the Saltmarsh smuggling line's resolution (either verdict) into the new docks arena "The Tidemother's Landing" (map 14 — water wall north, mooring-post cover, one way out). Signature **Undertow**: every 4th combat round telegraphs a pull dragging each hero 2 tiles toward the water line; falling in deals 10 and hauls you back onto the boards; `drag_who` (Laya choice) marks which hero the tide seizes for +4 grip; `sacrifice_crew` has her drown a crewman to mend (D10). Relic: **Saltcrown** — root immunity to pulls and displacement.
- [x] **Cragmother** (≈112 HP, L6): the open ridge den (map 13, mouth at the northern crest east of the river — our smallest boss room by design). Signature **Avalanche Slam**: existing wind-up machinery telegraphs a 3×3 slam whose struck ground stays rubble (slow Rock) for the fight's remainder, restored at combat end via a `Game.rubble` ledger; phase-two roar **guard_cubs** calls 2 cub bears from the nests; `commit_slam` is Laya's reposition-or-commit lever. Relic: **Stoneheart** — +1 DEF; rough ground (rubble, rock, mountain, forest) reads as road under your step.
- [x] Arena gear: two new archetypes (drowned kelp-and-barnacle Tidemother painter; quadruped Cragmother), relic detail marks on the player sprite (brow-diadem, ridge pauldron), boss/question banks wired (`question_bank`, heuristic fallback answers with real decision stories), and both arenas reachable via portals on the overworld crest and Saltmarsh's dockboard.

**Verification:** 51 lib tests (8 new world/game regressions: arena anchoring + bidirectional portals, undertow drag two-tiles + exact 4-grip HP, salt-water dunk 10+4 HP with dock-edge return, Saltcrown full root + no grip, avalanche rubble leaves and combat-end restore, cub roar at deep phase, both relic awards + Stoneheart road-reading, once-only landing stir), CLI smoke on 75 modal/resize render paths across 15 maps / 262 NPCs, workspace error-clean.

## M17 — Bevy world view and the art pipeline (E0 + E4)

- [x] **E0 engine spike**: Bevy 0.19.1 + bevy_light_2d 0.10 validated on tile-wall occluders; route-a render-to-texture light map from sim torch/night/vision (Chebyshev bands) adopted as the base layer; route-b point lights with `cast_shadows` for torch pools. Post stack bloom 0.08 / vignette 0.35 / exposure-lift judgment. Evidence: [e0-a-lgttorch](gfx/proto/e0-a-lgttorch.png), [e0-b-diorama](gfx/proto/e0-b-diorama.png).
- [x] **D19 art pipeline**: `tools/atlas` procedural baker (C/C+ hybrid: grim unlit-albedo environment plates + chunky outlined lifted actors + de-block edge dither) over `assets/atlas/manifest.json` (tile 64px; square sheet 16 tile variants, iso sheet with top/left/right face rows; 19-actor sheet incl. Tidemother/Cragmother + 6 class-caped heroes under `states.Player.<Class>`; props). Contract: [ART_PIPELINE.md](ART_PIPELINE.md). Contact sheets reviewed, one weak batch (mountain facets, bear read) parked for replate.
- [x] **`crates/view` Bevy window** — world parity on the real sim: atlas-textured terrain (unlit-albedo plates, light-only tints), separately-synced snapshot model (`SimSlot`/`WorldView`), follow + wheel-zoom camera, per-frame route-a light + torch pools + day/night ambient, floating text, combat vignette pulse, actor reconcile with class-hued heroes + gear/phase flash channel, props (braziers/rune-stones/relic pedestals), orbs HUD (HP/mana orbs, stamina ribbon, sigil pips, XP sliver), class meter chip verbatim from `ui.rs::class_meter`, gold-serif boss nameplates with combat-end fade, and a silent ExitHandoff round-trip writing the played `Game` back to the caller (App::run() swallows it otherwise — reworker of state loss).
- [x] **Flagship wiring**: `cargo run --release --features bevy-view -- --view bevy` boots the same entry as tui/gui behind an opt-in feature; sim library moved to `crates/core` to break the dependency cycle; journeys start unsworn until E6's in-window creation.

**Verification:** `cargo check -p realms-view` clean; three purpose-built frame captures pixel-inspected — [e4-foundation](gfx/proto/e4-foundation.png) (torch pool + fog tiers + textures), [e4-actors](gfx/proto/e4-actors.png) (nameplate, evidence sprites, props), [e4-hud](gfx/proto/e4-hud.png) (orbs, CHANNEL chip with live charge, ribbon, pips, combat tint); flagship `cargo run --features bevy-view` boots under the same renderer 8+ min stable. Full lib suite 51/51 and CLI smoke on the sim (75 modal paths, 15 maps, 262 NPCs). Known debts: nameplate bloom-bleed on world-space text, Cinzel display font blocked by network (hot-slot ready), macroquad C frozen in maintenance pending D18.

## M18 — Bosses II, side bosses and signatures (E5)

- [x] Side bosses quest-tied: **Gnaw-Thane** (fen barrow from the rat-catch claim; Plague Tide pours rats per living brood-hole — kill-the-holes-cuts-the-tide; bites stack **fester**), **Tollmaster Grudge** (toll warlord; Bridge Tax shoves + caltrops), **Mirelight** (night-only, Three False Lights tell-game), **The Pale Stag** (bounty-escalation duel that breaks and runs unless cornered). Relics: Gnawbone Crown, Tollcoin Charm, Wisplight Lantern, Hartshorn — each with a shipped passive.
- [x] Support rails: fester status (every 3rd stack −1 max stamina; anti-toxin 8-turn guard, inn purge) + **alchemist** in every city (AntiToxin/ManaTonic mid-combat usable); Fensworn **bond-wolf** (Packlord truce → sworn companion, one slot); sellsword stats re-derived at hire AND every level-up (D22/D35); Waysworn bounty-economy +25%; essence respec items; Adjudicator no-retreat oath at the verdict-shrine; level cap 10→12 with mastery levels (stat growth only past L10).
- [x] **Playtest rebalance (§4.5, A1–A3 applied):** permanent harness `crates/core/src/balance.rs` + `examples/balance_model.rs` — v0.4 anchors reproduced to the tenth, D35 companion lift inside band (0.52–0.70×), all four side bosses outlast same-level trash (7–13 vs ~3.3 rounds), Adjudicator L12 0.57× anchor. Zero shipped constants changed; band tests make drift fail loudly.

**Verification:** 70 lib tests (16 new E5 + 3 balance regressions incl. pre-E5 save round-trip), E4 enemy-block/contact parity, balance band test, workspace error-clean.

## M19 — Bevy UI parity (E6) — in flight

- [x] Mouse/touch layer (earlier slice): click-to-move autowalk with halt rules (hostile LOS, modal, movement key, no-progress), edge-pan, tap-to-inspect, touch tap/drag; runtime display options (B/V/L toggles + persisted zoom, `saves/view-options.json`, D40).
- [ ] All modals + atlas + title/creation/save flow in-window (bevy_ui per research §4); `--view bevy` becomes the default graphical mode.

## M20 — The Second Watch (E7) — pending

Six-act campaign resequencing (§6.5), Vael/Lich rework + two Underkeep depth floors, the Oathless Curate (Unwrit, First Writ), class quests + trial arenas, epilogue stance, seal-glyphs and words (D31), transmute set + recipe codex (D32), scrounge pickups.

## Non-goals

Multiplayer, pointer-first UI (the Bevy view's click-to-move/tap are convenience layers over the same verbs, not a mouse-driven UI), save-sharing, freeform procedural quest narratives, crafting trees, and multi-slot parties. Saves, runes, one companion, forging, seeded bounties, and class talent trees shipped (M8, M15) — see [GAME_DESIGN.md](GAME_DESIGN.md) §§11–12.
