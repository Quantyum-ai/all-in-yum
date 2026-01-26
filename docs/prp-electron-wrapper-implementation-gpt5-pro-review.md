# PRP: Electron Desktop Wrapper Implementation Readiness — GPT‑5 Pro Team Review

**Target reviewer**: GPT‑5 Pro Team  
**Mode**: Planning / review only (no code changes)  
**Constraints**: No web browsing; offline-first assumptions; **do not request or include code, file paths, diffs, stack traces, or dependency trees** in the review input or output.

---

## 1) Mission

Perform a critical, adversarial review of our current planning state for an Electron desktop application that wraps an existing CLI (“CLI is source of truth”), and produce a concrete, risk‑aware implementation plan that can be executed locally.

Success = your review identifies implementation blockers and missing decisions *before* we start coding, and outputs a prioritized roadmap with acceptance criteria and test strategy.

---

## 2) Redaction / Safety Boundary (Non‑Negotiable)

You must assume the project operates under a strict privacy posture:

- **Never** ask for or rely on: source code, file paths, diffs, stack traces, dependency trees, secrets, or proprietary repository metadata.
- All guidance must be written so it can be executed locally without sharing sensitive artifacts to any cloud service.
- If you recommend external links, telemetry, crash reporting, or any cloud call, you must explicitly mark it as **NOT allowed by default** and propose a zero‑telemetry alternative.

---

## 3) Current State (What Exists Today)

Planning is complete through:
- PRD complete (v1 scope defined; explicit P0/P1/P2 backlog).
- UX Design spec complete through **Step 11 (Component Strategy)**, including:
  - Visual foundation (tokens, layout, dark mode, accessibility guidance).
  - Design direction mockups (compact HTML showcase) and a “recommended combined approach”.
  - User journeys (first launch, execute workflow, privacy verification, session recovery, cloud consent, error recovery).
  - Component hierarchy and key UI components (layout/shared/feature components; consent modals; sanitized external-link flows).

v1 implementation decisions are locked in:
- **Design direction**: implement the “combined approach” (workflow-centric + command palette + privacy-first posture + log transparency); Step 9 variants are exploratory only.
- **Platforms**: macOS 12+, Windows 10+, Linux (Ubuntu 20.04+ or equivalent) all required in v1.
- **CLI strategy**: bundle a pinned CLI binary per platform; fallback order is bundled → user-selected path → PATH lookup.
- **Auto-update**: include an auto-updater but make it opt-in (default OFF), no telemetry/analytics, and require explicit user confirmation + signed updates.

Known CLI integration realities:
- CLI is authoritative; UI is a thin wrapper.
- Some CLI commands are text-only → parsing/version-lock risk; some key flows have JSON output.
- Privacy check requires parsing `[PASS]`/`[FAIL]` markers (exit code may be unreliable).
- Local logging must never persist full request text across sessions; store only safe summaries/hashes; full request in-memory only.
- Hybrid/cloud planning may be partially unavailable until CLI plumbing exists; UX must support “feature unavailable” states.

Non‑negotiables preserved in UX spec:
- Always Local default posture (Electron-enforced v1).
- Hybrid only via explicit user toggle; **mode persists** (no auto-revert, no auto-detect flip).
- Per-action cloud consent w/ preview; no “remember/always allow”.
- Zero telemetry by default; offline-first; no runtime remote assets.
- Single repo/workdir per window (no in-window multi-repo).

---

## 4) Review Inputs You May Assume (No Attachment Needed)

You may assume the requirements include five core screens:
- Workflows
- Agents
- Credentials
- Privacy Mode
- Logs

You may assume the technical direction includes:
- Electron + React + TypeScript
- Tailwind + shadcn/ui (Radix primitives)
- Command palette, keyboard-first UX, accessible focus management
- Secure process spawning (no shell), streaming stdout/stderr, per-repo command queue

Do not ask for the full documents. If something is ambiguous, ask clarifying questions using **generic terms** (no paths).

---

## 5) What You Must Produce (Deliverables)

### A) “Stop‑Ship” Issues
List the top issues that must be resolved before implementation begins (security, privacy, cross-platform process management, logging safety, update/signing, parsing brittleness, etc.).

### B) Missing Decisions / Ambiguities
List what is underspecified and needs an explicit decision, with options and a recommended choice.

Examples of the decision categories we expect:
- App architecture (main/renderer/preload separation, IPC shape, state model)
- CLI lifecycle management (queueing, cancellation, orphan prevention, timeouts)
- Logging format + redaction/masking policy (local-only; no cloud payload transformation)
- Workspace/repo selection model (single repo per window; new window behavior)
- Packaging model for the CLI binary (bundled vs external; upgrade strategy)
- Auto-update strategy (if any) under zero telemetry constraints
- Cross-platform filesystem locations and permissions

### C) Implementation Plan (Phased Roadmap)
Provide an execution plan broken into milestones that yield value incrementally.

Each milestone must include:
- Goal
- Scope (what screens/flows)
- Acceptance criteria (testable)
- Key risks + mitigations

Minimum expected milestones:
1) “Vertical slice” (app shell + spawn CLI + logs capture + privacy indicator)
2) Core workflow execution UX (queue, streaming output, cancel, error panel)
3) Privacy verification UX (parse markers, show raw output safely)
4) Credentials/Agents management UX (text parsing strategy + future JSON roadmap)
5) Packaging/signing/update plan (or explicit deferral)

### D) Test Strategy (Local‑Only)
Define a layered test strategy:
- Unit tests (parsers, state reducers, log scrubbing, queue logic)
- Integration tests (spawn mocked CLI; simulate stdout/stderr; cancellation)
- End-to-end smoke tests (packaged app optional; no network required)

Include how to create a deterministic “fake CLI” fixture for tests without depending on cloud providers.

### E) Threat Model & Security Controls (Electron‑Specific)
Provide a threat model oriented to Electron wrappers:
- Renderer compromise and IPC abuse
- Untrusted CLI output rendering
- Local log leakage (secrets in stdout/stderr)
- Supply chain / update channel
- Process tree / orphan processes

Output concrete controls (e.g., contextIsolation, strict IPC allowlists, sanitize rendering, no `shell: true`, no remote content, hardened CSP, etc.).

### F) “Definition of Done” Checklist
Provide a concise checklist we can use to decide “ready to ship v1”.

---

## 6) Required Output Format

Return exactly these sections:
1. **Critical Issues**
2. **Missing Decisions**
3. **Recommended Architecture**
4. **Milestone Plan**
5. **Test Strategy**
6. **Security / Threat Model**
7. **Open Questions**
8. **Definition of Done**

Keep the response implementation-oriented and specific, but **do not** include code, file paths, or dependency trees.

---

## 7) Clarifying Questions (Only If Necessary)

If you must ask questions, limit to the minimum set and phrase them so the answers can be provided without sharing sensitive artifacts.

Examples of allowed question forms:
- “Should the auto-updater check on launch, or on a schedule?”
- “Is the update channel a public feed or an internal enterprise feed?”
- “Do you require code signing/notarization for internal builds too, or only for public releases?”
