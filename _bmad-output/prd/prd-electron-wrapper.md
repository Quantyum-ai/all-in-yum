---
document_type: prd
title: "PRD: Electron Desktop Wrapper for aiy CLI"
version: "1.0"
date: "2026-01-16"
author: "Claude Planning Team"
status: "draft"
---

# PRD: Electron Desktop Wrapper for aiy CLI

## 1. Overview

### 1.1 Purpose

This PRD defines the requirements for an Electron desktop application that wraps the existing `aiy` CLI tool, providing a graphical interface for developers to manage AI-assisted workflows while preserving strict privacy guarantees.

### 1.2 Background

- The `aiy` CLI orchestrates AI providers for development workflows
- CLI is the source of truth; Electron is a visual wrapper only
- **Hard constraint:** No code/paths/diffs/stack traces/dependencies to cloud without explicit user approval
- Closed-source models may do planning only with redacted metadata
- Local Ollama executes all sensitive operations
- Integration via `spawn` of resolved `aiy` binary + parse output (text/JSON)

### 1.3 Scope

This PRD covers v1 functionality (P0 + P1 items from brainstorming). The Electron app acts as a "dumb" display layer that invokes the CLI and presents its output. It never applies redaction logic itself.

### 1.4 Target Users

| Persona | Profile | Critical Needs |
|---------|---------|----------------|
| **Alex (Security)** | Defense/finance, audits before trusting | Verification, audit trail, credential isolation |
| **Jordan (Power)** | Dozens of workflows/day, keyboard-driven | Shortcuts, queue visibility, quick resume |
| **Sam (Occasional)** | Weekly user, forgets between sessions | State recovery, context reminder, guided actions |

---

## 2. Functional Requirements (P0 - Must Have)

### 2.1 Command Queue Infrastructure (P0-1)

**Description:** Implement command queue in Electron main process with per-repo serialization for workflow/privacy commands; allow concurrent read-only commands.

**Requirements:**

- REQ-2.1.1: Serialize workflow/privacy commands per-repo to prevent race conditions on `.aiy/workflow-state.json`
- REQ-2.1.2: Allow concurrent execution of read-only commands (`aiy agents list`, `aiy credentials status`)
- REQ-2.1.3: Handle command failure gracefully (failed command does not block subsequent commands in queue)
- REQ-2.1.4: Support cancellation via SIGTERM with process tree cleanup (use `ps-tree` on Unix, `taskkill /PID /T` on Windows)
- REQ-2.1.5: Implement timeout handling with user-configurable defaults (suggest: 60s for planning, 30s for other operations)
- REQ-2.1.6: Track all spawned processes and kill them on app exit to prevent orphaned processes

**Acceptance Criteria:**

- [ ] Workflow commands for same repo never run concurrently
- [ ] Read-only commands execute in parallel without blocking
- [ ] Failed command does not block queue; subsequent commands proceed
- [ ] Cancel button sends SIGTERM and kills child processes
- [ ] Timeout triggers graceful termination with user notification

---

### 2.2 Stdout/Stderr Capture to JSONL (P0-2)

**Description:** Capture all CLI output and store as structured logs for debugging and context retrieval.

**Requirements:**

- REQ-2.2.1: Log schema: `{timestamp, command, args, exitCode, stdout, stderr, durationMs}`
- REQ-2.2.2: Store logs in `~/.config/aiy-desktop/logs/commands.jsonl`
- REQ-2.2.3: Implement log rotation (50MB max file size, keep 5 files)
- REQ-2.2.4: **NEVER** log credentials, secrets, API keys, or actual code content
- REQ-2.2.5: Persist logs across app sessions
- REQ-2.2.6: Redact known sensitive patterns (API key formats, password fields) before writing to log

**Acceptance Criteria:**

- [ ] Every CLI invocation creates a log entry with required schema fields
- [ ] Logs survive app restart
- [ ] Old logs rotate when 50MB size limit reached (5 file maximum)
- [ ] Credentials passed to CLI are masked as `[REDACTED]` in logs

