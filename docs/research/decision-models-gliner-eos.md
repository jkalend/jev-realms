# Decision model additions: GLiNER2.5-Decide and Eos 0.8B

**Decision snapshot: 2026-10-04**  
**Status: proposal; no model or runtime default changed.**

## Recommendation

Add both models as selectable backends behind the existing local decision gateway. Integrate and evaluate **Decision 2.0 Eos 0.8B first**. Its published `system_one(state, questions)` interface directly matches this game's typed `noul`, `choice`, and `score` requests, and it returns a probability distribution for each answer. The model card reports 53.9 on JevArena, 50.3 median macro-F1 across 15 human-labelled transfer tasks, and 6 ms median latency for a single-question GPU request. These are model-publisher results, not measurements on Laya Realms.

Keep Laya as the runtime default until the new model passes a paired, game-specific evaluation. The repository's current local Laya run has measured evidence on an RTX 4090 (12/12 valid responses, p95 148 ms). Eos's results do not yet establish an improvement in this game's NPC behavior. The publisher benchmarks use different suites and metrics.

GLiNER2.5-Decide should be the second backend and the main challenger, especially for CPU-only play. Its 340M encoder has an official CPU latency result and can express constraints across related answers. It is more attractive if CPU latency, lower hardware requirements, or joint constraints matter more than the most direct match to the current provider interface.

## Fit with this project

The Rust game already constructs one structured state and a set of typed questions per NPC decision window. The Python sidecar serves `/v1/evaluate`; Rust validates that every question has one answer, that choices and score levels match their declared criteria, and that probability distributions are valid. Invalid, partial, or failed responses fall back to deterministic heuristics. The gateway also reports input-token usage, which the current Rust decoder requires.

Both candidates are self-hostable Apache-2.0 decision models. Both can be adapted without changing gameplay rules or giving the model authority over game state. The work belongs in the sidecar adapters, provider selection, and measurements; Rust remains responsible for validating answers and applying rules.

## Trade-offs

| Candidate | Advantages for Laya Realms | Costs and risks |
| --- | --- | --- |
| **Decision 2.0 Eos 0.8B** | Native state-plus-typed-questions API; supports choice, yes/no (`noul`), and ordinal score questions with a distribution per answer; 16,384-token context; strongest directly relevant published head-to-head evidence among these two on JevArena. | 0.75B parameters versus GLiNER's 340M; CPU latency and this game's real request latency are not published. The supported Transformers version is `>=5.17`, and the documented loader uses `trust_remote_code=True`, so pin and review the model revision/code. The documented quickstart shows answer output; the adapter must also provide the input-token usage field required by the game, counting tokens if the native result does not expose them. |
| **GLiNER2.5-Decide** | 340M English encoder; schema-defined choices and ordinal decisions, with yes/no expressible as a two-label classification; related answers can be constrained jointly; can also extract spans, relations, and structured records. Fastino reports local CPU and GPU timings, including 167.3 ms p50 for a 64-token, two-head request on a 48-vCPU Xeon. This makes it a credible CPU-first option. | English-only checkpoint; narrower specialist rather than a general reasoning model. Published timings are vendor measurements on specified hardware and short inputs, not the game's 1.5 KB state or target-machine p95. It needs a model-specific schema/output adapter. There is an upstream loading-path discrepancy: the model card's quickstart names `GLiNER2.from_pretrained`, while the current GLiNER2 repository says to load GLiNER2.5 boundary checkpoints through `AutoExtractor`; verify the pinned release's actual loader before integrating. Token usage also needs to be counted for the current Rust contract. |

Eos's card reports a 6 ms median for a single-question GPU request; GLiNER's CPU/GPU numbers use a different schema and timing setup. Treat these figures as reasons to benchmark because the workloads differ. Fastino reports 60.1% in its release post and 60.2% in the model card for Fast Decisions, its own 17-domain suite; Eos's JevArena and transfer results use other evaluation sets. None of those figures establishes which model produces better NPC behavior.

