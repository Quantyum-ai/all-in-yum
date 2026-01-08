# PRP: Phase 0 Security Foundations Verification

**Project**: all-in-yum
**Repository**: https://github.com/Quantyum-ai/all-in-yum
**Target Reviewer**: Codex Team
**Date**: 2026-01-08
**Status**: Ready for Verification

---

## Executive Summary

Phase 0 implements the security foundations for the All-in-Yum multi-agent consensus pipeline. This PRP requests verification of cryptographic implementations, prompt injection defenses, and security test coverage before proceeding to Phase 1.

---

## Deliverables Checklist

| Deliverable | Status | Location | Lines |
|------------|--------|----------|-------|
| Cargo Workspace | ✅ | `Cargo.toml` | 1-53 |
| Secure Credential Manager | ✅ | `crates/aiy-core/src/security/credential_manager.rs` | 1-851 |
| Prompt Injection Defenses | ✅ | `crates/aiy-core/src/security/sanitization.rs` | 1-855 |
| Security Test Suite | ✅ | Embedded in above files | 47 tests |
| Memory Zeroization | ✅ | `MasterKey` with `ZeroizeOnDrop` | Lines 105-129 |
| File Permissions | ✅ | 600 on credential files (Unix) | Lines 546-550 |

---

## Verification Tasks for Codex Team

### 1. Cryptographic Implementation Review

#### 1.1 AES-256-GCM Nonce Handling (CRITICAL)
**File**: `crates/aiy-core/src/security/credential_manager.rs:393-415`

**Verify**:
- [ ] Each `encrypt()` call generates a fresh 12-byte nonce via `OsRng`
- [ ] Nonce is prepended to ciphertext (not stored separately)
- [ ] Ciphertext format: `[12-byte nonce][encrypted data][16-byte auth tag]`
- [ ] No nonce reuse is possible (randomness source is cryptographically secure)

**Expected Constants**:
```rust
const NONCE_SIZE: usize = 12;    // Line 36
const TAG_SIZE: usize = 16;       // Line 39
const MIN_CIPHERTEXT_SIZE: usize = NONCE_SIZE + TAG_SIZE; // 28 bytes
```

**Test to Run**:
```bash
cargo test test_encrypt_produces_different_ciphertexts -- --nocapture
```

#### 1.2 Argon2id Key Derivation
**File**: `crates/aiy-core/src/security/credential_manager.rs:248-276`

**Verify Parameters**:
- [ ] Algorithm: `Argon2id` (Line 252)
- [ ] Memory Cost: 65536 KiB (64 MiB) - Line 44
- [ ] Time Cost: 3 iterations - Line 47
- [ ] Parallelism: 4 - Line 50
- [ ] Output Length: 32 bytes (256-bit key) - Line 259

**Security Rationale**: These parameters exceed OWASP recommendations for Argon2id (19 MiB memory minimum).

#### 1.3 Memory Zeroization
**File**: `crates/aiy-core/src/security/credential_manager.rs:105-129`

**Verify**:
- [ ] `MasterKey` struct derives `Zeroize` and `ZeroizeOnDrop`
- [ ] Key material is `[u8; 32]` (fixed-size array, stack-allocated)
- [ ] `CredentialManager::lock()` drops the `MasterKey`, triggering zeroization
- [ ] `CredentialManager` implements `Drop` trait calling `lock()` (Lines 655-660)

---

### 2. Prompt Injection Defense Review

#### 2.1 Input Sanitization
**File**: `crates/aiy-core/src/security/sanitization.rs:179-218`

**Verify Sanitization Layers**:
| Layer | Purpose | Verification |
|-------|---------|--------------|
| Unicode NFKC | Prevent homograph attacks | Line 182 |
| Size Truncation | 100KB limit | Lines 185-196 |
| Pattern Escape | 14 injection patterns | Lines 33-51 |
| XML Tag Escape | `<system>` → `[LT]system[GT]` | Lines 55-57, 204-208 |
| API Key Redaction | 12 patterns (OpenAI, Anthropic, AWS, GitHub) | Lines 60-84 |

**Test Commands**:
```bash
cargo test test_escapes_ignore_instructions -- --nocapture
cargo test test_redacts_openai_api_keys -- --nocapture
cargo test test_escapes_xml_system_tags -- --nocapture
```

#### 2.2 Injection Patterns Covered
**File**: `crates/aiy-core/src/security/sanitization.rs:33-51`

| Pattern | Regex |
|---------|-------|
| Ignore instructions | `ignore\s+(all\s+)?previous\s+instructions?` |
| Forget commands | `forget\s+(all\s+)?(previous\s+)?everything` |
| Role hijacking | `you\s+are\s+now\s+` |
| Disregard prior | `disregard\s+(all\s+)?prior\s+` |
| New instructions | `new\s+instructions?\s*:` |
| System override | `system\s*:\s*you\s+are` |
| Override previous | `override\s+previous` |
| Act as | `act\s+as\s+(if\s+you\s+are\|a)\s+` |
| Pretend to be | `pretend\s+(to\s+be\|you\s+are)\s+` |
| From now on | `from\s+now\s+on\s+` |
| Must obey | `you\s+must\s+obey` |
| Don't follow | `do\s+not\s+follow\s+` |
| Jailbreak | `jailbreak` |
| DAN mode | `dan\s+mode` |

