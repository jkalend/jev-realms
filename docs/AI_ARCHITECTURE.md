# AI Architecture — Laya Realms

How [Laya](https://huggingface.co/convaiinnovations/laya) (open-weights System One decision model, Apache 2.0) drives NPC judgment while Rust owns authoritative game state. The implementation lives in `src/laya_client.rs`, `src/engine.rs` and `src/social.rs`. Original design budgets below are targets, not performance guarantees; measured results are in [ROADMAP.md](ROADMAP.md).

## 1. Principle

Laya is a **decision oracle**, not a generator. It receives a `state` (structured snapshot of the game world from one NPC's perspective) and a set of typed `questions` (`noul` = calibrated yes/no, `choice` = pick from options, `score` = ordered scale). It never sees the whole game and never writes back to it — Rust owns all state, all mutation, all truth.

```
 game state (Rust, authoritative)
        │  build state per NPC per decision window
        ▼
   ┌─────────┐   questions      ┌─────────┐
   │  state  │ ───────────────▶ │  Laya   │
   │ builder │                  └────┬────┘
   └─────────┘                       │ answers {probabilities, confidence}
        ▲                            │
        │  apply via behavior rules  │
   ┌───┴──────┐                      │
   │ behavior │ ◀────────────────────┘
   │  layer   │
   └──────────┘
```

## 2. Wire protocol

Local Laya gateway (`laya_gateway.py`, the `laya` Python SDK behind the same `/v1/evaluate` dialect), model `convaiinnovations/laya`:

```http
POST http://127.0.0.1:8128/v1/evaluate
# Authorization: Bearer <token>  — only when the gateway is token-gated
```

Question types map to Laya primitives:

| Game concept | Laya type | Example question |
| --- | --- | --- |
| Should this NPC do X? | `noul` | `ambush_player` → probability 0.0–1.0 |
| Which target/action? | `choice` | `target_pick` with criteria {player, packmate, cattle} |
| How strongly? | `score` | `flee` on [calm … panic] legend |

**Wire distinction:** this document uses the design's primitive name `noul`. The `/v1/evaluate` dialect kept from the gateway era requires `type: "boolean"` and returns `probability`, with `usage.inputTokens`. `gateway_body` translates requests; `decode_answers` validates the actual gateway response. Choice/score distributions are preserved, and score indices are normalized by `(criteria.len() - 1)` before gameplay thresholds use them. The local gateway (`laya_gateway.py`) performs the reverse translation on its side, so the Rust client is unchanged in dialect.

Batching rule: **one request per NPC per decision window**, all of that NPC's questions for the window in a single call (Laya evaluates multiple questions against one state in one request). NPCs are never batched into one request — states differ, and per-NPC isolation keeps the replay log clean.

## 3. State building

### 3.1 State schema

Each request's `state` is a flat, human-readable JSON object, per NPC:

```json
{
  "npc": { "archetype": "bandit", "name": "bandit-7", "hp": 9, "level": 2,
           "personality": { "brave": 0.4, "greedy": 0.8, "spite": 0.3 } },
  "self": { "allies_alive": 2, "allies_dead": 1, "armed": true, "wounded": false },
  "player": { "level": 3, "visible": true, "distance": 4, "armed": true,
              "looks_rich": true, "recent_killed_nearby": 1 },
  "memory": { "disposition": -0.2, "events": [
      { "turn": 342, "kind": "player_killed_packmate", "weight": -0.6 } ] },
  "environment": { "location": "ford", "terrain": "ford", "time": "dusk",
                   "witnesses_nearby": 0, "guards_nearby": 0 }
}
```

Budget: ≤ 120 keys, ≤ ~1.5 KB per NPC state. Laya's context window (32k) makes this trivial; the budget exists for **latency and cost discipline**, not capacity.

### 3.2 Personality axes

Each NPC rolls personality values at spawn (deterministic from world seed + npc id — same session, same NPC, same personality):

| Axis | Range | Affects |
| --- | --- | --- |
| `brave` | 0–1 | flee threshold, ambush willingness |
| `greedy` | 0–1 | vendor cheating, bandit targeting rich-looking players |
| `gullibility` | 0–1 | bribe success on guards |
| `spite` | 0–1 | parting shots when fleeing, post-defeat grudges |
| `chatty` | 0–1 | rumour sharing, dialogue willingness |

Personality is **state, not instruction** — Laya reads it like any other fact and the same question set produces different behavior per NPC. This is the "small judgments" thesis made concrete: one shared question bank, individualized by state.

## 4. Decision tiers and cadence

Full tier table lives in the README; the operative rules:

- **Tier 0 (reflex) never calls Laya.** Pathfinding, wandering, repathing, collision — pure Rust. Laya would add cost and nondeterminism for zero observable gain.
- **Tier 1 (tactical)** fires on events: player enters perception radius, ally dies, NPC drops below HP thresholds (25/50/75%). At most 8 concurrent tier-1 NPCs per decision window.
- **Tier 2 (social)** on player-initiated interaction only (talk/trade/interrogate).
- **Tier 3 (strategic)**: boss NPCs, every 5 game-turns or on phase change.
- **Tier 4 (oracle)**: explicit user actions only (prophecy at shrines, artifact identification).

The game loop never blocks on Laya: requests run asynchronously (`reqwest` + tokio), and completed answers apply on the next main-loop iteration. Answers update intentions; they never advance a combat turn or directly damage the player. An NPC mid-decision continues tier-0 behavior. Offline decisions are queued synchronously in submission order for reproducibility.

## 5. Question bank (v1)

### Bandit (tier 1)

```json
{
  "ambush_player": { "type": "noul", "instructions": "Should this bandit ambush the player now?" },
  "target_pick": { "type": "choice", "instructions": "Who is the best target?",
              "criteria": { "player": "the armed traveller", "packmate": "a wounded ally", "cattle": "a farm animal" } },
  "flee":   { "type": "score", "instructions": "How close is this bandit to breaking and running?",
              "criteria": ["steady", "nervous", "afraid", "panicking"] }
}
```

### Guard (tier 1–2)

```json
{
  "suspect_player": { "type": "noul", "instructions": "Does this guard find the player suspicious?" },
  "accept_bribe": { "type": "noul", "instructions": "Would this guard look away if offered gold? Consider personality.gullibility and nearby witnesses." },
  "pursue_fleeing_player": { "type": "noul", "instructions": "Should this guard chase a fleeing suspect?" }
}
```

### Vendor (tier 2)

```json
{
  "haggle_accept": { "type": "noul", "instructions": "Should this vendor accept the offered price?" },
  "cheat_player":  { "type": "noul", "instructions": "Would this vendor overcharge a naive-looking player?" },
  "offer_quest":   { "type": "choice", "instructions": "What work does this vendor offer?",
                     "criteria": { "delivery": "carry a parcel to another city", "escort": "guard a caravan leg", "collection": "retrieve owed goods" } }
}
```

### Lich (tier 3 — the showcase fight)

Every 5 turns or on phase change, one request:

```json
{
  "tactic": { "type": "choice", "instructions": "Choose the next tactic for this phase of the fight.",
              "criteria": { "pressure": "aggressive damage focus", "summon": "raise skeletons to delay",
                            "curse": "debuff the player's defense", "retreat": "fall back and recover mana" } },
  "focus_target": { "type": "choice", "instructions": "Who should the minions focus on?",
                    "criteria": { "player": "attack the adventurer", "packmate": "protect an ally", "cattle": "seek easier prey" } },
  "phase_shift": { "type": "noul", "instructions": "Has the fight turned against the Lich enough to change phases?" },
  "desperation": { "type": "score", "instructions": "How desperate is the Lich?", "criteria": ["composed", "concerned", "worried", "frantic"] }
}
```

The Lich's answer drives which scripted sub-behaviors Rust enables; the sub-behaviors themselves (summon counts, curse durations) are deterministic tables.

### The Adjudicator (tier 3)

See GAME_DESIGN §10. The state combines fixed deed counters with the last eight combat actions. Mercy/greed/courage ratings shape attacks, while each phase can adapt its tactic. Tier 4 remains reserved for explicit shrine prophecy and relic identification.

### Boss bank v2 — E3/E5 side bosses (per D2_EVOLUTION §6 tables; each question has a heuristic fallback in `question_bank`)

| Boss | Questions | Drives |
|---|---|---|
| The Tidemother (E3) | `drag_who` (choice), `sacrifice_crew` (noul) | Which hero the Undertow grip seizes; whether she drowns a crewman to mend |
| Cragmother (E3) | `commit_slam` (noul), `guard_cubs` (noul) | Commit the Avalanche Slam vs reposition; call the cub bears at deep phase |
| Gnaw-Thane (E5) | `call_the_tide` (noul), `scatter_when_thinned` (noul) | Timing the Plague Tide pours; break-off when the horde thins |
| Tollmaster Grudge (E5) | `shove_now` (noul), `collect_or_cut` (choice) | Whether the Bridge Tax fires this round; press the player or loot the caravan |
| Mirelight (E5) | `which_light` (choice), `strike_or_subside` (noul) | Which of the Three False Lights is real (the tell); whether lights hold or subside |
| The Pale Stag (E5) | `break_and_run` (score), `stand_ground` (score) | Break-and-bolt toward the portal vs gore through when cornered |
| Oathless Curate (E7, pending) | `which_sigil_falls` (choice, telegraphed 2 rounds ahead), `mercy_for_the_oathbreaker` (noul) | Which gathered sigil the Unwrit unwrites; mercy answer feeds the Adjudicator's final rating |

Brood-holes and False Lights carry empty banks (scripted bodies); the Adjudicator's oath mode uses its existing bank.

## 6. Offline mode (deterministic fallback)

**Laya is not a runtime dependency for playability.** A `DecisionProvider` trait abstracts the source:

```
DecisionProvider
├── LayaProvider       // live: HTTP to the local gateway, async, cached, logged
└── HeuristicProvider // offline: seeded hash of (npc_id, question, state) → plausible value
```

`HeuristicProvider` produces deterministic pseudo-decisions (seeded from world seed + npc id + state bucket) with hand-tuned plausibility so the game is fully playable offline. This also gives us the A/B hook for the writeup: same session seed, `--ai heuristics` vs `--ai laya`, diff the observable behavior. Switching is a CLI flag. Replay logs record which provider decided what.

## 6.1 Failure policy

- Gateway error / timeout (5s) → that NPC falls back to `HeuristicProvider` for this decision and a `degraded` counter increments in the HUD.
- Sustained failures (5 consecutive) → whole session auto-switches to heuristics with a persistent HUD banner. No crashes, ever, from AI failure.
- The local gateway returning 503 while checkpoints load is treated like any other failure — heuristics carry the session until it is ready.

## 7. Latency & cost model

| Quantity | Measured (RTX 4090, local gateway) |
| --- | --- |
| Tier-1 events per minute of play | ~15 (active exploration) |
| Questions per tier-1 request | 3 |
| State size per request | ~1.5 KB ≈ 400 tokens |
| Cost per request | **$0** (self-hosted) |
| Latency budget (tier 1) | 150 ms p95 |
| Measured p95 | 148 ms (`--ai-smoke 12`, 12/12 live) |

The original table estimated $0.042/1M input tokens against the hosted gateway (≈$0.015/hour at 15 requests/minute). The port to self-hosted Laya removed the cost axis entirely; latency now depends on the gateway host (GPU ≈ 60–90 ms per batched request, CPU ≈ 200–500 ms). These are observations, not guarantees.

## 8. Laya inspector (pillar #1 tool)

`J` toggles the inspector panel: the last 10 decisions (either provider), each shown as `npc · question · probability · applied_rule`. For `choice` questions the top-3 probabilities are shown; for `score` the chosen band. The inspector is the debugging surface during development and the showcase surface in demos — it is shipped UI, not a dev-only hack.

## 9. Testing strategy

- **Unit**: state builders (game state → per-NPC JSON), behavior rules (answer → action), HeuristicProvider determinism.
- **Integration**: recorded Laya responses (fixtures) replayed through the decision pipeline.
- **Live smoke** (manual, needs the local gateway): run a scripted 50-decision session, assert no latency spikes above 2× budget, log token usage.
- The question bank itself is data (serde types + JSON), so adding archetypes is content work, not code work.

## 10. Open questions

1. **Decision jitter:** not added. Event triggers and the bounded service control concurrency without introducing artificial response delay.
2. **Personality representation:** structured numeric axes are implemented and accepted by live Laya. A controlled prose-versus-numeric comparison remains unmeasured.
3. **Adjudicator history:** fixed counters plus an eight-action recent window; no unbounded session transcript or generated digest.
