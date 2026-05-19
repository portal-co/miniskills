# goals.md — Miniskill System Implementation Roadmap

All components are `planned` unless noted otherwise.

---

## 1. `miniskill-parse` — Parser and emitter library `in-progress`

Rust crate (`crates/miniskill-parse/`). `nom`-based parsers for all machine-readable miniskill syntax; typed emitters for all token formats.

Parses:
- `.ms.md` files: front-matter (via `serde_yaml`), skill section, gate section
- AGENTS.md link lines: `[miniskill: slug](path) <!-- scope=... -->`
- `MINISKILL-*` token lines: SELF, ATTEST, VOUCH, REPLAY, CHALLENGE, RESPONSE

Emits: canonical wire-format strings for all token types.

Foundation for all other components.

---

## 2. `miniskill-check` — Checker bot CLI

CLI tool implementing the checker bot algorithm (ATTESTATION.md §6).

- Reads AGENTS.md link lines; resolves scope matches for a given set of changed files
- Collects `MINISKILL-*` tokens from PR body, commit messages, and PR comments
- Validates attestations: volar native verifier for ZK proofs, Ed25519 for standard-sig, GPG/SSH for standalone vouches
- Posts structured PR comment summaries (green/yellow/red)
- Creates GitHub check runs via the GitHub API
- Can post challenges as PR comments when gate output is absent and the harness is untrusted

GitHub Actions integration: `.github/workflows/miniskill-check.yml` (see ATTESTATION.md §8).

---

## 3. `miniskill-server` — Local API

HTTP server wrapping checker logic. Intended for editor integrations (VS Code, JetBrains) and local CI hooks that cannot run a full CLI.

Endpoints:
- `POST /check` — check a set of changed files against AGENTS.md scope; return attestation status per miniskill
- `POST /validate-token` — parse and validate a `MINISKILL-*` token string
- `GET /scope?path=<file>` — list miniskills whose scope matches a given file path

---

## 4. Guide — Agent skill + human web UI

Two parts, developed together.

**Agent skill** (`.ms.md` file or files): a miniskill about miniskills. Teaches AI agents how to work with the system:
- How to fill out the gate section self-assessment
- How to include `MINISKILL-SELF` / `MINISKILL-ATTEST` tokens in commits and PRs
- How to read checker bot output and respond to challenges
- Common mistakes (wrong format, missing topic slug, forgetting probe answers)

This is a miniskill that any project adopting the system should link in its AGENTS.md.

**Human web UI**: a web app for project maintainers.
- Browse miniskills in a project (read AGENTS.md scope graph)
- View CT/IT requirements and enforcement status per miniskill
- View attestation history for recent PRs
- Manage `vouch-keys.txt` (add/revoke maintainer signing keys)
- Visualise the scope graph (which files are covered by which miniskills)

---

## 5. `miniskill-link` — Volar integration

CLI tool implementing ATTESTATION.md §5.3.

```
miniskill-link \
  --enforcer   [slug].gate.wasm \
  --vouch-keys .miniskills/vouch-keys.txt \
  --answer-key $GATE_KEY \
  --output-vk  [slug].gate.wasm.vk
```

- Links enforcer WASM + standard Ed25519 verifier WASM + vouch public keys + answer material into a composed WASM input
- Feeds composed WASM through volar's WASM frontend: WAFFLE → VAFFLE → Volar IR
- Writes `.vk` from the composed circuit's public parameters

Depends on volar being importable as a Rust library or callable as a subprocess. Blocked on volar's WASM frontend reaching a usable state.

---

## Affected repositories (on adoption)

When the miniskill system expands beyond this repo, the initial affected repositories are:

- `volar` (this system's origin repo)
- `dreamcomp`
- `jsaw`
- all transitive dependencies under `github.com/portal-co`

Each adopting repo links miniskills from its `AGENTS.md`. The `miniskill-check` CI step is added to each repo's GitHub Actions. `miniskill-parse` is a shared library dependency.
