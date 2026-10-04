#!/usr/bin/env python3
"""Laya gateway — serves the /v1/evaluate decision dialect on top of the laya SDK.

Clients (Go `jev-cicd` backend, Rust `jev-realms` game) speak this wire contract:

  POST /v1/evaluate
  {"model": "convaiinnovations/laya", "state": {...}, "questions": {
      "name": {"type": "boolean|choice|score", "instructions": "...",
               "criteria": {...}|[...] }}}

  -> {"model": "laya", "answers": {
      "name": {"type": "boolean", "probability": 0..1} |
              {"type": "choice", "choice": "key", "probabilities": {...}} |
              {"type": "score", "score": 0..n-1, "probabilities": {...},
               "legend": {...}}},
      "usage": {"inputTokens": N, "input_tokens": N, "output_tokens": 0},
      "routing": {...}}

The gateway-era "boolean" primitive is translated to laya's native "noul" on
the way in and back to "boolean"/"probability" on the way out, so clients keep
their existing wire validation while running on open Apache-2.0 weights.

Configuration (environment):
  LAYA_GATEWAY_HOST   bind host                        (default 127.0.0.1)
  LAYA_GATEWAY_PORT   bind port                        (default 8128)
  LAYA_DEVICE         cuda|cpu|mps                     (default: auto)
  LAYA_MODELS         comma list of checkpoints to preload
                                                        (default english,multilingual)
  LAYA_GATEWAY_TOKEN  require "Authorization: Bearer <token>" (default: none)

The socket binds immediately; /health returns 503 ("loading") until the
checkpoints are built, then 200. Point clients at
http://127.0.0.1:8128/v1/evaluate (LAYA_GATEWAY_URL).
"""
import hmac
import json
import os
import threading
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from urllib.parse import urlsplit

# transformers probes for TensorFlow at import; when TF is installed its
# abseil runtime can deadlock model construction (see the model card).
os.environ.setdefault("USE_TF", "0")

# Request "model" strings that force a specific checkpoint. Anything else —
# including the client default "convaiinnovations/laya" — auto-routes
# (script/language detection picks english vs multilingual per request).
MODEL_OVERRIDES = {
    "convaiinnovations/laya-multilingual": "multilingual",
    "convaiinnovations/laya-typed-decisions": "typed-decisions",
    "laya-multilingual": "multilingual",
    "laya-typed-decisions": "typed-decisions",
    "typed_decisions": "typed-decisions",
    "multilingual": "multilingual",
    "english": "english",
    "en": "english",
}

MAX_BODY = 10 << 20  # 10 MiB, matching the clients' readJSON limit


class _State:
    def __init__(self):
        self.lock = threading.Lock()  # serialises forward passes
        self.router = None
        self.ready = False
        self.error = None
        self.requests = 0


_STATE = _State()


def _load():
    try:
        from laya import Router

        names = [m.strip() for m in os.environ.get("LAYA_MODELS", "english,multilingual").split(",") if m.strip()]
        router = Router(device=os.environ.get("LAYA_DEVICE") or None)
        router.preload(names)
        with _STATE.lock:
            _STATE.router = router
            _STATE.ready = True
        device = "auto"
        for agent in router._agents.values():
            device = str(agent.device)
            break
        print("[laya-gateway] ready: models=%s device=%s" % (router.loaded, device), flush=True)
    except Exception as exc:  # noqa: BLE001 - reported verbatim to /health
        _STATE.error = "%s: %s" % (type(exc).__name__, exc)
        print("[laya-gateway] load failed: %s" % _STATE.error, flush=True)


class _NotReady(Exception):
    pass


def _renormalize(probs, prefer=None):
    """Round to 4 dp and force the distribution to sum to exactly 1.0.

    laya already rounds to 4 dp; the residual (<= k*5e-5) is added to the
    preferred key so the reported argmax always matches the reported choice.
    """
    out = {str(k): round(float(v), 4) for k, v in probs.items()}
    if not out:
        return out
    residual = round(1.0 - sum(out.values()), 6)
    if residual:
        key = prefer if prefer in out else max(out, key=out.get)
        out[key] = round(out[key] + residual, 6)
    return out


def _to_laya_questions(questions_in):
    questions = {}
    for qid, q in questions_in.items():
        if not isinstance(q, dict):
            raise ValueError("question %r must be an object" % qid)
        qtype = q.get("type")
        if qtype == "boolean":
            qtype = "noul"
        if qtype not in ("choice", "score", "noul"):
            raise ValueError("question %r has unknown type %r" % (qid, q.get("type")))
        if not isinstance(q.get("instructions"), str):
            raise ValueError("question %r needs string instructions" % qid)
        out = {"type": qtype, "instructions": q["instructions"]}
        if "criteria" in q:
            out["criteria"] = q["criteria"]
        questions[qid] = out
    return questions


