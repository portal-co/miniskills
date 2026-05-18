# ATTESTATION.md — Attestation Protocol v0.0.6

> This document specifies the attestation protocol for miniskills: the token system, enforcer ABI, checker bot algorithm, and GitHub Actions integration. For the miniskill format spec (skill sections, CT/IT declarations, gate format, gotchas, topic classes), see [`MINISKILL.md`](MINISKILL.md).

Attestation is a conversation, not a one-shot token. Multiple parties contribute independently verifiable signals for a single contribution — the model, the WASM enforcer, checker bots, and maintainers. These signals compose, accumulate across the PR lifecycle, and are checked against the Impact Tier declared in the miniskill's front-matter.

---

## 1. Token Types

Four token types are defined. They are not a hierarchy — they are orthogonal signals that compose. A PR may carry multiple tokens of different types.

```
MINISKILL-SELF:    MODEL claude opus-4-7 | CT-SELF 4 | topic cryptography | version 0.0.1
MINISKILL-ATTEST:  [signed token from gate.wasm, or ZK proof bundle]
MINISKILL-VOUCH:   [maintainer signature over a specific MINISKILL-SELF or MINISKILL-ATTEST]
MINISKILL-REPLAY:  [checker bot's independent gate result, posted as PR comment]
```

All four can appear in a commit message, PR description, or a sidecar file (`.miniskill-attestations` in the PR branch). The format is machine-readable; checker bots parse all four. Human reviewers can read them directly.

**`MINISKILL-SELF`**: The contributor's own declaration. Traceable, auditable, gameable. Sufficient for IT1–IT2.

**`MINISKILL-ATTEST`**: Produced by the WASM enforcer after evaluating the contributor's gate responses. Carries an Ed25519 signature from the enforcer's embedded keypair (the public half of which is committed to the repo as `[slug].gate.wasm.vk`). May optionally carry a ZK proof bundle (see §5). Sufficient for IT3; required alongside `MINISKILL-VOUCH` for IT4.

**`MINISKILL-VOUCH`**: A maintainer's signature over an existing token (self-report or attestation) that they have reviewed. Semantically: "I, a maintainer with push access, have reviewed this contribution and attest that the CT gap is acceptable / the output is correct / the attestation is sufficient for this merge." Vouching does not raise the contributor's CT; it substitutes human judgment for the missing mechanical assurance. Required for IT4 when WASM attestation is unavailable; co-required with attestation for IT5.

**`MINISKILL-REPLAY`**: The checker bot's own gate result, produced by replaying the gate section with the PR's diff as context. Posted as a PR comment. Not a signature — it is a structured report. It is the most objective signal in the system because it is produced by a party that did not author the contribution.

---

## 2. Token Composition by IT

| IT | Minimum required attestation |
|----|------------------------------|
| IT1 | `MINISKILL-SELF` (or omitted; checker bot posts a notice, not a block) |
| IT2 | `MINISKILL-SELF` + passing external compilers |
| IT3 | `MINISKILL-ATTEST`, or `MINISKILL-SELF` + `MINISKILL-VOUCH` |
| IT4 | `MINISKILL-ATTEST` + `MINISKILL-VOUCH` |
| IT5 | Human expert review; `MINISKILL-ATTEST` and `MINISKILL-VOUCH` both required, plus explicit IT5 sign-off in PR description |

The `MINISKILL-VOUCH` substitution path at IT3 exists specifically for the case where the WASM enforcer is not yet built. It is not a permanent alternative — if a project consistently uses the vouch path for IT3 topics, the miniskill needs an enforcer.

---

## 3. Interactive Attestation Flow

Attestation is designed to be conversational. The model, checker bot, and maintainers interact through PR comments and commit additions; the attestation record accumulates across those interactions.

**Typical flow for an IT3 contribution:**

