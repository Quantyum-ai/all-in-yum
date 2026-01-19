# PRP: aiy Desktop — Git Hygiene + PR Creation (Claude Execution)

**Project**: all-in-yum / aiy Desktop (Electron wrapper around `aiy` CLI)  
**Target executor**: Claude implementation team  
**Date**: 2026-01-19  
**Mode**: Repo modifications allowed (git operations allowed)  
**Constraints**: No web searches; avoid destructive shell; do not commit secrets; respect “no cloud exfiltration” policy.

---

## 0) Objective

Prepare the repository for review by:

1) Ensuring the working tree is clean and contains no secrets,  
2) Committing the Milestone 1 Electron app + planning artifacts (as approved), and  
3) Creating a PR for review.

---

## 1) Stop‑Ship Policy Gate (MUST CONFIRM BEFORE PUSH/PR)

The project’s non‑negotiable posture includes: **no code/paths/diffs/stack traces/dependency trees to cloud (ever)**.

Before pushing or opening any PR:

1) **Confirm the git remote is approved** (on‑prem/internal) and not a public cloud service.
2) If the remote is not approved, **do not push**. Instead:
   - Create a local branch + commits, and
   - Produce a `git bundle` file for offline review, or
   - Provide patch files (`git format-patch`) for transfer via an approved channel.

Deliverable: state explicitly which path you took (approved remote PR vs offline bundle/patch).

---

## 2) Artifact Strategy (Confirm Owner Intent)

Current working tree contains many untracked planning artifacts and documentation (BMad outputs, research report, PRPs, and the new Electron app).

**Default instruction (per owner request “push everything”)**:
- Commit **all** planning artifacts and implementation artifacts that are currently untracked **except** build/deps artifacts (e.g., `node_modules/`, `out/`, `dist/`, caches).

If the owner changes intent, stop and ask whether to:
- commit `_bmad-output/**` + `docs/research-electron-wrapper.md`, or
- keep them untracked and add to `.gitignore`, or
- commit them on a separate “planning” branch/PR.

---

## 3) Preflight: Ensure No Secrets / No Build Artifacts

### 3A) Confirm ignore rules prevent dependencies/build outputs from being committed

Validate these are ignored:
- `apps/aiy-desktop/node_modules/`
- `apps/aiy-desktop/out/`
- `apps/aiy-desktop/dist/`
- any `coverage/` output

Commands:
```bash
cd /home/aip0rt/Desktop/all-in-yum

# Show what would be added if we staged everything
git status --porcelain

# Confirm ignores (spot checks)
git check-ignore -v apps/aiy-desktop/node_modules 2>/dev/null || true
git check-ignore -v apps/aiy-desktop/out 2>/dev/null || true
git check-ignore -v apps/aiy-desktop/dist 2>/dev/null || true
```

If ignore rules are missing, add minimal ignore entries (prefer repo-root `.gitignore` additions like `**/node_modules/`), then re-check.

### 3B) Scan for likely secrets (must be clean before commit)

Commands (adjust patterns conservatively; do not print large files):
```bash
cd /home/aip0rt/Desktop/all-in-yum

rg -n --hidden --no-ignore -S "AKIA[0-9A-Z]{16}" . || true
rg -n --hidden --no-ignore -S "sk-[A-Za-z0-9]{20,}" . || true
rg -n --hidden --no-ignore -S "-----BEGIN (RSA|EC|OPENSSH|PGP) PRIVATE KEY-----" . || true
rg -n --hidden --no-ignore -S "(?i)api[_-]?key\\s*[:=]" . || true
rg -n --hidden --no-ignore -S "(?i)password\\s*[:=]" . || true
```

If anything matches:
- Remove/rotate the secret immediately (do not commit).
- Ensure `.env`, `credentials.json`, and similar are ignored (repo root `.gitignore` already has rules; verify).

---

## 4) Verification Before Commit (Must Pass)

### 4A) Milestone 1 verification (Electron app)
```bash
cd /home/aip0rt/Desktop/all-in-yum/apps/aiy-desktop
pnpm lint
pnpm exec vitest run
```

Expected: **100 passed (100)**.

### 4B) Sanity: no forbidden process execution primitives
```bash
cd /home/aip0rt/Desktop/all-in-yum/apps/aiy-desktop
rg -n "shell\\s*:\\s*true" src || true
rg -n "\\bexecSync\\(|\\bexec\\(" src || true
```

Expected: no matches.

---

## 5) Commit Plan (Keep Reviewable)

Create **two commits** (preferred):

1) **Commit A — Planning artifacts**
   - `_bmad-output/**`
   - `docs/research-electron-wrapper.md`
   - UX/PRD/verification PRPs under `docs/prp-*.md`

2) **Commit B — aiy Desktop Milestone 1 implementation**
   - `apps/aiy-desktop/**` (excluding ignored paths)
   - Any additional milestone handoff docs (e.g., `docs/milestone-*.md`)

Commands:
```bash
cd /home/aip0rt/Desktop/all-in-yum

# New branch for review (recommended)
git checkout -b feat/aiy-desktop-m1

# Commit A (planning artifacts)
git add _bmad-output docs/research-electron-wrapper.md docs/prp-*.md || true
git commit -m "planning: electron wrapper PRD + UX artifacts"

# Commit B (implementation)
git add apps/aiy-desktop docs/milestone-*.md || true
git commit -m "aiy-desktop: milestone 1 walking skeleton"

git status --porcelain
```

If `git add` tries to stage `node_modules/` or build outputs:
- Stop, fix ignore rules, `git reset`, and re-stage.

---

## 6) PR Creation (Approved Remote Only)

### 6A) Confirm remote
```bash
cd /home/aip0rt/Desktop/all-in-yum
git remote -v
```

If approved:
```bash
git push -u origin feat/aiy-desktop-m1
```

Create PR (method depends on tooling available):
- If `gh` CLI is available and approved: `gh pr create ...`
- Otherwise create PR in the approved internal Git UI.

PR title:
- `aiy Desktop: Milestone 1 (walking skeleton)`

PR description must include:
- What’s included (planning artifacts + Milestone 1 app)
- Key security posture (no shell, isolation, IPC allowlist, scrubbed logs)
- Test results (`pnpm lint`, `vitest run` summary)
- Known environment note: dev server may fail to bind in sandbox (`listen EPERM :5173`)

---

## 7) Offline Review Path (If Remote Not Approved)

If remote is not approved for push/PR:

```bash
cd /home/aip0rt/Desktop/all-in-yum

# Bundle the branch for transfer
git bundle create aiy-desktop-m1.bundle feat/aiy-desktop-m1

# Or create patch series
git format-patch --stdout HEAD~2..HEAD > aiy-desktop-m1.patch
```

Deliverables:
- `aiy-desktop-m1.bundle` (preferred) OR `aiy-desktop-m1.patch`
- A short README describing how to apply/review offline.

---

## 8) Completion Checklist

- [ ] Secret scan clean
- [ ] `apps/aiy-desktop` tests pass (100/100) and lint passes
- [ ] No `exec`/`execSync` and no `shell: true` in `src/`
- [ ] Two clean commits (planning + implementation)
- [ ] PR created on approved remote **or** offline bundle produced

