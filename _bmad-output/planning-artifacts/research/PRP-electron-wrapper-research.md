---
document_type: PRP
title: "Electron Desktop Wrapper Research Brief"
project: all-in-yum (aiy CLI)
assigned_to: GPT-5 Pro Research Team
created_by: Claude Planning Team
date: 2026-01-16
status: ready_for_research
output_path: docs/research-electron-wrapper.md
---

# Planning Research Proposal (PRP)

## Electron Desktop Wrapper for aiy CLI — Best Practices & Proven Patterns

---

## 1. Executive Context

### 1.1 Project Background

The **all-in-yum** project (`aiy` CLI) is a Rust-based command-line tool. We are planning an Electron desktop wrapper that treats the **CLI as the source of truth** — the Electron app is a thin UI layer that invokes the CLI, displays results, and watches state files.

### 1.2 Research Objective

Produce a comprehensive, sourced research document covering best practices and proven patterns for building a **secure, enterprise-grade "CLI-as-source-of-truth" desktop application** using Electron.

> **This research task must not modify the repository. Deliverable is documentation only.**

### 1.3 Why This Research Matters

- Architectural decisions made now will affect maintainability for years
- Security/privacy posture is a hard requirement, not a nice-to-have
- Cross-platform reliability is critical (macOS, Windows, Linux)
- We want to learn from production-proven patterns (GitHub Desktop, Docker Desktop, 1Password)

---

## 2. Hard Constraints (Non-Negotiable)

These constraints MUST inform all research findings and recommendations:

| Constraint | Description |
|------------|-------------|
| **CLI is Source of Truth** | All business logic lives in the Rust CLI. Electron only invokes commands and displays results. |
| **No Code/Paths/Diffs to Cloud** | Source code, file paths, diffs, stack traces, and dependency trees MUST NOT leave the local machine without explicit user approval. |
| **Redacted Cloud Planning** | If cloud features exist (e.g., AI-assisted planning), only redacted metadata may be sent, and only with per-action user confirmation. |
| **Zero Telemetry Default** | No analytics, crash reporting, or usage tracking enabled by default. User must explicitly opt-in. |
| **Local-First Architecture** | All state, logs, and configuration stored locally. Network features are additive, not required. |
| **No Proprietary Content in Report** | Do not include any proprietary/all-in-yum source code, file paths, stack traces, diffs, dependency lists, or other sensitive repo-specific content in the report. Use generic examples or redacted placeholders only. |

---

## 3. Research Topics

### 3.1 Electron + CLI Integration Best Practices

**Research Goal:** Document proven patterns for reliable CLI process management from Electron.

#### 3.1.1 Process Spawning

Research and document:

- **`child_process.spawn` vs `execFile` vs `exec`**
  - When to use each approach
  - Memory and performance implications
  - Shell vs no-shell execution security considerations

- **Streaming stdout/stderr**
  - Real-time output handling patterns
  - Avoiding buffering deadlocks
  - Backpressure handling when output is faster than UI can render

- **Large Output Handling**
  - Memory management for commands that produce megabytes of output
  - Streaming to disk vs in-memory buffering thresholds
  - Truncation strategies for display

- **Platform Differences**
  - Windows vs POSIX spawn behavior differences
  - PATH resolution differences across platforms
  - Shell quoting and escaping requirements

#### 3.1.2 Command Queue Patterns

Research and document:

- **Per-Repository Serialization**
  - Why certain commands must not run concurrently
  - Queue implementation patterns (in-memory vs persistent)
  - Priority queue considerations (user-initiated vs background)

- **Cancellation**
  - Graceful cancellation via signals (SIGTERM, SIGINT)
  - Windows-specific cancellation challenges
  - Cleanup on cancellation (temp files, locks)

- **Retries and Idempotency**
  - Which commands are safe to retry
  - Exponential backoff patterns
  - User notification for retry scenarios

- **Timeout Handling**
  - Reasonable defaults for different command types
  - User-configurable timeouts
  - Zombie process prevention

#### 3.1.3 Output Parsing

Research and document:

- **Mixed Output Handling**
  - Parsing JSON from stdout while stderr contains progress/warnings
  - Handling interleaved stdout/stderr
  - Non-JSON output (plain text, progress bars, ANSI codes)

- **Exit Code Semantics**
  - Standard exit code conventions (0 success, 1 error, 2 usage)
  - Signal-based exits (128 + signal number)
  - Distinguishing "command failed" from "command not found"