```
1. Model submits PR with MINISKILL-SELF token.
2. Checker bot posts MINISKILL-REPLAY to PR comments:
     MINISKILL-REPLAY: topic=cryptography | ct-required=4 | ct-claimed=4 |
       probe-score=0.82 | threshold=0.75 | result=PASS | enforcer=gate.wasm@0.0.1
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

**Enforcer-initiated challenge flow ("add if absent"):**

When the runtime detects that the model's output does not contain a `MINISKILL-SELF` or `MINISKILL-ATTEST` token after task completion, it may request gate output via the enforcer's challenge mechanism:

```
1. Runtime calls enforcer.assess("") — empty gate output.
2. Enforcer returns AttestChallenge with a scoped, self-explaining message.
3. Runtime injects the challenge message into model context as a new conversation turn.
4. Model produces gate output.
5. Runtime calls enforcer.assess(gate_output) → AttestResult.
6. Runtime includes the token in the PR/commit.
```

The challenge message is plain text in the conversation — not a tool call, not a system prompt injection. It must explain its own existence so that any agent encountering it can verify legitimacy independently. See §8.3 for challenge message format and scoping requirements.

**Human/checker-initiated challenge (optional, for IT4+ or contested contributions):**

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

---

## 4. Maintainer Vouching in Detail

A vouch is a GPG or SSH signature (using the maintainer's existing commit-signing key — no new key infrastructure required) over a structured token. The token includes:

- The hash of the thing being vouched for (a `MINISKILL-SELF` token, a `MINISKILL-ATTEST` token, or a specific commit hash)
- The topic slug and miniskill version
- An optional human-readable reason
- The maintainer's identity (derived from the signing key)

```
MINISKILL-VOUCH: topic=cryptography | version=0.0.1 |
  vouching-for=sha256:<hex> |
  reason="<optional free text>" |
  sig=<base64-encoded GPG/SSH sig over the above fields>