def _translate(result):
    answers = {}
    for qid, ans in result.get("answers", {}).items():
        atype = ans.get("type")
        if atype == "noul":
            answers[qid] = {
                "type": "boolean",
                "probability": ans.get("noul"),
                "confidence": ans.get("confidence"),
            }
        elif atype == "choice":
            answers[qid] = {
                "type": "choice",
                "choice": ans.get("choice"),
                "probabilities": _renormalize(ans.get("probabilities", {}), prefer=ans.get("choice")),
                "confidence": ans.get("confidence"),
            }
        elif atype == "score":
            answers[qid] = {
                "type": "score",
                "score": ans.get("score"),
                "probabilities": _renormalize(ans.get("probabilities", {})),
                "legend": ans.get("legend"),
                "confidence": ans.get("confidence"),
            }
        else:
            raise ValueError("answer %r has unknown type %r" % (qid, atype))
    usage = result.get("usage", {})
    tokens = usage.get("input_tokens", 0)
    return {
        "model": "laya",
        "answers": answers,
        "usage": {
            "inputTokens": tokens,
            "input_tokens": tokens,
            "output_tokens": usage.get("output_tokens", 0),
        },
        "routing": result.get("routing"),
    }


def evaluate(body):
    if not isinstance(body, dict):
        raise ValueError("request body must be a JSON object")
    questions_in = body.get("questions")
    if not isinstance(questions_in, dict) or not questions_in:
        raise ValueError("request needs a non-empty questions object")
    questions = _to_laya_questions(questions_in)
    state_in = body.get("state", {})
    override = MODEL_OVERRIDES.get(str(body.get("model", "")).strip().lower())
    with _STATE.lock:
        if not _STATE.ready:
            raise _NotReady(_STATE.error or "checkpoints still loading")
        result = _STATE.router.predict(state_in, questions, model=override)
    return _translate(result)


class Handler(BaseHTTPRequestHandler):
    server_version = "laya-gateway/1.0"
    protocol_version = "HTTP/1.1"

    def _send(self, code, payload):
        raw = json.dumps(payload).encode("utf-8")
        self.send_response(code)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(raw)))
        self.end_headers()
        self.wfile.write(raw)

    def _authorized(self):
        token = os.environ.get("LAYA_GATEWAY_TOKEN", "")
        if not token:
            return True
        return hmac.compare_digest(self.headers.get("Authorization", ""), "Bearer " + token)

    def do_GET(self):  # noqa: N802 - http.server API
        if urlsplit(self.path).path != "/health":
            self._send(404, {"error": {"type": "not_found", "message": "use GET /health or POST /v1/evaluate"}})
            return
        with _STATE.lock:
            if _STATE.ready:
                self._send(200, {
                    "status": "ok",
                    "ready": True,
                    "models": list(_STATE.router.loaded),
                    "requests": _STATE.requests,
                })
            elif _STATE.error:
                self._send(503, {"status": "error", "detail": _STATE.error})
            else:
                self._send(503, {"status": "loading"})

    def do_POST(self):  # noqa: N802 - http.server API
        if urlsplit(self.path).path != "/v1/evaluate":
            self._send(404, {"error": {"type": "not_found", "message": "POST /v1/evaluate"}})
            return
        if not self._authorized():
            self._send(401, {"error": {"type": "unauthorized", "message": "missing or invalid bearer token"}})
            return
        length = int(self.headers.get("Content-Length") or 0)
        if length <= 0 or length > MAX_BODY:
            self._send(400, {"error": {"type": "invalid_request", "message": "bad Content-Length"}})
            return
        raw = self.rfile.read(length)
        started = time.time()
        try:
            body = json.loads(raw)
        except (ValueError, UnicodeDecodeError) as exc:
            self._send(400, {"error": {"type": "invalid_json", "message": str(exc)}})
            return
        try:
            result = evaluate(body)
        except _NotReady as exc:
            self._send(503, {"error": {"type": "not_ready", "message": str(exc)}})
            return
        except ValueError as exc:
            self._send(400, {"error": {"type": "invalid_request", "message": str(exc)}})
            return
        except Exception as exc:  # noqa: BLE001 - surfaced as a typed 500
            self._send(500, {"error": {"type": "evaluation_failed", "message": "%s: %s" % (type(exc).__name__, exc)}})
            return
        with _STATE.lock:
            _STATE.requests += 1
        route = (result.get("routing") or {}).get("model", "auto")
        print("[laya-gateway] evaluate questions=%d route=%s %.1fms" % (
            len(result["answers"]), route, (time.time() - started) * 1000.0), flush=True)
        self._send(200, result)

    def log_message(self, fmt, *args):  # keep the default access log quiet
        pass


def main():
    host = os.environ.get("LAYA_GATEWAY_HOST", "127.0.0.1")
    port = int(os.environ.get("LAYA_GATEWAY_PORT", "8128"))
    threading.Thread(target=_load, daemon=True).start()
    server = ThreadingHTTPServer((host, port), Handler)
    print("[laya-gateway] listening on http://%s:%d (checkpoints load in the background; "
          "/health flips to 200 when ready)" % (host, port), flush=True)
    server.serve_forever()


if __name__ == "__main__":
    main()
