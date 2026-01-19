# PRP: UX Steps 9–10 (Design Directions Summary + User Journey Flows) — Codex Verification

**Project**: all-in-yum  
**Target Reviewer**: Codex Team  
**Date**: 2026-01-18  
**Review Mode**: Read-only verification (no edits unless explicitly requested)  
**Constraints**: Offline-only, no web searches, no network access

---

## Executive Summary

This PRP asks Codex Team to independently verify that the BMad UX workflow outputs for **Step 9** and **Step 10** have been saved into the canonical UX spec without losing required constraints due to compacting.

Primary artifact under review:
- `_bmad-output/planning-artifacts/ux-design-specification.md` (should show `stepsCompleted` through **10**)

Step 9 reference artifact (must exist and match Step 9 summary claims):
- `_bmad-output/planning-artifacts/ux-design-directions.html`

Alignment references (for “no drift” checks):
- `_bmad-output/prd/prd-electron-wrapper.md`
- `_bmad-output/briefs/product-brief-electron-wrapper.md`
- `_bmad-output/analysis/research-takeaways-electron-wrapper.md`
- `_bmad-output/analysis/brainstorming-session-2026-01-16.md`
- `docs/research-electron-wrapper.md`

---

## Non-Negotiable Context (Must Remain True)

- **CLI is source of truth**; Electron is a wrapper/display layer.
- **Privacy posture**:
  - Always Local default (Electron-enforced v1).
  - Persistent indicator on all screens.
  - Per-action cloud confirmation with preview; **no “remember/always allow”**.
  - No code/paths/diffs/stack traces/dependency trees to cloud without explicit per-action approval.
- **Local logging safety**:
  - UI does **not** redact/transform cloud payloads.
  - UI **may** mask/scrub sensitive patterns before persisting local JSONL logs (defense-in-depth).
  - Never persist full request text across sessions; store only safe `request_summary`/optional hash.
- **Offline-first**:
  - No runtime remote assets (fonts/icons/CDNs).
  - System font stack; bundled/local assets only.
- **v1 repo model**:
  - **Single repo/workdir per window** (no in-window multi-repo management).
  - “Open Repo…” opens a new window (or otherwise stays v1-compliant).
- **Known CLI behavior**:
  - `aiy privacy check` exit code can be unreliable → parse `[PASS]`/`[FAIL]` markers.

---

## Claims to Verify

| Claim | Expected Result |
|---|---|
| UX spec steps updated | `_bmad-output/planning-artifacts/ux-design-specification.md` exists and `stepsCompleted` includes **9** and **10** |
| Step 9 summary saved | UX spec contains `## Design Directions (Step 9)` with: A–F comparison, recommended combined approach, coverage checklist, and a reference to the HTML artifact |
| Step 10 journeys saved | UX spec contains `## User Journey Flows (Step 10)` with Journeys 1–6, plus shortcuts summary + state machine diagram + per-journey success criteria |
| No drift vs PRD | Step 9/10 content doesn’t contradict PRD scope (especially repo model + privacy posture semantics) |
| CLI command references are correct | Any `aiy ...` commands mentioned in Step 9/10 exist in the repo’s CLI (no “invented” commands) |
| No privacy leakage via external links | “Open browser/GitHub/search” links don’t auto-embed sensitive text (paths, diffs, stack traces) without explicit approval and sanitization |

---

## Verification Tasks

### 1) Record Repo State (Traceability)

```bash
git rev-parse --abbrev-ref HEAD
git rev-parse HEAD
git status --porcelain
```

Note: planning artifacts may be untracked; still review them as present in working tree.

---

### 2) Artifact Presence & Basic Sanity

```bash
ls -la _bmad-output/planning-artifacts/ux-design-specification.md
ls -la _bmad-output/planning-artifacts/ux-design-directions.html
```

Expected:
- Both files exist.

---

### 3) Step Completion Marker + Inputs List (Frontmatter)