```

**What a vouch means:** "I have reviewed this contribution. I am satisfied that the quality and correctness are acceptable for merge despite the attestation being below the mechanical threshold." It is *not* a claim that the contributor is CT4 — it is a claim that this specific contribution is acceptable. The contributor's declared CT remains in the audit log.

**What a vouch does not mean:** It does not change the miniskill's CT or IT. It does not retroactively certify the model. Multiple maintainer vouches on the same low-CT contribution do not create a precedent for future low-CT contributions — each is evaluated independently.

**Vouch scope:** A maintainer can vouch for a specific commit, a specific file, or a whole PR. Narrower vouches are preferred — "I vouch for the changes to `kdf.rs`" is more meaningful than "I vouch for the whole PR" when the PR touches multiple miniskill scopes.

**Vouch keys:** Checker bots validate vouches against the repo's committed key list (`.miniskills/vouch-keys.txt`). A vouch from a non-maintainer key is surfaced as a comment but does not satisfy attestation requirements.

---

## 5. ZK Proof Attestations (volar)

The WASM enforcer can produce a ZK proof alongside its standard Ed25519 signature. The ZK proof attests: "the scoring function F was applied to probe responses R, and the result exceeded threshold T" — without revealing R, F, or T to the verifier.

**What ZK adds:** The verifier (checker bot, maintainer, or any third party) can confirm the attestation was honestly computed — that the enforcer did not simply emit a passing token without actually evaluating the responses. This is stronger than a standard signed token, which requires trusting the enforcer to run correctly.

**What ZK does not add — and why WASM protection is still required:**

ZK proves the computation was honest. It does not prove:
- **Model identity**: nothing in the ZK proof prevents a higher-tier model from producing probe responses on behalf of a lower-tier model's contribution. The model that authored the code and the model that answered the probe questions could be different invocations.
- **Causality**: the ZK proof attests that *some* probe responses passed the gate, not that the model that produced those responses also produced the contribution being attested.
- **Answer confidentiality**: ZK can prove the threshold was met without revealing the score, but it cannot prevent the model from *remembering* probe questions it has answered and using that information in future sessions. The answer key material in the WASM enforcer remains the primary protection against key leakage; ZK is not a substitute for it.

The relationship between ZK and WASM protection is therefore: WASM protects the secret material (answer keys, scoring logic) from being read. ZK proves the secret material was applied honestly to produce the attestation. They address different threat models and are complementary, not redundant.

### 5.1 volar

The supported ZK proof system is **volar** — a compiler implementing VOLE-in-the-head ZK (Quicksilver construction) over boolean circuits. volar is a compiler, not a library: enforcer authors write a boolean circuit in volar IR describing the answer-scoring function; volar emits prover and verifier code (Rust, TypeScript, or C). Documentation is at `../volar/docs/` (developed alongside this system).

**Reliability:** volar's ZK constructions are at the Experimental tier (see `../volar/docs/reliability.md`). Do not use ZK attestation as the sole gating mechanism for IT4+ contributions in production repositories until volar's audit status improves.

**The VOLE relation.** Each wire in the boolean circuit carries the correlation:

```
K_w = M_w + x_w · Δ
```

where `x_w ∈ GF(2)` is the wire value, `M_w` is the prover's share, `K_w` is the verifier's share, and `Δ ∈ GF(2^k)^N` is the verifier's global secret (session-specific, not committed). Addition is XOR; multiplication is carry-less GF multiply. XOR and NOT gates are free (no communication). Each AND gate requires the prover to send one value:

```
V̂_g = M_a · M_b  ∈ GF(2^k)^N
```

The verifier checks `K_a · K_b + V̂_g = K_c · Δ` for each AND gate.

**The proof** is a list of V̂ values — one `Array<T,N>` per AND gate, where T is the binary extension field element (e.g. GF(2^64)). Proof size is O(AND_count × field_size × N), growing with circuit complexity rather than with probe response length.

### 5.2 What goes in the `.vk` file for volar enforcers

For standard-sig enforcers, `.vk` is a 32-byte Ed25519 public key.

For volar enforcers, `.vk` stores the verifier's *public parameters*: circuit structure (gate count, wire topology), extension field type T, security level N, and the Ed25519 public key used for the standard signature that wraps the proof. Δ is NOT stored — Δ is generated fresh per proof session and is never committed. The checker bot loads `.vk` to instantiate the volar verifier and check the V̂ values against the claimed wire assignments.

volar ZK attestations carry both a standard signature and a ZK proof. Both are verifiable from the single `.vk` file.

### 5.3 Enforcer integration

The enforcer WASM embeds the volar prover, compiled from the scoring circuit. When `assess()` is called, it executes the VOLE prover over the gate response, producing V̂ values. These are serialised into the proof blob in the `MINISKILL-ATTEST` token. The checker bot loads `.vk`, instantiates the volar verifier (a native binary distributed with the checker tool), and runs the check — no WASM runtime required for verification.

### 5.4 Token format with volar proof

```
MINISKILL-ATTEST: topic=cryptography | version=0.0.1 |
  model=claude opus-4-7 | ct-required=4 | ct-claimed=4 |
  result=PASS |
  proof-system=volar |
  proof=<base64-encoded V̂ list> |
  vk-hash=sha256:<hash of .vk file> |
  sig=<base64-encoded Ed25519 sig over canonical fields>