---

### 2.3 Privacy Mode Screen (P0-3)

**Description:** Privacy Mode screen with Always Local as the default mode, providing visibility into privacy configuration and Ollama health.

**Requirements:**

- REQ-2.3.1: **Always Local = ON by default** (Electron-enforced on first-run; stored in Electron local storage)
- REQ-2.3.2: Toggle to enable Hybrid mode requires explicit user action (not accidental)
- REQ-2.3.3: Ollama health indicator: green (reachable), red (unreachable), spinner (checking)
- REQ-2.3.4: **Parse `aiy privacy check` output for `[PASS]`/`[FAIL]` markers; DO NOT rely on exit code** (exit code is 0 even on failures)
- REQ-2.3.5: Display current configuration from `aiy privacy config show` (parse text output)
- REQ-2.3.6: Show last privacy check timestamp and result

**CLI Output Gap Note:** `aiy privacy check/enable/disable/config` commands output text only (no JSON). Parser must extract status from text markers.

**Acceptance Criteria:**

- [ ] First app launch defaults to Always Local mode (persisted in Electron storage)
- [ ] Privacy check correctly identifies `[PASS]` vs `[FAIL]` regardless of exit code
- [ ] Ollama health indicator refreshes on screen load and every 30 seconds
- [ ] Hybrid mode toggle requires confirmation dialog

---

### 2.4 Global Kill-Switch - Layer 1 (P0-4)

**Description:** When Always Local is ON, Electron prevents all cloud operations at the UI layer.

**Requirements:**

- REQ-2.4.1: Hide/disable all "Send to Cloud" UI elements when Always Local mode is active
- REQ-2.4.2: Command queue refuses to enqueue cloud-planning commands when Always Local is active
- REQ-2.4.3: Toggle state persists in Electron local storage across app restarts
- REQ-2.4.4: Privacy indicator (P0-5) reflects current mode at all times

**Important Note:** V1 enforcement is Electron UI-only. The CLI does not currently expose a `privacy.mode` config flag. CLI-level enforcement is future work (P2-8).

**Acceptance Criteria:**

- [ ] No cloud buttons visible when Always Local mode is ON
- [ ] Mode persists across app restarts (stored in Electron local storage)
- [ ] Attempting cloud action via code path is blocked with clear error message
- [ ] Mode state is readable from any screen in the application

---

### 2.5 Persistent Privacy Indicator - Layer 3 (P0-5)

**Description:** Status badge showing current privacy mode on ALL screens, providing constant visibility.

**Requirements:**

- REQ-2.5.1: Indicator always visible in header/sidebar (never hidden)
- REQ-2.5.2: Always Local mode = green shield icon with "Local" label
- REQ-2.5.3: Hybrid mode = yellow warning icon with "Hybrid" label
- REQ-2.5.4: Click on indicator navigates to Privacy Mode screen
- REQ-2.5.5: Tooltip on hover explains current mode

**Acceptance Criteria:**

- [ ] Indicator visible on every screen (Agents, Credentials, Workflows, Privacy, Logs)
- [ ] Color and icon match current mode accurately
- [ ] Click opens Privacy Mode screen from any location

---

### 2.6 Agents Screen (P0-6)

**Description:** List and manage AI agents with enable/disable controls.

**Requirements:**

- REQ-2.6.1: Display agent ID, enabled status, and credentials presence for each agent
- REQ-2.6.2: Toggle to enable/disable each agent via `aiy agents enable <id>` / `aiy agents disable <id>`
- REQ-2.6.3: Link to Credentials screen if credentials are missing for an agent
- REQ-2.6.4: **Parse text output from `aiy agents list` and `aiy agents status`** (NO JSON output available)
- REQ-2.6.5: Refresh agent list on screen load and after enable/disable actions

**CLI Output Gap Note:** Agent commands have NO JSON output. Parser must extract agent information from text output format.

**Acceptance Criteria:**

- [ ] All agents listed with correct enabled/disabled status
- [ ] Enable/disable toggle works and updates UI immediately
- [ ] Missing credentials show link to Credentials screen
- [ ] Text output from CLI is parsed correctly for all known agent types

