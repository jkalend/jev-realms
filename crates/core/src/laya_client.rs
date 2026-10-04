use crate::model::{
    mix, Answer, Archetype, DecisionRecord, DecisionRequest, DecisionResult, Question,
};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::future::Future;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::{mpsc, OwnedSemaphorePermit, Semaphore};

const DEFAULT_GATEWAY_URL: &str = "http://127.0.0.1:8128/v1/evaluate";
const MODEL: &str = "convaiinnovations/laya";
const CAPACITY: usize = 8;
/// Self-hosted Laya checkpoints: the token count is reported for budgeting, the
/// marginal cost of a local forward pass is zero.
pub const INPUT_PRICE_PER_MILLION: f64 = 0.0;
const DISTRIBUTION_TOLERANCE: f64 = 0.005;

pub trait DecisionProvider: Send + Sync {
    fn decide<'a>(
        &'a self,
        request: DecisionRequest,
    ) -> Pin<Box<dyn Future<Output = DecisionResult> + Send + 'a>>;
}

pub struct HeuristicProvider;
impl DecisionProvider for HeuristicProvider {
    fn decide<'a>(
        &'a self,
        request: DecisionRequest,
    ) -> Pin<Box<dyn Future<Output = DecisionResult> + Send + 'a>> {
        Box::pin(async move { heuristic(&request) })
    }
}

// Deliberately not Debug: credentials never enter diagnostic output.
pub struct LayaProvider {
    client: reqwest::Client,
    authorization: reqwest::header::HeaderValue,
    endpoint: String,
}

impl LayaProvider {
    pub fn new(url: String, key: Option<String>) -> Result<Self, String> {
        // The local Laya gateway needs no credential; the header is kept so a
        // token-gated deployment (LAYA_GATEWAY_TOKEN on the server) still works.
        let bearer = key.unwrap_or_default();
        let mut authorization = reqwest::header::HeaderValue::from_str(&format!("Bearer {bearer}"))
            .map_err(|_| "invalid_api_key_format".to_string())?;
        authorization.set_sensitive(true);
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .connect_timeout(Duration::from_secs(5))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|_| "http_client_initialization_failed".to_string())?;
        Ok(Self {
            client,
            authorization,
            endpoint: url,
        })
    }

    async fn attempt(&self, body: &Value) -> Result<Value, String> {
        // The outer timeout includes reading the response body, not only receiving headers.
        tokio::time::timeout(Duration::from_secs(5), async {
            let mut response = self
                .client
                .post(&self.endpoint)
                .header(reqwest::header::AUTHORIZATION, self.authorization.clone())
                .json(body)
                .send()
                .await
                .map_err(transport_error)?;
            let status = response.status();
            if !status.is_success() {
                // Keep only recognized codes, never arbitrary upstream messages or headers.
                let mut bytes = Vec::new();
                while let Some(chunk) = response.chunk().await.map_err(transport_error)? {
                    if bytes.len() + chunk.len() > 16_384 {
                        break;
                    }
                    bytes.extend_from_slice(&chunk);
                }
                let raw: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
                let kind = raw
                    .pointer("/error/type")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                return Err(if kind == "customer_verification_required" {
                    "http_403_customer_verification_required".into()
                } else {
                    format!("http_{}", status.as_u16())
                });
            }
            let mut bytes = Vec::new();
            while let Some(chunk) = response.chunk().await.map_err(transport_error)? {
                if bytes.len() + chunk.len() > 262_144 {
                    return Err("response_too_large".into());
                }
                bytes.extend_from_slice(&chunk);
            }
            serde_json::from_slice(&bytes).map_err(|_| "invalid_json".into())
        })
        .await
        .map_err(|_| "timeout".to_string())?
    }
}

/// One cheap reachability probe for the local Laya gateway (GET /health).
/// Used once at startup so the default mode never assumes the sidecar is up.
pub async fn gateway_ready() -> bool {
    let base = std::env::var("LAYA_GATEWAY_URL").unwrap_or_else(|_| DEFAULT_GATEWAY_URL.into());
    let health = base
        .strip_suffix("/v1/evaluate")
        .unwrap_or(&base)
        .to_string()
        + "/health";
    let Ok(client) = reqwest::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
    else {
        return false;
    };
    let mut request = client.get(&health);
    // Forward an optional bearer so token-gated gateways answer honestly.
    if let Ok(token) = std::env::var("LAYA_GATEWAY_TOKEN") {
        if !token.trim().is_empty() {
            request = request.bearer_auth(token.trim());
        }
    }
    request
        .send()
        .await
        .map(|response| response.status().is_success())
        .unwrap_or(false)
}
fn transport_error(error: reqwest::Error) -> String {
    if error.is_timeout() {
        "timeout"
    } else if error.is_connect() {
        "connection_failed"
    } else {
        "transport_failed"
    }
    .into()
}

impl DecisionProvider for LayaProvider {
    fn decide<'a>(
        &'a self,
        request: DecisionRequest,
    ) -> Pin<Box<dyn Future<Output = DecisionResult> + Send + 'a>> {
        Box::pin(async move {
            let started = Instant::now();
            let body = gateway_body(&request);
            let mut failure = "evaluation_failed".to_string();
            let mut input_tokens = 0;
            for attempt in 0..2 {
                match self.attempt(&body).await {
                    Ok(raw) => {
                        input_tokens += reported_input_tokens(&raw).unwrap_or(0);
                        match decode_answers(&request.questions, &raw) {
                            Ok(answers) => {
                                return DecisionResult {
                                    request,
                                    provider: "laya".into(),
                                    answers,
                                    latency_ms: started.elapsed().as_millis() as u64,
                                    input_tokens,
                                    error: None,
                                }
                            }
                            Err(error) => {
                                failure = error;
                                // A malformed/partial answer is a whole-request fallback, never a mixed result.
                                break;
                            }
                        }
                    }
                    Err(error) => failure = error,
                }
                if attempt == 0 {
                    tokio::time::sleep(Duration::from_millis(150)).await;
                }
            }
            let mut result = heuristic(&request);
            result.error = Some(failure);
            result.latency_ms = started.elapsed().as_millis() as u64;
            result.input_tokens = input_tokens;
            result
        })
    }
}
// The local Laya gateway keeps the /v1/evaluate dialect: boolean/probability on the
// wire, noul/noul in the game's design vocabulary. Translation happens only here.
fn gateway_body(request: &DecisionRequest) -> Value {
    let questions: BTreeMap<_, _> = request
        .questions
        .iter()
        .map(|(key, question)| {
            let mut question = question.clone();
            if question.kind == "noul" {
                question.kind = "boolean".into();
            }
            (key, question)
        })
        .collect();
    json!({ "model": MODEL, "state": request.state, "questions": questions })
}

