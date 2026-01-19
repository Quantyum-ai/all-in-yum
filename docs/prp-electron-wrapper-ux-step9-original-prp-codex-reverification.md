# PRP: UX Step 9 (Original PRP) — Codex Re‑Verification After “Paused” Fix

**Project**: all-in-yum  
**Target Reviewer**: Codex Team  
**Date**: 2026-01-18  
**Review Mode**: Read-only verification (no edits unless explicitly requested)  
**Constraints**: Offline-only, no web searches, no network access

---

## Executive Summary

Codex previously reported that the Step 9 design directions artifact passed the **fixpack PRP** but failed the **original Step 9 PRP** Task 7 due to missing explicit representation of the workflow **Paused** state.

Claude team reports they applied the minimal “Paused” fix (Option A):
- Added `.status-badge.paused` CSS
- Changed one workflow badge from `Idle` → `Paused`

This PRP asks Codex to re-run the **original** Step 9 PRP check to confirm Task 7 is now PASS and that no regressions were introduced.

Primary artifact:
- `_bmad-output/planning-artifacts/ux-design-directions.html`

PRP to execute:
- `docs/prp-electron-wrapper-ux-step9-design-directions-codex-verification.md`

---

## Verification Tasks

### 1) Record Repo State (Source of Truth = Working Tree)

```bash
git rev-parse --abbrev-ref HEAD
git rev-parse HEAD
git status --porcelain
```

---

### 2) Confirm Paused State Is Present (Preflight)

```bash
rg -n "(?i)paused" _bmad-output/planning-artifacts/ux-design-directions.html
```

Expected: at least one match (either a status badge or a log entry).

Optional spot-check for the paused badge style:
```bash
rg -n "\\.status-badge\\.paused" _bmad-output/planning-artifacts/ux-design-directions.html
```

---

### 3) Re-Run Original Step 9 PRP

Execute `docs/prp-electron-wrapper-ux-step9-design-directions-codex-verification.md` exactly, and report PASS/FAIL per task.

Key expected delta vs prior run:
- **Task 7 (Coverage completeness guardrail): PASS**
  - “Paused” is now explicitly represented alongside “running” and “failed”.

---

### 4) Regression Guardrails (Quick Checks)

Ensure nothing regressed:

**Offline-first still intact**
```bash
rg -n "https?://|cdn|fonts\\.googleapis\\.com|use\\.typekit\\.net|unpkg\\.com|jsdelivr\\.net|<script\\s+src=|<link\\s+rel=\\\"stylesheet\\\"\\s+href=" _bmad-output/planning-artifacts/ux-design-directions.html || true
```

**No in-window multi-repo scope creep**
```bash
rg -n "recent repos|switch repos|repo dropdown|repo selector|open other" _bmad-output/planning-artifacts/ux-design-directions.html || true
```

**Accessibility focus styling still present**
```bash
rg -n "outline:\\s*none" _bmad-output/planning-artifacts/ux-design-directions.html || true
rg -n ":focus-visible" _bmad-output/planning-artifacts/ux-design-directions.html || true
```

---

## Deliverable (What Codex Team Should Return)

- PASS/FAIL checklist for the original Step 9 PRP tasks (1–9)
- Explicit confirmation that Task 7 is now PASS
- Any remaining minimal fixes (if any), with exact `rg -n` evidence

