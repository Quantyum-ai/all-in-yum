# PRP: GPT-5 Pro Review — Phase 1 Foundation (CLI + Config + SystemKeychain)

**Project**: all-in-yum
**Repository**: https://github.com/Quantyum-ai/all-in-yum
**Branch**: `phase1-foundation`
**Target Commit**: `babed24e5eb447f1feaf37a0166622e76a1a61ff`
**Review Mode**: Read-only verification
**Hard Constraints**: Offline-only (`--offline`), no network, no writes to real user paths

---

## Review Setup

### 0) Pin the Review Target

```bash
cd /home/aip0rt/Desktop/all-in-yum
git checkout babed24e5eb447f1feaf37a0166622e76a1a61ff
git rev-parse HEAD
git show -1 --name-status
```

**Expected**:
- HEAD matches `babed24e5eb447f1feaf37a0166622e76a1a61ff`
- Diff includes:
  - New `crates/aiy-cli/` crate
  - New `crates/aiy-core/src/config/` module
  - Modified `crates/aiy-core/src/security/credential_manager.rs` (SystemKeychain fixes)

---

## 1) Build/Test/Lint Verification (Offline Only)

**CRITICAL**: All commands must use `--offline` flag.

```bash
# Full test suite
cargo test --workspace --offline 2>&1 | tee test-results.txt

# Lint check
cargo clippy --workspace --all-targets --offline -- -D warnings 2>&1 | tee clippy-results.txt

# Build verification
cargo build --workspace --offline 2>&1 | tee build-results.txt
```

**Expected Results**:

| Command | Expected Output |
|---------|-----------------|
| `cargo test` | **76 unit/integration tests passed** (excludes 5 doc-tests, 1 ignored) |
| `cargo clippy` | **0 warnings, 0 errors** |
| `cargo build` | **Success** (dev profile) |

**Test Breakdown**:
- aiy-core: 56 tests (47 original + 2 keychain + 7 config)
- aiy-adapter-grok: 19 tests (12 unit + 7 integration)
- aiy-cli: 1 test (key redaction)
- **Total**: 76 unit/integration tests

---

## 2) Secret Hygiene & Repository Safety

### 2.1 No Committed Secret Files

```bash
git ls-files | rg "\.(enc|salt|env)$" || echo "✅ No secret files"
```

**Expected**: No output (exit code 1) or "✅ No secret files"

### 2.2 No Hardcoded API Keys

```bash
rg -n "sk-[A-Za-z0-9]{20,}|sk-ant-[A-Za-z0-9_-]{20,}|AKIA[0-9A-Z]{16}|gh[pousr]_[A-Za-z0-9]{36}" \
   --type rust \
   crates/ \
   || echo "✅ No API keys found"
```

**Expected**: "✅ No API keys found"

**Exception**: Test fixtures with obviously fake keys like `"test-api-key"`, `"xai-secret-key"` in test code are acceptable.

### 2.3 .gitignore Coverage

```bash
rg -n "\\.enc|\\.salt|\\.env|credentials" .gitignore
```

**Expected**: All sensitive patterns present

---

## 3) Hard Requirement: No Environment Variable Credentials

**CRITICAL**: No runtime environment variable reads for credentials.

```bash
# Check for env var credential sourcing (exclude compile-time macros)
rg -n "std::env::var|process\.env" \
   crates/aiy-cli crates/aiy-core crates/aiy-adapter-grok \
   --type rust \
   | grep -v 'env!("CARGO' \
   || echo "✅ No env var credential sourcing"
```

**Expected**: "✅ No env var credential sourcing"

**Allowed**: Compile-time macros like `env!("CARGO_PKG_VERSION")` for version strings

---

## 4) Phase 1 Foundation Code Review

### 4.1 CLI Implementation (crates/aiy-cli/)

**Files to Review**:
- `crates/aiy-cli/src/main.rs` (CLI structure, clap parsing)
- `crates/aiy-cli/src/commands/credentials.rs` (credential operations)
- `crates/aiy-cli/src/commands/version.rs` (version display)

**Verification Points**:

