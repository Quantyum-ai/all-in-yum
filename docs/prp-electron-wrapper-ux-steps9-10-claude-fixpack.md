# PRP: UX Steps 9–10 — Claude Fixpack (Get to 100% PASS)

**Project**: all-in-yum  
**Target Executor**: Claude BMad UX Team  
**Date**: 2026-01-18  
**Mode**: Minimal edits only (wording-level; no redesign)  
**Constraints**: Offline-only; no web searches; no network access; preserve prior non-negotiables

---

## Executive Summary

Codex ran `docs/prp-electron-wrapper-ux-steps9-10-codex-verification.md` against the working tree and found Steps 9–10 are present, but **Tasks 6–8 FAIL** due to:

1) **PRD privacy-mode semantics drift** (Hybrid implied as auto-detected; auto-revert to Always Local)  
2) **Invented CLI command** (`aiy init` does not exist; correct is `aiy privacy init`)  
3) **External-link leakage risk** (“Search docs with error text” / “GitHub issue template” implies auto-sharing sensitive strings without explicit preview/consent)

This PRP instructs the Claude team to apply **minimal, wording-level fixes** in:
- `_bmad-output/planning-artifacts/ux-design-specification.md`

Then re-run the Codex PRP to confirm **100% PASS**.

---

## Non‑Negotiables (Do Not Break)

- **Always Local is the default posture** (Electron-enforced on first run; persisted in Electron storage).
- **Hybrid mode requires explicit user action** (toggle + confirmation as defined in PRD).
- **No automatic mode flips** based on “cloud features detected”, credentials presence, or privacy-check results.
- **No automatic reversion** from Hybrid → Always Local after a cloud action; mode persists until user toggles.
- **CLI commands referenced must exist in-repo**.
- **No external sharing** of sensitive data without explicit user preview + approval; never auto-embed error text/paths/stack traces in URLs.

---

## Source of Truth Failures (Codex Evidence)

Fix these exact locations in the UX spec:

1) **Auto-Hybrid on detection (semantic drift)**
   - `_bmad-output/planning-artifacts/ux-design-specification.md:992`
   - Current text implies: “Hybrid Mode (amber) if cloud features detected”

2) **Privacy verification changes mode (semantic drift)**
   - `_bmad-output/planning-artifacts/ux-design-specification.md:1158`
   - Current text implies: badge changes to Hybrid Mode on `[FAIL]`

3) **Auto-revert to Always Local after cloud action (semantic drift)**
   - `_bmad-output/planning-artifacts/ux-design-specification.md:1288`
   - Current text implies: returns to Always Local after completion

4) **Invented command**
   - `_bmad-output/planning-artifacts/ux-design-specification.md:979`
   - Current text runs: ``aiy init`` (not supported by this repo’s CLI)
   - CLI reality: `PrivacyCommands::Init` exists → correct is ``aiy privacy init`` (optionally `--path`)

5) **External-link leakage risk**
   - `_bmad-output/planning-artifacts/ux-design-specification.md:1345`
   - `_bmad-output/planning-artifacts/ux-design-specification.md:1346`
   - Current text suggests “Search docs with error text” and “Report issue opens GitHub with template” (implies prefill)

PRD references (for semantics alignment):
- Hybrid requires explicit toggle: `_bmad-output/prd/prd-electron-wrapper.md:95`
- Always Local default and persisted setting: `_bmad-output/prd/prd-electron-wrapper.md:94`

---

## Required Fixes (Minimal Wording Changes)

### Fix A — Always Local vs Hybrid Mode Semantics (No Auto-Flip)

**Edit location:** `_bmad-output/planning-artifacts/ux-design-specification.md:990-993`

**Goal:** Badge/mode reflects **user-selected persisted mode**, not detection.

**Required outcome wording (example; adjust as needed):**
- “Badge shows current mode (persisted setting):  
  - 🛡️ Always Local (green) by default on first launch  
  - ⚠️ Hybrid Mode (amber) only when the user explicitly enables Hybrid”