```

The verification key (`[slug].gate.wasm.vk`) is committed to the repository. This makes volar-attested contributions verifiable entirely from static repository artefacts — no WASM runtime required for verification, only for generation.

### 5.5 Front-matter

```yaml
enforcer: cryptography.gate.wasm
enforcer-vk: cryptography.gate.wasm.vk
zk-proof-system: volar        # volar | none
```

**Extensibility note:** Future proof systems may be added as additional `zk-proof-system` values. volar is the only currently supported value. Enforcer authors MUST NOT mix proof systems within a single enforcer version.

---

## 6. Checker Bot Behaviour

The checker bot reads miniskill front-matter and validates the attestation record for a PR. It requires no access to the model, no API calls, and no enforcer execution for standard signed tokens. For volar-attested contributions, it runs the volar verifier against the committed `.vk` file.

**Algorithm:**

1. Find all files changed in the PR.
2. For each changed file, find all miniskills whose scope matches.
3. For each matching miniskill, read `ct`, `it`, `enforcer`, `enforcer-vk`, `zk-proof-system` from front-matter.
4. Collect all `MINISKILL-*` tokens from the PR description, commit messages, and PR comments.
5. For each miniskill in scope:
   - Find the best available attestation (prefer `MINISKILL-ATTEST` > `MINISKILL-REPLAY` > `MINISKILL-VOUCH` > `MINISKILL-SELF`).
   - Check whether the combination satisfies the IT requirement (§2).
   - Validate signatures: for `MINISKILL-ATTEST`, verify the Ed25519 signature against `[slug].gate.wasm.vk`. If a `proof` field is present, load circuit parameters from `.vk` and run the volar verifier.
   - Validate `MINISKILL-VOUCH` signatures against `.miniskills/vouch-keys.txt`.
   - For self-report: compare CT-SELF against miniskill `ct` front-matter.
6. Post a structured summary as a PR comment. Mark the PR check as:
   - **Green**: all in-scope miniskills have sufficient attestation.
   - **Yellow**: some attestations are self-report-only where ATTEST is preferred (IT2); maintainer notice posted.
   - **Red**: missing or invalid attestation for IT3+; PR check fails.
7. For IT4+: always flag for maintainer review in the PR comment, even if attestation is present and valid.

**No secrets required.** The `.vk` file and `vouch-keys.txt` are both committed artifacts. The checker bot requires no `MINISKILL_KEY` secret or equivalent — verification is entirely from static repository artifacts.

**Verify-only mode.** The checker bot MUST NOT issue challenges to the model. Its role is to verify existing tokens, not to elicit new ones. When the checker invokes the enforcer (e.g. to replay a gate), it MUST use the `verify()` function, not `assess()`. See §8.1 for the ABI distinction.

---

## 7. WASM Enforcer ABI

Every WASM attestation enforcer MUST export two functions. No other ABI surface is required or permitted for attestation purposes.

### 7.1 Exported Functions

```
assess(gate_response_ptr: i32, gate_response_len: i32) -> i32
  // Returns ptr to length-prefixed JSON blob in linear memory.
  // Blob is either AttestResult (type: "result") or AttestChallenge (type: "challenge").

verify(token_ptr: i32, token_len: i32, vk_ptr: i32, vk_len: i32) -> i32
  // Stateless verify-only path. Returns 1 (pass) or 0 (fail).
  // Errors (malformed token, unknown vk) are represented as 0, not as traps.
