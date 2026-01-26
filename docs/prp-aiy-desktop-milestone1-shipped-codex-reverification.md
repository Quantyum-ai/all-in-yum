# PRP: aiy Desktop — Milestone 1 “Shipped” Codex Re‑Verification

**Project**: all-in-yum / aiy Desktop (Electron wrapper around `aiy` CLI)  
**Target reviewer**: Codex verification team  
**Date**: 2026-01-19  
**Mode**: Verification / read-only (do not edit files unless explicitly authorized)  
**Constraints**: No web searches; no destructive commands; do not delete `node_modules/`; prefer `rg`/`sed`/`nl` for inspection.

---

## 1) Objective

Claude reports Milestone 1 is **shipped and verified**, claiming:

- ✅ Tests: **100/100** passing (6 suites)
- ✅ Security: no `execSync`, no `shell: true`, Electron hardening enabled
- ✅ TypeScript: `pnpm lint` passes
- ✅ Dev server: app launches successfully (note: sandbox may block port bind)
- ✅ Documentation: README + milestone/status docs present

Codex must independently re‑verify these claims against the **current working tree**.

---

## 2) Primary Artifact Under Test

- App root: `apps/aiy-desktop/`

---

## 3) Verification Tasks (PASS/FAIL + Evidence)

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

Deliverable: outputs verbatim.

---

### Task 2 — Artifact Presence + Inventory

```bash
cd /home/aip0rt/Desktop/all-in-yum/apps/aiy-desktop
ls -la
find src -type f | wc -l
find tests -type f | wc -l
find src tests -type f \\( -name '*.ts' -o -name '*.tsx' \\) | wc -l
```

PASS if `apps/aiy-desktop/` exists with populated `src/` and `tests/`.

---

### Task 3 — Electron Dist Present (prior blocker should stay fixed)

```bash
cd /home/aip0rt/Desktop/all-in-yum/apps/aiy-desktop
node -e "console.log(require('electron'))"
ls -la node_modules/.pnpm/electron@*/node_modules/electron/dist 2>/dev/null || true
cat node_modules/.pnpm/electron@*/node_modules/electron/dist/version 2>/dev/null || true
```

PASS if:
- `require('electron')` does not throw, and
- `dist/` exists, and
- `dist/version` exists (expect `32.2.5` unless intentionally changed).

Note: `pnpm exec electron --version` may fail as root due to Chromium sandbox; treat as non-blocking if `dist/version` exists.

---

### Task 4 — TypeScript / Lint

```bash
cd /home/aip0rt/Desktop/all-in-yum/apps/aiy-desktop
pnpm lint
```

PASS if exit code is 0.

---

### Task 5 — Tests (must be 100/100)

Run tests in non-watch mode:

```bash
cd /home/aip0rt/Desktop/all-in-yum/apps/aiy-desktop
pnpm exec vitest run
```

PASS if exit code is 0 and summary reports **100 passed (100)** across 6 files.

---

### Task 6 — Security Regression Scans (no `exec`/`execSync`, no `shell: true` in implementation)

```bash
cd /home/aip0rt/Desktop/all-in-yum/apps/aiy-desktop
rg -n "shell\\s*:\\s*true" src || true
rg -n "\\bexecSync\\(|\\bexec\\(" src || true
```

PASS if both commands return no matches in `src/`.

Note: `shell: true` may appear in **tests** only as a string in a negative test; that is acceptable.

---

### Task 7 — Electron Hardening (BrowserWindow + CSP + no remote)

```bash
cd /home/aip0rt/Desktop/all-in-yum/apps/aiy-desktop
rg -n "new BrowserWindow\\(|nodeIntegration\\s*:\\s*|contextIsolation\\s*:\\s*|sandbox\\s*:\\s*" src/main
sed -n '1,120p' src/main/index.ts

rg -n "@electron/remote|\\bremote\\b" src || true

rg -n "Content-Security-Policy|http-equiv=\\\"Content-Security-Policy\\\"" src/renderer/index.html src/main || true
sed -n '1,60p' src/renderer/index.html

rg -n "setWindowOpenHandler\\(|will-navigate" src/main || true
```

PASS if:
- BrowserWindow includes `nodeIntegration: false`, `contextIsolation: true`, `sandbox: true`.
- No `@electron/remote` usage.
- CSP is present (meta tag or header logic).
- Navigation/popups are blocked or explicitly handled.

