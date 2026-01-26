# PRP: UX Step 9 (Design Directions) — Add “Paused” State (Post Codex Re‑Verification)

Owner: Claude BMad UX Team (Sally + Planning)  
Target branch: `feat/privacy-mode-253b`  
Verified head (Codex): `fa56e8b9e757b717d99e8fde03c76696af09fed2`  
Date: 2026-01-18  
Mode: Doc/mock update only (no product code changes)

---

## 0) Goal

Codex re-verified the Step 9 **fixpack PRP** as PASS, but the **original Step 9 PRP** still has one remaining FAIL in coverage completeness:

- Missing explicit representation of the workflow **“Paused”** state in `_bmad-output/planning-artifacts/ux-design-directions.html`.

This PRP instructs the minimal change required to make the original Step 9 PRP Task 7 pass, without regressing any previously fixed items.

Primary artifact to patch:
- `_bmad-output/planning-artifacts/ux-design-directions.html`

---

## 1) Non‑Negotiables (Do Not Regress)

- Offline-first: no runtime remote assets (no external fonts/icons/JS/CSS/CDNs).
- v1 repo model: single repo/workdir per window (no in-window multi-repo switching UI).
- Privacy posture terminology: “Always Local” preferred; no “remember/always allow”.
- Keep existing Step 9 fixpack additions intact:
  - Logs search + filter controls
  - “Copy CLI Command (redacted)” button
  - `.palette-input` focus-visible replacement styling
  - `prefers-reduced-motion` CSS

---

## 2) Required Fix (P0): Add “Paused” Workflow State Representation

Codex requirement: the compact Step 9 artifact must explicitly cover **running/paused/failed** states. “Running” and “Failed” are already present; only “Paused” is missing.

Implement **one** of the options below (Option A recommended).

### Option A (Recommended): Add a Paused Status Badge + Style

**A1) Add CSS style for paused badge**

In the `<style>` section near the existing status badge rules:
- `.status-badge.running`
- `.status-badge.idle`

Add:
- `.status-badge.paused` with an amber background + amber text.

Suggested (matches existing amber usage elsewhere in the file):
```css
.status-badge.paused {
  background: #fef3c7; /* amber-100-ish */
  color: #d97706;      /* amber-600-ish */
}
```

**A2) Change one workflow status from Idle → Paused**

Pick any one existing occurrence of:
```html
<span class="status-badge idle">Idle</span>
```

Change it to:
```html
<span class="status-badge paused">Paused</span>
```

Optional (nice, not required): change the adjacent CTA from “Execute” to “Resume” on that card if it makes sense in that mock.

### Option B (Alternate): Add a Paused Log Entry

If you want to avoid CSS changes, add a log entry in Direction F’s timeline that includes the word “Paused”, e.g.:
- `Workflow paused — awaiting user confirmation`

This is sufficient for the PRP’s text-based coverage check, but Option A is preferred because it visually represents paused state in the UI.

---

## 3) Self‑Verification (Run After Edit)

Confirm “Paused” now exists:
```bash
rg -n "(?i)paused" _bmad-output/planning-artifacts/ux-design-directions.html
```

Confirm running + failed still exist somewhere (should already be true):
```bash
rg -n "(?i)running" _bmad-output/planning-artifacts/ux-design-directions.html | head
rg -n "(?i)failed" _bmad-output/planning-artifacts/ux-design-directions.html | head
```

Confirm no regressions to offline-first:
```bash
rg -n "https?://|cdn|fonts\\.googleapis\\.com|<script\\s+src=|<link\\s+rel=\\\"stylesheet\\\"\\s+href=" _bmad-output/planning-artifacts/ux-design-directions.html || true
```

---

## 4) Codex Re‑Verification Request

After applying the change, ask Codex to rerun the **original** Step 9 PRP:
- `docs/prp-electron-wrapper-ux-step9-design-directions-codex-verification.md`

Success condition:
- Task 7 “Coverage completeness guardrail” becomes PASS.

