# PRP: Electron Desktop Wrapper (aiy Desktop) — Claude Implementation Execution

**Project**: all-in-yum (`aiy`)  
**Target executor**: Claude implementation team  
**Date**: 2026-01-19  
**Mode**: Implementation (repo modifications allowed)  
**Constraints**: Offline-first runtime, zero telemetry by default, no web searches

---

## 0) Goal

Implement the **aiy Desktop** Electron application (greenfield UI) as a thin desktop wrapper around the existing `aiy` CLI, aligned with:

- `_bmad-output/prd/prd-electron-wrapper.md`
- `_bmad-output/planning-artifacts/ux-design-specification.md` (Steps 1–11)
- `docs/research-electron-wrapper.md`

The CLI remains the **single source of truth** for business logic. Electron provides UX, orchestration, safe logging, and strict user-consent gating.

---

## 1) v1 Lock‑In Decisions (Already Approved — Do Not Re-litigate)

From `_bmad-output/planning-artifacts/ux-design-specification.md` “Implementation Decisions (v1 Lock‑In)”:

- **Design direction**: implement the combined approach **B + C + A + F** (workflow-centric + command palette + privacy-first posture + log transparency). Step 9 variants are exploratory only.
- **Platforms**: macOS 12+, Windows 10+, Linux (Ubuntu 20.04+ or equivalent) all required for v1.
- **CLI strategy**: bundle a pinned `aiy` binary per platform; fallback order: bundled → user-selected path → PATH lookup.
- **Auto-update**: include an auto-updater but make it **opt-in (default OFF)**, no telemetry/analytics, explicit user confirmation for download/install, signed updates only; check on launch at most once per 24h unless manually triggered.

---

## 2) Non‑Negotiables (Stop‑Ship if Violated)

### Privacy posture
- Always Local default posture (Electron-enforced v1).
- Hybrid requires explicit user action; **mode persists** (no auto-detect flips, no auto-revert).
- Privacy mode is a **global persisted setting** (applies to all windows); changing it updates all windows’ indicators.
- Per-action cloud consent required; **no “remember/always allow”**.
- No code/paths/diffs/stack traces/dependency trees sent to cloud without explicit per-action approval.

### Electron security hardening
- `nodeIntegration: false`, `contextIsolation: true`, no `remote` module.
- Strict IPC allowlist + schema validation.
- No remote content loading at runtime; strict CSP; block navigation and popups.
- Treat all CLI output as untrusted text (no `innerHTML`).

### Logging safety
- Never persist full request text across sessions; persist only safe `request_summary` + optional hash/ID.
- Credentials are never written to logs; scrub/mask sensitive patterns before persisting local logs.
- UI may mask/scrub before writing local logs; UI must never “redact/transform cloud payloads” (cloud boundary is CLI-owned).

### Process safety
- Spawn CLI with `shell: false`.
- Track PIDs; implement cancel + app-exit cleanup; prevent orphaned processes cross-platform.
- Per-window/per-repo command queue; no concurrent repo-mutating commands in same workdir.

---

## 3) Implementation Architecture (Required)

### Process model
- **Main process**: owns OS access, CLI spawning, filesystem writes for logs/config, auto-update, window lifecycle.
- **Preload**: exposes a minimal, typed API via `contextBridge`; no direct `ipcRenderer` in the React app.
- **Renderer**: React UI only; no Node APIs; communicates solely through the preload API.

### IPC contract (examples)
- `cli.run` (request-response for starting a job; returns job id)
- `cli.cancel` (request-response)
- `cli.onOutput` (event stream for stdout/stderr chunks)
- `cli.onExit` (event stream for completion)
- `app.openRepoWindow` (request-response)
- `app.openExternal` (request-response; URL allowlist + protocol validation)
- `settings.get` / `settings.set` (request-response; strict schema; secrets never returned)

All IPC messages must be validated in main (schema + allowlisted commands).

### CLI execution controller
Implement a “CLI Service” in main that provides:
- Binary resolution: bundled → user path → PATH.
- Version detection: read `aiy version` and compare against supported range; warn/refuse if incompatible (especially if external CLI selected).
- Per-window queue with cancellation and timeouts:
  - Repo-mutating commands are **exclusive** per window/workdir.
  - Read-only commands may run concurrently (but never concurrently with a mutating command in the same window/workdir).
  - A failed command must not wedge the queue; subsequent queued commands should proceed unless explicitly canceled.
- Stream stdout/stderr to renderer (buffering/batching to avoid UI perf issues).
- Robust kill strategy:
  - Windows: explicit `taskkill /PID <pid> /T /F` (PID-based only).
  - macOS/Linux: kill process tree using PID-based traversal (no pattern kills).

### Data storage locations
Use Electron `app.getPath('userData')` for app config and safe logs. No runtime remote assets.

---

## 4) Scope Plan (Milestones)

### Milestone 1 — Walking Skeleton (Vertical Slice)
**Goal**: Prove end-to-end main/preload/renderer wiring + spawn CLI + stream output + cancel.

**Scope**
- App shell (Header/Sidebar/Main) + basic Logs panel.
- CLI run of a safe command (`aiy version` or `aiy privacy --help`) with streamed output.
- Per-window queue (single queue) + cancel + exit cleanup.
- Privacy badge placeholder (Always Local / Hybrid toggle UI only; no cloud behavior yet).

**Acceptance**
- Launches in dev mode on macOS/Windows/Linux.
- Streams stdout/stderr without freezing UI.
- Cancel reliably stops process and children.
- No Node access in renderer; IPC allowlist enforced.

---