fn reported_input_tokens(raw: &Value) -> Option<u64> {
    raw.pointer("/usage/inputTokens")
        .or_else(|| raw.pointer("/usage/input_tokens"))?
        .as_u64()
}

fn probability(raw: &Value) -> Result<f64, String> {
    raw.as_f64()
        .filter(|p| p.is_finite() && (0.0..=1.0).contains(p))
        .ok_or_else(|| "invalid_probability".into())
}

fn distribution(raw: &Value, keys: &[String]) -> Result<BTreeMap<String, f32>, String> {
    let object = raw.as_object().ok_or("missing_distribution")?;
    if object.len() != keys.len() || keys.iter().any(|key| !object.contains_key(key)) {
        return Err("distribution_keys_mismatch".into());
    }
    let mut result = BTreeMap::new();
    let mut sum = 0.0;
    for key in keys {
        let value = probability(&object[key])?;
        sum += value;
        result.insert(key.clone(), value as f32);
    }
    if (sum - 1.0).abs() > DISTRIBUTION_TOLERANCE {
        return Err("distribution_sum_invalid".into());
    }
    Ok(result)
}

fn confidence(answer: &Value, probabilities: &BTreeMap<String, f32>) -> Result<f32, String> {
    match answer.get("confidence") {
        Some(value) => Ok(probability(value)? as f32),
        // Gateway's normalized schema does not promise confidence. Derive concentration
        // from normalized entropy; this is not an invented provider-supplied confidence.
        None => {
            if probabilities.len() <= 1 {
                return Ok(1.0);
            }
            let entropy: f32 = probabilities
                .values()
                .filter(|p| **p > 0.0)
                .map(|p| -p * p.ln())
                .sum();
            Ok((1.0 - entropy / (probabilities.len() as f32).ln()).clamp(0.0, 1.0))
        }
    }
}

pub fn decode_answers(
    questions: &BTreeMap<String, Question>,
    raw: &Value,
) -> Result<BTreeMap<String, Answer>, String> {
    let answers = raw
        .get("answers")
        .and_then(Value::as_object)
        .ok_or("missing_answers")?;
    if answers.len() != questions.len() || questions.keys().any(|key| !answers.contains_key(key)) {
        return Err("answer_keys_mismatch".into());
    }
    if reported_input_tokens(raw).is_none() {
        return Err("missing_input_usage".into());
    }
    let mut decoded = BTreeMap::new();
    for (key, question) in questions {
        let raw_answer = &answers[key];
        let answer_type = raw_answer
            .get("type")
            .and_then(Value::as_str)
            .ok_or("missing_answer_type")?;
        let answer = match question.kind.as_str() {
            "noul" => {
                let field = match answer_type {
                    "noul" => "noul",
                    "boolean" => "probability",
                    _ => return Err("answer_type_mismatch".into()),
                };
                let value = probability(&raw_answer[field])? as f32;
                let probabilities =
                    BTreeMap::from([("false".into(), 1.0 - value), ("true".into(), value)]);
                Answer {
                    value,
                    choice: None,
                    confidence: confidence(raw_answer, &probabilities)?,
                    probabilities,
                }
            }
            "choice" => {
                if answer_type != "choice" {
                    return Err("answer_type_mismatch".into());
                }
                let criteria = question
                    .criteria
                    .as_ref()
                    .and_then(Value::as_object)
                    .ok_or("invalid_choice_criteria")?;
                if criteria.is_empty() {
                    return Err("empty_choice_criteria".into());
                }
                let keys: Vec<_> = criteria.keys().cloned().collect();
                let probabilities = distribution(&raw_answer["probabilities"], &keys)?;
                let choice = raw_answer["choice"].as_str().ok_or("missing_choice")?;
                let value = *probabilities.get(choice).ok_or("unknown_choice")?;
                if probabilities.values().any(|p| *p > value + 0.00001) {
                    return Err("choice_not_highest_probability".into());
                }
                Answer {
                    value,
                    choice: Some(choice.into()),
                    confidence: confidence(raw_answer, &probabilities)?,
                    probabilities,
                }
            }
            "score" => {
                if answer_type != "score" {
                    return Err("answer_type_mismatch".into());
                }
                let criteria = question
                    .criteria
                    .as_ref()
                    .and_then(Value::as_array)
                    .ok_or("invalid_score_criteria")?;
                if criteria.len() < 2 || criteria.iter().any(|v| !v.is_string()) {
                    return Err("invalid_score_criteria".into());
                }
                let maximum = (criteria.len() - 1) as f64;
                let score = raw_answer["score"]
                    .as_f64()
                    .filter(|s| s.is_finite() && (0.0..=maximum).contains(s))
                    .ok_or("score_out_of_range")?;
                let keys: Vec<_> = (0..criteria.len()).map(|i| i.to_string()).collect();
                let probabilities = distribution(&raw_answer["probabilities"], &keys)?;
                if let Some(legend) = raw_answer.get("legend") {
                    let legend = legend.as_object().ok_or("invalid_score_legend")?;
                    if legend.len() != criteria.len()
                        || keys
                            .iter()
                            .enumerate()
                            .any(|(i, key)| legend.get(key) != Some(&criteria[i]))
                    {
                        return Err("score_legend_mismatch".into());
                    }
                }
                let best = (0..criteria.len())
                    .max_by(|a, b| {
                        probabilities[&a.to_string()].total_cmp(&probabilities[&b.to_string()])
                    })
                    .unwrap_or(0);
                Answer {
                    value: (score / maximum) as f32,
                    choice: Some(criteria[best].as_str().unwrap_or_default().into()),
                    confidence: confidence(raw_answer, &probabilities)?,
                    probabilities,
                }
            }
            _ => return Err("unknown_question_type".into()),
        };
        decoded.insert(key.clone(), answer);
    }
    Ok(decoded)
}

