# PRP: aiy Desktop — Electron Install Hold‑Up (Codex Investigation)

**Project**: all-in-yum / aiy Desktop (Electron wrapper around `aiy` CLI)  
**Target reviewer**: Codex Team  
**Date**: 2026-01-19  
**Mode**: Investigation / verification first (read-only unless explicitly authorized)  
**Constraints**: No web searches; prefer offline-safe checks; avoid destructive commands outside generated `node_modules/`

---

## 1) Objective

Claude reports that the new Electron app scaffold and most implementation work is in place, but **Electron’s binary is not installed** (“Electron failed to install correctly”), blocking `pnpm dev` / `electron-vite dev`.

Codex must:
1) Reproduce the failure,  
2) Identify the root cause (configuration vs network vs platform/tooling), and  
3) Provide a minimal, actionable fix plan (and optionally a patch plan, but do not edit unless authorized).

---

## 2) Primary Artifact Under Test

- Electron app root: `apps/aiy-desktop/`

Expected symptom:
- Running any command that needs the Electron executable fails with:
  - `Error: Electron failed to install correctly, please delete node_modules/electron and try installing again`

---

## 3) Investigation Hypotheses (Likely Causes)

Check these in order:

1) **pnpm lifecycle scripts disabled** (`ignore-scripts=true`) → Electron postinstall never downloads `dist/` binary.
2) **Network policy blocks GitHub/electron downloads** while allowing npm registry (common in locked-down environments).
3) **Filesystem permissions / cache path issues** prevent Electron from writing into its cache/dist.
4) **Node.js version/tooling incompatibility** (less likely, but verify).

---

## 4) Required Commands (Evidence‑Backed)

### Task A — Record Repo + Runtime Context

```bash
cd /home/aip0rt/Desktop/all-in-yum
git rev-parse --abbrev-ref HEAD
git rev-parse HEAD
git status --porcelain

node -v
pnpm -v
uname -s
uname -m
```

Deliverable: include outputs verbatim.

---

### Task B — Reproduce the Electron Failure (Minimal)

```bash
cd /home/aip0rt/Desktop/all-in-yum/apps/aiy-desktop
node -e "console.log(require('electron'))"
```

Expected: it throws “Electron failed to install correctly”.

Also check if the binary exists where Electron expects it:
```bash
ls -la node_modules/.pnpm/electron@*/node_modules/electron/dist 2>/dev/null || true
```

Deliverable: confirm whether `dist/` exists (and whether it contains an `electron` executable).

---

### Task C — Check for Disabled Lifecycle Scripts (Highest Likelihood)

```bash
cd /home/aip0rt/Desktop/all-in-yum/apps/aiy-desktop
pnpm config get ignore-scripts
```

If `true`, confirm it’s not just local to one command by checking config sources (best-effort):
```bash
pnpm config list
```

Deliverable:
- Report whether `ignore-scripts` is set and where it appears to come from (global/user/project).

---

### Task D — Check Electron Download Constraints (If Scripts Are Enabled OR Will Be Enabled)

If scripts are disabled, **do not** attempt downloads yet—first document the policy constraint.

If scripts can be enabled for this project, perform *connectivity-only* checks (no web search; short timeouts):
```bash
cd /home/aip0rt/Desktop/all-in-yum/apps/aiy-desktop
env | rg -n '^(ELECTRON_|npm_config_electron|npm_config_registry|https?_proxy|no_proxy|NO_PROXY)=' || true

# HEAD request only; keep timeouts short
curl -I --max-time 5 https://github.com/electron/electron/releases/download/v32.2.5/ 2>/dev/null | head -n 5 || true
```

Deliverable:
- Whether GitHub/electron downloads appear blocked.
- Whether proxy env vars are set that might affect downloads.

---

### Task E — Minimal Fix Plan (Do Not Apply Unless Authorized)

Based on findings:

**If `ignore-scripts=true` is the blocker:**
- Propose the lowest-risk way to enable scripts **only** for `apps/aiy-desktop` (not global), e.g.:
  - command-scoped: `pnpm install --ignore-scripts=false` (run from `apps/aiy-desktop`)
  - or project-scoped config (if supported): local `.npmrc` inside `apps/aiy-desktop` with `ignore-scripts=false`
- Then re-run:
  - `pnpm install`
  - `node -e "console.log(require('electron'))"`
  - `pnpm exec electron --version`

**If GitHub downloads are blocked:**
- Propose setting an Electron mirror (enterprise/internal artifact proxy) via `ELECTRON_MIRROR` / `npm_config_electron_mirror` (do not hardcode a public mirror without approval).
- Document required artifact(s): Electron zip for v32.2.5 linux-x64/win32-x64/darwin-x64/darwin-arm64.

**If filesystem/cache permissions are blocked:**
- Identify the cache dir Electron is trying to use (typically `~/.cache/electron`) and propose a writable override (e.g., `ELECTRON_CACHE_DIR`).

Deliverable: a concrete “Fix Option A/B/C” list with exact commands and expected outputs.

---

## 5) Success Criteria

Codex report must include:
- Root cause determination (scripts disabled vs download blocked vs other).
- Minimal fix plan that preserves safety (scoped changes, no broad `rm -rf` outside `apps/aiy-desktop/node_modules`).
- A re-test checklist showing how to confirm the fix:
  - `require('electron')` returns a path
  - `pnpm exec electron --version` works
  - `pnpm dev` (or `electron-vite dev`) starts

If not solvable under current environment constraints (e.g., outbound downloads forbidden), clearly state what policy/infra change is required (e.g., internal mirror URL).

