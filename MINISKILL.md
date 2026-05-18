# MINISKILL.md — Draft Specification v0.5

> A **miniskill** is a single Markdown file, linked from `AGENTS.md`, that combines in-context topic guidance with a self-reported capability declaration and an optional WASM attestation target. The skill loads lazily when the model touches scoped files. The gate does not block upfront — it attests *after the fact*, letting checker bots, PR harnesses, and maintainers replay and verify the assessment without requiring harness changes to the agentic workflow itself.

---

## 1. Design Principles

**Optimistic loading, retroactive enforcement.** The skill text is injected freely into context — it costs nothing and harms nothing if an incapable model reads it. The gate's job is not to prevent reading; it is to produce a verifiable attestation that travels with the model's output into review. A bad attestation (or a missing one) is caught at PR time, not at task-start time, which is where quality gates on contributions belong anyway.

**No mandatory harness changes.** The miniskill works as pure context injection today. The WASM attestation layer is additive — runtimes that support it produce richer signals; runtimes that don't still surface the self-report. Checker bots can replay the gate independently from a PR comment. Nothing about the design requires the agentic workflow to change.

**Two independent tier axes.** *Capability tier* (CT) describes the minimum model class that can produce correct output for this topic. *Impact tier* (IT) describes how much damage wrong output causes, which determines what attestation strength is required before merge. These are orthogonal: a low-CT / high-IT topic (e.g. well-known API, production-critical path) needs minimal model gating but strong review. A high-CT / low-IT topic (e.g. conlangs, demoscene art) needs strong model gating but the stakes of a mistake are low.

**Attestation is a conversation, not a one-shot token.** The model, the WASM enforcer, checker bots, and maintainers each contribute independently verifiable signals. These compose — multiple tokens of different types can cover a single contribution — and accumulate across the PR review lifecycle. A maintainer vouch can substitute for a missing mechanical attestation; a checker bot replay can provide an independent third-party signal; a ZK proof can make the enforcer's evaluation independently verifiable without revealing what it evaluated.

**Single file per miniskill.** The skill and gate spec live together. The WASM enforcer is a sidecar, not a required component. Simpler to author, simpler to review, simpler to link.

---

## 2. File Layout

```
.miniskills/
  retro-computing-6502.ms.md      # the miniskill (skill + gate spec in one file)
  retro-computing-6502.gate.wasm  # optional WASM attestation enforcer
  retro-computing-6502.gate.wasm.pub  # enforcer public key — committed
  retro-computing-6502.gate.wasm.vk   # ZK verification key — committed (omit if no ZK)
  retro-computing-6502.gate.key   # decryption key for answer material — NOT committed
  vouch-keys.txt                  # maintainer signing keys authorised to vouch
  refs/
    sid-register-map.md           # pinned reference documents
```

`AGENTS.md` links miniskills with a typed introducer comment:

```markdown
## Miniskills
<!-- miniskill: .miniskills/retro-computing-6502.ms.md  scope=src/asm/**,*.s,*.asm -->
<!-- miniskill: .miniskills/conlang.ms.md               scope=src/conlang/** -->
<!-- miniskill: .miniskills/cryptography.ms.md          scope=src/crypto/**,tests/crypto/** -->
```

The `scope` attribute is on the introducer, not in the file — this keeps the miniskill file portable across projects and lets a project override scope without editing the miniskill itself.

**Lazy loading:** the runtime injects a miniskill's contents only when the current task touches at least one file matching the declared scope. An unmatched miniskill costs zero tokens.

---

## 3. Miniskill File Format (`.ms.md`)

A miniskill is a single Markdown file with two clearly delimited sections: `## Skill` and `## Gate`. The YAML front-matter belongs to the file as a whole.

```markdown
---
name: retro-computing-6502
version: 0.1.0
topic-class: frozen-knowledge
ct: 3          # Capability Tier — minimum model class; see §5
it: 2          # Impact Tier — review weight; see §6
enforcer: retro-computing-6502.gate.wasm   # optional; omit if no WASM enforcer
---

## Skill

[Everything the model reads. Topic context, gotchas, references, permissions,
 prohibitions. See §4.]

---gate---

## Gate

[Self-assessment declaration the model fills out. Probe questions with
 expected response format. Attestation instructions. See §7.]
```

The `---gate---` delimiter is the only structural requirement beyond valid YAML front-matter. Runtimes inject the `## Skill` section into model context. The `## Gate` section is injected after the skill, as the final instruction block, telling the model what it must self-report and — if an enforcer is present — how to trigger attestation.

