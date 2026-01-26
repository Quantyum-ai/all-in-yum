# PRP: Electron Wrapper UX Design — Codex Verification

**Project**: all-in-yum  
**Target Reviewer**: Codex Team  
**Date**: 2026-01-17  
**Review Mode**: Read-only verification (no edits unless explicitly requested)  
**Constraints**: Offline-only, no web searches, no network access

---

## Executive Summary

This PRP asks Codex Team to verify the **BMad UX Design workflow outputs** for the greenfield Electron desktop wrapper around the existing `aiy` CLI.

Primary objective: confirm the UX artifacts are **internally consistent**, **aligned with the PRD**, and preserve the **non‑negotiable privacy posture** (local-first, explicit cloud consent, zero telemetry).

Scope focus:
- `_bmad-output/planning-artifacts/ux-design-specification.md` (Steps 1–8, including Step 8 Visual Foundation)
- Consistency with:
  - `_bmad-output/briefs/product-brief-electron-wrapper.md`
  - `_bmad-output/prd/prd-electron-wrapper.md`
  - `_bmad-output/analysis/research-takeaways-electron-wrapper.md`
  - `_bmad-output/analysis/brainstorming-session-2026-01-16.md`

---

## Review Target

Record the exact repo state under review:
```bash
git rev-parse HEAD
git status --porcelain
```

Note: Some planning artifacts may be untracked; verification should still review their contents as present in the working tree.

---

## Claims to Verify

| Claim | Expected Result |
|---|---|
| UX spec exists and is progressing | `_bmad-output/planning-artifacts/ux-design-specification.md` exists and `stepsCompleted` includes Steps through **8** |
| CLI remains source of truth | UX spec consistently states: Electron is a “dumb display layer”; no UI-side redaction logic |
| Privacy posture is enforced in UX | Always Local default, persistent indicator, explicit per-action cloud confirmation, and no “remember/always allow” |
| `aiy privacy check` handling is correct | UX spec explicitly: parse `[PASS]`/`[FAIL]` markers; do **not** rely on exit code |
| Logging is safe | UX spec requires JSONL logging of command metadata/output with secrets masked, and **never persists full request text across sessions** (use `request_summary`/hash) |
| Offline-first is real | UX spec forbids runtime remote assets (fonts/icons/CDNs), uses system font stack, and avoids “you’re offline” blockers |
| Accessibility is first-class | UX spec calls out keyboard navigation, focus rings, non-color cues (icon + label), contrast targets, and `prefers-reduced-motion` |
| Visual foundation is coherent | Step 8 defines light/dark tokens, typography, spacing, layout, and maps to the chosen shadcn/ui + Tailwind approach |

---

## Verification Tasks

### 1) Document Inventory & Step Completion

Confirm required artifacts exist:
```bash
ls -la _bmad-output/planning-artifacts/ux-design-specification.md
ls -la _bmad-output/briefs/product-brief-electron-wrapper.md
ls -la _bmad-output/prd/prd-electron-wrapper.md
```

Confirm UX spec frontmatter indicates Step 8 completion:
```bash
sed -n '1,80p' _bmad-output/planning-artifacts/ux-design-specification.md
```

Expected:
- `stepsCompleted` includes `8`
- `inputDocuments` list includes the PRD/brief/takeaways/brainstorming

If Step 8 is not yet saved into the UX spec, mark this PRP as **blocked** and report “Step 8 missing from ux-design-specification.md”.

---

### 2) PRD ↔ UX Spec Alignment (No Scope Drift)

Verify UX spec matches PRD scope and doesn’t introduce unapproved v1 requirements:
```bash
rg -n "Workflows|Agents|Credentials|Privacy Mode|Logs" _bmad-output/planning-artifacts/ux-design-specification.md
rg -n "P0-|P1-|Out of Scope|Non-Functional Requirements" _bmad-output/prd/prd-electron-wrapper.md
```

