# PRP: aiy Desktop — Post‑PR Cleanup + Hardening Fixes (Claude Execution)

**Project**: all-in-yum / aiy Desktop  
**Branch under work**: `feat/aiy-desktop-m1` (already pushed)  
**Date**: 2026-01-19  
**Mode**: Implementation + git updates allowed  
**Constraints**: No web searches; do not commit secrets; avoid destructive shell; do not commit `node_modules/` anywhere.

---

## 0) Why This PRP Exists

Codex verified Milestone 1 is correct and passing, but identified a few cleanup/fix items needed before continuing to Milestone 2:

1) Repo root has an **untracked `node_modules/`** directory (should not appear in `git status`).  
2) `apps/aiy-desktop/package.json` still specifies `electron` as `^32.2.5` (caret); for offline reproducibility it should be pinned to `32.2.5`.  
3) Some documentation claims “dev server launches successfully” without noting sandbox environments may block port binding (`listen EPERM :5173`).  
4) Clarify secret-scan wording: example/test patterns may match (e.g., `AKIA...`, `sk-...`) but are not real secrets.

This PRP performs targeted fixes, creates a small follow-up commit on the existing PR branch, and updates the PR accordingly.

---

## 1) Stop‑Ship Policy Gate (Confirm Before Any Further Push)

The project posture originally stated “no code/paths/diffs to cloud (ever)”.

Since the branch is already pushed to `origin` (GitHub), **confirm with the owner** that:
- GitHub is explicitly approved for this repo/PR, OR
- This push was an exception and should be reverted/migrated to an approved internal remote.

If the owner says GitHub is *not* approved:
- Stop immediately and do not push more commits.
- Prepare an offline review bundle (`git bundle`) and coordinate remote migration.

---

## 2) Preflight (Evidence)

```bash
cd /home/aip0rt/Desktop/all-in-yum
git rev-parse --abbrev-ref HEAD
git status --porcelain
git remote -v
```

Expected:
- Branch `feat/aiy-desktop-m1`
- Untracked `node_modules/` at repo root currently shows up

---

## 3) Fix A — Ignore (and optionally remove) repo‑root `node_modules/`

### A1) Add ignore rule at repo root

Update repo root `.gitignore` to include:
- `/node_modules/`

Rationale: prevents accidental staging and keeps `git status` clean.

Note: per owner safety posture, **do not delete** the repo-root `node_modules/` unless explicitly approved. Ignoring is sufficient for git hygiene.

---

## 4) Fix B — Pin Electron Version for Offline Reproducibility

File: `apps/aiy-desktop/package.json`

Change:
- `"electron": "^32.2.5"` → `"electron": "32.2.5"`

Then ensure the lockfile is still consistent:
```bash
cd /home/aip0rt/Desktop/all-in-yum/apps/aiy-desktop
pnpm -v
pnpm lint
pnpm exec vitest run
```

If `pnpm-lock.yaml` changes, commit it (expected and acceptable).

---

## 5) Fix C — Documentation Accuracy (Dev Server + Environment Notes)

Update docs to avoid overstating dev-server success:

1) `apps/aiy-desktop/README.md`
   - Keep “Quick Start”, but add a short note:
     - `pnpm dev` may fail in sandboxed environments due to port-bind restrictions (`listen EPERM :5173`).
   - Update pnpm requirement if inaccurate (current environment uses pnpm 9.x; do not claim pnpm 8.x).
   - Optional: add a note that `pnpm exec electron --version` may fail when running as root in some sandboxes; in that case, verify Electron via `cat node_modules/.pnpm/electron@*/node_modules/electron/dist/version` or `node -e "console.log(require('electron'))"`.

2) `docs/milestone-1-completion-report.md` and/or `docs/milestone-1-final-handoff.md`
   - Ensure they mention the same environment note.
   - Clarify that Milestone 1 is verified by Codex PRPs and tests are deterministic.

Keep changes minimal and factual.

---

## 6) Fix D — Secret Scan Language (No Real Secrets, but Test Examples May Match)

Do not remove test/example content that is intentionally illustrative.

Instead, ensure docs that claim “secret scan clean” are precise:
- “No real secrets detected; example/test strings may match secret-like patterns.”

Only adjust wording; do not add new scanning scripts unless requested.

---

## 7) Post‑Fix Verification (Must Pass)

### 7A) Repo status clean (or only ignored files)
```bash
cd /home/aip0rt/Desktop/all-in-yum
git status --porcelain
```

Expected:
- No untracked `node_modules/` showing (either deleted or ignored).

### 7B) Milestone 1 still passes
```bash
cd /home/aip0rt/Desktop/all-in-yum/apps/aiy-desktop
pnpm lint
pnpm exec vitest run

rg -n "shell\\s*:\\s*true" src || true
rg -n "\\bexecSync\\(|\\bexec\\(" src || true
```

Expected:
- 100/100 passing
- no matches in `src/`

---

## 8) Commit + Push (Update Existing PR)

Create a single small follow-up commit on `feat/aiy-desktop-m1`:

```bash
cd /home/aip0rt/Desktop/all-in-yum
git add .gitignore apps/aiy-desktop/package.json apps/aiy-desktop/pnpm-lock.yaml apps/aiy-desktop/README.md docs/milestone-1-completion-report.md docs/milestone-1-final-handoff.md docs/prp-aiy-desktop-post-pr-cleanup-claude-execution.md || true
git commit -m "chore: post-PR cleanup (pin electron, ignore root node_modules, doc notes)"
git push
```

If the owner disallows further GitHub pushes, stop before `git push` and produce an offline bundle instead.

---

## 9) Optional — Ask Codex to Re-Verify

After pushing, request Codex to re-run:
- `docs/prp-aiy-desktop-milestone1-shipped-codex-reverification.md`

Expected: all PASS.

---

## 10) Done When

- Root `node_modules/` is ignored (and optionally removed)
- Electron version is pinned to `32.2.5` (and lockfile consistent)
- Docs accurately reflect environment constraints and secret-scan wording
- Tests remain 100/100 and lint passes
- PR updated with the follow-up commit (or offline bundle produced if GitHub not approved)
