# PRP: aiy Desktop — GPT‑5 Pro Implementation Readiness Review (Cloud‑Safe)

**Product**: “aiy Desktop” — Electron desktop GUI wrapper around an existing Rust CLI (`aiy`)  
**Target reviewer**: GPT‑5 Pro (planning/review only)  
**Date**: 2026-01-19  
**Mode**: Review / planning only (NO implementation)  
**Security posture (hard constraint)**: Do not include any code, file paths, diffs, stack traces, dependency trees, or proprietary repo details in the prompt or outputs. No web browsing.

---

## 0) Purpose

Provide an implementation readiness review that is rigorous enough to guide successful delivery of the Electron app through v1, while respecting strict privacy constraints.

You are reviewing **a project status snapshot** and must produce:

1) A risk map (stop‑ship risks first)  
2) Missing decisions / ambiguities to resolve  
3) A milestone roadmap (M2–M5) with acceptance criteria  
4) A security hardening checklist specific to Electron + CLI wrappers  
5) A test strategy emphasizing offline determinism and privacy constraints  
6) A “Definition of Done” checklist for v1 release readiness

---

## 1) Product Summary (Sanitized)

The product is a privacy‑first Electron desktop app that wraps a Rust CLI which orchestrates AI agents and a “Privacy Mode” local‑execution workflow (local model via Ollama).

Core UX premise:
- Electron is a **thin display/orchestration layer**; the CLI remains the **source of truth** for business logic.

Non‑negotiables:
- Always Local is the default posture (UI‑enforced in v1)
- Hybrid (cloud‑enabled) is only via explicit user toggle; mode persists; no auto‑flip; no auto‑revert
- Per‑action cloud consent with preview; no “remember/always allow”
- Zero telemetry by default; offline‑first runtime (no runtime remote assets)
- CLI output is treated as untrusted text; strict Electron hardening (context isolation, no Node in renderer, IPC allowlist)
- Logging is transparent but safe: never persist full request text across sessions; only safe summaries/hashes; scrub secrets in local logs; never transform cloud payloads in UI
- v1 repo/workdir model: **single repo per window**; opening another repo opens a new window

Platforms required for v1:
- macOS 12+, Windows 10+, Linux (Ubuntu 20.04+ equivalent)

Updater:
- Opt‑in, default OFF; no telemetry; explicit user confirmation; signed updates; check at most once per 24h (or manually)

---

## 2) What Is Already Complete (Sanitized)

Planning artifacts are complete (PRD + UX design specification through component strategy), with implementation decisions locked in (design direction combined approach; platform scope; CLI bundling; updater opt‑in).

Milestone 1 (“Walking Skeleton”) is implemented:
- Electron app scaffold using modern tooling (Vite‑based Electron build, React 18, TypeScript, Tailwind, Zustand, Vitest)
- Hardened Electron window configuration (no Node in renderer, context isolation, sandboxing)
- Preload bridge exposes a minimal API; renderer does not access ipcRenderer directly
- IPC allowlist + schema validation in main process
- CLI service in main process: spawn (no shell), streamed stdout/stderr, cancellation, PID tracking, cross‑platform process‑tree kill strategy, JSONL logging with secret scrubbing, log rotation
- CLI output parsers exist with fixtures and tests (privacy markers, agents, credentials, workflow status JSON)
- Test suite is deterministic and fully passing in the current environment (100 tests across 6 suites)

Known environment note:
- Some sandboxed environments may block loopback port binding for the dev server; this is treated as environmental, not architectural.

---

## 3) Review Questions You Must Answer

### 3.1 Stop‑Ship Risks
- What could cause accidental cloud leakage or undermine the “Always Local by default” posture?
- What could cause orphaned processes or unintended continued execution after cancel/exit?
- What could cause UI/CLI desync (text parsing fragility, version mismatches) and how should the product mitigate?
- What could compromise updater integrity (supply chain)?

### 3.2 Missing Decisions / Clarifications
Provide a numbered list of decisions still required before implementation proceeds beyond M1, including suggested defaults.

### 3.3 Architecture Validation
Evaluate the proposed Electron architecture for:
- Main/preload/renderer separation and minimal attack surface
- IPC contract discipline (allowlist, schema validation, no generic “run arbitrary command” channel)
- CLI execution controller (queues, cancellation, timeout policy)
- Logging policy correctness (safe persistence vs in-memory only)

### 3.4 Milestone Roadmap (M2–M5)
Provide a recommended milestone plan that is implementable and de‑risks early:
- M2: Workflows + workflow state watching
- M3: Privacy verification + consent UX (and “feature unavailable” states)
- M4: Agents + credentials UX (secure credential storage, no leakage)
- M5: Packaging + signing + opt‑in updater

For each milestone, include:
- Primary goal
- Scope bullets
- Acceptance criteria bullets
- Key risks + mitigations

### 3.5 Test Strategy
Provide a layered test plan:
- Unit tests (parsers, redaction/scrubbing, queue logic)
- Integration tests (fake CLI harness; streaming; cancel; consent flows)
- E2E smoke tests (per platform, offline)
- Security regression tests (CSP, IPC allowlist, renderer isolation)

### 3.6 Definition of Done
Provide a release readiness checklist for v1 that maps to:
- PRD P0 requirements
- UX non‑negotiables
- Cross‑platform reliability
- Security posture & privacy guarantees
- Packaging/signing/updater requirements

---

## 4) Output Format (Required)

Return a structured report with these sections (in this order):

1) Executive summary (≤ 12 bullets)  
2) Stop‑ship risk map (ranked; include mitigations)  
3) Missing decisions / open questions (ranked; include recommended defaults)  
4) Milestone plan (M2–M5) with acceptance criteria  
5) Test plan (unit → integration → e2e → security)  
6) Definition of Done checklist  

Do **not** request code, file paths, or diffs. If you need more detail, ask for it in abstract terms (e.g., “provide a sanitized example of the CLI output for X”).

---

## 5) Constraints Reminder (Must Comply)

- No web browsing.
- No code, file paths, diffs, or stack traces.
- No dependency trees.
- Assume offline-first runtime.

