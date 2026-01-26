---
document_type: product-brief
title: "Electron Desktop Wrapper for aiy CLI"
version: "1.0"
date: "2026-01-16"
author: "Claude Planning Team"
status: "draft"
---

# Product Brief: Electron Desktop Wrapper for aiy CLI

## 1. Vision Statement

The aiy CLI orchestrates multiple AI providers for software development workflows, enabling developers to leverage local and cloud-based AI models while maintaining strict privacy boundaries. This Electron wrapper provides a visual interface that surfaces workflow state, agent status, and privacy configuration while preserving the CLI as the sole source of truth. The wrapper enforces "Always Local" privacy mode by default and requires explicit user approval for any cloud operations, making aiy accessible to visual learners and occasional users without compromising the security posture demanded by enterprise environments.

## 2. Target Users (Personas)

### Alex - Security-Conscious Developer
- **Profile:** Works in defense/finance sectors, audits tools before trusting them with sensitive code
- **Critical Needs:** Verification capabilities, audit trail, credential isolation, privacy confidence
- **Success Metric:** Can verify no data leaves machine without explicit approval; can audit all cloud interactions

### Jordan - Power User
- **Profile:** Runs dozens of workflows daily, keyboard-driven, efficiency-focused
- **Critical Needs:** Keyboard shortcuts, queue visibility, quick resume, minimal clicks
- **Success Metric:** Completes common workflows faster than CLI alone; never needs to touch the mouse for routine tasks

### Sam - Occasional User
- **Profile:** Weekly user, forgets context between sessions
- **Critical Needs:** State recovery, context reminder, guided actions
- **Success Metric:** Can resume work without re-reading documentation; understands what was happening when they left off

## 3. Problem Statement

- CLI-only interface creates friction for visual learners and occasional users who struggle to remember commands
- No persistent view of workflow state, logs, or agent status across sessions
- Privacy mode configuration requires remembering CLI commands and flags
- No visual confirmation of what data would be sent to cloud services before transmission
- Users cannot easily verify their privacy posture without running manual CLI checks
- Paused or failed workflows go unnoticed without active monitoring

## 4. Solution Overview

A thin Electron wrapper that:
- Invokes the aiy CLI for all operations (CLI remains single source of truth)
- Captures stdout/stderr to JSONL logs for audit and context recovery
- Watches state files for real-time UI updates via chokidar with polling fallback
- Enforces "Always Local" privacy mode by default (UI-enforced in v1)
- Requires explicit confirmation modal with redacted preview for any cloud operations
- Displays persistent privacy indicator on all screens
- Surfaces paused/failed workflows immediately on launch

The Electron app acts as a "dumb display layer" - it never applies redaction logic or makes privacy decisions. All sensitive operations are delegated to the CLI's `aiy-privacy` crate (PrivacyGuard, Redactor, CloudCommunicator).

## 5. v1 Scope

### In Scope (P0 - Must Have)

| ID | Feature | Description |
|----|---------|-------------|
| P0-1 | Command Queue Infrastructure | Per-repo serialization for workflow/privacy commands; concurrent read-only commands |
| P0-2 | Stdout/Stderr Capture | JSONL logs in `~/.config/aiy-desktop/logs/commands.jsonl` with rotation (50MB max, 5 files) |
| P0-3 | Privacy Mode Screen | Always Local default; Hybrid toggle; Ollama health indicator (green/red/spinner) |
| P0-4 | Global Kill-Switch (Layer 1) | UI-enforced Always Local mode - hides/disables all cloud actions when ON |
| P0-5 | Persistent Privacy Indicator (Layer 3) | Status badge on ALL screens: green shield (Always Local) or yellow warning (Hybrid) |
| P0-6 | Agents Screen | List agents with enabled/disabled status, credential presence; enable/disable actions |
| P0-7 | Credentials Screen | Status per provider (xai, anthropic, google, openai); set/delete with confirmation |
| P0-8 | Workflows Screen | Init, execute, status, resume, cancel lifecycle; JSON parsing for execute/workflow-status |
| P0-9 | Logs Screen | Display JSONL logs; filter by command type, status, time; search; expand for full output |
| P0-10 | Basic Error Handling | Toast on command failure; error panel for history/detail with full stderr |
| P0-11 | Binary Location Resolution | Bundled -> user-specified -> PATH lookup; error if not found |

### In Scope (P1 - Should Have)

| ID | Feature | Description |
|----|---------|-------------|
| P1-1 | Cloud Confirmation Modal (Layer 2) | Show `cloud_payload_preview` from CLI; require explicit "Send to Cloud" confirmation |
| P1-2 | File Watchers | Watch `.aiy/workflow-state.json` and `.aiy/config.toml`; debounce 100ms; 30s polling fallback |
| P1-3 | Workflow Context Display | Show original request (from JSONL logs + in-memory state); fallback: "Request context unavailable" |
| P1-4 | Dashboard Attention Cards | Surface paused/failed workflows at top on launch; one-click resume/view error |
| P1-5 | Keyboard Shortcuts | `Cmd+K` palette, `Cmd+1-5` screens, `Cmd+Shift+E/R/S` for execute/resume/status |
| P1-6 | Verify Privacy Button | Runs `aiy privacy check`; shows raw CLI output; highlights [PASS]/[FAIL] |
| P1-7 | Stage Indicator | Visual progress (Init -> Executing -> Finalizing) instead of just spinner |

