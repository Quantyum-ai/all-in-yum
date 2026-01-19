# PRP: UX Step 11 (Component Strategy) — Codex Verification

**Project**: all-in-yum  
**Target Reviewer**: Codex Team  
**Date**: 2026-01-18  
**Review Mode**: Read-only verification (no edits unless explicitly requested)  
**Constraints**: Offline-only, no web searches, no network access

---

## Executive Summary

This PRP asks Codex Team to verify that **Step 11 (Component Strategy)** has been added to the canonical UX spec without losing the established constraints from Steps 1–10 (privacy posture semantics, CLI command correctness, offline-first, v1 scope).

Primary artifact under review:
- `_bmad-output/planning-artifacts/ux-design-specification.md`

Alignment references (no drift):
- `_bmad-output/prd/prd-electron-wrapper.md`
- `_bmad-output/briefs/product-brief-electron-wrapper.md`

---

## Non‑Negotiable Context (Must Remain True)

- **Always Local is default** (Electron-enforced v1), **Hybrid only via explicit user action**.
- **Mode persists** until user toggles (no auto-revert after cloud actions; no auto-flip due to detection/credentials).
- **CLI is source of truth**; UI does not redact/transform cloud payloads (may scrub/mask before persisting local logs).
- **Known CLI behavior**: `aiy privacy check` exit code unreliable → parse `[PASS]`/`[FAIL]`.
- **Init command correctness**: initialization uses `aiy privacy init` (no `aiy init` in this repo).
- **External links must be safe**: no auto-embedding sensitive strings (paths/diffs/stack traces) into URLs; require sanitized preview + explicit user consent.
- **v1 repo model**: single repo/workdir per window (no in-window multi-repo management).
- **Offline-first**: no runtime remote assets (fonts/icons/CDNs).

---

## Claims to Verify

| Claim | Expected Result |
|---|---|
| Step 11 recorded in frontmatter | `stepsCompleted` includes `11` in UX spec frontmatter |
| Step 11 exists in doc | A `##`/`###` section for Step 11 component strategy exists |
| Component hierarchy present | Defines a clear multi-layer component architecture (e.g., Screens → Feature → Layout → Shared) |
| Key components enumerated | Includes (at minimum) the components Sally listed (PrivacyBadge, WorkflowCard, LogEntry, AttentionCard, CloudConsentModal, ErrorPanel, CliNotFoundDialog, RepoInitDialog, SanitizedSearchModal, ReportIssueModal, PrivacyModeToggle, PrivacyVerificationPanel, AppShell/Header/Sidebar/CommandPalette) |
| State patterns described | Defines `PrivacyState` / workflow execution state patterns consistent with prior steps (no unsafe persistence of full request text) |
| Accessibility patterns captured | Focus-visible, keyboard navigation, focus trap for dialogs, reduced motion, icon+label not color-only |
| No drift on non-negotiables | Step 11 does not reintroduce prior fixed issues (auto-Hybrid, auto-revert, `aiy init`, unsafe external-link phrasing) |

---

## Verification Tasks

### 1) Record Repo State (Traceability)

```bash
git rev-parse --abbrev-ref HEAD
git rev-parse HEAD
git status --porcelain
```

---

### 2) Artifact Presence

```bash
ls -la _bmad-output/planning-artifacts/ux-design-specification.md
```

---

### 3) Frontmatter Step Completion

```bash
sed -n '1,40p' _bmad-output/planning-artifacts/ux-design-specification.md
```

Expected:
- `stepsCompleted` includes `11`.

---

### 4) Step 11 Section Presence

```bash
rg -n "Step 11|Component Strategy" _bmad-output/planning-artifacts/ux-design-specification.md
```

Expected:
- A distinct Step 11 section header and content (not just a mention in a list).

---

### 5) Component Strategy Content Completeness (Coverage Guardrail)

Verify the component hierarchy and the specific component set are present (names may vary slightly; use judgment, but don’t allow key flows to disappear due to compacting).

```bash
rg -n "Screens|Feature Components|Layout Components|Shared" _bmad-output/planning-artifacts/ux-design-specification.md
rg -n "PrivacyBadge|WorkflowCard|LogEntry|AttentionCard|CloudConsentModal|ErrorPanel" _bmad-output/planning-artifacts/ux-design-specification.md
rg -n "CliNotFoundDialog|RepoInitDialog" _bmad-output/planning-artifacts/ux-design-specification.md
rg -n "SanitizedSearchModal|ReportIssueModal" _bmad-output/planning-artifacts/ux-design-specification.md
rg -n "PrivacyModeToggle|PrivacyVerificationPanel" _bmad-output/planning-artifacts/ux-design-specification.md
rg -n "AppShell|Header|Sidebar|CommandPalette" _bmad-output/planning-artifacts/ux-design-specification.md
```

Expected:
- Matches for each key component (or an equivalent clearly-described substitute).
- If any are missing, mark **FAIL** and list what’s missing with file+line references.

---

### 6) Non‑Negotiables Regression Scan (Must Stay Fixed)

These were previously failing patterns. They must remain absent:

```bash
rg -n 'cloud features detected|Returns to "Always Local"|Badge changes to amber "Hybrid Mode"|`aiy init`|\\baiy init\\b|Search docs\" link with error text|opens GitHub with template' _bmad-output/planning-artifacts/ux-design-specification.md || true
```

Expected:
- No matches.

Confirm the correct semantics remain present:
```bash
rg -n 'current mode \\(persisted setting\\)|Hybrid.*explicit|Mode remains unchanged|aiy privacy init|preview/consent|sanitized' _bmad-output/planning-artifacts/ux-design-specification.md
```

Expected:
- Positive matches.

---

### 7) PRD Alignment (Spot Check)

Hybrid must remain explicitly enabled (not auto-detected), and repo model must remain single-repo-per-window.

```bash
rg -n "Hybrid.*explicit|Always Local|persist" _bmad-output/prd/prd-electron-wrapper.md | head
rg -n "single repo|per window|Open Repo" _bmad-output/planning-artifacts/ux-design-specification.md
rg -n "multi-repo|repo selector|switch repos|recent repos" _bmad-output/planning-artifacts/ux-design-specification.md || true
```

Expected:
- Step 11 doesn’t contradict PRD semantics or v1 scope.

---

### 8) CLI Command Correctness (Spot Check)

Step 11 may mention init/verification flows via component descriptions. Ensure any referenced CLI commands exist.

```bash
rg -n 'aiy privacy init|aiy privacy check|`aiy [^`]+`' _bmad-output/planning-artifacts/ux-design-specification.md
rg -n '\\bPrivacyCommands::Init\\b|\\bCheck\\b' crates/aiy-cli/src/main.rs
```

Expected:
- `aiy privacy init` and `aiy privacy check` remain consistent with CLI source.
- No `aiy init` references reappear.

---

## Deliverable (What Codex Team Should Return)

- PASS/FAIL per Verification Task (1–8)
- Evidence for each task using file+line references (`rg -n`, `sed -n`)
- If any FAIL: minimal wording-level fix list (exact file+line)

---

## Acceptance Criteria

- Step 11 is present, complete, and consistent with Steps 1–10 constraints.
- Non-negotiables remain intact (Hybrid explicit, mode persists, correct init command, safe external links).
- No v1 scope drift or offline-first regressions introduced by Step 11 component strategy.

