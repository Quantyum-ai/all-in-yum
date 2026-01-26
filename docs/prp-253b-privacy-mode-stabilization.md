# PRP: Privacy Mode 2.5.3b — “100% GREEN” Stabilization (RAG + Tests + Hygiene)

## Objective
Bring `feat/privacy-mode-253b` to **true 100% approval** by eliminating ignored **unit** tests, fixing underlying RAG logic issues, removing clippy warnings under `-D warnings`, and cleaning repo hygiene (ownership/permissions + Phase 4 artifacts), while preserving the national‑security privacy boundary.

## Current Ground Truth (must re-verify before starting)
- Branch: `feat/privacy-mode-253b`
- HEAD: `20d527e` (“Remove non-US models…”)
- Working tree dirty with changes in:
  - `crates/aiy-privacy/src/rag/{chunk,embedder,indexer,mod,query}.rs`
  - `crates/aiy-privacy/src/lib.rs`
  - `crates/aiy-privacy/src/orchestration/error.rs`
  - untracked: `.github/workflows/`, `CLAUDE.md`, `crates/aiy-privacy/src/rag/test_support.rs`, `crates/aiy-privacy/tests/integration_tests.rs`
- Ignored tests: **12 total** (11 RAG unit tests + 1 verification unit test):
  - `crates/aiy-privacy/src/rag/query.rs` (3)
  - `crates/aiy-privacy/src/rag/mod.rs` (3)
  - `crates/aiy-privacy/src/rag/indexer.rs` (3)
  - `crates/aiy-privacy/src/rag/chunk.rs` (2)
  - `crates/aiy-privacy/src/verification/engine.rs` (1)
- At least one ignored test currently **fails** when run:
  - `cargo test -p aiy-privacy rag::chunk::tests::test_hybrid_with_rust_code -- --ignored` fails (missing `ChunkType::TypeDefinition`)

## Non‑Negotiable Directives (Safety + Policy)
1. **No process killing by pattern**: do **not** use `pkill -f`, `killall`, or `pgrep | xargs kill`.
   - Use `timeout` to bound test runs.
   - If something truly hangs, show the exact PID list first and wait for explicit approval.
2. **No new `#[ignore]` in unit tests**.
   - Allowed ignores: only **manual** integration tests (real Ollama daemon, etc.), and they must live under `crates/*/tests/` and be clearly labeled.
3. **National security posture** remains: cloud planners never see code/paths/symbols/diffs/stack traces/deps; local only.
   - Keep “non‑US models removed” policy intact. “Custom” model variant is allowed but must not be recommended/default in privacy mode.
4. **Offline-safe tests**: no network dependency and no port binding in tests (mock everything).

---

## Plan (Tasks + Commits)

### Task 0 — Repo Hygiene & Phase 4 Artifact Quarantine (Commit 10 stays blocked)
**Goal:** ensure the repo can be edited by the normal user and keep Phase 4 artifacts from muddying RAG fixes.

- Fix ownership/permissions for root-owned repo files (these are currently `600 root:root` and will break normal editing):
  - `.github/workflows/*`
  - `CLAUDE.md`
  - `crates/aiy-privacy/src/rag/test_support.rs`
  - `crates/aiy-privacy/tests/integration_tests.rs`
- Either:
  - **A (preferred):** delete untracked Phase 4 artifacts for now and recreate later as normal user, OR
  - **B:** `chown/chmod` them to the repo user immediately and keep them untracked until Phase 4.

**Validation**
- `git status --porcelain=v1` reflects the intended untracked/modified set.
- No new files created via `sudo tee` going forward.

**Commit**
- None yet (unless you choose to add `CLAUDE.md` now as a policy doc; if so, commit it alone).

---

### Task 1 — Fix Chunking Semantic Parser (unblock `test_hybrid_with_rust_code`) (Commit A)
**Root cause:** `parse_semantic_blocks()` treats `use` lines as `ChunkType::Imports` “blocks” but never ends them (no braces), so they swallow structs/impls and chunk typing becomes wrong.

**Required fix**
- In `crates/aiy-privacy/src/rag/chunk.rs`, make `Imports` blocks **terminate** without brace tracking:
  - Group consecutive `use ...;` lines as one `Imports` block, and end on first non-`use` line (or blank).
  - Do not set `in_block = true` for imports, or end the imports block immediately.

**Remove ignore**
- Remove `#[ignore]` from `rag::chunk::tests::test_hybrid_with_rust_code` once passing.
- Remove `#[ignore]` from `test_sliding_window_large_content` (it’s now 5k chars and should be fast). If it’s still slow, optimize `sliding_window_chunk()` until it runs quickly.

**Validation (use `timeout`, no kill commands)**
- `timeout 2m cargo test -p aiy-privacy rag::chunk::tests::test_hybrid_with_rust_code -- --nocapture`
- `timeout 2m cargo test -p aiy-privacy rag::chunk::tests::test_sliding_window_large_content -- --nocapture`

**Commit A message**
- `fix(rag): correct imports block parsing; unignore chunk tests`

---

### Task 2 — Fix Indexer Exclude Matching for TempDirs/Absolute Paths (Commit B)
**Root cause:** `IndexerConfig::is_excluded()` matches glob patterns against absolute paths/components incorrectly; patterns like `target/**` never match `/tmp/.../target/...`.

