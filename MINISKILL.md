# MINISKILL.md — Draft Specification v0.0.6

> A **miniskill** is a single Markdown file, linked from `AGENTS.md`, that combines in-context topic guidance with a self-reported capability declaration and an optional WASM attestation target. The skill loads lazily when the model touches scoped files. The gate does not block upfront — it attests *after the fact*, letting checker bots, PR harnesses, and maintainers replay and verify the assessment without requiring harness changes to the agentic workflow itself.

---

## 1. Design Principles

**Optimistic loading, retroactive enforcement.** The skill text is injected freely into context — it costs nothing and harms nothing if an incapable model reads it. The gate's job is not to prevent reading; it is to produce a verifiable attestation that travels with the model's output into review. A bad attestation (or a missing one) is caught at PR time, not at task-start time, which is where quality gates on contributions belong anyway.

**No mandatory harness changes.** The miniskill works as pure context injection today. The WASM attestation layer is additive — runtimes that support it produce richer signals; runtimes that don't still surface the self-report. Checker bots can replay the gate independently from a PR comment. Nothing about the design requires the agentic workflow to change.

**Two independent tier axes.** *Capability tier* (CT) describes the minimum model class that can produce correct output for this topic. *Impact tier* (IT) describes how much damage wrong output causes, which determines what attestation strength is required before merge. These are orthogonal: a low-CT / high-IT topic (e.g. well-known API, production-critical path) needs minimal model gating but strong review. A high-CT / low-IT topic (e.g. conlangs, demoscene art) needs strong model gating but the stakes of a mistake are low.

**Attestation is a conversation, not a one-shot token.** The model, the WASM enforcer, checker bots, and maintainers each contribute independently verifiable signals. These compose — multiple tokens of different types can cover a single contribution — and accumulate across the PR review lifecycle. A maintainer vouch can substitute for a missing mechanical attestation; a checker bot replay can provide an independent third-party signal; a ZK proof can make the enforcer's evaluation independently verifiable without revealing what it evaluated.

**Single file per miniskill.** The skill and gate spec live together. The WASM enforcer is a sidecar, not a required component. Simpler to author, simpler to review, simpler to link.

**Prior art.** The immediate precursor to this system is the reliability and AI tier policy in [`../volar/docs/reliability.md`](../volar/docs/reliability.md). That system routes AI contributions within a single trusted codebase using hardcoded model-to-tier mappings (Glue / Compiler / Cryptography), enforced through human code review — no attestation, no portability. Miniskills extend the same insight (capability routing is a precondition for AI contribution quality, not a post-hoc filter) into an open, portable, machine-attested form.

---

## 2. File Layout

```
.miniskills/
  retro-computing-6502.ms.md      # the miniskill (skill + gate spec in one file)
  retro-computing-6502.gate.wasm  # optional WASM attestation enforcer
  retro-computing-6502.gate.wasm.vk   # enforcer verification key — committed
                                      # standard-sig enforcer: Ed25519 public key (32 bytes, raw)
                                      # ZK enforcer: circuit public parameters (see ATTESTATION.md §5)
  retro-computing-6502.gate.key   # decryption key for answer material — NOT committed
  vouch-keys.txt                  # maintainer signing keys authorised to vouch
  refs/
    sid-register-map.md           # pinned reference documents
```

`AGENTS.md` links miniskills with tagged Markdown links:

```markdown
## Miniskills

[miniskill: retro-computing-6502](.miniskills/retro-computing-6502.ms.md) <!-- scope=src/asm/**,*.s,*.asm -->
[miniskill: conlang](.miniskills/conlang.ms.md) <!-- scope=src/conlang/** -->
[miniskill: cryptography](.miniskills/cryptography.ms.md) <!-- scope=src/crypto/**,tests/crypto/** -->
```

The `scope` attribute is in the trailing HTML comment on each link line — this keeps the miniskill file portable across projects and lets a project override scope without editing the miniskill itself. Tooling locates miniskill declarations by matching the pattern `\[miniskill:[^\]]+\]\([^)]+\)`.

**Lazy loading:** the runtime injects a miniskill's contents only when the current task touches at least one file matching the declared scope. An unmatched miniskill costs zero tokens.

---

## 3. Miniskill File Format (`.ms.md`)

A miniskill is a single Markdown file with two clearly delimited sections: `## Skill` and `## Gate`. The YAML front-matter belongs to the file as a whole.