Sources: [Eos model card](https://huggingface.co/vllm-sr/Decision-2.0-Eos-0.8B); [GLiNER2.5-Decide model card](https://huggingface.co/fastino/GLiNER2.5-Decide); [Fastino's release and benchmark methodology](https://fastino.ai/blog/gliner-2-5-decide-open-weight-decision-model); [GLiNER2 upstream loading and architecture notes](https://github.com/fastino-ai/GLiNER2).

## Addition plan

1. **Keep one selected model per sidecar process for the first version.** Add an explicit model selection (`laya`, `gliner`, or `eos`) and report the active model from `/health`. This keeps the comparison reproducible and avoids assuming the models can share the currently measured GPU memory budget. Preserve `heuristics` as the no-sidecar/offline mode and keep Laya as the default during rollout.
2. **Keep one `/v1/evaluate` response contract.** Implement one adapter per model. Map the existing state and question schema to each native API, then normalize outputs to the wire types already accepted by Rust. For GLiNER, encode `noul` as a two-label yes/no question, use the supported GLiNER2.5 loader from the pinned upstream library, and verify how the pinned version exposes the probabilities needed by the game's consumer. Initially do not enable new cross-question constraints: that would change semantics as well as change the model. Test joint constraints separately if a concrete contradictory-answer case appears.
3. **Preserve strict validation and truthful telemetry.** Require every requested answer; verify answer type, allowed choice keys, score range, and finite probabilities; normalize only small rounding residuals; verify the selected choice is the distribution maximum. Ensure the adapter returns `usage.inputTokens`/`input_tokens`; if a model API does not report token usage, count it with that model's tokenizer rather than reporting a fabricated zero. Keep the existing retry and heuristic-fallback policy.
4. **Expose the provider identity.** Include the selected model in the CLI selection, decision inspector, replay session record, and gateway health response. Reject a client/server model mismatch early instead of silently using another checkpoint. Do not automatically switch existing users from Laya to a new model based only on an upstream benchmark.
5. **Document model setup separately.** Pin model revisions and Python dependencies; list each model's download size, device requirements, startup behavior, and tested Python setup after measuring them. Eos's documented path needs Transformers 5.17 or later and reviewed remote model code. GLiNER needs the local inference extra and a tested GLiNER2.5 loader.

## Evaluation gate

Compare the candidates against Laya using the same frozen requests, prompt/state serialization, decision rules, hardware, and warm-up policy. Build the corpus from actual game snapshots and cover social judgments, tactical combat, each NPC archetype, bosses, player mercy, and edge cases with small or changing choice sets. Keep vendor benchmark numbers in context; do not mix them into the game score.

For each request, record answer validity, fallback count, chosen action, full distribution, confidence, warm/cold startup time, p50/p95 latency, peak memory, CPU/GPU, and token count. Run the same fixed-seed campaign paths with each provider and review behavior without seeing provider labels. The existing `--ai-smoke` is useful for live validity and latency checks, but it is not a comparative quality benchmark by itself.

Promote a model only if it has no malformed or partial outputs on the frozen set, stays within the game's 150 ms tier-1 p95 target on the reference RTX 4090, and shows no regression in reviewed NPC behavior or fixed-seed completion. Record p50/p95 on a named CPU host; use 500 ms p95 as a provisional ceiling, informed by the README's observed 200–500 ms Laya CPU range. Confirm that slow CPU requests remain asynchronous and preserve the existing heuristic fallback. If neither new model clears those gates, keep Laya as default and retain the new choices for further tuning.

## What would change the recommendation

- **Choose GLiNER first or make it the preferred option** if it matches or beats Eos on the game corpus, its CPU p95 or memory use is materially better on the machines the game should support, or its joint constraints solve a real cross-question consistency issue without harming NPC behavior.
- **Prefer Eos with more confidence** if it passes the game-specific quality and latency gates, while GLiNER's game decisions are weaker or its current loader/output API adds meaningful fragility.
- **Stay with Laya** if both candidates fail answer validation, produce worse or less coherent NPC behavior, or add setup/runtime costs without a visible improvement. A lower publisher benchmark score alone is not a reason to switch.
- Revisit the choice when pinned model/library versions, CPU/GPU targets, or project request patterns change; the current recommendation reflects published information and this repository's measurements available on 2026-10-04.

## Project references

- [AI architecture](../AI_ARCHITECTURE.md): gateway contract, state construction, and provider failure policy.
- [README](../../README.md): setup, current live-model configuration, and observed Laya measurements.
- `laya_gateway.py` and `crates/core/src/laya_client.rs`: current gateway translation, token-usage requirement, and strict answer validation.