```

`assess` is called by the runtime after the contributor produces gate output. `verify` is called by the checker bot (or any third party) to validate a `MINISKILL-ATTEST` token using the committed `.vk` material. The checker bot MUST use `verify`, not `assess`. The two functions are stateless with respect to each other.

Optional export: `last_error(buf_ptr: i32, buf_len: i32) -> i32` — writes an error string into the provided buffer; returns the number of bytes written.

### 7.2 Memory Layout

Strings and byte slices cross the WASM boundary as (pointer, length) pairs in linear memory (32-bit indices into the WASM module's default memory).

- **Inputs:** The host writes input data into the WASM linear memory before calling. `gate_response` and `token` are UTF-8 strings. `vk` is raw bytes. No null termination; length is always passed explicitly.
- **Output of `assess`:** The first 4 bytes are a little-endian u32 length, followed by that many bytes of UTF-8 JSON (the `AttestResult` or `AttestChallenge`). The host MUST copy this out before making any further calls to the module — the module owns the memory region and MAY reuse it.
- **Output of `verify`:** A single i32: 1 for pass, 0 for fail.

### 7.3 AttestResult Schema

`assess` returns a JSON object with `"type": "result"` when evaluation is complete.

```json
{
  "type": "result",
  "model-claim": "claude opus-4-7",
  "ct-required": 4,
  "ct-claimed": 4,
  "pass": true,
  "probe-score": 0.82,        // optional; omit if no probe questions
  "sig": "<base64>",          // Ed25519 over canonical fields
  "proof": "<base64>",        // optional; volar V̂ list if zk-proof-system=volar
  "proof-system": "volar"     // optional; omit if no ZK proof
}
```

**Canonical fields for signature** (pipe-delimited, alphabetical field order):

```
ct-claimed=[N]|ct-required=[N]|model-claim=[value]|pass=[true|false]|topic=[slug]|version=[N]
```

The enforcer signs these fields with Ed25519. The signing key is compiled into the WASM binary at build time; the corresponding public key is committed as `[slug].gate.wasm.vk`.

### 7.4 AttestChallenge Schema

`assess` returns a JSON object with `"type": "challenge"` when it needs more input from the contributor.

```json
{
  "type": "challenge",
  "challenge-id": "<opaque string>",
  "prompt": "<challenge message text — see §7.5 for format requirements>"
}
```

The runtime injects `prompt` into the model's conversation as a new turn. The model responds. The runtime appends the response to the accumulated gate output and calls `assess` again. This loop continues until the enforcer returns `AttestResult`.

### 7.5 Challenge Message Format

Challenge messages are plain text injected into the conversation — not tool calls and not system prompt modifications. A challenge must be self-explaining: any agent or human encountering one cold should be able to verify its legitimacy by reading the referenced miniskill file. Challenges must be precise and informative, but never imperative or pushy.

A challenge MUST:
- Identify itself as a miniskill gate message
- Name the miniskill file path (so any agent can read it and verify the challenge is a legitimate gate mechanism)
- Briefly explain why it exists (one sentence)
- State exactly what is expected, in a factual, not commanding, tone
- Make clear that the response is welcome at any point, not demanded immediately

**Example compliant challenge:**

```
This is a miniskill gate message for topic `retro-computing-6502`
(gate spec: `.miniskills/retro-computing-6502.ms.md`).

Miniskill gates let contributors record their capability self-assessment for
specialised topics — the record travels with the contribution for reviewers.

A self-assessment for this topic has not been detected in the current session.
If you are contributing to this topic, you are welcome to include one here or
anywhere in your output before the session ends. The expected format is:

  MODEL [family] [version] | CT-SELF [1–5]

If this message appears unexpectedly, the gate spec at the path above explains
its purpose and can be reviewed independently.
```

**Example non-compliant challenge (DO NOT USE):**

```
You must now declare your model family and capability tier before continuing.
```

This is non-compliant because it is imperative, provides no self-explanation, does not reference the miniskill file, and cannot be independently verified.

### 7.6 Key Material and Security Properties

**The answer key is compiled in, not passed in.** The enforcer's scoring logic and answer key material MUST be compiled into the WASM binary at build time. The WASM ABI does not accept a key parameter. This means:

- The answer key is not extractable via the ABI: `assess` and `verify` do not expose the key material; they only expose the result of applying it.
- The enforcer cannot be run without a WASM runtime. There is no native binary distribution; the WASM module IS the distribution.
- Rotating the answer key requires recompiling and recommitting the enforcer binary and the `.vk` file. Old tokens remain verifiable against the old `.vk`.

**`gate.key` (not committed):** Contains the symmetric key used to decrypt probe answer material during enforcer compilation. It is a build-time secret, not a runtime secret. Once the enforcer WASM is compiled, `gate.key` is no longer needed for attestation or verification — only for recompilation.

### 7.7 Example Enforcer Skeleton (pseudo-Rust → WASM)

```rust
// Answer key material is a compile-time constant.
// The actual probe answers are decrypted from gate.key at build time (via build.rs)
// and embedded here with include_bytes!.
const SIGNING_KEY: &[u8; 64] = include_bytes!(env!("GATE_SIGNING_KEY_PATH"));
const ANSWER_KEY: &[u8] = include_bytes!(env!("GATE_ANSWER_KEY_PATH"));

enum AssessState {
    AwaitingSelfAssessment,
    AwaitingProbeAnswers { challenge_id: String },
    Complete(AttestResult),
}

