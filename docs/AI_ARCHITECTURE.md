# AI Architecture — Jev Realms

How [Jev](https://docs.typesafe.ai/) (`typesafe-ai/jev` via the Vercel AI Gateway) drives every NPC judgment, and where plain Rust takes over. Design-only document; module names are planned targets, not existing code.

## 1. Principle

Jev is a **decision oracle**, not a generator. It receives a `state` (structured snapshot of the game world from one NPC's perspective) and a set of typed `questions` (`noul` = calibrated yes/no, `choice` = pick from options, `score` = ordered scale). It never sees the whole game and never writes back to it — Rust owns all state, all mutation, all truth.

```
 game state (Rust, authoritative)
        │  build state per NPC per decision window
        ▼
   ┌─────────┐   questions      ┌─────────┐
   │  state  │ ───────────────▶ │  Jev    │
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

Vercel AI Gateway evaluate endpoint, model `typesafe-ai/jev`:

```http
POST https://ai-gateway.vercel.sh/v1/evaluate
Authorization: Bearer $AI_GATEWAY_API_KEY
```

Question types map to Jev primitives:

| Game concept | Jev type | Example question |
| --- | --- | --- |
| Should this NPC do X? | `noul` | `ambush_player` → probability 0.0–1.0 |
| Which target/action? | `choice` | `target_pick` with criteria {player, packmate, cattle} |
| How strongly? | `score` | `flee` on [calm … panic] legend |

Batching rule: **one request per NPC per decision window**, all of that NPC's questions for the window in a single call (Jev evaluates multiple questions against one state in one request). NPCs are never batched into one request — states differ, and per-NPC isolation keeps the replay log clean.

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

Budget: ≤ 120 keys, ≤ ~1.5 KB per NPC state. Jev's context window (32k) makes this trivial; the budget exists for **latency and cost discipline**, not capacity.

### 3.2 Personality axes

Each NPC rolls personality values at spawn (deterministic from world seed + npc id — same session, same NPC, same personality):

| Axis | Range | Affects |
| --- | --- | --- |
| `brave` | 0–1 | flee threshold, ambush willingness |
| `greedy` | 0–1 | vendor cheating, bandit targeting rich-looking players |
| `gullibility` | 0–1 | bribe success on guards |
| `spite` | 0–1 | parting shots when fleeing, post-defeat grudges |
| `chatty` | 0–1 | rumour sharing, dialogue willingness |

Personality is **state, not instruction** — Jev reads it like any other fact and the same question set produces different behavior per NPC. This is the "small judgments" thesis made concrete: one shared question bank, individualized by state.

## 4. Decision tiers and cadence

Full tier table lives in the README; the operative rules:

- **Tier 0 (reflex) never calls Jev.** Pathfinding, wandering, repathing, collision — pure Rust. Jev would add cost and nondeterminism for zero observable gain.
- **Tier 1 (tactical)** fires on events: player enters perception radius, ally dies, NPC drops below HP thresholds (25/50/75%). At most 8 concurrent tier-1 NPCs per decision window.
- **Tier 2 (social)** on player-initiated interaction only (talk/trade/interrogate).
- **Tier 3 (strategic)**: boss NPCs, every 5 game-turns or on phase change.
- **Tier 4 (oracle)**: explicit user actions only (prophecy at shrines, artifact identification).

The game loop never blocks on Jev: requests are async (`reqwest` + tokio), decisions apply on the next tick after arrival. An NPC mid-decision continues tier-0 behavior — it doesn't freeze.

## 5. Question bank (v1)

### Bandit (tier 1)

```json
{
  "ambush": { "type": "noul", "instructions": "Should this bandit ambush the player now?" },
  "target": { "type": "choice", "instructions": "Who is the best target?",
              "criteria": { "player": "the armed traveller", "packmate": "a wounded ally", "cattle": "a farm animal" } },
  "flee":   { "type": "score", "instructions": "How close is this bandit to breaking and running?",
              "criteria": ["steady", "nervous", "afraid", "panicking"] }
}
```

### Guard (tier 1–2)

```json
{
  "suspect":  { "type": "noul", "instructions": "Does this guard find the player suspicious?" },
  "accept_bribe": { "type": "noul", "instructions": "Would this guard look away if offered gold?",
                    "criteria_note": "personality.gullibility in state" },
  "pursue":   { "type": "noul", "instructions": "Should this guard chase a fleeing suspect?" }
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
  "focus_target": { "type": "choice", "instructions": "Who do the minions focus?", ... },
  "phase_shift": { "type": "noul", "instructions": "Has the fight turned against the Lich enough to change phases?" },
  "desperation": { "type": "score", "instructions": "How desperate is the Lich?", "criteria": ["composed", "concerned", "worried", "frantic"] }
}
```

The Lich's answer drives which scripted sub-behaviors Rust enables; the sub-behaviors themselves (summon counts, curse durations) are deterministic tables.

### The Adjudicator (tier 4)

See GAME_DESIGN §10 — the state is the player's summarized run history; questions rate mercy/greed/courage and adapt each arena phase.

## 6. Offline mode (deterministic fallback)

**Jev is not a runtime dependency for playability.** A `DecisionProvider` trait abstracts the source:

```
DecisionProvider
├── JevProvider      // live: HTTP to the gateway, async, cached, logged
└── HeuristicProvider // offline: seeded hash of (npc_id, question, state) → plausible value
```

`HeuristicProvider` produces deterministic pseudo-decisions (seeded from world seed + npc id + state bucket) with hand-tuned plausibility so the game is fully playable offline. This also gives us the A/B hook for the writeup: same session seed, `--ai heuristics` vs `--ai jev`, diff the observable behavior. Switching is a CLI flag. Replay logs record which provider decided what.

## 6.1 Failure policy

- Gateway error / timeout (3s) → that NPC falls back to `HeuristicProvider` for this decision and a `jev_degraded` counter increments in the HUD.
- Sustained failures (5 consecutive) → whole session auto-switches to heuristics with a persistent HUD banner. No crashes, ever, from AI failure.
- `customer_verification_required` (the current gateway state until a card is added) is treated identically to any other auth error — heuristics carry the session.

## 7. Latency & cost model

| Quantity | Estimate |
| --- | --- |
| Tier-1 events per minute of play | ~15 (active exploration) |
| Questions per tier-1 request | 3 |
| State size per request | ~1.5 KB ≈ 400 tokens |
| Cost per request | ~$0.000017 |
| **Cost per hour of active play** | **~$0.001** |
| Latency budget (tier 1) | 150 ms p95 |

Numbers assume the gateway's published $0.042/1M input tokens and no output-token charge (Jev's max output tokens is 0; answers come from the structured evaluation). Validation of these numbers is a Roadmap M2 exit criterion.

## 8. Jev inspector (pillar #1 tool)

`J` toggles the inspector panel: the last 10 decisions (either provider), each shown as `npc · question · probability · applied_rule`. For `choice` questions the top-3 probabilities are shown; for `score` the chosen band. The inspector is the debugging surface during development and the showcase surface in demos — it is shipped UI, not a dev-only hack.

## 9. Testing strategy

- **Unit**: state builders (game state → per-NPC JSON), behavior rules (answer → action), HeuristicProvider determinism.
- **Integration**: recorded Jev responses (fixtures) replayed through the decision pipeline.
- **Live smoke** (manual, needs gateway): run a scripted 50-decision session, assert no latency spikes above 2× budget, log costs.
- The question bank itself is data (serde types + JSON), so adding archetypes is content work, not code work.

## 10. Open questions

1. Should NPC decision windows jitter (±1 turn) to avoid synchronized group decisions? Lean yes; decide at M2.
2. Does Jev need the personality axes expressed as instructions (e.g. "this bandit is greedy") rather than raw state values? Test both at M2 with the replay log.
3. Adjudicator state summarization: fixed template vs weighted event digest — decide at M5.
