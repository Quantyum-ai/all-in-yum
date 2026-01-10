# PRP: Phase 1 Complete Implementation - All 5 Options (Wave-Based Execution)

**Project**: all-in-yum (Multi-Agent Consensus Pipeline)
**Repository**: https://github.com/Quantyum-ai/all-in-yum
**Base Branch**: `phase1-foundation`
**Base Commit**: `<SET_THIS_TO_THE_PHASE1_FOUNDATION_BASELINE_SHA>`
**Target Team**: Claude Dev Team
**Execution Mode**: Wave-based (3 waves, parallel within each wave)
**Status**: Ready for Implementation
**Date**: 2026-01-08

**IMPORTANT**: Before starting, set `Base Commit` to the exact SHA all agents will branch from.
This PRP assumes the baseline already includes:
- `crates/aiy-cli` + `crates/aiy-core/src/config` (TOML config)
- Working `CredentialBackend::SystemKeychain` behavior in `aiy-core`
- `cargo test --workspace --offline` passing (currently 76 tests)

---

## Table of Contents

1. [Executive Summary](#1-executive-summary)
2. [Current State Analysis](#2-current-state-analysis)
3. [Implementation Scope - All 5 Options](#3-implementation-scope---all-5-options)
4. [Wave-Based Execution Strategy](#4-wave-based-execution-strategy)
5. [Wave 1: Adapter Completion + Testing Infrastructure](#5-wave-1-adapter-completion--testing-infrastructure)
6. [Wave 2: CLI Enhancement](#6-wave-2-cli-enhancement)
7. [Wave 3: Consensus Engine](#7-wave-3-consensus-engine)
8. [Security Requirements (All Waves)](#8-security-requirements-all-waves)
9. [Testing Requirements (All Waves)](#9-testing-requirements-all-waves)
10. [Integration & Merge Strategy](#10-integration--merge-strategy)
11. [Acceptance Criteria](#11-acceptance-criteria)
12. [Verification Protocol](#12-verification-protocol)

---

## 1. Executive Summary

### 1.1 Mission

Complete Phase 1 of the all-in-yum multi-agent consensus pipeline by implementing all foundational components across 3 execution waves:

**Wave 1**: Real HTTP adapters + Testing infrastructure (parallel)
**Wave 2**: CLI expansion for user workflows (depends on Wave 1)
**Wave 3**: Consensus engine (depends on Wave 1 + Wave 2)

### 1.2 Scope Summary

| Option | Component | LOC | Complexity | Wave |
|--------|-----------|-----|------------|------|
| 1 | Grok Stage B (Real HTTP) | ~200 | Medium | 1 |
| 2 | Claude/Gemini/Codex Adapters | ~1,500 | Medium | 1 |
| 5 | Testing Infrastructure | ~400 | Low | 1 |
| 4 | CLI Expansion | ~300 | Low | 2 |
| 3 | Consensus Engine | ~800 | High | 3 |
| **Total** | **All Options** | **~3,200** | | **3 Waves** |

### 1.3 Execution Timeline

| Wave | Duration | Agents | Parallelizable |
|------|----------|--------|----------------|
| Wave 1 | 2-3 days | 5 Opus | ✅ Yes |
| Wave 2 | 1 day | 1 Opus | Partial |
| Wave 3 | 2 days | 2 Opus | ✅ Yes |
| **Total** | **5-6 days** | **8 Opus** | |

---

## 2. Current State Analysis

### 2.1 Existing Infrastructure (Phase 1 foundation baseline)

| Crate | Status | LOC | Tests |
|-------|--------|-----|-------|
| `aiy-core` | ✅ Complete | ~1,900 | 56 |
| `aiy-adapters` | ✅ Traits defined | ~60 | 0 |
| `aiy-adapter-grok` | ⚠️ Stage A only | ~1,000 | 19 |
| `aiy-cli` | ✅ Basic commands | ~250 | 1 |
| **Total** | | **~3,210** | **76** |

### 2.2 What's Missing

| Component | Status | Required For |
|-----------|--------|--------------|
| Real HTTP transport (Grok) | ❌ | Production API calls |
| Claude adapter | ❌ | Anthropic integration |
| Gemini adapter | ❌ | Google integration |
| Codex adapter | ❌ | OpenAI integration |
| Consensus engine | ❌ | Multi-agent voting |
| Review CLI command | ❌ | User workflow |
| CI/CD pipeline | ❌ | Automated verification |

### 2.3 Dependency Chain

```
Testing Infra (Option 5)
    │
    ├─→ Can develop in parallel with everything
    │
Grok Stage B (Option 1) ──┐
Claude Adapter (Option 2) ─┼─→ Wave 1 (Parallel)
Gemini Adapter (Option 2) ─┤
Codex Adapter (Option 2) ──┘
    │
    ├─→ All adapters complete
    │
    ├─→ CLI Expansion (Option 4) ← Wave 2
    │
    └─→ Consensus Engine (Option 3) ← Wave 3
```

---

## 3. Implementation Scope - All 5 Options

### Option 1: Grok Stage B - Real HTTP Transport

**Goal**: Complete Grok adapter with production-ready HTTP client

**Deliverables**:
- [ ] Add `reqwest` dependency (feature-gated: `http`)
- [ ] Implement `ReqwestTransport: HttpTransport`
- [ ] Real HTTP POST with timeout, retry logic
- [ ] TLS verification
- [ ] Integration tests with `wiremock` (mock server)
- [ ] Error handling for network failures, timeouts, rate limits

**Files to Create/Modify**:
```
crates/aiy-adapter-grok/
├── Cargo.toml (add reqwest, wiremock)
├── src/
│   ├── client.rs (add ReqwestTransport impl)
│   └── lib.rs (export with feature flag)
└── tests/
    └── http_integration_tests.rs (wiremock tests)
```

**Acceptance Criteria**:
- Grok API calls work with real endpoint (can be tested in isolation)
- Timeout handling verified
- Error messages don't leak API keys
- Tests run offline by default, opt-in for HTTP tests

---

### Option 2: Additional Adapters (Claude/Gemini/Codex)

**Goal**: Implement 3 more adapters following Grok pattern

#### 2A: Claude Adapter (Anthropic API)

**Deliverables**:
- [ ] New crate: `crates/aiy-adapter-claude`
- [ ] Claude model enum (claude-3-5-sonnet, claude-3-opus, etc.)
- [ ] Anthropic API client (reqwest-based)
- [ ] Implements `AgentAdapter` trait
- [ ] Uses aiy-core credentials (provider: "anthropic")
- [ ] Prompt injection defenses integrated
- [ ] 15+ tests (model mapping, API format, credential integration)

**API Details**:
- Base URL: `https://api.anthropic.com/v1`
- Auth: `x-api-key` header
- Format: Anthropic Messages API

#### 2B: Gemini Adapter (Google API)

**Deliverables**:
- [ ] New crate: `crates/aiy-adapter-gemini`
- [ ] Gemini model enum (gemini-1.5-pro, gemini-1.5-flash, etc.)
- [ ] Google Generative AI client
- [ ] Implements `AgentAdapter` trait
- [ ] Uses aiy-core credentials (provider: "google")
- [ ] Prompt injection defenses integrated
- [ ] 15+ tests

**API Details**:
- Base URL: `https://generativelanguage.googleapis.com/v1beta`
- Auth: API key query parameter
- Format: Google Generative AI API

#### 2C: Codex Adapter (OpenAI API)

**Deliverables**:
- [ ] New crate: `crates/aiy-adapter-codex`
- [ ] OpenAI model enum (gpt-4, gpt-4-turbo, o1, etc.)
- [ ] OpenAI API client (similar to Grok, different models)
- [ ] Implements `AgentAdapter` trait
- [ ] Uses aiy-core credentials (provider: "openai")
- [ ] Prompt injection defenses integrated
- [ ] 15+ tests

**API Details**:
- Base URL: `https://api.openai.com/v1`
- Auth: `Authorization: Bearer <key>`
- Format: OpenAI Chat Completions API

**Files Created** (per adapter):
```
crates/aiy-adapter-{claude,gemini,codex}/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── error.rs
│   ├── models.rs
│   ├── types.rs
│   └── client.rs
└── tests/
    └── integration_tests.rs
```

---

### Option 3: Consensus Engine

**Goal**: Orchestrate multiple agents and aggregate their reviews

**Deliverables**:
- [ ] New crate: `crates/aiy-consensus`
- [ ] `ConsensusEngine` struct
- [ ] Voting strategies: unanimous, majority, weighted
- [ ] Conflict resolution logic
- [ ] Aggregation of AgentReview → ConsensusResult
- [ ] Parallel agent execution (tokio tasks)
- [ ] Timeout handling per agent
- [ ] 20+ tests (voting, aggregation, edge cases)

**Core Types**:
```rust
pub struct ConsensusEngine {
    adapters: Vec<Box<dyn AgentAdapter>>,
    strategy: VotingStrategy,
    timeout_ms: u64,
}

pub enum VotingStrategy {
    Unanimous,           // All must approve
    Majority,            // >50% must approve
    Weighted { weights: HashMap<String, f64> },
    Custom(Box<dyn Fn(&[AgentReview]) -> Verdict>),
}

pub struct ConsensusResult {
    pub verdict: Verdict,
    pub confidence: f64,
    pub agent_reviews: Vec<AgentReview>,
    pub disagreements: Vec<Disagreement>,
    pub reasoning: String,
}
```

**Files to Create**:
```
crates/aiy-consensus/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── engine.rs (ConsensusEngine)
│   ├── voting.rs (VotingStrategy implementations)
│   ├── aggregation.rs (merge reviews)
│   └── error.rs
└── tests/
    ├── voting_tests.rs
    ├── aggregation_tests.rs
    └── integration_tests.rs
```

---

### Option 4: CLI Expansion

**Goal**: Add user-facing commands for review workflows

**Deliverables**:
- [ ] `aiy review <file>` - Review a code file with consensus
- [ ] `aiy review --artifact <path>` - Review any artifact
- [ ] `aiy agent list` - List available agents
- [ ] `aiy agent enable <name>` - Enable an agent
- [ ] `aiy agent disable <name>` - Disable an agent
- [ ] `aiy config show` - Display current config
- [ ] `aiy config set <key> <value>` - Update config
- [ ] `aiy config reset` - Reset to defaults

**Files to Modify/Create**:
```
crates/aiy-cli/src/
├── main.rs (add new subcommands)
└── commands/
    ├── review.rs (NEW)
    ├── agent.rs (NEW)
    └── config.rs (NEW)
```

**Features**:
- Progress display during multi-agent review
- Color-coded output (agent verdicts)
- Summary table with consensus result
- Export to JSON/markdown

---

### Option 5: Testing Infrastructure

**Goal**: Automated testing and CI/CD pipeline

**Deliverables**:
- [ ] GitHub Actions workflow (`.github/workflows/ci.yml`)
- [ ] Test matrix: Linux, macOS, Windows
- [ ] Clippy check (deny warnings)
- [ ] Optional: Security audit (`cargo audit`) (network required)
- [ ] Optional: Code coverage (`cargo tarpaulin`) (Linux-only, extra tooling)
- [ ] Integration test framework
- [ ] Mock agent responses for consensus testing
- [ ] Performance benchmarks (optional)

**Files to Create**:
```
.github/workflows/
├── ci.yml (main CI pipeline)
├── security-audit.yml (weekly audit)
└── release.yml (automated releases)

crates/aiy-testing/  (optional helper crate)
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── mock_agents.rs (mock AgentAdapter implementations)
    ├── fixtures.rs (test data)
    └── assertions.rs (custom test helpers)
```

**CI Pipeline Steps**:
1. Checkout code
2. Rust toolchain setup
3. `cargo build --workspace`
4. `cargo test --workspace`
5. `cargo clippy --workspace --all-targets -- -D warnings`
6. Optional: `cargo audit` (network required)
7. Optional: code coverage (Linux only)

---

## 4. Wave-Based Execution Strategy

### 4.1 Interface Freeze (Required Before Wave 1 Parallel Work)

**Why**: The current `aiy-adapters::AgentAdapter` trait only exposes metadata (`id`, `display_name`).
Wave 2 (`aiy review`) and Wave 3 (consensus engine) require a stable, shared async review method.

**Deliverables (single PR, merge first)**:
- [ ] Extend `crates/aiy-adapters/src/traits.rs`:
  - Add `async fn review_artifact(&self, artifact: &str) -> Result<AgentReview, AdapterError>;`
  - Add a minimal `AdapterError` type in `aiy-adapters` (prefer no new deps; a `message: String` wrapper is sufficient).
- [ ] Update `crates/aiy-adapter-grok` to implement the new trait method (delegate to the client; map errors to `AdapterError` without leaking secrets).
- [ ] Update any call sites/tests that assumed inherent `review_artifact()` only.
- [ ] Verify: `cargo test --workspace --offline` passes.

**Rule**: Do not start Wave 1 parallel adapter work until this interface PR is merged into `phase1-foundation`.

### Wave 1: Foundations (Parallel - 5 Agents)

**Duration**: 2-3 days
**Can Start**: After Interface Freeze PR is merged
**Dependencies**: Interface Freeze PR (Section 4.1)

| Agent | Task | Output | Estimated LOC |
|-------|------|--------|---------------|
| **Opus 1** | Grok Stage B | reqwest integration | ~200 |
| **Opus 2** | Claude Adapter | Complete crate | ~500 |
| **Opus 3** | Gemini Adapter | Complete crate | ~500 |
| **Opus 4** | Codex Adapter | Complete crate | ~500 |
| **Opus 5** | Testing Infra | CI/CD + test helpers | ~400 |

**Coordination**: Minimal - each agent works on separate directories

**Merge Strategy**: Can merge all in single commit or individually

---

### Wave 2: CLI Enhancement (1 Agent)

**Duration**: 1 day
**Can Start**: After Wave 1 adapters complete
**Dependencies**: Needs adapters for `aiy review` command

| Agent | Task | Output | Estimated LOC |
|-------|------|--------|---------------|
| **Opus 6** | CLI Expansion | review/agent/config commands | ~300 |

**Coordination**: Depends on Wave 1 consensus about adapter interfaces

---

### Wave 3: Consensus Engine (2 Agents)

**Duration**: 2 days
**Can Start**: After Wave 1 + Wave 2 complete
**Dependencies**: Needs all adapters + CLI review command

| Agent | Task | Output | Estimated LOC |
|-------|------|--------|---------------|
| **Opus 7** | Consensus Core | Engine + voting strategies | ~400 |
| **Opus 8** | Consensus Aggregation | Review merging + conflict resolution | ~400 |

**Coordination**: Agents 7 and 8 must coordinate on shared types

---

## 5. Wave 1: Adapter Completion + Testing Infrastructure

### 5.1 Agent 1: Grok Stage B (Real HTTP)

**Objective**: Complete Grok adapter with production HTTP client

**Current State**: Mock transport only (client.rs:25-77)

**Implementation Tasks**:

1. **Add Dependencies** (Cargo.toml):
   ```toml
   [dependencies]
   # Existing...

   # HTTP client (feature-gated)
   reqwest = { version = "0.12", features = ["json"], optional = true }

   [dev-dependencies]
   wiremock = "0.6"

   [features]
   default = []
   http = ["dep:reqwest"]
   ```

2. **Create ReqwestTransport** (client.rs):
   ```rust
   #[cfg(feature = "http")]
   pub struct ReqwestTransport {
       client: reqwest::Client,
       timeout: Duration,
   }

   #[cfg(feature = "http")]
   #[async_trait]
   impl HttpTransport for ReqwestTransport {
       async fn post_json(&self, url: &str, headers: &[(&str, &str)], body: &str)
           -> Result<String, GrokError> {
           // Implementation with timeout, error handling
       }
   }
   ```

3. **Update GrokClient** (client.rs):
   ```rust
   #[cfg(feature = "http")]
   pub fn new_with_http(credential_manager: Arc<Mutex<CredentialManager>>) -> Self {
       let client = reqwest::Client::builder()
           .timeout(Duration::from_millis(DEFAULT_TIMEOUT_MS))
           .build()
           .expect("Failed to build HTTP client");

       Self::new_with_mock(
           credential_manager,
           Arc::new(ReqwestTransport { client, timeout: ... })
       )
   }
   ```

4. **Add HTTP Integration Tests** (tests/http_integration_tests.rs):
   ```rust
   #[cfg(feature = "http")]
   #[tokio::test]
   async fn test_real_http_request_format() {
       let mock_server = MockServer::start().await;
       // Verify request headers, body format
   }

   #[cfg(feature = "http")]
   #[tokio::test]
   async fn test_timeout_handling() {
       // Verify timeout behavior
   }
   ```

**Security Requirements**:
- [ ] API key never logged in errors
- [ ] TLS verification enabled (no `danger_accept_invalid_certs`)
- [ ] Timeout prevents indefinite hangs
- [ ] Error messages redact sensitive headers

**Acceptance Criteria**:
- [ ] `cargo build --features http` succeeds
- [ ] `cargo test --features http` passes (including wiremock tests)
- [ ] Default build (no features) remains offline-safe
- [ ] Manual test with real Grok API succeeds (documented, not automated)

---

### 5.2 Agent 2: Claude Adapter

**Objective**: Create complete Anthropic Claude adapter

**Reference**: Use Grok adapter as template

**Implementation Tasks**:

1. **Create Crate Structure**:
   ```bash
   crates/aiy-adapter-claude/
   ├── Cargo.toml
   ├── src/
   │   ├── lib.rs (ClaudeAdapter)
   │   ├── error.rs (ClaudeError)
   │   ├── models.rs (ClaudeModel enum)
   │   ├── types.rs (Message types)
   │   └── client.rs (ClaudeClient + HttpTransport)
   └── tests/
       └── integration_tests.rs
   ```

2. **Model Configuration** (models.rs):
   ```rust
   pub enum ClaudeModel {
       Claude35Sonnet,   // claude-3-5-sonnet-20241022
       Claude3Opus,      // claude-3-opus-20240229
       Claude3Sonnet,    // claude-3-sonnet-20240229
       Claude3Haiku,     // claude-3-haiku-20240307
       Claude35Haiku,    // claude-3-5-haiku-20241022
       Claude4Opus,      // claude-opus-4-20250514 (if available)
   }
   ```

3. **API Client** (client.rs):
   - Base URL: `https://api.anthropic.com/v1/messages`
   - Headers:
     - `x-api-key: <key>` (NOT Bearer token)
     - `anthropic-version: 2023-06-01`
     - `content-type: application/json`
   - Request format: Anthropic Messages API
   - Mock transport by default, reqwest feature-gated

4. **Credential Integration**:
   - Provider name: `"anthropic"`
   - Fallback: `"claude"` (compatibility)
   - Uses `aiy_core::CredentialManager`

5. **Tests** (15+ tests):
   - Model string mapping
   - API request format
   - Credential retrieval (anthropic → claude fallback)
   - Message conversion
   - Sanitization integration
   - Schema validation

**Acceptance Criteria**:
- [ ] Implements `AgentAdapter` trait
- [ ] 15+ tests passing (offline by default)
- [ ] Clippy clean
- [ ] Credential integration verified
- [ ] Sanitization pipeline integrated

---

### 5.3 Agent 3: Gemini Adapter

**Objective**: Create complete Google Gemini adapter

**Implementation Tasks**:

1. **Create Crate Structure** (same pattern as Claude)

2. **Model Configuration** (models.rs):
   ```rust
   pub enum GeminiModel {
       Gemini15Pro,      // gemini-1.5-pro-latest
       Gemini15Flash,    // gemini-1.5-flash-latest
       Gemini20FlashExp, // gemini-2.0-flash-exp
   }
   ```

3. **API Client** (client.rs):
   - Base URL: `https://generativelanguage.googleapis.com/v1beta`
   - Auth: API key as query parameter `?key=<key>` (different from others!)
   - Format: Google Generative AI API
   - Request: `generateContent` method

4. **Credential Integration**:
   - Provider name: `"google"`
   - Fallback: `"gemini"` (compatibility)

5. **Tests** (15+ tests)

**Special Considerations**:
- API key in query string (not header) - ensure not logged
- Different request/response format than OpenAI-compatible APIs

**Acceptance Criteria**: Same as Claude adapter

---

### 5.4 Agent 4: Codex Adapter

**Objective**: Create complete OpenAI adapter (Codex) following the Grok pattern

**Implementation Tasks**:

1. **Create Crate Structure**:
   ```bash
   crates/aiy-adapter-codex/
   ├── Cargo.toml
   ├── src/
   │   ├── lib.rs (CodexAdapter)
   │   ├── error.rs (CodexError)
   │   ├── models.rs (OpenAI model enum)
   │   ├── types.rs (OpenAI chat request/response types)
   │   └── client.rs (CodexClient + HttpTransport)
   └── tests/
       └── integration_tests.rs
   ```

2. **API Client** (client.rs):
   - Base URL: `https://api.openai.com/v1`
   - Endpoint: `POST /chat/completions` (OpenAI Chat Completions API)
   - Auth: `Authorization: Bearer <api_key>`
   - Mock transport by default, reqwest transport behind `http` feature flag

3. **Credential Integration**:
   - Provider name: `"openai"`
   - Fallback: `"codex"` (compatibility)
   - Uses `aiy_core::CredentialManager`

4. **Security Integration**:
   - `sanitize_artifact_content()` on artifact input
   - `build_secure_review_prompt()` for prompt construction
   - `validate_review_response()` + `validate_review_schema()` on outputs

5. **Tests** (15+ tests):
   - Model string mapping
   - Request format
   - Credential retrieval (openai → codex fallback)
   - Sanitization + schema validation integration

**Acceptance Criteria**:
- [ ] Implements `AgentAdapter` (including `review_artifact()` per Section 4.1)
- [ ] 15+ tests passing (offline by default)
- [ ] Clippy clean
- [ ] Credential integration verified
- [ ] Sanitization pipeline integrated

---

### 5.5 Agent 5: Testing Infrastructure

**Objective**: Automated testing and CI/CD

**Implementation Tasks**:

1. **GitHub Actions Workflow** (.github/workflows/ci.yml):
   ```yaml
   name: CI

   on: [push, pull_request]

   jobs:
     test:
       strategy:
         matrix:
           os: [ubuntu-latest, macos-latest, windows-latest]
           rust: [stable]

       steps:
         - uses: actions/checkout@v4
         - uses: actions-rust-lang/setup-rust-toolchain@v1

         - name: Build
           run: cargo build --workspace

         - name: Test
           run: cargo test --workspace

         - name: Clippy
           run: cargo clippy --workspace --all-targets -- -D warnings

     security:
       runs-on: ubuntu-latest
       steps:
         - uses: actions/checkout@v4
         - run: cargo install cargo-audit
         - run: cargo audit
   ```

2. **Mock Agent Framework** (crates/aiy-testing/src/mock_agents.rs):
   ```rust
   pub struct MockAgent {
       id: String,
       response: AgentReview,
   }

   impl MockAgent {
       pub fn new_approving(id: &str) -> Self {
           // Returns Pass verdict
       }

       pub fn new_rejecting(id: &str) -> Self {
           // Returns Block verdict
       }

       pub fn with_issues(id: &str, issues: Vec<Issue>) -> Self {
           // Returns Issue verdict
       }
   }
   ```

3. **Test Fixtures** (crates/aiy-testing/src/fixtures.rs):
   ```rust
   pub fn sample_code_artifact() -> &'static str { ... }
   pub fn malicious_injection_artifact() -> &'static str { ... }
   pub fn valid_agent_review_json() -> &'static str { ... }
   ```

4. **Code Coverage** (.github/workflows/coverage.yml):
   ```yaml
   - name: Install tarpaulin
     run: cargo install cargo-tarpaulin

   - name: Generate coverage
     run: cargo tarpaulin --out Xml --workspace

   - name: Upload to codecov
     uses: codecov/codecov-action@v3
   ```

**Acceptance Criteria**:
- [ ] CI runs on push/PR
- [ ] Tests run on all 3 OSes
- [ ] Security audit in CI
- [ ] Mock agents available for consensus testing
- [ ] Coverage reporting (optional but recommended)

---

## 6. Wave 2: CLI Enhancement

### 6.1 Agent 6: Review and Management Commands

**Objective**: User-facing review workflow

**Dependencies**: All adapters from Wave 1 must be complete

**Implementation Tasks**:

1. **Review Command** (commands/review.rs):
   ```rust
   pub async fn run(args: ReviewArgs) -> anyhow::Result<()> {
       // Load artifact from file
       let artifact = std::fs::read_to_string(&args.file)?;

       // Load config to determine enabled agents
       let config = PipelineConfig::load(&PipelineConfig::config_path()?)?;

       // Initialize adapters based on config
       let adapters = initialize_adapters(&config).await?;

       // Run consensus review
       let engine = ConsensusEngine::new(adapters, VotingStrategy::Majority);
       let result = engine.review(&artifact).await?;

       // Display results
       display_consensus_result(&result);
   }
   ```

2. **Agent Management** (commands/agent.rs):
   ```rust
   pub fn list() -> anyhow::Result<()> {
       let config = PipelineConfig::load(...)?;
       // Display enabled/disabled agents
   }

   pub fn enable(name: String) -> anyhow::Result<()> {
       let mut config = PipelineConfig::load(...)?;
       config.enabled_agents.push(name);
       config.save(...)?;
   }
   ```

3. **Config Management** (commands/config.rs):
   ```rust
   pub fn show() -> anyhow::Result<()> {
       let config = PipelineConfig::load(...)?;
       println!("{}", toml::to_string_pretty(&config)?);
   }

   pub fn set(key: String, value: String) -> anyhow::Result<()> {
       // Update config field
   }
   ```

**User Experience**:
```bash
$ aiy review src/main.rs

Reviewing with enabled agents: claude, grok, gemini...

[Claude] ✓ Pass (confidence: 0.95)
[Grok]   ✓ Pass (confidence: 0.88)
[Gemini] ⚠ Issue (confidence: 0.92)
  - Minor: Consider adding error handling at line 42

Consensus: PASS (2/3 approve, majority reached)
Confidence: 0.92 (average)

Issues found: 1 minor
  • Gemini: Line 42 - Missing error handling

Recommendation: Safe to merge with minor improvements
```

**Acceptance Criteria**:
- [ ] `aiy review <file>` completes full consensus workflow
- [ ] Progress display during review
- [ ] Summary table with verdict
- [ ] Agent commands work
- [ ] Config commands work
- [ ] All commands tested

---

## 7. Wave 3: Consensus Engine

### 7.1 Agent 7: Consensus Core

**Objective**: Core orchestration and voting logic

**Dependencies**: All Wave 1 adapters + Wave 2 CLI

**Implementation Tasks**:

1. **ConsensusEngine** (engine.rs):
   ```rust
   pub struct ConsensusEngine {
       adapters: Vec<Box<dyn AgentAdapter>>,
       strategy: VotingStrategy,
       timeout_per_agent_ms: u64,
       parallel: bool,
   }

   impl ConsensusEngine {
       pub async fn review(&self, artifact: &str) -> Result<ConsensusResult> {
           // Parallel execution
           let reviews: Vec<AgentReview> = if self.parallel {
               self.review_parallel(artifact).await?
           } else {
               self.review_sequential(artifact).await?
           };

           // Apply voting strategy
           let verdict = self.strategy.decide(&reviews)?;

           // Aggregate results
           Ok(ConsensusResult { ... })
       }
   }
   ```

2. **Voting Strategies** (voting.rs):
   ```rust
   pub enum VotingStrategy {
       Unanimous,
       Majority,
       Weighted { weights: HashMap<String, f64> },
       Supermajority { threshold: f64 }, // e.g., 0.67 for 2/3
   }

   impl VotingStrategy {
       pub fn decide(&self, reviews: &[AgentReview]) -> Result<Verdict> {
           match self {
               Unanimous => {
                   if reviews.iter().all(|r| r.verdict == Verdict::Pass) {
                       Ok(Verdict::Pass)
                   } else {
                       Ok(Verdict::Issue)
                   }
               }
               Majority => {
                   let pass_count = reviews.iter()
                       .filter(|r| r.verdict == Verdict::Pass)
                       .count();
                   if pass_count > reviews.len() / 2 {
                       Ok(Verdict::Pass)
                   } else {
                       Ok(Verdict::Issue)
                   }
               }
               // ... other strategies
           }
       }
   }
   ```

3. **Parallel Execution** (engine.rs):
   ```rust
   async fn review_parallel(&self, artifact: &str) -> Result<Vec<AgentReview>> {
       let futures: Vec<_> = self.adapters.iter().map(|adapter| {
           let artifact = artifact.to_string();
           async move {
               timeout(
                   Duration::from_millis(self.timeout_per_agent_ms),
                   adapter.review_artifact(&artifact)
               ).await
           }
       }).collect();

       let results = futures::future::join_all(futures).await;
       // Handle timeouts and errors
   }
   ```

**Tests** (12+ tests):
- Unanimous voting with all Pass
- Unanimous voting with one Block
- Majority voting (2/3, 3/5, etc.)
- Weighted voting
- Timeout handling
- Parallel vs sequential comparison
- Agent failure handling

**Acceptance Criteria**:
- [ ] All voting strategies implemented
- [ ] Parallel execution works
- [ ] Timeout per agent enforced
- [ ] Error handling for agent failures
- [ ] 12+ tests passing

---

### 7.2 Agent 8: Consensus Aggregation

**Objective**: Merge reviews, resolve conflicts, generate final report

**Dependencies**: Agent 7 (consensus core)

**Implementation Tasks**:

1. **Aggregation Logic** (aggregation.rs):
   ```rust
   pub fn aggregate_reviews(reviews: &[AgentReview]) -> ConsensusResult {
       let verdict = /* from voting strategy */;

       let confidence = reviews.iter()
           .map(|r| r.confidence)
           .sum::<f64>() / reviews.len() as f64;

       let all_issues = merge_issues(reviews);
       let disagreements = find_disagreements(reviews);
       let reasoning = generate_reasoning(reviews, &disagreements);

       ConsensusResult {
           verdict,
           confidence,
           agent_reviews: reviews.to_vec(),
           all_issues,
           disagreements,
           reasoning,
       }
   }
   ```

2. **Issue Merging** (aggregation.rs):
   ```rust
   fn merge_issues(reviews: &[AgentReview]) -> Vec<AggregatedIssue> {
       // Deduplicate similar issues
       // Aggregate severity (most severe wins)
       // Merge suggestions
   }
   ```

3. **Disagreement Analysis** (aggregation.rs):
   ```rust
   pub struct Disagreement {
       pub agents_approving: Vec<String>,
       pub agents_blocking: Vec<String>,
       pub focal_issues: Vec<Issue>,
       pub reasoning_divergence: String,
   }

   fn find_disagreements(reviews: &[AgentReview]) -> Vec<Disagreement> {
       // Identify where agents disagree
       // Extract reasons for divergence
   }
   ```

4. **Reasoning Generation** (aggregation.rs):
   ```rust
   fn generate_reasoning(
       reviews: &[AgentReview],
       disagreements: &[Disagreement]
   ) -> String {
       // Synthesize explanations from all agents
       // Highlight consensus points
       // Note areas of disagreement
   }
   ```

**Tests** (8+ tests):
- Issue deduplication
- Severity aggregation
- Confidence averaging
- Disagreement detection
- Reasoning generation
- Edge cases (all agree, all disagree, split)

**Acceptance Criteria**:
- [ ] Issues properly merged/deduplicated
- [ ] Confidence calculated correctly
- [ ] Disagreements identified
- [ ] Reasoning is coherent
- [ ] 8+ tests passing

---

## 8. Security Requirements (All Waves)

### 8.1 Universal Security Rules

| Rule | Applies To | Enforcement |
|------|------------|-------------|
| No env var credentials | All crates | `rg` check in CI |
| API keys never logged | All adapters | Code review + tests |
| TLS verification | All HTTP transports | reqwest defaults |
| Prompt injection defense | All `review_artifact()` | Use sanitization.rs |
| Output validation | All agent responses | Use validation.rs |
| Credential encryption | aiy-core only | AES-256-GCM + Argon2id |

### 8.2 Per-Adapter Security Checklist

For each adapter (Grok/Claude/Gemini/Codex):

- [ ] Uses `aiy_core::CredentialManager` (no custom storage)
- [ ] Provider name documented
- [ ] Fallback provider name (if any)
- [ ] `review_artifact()` calls `sanitize_artifact_content()`
- [ ] Response validated with `validate_review_response()`
- [ ] Schema enforced with `validate_review_schema()`
- [ ] API key retrieved only when needed (not cached long-term)
- [ ] Error messages redact sensitive data

### 8.3 Credential Provider Keys (Canonical + Fallbacks)

**Rule**: Credential *provider keys* are not the same as agent IDs in `PipelineConfig`.
Each adapter must document and implement a canonical credential key plus optional compatibility fallbacks.

| Adapter | Agent ID | Canonical credential key | Accepted fallbacks |
|--------:|----------|--------------------------|--------------------|
| Grok (xAI) | `grok` | `xai` | `grok` |
| Claude (Anthropic) | `claude` | `anthropic` | `claude` |
| Gemini (Google) | `gemini` | `google` | `gemini` |
| Codex (OpenAI) | `codex` | `openai` | `codex` |

**CLI guidance**: `aiy credentials set <provider>` should recommend the canonical key (e.g., `anthropic`, `openai`, `google`, `xai`).

---

## 9. Testing Requirements (All Waves)

### 9.1 Test Coverage Targets

| Component | Min Tests | Categories |
|-----------|-----------|------------|
| Each adapter | 15 | Model mapping, API format, credentials, sanitization |
| Consensus engine | 20 | Voting strategies, aggregation, edge cases |
| CLI commands | 5 | Each command tested (can be integration tests) |
| Testing infrastructure | N/A | Enables other tests |

### 9.2 Test Categories

**Unit Tests**:
- Model enum mapping
- Request/response serialization
- Voting logic
- Issue aggregation

**Integration Tests**:
- Credential retrieval path (real CredentialManager + temp file)
- Sanitization pipeline (inject → sanitize → validate → parse)
- Multi-agent consensus (using mock agents)

**Offline Safety**:
- All tests must pass with `--offline` flag
- HTTP tests behind feature flags
- No real API calls in default test suite

---

## 10. Integration & Merge Strategy

### 10.1 Branch Strategy

```
main (pinned at 57f71db for GPT-5 Pro review)
    │
    └─→ phase1-foundation (<SET_BASELINE_SHA> - Phase 1 foundation baseline)
            │
            ├─→ interface-freeze (Section 4.1, must merge first)
            ├─→ wave1/grok-stage-b (Agent 1)
            ├─→ wave1/claude-adapter (Agent 2)
            ├─→ wave1/gemini-adapter (Agent 3)
            ├─→ wave1/codex-adapter (Agent 4)
            ├─→ wave1/testing-infra (Agent 5)
            │       │
            │       └─→ Merge all Wave 1 → phase1-foundation
            │
            ├─→ wave2/cli-expansion (Agent 6)
            │       │
            │       └─→ Merge Wave 2 → phase1-foundation
            │
            └─→ wave3/consensus-engine (Agent 7 + 8)
                    │
                    └─→ Merge Wave 3 → phase1-foundation
```

### 10.2 Merge Frequency

**Option A**: Merge after each wave (3 merge commits)
**Option B**: Merge all at end (1 large merge)
**Recommendation**: **Option A** - easier to review, bisect issues

---

## 11. Acceptance Criteria

### 11.1 Wave 1 Complete When:

- [ ] Grok Stage B: `cargo build -p aiy-adapter-grok --features http` succeeds
- [ ] Claude adapter: 15+ tests passing
- [ ] Gemini adapter: 15+ tests passing
- [ ] Codex adapter (if included): 15+ tests passing
- [ ] CI workflow runs successfully on GitHub
- [ ] All workspace tests pass: **Expected ~120 tests** (76 + 45 new)
- [ ] Clippy clean across all crates
- [ ] No env var credential sourcing anywhere
- [ ] All adapters use aiy-core security

### 11.2 Wave 2 Complete When:

- [ ] `aiy review <file>` works end-to-end
- [ ] Agent management commands functional
- [ ] Config management commands functional
- [ ] Help text comprehensive
- [ ] User experience polished (colors, progress, summaries)
- [ ] Integration tests for new commands
- [ ] All workspace tests pass: **Expected ~130 tests**

### 11.3 Wave 3 Complete When:

- [ ] Consensus engine handles 2-5 agents
- [ ] All voting strategies work
- [ ] Issue aggregation and deduplication work
- [ ] Disagreement detection works
- [ ] Timeout handling per agent
- [ ] Parallel execution faster than sequential
- [ ] 20+ consensus tests passing
- [ ] All workspace tests pass: **Expected ~150 tests**
- [ ] Full end-to-end workflow: `aiy review` → consensus → result

---

## 12. Verification Protocol

### 12.1 Per-Wave Verification

After each wave, run:

```bash
# Offline-safety suite (must pass; suitable for restricted environments; no live API calls)
cargo build --workspace --offline
cargo test --workspace --offline
cargo clippy --workspace --all-targets --offline -- -D warnings

# CI suite (network allowed for dependency fetch; tests must NOT call live LLM APIs)
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings

# Optional: wiremock-based HTTP tests (MUST only hit localhost; no live endpoints)
# cargo test -p aiy-adapter-grok --features http

# Secret scan
git ls-files | rg "\.(enc|salt|env)$"
rg -n "sk-[a-zA-Z0-9]{20,}" --type rust crates/
```

**Notes**:
- Do **not** require `--offline` in GitHub Actions; runners fetch dependencies from crates.io. The key requirement is: **no live LLM API calls in CI**.
- `cargo audit` and coverage tooling may require network access; run them as separate CI jobs (scheduled or non-blocking), not as part of the offline-safety gate.

### 12.2 Final Verification (After Wave 3)

```bash
# Full end-to-end test (requires real API keys - manual)
aiy credentials set anthropic
aiy credentials set openai
aiy credentials set xai
aiy agent list
aiy review tests/fixtures/sample.rs
```

**Expected Output**:
- All agents participate
- Consensus reached
- No errors, no key leakage
- Clean summary output

---

## 13. Execution Instructions for Claude Dev Team

### 13.1 Interface Freeze (Must Merge First)

```
Agent Prompt: Implement Section 4.1 (AgentAdapter trait `review_artifact()` + AdapterError), update Grok adapter, and verify `cargo test --workspace --offline`.
```

**Dependency**: This must merge before any Wave 1 adapter work begins.

### 13.2 Wave 1 - Launch 5 Opus Agents in Parallel (After 13.1)

```
Agent 1 Prompt: Implement Grok Stage B (reqwest integration) per Section 5.1
Agent 2 Prompt: Implement Claude Adapter per Section 5.2
Agent 3 Prompt: Implement Gemini Adapter per Section 5.3
Agent 4 Prompt: Implement Codex Adapter per Section 5.4
Agent 5 Prompt: Implement Testing Infrastructure per Section 5.5
```

**Coordination**: Each agent works in separate directory, minimal conflicts.

### 13.3 Wave 2 - Launch 1 Opus Agent (After Wave 1)

```
Agent 6 Prompt: Implement CLI Expansion per Section 6.1
```

**Dependency**: Wait for Wave 1 adapters to merge.

### 13.4 Wave 3 - Launch 2 Opus Agents (After Wave 2)

```
Agent 7 Prompt: Implement Consensus Core per Section 7.1
Agent 8 Prompt: Implement Consensus Aggregation per Section 7.2
```

**Coordination**: Agents 7 and 8 must coordinate on shared types (ConsensusResult, etc.).

---

## 14. Risk Mitigation

### 14.1 Merge Conflicts

**Risk**: 8 agents editing different files could conflict

**Mitigation**:
- Wave-based execution (merge between waves)
- Interface Freeze PR merged first (Section 4.1)
- Clear file ownership per agent
- Coordinate on shared types (ConsensusResult)

### 14.2 API Breaking Changes

**Risk**: Adapter trait might need changes mid-implementation

**Mitigation**:
- Define full `AgentAdapter` trait upfront (before Wave 1)
- Add methods to trait as needed
- Update all adapters in lockstep

### 14.3 Test Complexity

**Risk**: 150 tests might be slow in CI

**Mitigation**:
- Offline tests by default
- HTTP tests behind feature flags
- Parallel test execution in CI

---

## 15. Success Metrics

### 15.1 By End of Wave 1

- ✅ 4 complete adapters (Grok HTTP, Claude, Gemini, Codex)
- ✅ ~120 tests passing
- ✅ CI running on GitHub
- ✅ All adapters follow same security pattern

### 15.2 By End of Wave 2

- ✅ Functional `aiy review` command
- ✅ Agent/config management working
- ✅ ~130 tests passing

### 15.3 By End of Wave 3

- ✅ Multi-agent consensus working
- ✅ All voting strategies implemented
- ✅ ~150 tests passing
- ✅ Full end-to-end workflow functional

---

## 16. Out of Scope (Phase 2)

These are explicitly NOT included in this PRP:

- Web UI or dashboard
- Database persistence
- User authentication
- Cloud deployment
- Rate limiting / quota management
- Agent response caching
- Streaming consensus updates
- Plugin system for custom agents

---

## Appendix A: Estimated Timeline

```
Week 1:
├─ Day 1-2: Wave 1 execution (4 agents parallel)
├─ Day 3: Wave 1 merge + verification
├─ Day 4: Wave 2 execution (1 agent)
└─ Day 5: Wave 2 merge + verification

Week 2:
├─ Day 1-2: Wave 3 execution (2 agents parallel)
├─ Day 3: Wave 3 merge + verification
├─ Day 4: Integration testing
└─ Day 5: Final review + documentation
```

---

## Appendix B: Agent Prompts (Quick Reference)

### Interface Freeze (Prerequisite)
"Implement Section 4.1: extend `aiy-adapters::AgentAdapter` with async `review_artifact()` + add `AdapterError`, update Grok adapter accordingly, and verify `cargo test --workspace --offline`."

### Agent 1 (Grok Stage B)
"Implement real HTTP transport for Grok adapter using reqwest. Add feature flag (`http`), timeout handling, wiremock tests (localhost only). Reference: Section 5.1 of prp-phase1-complete-implementation.md"

### Agent 2 (Claude)
"Create aiy-adapter-claude crate following Grok pattern. Anthropic API, x-api-key auth, 15+ tests. Reference: Section 5.2"

### Agent 3 (Gemini)
"Create aiy-adapter-gemini crate following Grok pattern. Google API, query-string auth, 15+ tests. Reference: Section 5.3"

### Agent 4 (Codex / OpenAI)
"Create aiy-adapter-codex crate following Grok pattern. OpenAI Chat Completions API, Bearer auth, 15+ tests. Reference: Section 5.4"

### Agent 5 (Testing)
"Implement CI/CD workflows, mock agent framework, test fixtures. Reference: Section 5.5"

### Agent 6 (CLI)
"Add review/agent/config commands to aiy-cli. Requires all adapters. Reference: Section 6.1"

### Agent 7 (Consensus Core)
"Implement ConsensusEngine with voting strategies, parallel execution, timeout handling. Reference: Section 7.1"

### Agent 8 (Consensus Aggregation)
"Implement issue merging, disagreement detection, reasoning generation. Reference: Section 7.2"

---

**Document Version**: 1.0
**Classification**: Internal - Implementation Specification
**Distribution**: Claude Dev Team, Project Maintainers
**Base Commit**: `<SET_THIS_TO_THE_PHASE1_FOUNDATION_BASELINE_SHA>` (phase1-foundation)