---

### 2.7 Credentials Screen (P0-7)

**Description:** Manage provider credentials with secure input handling.

**Requirements:**

- REQ-2.7.1: List providers: `xai`, `anthropic`, `google`, `openai`
- REQ-2.7.2: Show status per provider: configured (value masked as `********`) / not configured
- REQ-2.7.3: Set credential via secure input field (password type, not logged anywhere)
- REQ-2.7.4: Delete credential with confirmation dialog ("Are you sure you want to delete X credential?")
- REQ-2.7.5: **Parse text output from `aiy credentials status/set/delete`** (NO JSON available)
- REQ-2.7.6: Credential values are NEVER written to JSONL logs

**CLI Output Gap Note:** Credentials commands have NO JSON output. Parser must extract status from text format.

**Acceptance Criteria:**

- [ ] All providers shown with correct configuration status
- [ ] Credential input field is password-masked and value is not visible in logs
- [ ] Delete requires explicit confirmation before executing
- [ ] Text output parsed correctly for status/success/error states

---

### 2.8 Workflows Screen (P0-8)

**Description:** Core workflow lifecycle management: init, execute, status, resume, cancel.

**Requirements:**

- REQ-2.8.1: Init button runs `aiy privacy init` (disabled if `.aiy/workflow-state.json` exists)
- REQ-2.8.2: Execute with request input field (multi-line text area)
- REQ-2.8.3: Status display showing state: Ready/Executing/Completed/Failed/Paused
- REQ-2.8.4: Resume button runs `aiy privacy resume` (enabled only when state is Paused)
- REQ-2.8.5: Cancel button runs `aiy privacy cancel` (enabled only when state is Executing)
- REQ-2.8.6: Parse JSON from `aiy privacy execute --format json` and `aiy privacy workflow-status --format json`
- REQ-2.8.7: Show execution duration and timestamps

**Note on workflow-state.json:** The file is metadata-only (state, timestamps). It does NOT contain the original request text. Request context must be retrieved from JSONL logs or in-memory state (see P1-3).

**Acceptance Criteria:**

- [ ] Init button correctly detects existing workflow state
- [ ] All workflow states (Ready/Executing/Completed/Failed/Paused) correctly displayed
- [ ] Resume button enabled only for Paused state
- [ ] Cancel button enabled only for Executing state
- [ ] JSON output from execute and workflow-status parsed correctly

---

### 2.9 Logs Screen (P0-9)

**Description:** Display captured command logs with filtering and search capabilities.

**Requirements:**

- REQ-2.9.1: List view showing: timestamp, command, exit code, duration
- REQ-2.9.2: Expand row to see full stdout/stderr content
- REQ-2.9.3: Filter by: success/failure, command type, date range
- REQ-2.9.4: Search within log content (stdout, stderr, command args)
- REQ-2.9.5: Export selected logs to file
- REQ-2.9.6: "Open Logs Folder" action to reveal log directory in system file manager

**Acceptance Criteria:**

- [ ] All log entries displayed with correct metadata
- [ ] Filters work correctly (success/failure, command type, date range)
- [ ] Search finds matching entries in command, stdout, and stderr
- [ ] Expanded view shows complete stdout/stderr content

---

### 2.10 Error Handling (P0-10)

**Description:** Toast notifications and error panel for command failures.

**Requirements:**

- REQ-2.10.1: Toast notification appears on non-zero exit code from CLI
- REQ-2.10.2: Toast shows brief error message (first line of stderr or generic "Command failed")
- REQ-2.10.3: Click toast opens error detail panel
- REQ-2.10.4: Error panel shows: full stderr content, command that failed, exit code, duration
- REQ-2.10.5: Toast auto-dismisses after 5 seconds but error persists in panel
- REQ-2.10.6: Error panel accessible from sidebar (badge shows count of recent errors)

**Acceptance Criteria:**