| Requirement | Verification Method |
|-------------|---------------------|
| Version command works | `cargo run -p aiy-cli -- version` → prints "aiy 0.1.0" |
| Uses CredentialManager | `rg -n "CredentialManager" crates/aiy-cli/src/commands/credentials.rs` |
| Secure password input | `rg -n "rpassword::prompt_password" crates/aiy-cli/src/commands/credentials.rs` |
| API key redaction | Check `redact_api_key()` function (should show only first/last 4 chars) |
| Backend selection | Tries SystemKeychain → EncryptedFile fallback |

**Safe Smoke Test** (no user input required):
```bash
cargo build --package aiy-cli --offline
./target/debug/aiy version
./target/debug/aiy --help
```

**⚠️ WARNING**: Do NOT run `aiy credentials` commands during review as they will:
- Write to real OS keychain (macOS Keychain, Windows Credential Manager)
- Write to `~/.config/all-in-yum/credentials.enc`
- Prompt for master password

Instead, verify behavior through code review and unit tests.

### 4.2 Config System (crates/aiy-core/src/config/)

**Files to Review**:
- `crates/aiy-core/src/config/mod.rs` (module exports)
- `crates/aiy-core/src/config/pipeline.rs` (PipelineConfig implementation)

**Verification Points**:

```bash
# Verify config struct fields
rg -n "pub struct PipelineConfig" -A 10 crates/aiy-core/src/config/pipeline.rs

# Verify no secret fields
rg -n "api_key|password|secret|token" crates/aiy-core/src/config/pipeline.rs

# Check config path resolution
rg -n "config_dir|all-in-yum/config.toml" crates/aiy-core/src/config/pipeline.rs

# Verify tests guard against secrets
rg -n "test_config_contains_no_secrets" crates/aiy-core/src/config/pipeline.rs
```

**Expected**:
- [ ] `PipelineConfig` contains only non-secret settings
- [ ] Fields: `enabled_agents`, `default_models`, `base_urls`, `timeouts`, `credential_backend`
- [ ] Config path: `~/.config/all-in-yum/config.toml`
- [ ] Test explicitly verifies config contains no secrets

### 4.3 SystemKeychain Backend (crates/aiy-core/src/security/credential_manager.rs)

**Critical Changes to Verify**:

#### No unlock() Required

```bash
# Find unlock() implementation for SystemKeychain
rg -n -A 5 "fn unlock.*password.*str.*Result" crates/aiy-core/src/security/credential_manager.rs \
   | grep -A 5 "SystemKeychain"
```

**Expected**: Early return `Ok(())` for SystemKeychain (line ~257-261)

#### Direct Keyring Access

```bash
# Verify get_key() for SystemKeychain
rg -n -B 2 -A 8 "CredentialBackend::SystemKeychain =>" crates/aiy-core/src/security/credential_manager.rs \
   | grep -A 8 "get_key"

# Verify store_key() for SystemKeychain
rg -n -B 2 -A 8 "CredentialBackend::SystemKeychain =>" crates/aiy-core/src/security/credential_manager.rs \
   | grep -A 8 "store_key"
```

**Expected**:
- `get_key()`: Uses `entry.get_password()` (NOT `credentials_cache`)
- `store_key()`: Uses `entry.set_password()`

#### Provider Index Mechanism

```bash
# Find __providers__ constant and usage
rg -n "__providers__|KEYCHAIN_PROVIDER_INDEX_USER" crates/aiy-core/src/security/credential_manager.rs
```

**Expected**:
- Constant defined (line ~45)
- Used in `list_providers()`, `keychain_add_provider()`, `keychain_remove_provider()`
- Stored as JSON array in keychain

#### New Tests

```bash
# Find the 2 new keychain tests
rg -n "test_system_keychain" crates/aiy-core/src/security/credential_manager.rs
```

**Expected**: 2 tests found:
1. `test_system_keychain_store_get_list_delete_without_unlock` (~line 1060)
2. `test_system_keychain_persists_across_manager_instances` (~line 1085)

**Verify Tests Use In-Memory Backend**:
```bash
rg -n "InMemoryCredential|install_in_memory_keyring" crates/aiy-core/src/security/credential_manager.rs
```

**Expected**: Test infrastructure present (lines ~977-1057), tests don't touch real keychain

---