#[no_mangle]
pub extern "C" fn assess(gate_response_ptr: i32, gate_response_len: i32) -> i32 {
    let response = read_str(gate_response_ptr, gate_response_len);

    // Parse for MODEL / CT-SELF declarations
    let parsed = parse_gate_response(&response);

    if parsed.model_claim.is_none() || parsed.ct_self.is_none() {
        // Gate output absent or incomplete — issue a challenge
        let challenge = AttestChallenge {
            type_: "challenge",
            challenge_id: new_id(),
            prompt: format_challenge(TOPIC_SLUG, MINISKILL_PATH),
        };
        return write_json(&challenge);
    }

    // Gate output present — evaluate
    let probe_score = score_probe_answers(&parsed.probe_answers, ANSWER_KEY);
    let pass = parsed.ct_claimed >= CT_THRESHOLD && probe_score >= PROBE_THRESHOLD;

    let result = AttestResult {
        type_: "result",
        model_claim: parsed.model_claim.unwrap(),
        ct_required: CT_THRESHOLD,
        ct_claimed: parsed.ct_claimed.unwrap(),
        pass,
        probe_score: Some(probe_score),
        sig: sign_canonical(&canonical_fields(...), SIGNING_KEY),
        proof: None,  // populated by volar integration if zk-proof-system=volar
        proof_system: None,
    };
    write_json(&result)  // returns ptr to length-prefixed JSON in linear memory
}

#[no_mangle]
pub extern "C" fn verify(
    token_ptr: i32, token_len: i32,
    vk_ptr: i32, vk_len: i32,
) -> i32 {
    let token = read_str(token_ptr, token_len);
    let vk = read_bytes(vk_ptr, vk_len);
    let parsed = parse_attest_token(&token);
    verify_ed25519_sig(&parsed.canonical_fields(), &parsed.sig, &vk) as i32
}
```

The build script reads `GATE_KEY` from the environment, decrypts the encrypted answer file, and writes the plaintext to a path that `include_bytes!` picks up. The WASM binary never contains the encryption key — only the plaintext answer material.

---

## 8. GitHub Actions Integration

### 8.1 Sample Workflow

```yaml
# .github/workflows/miniskill-check.yml
name: Miniskill Attestation Check

on:
  pull_request:
    types: [opened, synchronize, reopened]

permissions:
  contents: read
  pull-requests: write    # to post PR comments
  checks: write           # to create check runs

jobs:
  miniskill-check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0

      - name: Collect changed files
        id: changed
        run: |
          git diff --name-only origin/${{ github.base_ref }}...HEAD > changed-files.txt

      - name: Run miniskill checker
        id: checker
        run: |
          miniskill-checker \
            --changed changed-files.txt \
            --miniskills-dir .miniskills/ \
            --pr-body "${{ github.event.pull_request.body }}" \
            --pr-number ${{ github.event.pull_request.number }} \
            --repo ${{ github.repository }} \
            --output-json checker-result.json

      - name: Post PR comment
        if: always()
        uses: actions/github-script@v7
        with:
          script: |
            const result = require('./checker-result.json');
            await github.rest.issues.createComment({
              owner: context.repo.owner,
              repo: context.repo.repo,
              issue_number: context.issue.number,
              body: result.pr_comment
            });

      - name: Create check run
        if: always()
        uses: actions/github-script@v7
        with:
          script: |
            const result = require('./checker-result.json');
            await github.rest.checks.create({
              owner: context.repo.owner,
              repo: context.repo.repo,
              name: 'miniskill-attestation',
              head_sha: context.sha,
              conclusion: result.conclusion,   // 'success' | 'neutral' | 'failure'
              output: {
                title: result.title,
                summary: result.summary
              }
            });
