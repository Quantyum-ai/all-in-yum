# PRP: Phase 1 Foundation — Codex Verification

**Project**: all-in-yum  
**Target Reviewer**: Codex Team  
**Date**: 2026-01-08  
**Review Mode**: Read-only verification (no edits unless explicitly requested)  
**Constraints**: Offline-only (`--offline`), no network access

---

## Executive Summary

This PRP asks Codex Team to verify the Phase 1 Foundation implementation reported by Claude Team, including:

- New `aiy-cli` crate (clap-based) with `version` and `credentials` commands
- New TOML-based `PipelineConfig` in `aiy-core` (`crates/aiy-core/src/config/`)
- Credential commands are implemented (status/set/get/delete) and use `aiy-core::CredentialManager`
- No environment-variable credential sourcing
- Redacted key display for `credentials get`
- Offline-safe workspace: `cargo test/clippy/build --workspace --offline` all pass
- Docs added/updated (references + GPT‑5 Pro review PRP)

---

## Review Target

Record the exact commit under review:
```bash
git rev-parse HEAD
git show --stat HEAD
```

---

## Claims to Verify

| Claim | Expected Result |
|---|---|
| Workspace members include `aiy-cli` | `Cargo.toml` lists `crates/aiy-cli` |
| Config system exists | `crates/aiy-core/src/config/{mod.rs,pipeline.rs}` |
| CLI crate exists | `crates/aiy-cli/src/main.rs` + commands modules |
| Credential commands functional | Uses `CredentialManager`; prompts securely; redacted output |
| No env-var credentials | No `std::env::var`/`process.env` in CLI/core/grok |
| Tests passing | `cargo test --workspace --offline` passes; **76** tests total (expected breakdown below) |
| Clippy clean | `cargo clippy --workspace --all-targets --offline -- -D warnings` passes |
| Offline build clean | `cargo build --workspace --offline` passes |
| Docs present | `docs/references.md`, `docs/prp-phase1-gpt5-pro-review.md`; README links references |

Expected test breakdown:
- `aiy-core`: **56** tests
- `aiy-adapter-grok`: **12** unit + **7** integration
- `aiy-cli`: **1** unit test (redaction)
- Total: **76** tests

---

## Verification Tasks

### 1) Workspace Wiring & File Layout

Verify members and dependencies:
```bash
rg -n '"crates/aiy-cli"|toml\\s*=|clap\\s*=|rpassword\\s*=' Cargo.toml
```

Verify expected files exist:
```bash
find crates/aiy-cli -maxdepth 3 -type f -print
find crates/aiy-core/src/config -maxdepth 2 -type f -print
```

Verify `aiy-core` exports config:
```bash
rg -n "pub mod config|pub use config::PipelineConfig" crates/aiy-core/src/lib.rs
```

Optional line-count sanity checks:
```bash
wc -l crates/aiy-cli/src/**/*.rs crates/aiy-core/src/config/*.rs
```

---

### 2) Offline Build / Tests / Clippy (Must Pass)

Run:
```bash
cargo test --workspace --offline
cargo clippy --workspace --all-targets --offline -- -D warnings
cargo build --workspace --offline
```

Verify:
- test counts match the expected breakdown (56 + 19 + 1)
- no network access is attempted

---

### 3) Secret Hygiene & Repo Safety

Verify no secret artifacts are tracked:
```bash
git ls-files | rg -n "\\.(enc|salt|env)$" || true
```

Scan for key-like strings (should be none outside benign test fixtures):
```bash
rg -n "sk-[A-Za-z0-9]{20,}|sk-ant-[A-Za-z0-9_-]{20,}|AKIA[0-9A-Z]{16}|gh[pousr]_[A-Za-z0-9]{36}" -S .
```

Verify `.gitignore` covers sensitive artifacts:
```bash
rg -n "\\*\\.enc|\\*\\.salt|\\.env" .gitignore
```

---

### 4) No Environment Variable Credential Sourcing (Hard Requirement)

