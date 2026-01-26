# External References

This document lists external projects used as **reference material only**. These are not dependencies, submodules, or integrated code.

---

## grok-cli

**Status**: Reference-only (local development reference, no public link yet)

**Location**: `~/Desktop/grok-cli` (URL TBD)

**License**: No license file present (local development project)

### Purpose

grok-cli is a TypeScript/Node.js CLI for interacting with xAI's Grok API. It serves as:

- **UX reference** for terminal interface patterns
- **Architecture reference** for agent loop design
- **Prototyping tool** for rapid iteration

**all-in-yum is the production Rust implementation.** grok-cli is kept separate for prototyping and UX exploration.

### Hard Rules

| Rule | Rationale |
|------|-----------|
| **DO NOT** add grok-cli as a git submodule | Keeps repos independent; avoids version coupling |
| **DO NOT** add Node.js or TypeScript as a dependency | all-in-yum is pure Rust |
| **DO NOT** copy/paste grok-cli credential implementation | all-in-yum has its own secure credential system |
| **DO NOT** include any secrets or token examples | Security hygiene |

### What May Be Ported (Planned for Phase 4)

The following non-credential logic may be ported to Rust in a future phase (per `docs/mossy-dazzling-feather.md`):

| TypeScript Source | Planned Rust Destination | Description |
|-------------------|--------------------------|-------------|
| `src/client/grok-client.ts` | `crates/aiy-adapter-grok/src/client.rs` | xAI API client wrapper |
| `src/config/models.ts` | `crates/aiy-adapter-grok/src/models.rs` | Grok model configurations |
| `src/agent/grok-agent.ts` | `crates/aiy-adapter-grok/src/agent.rs` | Streaming agent loop |

**Note**: These are planned, not committed. Implementation details may change.

### What Will NOT Be Ported

| TypeScript Source | Reason |
|-------------------|--------|
| `src/credentials/*` | all-in-yum uses `crates/aiy-core` credential manager |
| `src/security/*` | all-in-yum has its own security layer |
| `src/ui/*` | Terminal UI will use Rust libraries (ratatui/crossterm) |
| `src/commands/*` | CLI commands are all-in-yum specific |

### Security Note

all-in-yum implements its own security layer with stronger guarantees than grok-cli:

| Feature | all-in-yum | grok-cli |
|---------|-----------|----------|
| Key Derivation | Argon2id (64 MiB, 3 iterations) | PBKDF2-SHA256 (600k iterations) |
| Encryption | AES-256-GCM with random nonces | AES-256-GCM |
| Memory Safety | `ZeroizeOnDrop` (compile-time) | JavaScript GC (no guarantees) |
| Prompt Injection | 14 patterns + XML escaping + API key redaction | Not implemented |
| File Permissions | 0o600 enforced on Unix | 0o600 enforced on Unix |

See:
- `crates/aiy-core/src/security/credential_manager.rs` - Secure credential storage
- `crates/aiy-core/src/security/sanitization.rs` - Prompt injection defenses

---

## Adding New References

When adding external references:

1. Document in this file with clear "reference-only" status
2. State what may/may not be used from the reference
3. Never add as submodule or dependency
4. Never copy security-sensitive code
