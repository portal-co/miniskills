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

**Checker bot challenge framing.** Checker bot challenges appear as PR comments and, in agentic workflows, are read as tool call results — they are process output, not user directives. Challenge text MUST attribute the query to the checker process, use question form rather than imperative, and avoid urgent or commanding language. The goal is to gather information for the attestation record, not to direct the contributor's behaviour. Neutral process framing avoids triggering model safety filters that flag commanding or authority-claiming text.

Compliant example:
```
MINISKILL-CHALLENGE: topic=cryptography | challenge-id=c7f3a | source=checker-bot |
  "The miniskill checker is reviewing the changes to src/crypto/kdf.rs.
   Regarding the key derivation at line 42: the checker would find it useful to
   understand why HKDF-SHA256 is used here rather than PBKDF2.
   A response here or in the PR description helps with the attestation record."
```

Non-compliant example (DO NOT USE — imperative, authority-claiming, likely to trigger safety filters):
```
MINISKILL-CHALLENGE: topic=cryptography | challenge-id=c7f3a |
  "You must immediately explain your key derivation choice."
```

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

The supported ZK proof system is **volar** — a system implementing VOLE-in-the-head ZK (Quicksilver construction) over boolean circuits. Documentation is at `../volar/docs/` (developed alongside this system).

**Reliability:** volar's ZK constructions are at the Experimental tier in volar's internal reliability classification (see `.miniskills/refs/reliability.md` — a reference copy; not a required component of miniskills). Do not use ZK attestation as the sole gating mechanism for IT4+ contributions in production repositories until volar's audit status improves.

**The WASM frontend.** The primary path for ZK-provable enforcers is volar's WASM frontend, which lifts ordinary WASM bytecode through WAFFLE (a WASM CFG IR) and VAFFLE into volar's own IR. The enforcer WASM itself — the module linked in the miniskill's front-matter — is what gets lifted and proved. There is no intermediate build step that embeds a prover into the enforcer; the enforcer is plain logic, and volar proves its execution dynamically at attestation time.

**The linker tool** (`miniskill-link`) composes the enforcer WASM with external components before passing them to volar. This is described in §5.3.

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

For standard-sig enforcers (no ZK), `.vk` is a 32-byte Ed25519 public key.

For volar enforcers, `.vk` stores the verifier's *public parameters* for the **composed** circuit — the enforcer WASM linked together with its external components (Ed25519 verifier, vouch-keys, answer material) by the linker tool (§5.3). The public parameters include: circuit structure (gate count, wire topology), extension field type T, security level N, and the public inputs baked in from the linked components (vouch public keys, etc.). Δ is NOT stored — it is generated fresh per proof session. No separate Ed25519 signing key is committed; the ZK proof covers the entire computation, including any signature verification.

### 5.3 Enforcer integration and the linker tool

The enforcer WASM contains only domain-specific gate logic (probe scoring, challenge/response flow). It does not contain signing keys, ZK prover code, or verification logic for other token types. These are linked in externally by the `miniskill-link` tool, keeping the enforcer itself auditable and lightweight.

**`miniskill-link`** composes all components into a single WASM input for volar and derives the `.vk`. It is run once when the enforcer is first authored, and again whenever the enforcer or any component changes:

```
miniskill-link \
  --enforcer   [slug].gate.wasm \
  --vouch-keys .miniskills/vouch-keys.txt \
  --answer-key $GATE_KEY \
  --output-vk  [slug].gate.wasm.vk
```

The tool links:
- The enforcer WASM (scoring and challenge flow)
- A standard Ed25519 signature verifier WASM (for vouch token verification)
- Vouch public keys from `vouch-keys.txt` (as public constants in the circuit)
- The gate's answer key (decrypted at link time using `GATE_KEY`; not stored in the enforcer)

The combined WASM is lifted through volar's WASM frontend: WASM → WAFFLE → VAFFLE → Volar IR. volar generates the `.vk` from the composed circuit. The `.vk` is committed; the answer key and the linked Volar IR are not (the Volar IR is reproduced deterministically by running `miniskill-link` again from the same inputs).

**At attestation time:**

1. Runtime calls `enforcer.assess(gate_response)` — plain WASM execution; returns `AssessResult` (no ZK).
2. Runtime runs the volar prover on the enforcer's execution trace using the linked circuit. This produces V̂ values attesting: "the enforcer, run on these inputs with these vouch keys and answer materials, produced this result."
3. Runtime constructs the `MINISKILL-ATTEST` token from `AssessResult` and the V̂ values.
4. Checker bot runs the volar native verifier against `.vk` — no WASM runtime, no enforcer execution, no separate Ed25519 call. The volar proof covers the entire computation including vouch signature checks.