- [ ] Errors trigger toast notification immediately
- [ ] Error panel accessible from toast click
- [ ] Full error details (stderr, command, exit code) visible in panel
- [ ] Multiple errors can be viewed in error history

---

### 2.11 Binary Location Resolution (P0-11)

**Description:** Resolve CLI binary location with fallback chain for flexibility.

**Requirements:**

- REQ-2.11.1: Check bundled binary in app resources first (`process.resourcesPath + '/bin/aiy'`)
- REQ-2.11.2: Settings option for user-specified binary path (overrides bundled)
- REQ-2.11.3: Fallback to PATH lookup (`which aiy` on Unix, `where aiy` on Windows)
- REQ-2.11.4: Error dialog if no binary found with instructions for resolution
- REQ-2.11.5: Verify binary is executable before use
- REQ-2.11.6: Display current binary path in Settings screen

**Acceptance Criteria:**

- [ ] Bundled binary used by default when present
- [ ] User-specified path in settings takes precedence
- [ ] PATH lookup works as final fallback
- [ ] Clear error message if binary not found (with suggested actions)

---

## 3. P1 Requirements (Should Have)

### 3.1 Cloud Confirmation Modal - Layer 2 (P1-1)

**Description:** When Hybrid mode is enabled and cloud action triggered, show modal with data preview requiring explicit confirmation.

**Requirements:**

- REQ-3.1.1: Show modal displaying `cloud_payload_preview` from CLI JSON output
- REQ-3.1.2: Require explicit "Send to Cloud" button click to proceed
- REQ-3.1.3: Preview content is read-only (user cannot edit)
- REQ-3.1.4: "Cancel" button returns to previous state without sending data
- REQ-3.1.5: Modal clearly states destination service (e.g., "Data will be sent to OpenAI")

**CLI Dependency:** Requires `aiy privacy execute --format json` to include `cloud_payload_preview` field. If not present, this feature is blocked.

**Acceptance Criteria:**

- [ ] Modal shows exactly what will be sent (redacted preview from CLI)
- [ ] User must click "Send to Cloud" to proceed
- [ ] Cancel returns to previous state safely
- [ ] Preview is read-only

---

### 3.2 File Watchers for State Changes (P1-2)

**Description:** Watch CLI state files for external changes and update UI immediately.

**Requirements:**

- REQ-3.2.1: Watch `.aiy/workflow-state.json` for workflow state changes
- REQ-3.2.2: Watch `.aiy/config.toml` for configuration changes
- REQ-3.2.3: UI updates within 1 second of file change detection
- REQ-3.2.4: Debounce rapid changes (100-200ms) to prevent UI thrashing
- REQ-3.2.5: Enable `awaitWriteFinish` option (500ms stability threshold) for stable reads
- REQ-3.2.6: Fallback to polling (2-5 second interval) if watcher fails or on network drives
- REQ-3.2.7: Handle file deletion gracefully (show "Not initialized" state)

**Technical Note:** Use Chokidar v4 with native events. Detect network/WSL paths and use polling automatically.

**Acceptance Criteria:**

- [ ] UI updates within 1 second of file change
- [ ] Rapid changes debounced correctly
- [ ] Polling fallback activates for network drives
- [ ] File deletion handled without crash

---

### 3.3 Workflow Context Display (P1-3)

**Description:** Show original request text alongside workflow status for context.

**Requirements:**

- REQ-3.3.1: Display original request text alongside workflow state on Workflows screen
- REQ-3.3.2: **Source context from JSONL logs** - parse the most recent `aiy privacy execute` invocation's args to extract request
- REQ-3.3.3: Use in-memory state for active session (request entered in current session)
- REQ-3.3.4: Graceful fallback message: "Request context unavailable" if not found in logs
- REQ-3.3.5: Truncate long requests with "Show more" expansion option

**Important Note:** `workflow-state.json` is intentionally metadata-only and does NOT contain the original request text. Context must be reconstructed from JSONL logs or in-memory state.

**Acceptance Criteria:**