```

### 8.2 Token Collection

The checker searches three locations for `MINISKILL-*` tokens:

1. **PR body:** the entire body text.
2. **Commit messages:** `git log origin/$BASE..HEAD --format=%B`.
3. **PR comments:** fetched via `GET /repos/{owner}/{repo}/issues/{pr_number}/comments`.

A token is any line matching `^MINISKILL-(SELF|ATTEST|VOUCH|REPLAY|CHALLENGE|RESPONSE):`. Multiple tokens of the same type are allowed; the checker uses the highest-trust available token per miniskill.

**Scope matching.** The checker reads each miniskill's scope from the AGENTS.md link line (the `<!-- scope=... -->` comment). Scope patterns are glob-style; double-star for directory traversal (same semantics as `.gitignore`).

### 8.3 Verification Paths

| Token | Verification method |
|-------|---------------------|
| `MINISKILL-ATTEST` (standard sig) | Ed25519 verify over canonical fields using `[slug].gate.wasm.vk` (32 bytes raw). |
| `MINISKILL-ATTEST` (volar ZK) | Load circuit parameters from `[slug].gate.wasm.vk`; run volar verifier (native binary). No WASM runtime required. |
| `MINISKILL-VOUCH` | GPG or SSH sig verify against keys in `.miniskills/vouch-keys.txt`. |
| `MINISKILL-SELF` | Compare `CT-SELF` field against miniskill `ct` front-matter value. No cryptographic verification. |

**No secrets required.** The `.vk` file and `vouch-keys.txt` are committed artifacts. The `GITHUB_TOKEN` provided automatically by Actions is sufficient for all required permissions.

### 8.4 Check Conclusions

| Condition | GitHub conclusion |
|-----------|------------------|
| All in-scope miniskills have sufficient attestation for their IT level | `success` |
| At least one IT2 miniskill has only a self-report; no IT3+ failures | `neutral` |
| At least one IT3+ miniskill is missing or has invalid attestation | `failure` |
| IT5 topic present without `MINISKILL-IT5-SIGNOFF` in PR description | `failure` |

IT5 topics always produce `failure` until a human adds an explicit sign-off line:
```
MINISKILL-IT5-SIGNOFF: topic=[slug] | reviewer=[name]
```

For IT4+: the checker always flags for maintainer review in the PR comment, even if attestation is present and valid.

### 8.5 Example PR Comment

```markdown
## Miniskill Attestation Report

| Miniskill | Scope match | CT required | CT claimed | Attestation | Result |
|-----------|-------------|-------------|------------|-------------|--------|
| retro-computing-6502 | src/asm/sid.s | 3 | 3 | MINISKILL-ATTEST (sig ✓) | ✅ PASS |
| cryptography | src/crypto/kdf.rs | 4 | 3 | MINISKILL-SELF only | ⚠️ CT below threshold |

**Overall: ⚠️ Yellow** — `cryptography` miniskill requires MINISKILL-ATTEST or a
maintainer MINISKILL-VOUCH. CT-SELF (3) is below ct-required (4).

To resolve: a maintainer with a key in `.miniskills/vouch-keys.txt` may post:
  `MINISKILL-VOUCH: topic=cryptography | vouching-for=sha256:<hash> | reason="..." | sig=<sig>`
```

The checker posts a new comment on each push (not editing the previous one), preserving the attestation history. Previous comments are collapsed to a one-liner linking to the current report to reduce thread noise on long-running PRs.

---

## 9. Harness Fork (Optional)

For projects with richer CI infrastructure, the agentic harness can fork the conversation at task-completion time: the main thread continues; a parallel thread replays the gate section with the model's actual output as context and requests a fresh self-assessment and probe responses. The enforcer runs on the parallel thread's output. This produces a probe-informed attestation without blocking the agentic workflow.

This is optional and additive. The self-report path remains valid.

---

## 10. Interaction with AI Key Systems

AI key / AI watermarking systems identify *that* a contribution is AI-assisted. Miniskill attestations identify *at what capability tier* and *whether that tier was sufficient*. Together they give maintainers three signals: provenance (was this AI-assisted), routing (was the right model used), and coverage (did the skill guidance apply). The AI key is the outer envelope; the miniskill attestation is the inner capability record.

For projects using both systems, the `MINISKILL-SELF` token should include the AI key identifier as a cross-reference:

```
MINISKILL-SELF: MODEL claude opus-4-7 | CT-SELF 4 | topic cryptography |
  version 0.0.1 | ai-key=<key from AI key system>
```