**What the proof covers:** Probe answer scoring, CT comparison, validity of any vouch signatures included in the gate response, and the correctness of the result. Nothing in the attestation is verified separately from the volar proof.

### 5.4 Token format with volar proof

```
MINISKILL-ATTEST: topic=cryptography | version=0.0.1 |
  model=claude opus-4-7 | ct-required=4 | ct-claimed=4 |
  result=PASS |
  proof-system=volar |
  proof=<base64-encoded V̂ list> |
  vk-hash=sha256:<hash of .vk file>
```

There is no separate `sig=` field for volar-attested tokens — the ZK proof is the cryptographic guarantee. The V̂ list, verified against `.vk`, proves that the composed computation (enforcer + vouch verifier + public keys) produced this result on these inputs. The `vk-hash` pins the specific linked version; if the enforcer or any component changes, `miniskill-link` must be re-run and a new `.vk` committed.

Verification requires only the committed `.vk` file and the volar native verifier — no WASM runtime, no enforcer execution, no external key material.

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
   - For `MINISKILL-ATTEST` with `proof-system=volar`: run the volar native verifier against `[slug].gate.wasm.vk`. The proof covers the entire computation — no separate Ed25519 call and no vouch-key validation needed; vouch verification is inside the proven circuit.
   - For `MINISKILL-ATTEST` without a proof (standard-sig enforcer): verify the Ed25519 signature against `[slug].gate.wasm.vk` (32-byte raw public key).
   - For `MINISKILL-VOUCH` tokens: validate the GPG/SSH signature against `.miniskills/vouch-keys.txt`. (Vouches that are referenced inside a volar-proven `MINISKILL-ATTEST` are already covered by the proof; standalone vouches still need explicit verification.)
   - For self-report: compare CT-SELF against miniskill `ct` front-matter.
6. Post a structured summary as a PR comment. Mark the PR check as:
   - **Green**: all in-scope miniskills have sufficient attestation.
   - **Yellow**: some attestations are self-report-only where ATTEST is preferred (IT2); maintainer notice posted.
   - **Red**: missing or invalid attestation for IT3+; PR check fails.
7. For IT4+: always flag for maintainer review in the PR comment, even if attestation is present and valid.

**No secrets required.** The `.vk` file and `vouch-keys.txt` are both committed artifacts. The checker bot requires no `MINISKILL_KEY` secret or equivalent — verification is entirely from static repository artifacts.

**Challenge capability.** The checker bot MAY post challenges as PR comments when no runtime-issued challenge has been detected and gate output is absent or insufficient. This is the appropriate path when the contribution harness is untrusted or when the runtime does not support the enforcer's `assess` flow. Checker-bot challenges follow the same scoping and framing requirements as §7.5 — they must be clearly attributed to the checker process, question form not imperative, and must reference the miniskill file path so the legitimacy can be independently verified.

The checker bot does NOT invoke the enforcer's `assess` function. Challenges it posts are formulated from the miniskill's gate section (the probe questions and self-assessment format), not from a live enforcer execution. Token verification uses the volar native verifier against `.vk`.

---

## 7. WASM Enforcer ABI

The enforcer WASM exports a single function: `assess`. There is no `verify` export — token verification uses the volar native verifier against the committed `.vk` file (see §5.3). The enforcer does not contain signing keys, ZK prover code, or vouch-verification logic; those are composed in by `miniskill-link` before passing to volar.

The enforcer's job is: given a gate response, either request more information (challenge) or return a plain result. volar proves the execution of this function, including all linked components, to produce the cryptographic attestation.

Optional export: `last_error(buf_ptr: i32, buf_len: i32) -> i32` — writes an error string; returns bytes written.

### 7.1 Exported Function

```
assess(gate_response_ptr: i32, gate_response_len: i32) -> i32
  // Plain WASM execution. Returns ptr to length-prefixed JSON in linear memory.
  // Blob is either AssessResult (type: "result") or AttestChallenge (type: "challenge").
  // The runtime separately runs volar to prove this execution and produce the
  // MINISKILL-ATTEST token.
```

### 7.2 Memory Layout