- **Error Extraction**
  - Parsing structured errors from CLI output
  - Extracting user-friendly messages from technical errors
  - Preserving full error context for debugging

#### 3.1.4 Specific Questions to Answer

1. What is the recommended pattern for bi-directional communication with a long-running CLI process?
2. How do production apps handle the "CLI produces output faster than UI can render" problem?
3. What are the gotchas for running CLI commands that themselves spawn child processes?
4. How should the Electron app handle CLI crashes vs clean exits vs hangs?

---

### 3.2 Cross-Platform File Watching Reliability

**Research Goal:** Document reliable patterns for watching state files across all platforms.

#### 3.2.1 Watcher Technologies

Research and document:

- **Chokidar**
  - Current status and maintenance (as of 2025-2026)
  - Known issues and platform-specific quirks
  - Configuration best practices

- **Native fs.watch / fs.watchFile**
  - When native watchers are sufficient
  - Platform-specific behavior differences
  - Resource consumption comparison

- **Alternative Libraries**
  - @parcel/watcher, nsfw, watchman
  - Pros/cons comparison for desktop app use case
  - Maintenance status and community support

#### 3.2.2 Platform-Specific Issues

Research and document:

- **macOS FSEvents**
  - Coalescing behavior and timing
  - File ID vs path-based watching
  - Issues with case-insensitive filesystems

- **Windows**
  - ReadDirectoryChangesW limitations
  - Network drive watching challenges
  - Antivirus interference patterns

- **Linux**
  - inotify watch limits and configuration
  - Different filesystem behaviors (ext4, btrfs, NFS)
  - Container/WSL boundary issues

- **WSL (Windows Subsystem for Linux)**
  - Cross-filesystem watching challenges
  - Performance implications
  - Recommended approaches for hybrid setups

#### 3.2.3 Reliability Patterns

Research and document:

- **Debouncing**
  - Recommended debounce intervals for different file types
  - Trailing vs leading edge debounce
  - Per-file vs global debounce strategies

- **Missed Event Handling**
  - Detection strategies for missed events
  - Periodic reconciliation approaches
  - Checksum-based change detection

- **Fallback Polling**
  - When to fall back to polling
  - Recommended polling intervals (battery vs plugged-in)
  - Hybrid approaches (watch with periodic verification)

#### 3.2.4 Specific Questions to Answer

1. What is the recommended approach for watching a single JSON state file that changes frequently?
2. How do production apps handle the "event fired but file not yet fully written" race condition?
3. What are the best practices for watching files on network drives or cloud-synced folders?
4. How should the app behave when file watching fails silently?

---

### 3.3 Packaging & Distribution (Electron + Bundled Rust Binary)

**Research Goal:** Document proven patterns for bundling native binaries with Electron apps.

#### 3.3.1 Binary Bundling Approaches

Research and document:

- **Resources Folder Approach**
  - Placing binaries in `resources/` directory
  - Path resolution at runtime (`app.getPath('exe')`, `process.resourcesPath`)
  - Pros/cons vs asar approach

- **Asar Unpacked**
  - Using `asarUnpack` for native binaries
  - Directory structure best practices
  - Size implications

- **External Binary (User-Installed)**
  - PATH lookup patterns
  - User-specified binary path configuration
  - Version compatibility checking

- **Platform-Specific Considerations**
  - macOS: app bundle structure, Gatekeeper
  - Windows: exe location, UAC considerations
  - Linux: AppImage vs deb vs rpm binary locations

#### 3.3.2 Code Signing & Notarization

Research and document:

- **macOS**
  - Code signing requirements (Developer ID)
  - Notarization process and requirements
  - Hardened runtime implications for CLI invocation
  - Signing bundled binaries (Rust CLI must also be signed)

- **Windows**
  - Authenticode signing process
  - EV certificates vs standard certificates
  - SmartScreen reputation building
  - Signing bundled binaries

- **Linux**
  - GPG signing for packages
  - Repository signing
  - Flatpak/Snap considerations

#### 3.3.3 Auto-Update Patterns

Research and document:

- **Update Frameworks**
  - electron-updater (electron-builder)
  - Squirrel (Windows/macOS)
  - Custom update solutions

- **Secure Update Practices**
  - Update server requirements (HTTPS, signature verification)
  - Delta updates vs full updates
  - Rollback capabilities