- [ ] Workflow card shows state + request text (from JSONL or memory)
- [ ] Full request visible on detail view with "Show more" if truncated
- [ ] Timestamp of last state change displayed
- [ ] Graceful "Request context unavailable" shown when not found

---

### 3.4 Dashboard Attention Cards (P1-4)

**Description:** On app launch, prominently surface workflows requiring attention.

**Requirements:**

- REQ-3.4.1: "Attention Needed" section at top of dashboard when Paused or Failed workflows exist
- REQ-3.4.2: One-click "Resume" button for Paused workflows
- REQ-3.4.3: One-click "View Error" button for Failed workflows
- REQ-3.4.4: Cards dismissible but re-appear on next launch if still applicable
- REQ-3.4.5: Show workflow name/path and time since last activity

**Acceptance Criteria:**

- [ ] Attention cards appear at top for Paused/Failed workflows
- [ ] Resume button works directly from card
- [ ] View Error navigates to error details
- [ ] Cards reappear on restart if issue persists

---

### 3.5 Keyboard Shortcuts (P1-5)

**Description:** Global keyboard shortcuts for common workflow operations and navigation.

**Requirements:**

- REQ-3.5.1: `Cmd+K` / `Ctrl+K` = command palette
- REQ-3.5.2: `Cmd+1-5` / `Ctrl+1-5` = switch between screens (Workflows, Agents, Credentials, Privacy, Logs)
- REQ-3.5.3: `Cmd+Shift+E` / `Ctrl+Shift+E` = execute workflow
- REQ-3.5.4: `Cmd+Shift+R` / `Ctrl+Shift+R` = resume workflow
- REQ-3.5.5: `Cmd+Shift+S` / `Ctrl+Shift+S` = check workflow status
- REQ-3.5.6: Shortcuts work from any screen
- REQ-3.5.7: Shortcuts visible in command palette with descriptions

**Acceptance Criteria:**

- [ ] All shortcuts work from any screen
- [ ] Shortcuts listed in command palette
- [ ] Platform-appropriate modifier keys (Cmd for macOS, Ctrl for Windows/Linux)

---

### 3.6 Verify Privacy Button (P1-6)

**Description:** Button on Privacy Mode screen to run `aiy privacy check` and display raw output for verification.

**Requirements:**

- REQ-3.6.1: "Verify Privacy Configuration" button runs `aiy privacy check`
- REQ-3.6.2: Display raw CLI output in formatted panel
- REQ-3.6.3: Highlight `[PASS]` markers in green, `[FAIL]` markers in red
- REQ-3.6.4: Allow security-conscious users (Alex persona) to verify UI matches CLI output
- REQ-3.6.5: Copy raw output to clipboard button

**Acceptance Criteria:**

- [ ] Verify button runs privacy check
- [ ] Raw output displayed with syntax highlighting
- [ ] PASS/FAIL markers visually distinguished
- [ ] Output can be copied to clipboard

---

### 3.7 Stage Indicator for Long-Running Operations (P1-7)

**Description:** Show workflow execution stage instead of just a spinner for long-running operations.

**Requirements:**

- REQ-3.7.1: Visual stage indicator showing: Init -> Executing -> Finalizing
- REQ-3.7.2: Maps to workflow state from `aiy privacy workflow-status --format json`
- REQ-3.7.3: "Expand Output" button shows live captured stdout (streamed from CLI)
- REQ-3.7.4: Progress dots or step indicator for visual feedback
- REQ-3.7.5: Show elapsed time during execution

**Acceptance Criteria:**

- [ ] Stage indicator shows current phase
- [ ] Live stdout visible in expanded view
- [ ] Elapsed time displayed during execution

---

## 4. Non-Functional Requirements

### 4.1 Performance

| Metric | Target |
|--------|--------|
| App launch time | < 3 seconds |
| CLI command response | UI update within 100ms of output received |
| File watch reaction | < 1 second from file change to UI update |
| Log search | < 500ms for typical queries |

### 4.2 Security

