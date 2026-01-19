# PRP: UX Step 8 (Visual Foundation) — Codex Verification

**Project**: all-in-yum  
**Target Reviewer**: Codex Team  
**Date**: 2026-01-18  
**Review Mode**: Read-only verification (no edits unless explicitly requested)  
**Constraints**: Offline-only, no web searches, no network access

---

## Executive Summary

This PRP asks Codex Team to verify that **UX Design Workflow Step 8 (Visual Foundation)** has been correctly captured in the repo’s UX design specification and that it is:

- Present and marked complete (`stepsCompleted` includes `8`)
- Internally coherent (tokens, typography, spacing, layout, component patterns, dark mode)
- Consistent with prior UX decisions (shadcn/ui + Tailwind; offline-first; accessibility; privacy posture)
- Aligned with PRD v1 scope (no accidental “multi-repo management in a single window” scope creep via “repo selector” language)

Primary file under review:
- `_bmad-output/planning-artifacts/ux-design-specification.md`

Reference/context files (for alignment checks):
- `_bmad-output/prd/prd-electron-wrapper.md`
- `_bmad-output/briefs/product-brief-electron-wrapper.md`

---

## Review Target

Record the repo state under review:
```bash
git rev-parse HEAD
git status --porcelain
```

---

## Claims to Verify

| Claim | Expected Result |
|---|---|
| Step 8 is saved | `_bmad-output/planning-artifacts/ux-design-specification.md` contains a “Visual Design Foundation” section with color/typography/spacing/layout/dark-mode content |
| Step 8 marked complete | `stepsCompleted` includes `8` in the UX spec frontmatter |
| Offline-first visual stack | System font stack specified; no runtime remote fonts/icons/assets prescribed |
| Accessibility requirements present | Focus rings, keyboard-first, icon+label (not color-only), contrast targets, `prefers-reduced-motion` guidance |
| Token strategy implementable | Light/dark tokens and semantic status colors map cleanly to Tailwind/shadcn patterns (CSS variables preferred) |
| Layout matches UX direction | Header + sidebar + main content described, with key context always visible (active repo/workdir, privacy badge, Ollama health) |
| PRD v1 scope preserved | “Repo selector” does not imply multi-repo management within one window (or is explicitly defined as single-repo-per-window / open-in-new-window) |

---

## Verification Tasks

### 1) Confirm Step 8 Completion Marker

Inspect UX spec frontmatter:
```bash
sed -n '1,80p' _bmad-output/planning-artifacts/ux-design-specification.md
```

Expected:
- `stepsCompleted` includes `8`

If not present: **FAIL/BLOCKED** (Step 8 not completed).

---

### 2) Confirm Step 8 Content Exists (Not Just Token Fragments)

Find the Step 8 section header(s):
```bash
rg -n "## Visual Design Foundation|# Step 8|Visual Foundation" _bmad-output/planning-artifacts/ux-design-specification.md
```

Then spot-check that Step 8 includes all required subsections:
```bash
rg -n "### Color System|### Typography System|### Spacing|### Layout|Dark Mode|prefers-color-scheme" _bmad-output/planning-artifacts/ux-design-specification.md
```

Expected:
- Color system with light/dark palette + semantic status colors (privacy/health/workflow states)
- Typography system with system font stacks and type scale
- Spacing scale and layout structure (header/sidebar/main)
- Dark mode strategy (system preference + manual override)

If any are missing: **FAIL** with a minimal “what’s missing” list.

---

### 3) Offline-First: No Runtime Remote Assets

Verify the UX spec does not prescribe runtime remote assets (fonts/icons/CDNs):
```bash
rg -n "https?://|cdn|fonts\\.googleapis\\.com|use\\.typekit\\.net|unpkg\\.com|jsdelivr\\.net" _bmad-output/planning-artifacts/ux-design-specification.md || true
```

Expected:
- No matches (or only examples clearly labeled “DO NOT USE”).

---

### 4) Accessibility Checks (Visual Foundation)

Verify Step 8 (or surrounding sections) explicitly includes:
- Strong focus ring guidance
- Non-color cues (icon + label) for privacy/health/status
- Contrast target (AA+ or explicit contrast guidance)
- Reduced motion guidance (`prefers-reduced-motion`)

Suggested checks:
```bash
rg -n "focus ring|focus|keyboard|icon\\+label|not color|contrast|AA|prefers-reduced-motion|reduced motion" _bmad-output/planning-artifacts/ux-design-specification.md
```

Expected:
- At least one explicit statement for each bullet above.

If absent: **FAIL** with “missing accessibility callouts” list.

---

### 5) Token & Component Strategy Implementability (shadcn/ui + Tailwind)

Verify Step 8 aligns with the chosen design system approach:
- Semantic tokens map to Tailwind/shadcn conventions
- CSS variables (preferred) are used or clearly planned for light/dark themes
- Status colors match privacy/health/workflow semantics (Local=green, Hybrid=amber, Error=red, Running=blue)

Suggested checks:
```bash
rg -n "shadcn|Tailwind|Radix|CSS variables|--font-sans|--font-mono|green-|amber-|blue-|red-" _bmad-output/planning-artifacts/ux-design-specification.md
```

Expected:
- Coherent mapping, no contradictory token naming.

---

### 6) Layout Context: Active Repo/Workdir vs PRD Scope

Verify that any “Repo Selector” language is v1-compliant with PRD out-of-scope “multi-repo management in single window”.

Check PRD out-of-scope reference:
```bash
rg -n "Out of Scope|Multi-repo" _bmad-output/prd/prd-electron-wrapper.md
```

Check UX spec “repo selector” language:
```bash
rg -n "Repo Selector|repo selector|recent repos|switch repos|Open Repo|single repo|per window|new window" _bmad-output/planning-artifacts/ux-design-specification.md
```

Expected (v1-safe):
- Either “active repo/workdir indicator” (no switching), or
- Switching opens a new window / closes current window (explicitly avoiding multi-repo management within one window).

If UX spec implies in-window switching + multi-repo management: **FAIL** and propose minimal wording changes.

---

## Deliverable (What Codex Team Should Return)

- PASS/FAIL per Verification Task (1–6)
- If FAIL: a minimal “doc fix list” (wording-level) with exact section headings to edit
- Confirm whether Step 8 is actually saved and marked complete

---

## Acceptance Criteria

- `_bmad-output/planning-artifacts/ux-design-specification.md` includes Step 8 content and `stepsCompleted` includes `8`.
- Step 8 defines a coherent, offline-first, accessible visual foundation implementable with shadcn/ui + Tailwind.
- No PRD v1 scope drift (especially around repo switching / multi-repo in a single window).

