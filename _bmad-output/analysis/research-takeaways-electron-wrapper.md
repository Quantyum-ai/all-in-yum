# Research Takeaways: Electron Desktop Wrapper for aiy CLI

## Executive Summary

The research confirms that successful CLI-centric desktop apps (GitHub Desktop, Docker Desktop, 1Password) emphasize robust process management, cross-platform resilience, and user trust through transparency. Our Electron wrapper should leverage `child_process.spawn` for streaming CLI output, implement per-repo command queuing with reader/writer lock semantics, and enforce a layered privacy defense with "Always Local" as the default posture. Critical finding: several CLI commands lack JSON output, requiring v1 to parse text output or schedule CLI enhancements.

## Key Architectural Decisions

### 1. CLI Process Management
- Use `child_process.spawn` (no shell) for all CLI invocations
- Stream stdout/stderr in real-time; avoid buffering deadlocks
- Track all spawned processes; kill on app exit to prevent orphans

### 2. Command Queue
- Per-repo serialization for workflow/privacy commands
- Allow parallel execution for read-only commands (agents list, credentials status)
- Implement cancellation via SIGTERM with process tree cleanup

### 3. File Watching
- Use chokidar v4 with native events
- Enable `awaitWriteFinish` (stabilityThreshold: 200-500ms) for write completion
- Debounce rapid changes (100-200ms)
- Fallback to polling on network drives, WSL, or watch failures (2-5s interval)

### 4. Binary Bundling & Distribution
- Bundle aiy Rust binary in app resources using `asarUnpack`
- Resolve path via `process.resourcesPath`
- Code sign binary on macOS (Developer ID + notarization) and Windows (Authenticode)
- Support fallback: bundled -> user-specified path -> PATH lookup

### 5. Logging
- Electron captures all CLI stdout/stderr -> JSONL logs
- Schema: `{timestamp, command, args, exitCode, stdout, stderr, durationMs}`
- Rotation: size-based (50MB max) or time-based (5 files)
- Location: `~/.config/aiy-desktop/logs/commands.jsonl`
- NEVER log: credentials, secrets, actual code content

### 6. Privacy UX (Layered Defense)
- **Layer 1 - Kill-Switch:** "Always Local" toggle (Electron-enforced in v1)
  - When ON: hide/disable all cloud UI; queue refuses cloud commands
  - Persists in Electron local storage
- **Layer 2 - Modal Gate:** Per-action cloud confirmation with redacted preview
  - Show exactly what metadata will be sent
  - No "remember this choice" to prevent complacency
- **Layer 3 - Indicator:** Persistent badge on all screens
  - Green/shield = Always Local, Yellow/warning = Hybrid enabled
- **Zero telemetry default:** No analytics, crash reporting, or usage tracking

## Corrections to Apply

These corrections must be incorporated into PRD requirements:

1. **CLI JSON output gaps:** Many commands (agents, credentials, privacy check) have NO JSON output today. V1 must parse text output or schedule "add JSON output" as CLI enhancement.

2. **Privacy check exit code:** `aiy privacy check` exits 0 even on [FAIL]. Parse output for `[PASS]`/`[FAIL]` markers instead.

3. **workflow-state.json limitations:** This file is metadata-only (state enum, timestamps). Original request text is NOT stored. V1 retrieves workflow context from:
   - Electron's JSONL command logs (parse recent `aiy privacy execute` args)
   - In-memory UI state for active session

4. **Always Local enforcement:** V1 is Electron-enforced only. CLI does not expose a `privacy.mode` config flag today. CLI-level kill-switch is future work (P2-8).

## Risk Mitigations

| Risk | Mitigation |
|------|------------|
| Orphaned processes | Track all spawned PIDs; kill process tree on app exit |
| Cross-platform quirks | Platform-specific code paths (taskkill on Windows, SIGTERM on POSIX) |
| File watch failures | Detect errors, auto-fallback to polling, show warning in UI |
| Bundled binary trust | Code sign CLI with same cert as app; include in notarization |
| Privacy regression | Layered defense; no "remember" options; audit log of cloud calls |

## Sources

Key citations from research (see docs/research-electron-wrapper.md for full bibliography):
- Node.js Child Process docs: spawn for streaming, exec only for simple cases
- GitHub Desktop Engineering Blog: AsyncReaderWriterLock pattern for concurrency
- Chokidar v4: awaitWriteFinish for stable file reads
- 1Password CLI Integration Security: secure IPC and biometric auth patterns
- Pieces.app: local-first, opt-in cloud features model
