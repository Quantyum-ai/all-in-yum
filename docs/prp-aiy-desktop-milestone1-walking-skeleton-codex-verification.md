# PRP: aiy Desktop — Milestone 1 (Walking Skeleton) Codex Verification

**Project**: all-in-yum / aiy Desktop (Electron wrapper around `aiy` CLI)  
**Target reviewer**: Codex verification team  
**Date**: 2026-01-19  
**Mode**: Verification / read-only (do not edit files unless explicitly authorized)  
**Constraints**: No web searches; do not run destructive commands; do not delete `node_modules/`; prefer `rg`/`sed` for inspection.

---

## 1) Objective

Claude reports **Milestone 1 is complete** for `apps/aiy-desktop/` (walking skeleton / vertical slice) and claims:

- Electron app launches (dev server starts).
- Electron binary is installed (`v32.2.5`).
- `pnpm lint` passes; TypeScript compiles.
- Test suite passes (reported **100/100**).
- Security posture enforced: `sandbox: true`, `contextIsolation: true`, `nodeIntegration: false`, strict IPC allowlist + validation, no `remote`, no `shell: true`, CSP configured.
- Logging safety: sensitive pattern scrubbing (10 regex) and **no full request text persisted**.

Codex must independently verify these claims against the **current working tree**.

---

## 2) Artifacts Under Test

Primary:
- `apps/aiy-desktop/`

Key files (expected to exist):
- `apps/aiy-desktop/src/main/cli-service.ts`
- `apps/aiy-desktop/src/main/ipc-handlers.ts`
- `apps/aiy-desktop/src/main/index.ts`
- `apps/aiy-desktop/src/preload/index.ts`
- `apps/aiy-desktop/src/preload/api.ts`
- `apps/aiy-desktop/src/renderer/index.html`
- `apps/aiy-desktop/src/shared/ipc-channels.ts`
- `apps/aiy-desktop/src/shared/log-schema.ts`
- `apps/aiy-desktop/src/shared/parsers/*`
- `apps/aiy-desktop/tests/fixtures/fake-cli.js`

Reference docs (context only; do not modify):
- `docs/prp-electron-wrapper-implementation-claude-execution.md`
- `_bmad-output/planning-artifacts/ux-design-specification.md`
- `_bmad-output/prd/prd-electron-wrapper.md`

---

## 3) Verification Tasks (PASS/FAIL with Evidence)

### Task 1 — Repo State + Runtime Context (traceability)

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

### Task 2 — Artifact Presence + Basic Inventory

```bash
cd /home/aip0rt/Desktop/all-in-yum/apps/aiy-desktop
ls -la
find src -type f | wc -l
find tests -type f | wc -l
```

Also confirm key files exist:
```bash
test -f src/main/cli-service.ts && echo "ok cli-service" || echo "missing cli-service"
test -f src/main/ipc-handlers.ts && echo "ok ipc-handlers" || echo "missing ipc-handlers"
test -f src/preload/index.ts && echo "ok preload index" || echo "missing preload index"
test -f src/renderer/index.html && echo "ok renderer html" || echo "missing renderer html"
test -f src/shared/log-schema.ts && echo "ok log-schema" || echo "missing log-schema"
test -f tests/fixtures/fake-cli.js && echo "ok fake-cli" || echo "missing fake-cli"
```

PASS if the app scaffold and key files are present.

---

### Task 3 — Electron Binary Installation (the prior blocker)

Verify Electron resolves and has a dist installed:

```bash
cd /home/aip0rt/Desktop/all-in-yum/apps/aiy-desktop
node -e "console.log(require('electron'))"
pnpm exec electron --version

ls -la node_modules/.pnpm/electron@*/node_modules/electron/dist 2>/dev/null || true
ls -la node_modules/.pnpm/electron@*/node_modules/electron/path.txt 2>/dev/null || true
```

PASS if:
- `require('electron')` does **not** throw, and
- `pnpm exec electron --version` prints an Electron version (expected `v32.2.5`), and
- `dist/` exists under the installed electron package.

If the version is not `v32.2.5`, record as FAIL unless there’s a deliberate bump documented in-repo.

---

### Task 4 — TypeScript / Lint

```bash
cd /home/aip0rt/Desktop/all-in-yum/apps/aiy-desktop
pnpm lint
```

PASS if exit code is 0.

---

### Task 5 — Tests (reported 100/100)

```bash
cd /home/aip0rt/Desktop/all-in-yum/apps/aiy-desktop
pnpm test
```

Deliverable:
- PASS if tests exit 0.
- Record the reported test count from output (Claude claims 100 tests).

---

