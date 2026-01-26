# PRP: aiy Desktop — Post‑PR Cleanup + PR Claims (Codex Verification)

**Repo**: `/home/aip0rt/Desktop/all-in-yum`  
**Expected branch**: `feat/aiy-desktop-m1`  
**Scope**: Read‑only verification (no edits, no commits, no pushes)  
**No web**: Do not use web search; do not verify GitHub PR via network.  

## 0) Purpose

Verify the Claude team’s claims that:

1) Milestone 1 implementation is present and still green (lint + tests).  
2) Post‑PR cleanup fixes landed (pinned Electron version, root `node_modules/` ignored, docs updated).  
3) Repo is clean (`git status`), security scans remain clean.  
4) PR/commit claims are consistent with local git state (without contacting GitHub).

Claude’s referenced PRPs (inputs for context only; do not assume they were executed):
- `docs/prp-electron-wrapper-implementation-claude-execution.md`
- `docs/prp-aiy-desktop-git-pr-claude-execution.md`
- `docs/prp-aiy-desktop-post-pr-cleanup-claude-execution.md`

## 1) Output Format (Deliverable)

Produce a PASS/FAIL checklist for Tasks 1–8 (below) with:
- The exact command(s) executed (verbatim).
- Minimal evidence (key output lines + file/line refs where applicable).
- A minimal fix list for any FAILs (file path + what to change).

## 2) Tasks

### Task 1 — Repo State + Clean Working Tree

```bash
cd /home/aip0rt/Desktop/all-in-yum
git rev-parse --abbrev-ref HEAD
git rev-parse HEAD
git status --porcelain
```

**PASS if**:
- Branch is `feat/aiy-desktop-m1`
- `git status --porcelain` is empty (or only expected ignored files; no `?? node_modules/` at repo root).

---

### Task 2 — Commit Topology (3‑commit claim + delta sanity)

Capture the last ~10 commits and confirm the `aiy-desktop` commits exist, and that a cleanup commit exists after the Milestone 1 commit.

```bash
cd /home/aip0rt/Desktop/all-in-yum
git --no-pager log --oneline -10
git merge-base HEAD fa56e8b9e757b717d99e8fde03c76696af09fed2
git --no-pager log --oneline fa56e8b9e757b717d99e8fde03c76696af09fed2..HEAD
```

**PASS if**:
- The commit list includes the planning commit (`b4026b2` or equivalent), the Milestone 1 implementation commit (`4a6fd7a` or equivalent), and an additional cleanup commit after it (message may vary but must include the cleanup changes verified in Tasks 3–5).

**Note**: Do not require an exact PR number or remote confirmation; verify only local git history.

---

### Task 3 — Root `node_modules/` Hygiene (ignored or absent)

```bash
cd /home/aip0rt/Desktop/all-in-yum
test -d node_modules && echo "root node_modules exists" || echo "root node_modules absent"
git check-ignore -v node_modules/ || true
git ls-files | rg -n '^node_modules/' || true
```

**PASS if**:
- Either `node_modules/` at repo root is absent, **or** it exists but is ignored by root `.gitignore`, and no `node_modules/` paths are tracked.

Also capture the root ignore rule:
```bash
cd /home/aip0rt/Desktop/all-in-yum
rg -n '^/node_modules/$|^node_modules/$' .gitignore || true
```

---

### Task 4 — Electron Version Pinned (reproducibility)

Verify Electron is pinned in `apps/aiy-desktop/package.json` and lockfile resolves accordingly.

```bash
cd /home/aip0rt/Desktop/all-in-yum/apps/aiy-desktop
node -e "const p=require('./package.json'); console.log(p.devDependencies.electron)"
rg -n '^\\s*\"electron\"\\s*:\\s*\"\\^' package.json || true
rg -n '^\\s*\"electron\"\\s*:\\s*\"32\\.2\\.5\"' package.json || true
rg -n 'electron@32\\.2\\.5' pnpm-lock.yaml | head -n 5 || true
```

**PASS if**:
- `package.json` uses `"electron": "32.2.5"` (no caret), and lockfile still resolves to 32.2.5 (or at minimum does not resolve to a different version).

---

### Task 5 — Docs Updated (sandbox dev‑server note + pnpm requirement)

Verify docs do **not** overclaim dev‑server success in restricted sandboxes, and pnpm requirement is correct.

```bash
cd /home/aip0rt/Desktop/all-in-yum
rg -n 'pnpm\\s+8\\.x|pnpm\\s+9\\.x|listen\\s+EPERM|5173|sandbox' apps/aiy-desktop/README.md docs/milestone-1-completion-report.md docs/milestone-1-final-handoff.md || true
```

**PASS if**:
- `apps/aiy-desktop/README.md` does **not** incorrectly require pnpm 8.x if the project standard is pnpm 9.x.
- Milestone docs include an environment note that dev‑server port binding can fail in sandboxes (or otherwise avoid claiming “dev server runs” unconditionally).

---

### Task 6 — “Secret Scan” Claim (no real secrets; no accidental tracked creds)

This repo has many placeholder strings (tests/docs). Verification here is:
- No credential files tracked.
- No secret‑looking patterns in **implementation source** (`apps/aiy-desktop/src`) and **new docs** except fixtures/examples.

```bash
cd /home/aip0rt/Desktop/all-in-yum
git ls-files | rg -n '(^|/)\\.env(\\.|$)|credentials\\.json$|\\.enc$|\\.salt$' || true

# Scan app source only (exclude tests/fixtures)
rg -n --glob '!apps/aiy-desktop/tests/**' --glob '!**/node_modules/**' \
  '(AKIA[0-9A-Z]{16})|(sk-[A-Za-z0-9]{20,})|(ghp_[A-Za-z0-9]{30,})|(AIza[0-9A-Za-z\\-_]{35})|(-----BEGIN (RSA|OPENSSH|EC) PRIVATE KEY-----)' \
  apps/aiy-desktop/src docs || true
```

**PASS if**:
- No tracked credential files found.
- No secret‑looking matches found in `apps/aiy-desktop/src`.
- Any matches in `docs/` (if any) are clearly placeholders and do not include real keys (call out any hits explicitly).

---

### Task 7 — Milestone 1 Still Green (lint + tests)

```bash
cd /home/aip0rt/Desktop/all-in-yum/apps/aiy-desktop
pnpm lint
pnpm exec vitest run
```

**PASS if**:
- `pnpm lint` exits 0
- `pnpm exec vitest run` exits 0 and reports **100 passed**

---

### Task 8 — Security Regression Scans (no exec/execSync; no shell:true)

```bash
cd /home/aip0rt/Desktop/all-in-yum/apps/aiy-desktop
rg -n 'shell\\s*:\\s*true' src || true
rg -n '\\bexecSync\\(|\\bexec\\(' src || true
```

**PASS if**:
- No matches in `src/`

---

## 3) Optional / Informational (Do Not Gate PASS)

### PR URL Verification (Unverified Here)

Do not attempt to `curl` GitHub or otherwise reach external networks unless the owner explicitly authorizes outbound access.

Instead, capture local remote + upstream info:
```bash
cd /home/aip0rt/Desktop/all-in-yum
git remote -v
git branch -vv | rg -n 'feat/aiy-desktop-m1' || true
```

Report this as **INFO** only.

