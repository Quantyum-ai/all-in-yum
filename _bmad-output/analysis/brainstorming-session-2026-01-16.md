---
stepsCompleted: [1, 2, 3, 4]
inputDocuments: []
session_topic: 'Electron desktop app wrapping aiy CLI'
session_goals: 'UI/UX patterns, CLI integration architecture, privacy mode UX, workflow lifecycle visualization'
selected_approach: 'ai-recommended'
techniques_used: ['Constraint Mapping', 'Morphological Analysis', 'Role Playing']
ideas_generated: [26]
context_file: ''
status: 'complete'
last_updated: '2026-01-16'
---

# Brainstorming Session Results

**Facilitator:** airpln
**Date:** 2026-01-16

## Session Overview

**Topic:** Electron desktop app wrapping existing `aiy` CLI (greenfield UI, CLI stays source of truth)

**Goals:**
- UI layout & navigation patterns for 5 core screens
- CLI integration architecture (IPC, output parsing, error handling)
- Privacy mode UX (Ollama health indicators, config surfaces)
- Workflow lifecycle visualization (init → execute → status → resume → cancel)
- State management between UI and CLI-as-source-of-truth

**Hard Constraints:**
- Zero code/paths/diffs/stack traces/deps to cloud
- Closed-source models may do planning only with redacted metadata
- Local Ollama executes all sensitive operations
- Integration via shell out to `./target/release/aiy` + parse `--format json`

**Non-goals (v1):** Rewriting adapters, forcing OAuth, building cloud broker

---

## Technique Selection

**Approach:** AI-Recommended Techniques
**Analysis Context:** Technical product design with hard privacy constraints

**Recommended Techniques:**

1. **Constraint Mapping (Deep):** Map all constraints - real vs imagined - to find pathways and establish architectural foundation
2. **Morphological Analysis (Deep):** Systematically explore parameter combinations across 5 core screens for optimal feature/implementation pairings
3. **Role Playing (Collaborative):** Embody developer, security-conscious user, and workflow operator perspectives to stress-test UX decisions

**AI Rationale:** Privacy constraint is architectural linchpin requiring explicit mapping before ideation. Morphological analysis ensures systematic coverage of multi-screen complexity. Role playing validates against real-world usage patterns.

---

## Phase 1: Constraint Mapping Results

### Privacy Architecture

- **Redaction Layer:** CLI owns via `aiy-privacy` crate (PrivacyGuard, Redactor, CloudCommunicator)
- **Electron Role:** Dumb display - shows `cloud_payload_preview` from CLI, never applies redaction logic
- **Default Posture:** Privacy mode ON = Always Local; Hybrid = explicit opt-in + per-run approval
  - **v1 Note:** This default is enforced by Electron (on first-run and when enabling privacy mode). The CLI does not currently expose a persistent `privacy.mode` flag; CLI-level default/enforcement is future work (P2-8).

### Integration Surface (CLI Commands)

| Screen | Commands | JSON Support |
|--------|----------|--------------|
| Agents | `list`, `status`, `enable`, `disable` | ❌ (parse text) |
| Credentials | `status`, `set`, `get`, `delete` | ❌ (parse text) |
| Privacy Mode | `check`, `enable`, `disable`, `config show/get/set/reset` | ❌ (`check` has [PASS]/[FAIL] markers) |
| Workflows | `init`, `execute`, `workflow-status`, `resume`, `cancel` | ✅ `execute`, `workflow-status` only |
| Logs | (none - Electron captures stdout/stderr → JSONL) | N/A |

### Key Constraints

- Serialize workflow/privacy commands per-repo; read-only commands can parallelize
- `aiy privacy check` exits 0 even on [FAIL] - must parse output
- Binary: support bundled + PATH lookup + user-specified
- File watchers needed: `.aiy/workflow-state.json`, `.aiy/config.toml`

---

## Phase 2: Morphological Analysis Results

### Selected Architecture

