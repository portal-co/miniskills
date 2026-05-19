use crate::types::*;

// ── Top-level dispatcher ──────────────────────────────────────────────────────

pub fn emit_token(token: &MiniskillToken) -> String {
    match token {
        MiniskillToken::Self_(t) => emit_self(t),
        MiniskillToken::Attest(t) => emit_attest(t),
        MiniskillToken::Vouch(t) => emit_vouch(t),
        MiniskillToken::Replay(t) => emit_replay(t),
        MiniskillToken::Challenge(t) => emit_challenge(t),
        MiniskillToken::Response(t) => emit_response(t),
    }
}

// ── Per-type emitters ─────────────────────────────────────────────────────────

/// `MINISKILL-SELF: MODEL family version | CT-SELF N | topic slug [| version N] [| ai-key key]`
pub fn emit_self(t: &SelfToken) -> String {
    let mut s = format!(
        "MINISKILL-SELF: MODEL {} {} | CT-SELF {} | topic {}",
        t.model_family, t.model_version, t.ct_self, t.topic,
    );
    if let Some(v) = &t.version {
        s.push_str(&format!(" | version {v}"));
    }
    if let Some(k) = &t.ai_key {
        s.push_str(&format!(" | ai-key {k}"));
    }
    s
}

/// `MINISKILL-ATTEST: topic=slug | version=N | model=family version | ct-required=N | ct-claimed=N | result=PASS|FAIL [| ...]`
pub fn emit_attest(t: &AttestToken) -> String {
    let mut parts = vec![
        format!("topic={}", t.topic),
        format!("version={}", t.version),
        format!("model={}", t.model),
        format!("ct-required={}", t.ct_required),
        format!("ct-claimed={}", t.ct_claimed),
        format!("result={}", t.result.as_str()),
    ];
    if let Some(ps) = &t.proof_system {
        parts.push(format!("proof-system={ps}"));
    }
    if let Some(p) = &t.proof {
        parts.push(format!("proof={p}"));
    }
    if let Some(h) = &t.vk_hash {
        parts.push(format!("vk-hash={h}"));
    }
    if let Some(s) = &t.sig {
        parts.push(format!("sig={s}"));
    }
    format!("MINISKILL-ATTEST: {}", parts.join(" | "))
}

/// `MINISKILL-VOUCH: topic=slug | version=N | vouching-for=sha256:hex [| reason="..."] | sig=base64`
pub fn emit_vouch(t: &VouchToken) -> String {
    let mut parts = vec![
        format!("topic={}", t.topic),
        format!("version={}", t.version),
        format!("vouching-for={}", t.vouching_for),
    ];
    if let Some(r) = &t.reason {
        parts.push(format!("reason=\"{r}\""));
    }
    parts.push(format!("sig={}", t.sig));
    format!("MINISKILL-VOUCH: {}", parts.join(" | "))
}

/// `MINISKILL-REPLAY: topic=slug | ct-required=N | ct-claimed=N [| probe-score=f] [| threshold=f] | result=PASS|FAIL [| enforcer=...]`
pub fn emit_replay(t: &ReplayToken) -> String {
    let mut parts = vec![
        format!("topic={}", t.topic),
        format!("ct-required={}", t.ct_required),
        format!("ct-claimed={}", t.ct_claimed),
    ];
    if let Some(s) = t.probe_score {
        parts.push(format!("probe-score={s}"));
    }
    if let Some(th) = t.threshold {
        parts.push(format!("threshold={th}"));
    }
    parts.push(format!("result={}", t.result.as_str()));
    if let Some(e) = &t.enforcer {
        parts.push(format!("enforcer={e}"));
    }
    format!("MINISKILL-REPLAY: {}", parts.join(" | "))
}

/// `MINISKILL-CHALLENGE: topic=slug | challenge-id=id [| source=...] | prompt="..."`
pub fn emit_challenge(t: &ChallengeToken) -> String {
    let mut parts = vec![
        format!("topic={}", t.topic),
        format!("challenge-id={}", t.challenge_id),
    ];
    if let Some(src) = &t.source {
        parts.push(format!("source={src}"));
    }
    parts.push(format!("prompt=\"{}\"", t.prompt));
    format!("MINISKILL-CHALLENGE: {}", parts.join(" | "))
}

/// `MINISKILL-RESPONSE: challenge-id=id | respondent=model | body="..." [| sig=base64]`
pub fn emit_response(t: &ResponseToken) -> String {
    let mut parts = vec![
        format!("challenge-id={}", t.challenge_id),
        format!("respondent={}", t.respondent),
        format!("body=\"{}\"", t.body),
    ];
    if let Some(s) = &t.sig {
        parts.push(format!("sig={s}"));
    }
    format!("MINISKILL-RESPONSE: {}", parts.join(" | "))
}