```bash
sed -n '1,60p' _bmad-output/planning-artifacts/ux-design-specification.md
```

Expected:
- `stepsCompleted` includes `9` and `10`.
- `inputDocuments` includes the PRD, product brief, brainstorming, takeaways, and the saved research report.

---

### 4) Step 9 Saved Correctly (Summary Content + Link to HTML)

Confirm Step 9 section exists and includes the claimed components:

```bash
rg -n "## Design Directions \\(Step 9\\)" _bmad-output/planning-artifacts/ux-design-specification.md
rg -n "Artifact:.*ux-design-directions\\.html" _bmad-output/planning-artifacts/ux-design-specification.md
rg -n "\\*\\*A:|\\*\\*B:|\\*\\*C:|\\*\\*D:|\\*\\*E:|\\*\\*F:" _bmad-output/planning-artifacts/ux-design-specification.md
rg -n "Recommended Combined Approach" _bmad-output/planning-artifacts/ux-design-specification.md
rg -n "Direction B|Direction C|Direction A|Direction F" _bmad-output/planning-artifacts/ux-design-specification.md
rg -n "Coverage Verified|Workflow states|Running|Paused|Failed|Logs filter/search|Copy CLI Command \\(redacted\\)|prefers-reduced-motion|Offline-first" _bmad-output/planning-artifacts/ux-design-specification.md
```

Expected:
- The UX spec explicitly references `_bmad-output/planning-artifacts/ux-design-directions.html`.
- A–F direction comparison exists (table or equivalent).
- “Recommended Combined Approach” is present and includes the intended mix.
- A “Coverage Verified” checklist exists and includes at least: workflow states, privacy verification markers, logs search/filter, “Copy CLI Command (redacted)”, a11y callout, offline-first callout.

Optional cross-check that the referenced HTML still contains the required elements (quick sanity, not a full Step 9 PRP re-run):
```bash
rg -n "Direction A:|Direction B:|Direction C:|Direction D:|Direction E:|Direction F:" _bmad-output/planning-artifacts/ux-design-directions.html
rg -n "Running|Paused|Failed|Search logs|Copy CLI Command \\(redacted\\)|\\[PASS\\]|\\[FAIL\\]|prefers-reduced-motion|:focus-visible" _bmad-output/planning-artifacts/ux-design-directions.html
```

---

### 5) Step 10 Saved Correctly (Journeys 1–6 + Required Additions)

Confirm Step 10 exists and includes Journeys 1–6, each with a Trigger and Success criteria, plus shortcuts + state machine:

```bash
rg -n "## User Journey Flows \\(Step 10\\)" _bmad-output/planning-artifacts/ux-design-specification.md
rg -n "### Journey 1:|### Journey 2:|### Journey 3:|### Journey 4:|### Journey 5:|### Journey 6:" _bmad-output/planning-artifacts/ux-design-specification.md
rg -n "\\*\\*Trigger:\\*\\*|\\*\\*Success criteria:\\*\\*" _bmad-output/planning-artifacts/ux-design-specification.md
rg -n "### Keyboard Shortcuts Summary" _bmad-output/planning-artifacts/ux-design-specification.md
rg -n "### Journey State Machine" _bmad-output/planning-artifacts/ux-design-specification.md
```

Expected:
- All 6 journeys exist.
- Each journey includes `**Trigger:**` and `**Success criteria:**`.
- Keyboard shortcuts summary table exists.
- Journey state machine diagram exists.

Optional (if the UX team claimed there is a summary table of all journeys near the top of Step 10): verify presence; if absent, note as “claim mismatch / optional improvement” rather than a hard failure.

---

### 6) PRD Alignment: Repo Model + Privacy Semantics (No Contradictions)

Check that Step 9/10 content does not reintroduce v1 out-of-scope multi-repo-in-one-window behaviors and keeps privacy semantics consistent with PRD:

