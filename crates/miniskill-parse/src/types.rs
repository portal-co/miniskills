use serde::{Deserialize, Serialize};

// ── Miniskill file ────────────────────────────────────────────────────────────

/// A parsed `.ms.md` file.
pub struct MiniskillFile {
    pub meta: MiniskillMeta,
    /// Raw text of the `## Skill` section (everything between front-matter and `---gate---`).
    pub skill: String,
    /// Raw text of the `## Gate` section (everything after `---gate---`).
    pub gate: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MiniskillMeta {
    pub name: String,
    pub version: String,
    #[serde(rename = "topic-class")]
    pub topic_class: TopicClass,
    pub ct: u8,
    pub it: u8,
    pub enforcer: Option<String>,
    #[serde(rename = "enforcer-vk")]
    pub enforcer_vk: Option<String>,
    #[serde(rename = "zk-proof-system")]
    pub zk_proof_system: Option<ZkProofSystem>,
    pub status: Option<Status>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TopicClass {
    FrozenKnowledge,
    SparseCoverage,
    RapidlyEvolving,
    HighStakesPrecision,
    AdversarialCorpus,
    ModelPersonalitySensitive,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ZkProofSystem {
    Volar,
    None,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Status {
    Deprecated,
}

// ── AGENTS.md link ────────────────────────────────────────────────────────────

/// A parsed `[miniskill: slug](path) <!-- scope=glob,glob -->` line.
#[derive(Debug, Clone, PartialEq)]
pub struct AgentLink {
    pub slug: String,
    /// Relative path to the `.ms.md` file from the repo root.
    pub path: String,
    /// Glob patterns declared in the `scope=` attribute.
    pub scope: Vec<String>,
}

// ── Tokens ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum MiniskillToken {
    Self_(SelfToken),
    Attest(AttestToken),
    Vouch(VouchToken),
    Replay(ReplayToken),
    Challenge(ChallengeToken),
    Response(ResponseToken),
}

/// `MINISKILL-SELF: MODEL family version | CT-SELF N | topic slug [| version N] [| ai-key key]`
#[derive(Debug, Clone, PartialEq)]
pub struct SelfToken {
    pub model_family: String,
    pub model_version: String,
    pub ct_self: u8,
    pub topic: String,
    pub version: Option<String>,
    pub ai_key: Option<String>,
}

/// `MINISKILL-ATTEST: topic=slug | version=N | model=family version | ct-required=N | ct-claimed=N | result=PASS|FAIL | ...`
#[derive(Debug, Clone, PartialEq)]
pub struct AttestToken {
    pub topic: String,
    pub version: String,
    pub model: String,
    pub ct_required: u8,
    pub ct_claimed: u8,
    pub result: AttestResult,
    pub proof_system: Option<String>,
    /// Base64-encoded V̂ list (volar ZK proof).
    pub proof: Option<String>,
    /// `"sha256:hex"` hash of the `.vk` file.
    pub vk_hash: Option<String>,
    /// Base64-encoded Ed25519 signature (standard-sig enforcers only).
    pub sig: Option<String>,
}

/// `MINISKILL-VOUCH: topic=slug | version=N | vouching-for=sha256:hex | reason="..." | sig=base64`
#[derive(Debug, Clone, PartialEq)]
pub struct VouchToken {
    pub topic: String,
    pub version: String,
    pub vouching_for: String,
    pub reason: Option<String>,
    pub sig: String,
}

/// `MINISKILL-REPLAY: topic=slug | ct-required=N | ct-claimed=N | probe-score=f | threshold=f | result=PASS|FAIL | enforcer=path@ver`
#[derive(Debug, Clone, PartialEq)]
pub struct ReplayToken {
    pub topic: String,
    pub ct_required: u8,
    pub ct_claimed: u8,
    pub probe_score: Option<f64>,
    pub threshold: Option<f64>,
    pub result: AttestResult,
    pub enforcer: Option<String>,
}

/// `MINISKILL-CHALLENGE: topic=slug | challenge-id=id [| source=checker-bot] | "prompt text"`
#[derive(Debug, Clone, PartialEq)]
pub struct ChallengeToken {
    pub topic: String,
    pub challenge_id: String,
    /// `"checker-bot"` when posted by the checker bot; absent for runtime-issued challenges.
    pub source: Option<String>,
    pub prompt: String,
}

/// `MINISKILL-RESPONSE: challenge-id=id | respondent=model | "body" [| sig=base64]`
#[derive(Debug, Clone, PartialEq)]
pub struct ResponseToken {
    pub challenge_id: String,
    pub respondent: String,
    pub body: String,
    pub sig: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AttestResult {
    Pass,
    Fail,
}

impl AttestResult {
    pub fn as_str(&self) -> &'static str {
        match self {
            AttestResult::Pass => "PASS",
            AttestResult::Fail => "FAIL",
        }
    }
}
