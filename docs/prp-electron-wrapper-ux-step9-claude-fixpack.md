# PRP: UX Step 9 (Design Directions) Fixpack — Post Codex Verification

Owner: Claude BMad UX Team (Sally + Planning)  
Target branch: `feat/privacy-mode-253b`  
Verified head (from Codex report): `fa56e8b9e757b717d99e8fde03c76696af09fed2`  
Date: 2026-01-18  
Mode: Doc/mock updates only (no product code changes)

---

## 0) Goal

Update the Step 9 HTML design directions showcase so it passes Codex verification for:

- Coverage completeness (Logs filter/search + explicit “Copy CLI Command (redacted)”)
- Accessibility regression scan (no `outline: none` without a visible replacement focus style)

Primary artifact to patch:
- `_bmad-output/planning-artifacts/ux-design-directions.html`

This PRP assumes Step 1–8 UX spec is already correct and passing; Step 9 mockups must not drop or contradict that context.

---

## 1) Non‑Negotiables (Context That Must Remain True)

- **Offline-first:** No runtime remote assets (no external fonts/icons/JS/CSS/CDNs). System font stack only; inline SVG/icons; all styles embedded.
- **Privacy posture:**
  - Always Local is the default posture (Electron-enforced v1).
  - Persistent privacy indicator on all screens.
  - Hybrid actions require explicit per-action consent; **no “remember/always allow”**.
  - No code/paths/diffs/stack traces/dependency trees to cloud without explicit per-action approval.
- **Logging safety:** Never persist full request text across sessions; logs store only safe `request_summary`/optional hash. UI does not redact/transform cloud payloads; UI may mask/scrub secrets before persisting local logs.
- **v1 repo model:** Single repo/workdir per window (no in-window multi-repo management). “Open Repo…” = new window.
- **Known CLI behavior:** `aiy privacy check` exit code unreliable → parse `[PASS]`/`[FAIL]` markers.

---

## 2) Issues To Fix (From Codex Step 9 Verification)

### 2.1 Coverage Gap (FAIL)

Missing/partial in `_bmad-output/planning-artifacts/ux-design-directions.html`:

- **Logs filter/search UI**: only referenced as a future note; not visually represented in the Logs view.
- **Explicit “Copy CLI Command (redacted)” affordance/callout**: only “Copy Log” exists; there is a visible CLI line in the log, but no explicit button/text for copying a redacted CLI invocation.

### 2.2 Accessibility Regression (FAIL)

- `.palette-input` includes `outline: none;` with **no** replacement focus style (`:focus` / `:focus-visible`) defined.

### 2.3 Optional Consistency Polish (Not required for pass, recommended)

- Rename “100% Local Mode” / “Local” labels to **“Always Local”** to match `_bmad-output/planning-artifacts/ux-design-specification.md`.
- Rename “Recommended Hybrid Approach” (meaning hybridizing design directions) to **“Recommended Combined Approach”** to avoid confusion with “Hybrid privacy mode”.
- Note: prefers-reduced-motion isn’t represented in Step 9 mockups; acceptable if Step 8 UX spec remains the source, but adding a tiny CSS snippet is a low-risk improvement.

---

## 3) Fix Scope (Do These in Order)

### 3.1 Add Logs Filter/Search Controls (P0)

**File:** `_bmad-output/planning-artifacts/ux-design-directions.html`

**Add a minimal Logs toolbar** to the Logs-focused direction (Direction F), and optionally show the same toolbar pattern in one other direction’s Logs screen.

Minimum acceptable visual representation:
- A search input labeled “Search logs…” (or similar)
- A simple filter control (e.g., dropdown for level: All/Info/Warn/Error, and/or time range)
- These can be non-functional (mock), but must be clearly present in the UI

**Do not** introduce external JS/CSS or external assets.

---

### 3.2 Add Explicit “Copy CLI Command (redacted)” Affordance (P0)

**File:** `_bmad-output/planning-artifacts/ux-design-directions.html`

In Direction F (and/or in a compact callout block near the log timeline), add:
- A button labeled exactly **“Copy CLI Command (redacted)”**
- Clarify it is distinct from “Copy Log”

Acceptable options:
- Replace “Copy Log” with two buttons: “Copy Log” + “Copy CLI Command (redacted)”
- Or keep “Copy Log” and add the new button next to it

If you show a sample CLI line in the log, keep it **obviously illustrative** (avoid real repo file paths; do not leak code).

---

### 3.3 Fix Focus Styling for `.palette-input` (P0)

**File:** `_bmad-output/planning-artifacts/ux-design-directions.html`

Remove the a11y anti-pattern:
- Either remove `outline: none;`, **or** add a visible replacement focus style.

Minimum acceptable replacement:
- `.palette-input:focus-visible { outline: 2px solid <accent>; outline-offset: 2px; }`
- Or an equivalent box-shadow focus ring.

Optional (recommended): ensure buttons/interactive elements have some visible focus indicator too, but the Codex failure is currently tied to `.palette-input`.

---

### 3.4 Optional Improvements (P1)

These are not required for PRP pass but are recommended to preserve context and reduce confusion:

1) **Terminology alignment**
- Change “Local” / “100% Local Mode” labels to “Always Local”.

2) **Avoid “Hybrid” ambiguity**
- Change “Recommended Hybrid Approach” to “Recommended Combined Approach”.

3) **Reduced motion hint (safe)**
- Add:
  - `@media (prefers-reduced-motion: reduce) { * { animation: none !important; transition: none !important; } }`
  - Only if the mock uses animations/transitions; otherwise include as a comment-level placeholder.

4) **Cloud consent coverage note (optional)**
- Step 9 PRP noted cloud-consent UI wasn’t represented. If easy, add a compact callout (non-functional) showing:
  - “Hybrid action → Confirm modal → payload preview → Send to Cloud”
  - Explicitly “no remember/always allow”

---

## 4) Self‑Verification (Run After Edits)

Re-run the Step 9 checks locally (no web):

```bash
# confirm the two required additions are now present
rg -n "Copy CLI Command \\(redacted\\)|Search logs|Filter" _bmad-output/planning-artifacts/ux-design-directions.html

# confirm focus ring exists and outline: none is not left unhandled
rg -n "\\.palette-input|outline: none|:focus|:focus-visible" _bmad-output/planning-artifacts/ux-design-directions.html

# confirm still offline-safe
rg -n "https?://|cdn|fonts\\.googleapis\\.com|<script\\s+src=|<link\\s+rel=\\\"stylesheet\\\"\\s+href=" _bmad-output/planning-artifacts/ux-design-directions.html || true
```

Expected outcomes:
- Matches for “Copy CLI Command (redacted)” and “Search logs” (and at least one filter control).
- If `outline: none` remains, there must be an explicit focus ring rule in the same stylesheet.
- No external asset references.

---

## 5) Deliverable

- Updated `_bmad-output/planning-artifacts/ux-design-directions.html` that passes:
  - Step 9 Task 7 (coverage completeness)
  - Step 9 Task 8 (accessibility regression scan)
- Optional: minor terminology polish (“Always Local”, “Combined Approach”) if you choose to include it.

After this, request Codex to rerun `docs/prp-electron-wrapper-ux-step9-design-directions-codex-verification.md`.

