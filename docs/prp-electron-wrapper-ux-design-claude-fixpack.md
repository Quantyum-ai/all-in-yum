# PRP: UX Design Spec Fixpack (Post Codex Verification)

Owner: Claude BMad UX Team (Sally + Planning)
Target branch: `feat/privacy-mode-253b`
Last verified head: `fa56e8b9e757b717d99e8fde03c76696af09fed2`
Date: 2026-01-17

---

## 0) Goal

Address the issues reported by Codex verification so the UX Design artifacts are:

- Complete through **Step 8 (Visual Foundation)** in `_bmad-output/planning-artifacts/ux-design-specification.md`
- Fully aligned with PRD v1 scope (no multi-repo-in-window scope drift)
- Unambiguous about “UI does not redact cloud payloads” vs “UI masks secrets for local logs”

This PRP is **doc-only**. No code implementation changes.

---

## 1) Non-Negotiables

- No web searches; use only local repo documents already present.
- Preserve privacy posture:
  - No code/paths/diffs/stack traces/dependency trees to cloud without explicit per-action approval.
  - Always Local default; per-action consent for Hybrid; no “remember/always allow”.
  - Zero telemetry by default.
- CLI remains source of truth; Electron is a wrapper.
- Do not expand v1 scope beyond the PRD.

---

## 2) Current Issues (From Codex Report)

1) **BLOCKER:** Step 8 not saved/marked complete  
   - `_bmad-output/planning-artifacts/ux-design-specification.md` frontmatter has `stepsCompleted` only 1–7.

2) **Conflict:** Repo switching language vs PRD out-of-scope  
   - UX spec mentions in-window repo switching (“recent list”, “switch repos”), conflicting with PRD v1 out-of-scope: “Multi-repo management in single window”.

3) **Clarity:** “UI never applies redaction logic” vs local log masking  
   - Current wording reads contradictory; needs a precise distinction.

---

## 3) Fix Scope (Do These in Order)

### 3.1 Fix 1 — Save Step 8 Visual Foundation + Mark Complete (P0)

**File:** `_bmad-output/planning-artifacts/ux-design-specification.md`

**Do:**
- Append the full Step 8 content under a new section:
  - `## Visual Design Foundation`
    - `### Color System`
    - `### Typography System`
    - `### Spacing & Layout Foundation`
    - `### Accessibility Considerations`
- Ensure Step 8 includes (at minimum):
  - Light/dark semantic tokens (CSS variables preferred) compatible with shadcn/ui + Tailwind.
  - Offline-first constraints: **no runtime remote fonts/icons/assets**; system font stack; bundle icons locally.
  - Accessibility: strong focus rings, icon+label (not color-only), contrast AA+, `prefers-reduced-motion`.
  - Layout includes always-visible header with:
    - Active repo/workdir indicator (see Fix 2)
    - Privacy badge (Local/Hybrid)
    - Ollama health
    - Settings entrypoint
- Update frontmatter:
  - `stepsCompleted: [1,2,3,4,5,6,7,8]` (append `8` only; do not reorder prior steps)

**Acceptance criteria:**
- `stepsCompleted` includes `8`
- Step 8 content is present and reads as a complete “visual foundation” (not just token fragments)

---

### 3.2 Fix 2 — Resolve Repo Switching vs PRD Out-of-Scope (P0)

**PRD constraint:** `_bmad-output/prd/prd-electron-wrapper.md` lists “Multi-repo management in single window” as **out of scope** for v1.

**File:** `_bmad-output/planning-artifacts/ux-design-specification.md`

**Do (pick one, but make it explicit):**

**Option A (Recommended, minimal risk): Single repo per window (v1)**
- Keep an **Active Repo / Working Directory** indicator visible in the header.
- Remove “recent repos list” and “switch repos in-window” language for v1.
- Define v1 behavior as:
  - A window is tied to one repo/workdir.
  - “Open Repo…” opens a **new window** (or requires closing the current window) to avoid “multi-repo management in a single window”.
- If you want “recent repos” later, label it clearly as P2/future.

**Option B: Clarify PRD wording (only if owner approves)**
- If the product owner wants in-window switching, update PRD out-of-scope wording to clarify it means “simultaneous multi-repo dashboards”, not “switching the active repo”.
- Only do this if explicitly instructed; default to Option A.

**Acceptance criteria:**
- UX spec no longer contradicts the PRD on multi-repo scope.
- Repo context is still always visible (to prevent “wrong repo” operations).

---

### 3.3 Fix 3 — Clarify “No Cloud Redaction” vs “Local Log Masking” (P0)

**File:** `_bmad-output/planning-artifacts/ux-design-specification.md`

**Problem:** The spec says Electron “never applies redaction logic”, but also says Electron masks secrets before writing JSONL logs.

**Do:**
- Replace ambiguous wording with a crisp distinction:
  - **Cloud redaction / privacy decisions**: owned by CLI (`aiy-privacy`); Electron does not transform payloads for cloud.
  - **Local persistence safety**: Electron may **mask/scrub** sensitive patterns before writing local JSONL logs to disk (defense-in-depth), and must avoid logging credential inputs/args entirely.

**Acceptance criteria:**
- No apparent contradiction remains.
- Terms “redaction” (cloud boundary) vs “masking/scrubbing” (local logs) are used consistently.

---

## 4) Verification (Run After Fixes)

Run these local checks to confirm Codex’s blockers are resolved:

```bash
sed -n '1,60p' _bmad-output/planning-artifacts/ux-design-specification.md
rg -n "stepsCompleted" _bmad-output/planning-artifacts/ux-design-specification.md
```

Confirm Step 8 content exists:
```bash
rg -n "## Visual Design Foundation" _bmad-output/planning-artifacts/ux-design-specification.md
```

Confirm repo-scoping language is v1-compliant:
```bash
rg -n "recent repos|switch repos|repo selector|Open Repo|single repo|per window" _bmad-output/planning-artifacts/ux-design-specification.md
```

Confirm redaction vs masking wording is consistent:
```bash
rg -n "redaction|masking|scrub|JSONL|commands\\.jsonl" _bmad-output/planning-artifacts/ux-design-specification.md
```

Confirm no runtime remote assets are prescribed:
```bash
rg -n "https?://|cdn|fonts\\.googleapis\\.com|unpkg\\.com" _bmad-output/planning-artifacts/ux-design-specification.md || true
```

---

## 5) Deliverable

- Updated `_bmad-output/planning-artifacts/ux-design-specification.md` with:
  - Step 8 content appended
  - `stepsCompleted` updated to include `8`
  - Repo switching language aligned with v1 scope
  - Clear “cloud redaction vs local log masking” distinction

After these doc fixes, rerun the Codex verification PRP to confirm PASS.

