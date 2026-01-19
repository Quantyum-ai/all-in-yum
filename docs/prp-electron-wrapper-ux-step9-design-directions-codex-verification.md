# PRP: UX Step 9 (Design Directions) — Codex Verification

**Project**: all-in-yum  
**Target Reviewer**: Codex Team  
**Date**: 2026-01-18  
**Review Mode**: Read-only verification (no edits unless explicitly requested)  
**Constraints**: Offline-only, no web searches, no network access

---

## Executive Summary

This PRP asks Codex Team to verify the **Step 9 Design Direction Mockups** output created by the BMad UX team for the Electron desktop wrapper around the existing `aiy` CLI.

The Step 9 deliverable was produced in a **compact** format. Codex should ensure the compacting does **not lose or contradict** the key UX/product constraints established in earlier steps (privacy posture, offline-first, v1 scope, CLI-as-source-of-truth).

Primary artifact under review:
- `_bmad-output/planning-artifacts/ux-design-directions.html`

Alignment references (for “no drift” checks):
- `_bmad-output/planning-artifacts/ux-design-specification.md` (Steps 1–8 are complete)
- `_bmad-output/prd/prd-electron-wrapper.md`
- `_bmad-output/briefs/product-brief-electron-wrapper.md`

---

## Non-Negotiable Context (Must Remain True)

Codex should treat these as hard constraints when reviewing Step 9 outputs:

- **CLI is source of truth**; Electron is a wrapper. UI must not invent new business logic.
- **Privacy posture**:
  - Always Local default (UI-enforced v1).
  - Persistent indicator on all screens.
  - Per-action cloud confirmation with preview; **no “remember/always allow”**.
  - No code/paths/diffs/stack traces/dependency trees to cloud without explicit per-action approval.
- **Local logging safety**:
  - UI does **not** redact/transform cloud payloads.
  - UI **may** mask/scrub sensitive patterns before persisting local JSONL logs (defense-in-depth).
  - Never persist full request text across sessions; store only safe `request_summary`/optional hash.
- **Offline-first**:
  - No runtime remote assets (fonts/icons/JS/CSS/CDNs).
  - System font stack; bundled/local assets only.
- **v1 repo model**:
  - **Single repo/workdir per window** (no in-window multi-repo management).
  - “Open Repo…” should open a new window (or otherwise remain v1-compliant).
- **Known CLI behavior**:
  - `aiy privacy check` exit code can be unreliable → parse `[PASS]`/`[FAIL]` markers.

---

## Claims to Verify

| Claim | Expected Result |
|---|---|
| Step 9 HTML showcase exists | `_bmad-output/planning-artifacts/ux-design-directions.html` present and opens offline |
| Six directions included | Contains Direction A–F headings and distinct mockups |
| No external assets | No `http(s)://`, no `<script src=...>`, no `<link rel="stylesheet" href=...>` |
| v1 scope preserved | No in-window repo switching/multi-repo management implied |
| Privacy posture preserved | Persistent privacy indicator present in layouts; language does not undermine Always Local default / explicit consent |
| Accessibility not regressed | Compact mockups do not introduce anti-patterns (e.g., `outline: none` without a replacement focus style) |
| Compacting didn’t drop critical flows | The showcase still represents (visually or explicitly via callouts) the key v1 screens/flows: Workflows, Privacy (Verify Privacy), Logs (filters/search + “Copy CLI Command (redacted)”) |

---

## Verification Tasks

### 1) Record Repo State

```bash
git rev-parse HEAD
git status --porcelain
```

Note: planning artifacts may be untracked; still review them as present in working tree.

---

### 2) Artifact Presence & Basic Sanity

```bash
ls -la _bmad-output/planning-artifacts/ux-design-directions.html
```

Confirm the file renders as a standalone HTML doc (no build step).

---

### 3) Confirm All 6 Directions Exist

```bash
rg -n "Direction A:|Direction B:|Direction C:|Direction D:|Direction E:|Direction F:" _bmad-output/planning-artifacts/ux-design-directions.html
```

Expected: at least one match per direction.

---

### 4) Offline-First: No Runtime Remote Assets