```bash
rg -n "single repo|per window|Open Repo" _bmad-output/planning-artifacts/ux-design-specification.md
rg -n "multi-repo|in-window|repo selector|switch repos|recent repos" _bmad-output/planning-artifacts/ux-design-specification.md || true
rg -n "Out of Scope|multi-repo" _bmad-output/prd/prd-electron-wrapper.md
```

Additionally, sanity-check “Always Local default” semantics remain true in Step 10:
```bash
rg -n "Always Local = ON by default|Always Local" _bmad-output/prd/prd-electron-wrapper.md
rg -n "Always Local|Hybrid Mode|kill-switch|toggle" _bmad-output/planning-artifacts/ux-design-specification.md
```

Expected:
- UX spec preserves “single repo/workdir per window”.
- Step 10 does not imply in-window repo switching.
- “Always Local default” remains the default posture; Hybrid is explicit.

If Step 10 implies automatic mode changes (e.g., switching to Hybrid due to detected credentials, or auto-returning to Always Local after a cloud action), flag as a **potential PRD semantic conflict** and propose the minimal wording fix.

---

### 7) CLI Command Correctness (No Invented Commands)

Step 10 includes concrete CLI command examples; ensure they exist and are consistent with the actual CLI.

Extract likely command examples:
```bash
rg -n '`aiy [^`]+`' _bmad-output/planning-artifacts/ux-design-specification.md
rg -n '`aiy init`' _bmad-output/planning-artifacts/ux-design-specification.md || true
rg -n '`aiy privacy init`' _bmad-output/planning-artifacts/ux-design-specification.md || true
```

Then verify against CLI source:
```bash
rg -n \"enum Commands\" crates/aiy-cli/src/main.rs
rg -n \"PrivacyCommands\" crates/aiy-cli/src/main.rs
rg -n \"\\bInit\\b\" crates/aiy-cli/src/main.rs
rg -n \"aiy privacy init\" crates/aiy-cli/src || true
```

Expected:
- Any `aiy ...` commands referenced by the UX spec are supported by the CLI as implemented in-repo.
- Known required: `aiy privacy check` exists; `aiy privacy init` exists.

If a referenced command does not exist (or is ambiguous), mark as **FAIL** and propose the minimal correction to the UX spec (change the referenced command to the correct one).

---

### 8) External Links & Privacy Leakage Guardrail (Text-Level)

The UX spec can include “open browser/docs/GitHub” affordances, but must not implicitly leak sensitive data (paths, errors, stack traces) into cloud services without explicit approval and sanitization.

Search for risky phrasing:
```bash
rg -n \"opens browser|opens GitHub|Report issue|Search docs|with error text\" _bmad-output/planning-artifacts/ux-design-specification.md
```

Expected:
- Any external-sharing/link behavior is clearly user-initiated and does not auto-include sensitive details.
- If “Search docs with error text” or “Report issue with template” implies pre-filling potentially sensitive content, flag as **privacy risk** and recommend minimal wording: “copy sanitized error summary” / “open generic URL” / “confirm what will be shared”.

---

## Deliverable (What Codex Team Should Return)

- PASS/FAIL per Verification Task (1–8)
- Evidence for each task via `rg -n`/`sed` line references
- If any FAIL: minimal fix list (wording-level preferred) with exact locations in `_bmad-output/planning-artifacts/ux-design-specification.md`
- If everything passes: confirmation that the UX workflow can proceed to Step 11+ (or be marked complete if appropriate)

---

## Acceptance Criteria

- `_bmad-output/planning-artifacts/ux-design-specification.md` includes Step 9 + Step 10 and `stepsCompleted` includes 9,10.
- Step 9 summary accurately reflects the Step 9 HTML artifact (and links to it).
- Step 10 contains the required journeys, shortcuts, and state machine without contradicting PRD scope or privacy posture.
- CLI commands referenced by the UX spec are real and correct for this repo.
- No new privacy leakage risks are introduced by external-link behaviors described in Step 10.