### Milestone 2 — Workflows (P0 core)
**Goal**: Implement v1 “Workflows” lifecycle with CLI privacy workflow commands.

**Scope**
- Repo selection flow (single repo per window; “Open Repo…” opens a new window).
- Workflow init/status/execute/resume/cancel using:
  - `aiy privacy init`
  - `aiy privacy workflow-status --format json`
  - `aiy privacy execute <request> --format json`
  - `aiy privacy resume`
  - `aiy privacy cancel`
- Queue semantics: repo-mutating commands serialized per window.
- UI states: Ready/Executing/Paused/Completed/Failed + duration/timestamps.
- File watching:
  - Watch `.aiy/workflow-state.json` to update state in near-real-time.
  - Use chokidar-style semantics: debounce + `awaitWriteFinish`, with polling fallback for flaky filesystems.
- Logs capture (safe JSONL schema; no full request persisted) + rotation:
  - Size-based rotation and bounded retention.
  - “Open Logs Folder” support.

**Acceptance**
- Workflows screen matches PRD/UX journeys for init/execute/status/resume/cancel.
- JSON parsing for workflow-status/execute is correct and resilient.
- No full request text is persisted; request summaries only.

---

### Milestone 3 — Privacy Verification + Consent UX (P0/P1)
**Goal**: Implement “verifiable privacy confidence”.

**Scope**
- Privacy Mode screen:
  - Run `aiy privacy check` and parse `[PASS]`/`[FAIL]` markers (ignore exit code).
  - Show raw output safely (text-only rendering) with PASS/FAIL highlighting.
  - Show “Hybrid unavailable” reasons (missing creds, CLI support absent).
- Consent UI:
  - Implement `CloudConsentModal` and flows as per UX spec.
  - **If CLI cannot provide payload previews yet**, implement explicit “feature unavailable (requires CLI update)” states rather than inventing previews.

**Acceptance**
- Privacy check parsing is tested against fixtures (multiple PASS/FAIL cases).
- Consent modal has no “remember”; always explicit approval.
- No automatic mode flips; mode persists.

---

### Milestone 4 — Agents + Credentials (P0)
**Goal**: Implement Agents/Credentials screens with safe credential handling.

**Scope**
- Agents: `aiy agents list/status/enable/disable` (text parsing; fixtures + tests).
- Credentials:
  - Status: `aiy credentials status` parsing.
  - Set/Delete:
    - Preferred: use OS keychain integration consistent with CLI storage (system keychain), or implement a safe CLI interaction strategy that does not expose secrets via args/logs.
    - Never log credentials; never persist credentials in app logs/config.
- UX links: missing credentials → deep link to Credentials screen.

**Acceptance**
- Credentials value never appears in process args, logs, or persisted files.
- Status parsing robust; set/delete flows confirmed on all 3 platforms.

---

### Milestone 5 — Packaging, Signing, Auto‑Update (Opt‑In)
**Goal**: Produce distributable builds and secure updater (opt-in default OFF).

**Scope**
- Packaging formats: macOS (signed + notarized), Windows (signed), Linux (AppImage).
- Bundle pinned CLI binaries per platform; ensure executable permission on Unix.
- Package layout:
  - Use `asar` for app code, but ensure binaries (and any native deps) are unpacked (e.g., `asarUnpack`) so the CLI is executable at runtime.
- Updater:
  - Disabled by default; manual “Check for updates” always available.
  - When enabled: check on launch ≤ 24h; configurable feed URL; explicit confirmation for download/install; signature verification.
  - No telemetry/analytics; minimal update check payload only.

**Acceptance**
- Packaged apps run offline and can spawn bundled CLI successfully.
- Updater does not check when disabled; does not auto-download/install; does not include identifiers.

---

## 5) Test Plan (Local‑Only)

### Unit tests
- CLI output parsers: privacy check markers, agents list/status, credentials status output.
- Log scrubbing/masking utilities.
- Queue state reducer (enqueue/dequeue/cancel/timeouts).

### Integration tests
- Deterministic “fake CLI” fixture that simulates:
  - streaming output
  - long-running + cancel
  - error exit codes
  - PASS/FAIL marker output
  - JSON success/error envelopes for workflow commands
- Electron integration tests (Playwright or equivalent) to validate:
  - spawn + stream + cancel
  - queue behavior
  - consent modal display logic

### E2E smoke tests (offline)
- First launch → open repo → init → execute → view logs → verify privacy.
- Multi-window isolation: open two repos; ensure output/logs do not cross.

---

## 6) Implementation Notes / Guardrails

- Never use `exec` or `shell: true` for CLI commands.
- Do not rely on CLI exit code for `privacy check`; parse markers.
- Avoid heavy re-renders on streaming logs; batch updates and consider virtualization if needed.
- Never open external URLs without:
  - protocol allowlist (`https:` only for update feed; `https:`/`http:` for docs), and
  - a sanitized preview/consent modal for anything derived from errors.
- Avoid destructive commands and pattern-based process killing.

---

## 7) Definition of Done (Implementation Phase Entry)

You may begin implementation when:
- Milestone 1 vertical slice is working locally (spawn/stream/cancel).
- Security hardening baseline is in place (isolation, CSP, IPC allowlist).
- Logging policy and JSONL schema are implemented and tested.

---

## 8) Deliverables

- New Electron app code added to the repo (choose a clear top-level location such as `apps/aiy-desktop/`).
- Build/run instructions for dev mode and packaged builds.
- Test harness including fake CLI + parser unit tests.
- Clear documentation of supported/bundled CLI version + mismatch behavior.