### Task 6 — Security Hardening Regression Scan (static checks)

#### 6A) “No `shell: true` / no exec()”
```bash
cd /home/aip0rt/Desktop/all-in-yum/apps/aiy-desktop
rg -n "shell\\s*:\\s*true" src tests || true
rg -n "\\bexecSync\\(|\\bexec\\(" src tests || true
```

PASS if no matches for `shell: true` and no `exec()`/`execSync()` usage (spawn is expected).

#### 6B) Electron window hardening flags
```bash
cd /home/aip0rt/Desktop/all-in-yum/apps/aiy-desktop
rg -n "new BrowserWindow\\(|nodeIntegration\\s*:\\s*|contextIsolation\\s*:\\s*|sandbox\\s*:\\s*" src/main
```

Then show the BrowserWindow creation snippet:
```bash
sed -n '1,220p' src/main/index.ts
```

PASS if BrowserWindow webPreferences include:
- `nodeIntegration: false`
- `contextIsolation: true`
- `sandbox: true`

#### 6C) No remote module usage
```bash
rg -n "@electron/remote|\\bremote\\b" src || true
```

PASS if `@electron/remote` is absent and there’s no Electron `remote` usage.

#### 6D) CSP + navigation/popup restrictions
```bash
rg -n "Content-Security-Policy|http-equiv=\\\"Content-Security-Policy\\\"" src/renderer/index.html src/main || true
rg -n "setWindowOpenHandler\\(|will-navigate|will-redirect|navigation" src/main || true
sed -n '1,200p' src/renderer/index.html
```

PASS if:
- CSP is present (meta tag or response header injection), and
- navigation/popups are blocked (or explicitly handled) in main.

---

### Task 7 — IPC Boundary + Allowlist Validation

Verify:
- Renderer does **not** import/use `ipcRenderer` directly.
- Preload exposes a minimal API via `contextBridge`.
- Main validates allowed channels.

Commands:
```bash
cd /home/aip0rt/Desktop/all-in-yum/apps/aiy-desktop
rg -n "ipcRenderer" src/renderer src/preload || true
sed -n '1,220p' src/preload/index.ts
sed -n '1,260p' src/preload/api.ts
sed -n '1,260p' src/shared/ipc-channels.ts
sed -n '1,260p' src/main/ipc-handlers.ts
```

PASS if:
- No `ipcRenderer` usage in renderer code.
- Preload uses `contextBridge.exposeInMainWorld` (or equivalent) and does not expose raw `ipcRenderer`.
- Main side handlers validate channels/requests against an allowlist/schema.

---

### Task 8 — Logging Safety + Scrubbing Claim

Verify:
- Scrubbing patterns exist (Claude claims 10 regex).
- Persisted log schema does **not** store full request text across sessions (only summary/hash).

Commands:
```bash
cd /home/aip0rt/Desktop/all-in-yum/apps/aiy-desktop
sed -n '1,260p' src/shared/log-schema.ts
rg -n "request(_text|Text|Full|full)|requestSummary|request_hash|hash" src/shared src/main src/renderer || true
```

PASS if:
- There is an explicit scrubber/mask step before writing any persisted log lines, and
- No persisted log schema stores raw “full request text” (only safe summary + optional hash/ID), and
- Secrets are never written to logs (confirm by scanning for known secret keys patterns in fixture outputs, and ensure scrubber would catch them).

If the “10 regex” claim is not literally true, record as FAIL **only if** the scrubber is materially weaker than described; otherwise record as PASS with a note.

---

### Task 9 — Dev Server Launch (smoke; environment-dependent)

Goal: confirm the prior “Electron failed to install correctly” runtime error is gone.

Run with a short timeout so the command doesn’t hang indefinitely:
```bash
cd /home/aip0rt/Desktop/all-in-yum/apps/aiy-desktop
timeout 15s pnpm dev || true
```

PASS if:
- The output does **not** include “Electron failed to install correctly”.
- The dev server indicates it started (or at minimum, progressed beyond Electron resolution).

If it fails due to sandbox constraints (e.g., port bind restrictions or no display), record as **PASS with environment note** if the Electron resolution is successful and the failure is clearly unrelated.

---

## 4) Report Format (Required)

Provide:
- PASS/FAIL for Tasks 1–9.
- For each FAIL: a minimal, actionable fix list with exact file paths and (when possible) line references.
- For any environment-dependent outcomes (Task 9): note whether the failure is environmental vs code/config.

---

## 5) Success Criteria

This PRP is considered successful if:
- Tasks 1–8 are PASS, and
- Task 9 is PASS or PASS-with-environment-note (but not failing due to missing Electron dist).

