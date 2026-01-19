# PRP: UX Steps 9–10 Fixpack — Codex Re‑Verification

**Project**: all-in-yum  
**Target Reviewer**: Codex Team  
**Date**: 2026-01-18  
**Review Mode**: Read-only verification (no edits unless explicitly requested)  
**Constraints**: Offline-only, no web searches, no network access

---

## Executive Summary

Claude applied a **wording-only fixpack** to address Codex FAILs in the prior verification of UX Steps 9–10 (privacy-mode semantics drift, invented CLI command, and external-link leakage phrasing).

This PRP asks Codex to:
- Re-run the full verification workflow from `docs/prp-electron-wrapper-ux-steps9-10-codex-verification.md`, and
- Add a targeted “fixpack delta” check to confirm the specific issues are resolved.

Primary artifact under review:
- `_bmad-output/planning-artifacts/ux-design-specification.md`

Reference artifact (Step 9):
- `_bmad-output/planning-artifacts/ux-design-directions.html`

---

## Fixpack Targets (What Must Now Be True)

### A) Privacy mode semantics (align with PRD)
- No “Hybrid Mode if cloud features detected” (Hybrid must be user-enabled).
- Privacy check FAIL must not flip the app into Hybrid mode.
- Cloud consent flow must not auto-revert to Always Local after completion (mode persists until user toggles).

### B) CLI command correctness
- No `aiy init` (does not exist in this repo).
- Uses `aiy privacy init` for initialization.

### C) External-link leakage guardrail
- No “Search docs link with error text”.
- No “opens GitHub with template” phrasing implying auto-prefill.
- External sharing/searching must require explicit preview/consent and use sanitized content only.

PRD anchors:
- Hybrid requires explicit user action: `_bmad-output/prd/prd-electron-wrapper.md:95`
- Always Local default persisted in Electron storage: `_bmad-output/prd/prd-electron-wrapper.md:94`

---

## Verification Tasks

### 1) Record Repo State (Traceability)

```bash
git rev-parse --abbrev-ref HEAD
git rev-parse HEAD
git status --porcelain
```

---

### 2) Artifact Presence & Timestamp Sanity

```bash
ls -la _bmad-output/planning-artifacts/ux-design-specification.md
ls -la _bmad-output/planning-artifacts/ux-design-directions.html
```

Expected:
- Both files exist.

---

### 3) Fixpack Delta Preflight (Targeted “Bad Pattern” Scan)

These strings were direct FAIL evidence previously and must now be absent:

```bash
rg -n 'cloud features detected|Returns to "Always Local"|Badge changes to amber "Hybrid Mode"|`aiy init`|aiy init|Search docs" link with error text|opens GitHub with template' _bmad-output/planning-artifacts/ux-design-specification.md || true
```

Expected:
- No matches.

Confirm required replacements are present:
```bash
rg -n 'current mode \\(persisted setting\\)|Hybrid.*explicit|Mode remains unchanged|aiy privacy init|preview/consent|sanitized' _bmad-output/planning-artifacts/ux-design-specification.md
```

Expected:
- Matches indicating the corrected semantics/wording exist.

---

### 4) Re-run Full PRP: Steps 9–10 Verification (Tasks 1–8)

Execute the full verification PRP and report PASS/FAIL per task with evidence:
- `docs/prp-electron-wrapper-ux-steps9-10-codex-verification.md`

Minimum evidence expected in the report:
- Step completion marker includes 9 and 10.
- Step 9 summary includes A–F, recommended combined approach, coverage checklist, and reference to the HTML artifact.
- Step 10 includes Journeys 1–6 with Trigger/Success criteria + shortcuts + state machine.
- No PRD semantic drift (Hybrid is explicit user toggle; mode persists; verification doesn’t flip modes).
- CLI commands referenced are real (`aiy privacy init`, `aiy privacy check`); no invented commands.
- External-link behavior is user-initiated with preview/consent and sanitized payload only.

---

### 5) CLI Command Reality Check (In-Repo Source)

If Task 7 is still at risk, confirm CLI commands from source:

```bash
rg -n 'pub enum Commands' crates/aiy-cli/src/main.rs
rg -n 'pub enum PrivacyCommands' crates/aiy-cli/src/main.rs
rg -n '\\bInit\\b' crates/aiy-cli/src/main.rs
rg -n 'aiy privacy init' crates/aiy-cli/src || true
```

Expected:
- `PrivacyCommands::Init` exists (so `aiy privacy init` is real).
- No top-level `Init` command exists (so `aiy init` should not appear in UX spec).

---

## Deliverable (What Codex Team Should Return)

- PASS/FAIL per task for `docs/prp-electron-wrapper-ux-steps9-10-codex-verification.md` (Tasks 1–8)
- A separate PASS/FAIL for the “Fixpack Delta Preflight” (Task 3 here)
- If any FAIL remains: minimal wording fix list with exact file+line references in `_bmad-output/planning-artifacts/ux-design-specification.md`

---

## Acceptance Criteria

- Fixpack delta scan finds no regressions in `_bmad-output/planning-artifacts/ux-design-specification.md`.
- `docs/prp-electron-wrapper-ux-steps9-10-codex-verification.md` fully PASSes (Tasks 1–8).