### Out of Scope (v1)

- CLI streaming protocol changes
- CLI-level privacy kill-switch (tracked as P2-8 for future CLI work)
- New JSON output format for CLI commands that currently output text
- Credential hiding in Always Local mode (P2-5)
- Command queue visibility panel (P2-1)
- Auto-update mechanism (to be determined)
- Plugin/extension system
- Customizable keyboard shortcuts (P2-6)
- Cloud call audit counter (P2-7)

## 6. Success Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| Privacy compliance | 100% | No data sent to cloud without explicit user approval in confirmation modal |
| Workflow completion | 90%+ | Users complete initiated workflows vs abandon (tracked via local JSONL logs) |
| Error recoverability | 80%+ | Users successfully recover from error states without needing CLI fallback |
| Power user efficiency | <3 clicks | Common actions (execute, resume, cancel) accessible via keyboard or <3 clicks |
| Context retention | 100% | Paused/failed workflows visible on next launch (via attention cards) |

## 7. Privacy Boundary (Hard Constraint)

**Non-negotiable rules:**
- NO code, paths, diffs, stack traces, or dependencies to cloud - ever
- Cloud planning ONLY with redacted metadata AND explicit per-action user approval
- Zero telemetry, analytics, or crash reporting by default
- All state, logs, and configuration stored locally only
- CLI is source of truth; Electron is a dumb display layer
- Electron never applies redaction logic - just displays `cloud_payload_preview` from CLI

**Enforcement Layers:**

```
Layer 1: Global Kill-Switch ("Always Local" toggle)
         When ON -> Electron hides/disables all cloud actions
         v1: UI enforcement only; CLI enforcement is future work (P2-8)

Layer 2: Per-Action Gate (when Hybrid enabled)
         Modal with cloud_payload_preview -> explicit "Send to Cloud" confirm
         No "always allow" option to prevent complacency

Layer 3: Persistent Visual Indicator
         Status badge: Green/shield = Always Local, Yellow/warning = Hybrid
         Visible on ALL screens; click opens Privacy Mode screen
```

**Default Posture:**
- Privacy mode ON = Always Local (enforced by Electron on first-run and when enabling privacy mode)
- CLI does not currently expose a persistent `privacy.mode` flag; CLI-level default/enforcement is future work (P2-8)

## 8. Risks & Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| CLI lacks JSON output for most commands | High | Parse text output; schedule CLI enhancements for future versions |
| Privacy check exit code unreliable (`aiy privacy check` exits 0 even on [FAIL]) | Medium | Parse [PASS]/[FAIL] markers in output instead of relying on exit code |
| `workflow-state.json` lacks request context | Medium | Extract from JSONL logs (P0-2) + in-memory state; graceful fallback message |
| File watchers fail silently | Medium | Fallback to 30s polling (P1-2); periodic reconciliation; manual refresh button |
| Orphaned CLI processes | High | Track all spawned PIDs; kill process tree on app exit; use `ps-tree` or `taskkill /T` |
| Cross-platform behavior differences | High | Platform-specific code paths (SIGTERM vs taskkill); extensive testing on macOS/Windows/Linux |
| Security regressions from bundling | Medium | Sign CLI binary with same certificate as app; include in macOS notarization |
| User experience vs privacy conflict | High | Transparent logging; user-controlled sharing; no "always allow" for cloud actions |

## 9. Dependencies

**Runtime Dependencies:**
- Rust CLI (`aiy`) with current command surface
- Electron framework (latest LTS)
- chokidar v4 for file watching
- Zustand for lightweight state management

**Build Dependencies:**
- electron-builder for packaging
- Code signing certificates (macOS + Windows)
- Cross-platform CI/CD for building three binaries (win, mac, linux)

**CLI Commands Required:**

| Screen | Commands | Output Format |
|--------|----------|---------------|
| Agents | `list`, `status`, `enable`, `disable` | Text (parse) |
| Credentials | `status`, `set`, `get`, `delete` | Text (parse) |
| Privacy Mode | `check`, `enable`, `disable`, `config show/get/set/reset` | Text ([PASS]/[FAIL] markers) |
| Workflows | `init`, `execute`, `workflow-status`, `resume`, `cancel` | JSON (execute, workflow-status only) |

## 10. Open Questions

1. Should v1 include auto-update mechanism, or manual updates only? (Research recommends unified App+CLI updates for simplicity)
2. What is the minimum CLI version required for Electron wrapper compatibility?
3. Should we expose the bundled CLI to user's PATH (opt-in installation)?
4. What is the appropriate timeout for long-running CLI operations? (Research suggests command-type specific: 60s for planning, 30s for patches)
5. Should we implement atomic writes in CLI for `workflow-state.json` to avoid partial read issues?

## 11. References

- Research: `docs/research-electron-wrapper.md`
- Brainstorming: `_bmad-output/analysis/brainstorming-session-2026-01-16.md`
- PRP: `_bmad-output/planning-artifacts/research/PRP-electron-wrapper-research.md`