**Required fix**
- In `crates/aiy-privacy/src/rag/indexer.rs`, update `IndexerConfig::is_excluded()` to:
  - Normalize to forward slashes
  - Evaluate patterns against **all suffixes** of the path (components joined by `/`), so `target/**` matches `target/debug.rs` regardless of absolute prefix.
  - Keep it simple and deterministic; no extra deps.

**Remove ignores**
- Unignore and fix to pass:
  - `test_index_directory`
  - `test_preview_index`
  - `test_hidden_directories_skipped`

**Validation**
- `timeout 2m cargo test -p aiy-privacy rag::indexer::tests::test_index_directory -- --nocapture`
- `timeout 2m cargo test -p aiy-privacy rag::indexer::tests::test_preview_index -- --nocapture`
- `timeout 2m cargo test -p aiy-privacy rag::indexer::tests::test_hidden_directories_skipped -- --nocapture`

**Commit B message**
- `fix(rag): make exclude globs match absolute paths; unignore indexer tests`

---

### Task 3 — Deterministic RAG Tests via `TestEmbedder` (Commit C)
**Root cause:** current unit tests use “deterministic embeddings” that are random-ish and not correlated to token overlap, so results can be empty under `min_similarity` and tests become flaky; this led to ignores.

**Required fix**
- Keep `TestEmbedder` in `crates/aiy-privacy/src/rag/test_support.rs`, but update its embedding algorithm to be **token-overlap-friendly**:
  - hashed bag-of-words (hash each token -> bucket -> add signed count), then normalize.
  - This makes cosine similarity meaningful for queries like “add two numbers” against code containing “add”.
- Rewire unit tests to use `TestEmbedder` instead of `LocalEmbedder + MockEmbeddingTransport`:
  - `crates/aiy-privacy/src/rag/query.rs` tests
  - `crates/aiy-privacy/src/rag/mod.rs` tests

**Remove ignores**
- Remove all `#[ignore]` in:
  - `crates/aiy-privacy/src/rag/query.rs`
  - `crates/aiy-privacy/src/rag/mod.rs`

**Validation**
- `timeout 2m cargo test -p aiy-privacy rag::query::tests::test_basic_query -- --nocapture`
- `timeout 2m cargo test -p aiy-privacy rag::tests::test_query_after_indexing -- --nocapture`
- `timeout 2m cargo test -p aiy-privacy rag::tests::test_multi_query -- --nocapture`
- `timeout 5m cargo test -p aiy-privacy rag:: -- --test-threads=1`

**Commit C message**
- `test(rag): use token-overlap TestEmbedder; unignore query tests`

---

### Task 4 — Remove the Last Unit-Test Ignore + Fix All Warnings (Commit D)
**Goal:** `cargo clippy --workspace --all-targets -- -D warnings` passes.

**Required changes**
- Handle the remaining unit-test ignore in `crates/aiy-privacy/src/verification/engine.rs:626`:
  - Move this to an integration test under `crates/aiy-privacy/tests/` and keep it `#[ignore]` as **manual**, OR refactor to be mock-based and non-ignored.
  - End state requirement: `rg -n "#\\[ignore\\]" crates/aiy-privacy/src` returns **0**.
- Remove known warnings observed in current build:
  - unused imports (e.g., `Hash`, `TestStage`, `DiagnosticSeverity`, `FailureType`)
  - dead code (`run_stage_with_repairs`, `mock_repair_response`, `build_args`)
  - useless comparisons (`preview.file_count >= 0`, `result.chunks.len() >= 0`)
  - unused imports in `crates/aiy-privacy/tests/*`

**Validation**
- `timeout 10m cargo test -p aiy-privacy -- --test-threads=1`
- `timeout 10m cargo test -p aiy-adapter-ollama`
- `timeout 15m cargo clippy --workspace --all-targets -- -D warnings`

**Commit D message**
- `chore: remove ignores from src; clippy clean under -D warnings`

---

### Task 5 — Phase 4 (Only After Tasks 1–4 Are Green) (Commit E + Commit F)
**Commit E:** finalize offline-safe integration tests (mock cloud + mock ollama; no ports; no network).
**Commit F:** add `.github/workflows/privacy-mode.yml` (and create `.github/workflows/` explicitly) if repo policy requires CI gating.

**Validation (end-to-end)**
- `cargo fmt --all -- --check`
- `cargo test --workspace --offline`
- `cargo clippy --workspace --all-targets -- -D warnings`

---

## Acceptance Criteria (“100% Approved”)
1. `cargo test -p aiy-privacy -- --test-threads=1` => **0 failed**, **0 ignored** for unit tests (integration tests may be ignored only if clearly labeled/manual and placed in `crates/aiy-privacy/tests/`).
2. `rg -n "#\\[ignore\\]" crates/aiy-privacy/src` => **no matches**.
3. `cargo clippy --workspace --all-targets -- -D warnings` passes.
4. No non‑US models are present as first-class options in `crates/aiy-adapter-ollama/src/models.rs` (only US models + `Custom`).
5. Working tree clean after final commits: `git status --porcelain=v1` empty.

## Proof Required in Final Report
- Paste:
  - `rg -n "#\\[ignore\\]" crates/aiy-privacy/src || true`
  - `cargo test -p aiy-privacy -- --test-threads=1` summary line
  - `cargo clippy --workspace --all-targets -- -D warnings` result
  - `git status --porcelain=v1`

## Process Notes (to avoid repeating prior failures)
- Use `timeout` for long tests; do not attempt to kill by pattern.
- Make changes incrementally; keep tests green per commit.
- If a test hangs, report the exact command and the exact PID list, then wait for instruction.