```bash
rg -n "https?://|cdn|fonts\\.googleapis\\.com|use\\.typekit\\.net|unpkg\\.com|jsdelivr\\.net|<script\\s+src=|<link\\s+rel=\\\"stylesheet\\\"\\s+href=" _bmad-output/planning-artifacts/ux-design-directions.html || true
```

Expected: no matches.

---

### 5) v1 Repo Model: No In-Window Multi-Repo Management

The HTML may show repo/workdir context, but must not imply in-window multi-repo management.

```bash
rg -n "recent repos|switch repos|repo dropdown|repo selector|open other" _bmad-output/planning-artifacts/ux-design-directions.html || true
```

Expected: no matches.

If any matches exist, confirm they are explicitly “new window” behavior and v1-compliant.

---

### 6) Privacy Posture & Consent Patterns Are Not Lost

Verify that the showcase still reflects the core posture:
- persistent privacy indicator
- Always Local default framing
- Hybrid is explicit/consent-gated (even if only called out)

```bash
rg -n "Always Local|Local Mode|Hybrid|Send to Cloud|cloud_payload_preview|preview|remember" _bmad-output/planning-artifacts/ux-design-directions.html || true
```

Expected:
- Presence of “Local” posture indicators.
- If Hybrid is mentioned, it must not introduce “remember/always allow”.
- If cloud consent patterns are missing entirely, flag as “coverage gap due to compacting”.

---

### 7) Coverage Completeness (Compact Version Guardrail)

Because Step 9 is compact, verify that **none of these are dropped**:

**Required UI elements to appear visually OR be explicitly described in the HTML:**
- Workflows execution state (running/paused/failed) + stage indicator and/or live output panel concept
- Privacy Mode “Verify Privacy” concept (raw CLI output with `[PASS]`/`[FAIL]` highlighting)
- Logs screen concept (filter/search + “Copy CLI Command (redacted)”)

Suggested searches:
```bash
rg -n "\\[PASS\\]|\\[FAIL\\]|Verify Privacy|privacy check|Logs|Copy CLI|command palette|Cmd\\+K|Ctrl\\+K" _bmad-output/planning-artifacts/ux-design-directions.html
```

If any required element is missing:
- Mark as **FAIL (coverage gap)** and list exactly what’s missing.
- Recommend the minimal addition needed (e.g., add one compact Privacy mock + one compact Logs table mock).

---

### 8) Accessibility Regression Scan (HTML/CSS)

The HTML is a mock, but it must not normalize accessibility anti-patterns.

```bash
rg -n ":focus|outline\\s*:|prefers-reduced-motion|aria-|role=" _bmad-output/planning-artifacts/ux-design-directions.html || true
```

Expected:
- Either no `outline: none`, or if present, an explicit replacement focus style is shown.
- If reduced motion isn’t addressed in this artifact, it’s acceptable if the UX spec already covers it, but should be noted as “not represented in mockups”.

---

### 9) Alignment Check Against UX Spec + PRD (No Contradictions)

Confirm the mockups do not contradict the UX spec/PRD constraints (especially v1 repo model and privacy posture).

Suggested quick comparisons:
```bash
rg -n "Single repo|per window|Open Repo" _bmad-output/planning-artifacts/ux-design-specification.md
rg -n "Multi-repo" _bmad-output/prd/prd-electron-wrapper.md
```

If Step 9 introduces a new “Dashboard” screen or other additions, ensure it is clearly positioned as a design-direction exploration and not a v1 must-have requirement (unless PRD is updated).

---

## Deliverable (What Codex Team Should Return)

- PASS/FAIL per Verification Task (1–9)
- If FAIL: minimal “doc/mock fix list” with exact missing items
- A short note on whether compacting dropped any required flows (Privacy verification UI, cloud consent preview, Logs filtering/search)

---

## Acceptance Criteria

- `_bmad-output/planning-artifacts/ux-design-directions.html` is self-contained and offline-safe.
- All 6 directions (A–F) are present.
- No v1 scope drift (single repo/workdir per window preserved).
- Privacy posture and consent principles are preserved (even if partially via callouts).
- Compacting does not omit the key v1 screens/flows required for informed design direction selection.

