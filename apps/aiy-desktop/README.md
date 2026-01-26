# AIY Desktop - Electron Wrapper for aiy CLI

**Version**: 0.1.0 (Milestone 1 Complete)  
**Status**: ✅ Walking Skeleton Implemented  
**License**: MIT

---

## Project Overview

AIY Desktop is a privacy-first Electron desktop application that wraps the aiy CLI (Rust-based AI development assistant). The app provides a visual interface while maintaining strict privacy guarantees:

- **Always Local by default**: No code/data sent to cloud without explicit consent
- **CLI as source of truth**: Electron is a thin display layer, all business logic in Rust CLI
- **Security hardened**: Electron isolation, IPC allowlist, no shell execution
- **Offline-first**: Fully functional without internet access

---

## Quick Start

### Development

```bash
# Install dependencies
pnpm install

# Run in development mode
pnpm dev

# Run tests (100 tests)
pnpm test

# Type check
pnpm lint

# Build for production
pnpm build
```

### Prerequisites

- Node.js 22.x
- pnpm 9.x
- Electron 32.x (installed via pnpm)

### Environment Notes

- **Sandboxed Environments**: `pnpm dev` may fail with port binding errors (`listen EPERM :5173`) in restricted environments. This is an OS-level sandbox restriction, not an app issue.
- **Headless Linux**: GPU warnings are harmless (suppress with `ELECTRON_DISABLE_GPU=1`)
- **Electron Binary**: On first install, if `pnpm dev` fails with "Electron failed to install correctly", manually run:
  ```bash
  node node_modules/.pnpm/electron@32.2.5/node_modules/electron/install.js
  ```

---

## Architecture

### Process Model

- **Main Process**: CLI spawning, filesystem writes, IPC handling, window lifecycle
- **Preload**: Minimal typed API via contextBridge (NO direct ipcRenderer)
- **Renderer**: React UI only (NO Node APIs)

### Security Hardening

✅ `nodeIntegration: false`  
✅ `contextIsolation: true`  
✅ `sandbox: true`  
✅ NO `remote` module  
✅ NO `shell: true` for process spawning  
✅ IPC allowlist + schema validation  
✅ CSP headers configured

### Privacy Posture

- Default mode: **"always-local"** (hardcoded in privacy-store.ts)
- Mode persists via electron-store
- NO auto-detect mode flips
- Sensitive pattern scrubbing in logs (10 regex patterns)
- NO full request text persisted (only 100-char safe summary)

---

## Project Structure

```
apps/aiy-desktop/
├── src/
│   ├── main/                    # Main process (1,250 lines)
│   │   ├── index.ts             # App initialization, window creation
│   │   ├── cli-service.ts       # CLI spawning, streaming, process management
│   │   ├── ipc-handlers.ts      # IPC channels, validation, event forwarding
│   │   └── window-manager.ts    # Window lifecycle (placeholder)
│   ├── preload/                 # Preload (150 lines)
│   │   ├── index.ts             # contextBridge setup
│   │   └── api.ts               # Typed API for renderer
│   ├── renderer/                # Renderer (900 lines)
│   │   ├── App.tsx              # Root component
│   │   ├── components/          # React components
│   │   │   ├── layout/          # Header, Sidebar, MainContent
│   │   │   ├── logs/            # LogsPanel, LogLine, LogsControls
│   │   │   ├── privacy/         # PrivacyBadge
│   │   │   └── common/          # Button
│   │   ├── stores/              # Zustand stores (app, privacy, logs)
│   │   ├── hooks/               # use-ipc.ts
│   │   └── types/               # TypeScript types
│   └── shared/                  # Shared (2,100 lines)
│       ├── ipc-channels.ts      # IPC contract
│       ├── cli-service-types.ts # CLI service interface
│       ├── log-schema.ts        # JSONL schema + scrubbing
│       └── parsers/             # 6 CLI output parsers
├── tests/                       # Tests (800 lines)
│   ├── cli-service.test.ts      # 25 tests (CLI spawning, streaming, cancel)
│   ├── fixtures/
│   │   ├── fake-cli.js          # Fake aiy CLI for testing
│   │   ├── cli-outputs/         # 15 test fixtures
│   │   └── parser-tests/        # 5 test suites (75 tests)
│   └── setup.ts                 # Vitest setup
├── resources/
│   └── icon.png                 # App icon
├── package.json                 # pnpm workspace
├── electron.vite.config.ts      # Vite build config
├── vitest.config.ts             # Test config
├── tailwind.config.js           # Tailwind config
└── tsconfig.json                # TypeScript config
```

**Total**: 48 TypeScript files, ~4,200 lines of code

---

## Testing

### Test Suites (100 tests, 100% passing)