Check for contradictions on v1 constraints:
- “CLI is source of truth”
- “Always Local default” (UI-enforced v1)
- “Hybrid may be unavailable until CLI support exists”

Deliverable: a short list of any scope drift or conflicts (if any).

---

### 3) Privacy Posture & Consent UX (Hard Requirements)

Confirm the UX spec contains all of these, explicitly:
- Persistent privacy indicator on every screen
- Always Local blocks cloud actions everywhere (including command palette)
- Cloud actions require per-action confirmation with preview (no “remember”)

Suggested checks:
```bash
rg -n "Always Local|Hybrid|kill-switch|persistent|indicator|command palette|Cmd\\+K|Ctrl\\+K|cloud_payload_preview|Send to Cloud|remember" _bmad-output/planning-artifacts/ux-design-specification.md
```

Verify the “verifiable privacy confidence” framing is present:
- Green shield = posture
- Verify Privacy = evidence

---

### 4) Known CLI Behavior Handling (Correctness)

Confirm the UX spec encodes known CLI constraints:
- `aiy privacy check` exit code unreliable → parse `[PASS]`/`[FAIL]`
- Many commands are text-only → parsing/version-locking risk acknowledged

Suggested checks:
```bash
rg -n "\\[PASS\\]|\\[FAIL\\]|exit code|privacy check" _bmad-output/planning-artifacts/ux-design-specification.md
rg -n "text-only|parse text|no JSON|format drift|version-lock" _bmad-output/planning-artifacts/ux-design-specification.md
```

---

### 5) Logging & Context Recovery Safety (No Code Persistence)

Confirm the UX spec does **not** require persisting full request text in logs across sessions.

Expected:
- Across sessions: only `request_summary` (redacted + truncated) and/or a request hash/ID is persisted
- In-memory: full request can exist only for the active session (if needed)

Suggested checks:
```bash
rg -n "JSONL|commands\\.jsonl|request_summary|request hash|never store the full request|in-memory|workflow-state\\.json" _bmad-output/planning-artifacts/ux-design-specification.md
```

If the UX spec implies persisting full request text, flag as a **privacy violation** and propose the minimal wording fix.

---

### 6) Visual Foundation (Step 8) Coherence & Implementability

Verify Step 8 includes:
- Color system (light/dark) with semantic mapping
- Typography system (system font stack; mono for CLI output)
- Spacing/layout (header + sidebar + main content), including active repo selector visibility
- Dark mode strategy (system preference + manual override)
- Accessibility notes (focus, contrast, reduced motion, not color-only)

Suggested checks:
```bash
rg -n "Visual Design Foundation|Color System|Typography System|Spacing|Layout|Dark Mode|prefers-reduced-motion|focus ring|contrast" _bmad-output/planning-artifacts/ux-design-specification.md
```

Confirm the visual foundation is consistent with:
- shadcn/ui + Tailwind + Radix primitives
- Offline-first: no runtime remote fonts/icons

Suggested “remote assets” scan (UX spec should not prescribe runtime CDNs):
```bash
rg -n "https?://|cdn|fonts\\.googleapis\\.com|unpkg\\.com" _bmad-output/planning-artifacts/ux-design-specification.md || true
```

---

## Deliverable (What Codex Team Should Return)

- Pass/fail checklist for each Verification Task section
- Any contradictions, missing requirements, or unsafe implications
- A short “minimal doc fixes” list (wording-level) if anything is off
- If blocked: state why (e.g., “Step 8 not yet saved in UX spec”)

---

## Acceptance Criteria

- UX spec through Step 8 is present and coherent in `_bmad-output/planning-artifacts/ux-design-specification.md`.
- No scope drift vs `_bmad-output/prd/prd-electron-wrapper.md`.
- Privacy posture is explicit, verifiable, and consent-gated; no “remember” cloud approvals.
- Logging/context recovery does not persist code or full request text across sessions.
- Visual foundation is implementable with shadcn/ui + Tailwind, offline-first, and accessible by design.