Verify no env var reads exist in the new CLI/config and adapter code:
```bash
rg -n "std::env::var|std::env\\b|process\\.env|ENV\\b|getenv" crates/aiy-cli crates/aiy-core crates/aiy-adapter-grok -S
```

Expected: no matches (note: compile-time `env!("CARGO_PKG_VERSION")` in version output is acceptable; it is not runtime secret sourcing).

---

### 5) Config System Review (`PipelineConfig`)

Review:
- `crates/aiy-core/src/config/pipeline.rs`

Verify:
- TOML load/save works
- `config_path()` uses `dirs::config_dir()` and resolves to `all-in-yum/config.toml`
- config fields are non-sensitive (enabled agents, models, URLs, timeouts, backend preference)
- tests enforce “no secrets in config” (search for `api_key`, `password`, `token`, `sk-`, etc.)

Suggested spot-checks:
```bash
rg -n "struct PipelineConfig|credential_backend|config_path\\(|fn load\\(|fn save\\(" crates/aiy-core/src/config/pipeline.rs
rg -n "contains_no_secrets|api_key|password|token|sk-" crates/aiy-core/src/config/pipeline.rs
```

---

### 6) CLI Review (`aiy-cli`)

Review:
- `crates/aiy-cli/src/main.rs`
- `crates/aiy-cli/src/commands/version.rs`
- `crates/aiy-cli/src/commands/credentials.rs`

Verify:
- `aiy version` prints the Cargo package version.
- credential commands use `CredentialManager` and never print full keys.
- `credentials get` redacts keys (first 4 + last 4 only).
- secret input is via `rpassword` (no echo).
- backend selection: tries `SystemKeychain` when configured, falls back to encrypted file backend.

Non-interactive smoke test (no prompts):
```bash
cargo build --workspace --offline
./target/debug/aiy version
```

Note: `credentials set/get/delete/status` may require interactive password prompts (TTY). If verification environment cannot provide TTY input, validate correctness via code review + unit tests.

---

### 7) Backend Selection & Keychain Correctness (Must Be Explicitly Signed Off)

The CLI reports “SystemKeychain → EncryptedFile fallback”. Codex must verify whether the **SystemKeychain backend is functionally usable across separate CLI invocations**.

Audit `crates/aiy-core/src/security/credential_manager.rs`:
- For `CredentialBackend::SystemKeychain`, does `get_key()` read on-demand from the keychain (`keyring::Entry::get_password()`), not from `credentials_cache`?
- For `CredentialBackend::SystemKeychain`, does `list_providers()` use a keychain-stored provider index (username: `__providers__`), since enumeration is not portable?

If `SystemKeychain` only uses the in-memory cache (no on-demand read), document this as a discrepancy vs “fully functional” claims and recommend either:
- implement on-demand keychain retrieval in `get_key()` (via `keyring::Entry::get_password()`), and/or
- force CLI to default to encrypted-file backend until keychain backend is complete, and/or
- document the limitation prominently.

---

### 8) Docs Verification

Verify docs exist and are internally consistent:
```bash
ls -la docs | rg -n "references\\.md|prp-phase1-gpt5-pro-review\\.md" -S
rg -n "## References|docs/references\\.md" README.md
```

Verify `docs/prp-phase1-gpt5-pro-review.md` is coherent (pins a commit; read-only protocol; offline constraints).

---

## Deliverable (What Codex Team Should Return)

- A pass/fail checklist for each section above
- Any discrepancies vs Claude’s claims (especially keychain backend behavior)
- A short risk assessment (security + correctness)
- If something fails: minimal recommended fix list (do not patch unless explicitly requested)

---

## Acceptance Criteria

- Offline `cargo test/clippy/build` pass.
- No secrets committed; `.gitignore` patterns present.
- No env-var credential sourcing.
- Config contains no secrets and is TOML load/save tested.
- CLI redacts credentials on output and uses secure input.
- Keychain backend limitations (if present) are explicitly identified and decided (accept vs fix).
