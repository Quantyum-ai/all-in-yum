# PRP: Privacy Mode 2.5.3b — Stabilize to 100% GREEN (No Ignored Unit Tests)

## Repo / Branch
- Repo: `/home/aip0rt/Desktop/all-in-yum`
- Branch: `feat/privacy-mode-253b`
- Reference docs:
  - `docs/prp-253b-privacy-mode-stabilization.md` (baseline execution plan)
  - `docs/STATUS-privacy-mode-253b.md` (handoff context; treat as read-only if permissions block edits)

## Objective (Hard Requirement)
Achieve **100% GREEN** with national-security posture preserved:
- `cargo test -p aiy-privacy -- --test-threads=1` → **0 failed, 0 ignored** for unit tests
- `rg -n "#\\[ignore\\]" crates/aiy-privacy/src` → **no matches**
- `cargo clippy --workspace --all-targets -- -D warnings` → **PASS**
- Cloud planners never see code/paths/symbols/diffs/stack traces/deps; local executor only.

## Non‑Negotiables (Safety + Policy)
1. **No destructive deletes**: do not run `rm -rf` anywhere in this repo (including `.github/workflows/`).
2. **No pattern kill**: do not run `pkill -f`, `killall`, or `pgrep | xargs kill`.
   - Use `timeout` for long-running tests.
   - If something hangs, list exact PIDs and request approval before killing a specific PID.
3. **No new `#[ignore]` in unit tests**. Remove existing ignores by fixing root causes.
4. Phase 4 artifacts (`.github/workflows/`, `crates/aiy-privacy/tests/integration_tests.rs`) are **quarantined** until unit tests are green.
5. Maintain national-security model policy: `crates/aiy-adapter-ollama/src/models.rs` must keep US-based models as first-class options; non‑US models must not be recommended/default.

## Current Known Debt (Must Re-Verify)
- 12 `#[ignore]` markers exist today (mostly RAG + one verification unit test).
- At least one ignored test currently fails when run:
  - `cargo test -p aiy-privacy rag::chunk::tests::test_hybrid_with_rust_code -- --ignored` fails (imports parsing swallows structs/impls).
- Some repo files are root-owned (`600 root:root`) and block normal edits:
  - `.github/workflows/*`, `docs/STATUS-privacy-mode-253b.md`, `CLAUDE.md`,
    `crates/aiy-privacy/src/rag/test_support.rs`, `crates/aiy-privacy/tests/integration_tests.rs`
  - Do not create more root-owned files via `sudo tee`.

## Team Structure (Parallel but Non-Conflicting)
- **Agent A**: Fix chunk parsing + remove chunk ignores (`crates/aiy-privacy/src/rag/chunk.rs`)
- **Agent B**: Fix indexer exclude matching + remove indexer ignores (`crates/aiy-privacy/src/rag/indexer.rs`)
- **Agent C**: Wire `TestEmbedder` into RAG unit tests + remove query/mod ignores
  (`crates/aiy-privacy/src/rag/test_support.rs`, `crates/aiy-privacy/src/rag/query.rs`,
  `crates/aiy-privacy/src/rag/mod.rs`)
- **Lead agent**: Integrate, remove remaining ignore in `verification/engine.rs`, fix warnings, run full validation, prepare final commits.

## Execution Plan (Do in This Order)

### Step 0 — Preflight (No Changes)
Run and paste outputs:
- `git status --porcelain=v1`
- `rg -n "#\\[ignore\\]" crates/aiy-privacy`
- `timeout 5m cargo test -p aiy-privacy --lib | tail -40`

### Step 1 — Fix Chunk Semantic Parser (Agent A) (Commit 1)
Root cause: `parse_semantic_blocks()` treats `use` lines as a block type that never ends (no braces), so imports “swallow” struct/impl blocks.

Required:
- Update `parse_semantic_blocks()` in `crates/aiy-privacy/src/rag/chunk.rs`:
  - Treat `Imports` as a “non-brace” block: group consecutive `use ...;` lines and end on first non-`use`/blank line.
  - Ensure subsequent `struct`/`impl` blocks are detected as `TypeDefinition` / `ImplBlock`.