- **Binary Update Considerations**
  - Updating the Rust CLI independently of Electron shell
  - Version compatibility matrix
  - Atomic update patterns

- **Privacy-Respecting Updates**
  - Update checking without telemetry
  - User-controlled update preferences
  - Offline/airgapped update support

#### 3.3.4 Specific Questions to Answer

1. What is the recommended approach for bundling a Rust binary that works on all three platforms?
2. How do production apps handle the case where the bundled CLI version differs from a user-installed version?
3. What are the code signing gotchas when the Electron app invokes a bundled binary?
4. How should auto-update work when the Rust CLI has a different release cadence than the Electron shell?

---

### 3.4 Security/Privacy UX Patterns for Developer Tools

**Research Goal:** Document UX patterns that give users confidence their data stays local.

#### 3.4.1 "Always Local" Kill-Switch Patterns

Research and document:

- **Implementation Patterns**
  - Global offline mode toggle
  - Per-feature network access controls
  - Visual indicators of network status

- **UX Considerations**
  - Discoverability of privacy controls
  - Clear communication of what "local only" means
  - Graceful degradation when network features disabled

- **Technical Implementation**
  - Network request interception
  - Feature flagging for network-dependent features
  - Persistent preference storage

#### 3.4.2 Cloud Confirmation Modals

Research and document:

- **Redacted Preview Patterns**
  - What to show (metadata) vs what to hide (content)
  - Diff/preview with sensitive data masked
  - User-expandable detail levels

- **Per-Action Confirmation**
  - Single action vs batch confirmation
  - "Remember this choice" options and their security implications
  - Timeout and expiration for remembered choices

- **Trust Indicators**
  - Visual distinction between local and cloud operations
  - Destination/endpoint transparency
  - Data retention/deletion policies display

#### 3.4.3 Zero-Telemetry Defaults

Research and document:

- **Industry Patterns**
  - How privacy-focused tools communicate their stance
  - Opt-in vs opt-out telemetry patterns
  - Telemetry transparency (what is collected, where it goes)

- **Technical Implementation**
  - Build-time telemetry removal
  - Runtime telemetry disabling
  - Audit capabilities for users to verify no telemetry

- **Crash Reporting**
  - Local-only crash logs
  - User-initiated crash report submission
  - Automatic redaction of sensitive data in crash reports

#### 3.4.4 Local Logging Patterns

Research and document:

- **JSONL Logging**
  - Schema design for structured logs
  - Timestamp and session correlation
  - Log levels and filtering

- **Rotation and Retention**
  - Size-based vs time-based rotation
  - Retention period recommendations
  - User-accessible log location

- **What NOT to Log**
  - Sensitive data categories to exclude
  - PII handling in logs
  - Secrets and credentials detection

- **Log Viewer UX**
  - In-app log viewing capabilities
  - Export and sharing (with redaction)
  - Search and filtering

#### 3.4.5 Specific Questions to Answer

1. How do privacy-focused developer tools communicate their security posture to users?
2. What are the UX patterns for "show me exactly what will be sent" before a cloud operation?
3. How do production apps handle the tension between useful error reporting and privacy?
4. What are best practices for log file locations across platforms?

---

### 3.5 Comparable Product Patterns

**Research Goal:** Extract applicable patterns from production CLI-wrapper desktop apps.

#### 3.5.1 GitHub Desktop

Research and document:

- **Architecture**
  - How it invokes Git CLI
  - State management approach
  - IPC patterns between main and renderer

- **CLI Integration**
  - Git command execution patterns
  - Output parsing approaches
  - Error handling and display

- **Relevant Patterns for aiy**
  - Repository-scoped operations
  - Background refresh patterns
  - Conflict resolution UX

#### 3.5.2 Docker Desktop

Research and document:

- **Architecture**
  - Relationship between UI and Docker CLI/daemon
  - State synchronization approaches
  - Platform-specific backends (WSL2, Hyper-V, Lima)

- **CLI Integration**
  - Command execution patterns
  - Long-running process management
  - Resource monitoring approaches

- **Relevant Patterns for aiy**
  - Settings/configuration UX
  - Status/health indicators
  - Extension/plugin architecture (if applicable)

#### 3.5.3 1Password CLI/UI

Research and document:

- **Architecture**
  - Relationship between 1Password app and CLI
  - Authentication/session management
  - Secure IPC patterns

- **Security Patterns**
  - Vault locking mechanisms
  - Biometric authentication integration
  - Clipboard security