- **No data to cloud without explicit user approval** (hard constraint)
- Credentials NEVER logged to JSONL or any persistent storage
- Logs stored locally only (no telemetry or remote logging)
- Zero telemetry by default (no opt-out required)
- Process tree cleanup on cancellation to prevent orphaned processes

### 4.3 Privacy Enforcement Layers

```
Layer 1: Global Kill-Switch ("Always Local" toggle)
         When ON: Electron hides/disables all cloud actions
         v1: UI enforcement only; CLI enforcement is P2-8

Layer 2: Per-Action Gate (when Hybrid enabled)
         Modal with cloud_payload_preview
         Explicit "Send to Cloud" confirmation required

Layer 3: Persistent Visual Indicator
         Status badge visible on ALL screens
         Green shield = Always Local
         Yellow warning = Hybrid enabled
```

### 4.4 Compatibility

| Platform | Minimum Version |
|----------|-----------------|
| macOS | 12+ (Monterey) |
| Windows | 10+ |
| Linux | Ubuntu 20.04+ |
| Electron | 28+ |
| aiy CLI | v0.x (current) |

### 4.5 Logging

- Location: `~/.config/aiy-desktop/logs/commands.jsonl`
- Rotation: 50MB max, 5 files
- Retention: User-controlled deletion
- Sensitive data: Automatically redacted

---

## 5. Technical Architecture Summary

### 5.1 CLI Integration Pattern

- Use `child_process.spawn` (no shell) for all CLI invocations
- Stream stdout/stderr for real-time UI updates
- Per-repo command queue with reader/writer lock semantics
- Process tree tracking for cleanup

### 5.2 State Management

- Zustand for lightweight UI state
- Electron local storage for persistent settings (privacy mode, binary path)
- JSONL logs for command history and context retrieval
- File watchers (Chokidar) for CLI state file changes

### 5.3 CLI Output Parsing

| Command Category | JSON Support | Parsing Strategy |
|------------------|--------------|------------------|
| `aiy agents *` | No | Text parsing with regex |
| `aiy credentials *` | No | Text parsing with regex |
| `aiy privacy check/enable/disable/config` | No | Parse `[PASS]`/`[FAIL]` markers |
| `aiy privacy execute --format json` | Yes | JSON parsing |
| `aiy privacy workflow-status --format json` | Yes | JSON parsing |

---

## 6. Out of Scope (v1)

- CLI streaming protocol changes (stdout/stderr capture only)
- CLI-level privacy kill-switch (P2-8 - future CLI work)
- New JSON output for CLI commands (work with existing text output)
- Auto-update mechanism (installer-based updates for v1)
- Extension/plugin system
- Multi-repo management in single window
- Cloud account creation/authentication flows

---

## 7. Dependencies

| Component | Purpose | Notes |
|-----------|---------|-------|
| Existing aiy CLI | All business logic | Text output parsing required for many commands |
| Chokidar v4 | File watching | Native events with polling fallback |
| electron-builder | Packaging | Bundle CLI binary with app |
| Zustand | State management | Lightweight, minimal boilerplate |

---

## 8. Risks and Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| CLI text output format changes | Parser breaks | Version-lock CLI with app; integration tests |
| Orphaned CLI processes | Resource leak | Process tree tracking; kill on app exit |
| File watcher failures on network drives | Stale UI | Automatic polling fallback |
| User bypasses UI for direct CLI use | Privacy not enforced | Clear documentation; P2-8 CLI enforcement |

---

## 9. References

- Research report: `docs/research-electron-wrapper.md`
- Research takeaways: `_bmad-output/analysis/research-takeaways-electron-wrapper.md`
- Brainstorming: `_bmad-output/analysis/brainstorming-session-2026-01-16.md`
- Product brief: `_bmad-output/briefs/product-brief-electron-wrapper.md`
- CLI enhancement backlog (post-PRD): `_bmad-output/planning-artifacts/backlog/cli-enhancements-electron-wrapper.md`

---

## 10. Revision History

| Version | Date | Author | Changes |
|---------|------|--------|---------|
| 1.0 | 2026-01-16 | Claude Planning Team | Initial draft |