---

## 4. The Skill Section

The skill is the in-context learning material. It is the primary value of the miniskill for everyday use — even without any gate infrastructure, the skill alone improves agentic output on difficult topics.

Write it for the model as the audience. Dense and specific beats discursive and hedged. The goal is the minimal context that prevents the specific failure modes known for this topic.

### 4.1 Required Content

**Topic** (1–3 sentences): What this topic is, why it appears in this project, why AI behaviour is abnormal here.

**Gotchas** (at least one before shipping): Specific known failure modes. See §8 for format.

**References**: Pinned documents committed to the repo. Prefer paths over URLs; URLs rot.

### 4.2 Optional Content

**Permissions** (`MAY`): Positive guidance scoped to the capability tier. Do not restate the CT value — describe the behaviours that are permitted.

**Prohibitions** (`MUST NOT`): Hard constraints. Do not restate the IT value — describe the prohibited behaviours.

**External compilers**: Named tools the model must run on its output, with the exact command. These go in the skill (not the gate) because they are part of the model's workflow, are not secret, and can be re-run by any reviewer independently.

```markdown
## Skill

### Topic

6502 assembly targeting the Commodore 64. This project's `src/asm/` tree
contains performance-critical interrupt handlers and custom character sets.
AI behaviour is abnormal here because the C64's hardware corpus is frozen at
~1994; models confabulate SID register values, memory-map addresses, and
BASIC ROM entry points with high confidence and no self-correction signal.

### Compilers

After generating any `.s` or `.asm` file, run:
  `ca65 -t c64 {file}` — treat any error as blocking.
  `ld65 -C c64.cfg {objects}` — treat any warning as a review item.
Reviewers will rerun these independently.

### Gotchas

#### GOTCHA: SID register address confusion

**Trigger**: Any code addressing SID registers.
**Wrong output**: Models frequently use $D000 (VIC-II base) or $DC00 (CIA1) instead of $D400.
**Correct behaviour**: SID base is $D400. Verify against refs/sid-register-map.md.
**Verification**: grep for $D000 and $DC00 in SID-context code; flag if present.
**Affects**: All models.

### References

- `.miniskills/refs/sid-register-map.md` — committed copy of SID register table
- `.miniskills/refs/c64-memory-map.md` — committed copy of C64 memory map
```

---

## 5. Capability Tier (CT)

CT describes the minimum model class that can produce correct, non-hallucinated output for this topic. It is a routing signal, not a quality judgment. A model below the declared CT should not touch the scoped files — not because it is a bad model, but because the domain's failure modes are structural and cannot be overcome by instruction quality alone.

| CT | Minimum model class | Typical domains |
|----|---------------------|-----------------|
| **CT1** | Any model, including small open-source | General-purpose code, documentation, well-covered APIs |
| **CT2** | Models with reliable 2024+ training coverage | Actively-maintained libraries, current cloud APIs |
| **CT3** | Sonnet 4.6+ / GPT-5 / Gemini 3 class | Compiler development, complex algorithms, sparse-coverage topics |
| **CT4** | Opus 4.6+ / GPT-5 Pro class | Cryptographic protocol implementation, formal verification, high-stakes-precision topics |
| **CT5** | Mythos / frontier research-preview class | Creative domains with no ground truth (conlangs, novel formal systems) |

**CT is declared by the miniskill author based on documented evidence** — either observed failures on lower-tier models, or reasoning from the topic class (§9). It is not a benchmark score. A `frozen-knowledge` topic that any model hallucinates on is CT3 regardless of how capable the model is in general.

**CT and the self-assessment:** At gate time, the model declares its own family and version. The WASM enforcer (if present) compares this against the CT threshold and records whether the model meets it. If no enforcer is present, the self-report stands and is reviewable by a maintainer.

---

## 6. Impact Tier (IT)

IT describes the consequence of wrong output — how much damage a hallucination or slop in this topic causes. IT determines what attestation strength is required before a contribution can be merged.

| IT | Consequence of wrong output | Required attestation |
|----|----------------------------|----------------------|
| **IT1** | Low: cosmetic, easily caught in review | Self-report sufficient |
| **IT2** | Moderate: functional bug, caught by tests | Self-report + passing external compiler/test suite |
| **IT3** | Significant: security-adjacent, subtle failure | WASM attestation or maintainer review |
| **IT4** | Severe: direct security impact, data integrity | WASM attestation **and** maintainer sign-off |
| **IT5** | Critical: safety, life, or legal consequence | Human expert review; AI contribution blocked regardless of CT |