#### 2.3 Output Validation
**File**: `crates/aiy-core/src/security/sanitization.rs:350-367`

**Verify Detection of**:
- [ ] System tags in output (`</?system[^>]*>`)
- [ ] Execute commands (`execute\s*:\s*`)
- [ ] API key leaks (`sk-[a-zA-Z0-9]{20,}`)
- [ ] Bearer tokens (`bearer\s+[a-zA-Z0-9_.+-]{20,}`)
- [ ] Shell command substitution (`\$\([^)]+\)`)
- [ ] Backtick execution (`` `[^`]+` ``)
- [ ] Eval patterns (`eval\s*\([^)]+\)`)

---

### 3. Schema Enforcement Review

**File**: `crates/aiy-core/src/security/sanitization.rs:399-463`

**Verify Required Fields**:
```rust
const REQUIRED_REVIEW_FIELDS: &[&str] = &[
    "verdict",      // "approve" | "request_changes" | "reject"
    "confidence",   // 0.0 to 1.0
    "issues",       // Array
    "suggestions",  // Array
    "sign_off",     // String
    "reasoning",    // String
];
```

**Validation Rules**:
- [ ] Response must be JSON object (Line 400-402)
- [ ] All required fields present (Lines 404-417)
- [ ] Correct field types (Lines 420-425)
- [ ] Verdict in valid set (Lines 428-435)
- [ ] Confidence in 0.0-1.0 range (Lines 438-448)

---

### 4. File Permission Security (Unix)

**File**: `crates/aiy-core/src/security/credential_manager.rs:546-550`

**Verify**:
```rust
#[cfg(unix)]
{
    let mut perms = fs::metadata(path)?.permissions();
    perms.set_mode(0o600);  // rw------- (owner only)
    fs::set_permissions(path, perms)?;
}
```

**Test**:
```bash
cargo test test_file_permissions -- --nocapture
```

---

### 5. Test Coverage Analysis

**Run Full Test Suite**:
```bash
cargo test 2>&1 | tee test-results.txt
```

**Expected Output**:
```
test result: ok. 47 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

**Test Categories**:

| Category | Count | Key Tests |
|----------|-------|-----------|
| Crypto Roundtrip | 4 | `test_encrypt_decrypt_roundtrip` |
| Nonce Uniqueness | 1 | `test_encrypt_produces_different_ciphertexts` |
| Ciphertext Validation | 3 | `test_minimum_ciphertext_validation`, `test_tampered_ciphertext_fails` |
| Key Management | 4 | `test_unlock_lock_cycle`, `test_master_key_zeroization` |
| Credential Persistence | 2 | `test_store_and_retrieve_key`, `test_credential_persistence` |
| File Permissions | 1 | `test_file_permissions` |
| Injection Patterns | 10 | `test_escapes_*` |
| API Key Redaction | 5 | `test_redacts_*` |
| Output Validation | 7 | `test_rejects_*` |
| Schema Validation | 10 | `test_accepts_valid_schema`, `test_rejects_*` |

---

### 6. Dependency Audit

**File**: `Cargo.toml`

**Security-Critical Dependencies**:
| Crate | Version | Purpose | CVE Status |
|-------|---------|---------|------------|
| `aes-gcm` | 0.10 | AES-256-GCM encryption | ✅ No known CVEs |
| `argon2` | 0.5 | Password hashing | ✅ No known CVEs |
| `rand` | 0.8 | Cryptographic RNG | ✅ No known CVEs |
| `zeroize` | 1.7 | Memory clearing | ✅ No known CVEs |
| `keyring` | 2 | System keychain access | ✅ No known CVEs |

**Run Audit**:
```bash
cargo audit
```

---

## Crate Structure

```
crates/
├── aiy-core/
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs                    # Core exports
│       ├── types/
│       │   ├── mod.rs
│       │   └── error.rs              # CoreError enum
│       └── security/
│           ├── mod.rs                # Module exports
│           ├── credential_manager.rs # Secure credential storage (851 lines)
│           └── sanitization.rs       # Prompt injection defense (855 lines)
└── aiy-adapters/
    ├── Cargo.toml
    └── src/
        ├── lib.rs                    # Adapter exports
        └── traits.rs                 # AgentAdapter trait, Review types
```

---

## Known Limitations

1. **SecretManager Backend**: Not implemented (returns `todo!()`)
2. **System Keychain**: Basic implementation, no full enumeration
3. **Windows Permissions**: File permissions only enforced on Unix

---

## Acceptance Criteria

- [ ] All 47 tests pass
- [ ] No `cargo clippy` warnings
- [ ] `cargo audit` shows no vulnerabilities
- [ ] Code review confirms nonce randomization is correct
- [ ] Prompt injection patterns cover OWASP injection vectors

---

## Sign-off

**Claude Team**: Ready for Codex verification
**Codex Team**: _Pending review_
**Merge Approved**: _Pending_

---

## Next Steps (Post-Approval)

1. Initial commit to GitHub repository
2. Set up CI/CD with test and audit checks
3. Proceed to Phase 1: Agent Adapter Implementation