struct ReplayLog {
    file: Mutex<File>,
    failure: Mutex<Option<String>>,
}
impl ReplayLog {
    fn append(&self, value: &Value) -> Result<(), String> {
        let mut bytes =
            serde_json::to_vec(value).map_err(|_| "replay_serialization_failed".to_string())?;
        bytes.push(b'\n');
        let mut file = self
            .file
            .lock()
            .map_err(|_| "replay_lock_failed".to_string())?;
        file.write_all(&bytes)
            .map_err(|error| format!("replay_write_{:?}", error.kind()))
    }
    fn remember_failure(&self, error: String) {
        if let Ok(mut failure) = self.failure.lock() {
            *failure = Some(error);
        }
    }
}

pub struct AiService {
    live: Option<Arc<LayaProvider>>,
    offline: Arc<HeuristicProvider>,
    heuristics: AtomicBool,
    slots: Arc<Semaphore>,
    sender: mpsc::Sender<(DecisionResult, OwnedSemaphorePermit)>,
    receiver: mpsc::Receiver<(DecisionResult, OwnedSemaphorePermit)>,
    replay: Arc<ReplayLog>,
    replay_path: PathBuf,
    runtime: tokio::runtime::Handle,
}

impl AiService {
    pub fn new(mode: &str, key: Option<String>, seed: u64) -> Result<Self, String> {
        if !matches!(mode, "laya" | "heuristics") {
            return Err("unknown_ai_mode".into());
        }
        let runtime = tokio::runtime::Handle::try_current()
            .map_err(|_| "tokio_runtime_required".to_string())?;
        let gateway_url =
            std::env::var("LAYA_GATEWAY_URL").unwrap_or_else(|_| DEFAULT_GATEWAY_URL.into());
        let live = if mode == "laya" {
            Some(Arc::new(LayaProvider::new(gateway_url, key)?))
        } else {
            None
        };
        fs::create_dir_all("replay_logs")
            .map_err(|error| format!("replay_directory_{:?}", error.kind()))?;
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| "clock_before_epoch".to_string())?
            .as_nanos();
        let mut serial = 0u32;
        let (replay_path, file) = loop {
            let path = PathBuf::from(format!(
                "replay_logs/run-{seed}-{stamp}-{}-{serial}.jsonl",
                std::process::id()
            ));
            match OpenOptions::new().create_new(true).write(true).open(&path) {
                Ok(file) => break (path, file),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => serial += 1,
                Err(error) => return Err(format!("replay_open_{:?}", error.kind())),
            }
        };
        let replay = Arc::new(ReplayLog {
            file: Mutex::new(file),
            failure: Mutex::new(None),
        });
        replay.append(&json!({ "event": "session", "seed": seed, "provider": if live.is_some() { "laya" } else { "heuristics" },
            "input_price_per_million_usd_estimate": INPUT_PRICE_PER_MILLION,
            "cost_note": "Self-hosted Laya checkpoints: input tokens are reported for budgeting; the marginal cost of a local forward pass is zero. Confidence is provider supplied when present, otherwise normalized distribution concentration." }))?;
        let (sender, receiver) = mpsc::channel(CAPACITY);
        Ok(Self {
            heuristics: AtomicBool::new(live.is_none()),
            live,
            offline: Arc::new(HeuristicProvider),
            slots: Arc::new(Semaphore::new(CAPACITY)),
            sender,
            receiver,
            replay,
            replay_path,
            runtime,
        })
    }

    pub fn is_heuristic(&self) -> bool {
        self.heuristics.load(Ordering::Acquire)
    }
    pub fn log_path(&self) -> &Path {
        &self.replay_path
    }
    pub fn switch_to_heuristics(&self) {
        self.heuristics.store(true, Ordering::Release);
    }

    pub fn submit(&self, request: DecisionRequest) -> bool {
        let Ok(permit) = self.slots.clone().try_acquire_owned() else {
            return false;
        };
        if self.is_heuristic() {
            // Offline arrival order is input order, independent of runtime scheduling.
            if let Err(error) = self
                .replay
                .append(&json!({"event":"request","request":&request}))
            {
                self.replay.remember_failure(error);
            }
            let result = heuristic(&request);
            if let Err(error) = self
                .replay
                .append(&json!({"event":"result","result":&result,"estimated_input_cost_usd":0.0}))
            {
                self.replay.remember_failure(error);
            }
            return self.sender.try_send((result, permit)).is_ok();
        }
        let provider: Arc<dyn DecisionProvider> = if !self.is_heuristic() {
            self.live
                .as_ref()
                .map(|provider| provider.clone() as Arc<dyn DecisionProvider>)
                .unwrap_or_else(|| self.offline.clone())
        } else {
            self.offline.clone()
        };
        let sender = self.sender.clone();
        let replay = self.replay.clone();
        self.runtime.spawn(async move {
            if let Err(error) = replay.append(&json!({ "event": "request", "request": &request })) { replay.remember_failure(error); }
            let result = provider.decide(request).await;
            if let Err(error) = replay.append(&json!({ "event": "result", "result": &result,
                "estimated_input_cost_usd": result.input_tokens as f64 * INPUT_PRICE_PER_MILLION / 1_000_000.0 })) { replay.remember_failure(error); }
            // Keeping the permit with the result bounds completed-but-undrained work too.
            let _ = sender.send((result, permit)).await;
        });
        true
    }

    pub fn drain(&mut self) -> Vec<DecisionResult> {
        let mut results = Vec::new();
        while let Ok((result, _permit)) = self.receiver.try_recv() {
            results.push(result);
        }
        results
    }

    pub fn record(&self, record: &DecisionRecord) -> Result<(), String> {
        let pending = self
            .replay
            .failure
            .lock()
            .map_err(|_| "replay_lock_failed".to_string())?
            .take();
        self.replay
            .append(&json!({ "event": "applied", "record": record }))?;
        if let Some(error) = pending {
            return Err(error);
        }
        Ok(())
    }
}