IT is about the *output's blast radius*, not about how hard the topic is. A well-understood CT1 topic can be IT4 if it's on a critical path. A CT5 topic like conlangs is typically IT1 — a structurally broken conlang is art, not a CVE.

**IT drives PR policy, not model selection.** The CI system reads the IT from the miniskill's front-matter and enforces the required attestation level before merge. This is where "no mandatory harness changes" is slightly nuanced: the *agentic workflow* needs no changes, but a project opting into WASM attestation does need a CI step that reads miniskill IT and validates attestations. That CI step is optional and additive.

---

## 7. The Gate Section

The gate is the final block of the miniskill, injected after the skill. It tells the model what to self-report and what attestation to produce. It is always shown — unlike v0.3, the gate is not withheld until after a probe run. The model reads it, does the work, then fills it out.

### 7.1 Structure

```markdown
---gate---

## Gate

### Self-Assessment

Declare your model family and version:
  `MODEL: [family] [version]`  e.g. `MODEL: claude opus-4-7`

Declare your capability tier for this topic:
  `CT-SELF: [1–5]`

If CT-SELF is below the declared threshold (ct: 3), state this explicitly and
restrict your contribution to scaffolding and comments only (no domain logic).
Do not attempt to self-promote above CT3 without documented evidence in this
session that your output passed external compiler checks.

### Attestation

If a WASM enforcer is present at `.miniskills/retro-computing-6502.gate.wasm`,
the runtime will invoke it on your self-assessment and produce a signed
attestation token. Include the token in your commit message or PR description:

  `MINISKILL-ATTEST: [token]`

If no enforcer is present, include your self-assessment verbatim:
  `MINISKILL-SELF: MODEL claude opus-4-7 | CT-SELF 3 | topic retro-computing-6502`

Both forms are machine-readable by checker bots. The self-report form is
reviewable by a maintainer but carries no cryptographic guarantee.
```

### 7.2 Self-Assessment vs. Enforcer Attestation

These are two points on a continuum of trust, not two different mechanisms:

**Self-report** (`MINISKILL-SELF`): The model declares its own family, version, and CT. Costs zero infrastructure. A maintainer or checker bot can read it and flag contributions from models below the CT threshold. Gameable by a motivated bad actor, but: (a) most AI-assisted contributions are not adversarial, and (b) the self-report still creates a traceable audit trail and surfaces the issue in PR review.

**WASM attestation** (`MINISKILL-ATTEST`): The runtime runs the WASM enforcer after the model's self-assessment, verifies the claimed model identity against the CT threshold (and optionally runs probe questions against encrypted answer keys), and produces a signed token. The token is verified by CI without re-running the model. The model cannot forge it; it can only trigger the run and include the result.

The WASM enforcer for attestation purposes is simpler than the v0.3 gate design: it does not need to block context injection, it does not need to withhold answers, and it does not need to run before the skill is read. Its job is to sign a statement of the form: "model X claimed CT Y for topic Z; the declared CT threshold is W; X ≥ W: [true/false]; probe score: [optional]." The signature is what matters for CI verification.

### 7.3 Probe Questions (Optional)

For topics where CT self-report is insufficient (IT3+), the gate section may include probe questions. The model answers them as part of its gate output; the WASM enforcer evaluates the answers and folds the result into the attestation token.

Unlike v0.3, the answers are not withheld from the model — the goal is not to trick the model, it is to produce a verifiable record of what the model actually knows. A model that answers probe questions correctly and still produces wrong output has a gotcha problem (the skill section needs updating), not a gate problem. A model that answers probe questions incorrectly has declared its own CT limitation.

```markdown
### Probe (for IT3+ contributions)

Answer the following before submitting. Your answers will be evaluated by the
enforcer and included in the attestation token.

P01 [corpus-density]: What is the base address of the SID chip register file
on a stock C64?

P02 [calibration]: Describe the difference in filter behaviour between the
SID 6581 and SID 8580.

P03 [calibration]: A user asks for the SID firmware changelog from 2003.
What is the correct response?
```

---

## 8. Gotcha Format

A gotcha is a specific, falsifiable failure mode. It names the exact wrong output the model is likely to produce and the exact correct behaviour. It is not style guidance.