- **Relevant Patterns for aiy**
  - Session management
  - Secure data handling
  - Privacy-first UX patterns

#### 3.5.4 Other Notable CLI Wrappers

Research and identify other relevant products:

- **VS Code** (if applicable patterns exist)
- **Postman** (CLI + UI patterns)
- **TablePlus / DBeaver** (CLI database tools)
- **Warp Terminal** (if relevant patterns)
- Any other "desktop wrapper around CLI" tools worth studying

#### 3.5.5 Pattern Applicability Matrix

Create a matrix evaluating which patterns from each product are:
- Directly applicable to aiy
- Applicable with modifications
- Not applicable (and why)
- Applicable but conflict with privacy constraints

---

## 4. Deliverable Specifications

### 4.1 Output Document

**Location:** `docs/research-electron-wrapper.md`

### 4.2 Required Sections

```
# Electron Desktop Wrapper Research Report

## Executive Summary
- 1-2 page overview of key findings
- Top 5 recommendations with confidence levels
- Critical risks and mitigations

## Decision Matrix
- Tabular comparison of options for each major decision point
- Recommended choice with rationale
- Trade-offs clearly stated

## 1. CLI Integration Patterns
[Detailed findings with citations]

## 2. File Watching Strategies
[Detailed findings with citations]

## 3. Packaging & Distribution
[Detailed findings with citations]

## 4. Security/Privacy UX
[Detailed findings with citations]

## 5. Comparable Products Analysis
[Detailed findings with citations]

## Appendix A: Uncertain Claims
- Claims that could not be verified
- Recommendations for local verification

## Appendix B: Source Bibliography
- All sources with access dates
- Categorized by topic

## Appendix C: Glossary
- Technical terms defined
```

### 4.3 Citation Requirements

All factual claims must include:
- Source URL
- Source type (official docs, engineering blog, GitHub issue, etc.)
- Access date or publication date
- Confidence level: `[High]` `[Medium]` `[Low]`

Example:
> Electron's `child_process.spawn` should be preferred over `exec` for long-running processes to avoid buffer overflow issues. [High Confidence]
> *Source: Node.js Official Documentation, "Child Process" (https://nodejs.org/api/child_process.html), accessed 2026-01-16*

### 4.4 Uncertainty Handling

For claims that cannot be verified:
- Clearly mark as `[Unverified]` or `[Needs Local Testing]`
- Explain what verification would look like
- Do not present uncertain claims as facts

---

## 5. Source Priority

### 5.1 Preferred Sources (in order)

1. **Official Documentation**
   - Electron docs (electronjs.org)
   - Node.js docs (nodejs.org)
   - Platform vendor docs (Apple, Microsoft)

2. **Primary Engineering Sources**
   - GitHub repositories (source code, issues, discussions)
   - Official engineering blogs (GitHub Engineering, Docker Engineering)
   - Conference talks with published slides/videos

3. **Reputable Technical Publications**
   - Major engineering blogs (Slack Engineering, Discord Engineering, etc.)
   - Well-maintained community resources
   - Peer-reviewed or widely-cited technical articles

### 5.2 Sources to Approach with Caution

- Stack Overflow (verify against docs)
- Medium articles (verify author credibility)
- Outdated documentation (check version relevance)

### 5.3 Sources to Avoid

- AI-generated content without verification
- Promotional/marketing content
- Unattributed claims

---

## 6. Success Criteria

The research is complete when:

- [ ] All 5 research topics have comprehensive coverage
- [ ] Each major claim has at least one citation
- [ ] Critical claims have 2+ independent sources
- [ ] Decision matrix provides clear recommendations
- [ ] Hard constraints are reflected in all recommendations
- [ ] Uncertain claims are clearly marked
- [ ] Executive summary is actionable
- [ ] Document is saved to `docs/research-electron-wrapper.md`

---

## 7. Timeline & Coordination

### 7.1 Handoff

This PRP is ready for the GPT-5 Pro research team to execute.

### 7.2 Questions & Clarifications

If the research team needs clarification on scope or constraints, the following are authoritative:
- Hard constraints in Section 2 are non-negotiable
- Research topics in Section 3 define minimum coverage
- Deliverable format in Section 4 is required

### 7.3 Completion

Upon completion, save the research document to `docs/research-electron-wrapper.md` and notify the planning team.

---

*PRP Created: 2026-01-16*
*Created By: Claude Planning Team*
*For: GPT-5 Pro Research Team*