| Dimension | Choice | Rationale |
|-----------|--------|-----------|
| **Navigation** | Hybrid (sidebar + command palette) | Persistent nav for 5 screens + power-user actions |
| **CLI Integration** | Command Queue | Clean serialization per-repo, priority for read-only |
| **State Management** | Zustand | Lightweight cache, minimal ceremony |
| **Polling/Watching** | Hybrid (file watch + interval fallback) | Instant updates from file changes, 30s fallback |
| **Error Handling** | Hybrid (toast + error panel) | Immediate feedback + persistent history |
| **Long-running Feedback** | Hybrid (stage indicator + expandable log) | Honest progress + transparency for power users |
| **Security/Privacy UX** | Layered Defense | Kill-switch + modal gate + persistent indicator |

### Security/Privacy Enforcement Layers

```
Layer 1: Global Kill-Switch ("Always Local" toggle) [v1: UI-enforced]
         When ON → Electron hides/disables all cloud actions
         (v1: UI enforcement only; CLI enforcement is future work)

Layer 2: Per-Action Gate (when Hybrid enabled)
         Modal with cloud_payload_preview → explicit "Send to Cloud" confirm

Layer 3: Persistent Visual Indicator
         Status badge: Green/shield = Always Local, Yellow/warning = Hybrid enabled
```

---

## Phase 3: Role Playing Results

### Personas Evaluated

| Persona | Profile | Critical Needs |
|---------|---------|----------------|
| **Alex (Security)** | Defense/finance, audits before trusting | Verification, audit trail, credential isolation |
| **Jordan (Power)** | Dozens of workflows/day, keyboard-driven | Shortcuts, queue visibility, quick resume |
| **Sam (Occasional)** | Weekly user, forgets between sessions | State recovery, context reminder, guided actions |

### Cross-Persona Insights

- **Workflow cards need context:** Show original request, not just state
- **Error messages need actionability:** Not just "failed" but "try this"
- **Privacy verification should be accessible:** Even skeptics can verify
- **Attention on launch:** Surface paused/failed workflows immediately

---

## Prioritized Action List (v1 Scope)

**Scope Reminder:**
- Electron wrapper + capture stdout/stderr → JSONL logs
- No new CLI streaming protocol
- Hard constraint: no code/paths/diffs/traces/deps to cloud
- Always Local = safe default when privacy mode ON
  - **Note:** v1 default is enforced by Electron app (first-run + when enabling privacy mode); CLI does not currently expose a persistent `privacy.mode` flag. CLI-level enforcement is future work (P2-8).

---

### P0: Must Have (Core Functionality + Hard Constraints)

#### P0-1: Command Queue Infrastructure