- If you want to mention detection, do so as **“Hybrid available” / “cloud credentials configured”** on the Privacy screen, **without changing mode**.

### Fix B — Privacy Verification Result Must Not Change Mode

**Edit location:** `_bmad-output/planning-artifacts/ux-design-specification.md:1151-1161`

**Goal:** `aiy privacy check` PASS/FAIL is a **verification result**, not a mode switch.

**Required outcome wording (example):**
- PASS: “Badge remains in current mode; show ‘All privacy checks passed’ + timestamp”
- FAIL: “Badge remains in current mode; show warning + failures + ‘How to fix’ links; optionally mark ‘Verification failed’ state in the Privacy screen”

Remove any text that implies the badge flips to Hybrid Mode because checks failed.

### Fix C — Cloud Consent Flow Must Not Auto-Revert Modes

**Edit location:** `_bmad-output/planning-artifacts/ux-design-specification.md:1286-1289`

**Goal:** Mode is persistent; cloud action completion doesn’t change it.

**Required outcome wording (example):**
- Replace “Returns to Always Local after completion” with:
  - “Mode remains unchanged after completion; if Hybrid is enabled, badge remains Hybrid until user toggles back to Always Local.”
- If you want to show transient execution info, frame it as **“cloud action in progress”** (temporary) distinct from mode.

### Fix D — Replace `aiy init` With Real CLI Command

**Edit location:** `_bmad-output/planning-artifacts/ux-design-specification.md:977-981`

**Goal:** Initialization aligns to actual CLI.

**Required change:**
- Change ``runs `aiy init``` → ``runs `aiy privacy init```  
- Optional: clarify execution model:
  - Either run with repo root as `cwd` and call ``aiy privacy init``
  - Or explicitly pass ``aiy privacy init --path <selected-repo>``

### Fix E — External Links Must Not Auto-Embed Sensitive Text

**Edit location:** `_bmad-output/planning-artifacts/ux-design-specification.md:1343-1347`

**Goal:** Links are user-initiated and **sanitized**, with explicit preview/consent before sharing anything externally.

**Required wording changes:**
- Replace “Search docs link with error text” with something like:
  - “Search docs link (opens browser) using a sanitized query (error code / short message only); show preview + user confirms what will be searched/shared.”
- Replace “Report issue (opens GitHub with template)” with something like:
  - “Report issue link opens GitHub issue template without auto-filled sensitive text; offer ‘Copy sanitized diagnostics’ and require explicit preview/consent before including anything in the report.”

---

## Self-Verification (Claude Team Must Do Before Hand-off)

Run these local checks after edits and include the results in your completion report:

```bash
rg -n 'cloud features detected|Returns to "Always Local"|Badge changes to amber "Hybrid Mode"|aiy init|Search docs" link with error text|opens GitHub with template' _bmad-output/planning-artifacts/ux-design-specification.md || true
```

Expected: **no matches**.

Confirm the corrected commands are present:
```bash
rg -n 'aiy privacy init' _bmad-output/planning-artifacts/ux-design-specification.md
rg -n 'aiy privacy check' _bmad-output/planning-artifacts/ux-design-specification.md
```

---

## Codex Re-Verification (Exit Criteria)

After fixes, Codex must be able to re-run:
- `docs/prp-electron-wrapper-ux-steps9-10-codex-verification.md`

Expected: **Tasks 1–8 PASS** with no remaining minimal fix list.

---

## Deliverable (What Claude Team Should Return)

- A short change log listing each fix (A–E) with exact line references in `_bmad-output/planning-artifacts/ux-design-specification.md`
- A self-verification snippet showing the `rg` checks above (or equivalent) returning expected results
- Confirmation that the PRD semantics are now aligned (Hybrid explicit; mode persistence; no auto-sharing)