```markdown
---
name: retro-computing-6502
version: 0.0.1
topic-class: frozen-knowledge
ct: 3          # Capability Tier — minimum model class; see §5
it: 2          # Impact Tier — review weight; see §6
enforcer: retro-computing-6502.gate.wasm    # optional; omit if no WASM enforcer
enforcer-vk: retro-computing-6502.gate.wasm.vk  # required when enforcer is present
---

## Skill

[Everything the model reads. Topic context, gotchas, references, permissions,
 prohibitions. See §4.]

---gate---

## Gate

[Protocol declaration the contributor fills out. Self-assessment format and
 attestation instructions. See §7.]
```

The `---gate---` delimiter is the only structural requirement beyond valid YAML front-matter. Runtimes inject the `## Skill` section into model context. The `## Gate` section is injected after the skill, as the final instruction block, describing what the contributor must self-report and — if an enforcer is present — how attestation is produced.

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

**CT and the self-assessment:** At gate time, the contributor declares their model family and version. The WASM enforcer (if present) compares this against the CT threshold and records whether the model meets it. If no enforcer is present, the self-report stands and is reviewable by a maintainer.

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

The gate is the final block of the miniskill, injected after the skill. It describes the attestation protocol contributors must follow. It is always shown — unlike v0.0.3, the gate is not withheld until after a probe run. The contributor reads it, does the work, then fills it out.

The gate text is written in the third person as a protocol specification rather than as a direct instruction. This prevents the gate from being misread as a command when injected into a checker bot's context or when encountered by a reviewer's automated tooling.

For token format details, enforcer ABI, and the interactive attestation flow (including the challenge/response mechanism when gate output is absent), see [`ATTESTATION.md`](ATTESTATION.md).

### 7.1 Structure

```markdown
---gate---

## Gate

### Self-Assessment

A contributor filling this gate MUST include the following in their gate output:

  `MODEL: [family] [version]`  — the contributor's model family and version.
  `CT-SELF: [1–5]`             — the contributor's self-assessed capability tier for this topic.

If CT-SELF is below the declared threshold (ct: N), the contributor's output MUST
be restricted to scaffolding and comments; no domain logic MAY be included. The
contributor MUST state this restriction explicitly in their output.

A CT-SELF above the declared threshold without documented external-compiler evidence
from this session is not a valid claim. The enforcer will record whether the claim
is supported.

### Attestation

If a WASM enforcer is present at `.miniskills/[slug].gate.wasm`, the runtime will
invoke it on the contributor's gate output and produce a signed attestation token.
The contributor MUST include the token in their commit message or PR description:

  `MINISKILL-ATTEST: [token]`

If no enforcer is present, the contributor MUST include the self-assessment verbatim:
  `MINISKILL-SELF: MODEL [family] [version] | CT-SELF [N] | topic [slug]`

Both forms are machine-readable by checker bots. The self-report form carries no
cryptographic guarantee and is subject to maintainer review.

See ATTESTATION.md for token format details and enforcer ABI.
```

### 7.2 Self-Assessment vs. Enforcer Attestation

These are two points on a continuum of trust, not two different mechanisms:

**Self-report** (`MINISKILL-SELF`): The contributor declares their own model family, version, and CT. Costs zero infrastructure. A maintainer or checker bot can read it and flag contributions from models below the CT threshold. Gameable by a motivated bad actor, but: (a) most AI-assisted contributions are not adversarial, and (b) the self-report still creates a traceable audit trail and surfaces the issue in PR review.

**WASM attestation** (`MINISKILL-ATTEST`): The runtime runs the WASM enforcer after the contributor's self-assessment, verifies the claimed model identity against the CT threshold (and optionally runs probe questions against encrypted answer keys), and produces a signed token. The token is verified by CI without re-running the model. The model cannot forge it; it can only trigger the run and include the result.

The WASM enforcer for attestation purposes is simpler than the v0.0.3 gate design: it does not need to block context injection, it does not need to withhold answers, and it does not need to run before the skill is read. Its job is to sign a statement of the form: "model X claimed CT Y for topic Z; the declared CT threshold is W; X ≥ W: [true/false]; probe score: [optional]." The signature is what matters for CI verification.

### 7.3 Probe Questions (Optional)

For topics where CT self-report is insufficient (IT3+), the gate section may include probe questions. The contributor answers them as part of their gate output; the WASM enforcer evaluates the answers and folds the result into the attestation token.

Unlike v0.0.3, the answers are not withheld from the contributor — the goal is not to trick the model, it is to produce a verifiable record of what the model actually knows. A model that answers probe questions correctly and still produces wrong output has a gotcha problem (the skill section needs updating), not a gate problem. A model that answers probe questions incorrectly has declared its own CT limitation.