// ---- Session plumbing shared by every view (terminal, windowed, harnesses).

/// Drains completed decisions, applies them, then submits any new requests.
pub fn pump(game: &mut crate::model::Game, ai: &mut AiService) {
    // Answers update intentions, never advance turns. Drain even in menus so social
    // requests cannot deadlock behind eight completed tactical judgments.
    for result in ai.drain() {
        apply_result(game, ai, result);
    }
    let mut remaining = Vec::new();
    for request in game.outbox.drain(..) {
        if ai.submit(request.clone()) {
            game.ai.pending += 1;
        } else {
            remaining.push(request);
        }
    }
    game.outbox = remaining;
}

/// Applies one completed decision with session-level failure accounting.
pub fn apply_result(game: &mut crate::model::Game, ai: &mut AiService, result: DecisionResult) {
    game.ai.pending = game.ai.pending.saturating_sub(1);
    game.ai.requests += 1;
    game.ai.latency_ms = result.latency_ms;
    game.ai.input_tokens += result.input_tokens;
    if let Some(error) = &result.error {
        game.ai.degraded += 1;
        game.ai.consecutive_failures += 1;
        if game.ai.consecutive_failures == 1 {
            game.log(format!(
                "Laya gateway unavailable ({error}). This decision uses heuristics."
            ));
        }
        if game.ai.consecutive_failures >= 5 && !game.ai.switched {
            game.ai.switched = true;
            game.ai.provider = "heuristics".into();
            ai.switch_to_heuristics();
            game.log("Five consecutive Laya failures: offline decisions enabled for this session.");
        }
    } else if result.provider == "laya" {
        game.ai.consecutive_failures = 0;
    }
    let applied_rule = game.apply_decision(&result);
    let record = DecisionRecord {
        result,
        applied_rule,
    };
    if let Err(error) = ai.record(&record) {
        game.log(format!("Replay log unavailable: {error}"));
    }
    if game.decisions.len() == 10 {
        game.decisions.pop_front();
    }
    game.decisions.push_back(record);
}

fn noul(instructions: &str) -> Question {
    Question {
        kind: "noul".into(),
        instructions: instructions.into(),
        criteria: None,
    }
}
fn choice(instructions: &str, options: &[(&str, &str)]) -> Question {
    Question {
        kind: "choice".into(),
        instructions: instructions.into(),
        criteria: Some(json!(options.iter().copied().collect::<BTreeMap<_, _>>())),
    }
}
fn score(instructions: &str, bands: &[&str]) -> Question {
    Question {
        kind: "score".into(),
        instructions: instructions.into(),
        criteria: Some(json!(bands)),
    }
}
fn targets() -> Question {
    choice(
        "Who should this NPC focus on given injuries, loyalty, and danger?",
        &[
            ("player", "the adventurer"),
            ("packmate", "protect a wounded ally"),
            ("cattle", "seek safer prey instead"),
        ],
    )
}

