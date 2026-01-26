# PRP: UX Step 9 Fixpack — Codex Re‑Verification

**Project**: all-in-yum  
**Target Reviewer**: Codex Team  
**Date**: 2026-01-18  
**Review Mode**: Read-only verification (no edits unless explicitly requested)  
**Constraints**: Offline-only, no web searches, no network access

---

## Executive Summary

Claude BMad UX team applied a **fixpack** to the Step 9 design directions HTML to address Codex failures:

- Coverage gap: add **Logs filter/search UI** + explicit **“Copy CLI Command (redacted)”** affordance
- Accessibility regression: fix `.palette-input` focus styling (no `outline: none` without replacement)

This PRP asks Codex to re-verify that these fixes are present and that **compact Step 9** still preserves the full context we established in Steps 1–8 (offline-first, privacy posture, v1 scope, CLI-as-source-of-truth).

Primary artifact under review:
- `_bmad-output/planning-artifacts/ux-design-directions.html`

Context references (for “no drift” checks):
- `_bmad-output/planning-artifacts/ux-design-specification.md`
- `_bmad-output/prd/prd-electron-wrapper.md`

---

## Non‑Negotiable Context (Must Remain True)

- **Offline-first:** no runtime remote assets (fonts/icons/JS/CSS/CDNs); system font stack; embedded styles only.
- **v1 repo model:** single repo/workdir per window; no in-window multi-repo management; “Open Repo…” implies new window.
- **Privacy posture:** Always Local default; persistent indicator; per-action cloud confirmation w/ preview; no “remember/always allow”.
- **Cloud boundary:** UI does not redact/transform cloud payloads; CLI owns privacy decisions/redaction. UI may mask/scrub secrets before persisting local logs.

---

## Verification Tasks

### 1) Record Repo State (for traceability)

```bash
git rev-parse HEAD
git status --porcelain
```

Note: planning artifacts may be untracked; still review them as present in working tree.

---

### 2) Confirm Artifact Exists

```bash
ls -la _bmad-output/planning-artifacts/ux-design-directions.html
```

---

### 3) Confirm Fixpack Items (P0) Are Present

**A) Logs filter/search UI exists (Direction F toolbar)**
```bash
rg -n "Search logs\\.{3}|Search logs" _bmad-output/planning-artifacts/ux-design-directions.html
rg -n "Filter|<select" _bmad-output/planning-artifacts/ux-design-directions.html
```

Expected: a visible search input and at least one filter control (dropdown/select) in the Logs-focused mock.

**B) “Copy CLI Command (redacted)” affordance exists**
```bash
rg -n "Copy CLI Command \\(redacted\\)" _bmad-output/planning-artifacts/ux-design-directions.html
```

Expected: an explicit button/label distinct from “Copy Log”.

**C) `.palette-input` focus style is accessible**
```bash
rg -n "\\.palette-input" _bmad-output/planning-artifacts/ux-design-directions.html
rg -n "outline:\\s*none" _bmad-output/planning-artifacts/ux-design-directions.html || true
rg -n "\\.palette-input:focus-visible|:focus-visible" _bmad-output/planning-artifacts/ux-design-directions.html
```

Expected:
- If `outline: none` exists, there must be a visible replacement focus style (prefer `:focus-visible`) for `.palette-input`.

---

### 4) Confirm Optional Polish (P1) Didn’t Break Anything

These are not required for product correctness, but Codex should confirm they are benign and improve alignment:

```bash
# terminology alignment
rg -n "Always Local" _bmad-output/planning-artifacts/ux-design-directions.html
rg -n "100% Local Mode" _bmad-output/planning-artifacts/ux-design-directions.html || true

# avoid confusing “Hybrid Approach” wording (design-direction mixing vs privacy mode)
rg -n "Recommended Combined Approach" _bmad-output/planning-artifacts/ux-design-directions.html
rg -n "Recommended Hybrid Approach" _bmad-output/planning-artifacts/ux-design-directions.html || true

# reduced motion (optional but good)
rg -n "prefers-reduced-motion" _bmad-output/planning-artifacts/ux-design-directions.html || true
```

Expected:
- “Always Local” present at least once.
- “Recommended Combined Approach” present; “Recommended Hybrid Approach” absent.
- `prefers-reduced-motion` may be present; if absent, note only (Step 8 UX spec remains source).

---

### 5) Re-check Offline-First: No Runtime Remote Assets

```bash
rg -n "https?://|cdn|fonts\\.googleapis\\.com|use\\.typekit\\.net|unpkg\\.com|jsdelivr\\.net|<script\\s+src=|<link\\s+rel=\\\"stylesheet\\\"\\s+href=" _bmad-output/planning-artifacts/ux-design-directions.html || true
```

Expected: no matches.

---

### 6) Re-check v1 Scope (No Multi‑Repo In‑Window)

```bash
rg -n "recent repos|switch repos|repo dropdown|repo selector|open other" _bmad-output/planning-artifacts/ux-design-directions.html || true
```

Expected: no matches.

---

## Deliverable (What Codex Team Should Return)

- PASS/FAIL for Tasks 1–6
- If FAIL: minimal fix list (exact strings/sections to edit)
- If PASS: confirm Step 9 is now acceptable to proceed to Step 10 in the UX workflow