Strings and byte slices cross the WASM boundary as (pointer, length) pairs in linear memory (32-bit indices into the WASM module's default memory).

- **Input:** The host writes `gate_response` (UTF-8) into WASM linear memory before calling. No null termination; length passed explicitly.
- **Output:** The first 4 bytes are a little-endian u32 length, followed by that many bytes of UTF-8 JSON. The host MUST copy this out before making any further calls to the module — the module owns the memory region and MAY reuse it.

### 7.3 AssessResult Schema

`assess` returns a JSON object with `"type": "result"` when evaluation is complete. The runtime adds the proof, `ct-required`, topic, and version fields when constructing the `MINISKILL-ATTEST` token.

```json
{
  "type": "result",
  "model-claim": "claude opus-4-7",
  "ct-claimed": 4,
  "pass": true,
  "probe-score": 0.82        // optional; omit if no probe questions
}
```

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

**The enforcer WASM contains no key material.** Signing keys, vouch public keys, and answer material are linked in by `miniskill-link`, not compiled into the enforcer. This means the enforcer itself is fully auditable — it contains only the scoring and challenge logic, with no embedded secrets or cryptographic machinery.

**Key material flow:**
- `gate.key` (not committed): symmetric key used by `miniskill-link` to decrypt probe answer material at link time. It is a link-time secret. Once `miniskill-link` has run and produced the `.vk`, `gate.key` is not needed for attestation or verification — only for re-linking.
- Answer material is a public input to the composed volar circuit (encrypted under the proof's public parameters), not a runtime secret passed to the enforcer.
- Vouch public keys (from `vouch-keys.txt`) are public constants in the composed circuit, baked into `.vk` at link time.

**Rotating components:** Changing the enforcer WASM, vouch-keys, or answer material requires re-running `miniskill-link` and committing a new `.vk`. Old tokens remain verifiable against the old `.vk` indefinitely.

### 7.7 Example Enforcer and Linker Invocation

**The enforcer — plain scoring logic, no embedded keys or crypto:**

```rust
// enforcer/src/lib.rs
// No answer key. No signing key. No ZK prover.
// Scoring is against answer material linked in by miniskill-link.

#[no_mangle]
pub extern "C" fn assess(gate_response_ptr: i32, gate_response_len: i32) -> i32 {
    let response = read_str(gate_response_ptr, gate_response_len);
    let parsed = parse_gate_response(&response);

    if parsed.model_claim.is_none() || parsed.ct_self.is_none() {
        return write_json(&AttestChallenge {
            type_: "challenge",
            challenge_id: new_id(),
            prompt: format_challenge(TOPIC_SLUG, MINISKILL_PATH),
        });
    }

    // score_probe_answers operates against answer material that is a public
    // input to the volar circuit — not embedded here. The linker tool provides it.
    let probe_score = score_probe_answers(&parsed.probe_answers);
    write_json(&AssessResult {
        type_: "result",
        model_claim: parsed.model_claim.unwrap(),
        ct_claimed: parsed.ct_claimed.unwrap(),
        pass: parsed.ct_claimed.unwrap() >= CT_THRESHOLD && probe_score >= PROBE_THRESHOLD,
        probe_score: Some(probe_score),
    })
}
```

```
$ cargo build --target wasm32-unknown-unknown --release
# produces: target/wasm32-unknown-unknown/release/enforcer.wasm
# This is gate.wasm — commit it as-is.
```

**Run `miniskill-link` to compose components and derive `.vk`:**

```
$ miniskill-link \
    --enforcer   retro-computing-6502.gate.wasm \
    --vouch-keys .miniskills/vouch-keys.txt \
    --answer-key $GATE_KEY \
    --output-vk  retro-computing-6502.gate.wasm.vk
```

`miniskill-link` links the enforcer with the standard Ed25519 verifier WASM and the provided vouch keys and answer material. It feeds the composed WASM through volar's WASM frontend (WAFFLE → VAFFLE → Volar IR) and writes the `.vk` from the resulting circuit's public parameters. Commit `gate.wasm` and `gate.wasm.vk`; do not commit `$GATE_KEY`.

**At attestation time (handled by the runtime, not by the enforcer author):**

```
1. result   = enforcer.assess(gate_response)   # plain WASM call
2. v_hats   = volar.prove(enforcer, linked_circuit, gate_response, result)
3. token    = build_attest_token(result, v_hats, vk_hash)
   # MINISKILL-ATTEST: ... | proof=<V̂> | vk-hash=sha256:<.vk> | proof-system=volar
```

**Verification (checker bot, no WASM runtime needed):**

```
$ volar-verify retro-computing-6502.gate.wasm.vk <proof-from-token>
# Runs the volar native verifier against the committed .vk.
# No enforcer execution. No Ed25519 call. The proof covers everything.
```

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
| `MINISKILL-ATTEST` (`proof-system=volar`) | Run volar native verifier against `[slug].gate.wasm.vk`. Covers scoring, vouch verification, and CT check in one proof. No WASM runtime required. |
| `MINISKILL-ATTEST` (standard sig, no proof) | Ed25519 verify over canonical fields using `[slug].gate.wasm.vk` (32 bytes raw). |
| `MINISKILL-VOUCH` (standalone) | GPG or SSH sig verify against keys in `.miniskills/vouch-keys.txt`. Not needed for vouches already covered by a volar-proven ATTEST token. |
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