pub fn question_bank(archetype: Archetype, tier: u8) -> BTreeMap<String, Question> {
    use Archetype::*;
    let mut questions = BTreeMap::new();
    let mut add = |key: &str, question: Question| {
        questions.insert(key.into(), question);
    };
    if tier == 0 || tier > 4 {
        return questions;
    }
    if tier == 4 {
        add(
            "prophecy",
            choice(
                "Which warning best fits this adventurer's recorded deeds and current danger?",
                &[
                    ("danger", "prepare for the next battle"),
                    ("fortune", "seek lost treasure"),
                    ("mercy", "show compassion"),
                ],
            ),
        );
        add(
            "identify_relic",
            score(
                "How strongly does the relic resonate with this adventurer's experience and deeds?",
                &["dormant", "stirring", "awakened", "radiant"],
            ),
        );
        if archetype != Adjudicator {
            return questions;
        }
    }
    match archetype {
        Commoner => {
            add("flee_from_player", noul("Should this civilian flee the player, based on witnessed violence and disposition?"));
            add(
                "alert_guards",
                noul("Has the player done something dangerous enough to alert nearby guards?"),
            );
            if tier == 2 {
                add(
                    "share_rumour",
                    noul("Would this civilian share a useful local rumour with this player?"),
                );
            }
        }
        Vendor | Alchemist => {
            if tier == 1 {
                add(
                    "flee_from_player",
                    noul("Is this vendor in immediate danger from the player?"),
                );
                add(
                    "alert_guards",
                    noul("Should this vendor call the watch because of the player's conduct?"),
                );
            } else {
                add("haggle_accept", noul("Should the vendor accept a modest discount, considering greed, local reputation and personal memory?"));
                add("cheat_player", noul("Would this vendor overcharge this player, given greed and the risk of losing goodwill?"));
                add(
                    "offer_quest",
                    choice(
                        "What type of work should this vendor offer this traveller?",
                        &[
                            ("delivery", "carry a parcel to another city"),
                            ("escort", "guard a caravan leg"),
                            ("collection", "retrieve owed goods"),
                        ],
                    ),
                );
            }
        }
        Guard => {
            add("suspect_player", noul("Is this player suspicious based on observed misconduct, reputation and the guard's memory?"));
            add("pursue_fleeing_player", noul("Should this guard pursue the player rather than continue protecting civilians?"));
            if tier == 2 {
                add("accept_bribe", noul("Would this guard accept a gold bribe? Consider gullibility, greed, reputation and witnesses."));
            }
        }
        Thief => {
            add("steal_from_player", noul("Is stealing from this player worthwhile and safe given wealth, guards, witnesses, and night cover?"));
            add(
                "flee_when_noticed",
                noul("Should the thief run when this player notices them?"),
            );
        }
        Traveller => {
            add("share_rumour", noul("Would this traveller share a useful rumour given friendliness and willingness to chat?"));
            add(
                "warn_of_danger",
                noul("Would this traveller warn the player about nearby threats?"),
            );
        }
        Companion => {
            add("share_rumour", noul("Would this blade-for-hire share a remark about the road given loyalty and disposition?"));
            add(
                "warn_of_danger",
                noul("Would this companion call out a nearby threat to their employer?"),
            );
        }
        Smuggler => {
            add(
                "share_rumour",
                noul("Does this smuggler trust the player enough to reveal a contact or route?"),
            );
            add(
                "return_goods_or_sell_to_fence",
                choice(
                    "What advice would this smuggler give about recovered caravan goods?",
                    &[
                        ("return", "return goods to their rightful owner"),
                        ("fence", "sell stolen goods for private profit"),
                    ],
                ),
            );
        }
        Oracle => {
            add(
                "share_rumour",
                noul("Is this adventurer ready to hear guidance about the three sigils?"),
            );
            add(
                "warn_of_danger",
                noul("Does this adventurer need a warning before entering the dungeons?"),
            );
        }
        Bandit | Wolf | Bear => {
            add(if archetype == Bandit { "ambush_player" } else { "hunt_player" }, noul("Should this hostile NPC attack the visible player now, weighing its health, courage, allies and relative levels?"));
            add("flee", score("How close is this NPC to breaking and running? Healthy confident predators should stand their ground; severe wounds and losses cause fear.", &["steady", "nervous", "afraid", "panicking"]));
            add("target_pick", targets());
            if archetype == Bandit {
                add("mercy", noul("Would this bandit allow an already beaten player to escape once rather than kill them?"));
            }
        }
        Rat | Skeleton => {
            add(
                "aggro",
                noul("Should this dungeon creature attack the visible intruder?"),
            );
            add(
                "retreat_to_group",
                noul("Is this wounded isolated creature better off regrouping with living allies?"),
            );
        }
        BroodHole | FalseGlow => {}
        Chief
        | Matriarch
        | Lich
        | Adjudicator
        | Tidemother
        | Cragmother
        | GnawThane
        | Tollmaster
        | Mirelight
        | PaleStag
        | OathlessCurate => {
            add(
                "aggro",
                noul("Should this dungeon lord defend its lair against this intruder?"),
            );
            add("mercy", noul("Would this boss permit this weakened player one escape, considering past mercy and its own spite?"));
            if tier >= 3 {
                match archetype {
                    Tidemother => {
                        add(
                            "drag_who",
                            choice(
                                "Which engaged hero should the undertow seize by the legs: the journeyer or their hired blade?",
                                &[
                                    ("player", "drag the journeyer"),
                                    ("companion", "drag the hired blade"),
                                ],
                            ),
                        );
                        add(
                            "sacrifice_crew",
                            noul("Would the Tidemother drown one of her drowned crew to mend deep wounds?"),
                        );
                    }
                    Cragmother => {
                        add(
                            "commit_slam",
                            noul("Should the Cragmother commit to the Avalanche Slam now rather than reposition for a better angle?"),
                        );
                        add(
                            "guard_cubs",
                            noul("Should the Cragmother roar her cubs out of the nests to guard them in this deep phase?"),
                        );
                    }
                    Chief => {
                        add(
                            "rally_minions",
                            noul("Would rallying surviving minions turn this battle?"),
                        );
                        add("sacrifice_minion", noul("Would this ruthless chief sacrifice a minion to survive severe wounds?"));
                        add(
                            "morale",
                            score(
                                "How strong is the surviving warband's morale?",
                                &["broken", "shaken", "resolute", "fervent"],
                            ),
                        );
                    }
                    Matriarch => {
                        add("howl_summon", noul("Should the matriarch howl for reinforcements based on surviving pack size and battle pressure?"));
                        add("target_pick", targets());
                    }
                    Lich => {
                        add("tactic", choice("Choose the Lich's next tactic based on phase, wounds, minions and player defenses.", &[("pressure", "aggressive damage focus"), ("summon", "raise skeletons to delay"), ("curse", "debuff the player's defense"), ("retreat", "fall back and recover")]));
                        add("focus_target", targets());
                        add(
                            "phase_shift",
                            noul("Has the fight turned against the Lich enough to change phases?"),
                        );
                        add(
                            "desperation",
                            score(
                                "How desperate is this wounded Lich in the current phase?",
                                &["composed", "concerned", "worried", "frantic"],
                            ),
                        );
                    }
                    Adjudicator => {
                        add("rate_mercy", score("Rate the player's mercy from recorded mercy and kills, not wealth or skill.", &["merciless", "severe", "compassionate", "merciful"]));
                        add("rate_greed", score("Rate the player's greed from thefts, bribes, quests and other deeds.", &["selfless", "practical", "grasping", "avaricious"]));
                        add("rate_courage", score("Rate the player's courage from battles fought, defenses, victories and escapes.", &["timid", "cautious", "brave", "fearless"]));
                        add("adapt_tactic", choice("Adapt the trial to the player's deeds and previous phase: honourable players earn a duel; ruthless or evasive ones face their own tactics.", &[("duel", "honourable straightforward combat"), ("punish", "punish cruelty with overwhelming attacks"), ("deceive", "punish greed and evasiveness with tricks"), ("endure", "test patience and defensive play")]));
                    }
                    // E5 side bosses (§6): each teaches one lesson, and Laya holds
                    // the two knobs per fight the design table names.
                    GnawThane => {
                        add(
                            "call_the_tide",
                            noul("Should the Rat-King call the Plague Tide this cycle, spending the brood-holes' strength to drown the intruder in bodies?"),
                        );
                        add(
                            "scatter_when_thinned",
                            noul("Badly mauled, should the Rat-King scatter between the holes and let the warren fight on without his bulk?"),
                        );
                    }
                    Tollmaster => {
                        add(
                            "shove_now",
                            noul("Should the Tollmaster's crew throw their shoulders now and shove the heroes toward the water?"),
                        );
                        add(
                            "collect_or_cut",
                            choice(
                                "Does the Tollmaster press the fight, or turn his greed to the caravan's straps for a breath?",
                                &[
                                    ("press", "press the heroes toward the ford"),
                                    ("loot", "cut the caravan's straps and count takings"),
                                ],
                            ),
                        );
                    }
                    Mirelight => {
                        add(
                            "which_light",
                            choice(
                                "Which light carries the true wisp's tell this phase — the one that hunts the traveller's light?",
                                &[
                                    ("west", "the left-hand glow"),
                                    ("centre", "the middle glow"),
                                    ("east", "the right-hand glow"),
                                ],
                            ),
                        );
                        add(
                            "strike_or_subside",
                            noul("While the false lights hover, should the wisp's lights strike the traveller or subside and wait out the confusion?"),
                        );
                    }
                    PaleStag => {
                        add(
                            "break_and_run",
                            score(
                                "How hard does the wounded stag break: a quick sidestep to breathe, or a full bolt for the treeline?",
                                &["sidestep", "lope", "bolt", "vanish"],
                            ),
                        );
                        add(
                            "stand_ground",
                            noul("Cornered or hardened to the hunt, should the stag stop running and fight to the last breath?"),
                        );
                    }
                    // E7 (§6.5): the covenant-breaker, with the two knobs the
                    // table grants him — the falling sigil and the offered knee.
                    OathlessCurate => {
                        add(
                            "which_sigil_falls",
                            choice(
                                "Which of the traveller's gathered sigils should the Curate unwrite next, judged by what their build leans on hardest?",
                                &[
                                    ("first", "the Burrow sigil (the Chief's)"),
                                    ("second", "the Crimson sigil (the Matriarch's)"),
                                    ("third", "the Underkeep sigil (Vael's)"),
                                ],
                            ),
                        );
                        add(
                            "mercy_for_the_oathbreaker",
                            noul("Broken and kneeling, does the Curate beg quarter sincerely enough that this traveller might grant it?"),
                        );
                    }
                    _ => unreachable!(),
                }
            }
        }
    }
    questions
}