| Attribute | Value |
|-----------|-------|
| **Description** | Implement command queue in Electron main process with per-repo serialization for workflow/privacy commands; allow concurrent read-only commands |
| **Personas** | All |
| **CLI Dependencies** | All commands; queue manages invocation |
| **Acceptance Criteria** | ✅ Workflow commands serialized per-repo (no race on `.aiy/workflow-state.json`) ✅ Read-only commands (`agents list`, `credentials status`) execute concurrently ✅ Queue handles command failure gracefully (doesn't block subsequent) |

---

#### P0-2: Stdout/Stderr Capture → JSONL Logs

| Attribute | Value |
|-----------|-------|
| **Description** | Capture all CLI stdout/stderr per command invocation; store as JSONL in app data directory with timestamp, command, exit code, output |
| **Personas** | Jordan, Alex |
| **CLI Dependencies** | All commands (passthrough capture) |
| **Files** | Write to `~/.config/aiy-desktop/logs/commands.jsonl` |
| **Acceptance Criteria** | ✅ Every CLI invocation logged with: `{timestamp, command, args, exitCode, stdout, stderr, durationMs}` ✅ Logs persist across sessions ✅ Log rotation (e.g., 50MB max, 5 files) |

---

#### P0-3: Privacy Mode Screen (Always Local Default)

| Attribute | Value |
|-----------|-------|
| **Description** | Privacy Mode screen with Always Local as default; toggle to enable Hybrid mode; Ollama health indicator |
| **Personas** | Alex (critical), All |
| **CLI Dependencies** | `aiy privacy check` (parse [PASS]/[FAIL]), `aiy privacy enable/disable`, `aiy privacy config show` |
| **Acceptance Criteria** | ✅ Always Local = ON by default when privacy mode enabled (enforced by Electron on first-run and when enabling privacy mode; CLI has no persistent `privacy.mode` flag today—see P2-8) ✅ Ollama health shows: reachable (green), unreachable (red), checking (spinner) ✅ Parse `aiy privacy check` output for [PASS]/[FAIL] markers (ignore exit code) ✅ Hybrid toggle requires explicit user action (not accidental) |

---

#### P0-4: Layer 1 - Global Kill-Switch (UI-Enforced)

| Attribute | Value |
|-----------|-------|
| **Description** | When Always Local ON: Electron hides/disables all cloud-related UI options and refuses to invoke any cloud-planning commands. **v1 is UI-only enforcement; CLI-level enforcement is future work (see P2-8).** |
| **Personas** | Alex (critical) |
| **CLI Dependencies** | None for v1 (UI reads mode from Electron local storage; CLI has no `cloud_enabled` config key today) |
| **Acceptance Criteria** | ✅ No "Send to Cloud" buttons visible when Always Local ON ✅ Electron command queue refuses to enqueue cloud-planning commands when Always Local ✅ Toggle state persists in Electron local storage across app restarts ✅ Privacy indicator (P0-5) reflects current mode |
| **Note** | Defense-in-depth at CLI layer is future work (P2-8) |

---

#### P0-5: Layer 3 - Persistent Privacy Indicator

| Attribute | Value |
|-----------|-------|
| **Description** | Status badge in header/sidebar showing current privacy mode: Always Local (green/shield icon) or Hybrid (yellow/warning icon) |
| **Personas** | Alex, Sam |
| **CLI Dependencies** | `aiy privacy config show` (read current state) |
| **Acceptance Criteria** | ✅ Indicator visible on ALL screens (never hidden) ✅ Always Local = green shield, Hybrid = yellow warning ✅ Click opens Privacy Mode screen |

---

#### P0-6: Agents Screen

| Attribute | Value |
|-----------|-------|
| **Description** | List all agents with enabled/disabled status and credential presence; enable/disable actions |
| **Personas** | All |
| **CLI Dependencies** | `aiy agents list`, `aiy agents status`, `aiy agents enable <id>`, `aiy agents disable <id>` |
| **Acceptance Criteria** | ✅ Table/list showing: agent ID, enabled status, credentials present/missing ✅ Toggle to enable/disable each agent ✅ Link to Credentials screen if credentials missing ✅ Parse text output (no JSON) |

---

#### P0-7: Credentials Screen

| Attribute | Value |
|-----------|-------|
| **Description** | Show credential status per provider; set/delete credentials |
| **Personas** | All |
| **CLI Dependencies** | `aiy credentials status`, `aiy credentials set <provider>`, `aiy credentials delete <provider>` |
| **Providers** | xai, anthropic, google, openai |
| **Acceptance Criteria** | ✅ List providers with status: configured (masked) / not configured ✅ Set credential via secure input (not shown in logs) ✅ Delete with confirmation ✅ Parse text output (no JSON) |

---

#### P0-8: Workflows Screen (Core Lifecycle)

| Attribute | Value |
|-----------|-------|
| **Description** | Init, execute, view status, resume, cancel workflows |
| **Personas** | Jordan (critical), All |
| **CLI Dependencies** | `aiy privacy init`, `aiy privacy execute --format json`, `aiy privacy workflow-status --format json`, `aiy privacy resume`, `aiy privacy cancel` |
| **Files** | Watch `.aiy/workflow-state.json` |
| **Acceptance Criteria** | ✅ Init button (disabled if already initialized) ✅ Execute with request input field ✅ Status display: Ready/Executing/Completed/Failed/Paused ✅ Resume button (enabled only when Paused) ✅ Cancel button (enabled only when Executing) ✅ JSON parsing for `execute` and `workflow-status` |

---

#### P0-9: Logs Screen

| Attribute | Value |
|-----------|-------|
| **Description** | Display captured JSONL command logs; filter by command type, status, time |
| **Personas** | Jordan, Alex |
| **CLI Dependencies** | None (reads local JSONL) |
| **Files** | Read from `~/.config/aiy-desktop/logs/commands.jsonl` |
| **Acceptance Criteria** | ✅ List view with: timestamp, command, exit code, duration ✅ Expand to see full stdout/stderr ✅ Filter by: success/failure, command type, date range ✅ Search within logs |

---

#### P0-10: Basic Error Handling (Toast + Panel)

| Attribute | Value |
|-----------|-------|
| **Description** | Toast notification on command failure; error panel for history and detail |
| **Personas** | All |
| **CLI Dependencies** | All (capture stderr, exit codes) |
| **Acceptance Criteria** | ✅ Toast appears on non-zero exit code ✅ Toast shows brief error message ✅ Click toast opens error panel ✅ Error panel shows full stderr, command that failed |

---

#### P0-11: Binary Location Resolution

| Attribute | Value |
|-----------|-------|
| **Description** | Resolve CLI binary location: bundled → user-specified → PATH lookup |
| **Personas** | All |
| **CLI Dependencies** | Binary at resolved path |
| **Acceptance Criteria** | ✅ Check for bundled binary in app resources first ✅ Settings option for user-specified path ✅ Fallback to PATH lookup (`which aiy`) ✅ Error if no binary found |

---

### P1: Should Have (Significantly Improves UX)

#### P1-1: Layer 2 - Cloud Confirmation Modal

| Attribute | Value |
|-----------|-------|
| **Description** | When Hybrid enabled and cloud action triggered: show modal with `cloud_payload_preview` from CLI; require explicit "Send to Cloud" confirmation |
| **Personas** | Alex (critical) |
| **CLI Dependencies** | `aiy privacy execute --format json` must include `cloud_payload_preview` field (NEW CLI WORK if not present) |
| **Acceptance Criteria** | ✅ Modal shows exactly what will be sent (redacted preview) ✅ User must click "Send to Cloud" to proceed ✅ "Cancel" returns to previous state ✅ Preview is read-only (user cannot edit) |

---

#### P1-2: File Watchers for State Changes

| Attribute | Value |
|-----------|-------|
| **Description** | Watch `.aiy/workflow-state.json` and `.aiy/config.toml` for external changes; update UI immediately |
| **Personas** | Jordan, Sam |
| **Files** | `.aiy/workflow-state.json`, `.aiy/config.toml` |
| **Acceptance Criteria** | ✅ UI updates within 1s of file change ✅ Debounce rapid changes (100ms) ✅ Handle file deletion gracefully ✅ Fallback to 30s polling if watcher fails |

---

#### P1-3: Workflow Context Display

| Attribute | Value |
|-----------|-------|
| **Description** | Show original request text alongside workflow status, not just state enum. **Note:** `.aiy/workflow-state.json` is intentionally metadata-only and does NOT contain the original request. |
| **Personas** | Sam (critical), Jordan |
| **CLI Dependencies** | None for v1 (CLI does not expose request in `workflow-status` output today) |
| **v1 Data Source** | Electron's JSONL command logs (P0-2): parse the most recent `aiy privacy execute` invocation's args/stdout to extract request context. Also use in-memory UI state for active session. |
| **Acceptance Criteria** | ✅ Workflow card shows: state + last known request (from JSONL log or in-memory, truncated with expand) ✅ Full request visible on detail view ✅ Timestamp of last state change ✅ Graceful fallback if no request found in logs ("Request context unavailable") |
| **Future CLI Work** | Optional: Add `request_summary` field (redacted + truncated, non-sensitive) to `aiy privacy workflow-status --format json` output. NOT persisted to state file. |

---

#### P1-4: Dashboard Attention Cards

| Attribute | Value |
|-----------|-------|
| **Description** | On app launch, surface paused/failed workflows prominently at top of dashboard |
| **Personas** | Sam (critical) |
| **CLI Dependencies** | `aiy privacy workflow-status --format json` |
| **Acceptance Criteria** | ✅ "Attention needed" section at top when Paused or Failed workflows exist ✅ One-click to Resume (Paused) or view error (Failed) ✅ Dismissible but re-appears on next launch if still applicable |

---

#### P1-5: Keyboard Shortcuts for Common Actions

| Attribute | Value |
|-----------|-------|
| **Description** | Global keyboard shortcuts for workflow operations and navigation |
| **Personas** | Jordan (critical) |
| **Shortcuts** | `Cmd+K` = command palette, `Cmd+1-5` = switch screens, `Cmd+Shift+E` = execute, `Cmd+Shift+R` = resume, `Cmd+Shift+S` = status |
| **Acceptance Criteria** | ✅ Shortcuts work from any screen ✅ Shortcuts visible in command palette ✅ Shortcuts customizable in settings (P2) |

---

#### P1-6: Verify Privacy Button

| Attribute | Value |
|-----------|-------|
| **Description** | Button on Privacy Mode screen that runs `aiy privacy check` and shows raw output |
| **Personas** | Alex (critical) |
| **CLI Dependencies** | `aiy privacy check` |
| **Acceptance Criteria** | ✅ "Verify Privacy" button runs check ✅ Shows raw CLI output (not just parsed summary) ✅ Highlights [PASS] green, [FAIL] red ✅ Alex can verify UI matches CLI output |

---

#### P1-7: Stage Indicator for Long-Running Ops

| Attribute | Value |
|-----------|-------|
| **Description** | Show workflow stage (Init → Executing → Finalizing) instead of just spinner |
| **Personas** | All |
| **CLI Dependencies** | `aiy privacy workflow-status --format json` (state field) |
| **Acceptance Criteria** | ✅ Visual stage indicator (steps/progress dots) ✅ Maps to workflow state: Ready → Executing → Completed/Failed ✅ "Expand output" shows live captured stdout |

---

### P2: Nice to Have (Polish)

#### P2-1: Command Queue Visibility

| Attribute | Value |
|-----------|-------|
| **Description** | Collapsible panel showing pending/running commands in queue |
| **Personas** | Jordan |
| **Acceptance Criteria** | ✅ See queue depth ✅ See currently executing command ✅ Option to cancel pending (not running) commands |

---

#### P2-2: Copy to Clipboard on Log Entries

| Attribute | Value |
|-----------|-------|
| **Description** | One-click copy for any log entry (stdout, stderr, or full JSON) |
| **Personas** | Jordan |
| **Acceptance Criteria** | ✅ Copy icon on every log entry ✅ Copies formatted text ✅ Toast confirms "Copied" |

---

#### P2-3: Suggested Actions in Errors

| Attribute | Value |
|-----------|-------|
| **Description** | Error panel includes "Suggested fix" based on common error patterns |
| **Personas** | Sam (critical) |
| **Error Patterns** | "workflow-state.json not found" → "Run `aiy privacy init` first" |
| **Acceptance Criteria** | ✅ Pattern matching on stderr ✅ Actionable suggestion shown ✅ Click suggestion runs recommended command |

---

#### P2-4: Tooltips for Occasional Users

| Attribute | Value |
|-----------|-------|
| **Description** | "What's this?" tooltips on major UI elements explaining purpose |
| **Personas** | Sam |
| **Acceptance Criteria** | ✅ Hover or (?) icon shows tooltip ✅ Covers: all screens, privacy modes, workflow states ✅ Dismissible "don't show again" per tooltip |

---

#### P2-5: Credential Hiding in Always Local Mode

| Attribute | Value |
|-----------|-------|
| **Description** | When Always Local ON, hide cloud provider credentials from UI entirely (reduce temptation) |
| **Personas** | Alex |
| **Acceptance Criteria** | ✅ Credentials screen hides cloud providers when Always Local ✅ Only shows local/Ollama config ✅ Credentials still exist (not deleted), just hidden |

---

#### P2-6: Customizable Keyboard Shortcuts

| Attribute | Value |
|-----------|-------|
| **Description** | Settings screen to customize all keyboard shortcuts |
| **Personas** | Jordan |
| **Acceptance Criteria** | ✅ List all shortcuts ✅ Click to rebind ✅ Conflict detection ✅ Reset to defaults |

---

#### P2-7: Cloud Call Audit Counter

| Attribute | Value |
|-----------|-------|
| **Description** | "0 cloud calls in last 30 days" counter on Privacy Mode screen for peace of mind |
| **Personas** | Alex |
| **CLI Dependencies** | Parsed from JSONL logs (filter cloud-related commands) |
| **Acceptance Criteria** | ✅ Count of cloud calls (if any) in time period ✅ Click to see list of cloud calls ✅ Reassurance when count = 0 |

---

#### P2-8: CLI-Level Cloud Planning Kill-Switch (Future CLI Work)

| Attribute | Value |
|-----------|-------|
| **Description** | Add CLI config flag to enforce "Always Local" at CLI layer (defense-in-depth). When enabled, CLI refuses cloud-planning commands regardless of how invoked. |
| **Personas** | Alex (critical for enterprise/compliance) |
| **Proposed Config** | `privacy.mode = "always_local" | "hybrid"` OR `privacy.cloud_planning_enabled = false` in `.aiy/config.toml` or global config |
| **CLI Changes Required** | Add config key parsing; reject cloud-planning commands with clear error when `always_local` mode active |
| **Acceptance Criteria** | ✅ `aiy privacy config set mode always_local` (or equivalent) sets the flag ✅ `aiy privacy execute` with cloud intent returns error: "Cloud planning disabled (always_local mode)" ✅ Flag readable via `aiy privacy config show` ✅ Electron UI can read flag to sync state (optional) |
| **Rationale** | Provides defense-in-depth: even if UI is bypassed (direct CLI usage), CLI enforces policy. Critical for compliance/audit scenarios. |

---

## Summary

| Priority | Count | Focus |
|----------|-------|-------|
| **P0** | 11 | Core screens, command queue, privacy enforcement (UI-layer), basic UX |
| **P1** | 7 | Cloud confirmation modal, file watchers, workflow context, keyboard shortcuts, verification |
| **P2** | 8 | Queue visibility, copy, suggestions, tooltips, credential hiding, customization, CLI kill-switch |

**Hard Constraints Preserved:**
- ✅ No code/paths/diffs/traces/deps to cloud (CLI enforces via `aiy-privacy` crate)
- ✅ Always Local = default when privacy mode ON
  - **v1:** Enforced by Electron app (first-run + when enabling privacy mode)
  - **Why:** CLI does not currently expose a persistent `privacy.mode=always_local|hybrid` flag
  - **Future:** CLI-level default/enforcement tracked in P2-8
- ✅ Layered defense: kill-switch (P0-4, UI-only in v1) + modal gate (P1-1) + indicator (P0-5)
- ✅ Electron stays "dumb" - never applies redaction, just displays CLI output
- ✅ `aiy privacy check` must be parsed for [PASS]/[FAIL] markers (exit code unreliable)
- ✅ No new CLI streaming protocol for v1; Electron captures stdout/stderr → JSONL

---