```markdown
#### GOTCHA: [short title]

**Trigger**: When the model generates [specific thing].
**Wrong output**: [Specific enough to be falsifiable — name the wrong value, pattern, or claim.]
**Correct behaviour**: [What must happen instead.]
**Verification**: [A command if possible. "Manual review" if not — say so explicitly.]
**Affects**: [Which model families exhibit this. "All models" is valid.]
```

---

## 9. Topic Classes

Declared in front-matter as `topic-class`. Informs the default CT baseline and the kinds of gotchas that belong in the skill.

**`frozen-knowledge`**: Corpus closed. Models confabulate with no self-correction signal because there is no signal in training data that the domain stopped evolving. Default CT baseline: 3. Retro computing, legacy industrial protocols, early web standards, obsolete crypto implementations.

**`sparse-coverage`**: Domain exists but is underrepresented. Models interpolate from adjacent denser domains. Default CT baseline: 2–3. Low-resource languages, hyperlocal law, niche academic subfields, post-cutoff libraries, proprietary SDKs.

**`rapidly-evolving`**: Changes faster than retraining cadence. Even recent-cutoff models are structurally behind. Default CT baseline: 2 (with web access) or 3 (without). AI/ML tooling, cloud APIs, regulatory guidance, CVEs.

**`high-stakes-precision`**: Tolerance for plausible-but-wrong output near zero. Default IT: 4–5 regardless of CT. Medical dosing, legal citation, aviation specs, financial instrument definitions.

**`adversarial-corpus`**: Domain flooded with low-quality or SEO-poisoned content. Models reflect corpus noise. Default CT baseline: 3. Supplement claims, contested historical events, crypto hype, thin-content how-to.

**`model-personality-sensitive`**: Output quality and character varies structurally across model families — not cutoff, not capability in general, but alignment and emergent personality. Default CT baseline: determined case-by-case. Conlangs (CT5), long-form creative fiction with sustained stylistic requirements, open-ended philosophical reasoning.

---

## 10. Cross-Model Guidance

Kept brief — the CT system handles routing. This section is a reference for miniskill authors setting CT values.

**CT1 floor** (any model): GPT-4o (Oct 2023 cutoff) is the weakest widely-deployed model. If a topic requires knowledge that post-dates October 2023, GPT-4o cannot be CT1. Treat it as frozen-knowledge for any `rapidly-evolving` dependency newer than its cutoff.

**CT2 floor** (2024+ coverage): Llama 4 (Aug 2024 cutoff, no web access) is the practical CT2 floor for offline deployments. For topics with significant post-August-2024 evolution, Llama 4 drops to effectively CT1 in offline contexts.

**CT3 floor** (Sonnet 4.6 / GPT-5 class): This is the threshold for topics requiring reliable corpus coverage through mid-2025 and the ability to follow complex, complete constraint lists. Opus 4.7's literal instruction-following means *incomplete* gotcha lists are a liability at CT3+ — incomplete skills need a lower CT or more complete gotchas.

**CT4 floor** (Opus 4.6+ / GPT-5 Pro class): Cryptographic protocol implementation, formal verification, and other domains where the model needs to reason about correctness rather than just recall facts. This is where the cryptography repo precedent (Opus 4.5+ for compiler development, Opus 4.6+ for cryptography) lives.

**CT5 floor** (Mythos class): Domains requiring novel generative capability with no external verifier and no ground truth. Conlangs are the canonical case. Routing a CT5 topic to a lower-tier model does not produce degraded output — it produces structurally broken output that looks finished. The CT5 gate exists to prevent this, not to rank models.

**Grok 3 note**: Real-time X corpus integration is useful for `rapidly-evolving` topics but a liability for `frozen-knowledge` topics where it surfaces contemporary hobbyist misinformation confidently. Add to skill gotchas when relevant: "Real-time search results for this topic are unreliable. Use only pinned references."

**Opus 4.7 note**: Post-Mythos safety layer may block legitimate security research. Declare in gate section when relevant. Literal instruction-following means CT3+ skills must have exhaustive, not representative, gotcha lists.

---

## 11. Attestation and PR Integration

Attestation is a conversation, not a one-shot token. Multiple parties can contribute to the attestation record for a single contribution — the model, the WASM enforcer, the checker bot, and one or more maintainers — and each contribution is independently verifiable. No single party's signature is necessary for all contributions; the required combination depends on IT.