```markdown
### Probe (for IT3+ contributions)

The contributor MUST answer the following before submitting. Answers are evaluated
by the enforcer and included in the attestation token.

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

## 11. Miniskill Lifecycle

**Creation**: One documented failure is enough to justify a miniskill. Start with just a skill section and a CT1/IT1 declaration; raise CT and IT as evidence accumulates.

**Iteration**: Each new failure that passed the existing gate → add a gotcha. Each failure that passed the gate *and* was CT-appropriate → the gate has a gap; consider new probe items or a CT raise.

**Raising IT**: When a failure causes production impact (not just review failure), raise IT. Raising IT triggers a requirement for stronger attestation, which is low-friction for maintainers and high-friction for contributors producing low-quality AI output — exactly the right direction.

**Deprecation**: `status: deprecated` in front-matter. Never delete. A miniskill is a candidate for deprecation when external compiler enforcement fully subsumes its gotcha list, or the project no longer touches the scope.

---

## 12. Anti-Patterns

- **Do not front-load mandatory gates.** The optimistic-load model exists for a reason: blocking capable models from starting is worse than catching bad output in review.
- **Do not conflate CT and IT.** High CT (hard topic) does not mean high IT (dangerous output). Conlangs are CT5/IT1. A trivial CRUD endpoint on a payment processor is CT1/IT4.
- **Do not use self-report as the only signal for IT3+ topics.** Self-reports are gameable and unverifiable. WASM attestation or maintainer review is required at IT3+.
- **Do not write gotchas as style guidance.** "Be careful with registers" is not a gotcha. "Do not use $D000 as the SID base address; it is the VIC-II base" is a gotcha.
- **Do not set CT5 casually.** CT5 means only Mythos-class models can contribute. Use it only for topics where lower-tier output is structurally broken, not just imperfect.
- **Do not commit `gate.key`.** The key is the only secret in this system. Committing it reduces WASM attestation to a tamper-evident seal rather than a tamper-proof one.
- **Do not copy gotchas across miniskills without re-testing.** Failure modes are domain-specific, not topic-class-specific.
- **Do not mark a model below a CT threshold without a documented failure instance.** CT declarations are evidence-based.

---

## 13. Appendix: Miniskill Template

```markdown
---
name: [slug]
version: 0.0.1
topic-class: [frozen-knowledge | sparse-coverage | rapidly-evolving | high-stakes-precision | adversarial-corpus | model-personality-sensitive]
ct: [1–5]       # minimum model capability tier
it: [1–5]       # output impact tier; determines required attestation strength
enforcer: [slug].gate.wasm          # omit if no WASM enforcer
enforcer-vk: [slug].gate.wasm.vk    # required when enforcer is present
zk-proof-system: [volar | none]     # omit if no ZK proof; see ATTESTATION.md §5
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

A contributor filling this gate MUST include the following in their gate output:

  `MODEL: [family] [version]`
  `CT-SELF: [1–5]`

If CT-SELF is below ct: [N] declared for this miniskill, the contributor's output
MUST be restricted to comments and scaffolding. The contributor MUST state this
restriction explicitly.

### Attestation

Include one of the following in the commit message or PR description.

If WASM enforcer is available (produces a cryptographic attestation):
  `MINISKILL-ATTEST: topic=[slug] | version=[N] | model=[family] [version] | ct-required=[N] | ct-claimed=[N] | result=[PASS|FAIL] | [proof fields if ZK] | sig=[enforcer sig]`

If no enforcer (self-report only):
  `MINISKILL-SELF: MODEL [family] [version] | CT-SELF [N] | topic [slug] | version [miniskill-version]`

For IT4+ contributions, a maintainer vouch or checker bot challenge is expected.
See ATTESTATION.md for token format details, enforcer ABI, and the interactive
attestation flow.

### Probe (omit for IT1–IT2; include for IT3+)

[Probe questions here. The contributor answers them; answers are evaluated by the
enforcer or reviewed by a maintainer. Answer material belongs in the enforcer's
encrypted answer key, not in the gate text.]
```

### `AGENTS.md` Link Syntax

```markdown
## Miniskills

[miniskill: retro-computing-6502](.miniskills/retro-computing-6502.ms.md) <!-- scope=src/asm/**,*.s,*.asm -->
[miniskill: cryptography](.miniskills/cryptography.ms.md) <!-- scope=src/crypto/** -->
[miniskill: conlang](.miniskills/conlang.ms.md) <!-- scope=src/conlang/** -->
```

Scope is on the link line. The miniskill file itself is scope-agnostic and portable across projects.
```