- Remove `#[ignore]` on:
  - `rag::chunk::tests::test_hybrid_with_rust_code`
  - `rag::chunk::tests::test_sliding_window_large_content` (optimize if still slow at 5k input)

Validate:
- `timeout 2m cargo test -p aiy-privacy rag::chunk::tests::test_hybrid_with_rust_code -- --nocapture`
- `timeout 2m cargo test -p aiy-privacy rag::chunk::tests::test_sliding_window_large_content -- --nocapture`

### Step 2 — Fix Indexer Exclude Matching for Absolute Paths (Agent B) (Commit 2)
Root cause: glob excludes like `target/**` don’t match tempdir absolute paths.

Required:
- In `IndexerConfig::is_excluded()` (`crates/aiy-privacy/src/rag/indexer.rs`), implement suffix/segment matching:
  - Normalize to forward slashes
  - Match patterns against all suffixes of the path (e.g., `target/debug.rs`, `src/utils.rs`) so excludes work regardless of absolute prefix.
- Remove `#[ignore]` for:
  - `test_index_directory`
  - `test_preview_index`
  - `test_hidden_directories_skipped`

Validate:
- `timeout 2m cargo test -p aiy-privacy rag::indexer::tests::test_index_directory -- --nocapture`
- `timeout 2m cargo test -p aiy-privacy rag::indexer::tests::test_preview_index -- --nocapture`
- `timeout 2m cargo test -p aiy-privacy rag::indexer::tests::test_hidden_directories_skipped -- --nocapture`

### Step 3 — Deterministic RAG Tests using `TestEmbedder` (Agent C) (Commit 3)
Goal: remove async mock complexity and make similarity meaningful.

Required:
- Update `crates/aiy-privacy/src/rag/test_support.rs` `TestEmbedder` to produce token-overlap-friendly embeddings:
  - hashed bag-of-words → normalize.
- Replace `LocalEmbedder + MockEmbeddingTransport` usage in unit tests with `TestEmbedder`:
  - `crates/aiy-privacy/src/rag/query.rs` tests
  - `crates/aiy-privacy/src/rag/mod.rs` tests
- Remove all `#[ignore]` in those files.

Validate:
- `timeout 2m cargo test -p aiy-privacy rag::query::tests::test_basic_query -- --nocapture`
- `timeout 2m cargo test -p aiy-privacy rag::tests::test_query_after_indexing -- --nocapture`
- `timeout 2m cargo test -p aiy-privacy rag::tests::test_multi_query -- --nocapture`

### Step 4 — Remove Remaining Ignore in `src/` + Fix Warnings (Lead) (Commit 4)
Required:
- Remove the ignored test in `crates/aiy-privacy/src/verification/engine.rs`:
  - Either refactor to be mock-based and non-ignored, OR move to `crates/aiy-privacy/tests/` as a **manual integration test** (allowed to be ignored there).
- Remove warnings seen previously:
  - unused imports, dead code, useless comparisons (`>= 0`), etc.
- Confirm US-based model policy remains in `crates/aiy-adapter-ollama/src/models.rs` (no DeepSeek/Qwen enums).

Validate:
- `rg -n "#\\[ignore\\]" crates/aiy-privacy/src || true` (must be empty)
- `timeout 15m cargo test -p aiy-privacy -- --test-threads=1`
- `timeout 15m cargo clippy --workspace --all-targets -- -D warnings`

### Step 5 — Only After GREEN: Revisit Phase 4 Artifacts (Deferred)
Do not touch `.github/workflows/` or integration tests until Steps 1–4 are green. When ready, integrate them safely (no `rm -rf`).

## Commit Rules
- One fix group per commit (4 commits total as above).
- After each commit, run the targeted tests for that commit.
- End with full validation commands and paste results.

## Proof Required in Final Response
Paste:
- `git log --oneline -n 6`
- `rg -n "#\\[ignore\\]" crates/aiy-privacy/src || true`
- `timeout 15m cargo test -p aiy-privacy -- --test-threads=1` summary line
- `timeout 15m cargo clippy --workspace --all-targets -- -D warnings` result
- `git status --porcelain=v1` (must be clean)