### 11.1 Attestation Token Types

Four token types are defined. They are not a hierarchy — they are orthogonal signals that compose. A PR may carry multiple tokens of different types.

```
MINISKILL-SELF:    MODEL claude opus-4-7 | CT-SELF 4 | topic cryptography | version 0.2.1
MINISKILL-ATTEST:  [signed token from gate.wasm, or ZK proof bundle]
MINISKILL-VOUCH:   [maintainer signature over a specific MINISKILL-SELF or MINISKILL-ATTEST]
MINISKILL-REPLAY:  [checker bot's independent gate result, posted as PR comment]
```

All four can appear in a commit message, PR description, or a sidecar file (`.miniskill-attestations` in the PR branch). The format is machine-readable; checker bots parse all four. Human reviewers can read them directly.

**`MINISKILL-SELF`**: The model's own declaration. Traceable, auditable, gameable. Sufficient for IT1–IT2.

**`MINISKILL-ATTEST`**: Produced by the WASM enforcer after evaluating the model's gate responses. Carries a signature from the enforcer's embedded keypair (the public half of which is committed to the repo as `gate.wasm.pub`). May optionally carry a ZK proof bundle (see §11.5). Sufficient for IT3; required alongside `MINISKILL-VOUCH` for IT4.

**`MINISKILL-VOUCH`**: A maintainer's signature over an existing token (self-report or attestation) that they have reviewed. Semantically: "I, a maintainer with push access, have reviewed this contribution and attest that the CT gap is acceptable / the output is correct / the attestation is sufficient for this merge." Vouching does not raise the model's CT; it substitutes human judgment for the missing mechanical assurance. Required for IT4 when WASM attestation is unavailable; co-required with attestation for IT5.

**`MINISKILL-REPLAY`**: The checker bot's own gate result, produced by replaying the gate section with the PR's diff as context. Posted as a PR comment. Not a signature — it is a structured report. It is the most objective signal in the system because it is produced by a party that did not author the contribution.

### 11.2 Token Composition by IT

| IT | Minimum required attestation |
|----|------------------------------|
| IT1 | `MINISKILL-SELF` (or omitted; checker bot posts a notice, not a block) |
| IT2 | `MINISKILL-SELF` + passing external compilers |
| IT3 | `MINISKILL-ATTEST`, or `MINISKILL-SELF` + `MINISKILL-VOUCH` |
| IT4 | `MINISKILL-ATTEST` + `MINISKILL-VOUCH` |
| IT5 | Human expert review; `MINISKILL-ATTEST` and `MINISKILL-VOUCH` both required, plus explicit IT5 sign-off in PR description |

The `MINISKILL-VOUCH` substitution path at IT3 exists specifically for the case where the WASM enforcer is not yet built. It is not a permanent alternative — if a project consistently uses the vouch path for IT3 topics, the miniskill needs an enforcer.

### 11.3 Interactive Attestation Flow

Attestation is designed to be conversational. The model, checker bot, and maintainers interact through PR comments and commit additions; the attestation record accumulates across those interactions.

**Typical flow for an IT3 contribution:**

```
1. Model submits PR with MINISKILL-SELF token.
2. Checker bot posts MINISKILL-REPLAY to PR comments:
     MINISKILL-REPLAY: topic=cryptography | ct-required=4 | ct-claimed=4 |
       probe-score=0.82 | threshold=0.75 | result=PASS | enforcer=gate.wasm@v0.2.1
3. Runtime or contributor triggers WASM enforcer; MINISKILL-ATTEST is added
   to commit or PR description.
4. Checker bot re-validates; marks PR check green.
```

**IT4 flow with maintainer vouch:**

```
1. Model submits PR with MINISKILL-SELF token (CT-SELF 3, ct-required 4).
2. Checker bot posts: "CT-SELF below threshold. MINISKILL-ATTEST required,
   or MINISKILL-VOUCH from a maintainer."
3. Maintainer reviews diff, is satisfied with output quality.
4. Maintainer posts signed vouch:
     MINISKILL-VOUCH: topic=cryptography | vouching-for=<hash of MINISKILL-SELF> |
       reason="reviewed output; CT gap acceptable for this narrowly-scoped change" |
       sig=<maintainer GPG/SSH sig over this token>
5. Model or runner adds MINISKILL-ATTEST (probe score 0.71, just below threshold).
6. Checker bot: ATTEST present (below threshold) + VOUCH present → IT4 satisfied.
   Posts summary; PR check green.
```