## 5) Documentation Consistency

### 5.1 Codex Verification PRP

**File**: `docs/prp-phase1-foundation-codex-verification.md`

```bash
rg -n "76 tests|SystemKeychain|__providers__|CLI.*credential" docs/prp-phase1-foundation-codex-verification.md
```

**Verify**:
- [ ] Documents 76 tests
- [ ] Describes SystemKeychain fixes
- [ ] Describes CLI and config system

### 5.2 Reference Documentation

**File**: `docs/references.md`

```bash
rg -n "reference-only|DO NOT.*submodule" docs/references.md
```

**Verify**:
- [ ] grok-cli marked as reference-only
- [ ] Rules against submodule inclusion

---

## 6) Dependency Verification

**New Dependencies Expected**:

```bash
rg -n "^clap =|^rpassword =|^toml =" Cargo.toml
```

**Expected in workspace dependencies**:
- `clap = { version = "4", features = ["derive"] }`
- `rpassword = "7"`
- `toml = "0.8"`

**No Unexpected Dependencies**:
```bash
# Check if any HTTP clients were added
rg -n "reqwest|hyper|ureq|surf" Cargo.toml crates/*/Cargo.toml || echo "✅ No HTTP deps"
```

**Expected**: "✅ No HTTP deps" (Stage B deferred)

---

## 7) Regression Check

**Verify Prior Commits Unchanged**:

```bash
# Verify Phase 0 security tests still pass
cargo test --package aiy-core --lib -- security::credential_manager --offline
cargo test --package aiy-core --lib -- security::sanitization --offline

# Verify Grok adapter still works
cargo test --package aiy-adapter-grok --offline
```

**Expected**: All tests from prior phases still passing

---

## 8) Deliverable Format

### 8.1 Pass/Fail Checklist

| Section | Check | Status | Notes |
|---------|-------|--------|-------|
| **0. Setup** | Commit matches babed24 | ⏳ | |
| **1. Build** | `cargo build --workspace --offline` | ⏳ | |
| **1. Tests** | 76 tests passing | ⏳ | |
| **1. Clippy** | 0 warnings | ⏳ | |
| **2. Secrets** | No .enc/.salt/.env files | ⏳ | |
| **2. Secrets** | No hardcoded keys | ⏳ | |
| **2. Secrets** | .gitignore complete | ⏳ | |
| **3. Env Vars** | No credential env reads | ⏳ | |
| **4.1 CLI** | Version command works | ⏳ | |
| **4.1 CLI** | Uses CredentialManager | ⏳ | |
| **4.1 CLI** | Secure input (rpassword) | ⏳ | |
| **4.1 CLI** | Key redaction implemented | ⏳ | |
| **4.2 Config** | TOML load/save works | ⏳ | |
| **4.2 Config** | No secrets in config | ⏳ | |
| **4.2 Config** | Config path correct | ⏳ | |
| **4.3 Keychain** | No unlock() required | ⏳ | |
| **4.3 Keychain** | Direct keyring access | ⏳ | |
| **4.3 Keychain** | Provider index (__providers__) | ⏳ | |
| **4.3 Keychain** | 2 new tests present | ⏳ | |
| **4.3 Keychain** | Tests use in-memory backend | ⏳ | |
| **5. Docs** | PRPs updated correctly | ⏳ | |
| **6. Deps** | Only expected deps added | ⏳ | |
| **7. Regression** | Prior tests still pass | ⏳ | |

### 8.2 Risk Assessment

Provide assessment in this format:

```markdown
## Risk Assessment

### Security: [PASS | WARN | FAIL]
- [Finding 1 with severity]
- [Finding 2 with severity]

### Correctness: [PASS | WARN | FAIL]
- [Finding 1]

### Integration: [PASS | WARN | FAIL]
- [Finding 1]

### Maintenance: [PASS | WARN | FAIL]
- [Finding 1]

### Overall Risk Level: [LOW | MEDIUM | HIGH | CRITICAL]
```

### 8.3 Issues Found (If Any)

```markdown
## Issues

### Issue #1: [Severity: LOW|MEDIUM|HIGH|CRITICAL] [Category]
- **File**: `path/to/file.rs:line`
- **Problem**: Detailed description
- **Impact**: What could go wrong
- **Recommended Fix**: Specific code change
- **Blocking**: [YES | NO]
```