fn number(state: &Value, pointer: &str, default: f32) -> f32 {
    state
        .pointer(pointer)
        .and_then(Value::as_f64)
        .filter(|v| v.is_finite())
        .map(|v| v as f32)
        .unwrap_or(default)
}
fn stable_hash(mut hash: u64, bytes: &[u8]) -> u64 {
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

pub fn heuristic(request: &DecisionRequest) -> DecisionResult {
    let state = &request.state;
    let state_bytes = serde_json::to_vec(state).unwrap_or_default();
    let base = stable_hash(mix(request.seed ^ request.npc as u64), &state_bytes);
    let hp = (number(state, "/npc/hp", 10.0) / number(state, "/npc/max_hp", 10.0).max(1.0))
        .clamp(0.0, 1.0);
    let player_hp = (number(state, "/player/hp", 20.0)
        / number(state, "/player/max_hp", 20.0).max(1.0))
    .clamp(0.0, 1.0);
    let brave = number(state, "/npc/personality/brave", 0.6);
    let greedy = number(state, "/npc/personality/greedy", 0.5);
    let gullible = number(state, "/npc/personality/gullibility", 0.4);
    let spite = number(state, "/npc/personality/spite", 0.4);
    let chatty = number(state, "/npc/personality/chatty", 0.6);
    let disposition = number(state, "/memory/disposition", 0.0);
    let reputation = number(state, "/player/reputation", 50.0) / 100.0;
    let allies = number(state, "/self/allies_alive", 0.0);
    let dead = number(state, "/self/allies_dead", 0.0);
    let losses = dead / (allies + dead).max(1.0);
    let threat = ((number(state, "/player/level", 1.0) - number(state, "/npc/level", 1.0)) / 6.0)
        .clamp(-0.5, 0.6);
    let fear = ((1.0 - hp) * 0.8 + losses * 0.25 + threat * 0.3 - brave * 0.18).clamp(0.02, 0.98);
    let suspicion = ((0.4 - reputation) * 1.3 - disposition * 0.65).clamp(0.02, 0.98);
    let mercy = number(state, "/history/mercy", 0.0);
    let kills = number(state, "/history/kills", 0.0);
    let thefts = number(state, "/history/thefts", 0.0);
    let bribes = number(state, "/history/bribes", 0.0);
    let fled = number(state, "/history/fled", 0.0);
    let mut answers = BTreeMap::new();
    for (key, question) in &request.questions {
        let hash = mix(stable_hash(base, key.as_bytes()));
        let jitter = ((hash >> 40) as f32 / 16_777_215.0 - 0.5) * 0.16;
        let raw = match key.as_str() {
            "flee" | "desperation" => fear,
            "morale" => 1.0 - fear,
            "ambush_player" | "hunt_player" => 0.75 + brave * 0.15 - fear * 0.55 - threat * 0.15,
            "aggro" => 0.91 - fear * 0.15,
            "retreat_to_group" => {
                if allies > 0.0 {
                    (1.0 - hp) * 0.8
                } else {
                    0.03
                }
            }
            "suspect_player" | "alert_guards" => suspicion,
            "pursue_fleeing_player" => suspicion + brave * 0.15,
            "flee_from_player" => suspicion + (1.0 - brave) * 0.1,
            "accept_bribe" => {
                0.2 + greedy * 0.3 + gullible * 0.55
                    - number(state, "/environment/witnesses_nearby", 0.0) * 0.02
            }
            "haggle_accept" => 0.58 + disposition * 0.2 + reputation * 0.25 - greedy * 0.35,
            "cheat_player" => greedy * 0.8 - reputation * 0.35 - disposition * 0.15,
            "steal_from_player" => {
                let hour = number(state, "/environment/time", 12.0);
                let night = !(4.0..22.0).contains(&hour);
                let rich = state
                    .pointer("/player/looks_rich")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                0.12 + greedy * 0.4 + if night { 0.25 } else { 0.0 } + if rich { 0.18 } else { 0.0 }
                    - number(state, "/environment/guards_nearby", 0.0) * 0.22
            }
            "flee_when_noticed" => 0.85 - brave * 0.12,
            "share_rumour" => 0.42 + chatty * 0.38 + disposition * 0.2,
            "warn_of_danger" => 0.65 + chatty * 0.2 + disposition * 0.15,
            "mercy" => 0.48 + (1.0 - player_hp) * 0.22 + mercy.min(5.0) * 0.04 - spite * 0.28,
            "rally_minions" => {
                if allies > 0.0 {
                    0.74 + losses * 0.2
                } else {
                    0.1
                }
            }
            "sacrifice_minion" => {
                if allies > 0.0 {
                    (1.0 - hp) * 0.8 + spite * 0.2
                } else {
                    0.02
                }
            }
            "howl_summon" => 0.85 - allies.min(4.0) * 0.16,
            // E3 arena bosses: drowned math and den-mother instinct.
            "sacrifice_crew" => 0.15 + (1.0 - hp) * 0.75,
            "commit_slam" => {
                if hp > 0.3 {
                    0.68
                } else {
                    0.42
                }
            }
            "guard_cubs" => {
                if allies < 2.0 && hp < 0.67 {
                    0.72
                } else {
                    0.28
                }
            }
            // E5 side bosses (§6): the fallback reads the same math Laya would.
            "call_the_tide" => 0.78 - allies * 0.09,
            "scatter_when_thinned" => {
                if hp < 0.4 {
                    0.72
                } else {
                    0.1
                }
            }
            "shove_now" => 0.66 - (1.0 - hp) * 0.25,
            "strike_or_subside" => 0.42 + hp * 0.3,
            "stand_ground" => {
                if hp < 0.25 && allies == 0.0 {
                    0.68
                } else {
                    0.18
                }
            }
            "break_and_run" => (1.0 - hp) * 0.9 + 0.05,
            // E7: the Curate begs when the fight is truly lost, and unwrits the
            // sigil the build most leans on (fallback reads hash order).
            "mercy_for_the_oathbreaker" => {
                if hp < 0.2 {
                    0.6 + mercy.min(3.0) * 0.1
                } else {
                    0.05
                }
            }
            "phase_shift" => (1.0 - hp) * 1.25,
            "rate_mercy" => (mercy * 3.0 + 1.0) / (mercy * 3.0 + kills * 0.2 + 2.0),
            "rate_greed" => {
                (thefts * 2.0 + bribes + 0.5)
                    / (thefts * 2.0 + bribes + number(state, "/history/quests", 0.0) + 3.0)
            }
            "rate_courage" => (kills + 2.0) / (kills + fled * 3.0 + 3.0),
            "identify_relic" => 0.45 + number(state, "/player/level", 1.0) * 0.045,
            _ => 0.5,
        };
        let value = (raw + jitter).clamp(0.01, 0.99);
        let answer = match question.kind.as_str() {
            "choice" => {
                let options: Vec<String> = question
                    .criteria
                    .as_ref()
                    .and_then(Value::as_object)
                    .map(|options| options.keys().cloned().collect())
                    .unwrap_or_default();
                let preferred = match key.as_str() {
                    "drag_who" => {
                        // Confident, she takes the leader; beleaguered, the easier mark.
                        if hp > 0.5 {
                            "player"
                        } else {
                            "companion"
                        }
                    }
                    "collect_or_cut" => {
                        // Winning presses; losing paws the straps.
                        if hp > 0.5 {
                            "press"
                        } else {
                            "loot"
                        }
                    }
                    "which_light" => ["west", "centre", "east"][(hash % 3) as usize],
                    "which_sigil_falls" => ["first", "second", "third"][(hash % 3) as usize],
                    "target_pick" | "focus_target" => {
                        if hp < 0.25 && allies > 0.0 {
                            "packmate"
                        } else {
                            "player"
                        }
                    }
                    "tactic" => match (number(state, "/npc/phase", 1.0) as u64 + hash % 3) % 4 {
                        0 if hp < 0.5 => "retreat",
                        1 if allies < 3.0 => "summon",
                        2 => "curse",
                        _ => "pressure",
                    },
                    "adapt_tactic" => {
                        let recent = state
                            .pointer("/combat/recent_actions")
                            .and_then(Value::as_array);
                        let defenses = recent.map_or(0, |a| {
                            a.iter().filter(|v| v.as_str() == Some("defend")).count()
                        });
                        let attacks = recent.map_or(0, |a| {
                            a.iter().filter(|v| v.as_str() == Some("attack")).count()
                        });
                        if thefts + bribes > 3.0 || defenses >= 3 {
                            "deceive"
                        } else if attacks >= 6 {
                            "endure"
                        } else if kills > 15.0 && mercy == 0.0 {
                            "punish"
                        } else if fled > 3.0 {
                            "endure"
                        } else {
                            "duel"
                        }
                    }
                    "prophecy" => {
                        if player_hp < 0.5 {
                            "danger"
                        } else if mercy > 1.0 {
                            "mercy"
                        } else {
                            "fortune"
                        }
                    }
                    "return_goods_or_sell_to_fence" => {
                        if greedy > 0.6 {
                            "fence"
                        } else {
                            "return"
                        }
                    }
                    _ => "",
                };
                let selected = options
                    .iter()
                    .position(|option| option == preferred)
                    .unwrap_or_else(|| {
                        if options.is_empty() {
                            0
                        } else {
                            hash as usize % options.len()
                        }
                    });
                let chosen = options.get(selected).cloned();
                let best = if options.len() <= 1 {
                    1.0
                } else {
                    (0.76 + jitter).clamp(0.6, 0.9)
                };
                let rest = if options.len() > 1 {
                    (1.0 - best) / (options.len() - 1) as f32
                } else {
                    0.0
                };
                let probabilities = options
                    .into_iter()
                    .enumerate()
                    .map(|(i, option)| (option, if i == selected { best } else { rest }))
                    .collect();
                Answer {
                    value: best,
                    choice: chosen,
                    probabilities,
                    confidence: best,
                }
            }
            "score" => {
                let bands = question.criteria.as_ref().and_then(Value::as_array);
                let count = bands.map_or(0, Vec::len);
                let position = value * count.saturating_sub(1) as f32;
                let low = position.floor() as usize;
                let high = position.ceil() as usize;
                let fraction = position.fract();
                let probabilities = (0..count)
                    .map(|i| {
                        (
                            i.to_string(),
                            if low == high && i == low {
                                1.0
                            } else if i == low {
                                1.0 - fraction
                            } else if i == high {
                                fraction
                            } else {
                                0.0
                            },
                        )
                    })
                    .collect();
                let band = bands
                    .and_then(|bands| bands.get(position.round() as usize))
                    .and_then(Value::as_str)
                    .map(str::to_owned);
                Answer {
                    value,
                    choice: band,
                    probabilities,
                    confidence: (fraction - 0.5).abs() * 2.0,
                }
            }
            _ => Answer {
                value,
                choice: None,
                probabilities: BTreeMap::from([
                    ("false".into(), 1.0 - value),
                    ("true".into(), value),
                ]),
                confidence: (value - 0.5).abs() * 2.0,
            },
        };
        answers.insert(key.clone(), answer);
    }
    DecisionResult {
        request: request.clone(),
        provider: "heuristics".into(),
        answers,
        latency_ms: 0,
        input_tokens: 0,
        error: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn request() -> DecisionRequest {
        DecisionRequest {
            seed: 42,
            npc: 7,
            npc_name: "Ash".into(),
            version: 1,
            turn: 4,
            tier: 1,
            state: json!({ "npc": { "hp": 20, "max_hp": 20, "level": 2, "personality": { "brave": 0.7 } }, "player": { "level": 2 } }),
            questions: question_bank(Archetype::Bandit, 1),
        }
    }
    #[test]
    fn deterministic_heuristic_stands_ground_then_reacts_to_wounds() {
        let mut request = request();
        let a = heuristic(&request);
        let b = heuristic(&request);
        assert_eq!(
            serde_json::to_value(a.answers.clone()).unwrap(),
            serde_json::to_value(b.answers).unwrap()
        );
        assert!(a.answers["ambush_player"].value > 0.6);
        assert!(a.answers["flee"].value < 0.2);
        request.state["npc"]["hp"] = json!(1);
        let wounded = heuristic(&request);
        assert!(wounded.answers["flee"].value > a.answers["flee"].value + 0.4);
    }
    fn decoding_fixture() -> (BTreeMap<String, Question>, Value) {
        (
            BTreeMap::from([
                ("go".into(), noul("Go?")),
                (
                    "target".into(),
                    choice("Choose", &[("player", "enemy"), ("packmate", "friend")]),
                ),
                (
                    "fear".into(),
                    score("Fear?", &["steady", "nervous", "afraid", "panic"]),
                ),
            ]),
            json!({ "answers": {
            "go": { "type": "boolean", "probability": 0.8 },
            "target": { "type": "choice", "choice": "player", "probabilities": { "player": 0.8, "packmate": 0.2 }, "confidence": 0.65 },
            "fear": { "type": "score", "score": 2.4, "probabilities": { "0": 0.0, "1": 0.0, "2": 0.6, "3": 0.4 } }
        }, "usage": { "inputTokens": 123 } }),
        )
    }
    #[test]
    fn gateway_choice_and_raw_score_decode_without_losing_distribution() {
        let (questions, raw) = decoding_fixture();
        let answers = decode_answers(&questions, &raw).unwrap();
        assert_eq!(answers["target"].choice.as_deref(), Some("player"));
        assert_eq!(answers["target"].probabilities["packmate"], 0.2);
        assert_eq!(answers["target"].confidence, 0.65);
        assert!((answers["fear"].value - 0.8).abs() < 0.0001);
        assert_eq!(answers["fear"].choice.as_deref(), Some("afraid"));
        assert_eq!(answers["go"].value, 0.8);
    }
    #[test]
    fn malformed_or_partial_answer_rejects_entire_request() {
        let (questions, raw) = decoding_fixture();
        let mutations = [
            ("/answers/go/probability", json!(1.2)),
            ("/answers/go/type", json!("score")),
            ("/answers/target/choice", json!("cattle")),
            ("/answers/target/choice", json!("packmate")),
            ("/answers/target/probabilities", json!({ "player": 1.0 })),
            (
                "/answers/target/probabilities",
                json!({ "player": 0.9, "packmate": 0.9 }),
            ),
            ("/answers/target/confidence", json!(-0.1)),
            ("/answers/fear/score", json!(4.0)),
            (
                "/answers/fear/probabilities",
                json!({ "0": 0.0, "1": 0.0, "2": 0.0, "wrong": 1.0 }),
            ),
            ("/usage/inputTokens", json!(-1)),
        ];
        for (pointer, mutation) in mutations {
            let mut invalid = raw.clone();
            *invalid.pointer_mut(pointer).unwrap() = mutation;
            assert!(
                decode_answers(&questions, &invalid).is_err(),
                "accepted {pointer}"
            );
        }
        let mut partial = raw.clone();
        partial["answers"].as_object_mut().unwrap().remove("go");
        assert!(decode_answers(&questions, &partial).is_err());
        let mut bad_legend = raw;
        bad_legend["answers"]["fear"]["legend"] =
            json!({ "0": "steady", "1": "nervous", "2": "afraid", "3": "WRONG" });
        assert!(decode_answers(&questions, &bad_legend).is_err());
    }
    #[test]
    fn typesafe_noul_and_score_legend_are_validated() {
        let (questions, mut raw) = decoding_fixture();
        raw["answers"]["go"] = json!({ "type": "noul", "noul": 0.7 });
        raw["usage"] = json!({ "input_tokens": 456 });
        raw["answers"]["fear"]["legend"] =
            json!({ "0": "steady", "1": "nervous", "2": "afraid", "3": "panic" });
        assert_eq!(decode_answers(&questions, &raw).unwrap()["go"].value, 0.7);
    }
}