**Challenge-response (optional, for IT4+ or contested contributions):**

The checker bot or a maintainer can post a challenge to the model directly in a PR comment:

```
MINISKILL-CHALLENGE: topic=cryptography | challenge-id=c7f3a |
  "Explain why the HMAC key derivation in src/crypto/kdf.rs:42 uses
   HKDF-SHA256 rather than PBKDF2 for this use case."
```

The model (or the contributor, if human) responds in the same thread:

```
MINISKILL-RESPONSE: challenge-id=c7f3a | respondent=claude opus-4-7 |
  "HKDF is appropriate here because the input keying material is already
   a high-entropy secret (a 32-byte session key), not a low-entropy
   password. PBKDF2's iteration cost is designed to slow down brute-force
   on low-entropy inputs; it adds unnecessary overhead without security
   benefit when the IKM is already uniformly random..."
  sig=<optional, if enforcer can sign model responses>
```

The checker bot records the exchange. A maintainer reviews it. Challenge-response is not automated scoring — it produces human-reviewable evidence, not a pass/fail signal. It is particularly useful for IT4+ contributions where the right answer requires contextual reasoning that probe questions can't capture.

### 11.4 Maintainer Vouching in Detail

A vouch is a GPG or SSH signature (using the maintainer's existing commit-signing key — no new key infrastructure required) over a structured token. The token includes:

- The hash of the thing being vouched for (a `MINISKILL-SELF` token, a `MINISKILL-ATTEST` token, or a specific commit hash)
- The topic slug and miniskill version
- An optional human-readable reason
- The maintainer's identity (derived from the signing key)

```
MINISKILL-VOUCH: topic=cryptography | version=0.2.1 |
  vouching-for=sha256:<hex> |
  reason="<optional free text>" |
  sig=<base64-encoded GPG/SSH sig over the above fields>
```

**What a vouch means:** "I have reviewed this contribution. I am satisfied that the quality and correctness are acceptable for merge despite the attestation being below the mechanical threshold." It is *not* a claim that the model is CT4 — it is a claim that this specific contribution is acceptable. The model's declared CT remains in the audit log.

**What a vouch does not mean:** It does not change the miniskill's CT or IT. It does not retroactively certify the model. Multiple maintainer vouches on the same low-CT contribution do not create a precedent for future low-CT contributions — each is evaluated independently.

**Vouch scope:** A maintainer can vouch for a specific commit, a specific file, or a whole PR. Narrower vouches are preferred — "I vouch for the changes to `kdf.rs`" is more meaningful than "I vouch for the whole PR" when the PR touches multiple miniskill scopes.

**Vouch keys:** Checker bots validate vouches against the repo's `CODEOWNERS`-equivalent key list (committed as `.miniskills/vouch-keys.txt` or similar). A vouch from a non-maintainer key is surfaced as a comment but does not satisfy attestation requirements.

### 11.5 ZK Proof Attestations

The WASM enforcer can optionally produce a ZK proof alongside or instead of its standard signed token. The ZK proof attests: "the scoring function F was applied to probe responses R, and the result exceeded threshold T" — without revealing R, F, or T to the verifier.

**What ZK adds:** The verifier (checker bot, maintainer, or any third party) can confirm the attestation was honestly computed — that the enforcer did not simply emit a passing token without actually evaluating the responses. This is stronger than a standard signed token, which requires trusting the enforcer to run correctly.

**What ZK does not add — and why WASM protection is still required:**

ZK proves the computation was honest. It does not prove:
- **Model identity**: nothing in the ZK proof prevents a higher-tier model from producing probe responses on behalf of a lower-tier model's contribution. The model that authored the code and the model that answered the probe questions could be different invocations.
- **Causality**: the ZK proof attests that *some* probe responses passed the gate, not that the model that produced those responses also produced the contribution being attested.
- **Answer confidentiality**: ZK can prove the threshold was met without revealing the score, but it cannot prevent the model from *remembering* probe questions it has answered and using that information in future sessions. The answer key material in the WASM enforcer remains the primary protection against key leakage; ZK is not a substitute for it.

The relationship between ZK and WASM protection is therefore: WASM protects the secret material (answer keys, scoring logic) from being read. ZK proves the secret material was applied honestly to produce the attestation. They address different threat models and are complementary, not redundant.

**Token format with ZK proof:**

```
MINISKILL-ATTEST: topic=cryptography | version=0.2.1 |
  model=claude opus-4-7 | ct-required=4 | ct-claimed=4 |
  result=PASS |
  proof-system=groth16 |
  proof=<base64-encoded ZK proof> |
  vk-hash=sha256:<hash of verification key committed at gate.wasm.vk>
```

The verification key (`gate.wasm.vk`) is committed to the repository and used by the checker bot to verify the proof without running the enforcer. This makes ZK-attested contributions verifiable entirely from static repository artefacts — no WASM runtime required for verification, only for generation.

**ZK proof system:** The choice of proof system (Groth16, PLONK, STARKs) is left to the enforcer author. Groth16 produces the smallest proofs and is well-supported in WASM targets; STARKs require no trusted setup and are appropriate for projects that cannot distribute a trusted setup artifact. The miniskill front-matter should declare the proof system in use:

```yaml
enforcer: cryptography.gate.wasm
zk-proof-system: groth16        # groth16 | plonk | stark | none
zk-vk: cryptography.gate.wasm.vk  # committed verification key
```

**Interplay with the author's ZK project:** The enforcer WASM is the natural integration point. The enforcer generates the proof internally; the proof travels with the attestation token; verification uses only the committed `gate.wasm.vk`. The WASM module's internal structure (which circuit, which prover library) is an implementation detail — the token format and verification key path are the miniskill spec's concern.

### 11.6 Checker Bot Behaviour

The checker bot reads miniskill front-matter and validates the attestation record for a PR. It requires no access to the model, no API calls, and no enforcer execution for standard signed tokens. For ZK-attested contributions, it runs the ZK verifier against the committed verification key.

**Algorithm:**

1. Find all files changed in the PR.
2. For each changed file, find all miniskills whose scope matches.
3. For each matching miniskill, read `ct`, `it`, `enforcer`, `zk-proof-system` from front-matter.
4. Collect all `MINISKILL-*` tokens from the PR description, commit messages, and PR comments.
5. For each miniskill in scope:
   - Find the best available attestation (prefer `MINISKILL-ATTEST` > `MINISKILL-REPLAY` > `MINISKILL-VOUCH` > `MINISKILL-SELF`).
   - Check whether the combination satisfies the IT requirement (§11.2).
   - Validate signatures: `MINISKILL-ATTEST` signature against `gate.wasm.pub`; ZK proof against `gate.wasm.vk`; `MINISKILL-VOUCH` signature against `vouch-keys.txt`.
   - If ZK proof present: run verifier. If standard sig: verify sig. If self-report only: compare CT-SELF against `ct`.
6. Post a structured summary as a PR comment. Mark the PR check as:
   - **Green**: all in-scope miniskills have sufficient attestation.
   - **Yellow**: some attestations are self-report-only where ATTEST is preferred (IT2); maintainer notice posted.
   - **Red**: missing or invalid attestation for IT3+; PR check fails.
7. For IT4+: always flag for maintainer review in the PR comment, even if attestation is present and valid.

### 11.7 Harness Fork (Optional)

For projects with richer CI infrastructure, the agentic harness can fork the conversation at task-completion time: the main thread continues; a parallel thread replays the gate section with the model's actual output as context and requests a fresh self-assessment and probe responses. The enforcer runs on the parallel thread's output. This produces a probe-informed attestation without blocking the agentic workflow.

This is optional and additive. The self-report path remains valid.

### 11.8 Interaction with AI Key Systems

AI key / AI watermarking systems identify *that* a contribution is AI-assisted. Miniskill attestations identify *at what capability tier* and *whether that tier was sufficient*. Together they give maintainers three signals: provenance (was this AI-assisted), routing (was the right model used), and coverage (did the skill guidance apply). The AI key is the outer envelope; the miniskill attestation is the inner capability record.

For projects using both systems, the `MINISKILL-SELF` token should include the AI key identifier as a cross-reference:

```
MINISKILL-SELF: MODEL claude opus-4-7 | CT-SELF 4 | topic cryptography |
  version 0.2.1 | ai-key=<key from AI key system>
```

---

## 12. Miniskill Lifecycle

**Creation**: One documented failure is enough to justify a miniskill. Start with just a skill section and a CT1/IT1 declaration; raise CT and IT as evidence accumulates.

**Iteration**: Each new failure that passed the existing gate → add a gotcha. Each failure that passed the gate *and* was CT-appropriate → the gate has a gap; consider new probe items or a CT raise.

**Raising IT**: When a failure causes production impact (not just review failure), raise IT. Raising IT triggers a requirement for stronger attestation, which is low-friction for maintainers and high-friction for contributors producing low-quality AI output — exactly the right direction.

**Deprecation**: `status: deprecated` in front-matter. Never delete. A miniskill is a candidate for deprecation when external compiler enforcement fully subsumes its gotcha list, or the project no longer touches the scope.

---

## 13. Anti-Patterns

- **Do not front-load mandatory gates.** The optimistic-load model exists for a reason: blocking capable models from starting is worse than catching bad output in review.
- **Do not conflate CT and IT.** High CT (hard topic) does not mean high IT (dangerous output). Conlangs are CT5/IT1. A trivial CRUD endpoint on a payment processor is CT1/IT4.
- **Do not use self-report as the only signal for IT3+ topics.** Self-reports are gameable and unverifiable. WASM attestation or maintainer review is required at IT3+.
- **Do not write gotchas as style guidance.** "Be careful with registers" is not a gotcha. "Do not use $D000 as the SID base address; it is the VIC-II base" is a gotcha.
- **Do not set CT5 casually.** CT5 means only Mythos-class models can contribute. Use it only for topics where lower-tier output is structurally broken, not just imperfect.
- **Do not commit `gate.key`.** The key is the only secret in this system. Committing it reduces WASM attestation to a tamper-evident seal rather than a tamper-proof one.
- **Do not copy gotchas across miniskills without re-testing.** Failure modes are domain-specific, not topic-class-specific.
- **Do not mark a model below a CT threshold without a documented failure instance.** CT declarations are evidence-based.

---

## 14. Appendix: Miniskill Template

```markdown
---
name: [slug]
version: 0.1.0
topic-class: [frozen-knowledge | sparse-coverage | rapidly-evolving | high-stakes-precision | adversarial-corpus | model-personality-sensitive]
ct: [1–5]       # minimum model capability tier
it: [1–5]       # output impact tier; determines required attestation strength
enforcer: [slug].gate.wasm          # omit if no WASM enforcer
zk-proof-system: [groth16 | plonk | stark | none]   # omit if no ZK
zk-vk: [slug].gate.wasm.vk          # committed verification key; omit if no ZK
---

## Skill

### Topic

[What this topic is, why it is in this project, why AI behaviour is abnormal here.]

### Compilers

[Optional. Tools the model must run. Commands the model and reviewers can both invoke.]

### Gotchas

#### GOTCHA: [title]

**Trigger**: 
**Wrong output**: 
**Correct behaviour**: 
**Verification**: 
**Affects**: 

### References

- [.miniskills/refs/filename.md — pinned local copy]
- [External source, access date]

---gate---

## Gate

### Self-Assessment

Declare your model and capability tier for this topic:
  `MODEL: [family] [version]`
  `CT-SELF: [1–5]`

If CT-SELF is below ct: [N] declared for this miniskill, restrict your
contribution to comments and scaffolding. State this restriction explicitly.

### Attestation

Include one of the following in your commit message or PR description.

If WASM enforcer is available (produces a cryptographic attestation, optionally with ZK proof):
  `MINISKILL-ATTEST: topic=[slug] | version=[N] | model=[family] [version] | ct-required=[N] | ct-claimed=[N] | result=[PASS|FAIL] | [proof fields if ZK] | sig=[enforcer sig]`

If no enforcer (self-report only):
  `MINISKILL-SELF: MODEL [family] [version] | CT-SELF [N] | topic [slug] | version [miniskill-version]`

For IT4+ contributions, also request a maintainer vouch or expect a checker bot challenge.

### Probe (omit for IT1–IT2; include for IT3+)

[Probe questions here. The model answers them; answers are evaluated by the enforcer
 or reviewed by a maintainer. Do not include answers here — answers belong in the
 enforcer's encrypted answer key, not in the gate text.]
```

### `AGENTS.md` Introducer Syntax

```markdown
## Miniskills

<!-- miniskill: .miniskills/retro-computing-6502.ms.md  scope=src/asm/**,*.s,*.asm -->
<!-- miniskill: .miniskills/cryptography.ms.md          scope=src/crypto/** -->
<!-- miniskill: .miniskills/conlang.ms.md               scope=src/conlang/** -->
```

Scope is on the introducer. The miniskill file itself is scope-agnostic and portable across projects.
```