**DO NOT** apply fixes - this is read-only review only.

### 8.4 Discrepancies vs Claims

Report any differences between:
- Commit message claims
- PRP documentation claims
- Actual implementation

---

## Known Limitations (Expected, Not Issues)

These are intentional design decisions, not bugs:

| Limitation | Reason |
|------------|--------|
| Stage B (real HTTP) deferred | Explicit design - Stage A uses mock transport |
| CLI writes to real paths | No `--test-mode` flag implemented yet |
| No GPG signing for keychain | Out of scope for Phase 1 |
| Windows-specific keychain untested | Development on Linux/macOS |

---

## Quick Verification Script

```bash
#!/bin/bash
set -e

echo "=== Checkout Target ==="
git checkout babed24e5eb447f1feaf37a0166622e76a1a61ff
ACTUAL_SHA=$(git rev-parse HEAD)
if [ "$ACTUAL_SHA" != "babed24e5eb447f1feaf37a0166622e76a1a61ff" ]; then
    echo "❌ SHA mismatch: $ACTUAL_SHA"
    exit 1
fi
echo "✅ Commit verified: babed24"

echo ""
echo "=== Build ==="
cargo build --workspace --offline 2>&1 | tail -1

echo ""
echo "=== Tests ==="
TEST_COUNT=$(cargo test --workspace --offline 2>&1 | grep "test result: ok" | grep -o "[0-9]* passed" | awk '{sum+=$1} END {print sum}')
echo "Tests passed: $TEST_COUNT (expected: 76)"
if [ "$TEST_COUNT" != "76" ]; then
    echo "⚠️ Test count mismatch"
fi

echo ""
echo "=== Clippy ==="
cargo clippy --workspace --all-targets --offline -- -D warnings 2>&1 | tail -1

echo ""
echo "=== Secret Scan ==="
SECRET_FILES=$(git ls-files | rg "\.(enc|salt|env)$" | wc -l)
if [ "$SECRET_FILES" -eq 0 ]; then
    echo "✅ No secret files committed"
else
    echo "❌ Found $SECRET_FILES secret files"
fi

echo ""
echo "=== Env Var Check ==="
ENV_VARS=$(rg -n "std::env::var\|process\.env" crates/ --type rust | grep -v 'env!("CARGO' | wc -l)
if [ "$ENV_VARS" -eq 0 ]; then
    echo "✅ No env var credential sourcing"
else
    echo "⚠️ Found $ENV_VARS potential env var reads"
fi

echo ""
echo "=== SystemKeychain Verification ==="
UNLOCK_SKIP=$(rg -n "SystemKeychain.*return Ok" crates/aiy-core/src/security/credential_manager.rs | wc -l)
if [ "$UNLOCK_SKIP" -gt 0 ]; then
    echo "✅ SystemKeychain unlock() skips password"
else
    echo "❌ SystemKeychain unlock() behavior unclear"
fi

PROVIDER_INDEX=$(rg -n "__providers__" crates/aiy-core/src/security/credential_manager.rs | wc -l)
if [ "$PROVIDER_INDEX" -gt 0 ]; then
    echo "✅ Provider index mechanism present"
else
    echo "❌ Provider index not found"
fi

echo ""
echo "=== Verification Complete ==="
```

**Save as**: `verify-phase1.sh`
**Run**: `chmod +x verify-phase1.sh && ./verify-phase1.sh`

---

## Final Deliverable

Provide a complete review report with:

1. **Executive Summary** (2-3 sentences)
2. **Pass/Fail Checklist** (all 22 items above)
3. **Risk Assessment** (Security/Correctness/Integration/Maintenance)
4. **Issues Found** (if any, with file:line and recommended fixes)
5. **Discrepancies** (claims vs reality)
6. **Recommendation**: [APPROVE | REQUEST_CHANGES | REJECT]

---

**Document Version**: 2.0 (Improved with explicit SHA, test count clarification, warnings)
**Classification**: Internal - Code Review
**Distribution**: GPT-5 Pro Team (OpenAI), Project Maintainers
