# All-in-Yum (`aiy`)

Multi-agent AI consensus pipeline for verified development workflows.

## Overview

All-in-Yum orchestrates multiple AI agents (Claude, Codex, Gemini, Grok, local models) with consensus-based verification. All agents review artifacts until unanimous or supermajority approval is reached.

**Problem it solves:**
- Developers juggle multiple AI CLIs in separate terminals
- Context switching overhead between different AI assistants
- No verification mechanism for AI outputs
- Lost session state across conversations

**Solution:**
- Unified CLI managing all agents from one terminal
- Consensus pipeline (all agents must approve)
- Session persistence
- Cloud + local model support

## Phase 0: Security Foundations ✅

This phase implements critical security infrastructure:

| Feature | Status | Description |
|---------|--------|-------------|
| **Secure Credential Manager** | ✅ | AES-256-GCM encryption with random nonces |
| **Prompt Injection Defenses** | ✅ | Input sanitization + output validation |
| **Memory Zeroization** | ✅ | Automatic clearing of sensitive data |
| **File Permissions** | ✅ | 600 permissions on credential files |

### Security Fixes

1. **AES-GCM Nonce Randomization** - Each encryption operation generates a cryptographically random 12-byte nonce using `OsRng`. The nonce is prepended to the ciphertext.

2. **Prompt Injection Prevention** - Multi-layer defense:
   - Input sanitization (escape injection patterns)
   - Secure prompt construction with boundary markers
   - Output validation for suspicious patterns
   - Schema enforcement for agent responses

## Development

```bash
# Build
cargo build

# Run tests
cargo test

# Run clippy (no warnings allowed)
cargo clippy -- -D warnings

# Format code
cargo fmt
```

## Project Structure

```
all-in-yum/
├── Cargo.toml                      # Workspace root
├── crates/
│   ├── aiy-core/                   # Core library
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── types/
│   │       │   ├── mod.rs
│   │       │   └── error.rs
│   │       └── security/
│   │           ├── mod.rs
│   │           ├── credential_manager.rs
│   │           └── sanitization.rs
│   │
│   └── aiy-adapters/               # Agent adapters
│       └── src/
│           ├── lib.rs
│           └── traits.rs
│
└── tests/
    └── security/
        ├── crypto_tests.rs
        ├── injection_tests.rs
        └── credential_tests.rs
```

## Security

**DO NOT commit:**
- API keys or credentials
- `.enc` or `.salt` files
- `.env` files with secrets

Credentials are stored encrypted with:
- AES-256-GCM authenticated encryption
- Random nonce per encryption (12 bytes)
- Argon2 key derivation with per-user salt
- Optional system keychain integration

## License

MIT License - see [LICENSE](LICENSE)

## Contributing

Security-critical contributions require additional review. All changes must pass:
- `cargo test` - All tests passing
- `cargo clippy -- -D warnings` - No clippy warnings
- Security test suite - 14+ security tests

---

*Quantyum AI - Building secure AI development tools*