**CLI Service Tests** (25 tests):
- Process spawning with `shell: false`
- Stdout/stderr streaming
- Cancellation (cross-platform process tree kill)
- Timeout handling
- Job queue management
- JSONL logging with rotation
- Security verification

**Parser Tests** (75 tests):
- Privacy check: [PASS]/[FAIL] marker parsing
- Privacy status: Configuration extraction
- Workflow status: JSON state parsing
- Agents: Table parsing, credential detection
- Credentials: Provider status parsing

### Run Tests

```bash
# All tests
pnpm test

# With coverage
pnpm test:coverage

# With UI
pnpm test:ui
```

### Fake CLI Fixture

For testing without the real Rust binary, use `tests/fixtures/fake-cli.js`:

```bash
# Simulate commands
node tests/fixtures/fake-cli.js version
node tests/fixtures/fake-cli.js privacy check
node tests/fixtures/fake-cli.js agents list

# Test flags
node tests/fixtures/fake-cli.js --hang      # Hang indefinitely (test cancel)
node tests/fixtures/fake-cli.js --error     # Exit with code 1
node tests/fixtures/fake-cli.js --slow      # Take 35s (test timeout)
node tests/fixtures/fake-cli.js --stream    # Stream output over time
node tests/fixtures/fake-cli.js --stderr    # Output to both streams
```

---

## Implementation Status

### ✅ Milestone 1: Walking Skeleton (COMPLETE)

**Delivered**:
- CLI Service with spawning, streaming, cancellation
- IPC Handlers with validation and event forwarding
- Logs Panel UI with real-time display
- CLI Output Parsers for all commands
- 100/100 tests passing
- Dev server launches successfully

**Acceptance Criteria**: 4/4 met

### 📋 Milestone 2: Workflows (NEXT)

**Scope**:
- Repo selection flow
- Workflow init/status/execute/resume/cancel
- File watching (`.aiy/workflow-state.json` with chokidar)
- UI states: Ready/Executing/Paused/Completed/Failed
- Logs capture with JSONL rotation

### 📋 Milestone 3: Privacy Verification + Consent UX

**Scope**:
- Privacy Mode screen with `aiy privacy check` parsing
- CloudConsentModal with no "remember" option
- Mode toggle with persistence

### 📋 Milestone 4: Agents + Credentials

**Scope**:
- Agents screen with enable/disable controls
- Credentials screen with secure input handling
- Deep links for missing credentials

### 📋 Milestone 5: Packaging + Auto-Update

**Scope**:
- Platform builds (macOS universal, Windows, Linux AppImage)
- Code signing (macOS notarization, Windows Authenticode)
- Bundled CLI binaries per platform
- Opt-in auto-updater (default OFF, no telemetry)

---

## Development Notes

### Electron Binary Installation

If `pnpm dev` fails with "Electron failed to install correctly":

```bash
# Manually trigger Electron postinstall
node node_modules/.pnpm/electron@32.2.5/node_modules/electron/install.js

# Verify installation
npx electron --version  # Should output: v32.2.5
```

This is a known issue with pnpm where Electron's postinstall doesn't always run automatically.

### Running on Headless Linux

The app will show GPU warnings in headless environments (no X11 display). These are harmless but can be suppressed:

```bash
# Disable GPU acceleration for headless
export ELECTRON_DISABLE_GPU=1
pnpm dev
```

---

## Documentation

- **PRP**: `/docs/prp-electron-wrapper-implementation-claude-execution.md`
- **UX Spec**: `/_bmad-output/planning-artifacts/ux-design-specification.md`
- **PRD**: `/_bmad-output/prd/prd-electron-wrapper.md`
- **Milestone 1 Report**: `/docs/milestone-1-completion-report.md`
- **Implementation Status**: `/apps/aiy-desktop/IMPLEMENTATION-STATUS.md`

---

## Contributing

### Code Style

- **TypeScript**: Strict mode enabled
- **React**: Functional components with hooks
- **State**: Zustand for global state
- **Styling**: Tailwind CSS utility classes
- **Security**: NO `shell: true`, NO `innerHTML`, NO direct ipcRenderer

### Testing

- All new features require tests
- Parser tests must use fixtures from `tests/fixtures/cli-outputs/`
- Security tests verify NO `shell: true` in codebase

---

## License

MIT - See LICENSE file for details

---

## Credits

**Opus Agent Team** (Milestone 1):
- CLI Service Implementation (agent d1574c20)
- IPC Handlers Implementation (agent bb17db3a)
- Logs Panel UI Implementation (agent 8ff6602a)

**Sonnet Planning Team**:
- Project scaffold setup
- CLI output parsers + fixtures
- Type definitions + schemas