---

### Task 8 — IPC Boundary (no renderer `ipcRenderer`)

```bash
cd /home/aip0rt/Desktop/all-in-yum/apps/aiy-desktop
rg -n "ipcRenderer" src/renderer || true
sed -n '1,220p' src/preload/index.ts
sed -n '1,260p' src/preload/api.ts
sed -n '1,260p' src/main/ipc-handlers.ts
sed -n '1,260p' src/shared/ipc-channels.ts
```

PASS if:
- No `ipcRenderer` is imported/used in renderer.
- Preload uses `contextBridge.exposeInMainWorld` and does not expose raw `ipcRenderer`.
- Main validates channels/requests against an allowlist/schema.

---

### Task 9 — CLI Service Runtime Semantics (streaming + close ordering + Windows kill is spawn-based)

Confirm process completion uses `close` (not `exit`) and Windows kill does not use `execSync`:

```bash
cd /home/aip0rt/Desktop/all-in-yum/apps/aiy-desktop
rg -n "child\\.on\\(['\\\"]close['\\\"]" src/main/cli-service.ts
rg -n "child\\.on\\(['\\\"]exit['\\\"]" src/main/cli-service.ts || true
rg -n "taskkill" src/main/cli-service.ts || true
nl -ba src/main/cli-service.ts | sed -n '360,660p'
```

PASS if:
- `child.on('close', ...)` is used for completion.
- No `execSync` remains.
- Windows kill strategy uses `spawn('taskkill', ...)` with `shell: false`.

---

### Task 10 — Logging Safety + Scrubbing (10 patterns; no full request persisted)

```bash
cd /home/aip0rt/Desktop/all-in-yum/apps/aiy-desktop
sed -n '1,140p' src/shared/log-schema.ts
rg -n "scrubSensitivePatterns\\(|truncateForLog\\(" src/main/cli-service.ts
rg -n "requestText|request_text|fullRequest|requestFull" src/shared src/main src/renderer || true
```

PASS if:
- Scrubber patterns exist (Claude claims 10; verify count in `log-schema.ts`).
- Persisted schema uses only safe summaries/hashes (no full request text fields).
- CLI service scrubs/truncates before writing JSONL.

---

### Task 11 — Fake CLI Deterministic Output (writeSync-based)

```bash
cd /home/aip0rt/Desktop/all-in-yum/apps/aiy-desktop
rg -n "writeSync\\(1|writeSync\\(2" tests/fixtures/fake-cli.js
node tests/fixtures/fake-cli.js version
node tests/fixtures/fake-cli.js --stderr
node tests/fixtures/fake-cli.js privacy check
```

PASS if:
- The fixture writes via fd1/fd2, and
- Commands print expected strings (version, stderr message, [PASS]).

---

### Task 12 — Dev Server Smoke (environment-dependent)

```bash
cd /home/aip0rt/Desktop/all-in-yum/apps/aiy-desktop
timeout 15s pnpm dev || true
```

PASS if:
- Output does **not** include “Electron failed to install correctly”, and
- Main/preload build steps run successfully.

If it fails with `listen EPERM` or display sandbox errors, record as **PASS with environment note**.

Also confirm dev server binds to IPv4 loopback in config:
```bash
rg -n "server:\\s*\\{\\s*host:\\s*'127\\.0\\.0\\.1'" electron.vite.config.ts || true
```

---

### Task 13 — Documentation / Handoff Presence (sanity)

Confirm these exist and are not obviously stale:
```bash
cd /home/aip0rt/Desktop/all-in-yum/apps/aiy-desktop
test -f README.md && echo "ok README" || echo "missing README"
test -f IMPLEMENTATION-STATUS.md && echo "ok status" || echo "missing status"
sed -n '1,120p' README.md
sed -n '1,140p' IMPLEMENTATION-STATUS.md
```

PASS if files exist and accurately describe Milestone 1 state (tests passing, security posture, environment notes).

---

## 4) Deliverable Format

Return:
- PASS/FAIL for Tasks 1–13.
- For each FAIL: minimal fix list with exact file path(s) and evidence (line refs if possible).
- For any environment notes (Task 12): state clearly whether the issue is environmental vs code/config.

---

## 5) Success Criteria

This PRP is considered successful if:
- Tasks 1–11 and 13 are PASS, and
- Task 12 is PASS or PASS-with-environment-note.

