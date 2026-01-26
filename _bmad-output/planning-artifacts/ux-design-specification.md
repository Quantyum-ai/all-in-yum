---
stepsCompleted: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11]
inputDocuments:
  - _bmad-output/briefs/product-brief-electron-wrapper.md
  - _bmad-output/prd/prd-electron-wrapper.md
  - _bmad-output/analysis/research-takeaways-electron-wrapper.md
  - _bmad-output/analysis/brainstorming-session-2026-01-16.md
  - docs/research-electron-wrapper.md
---

# UX Design Specification: aiy Desktop

**Author:** airpln
**Date:** 2026-01-16

---

## Executive Summary

### Project Vision

aiy Desktop is a thin Electron wrapper around the aiy CLI that provides visual access to AI-assisted development workflows while enforcing strict privacy boundaries. The CLI remains the single source of truth; Electron acts as a "dumb display layer" that never redacts/transforms cloud payloads or makes privacy decisions (the CLI owns those). Electron may still mask/scrub sensitive patterns before persisting local logs to disk (defense-in-depth), without affecting cloud boundaries.

**Top UX Goal:** Trust + Speed
- Privacy posture must be instantly recognizable (persistent indicator on all screens)
- Common workflows must be keyboard-first for power users but discoverable for occasional users
- Cloud actions require explicit friction (confirmation modal, no "always allow" option)

**Privacy Non-Negotiables:**
- No code/paths/diffs/stack traces/dependency trees to cloud without explicit per-action approval
- Cloud planning only with redacted metadata
- Zero telemetry, analytics, or crash reporting by default
- All state, logs, and configuration stored locally only

**Design Philosophy:** Offline-first, cloud-additive. The app must be fully functional in Always Local mode with no degraded states for constrained/airgapped environments.

### Target Users

| Persona | Profile | Critical UX Needs |
|---------|---------|-------------------|
| **Alex (Security-Conscious)** | Defense/finance sectors, audits tools before trusting | Verification capabilities, audit trail, credential isolation, unmistakable privacy posture |
| **Jordan (Power User)** | Dozens of workflows/day, keyboard-driven | Keyboard shortcuts, command palette, queue visibility, <3 clicks for common actions |
| **Sam (Occasional User)** | Weekly user, forgets context between sessions | State recovery, context reminders, guided actions, attention cards on launch |

### Key Design Challenges

1. **Privacy Trust Without Paranoia** - Users need confidence their data stays local without constant anxiety. The privacy indicator must be reassuring, not alarming. Security-conscious users need to *verify*, not just *believe*.

2. **Power User Efficiency vs. Occasional User Discoverability** - Command palette and keyboard shortcuts for power users; obvious buttons and contextual guidance for occasional users. Same UI must serve both without feeling cluttered or hidden.

3. **CLI Transparency** - When something fails, users need to understand what the CLI said, not generic errors. Error messages must be actionable with suggested fixes.

4. **State Recovery Across Sessions** - Returning users must instantly know: What was I doing? Where did I leave off? Attention cards and workflow context must answer this at a glance.

5. **Cross-Platform Consistency** - macOS, Windows, Linux users expect native behaviors while maintaining consistent UX patterns across platforms.

6. **CLI Output + Hybrid Gap** - Many `aiy` commands output text-only (no JSON), requiring UI to parse human-readable output with format drift risk. Mitigation: version-locking CLI with app, future CLI JSON flags. Additionally, "Hybrid" cloud-planning UX may be partially unavailable until CLI cloud-planning wiring is complete. Design must include clear "feature unavailable" states with safe local-only fallback paths.

### Design Opportunities

1. **Trust-Building Through Transparency** - "Verify Privacy" button showing raw CLI output with PASS/FAIL highlighting lets skeptics see the truth, building genuine trust.

2. **Keyboard-First, Mouse-Friendly** - Command palette as the power core, with visible shortcuts on UI elements so occasional users learn keyboard paths over time.

3. **Context Resurrection** - Attention cards on launch + workflow context from logs make returning to work seamless: "You were working on X, which failed at step Y."

4. **Privacy as Premium Feature** - Green shield "Always Local" feels confident and premium. "Your code never leaves this machine" as a point of pride, not a defensive warning.

## Core User Experience

### Defining Experience

**Core Interaction:** Execute AI-assisted workflows with unwavering privacy confidence

The primary user action is executing workflows - asking the AI to plan, apply patches, or perform development tasks. This core flow must feel fast, trustworthy, and transparent. Everything else (agents, credentials, logs, privacy settings) exists to support this central interaction.

**The Execute Workflow Flow (Operational Realities):**

1. **CLI Binary Resolution**
   - App resolves `aiy` binary: bundled (in app resources) → user-specified path (settings) → PATH lookup
   - Clear error dialog with instructions if binary not found
   - Current binary path visible in Settings screen

2. **Command Queue**
   - Per-repo serialization for workflow/privacy commands (no concurrent ops on same `.aiy/workflow-state.json`)
   - Read-only commands (`agents list`, `credentials status`) can execute concurrently
   - Queue handles command failure gracefully (failed command doesn't block subsequent)

3. **Execution + Capture**
   - CLI spawned via `child_process.spawn` (no shell) for streaming stdout/stderr
   - Real-time stage indicator (Init → Executing → Finalizing) with expandable live output
   - All output captured to JSONL logs (`~/.config/aiy-desktop/logs/commands.jsonl`)
   - Secret-safe masking/scrubbing (local logs only): credentials, API keys, known sensitive patterns masked as `[REDACTED]` before logging
   - **Request text is NEVER stored in logs** (may contain code); only redacted/truncated `request_summary` + optional `request_hash`/ID + timestamps

4. **Cancel Semantics**
   - Cancel button sends SIGTERM to CLI process
   - Process tree cleanup ensures child processes are terminated (`ps-tree` on Unix, `taskkill /PID /T` on Windows)
   - Timeout handling with user-configurable defaults (60s planning, 30s other operations)
   - No orphaned processes on app exit (all spawned PIDs tracked and killed)

### Platform Strategy

**Platform:** Electron Desktop Application
- macOS 12+ (Monterey)
- Windows 10+
- Linux (Ubuntu 20.04+)

**Input Mode:** Keyboard-first with full mouse support

| User Type | Primary Input | UX Accommodations |
|-----------|---------------|-------------------|
| **Jordan (Power)** | Keyboard | `Cmd/Ctrl+K` palette, `Cmd/Ctrl+1-5` screens, `Cmd/Ctrl+Shift+E/R/S` for execute/resume/status |
| **Sam (Occasional)** | Mouse | Visible buttons, labeled icons, discoverable shortcuts shown on hover/in menus |
| **Alex (Security)** | Mixed | Verification buttons prominent, raw CLI output accessible, audit trail via Logs screen |

**Offline-First Design:**
- Always Local mode is 100% functional with zero degraded states
- Ollama health indicator shows local AI availability (green/red/spinner)
- No "you're offline" barriers - local mode IS the default complete experience
- Constrained/airgapped environments are first-class citizens

**Cross-Platform Considerations:**
- Cmd (macOS) vs Ctrl (Windows/Linux) for all shortcuts
- Platform-appropriate window chrome and behaviors
- File paths: `~/.config/aiy-desktop/` (Unix) vs `%APPDATA%/aiy-desktop/` (Windows)
- Process signals: SIGTERM (Unix) vs taskkill (Windows)

### Effortless Interactions

| Interaction | Design Approach | Operational Detail |
|-------------|-----------------|-------------------|
| **Knowing privacy mode** | Persistent badge on ALL screens (header/sidebar) | Green shield = Always Local, Yellow warning = Hybrid; click navigates to Privacy Mode screen |
| **Executing workflow** | `Cmd/Ctrl+Shift+E` or prominent button → multi-line text area → Execute | Enqueues command, shows stage indicator, captures to JSONL |
| **Resuming work** | Attention cards on launch surface Paused/Failed workflows | One-click Resume (Paused) or View Error (Failed) |
| **Canceling** | Cancel button during execution | Sends SIGTERM, cleans process tree, updates UI immediately |
| **Finding anything** | `Cmd/Ctrl+K` command palette | Searches screens, actions, recent workflows; **respects Always Local kill-switch** (cloud actions hidden when ON) |
| **Understanding errors** | Toast notification → click → full error panel | Shows stderr, command that failed, exit code, duration, suggested fix when pattern-matched |
| **Verifying privacy** | "Verify Privacy Configuration" button on Privacy Mode screen | Runs `aiy privacy check`, displays raw CLI output with `[PASS]` highlighted green / `[FAIL]` highlighted red; **parses output markers, ignores exit code** (known CLI behavior: exits 0 even on failures) |

### Critical Success Moments

| Moment | Success Criteria | UX Implementation |
|--------|------------------|-------------------|
| **First launch** | User immediately knows they're in Always Local mode | Green shield badge visible on dashboard; "Your code never leaves this machine" confidence message |
| **First execute** | Workflow runs, user sees progress, gets result | Stage indicator, expandable live output, completion toast |
| **First return (after session gap)** | User knows what they were doing | Attention cards show safe `request_summary` or "Request context unavailable" with one-click resume |
| **First error** | User understands what failed and what to try | Error panel with full stderr + "Suggested fix: [action]" for known patterns |
| **First verification (Alex)** | Security user trusts the app | Raw CLI output visible, `[PASS]/[FAIL]` markers highlighted, copy-to-clipboard for audit |
| **First Hybrid action** | User explicitly approves cloud data transmission | Confirmation modal shows `cloud_payload_preview` from CLI, "Send to Cloud" button, no "remember" option |
| **Hybrid unavailable** | User understands why cloud isn't available | Clear "Hybrid mode unavailable" state with reason: missing credentials / CLI support not present / `cloud_payload_preview` field absent |

**Make-or-Break Flows:**

1. **Privacy Confidence Loop:** See badge → Trust posture → (Optional) Click Verify → See raw [PASS] → Full confidence
2. **Execute → Watch → Complete:** Enter request → Execute → See stages → Get result or actionable error
3. **Resume from Attention:** Launch app → See attention card → One-click resume → Continue where left off
4. **Error → Understand → Fix:** See toast → Open panel → Read stderr + suggestion → Fix and retry

### Experience Principles

1. **Privacy is Visible, Not Hidden**
   - Privacy posture displayed on ALL screens via persistent indicator
   - Green shield = Always Local (confident, premium feel)
   - Yellow warning = Hybrid (explicit choice, not accidental)
   - Kill-switch blocks cloud actions everywhere, including `Cmd/Ctrl+K` palette results
   - No "remember this choice" or "always allow" for cloud actions - each requires explicit approval

2. **Keyboard-First, Mouse-Welcome**
   - Power users complete common flows without touching mouse
   - Shortcuts visible on buttons/menus so occasional users learn over time
   - Command palette (`Cmd/Ctrl+K`) is the power-user hub, but all actions have clickable equivalents

3. **CLI Truth, Human Clarity**
   - CLI is source of truth; UI never redacts/transforms cloud payloads or makes privacy decisions (it may mask/scrub sensitive patterns before persisting local logs)
   - Errors show what the CLI said (full stderr) + human-friendly suggested action
   - Stage indicators map to actual CLI workflow states, not fake progress
   - Privacy check parses `[PASS]/[FAIL]` markers from output (exit code unreliable)

4. **Context Survives Sessions (Privacy-Safe)**
   - Full request text is held **in-memory only** for active session (may contain code - never persisted)
   - JSONL logs store only: redacted/truncated `request_summary`, optional `request_hash`/ID, timestamps, command metadata
   - `.aiy/workflow-state.json` is metadata-only (state enum, timestamps) - does NOT contain request text
   - Attention cards display safe `request_summary` from logs; graceful fallback: "Request context unavailable / redacted"
   - Across sessions: users see summary or acknowledge context was redacted for privacy

5. **Local is Complete, Cloud is Additive**
   - Always Local mode is the full experience, not a "lite" version
   - Hybrid features are additive and may be unavailable (design explicit unavailable states)
   - If CLI cloud-planning wiring isn't complete: show "Feature requires CLI update" rather than broken UI
   - If credentials missing for cloud provider: show "Configure [provider] credentials to enable" state

## Desired Emotional Response

### Primary Emotional Goals

**Core Emotion: Confident Trust**

Users should feel genuinely confident that their code is private - not because they're asked to trust, but because they can *verify* and *see* the truth. This is especially critical for Alex (security-conscious) but benefits all personas.

| Persona | Primary Emotional Need | How It's Achieved |
|---------|----------------------|-------------------|
| **Alex (Security)** | Verified Trust | Green shield badge + "Verify Privacy" button showing raw `[PASS]` markers - evidence, not faith |
| **Jordan (Power)** | Efficient Flow | Keyboard shortcuts work instantly, command queue handles operations seamlessly, no mouse breaks focus |
| **Sam (Occasional)** | Clarity + Orientation | Attention cards restore context on launch, UI is self-explanatory without documentation |

### Emotional Journey Mapping

| Stage | Target Emotion | Design Approach | Anti-Pattern to Avoid |
|-------|----------------|-----------------|----------------------|
| **First launch** | Reassured + Welcomed | Green shield visible immediately; "Your code never leaves this machine" message | Anxiety ("is this really private?"), Overwhelm ("where do I start?") |
| **During execution** | Focused Flow | Stage indicator shows real progress; expandable live output for transparency | Uncertainty ("is it working?"), Boredom (spinner with no info) |
| **On error** | Empowered + Calm | Error panel shows full stderr + "Suggested fix: [action]" for known patterns | Panic ("what broke?"), Helplessness ("now what?") |
| **On cancellation** | In Control | Cancel is instant; process tree cleaned; UI confirms completion | Distrust ("did it really stop?"), Orphan anxiety |
| **On return (after days/weeks)** | Recognized + Oriented | Attention cards: "Workflow X paused at step Y" with one-click resume | Lost ("what was I doing?"), Frustration (starting over) |
| **On verification (Alex)** | Deep Trust | Raw CLI output with `[PASS]/[FAIL]` highlighted; copy-to-clipboard for audit | Skepticism ("the UI might be lying"), Paranoia |
| **On Hybrid action** | Informed Consent | Confirmation modal shows exactly what `cloud_payload_preview` contains; explicit "Send to Cloud" button | Surprise ("wait, that went to cloud?"), Regret |

### Micro-Emotions

| Target Emotion | Avoid | UX Lever |
|----------------|-------|----------|
| **Confidence** | Confusion | Persistent privacy badge on ALL screens - always know your posture |
| **Trust** | Skepticism | "Verify Privacy" shows raw CLI output - see the proof, don't take our word |
| **Control** | Helplessness | Cancel always works; process tree cleanup; clear completion states |
| **Accomplishment** | Frustration | Success toasts; clear workflow completion indicators |
| **Calm** | Anxiety | Green shield feels premium/protective, not paranoid; Always Local is the confident default |
| **Efficiency** | Friction | Keyboard-first design; <3 clicks for common actions; command palette for power users |
| **Clarity** | Disorientation | Attention cards orient returning users; breadcrumbs for navigation; self-explanatory labels |

### Design Implications

| Emotional Goal | UX Implementation |
|----------------|-------------------|
| **Trust through Transparency** | "Verify Privacy" shows raw `[PASS]/[FAIL]` markers from CLI; confirmation modals display exact `cloud_payload_preview`; no hidden behaviors |
| **Confidence through Consistency** | Privacy indicator on EVERY screen in same position with same semantics; keyboard shortcuts work identically everywhere |
| **Control through Responsiveness** | Cancel is instant and complete (SIGTERM + process tree cleanup); stage indicators reflect actual CLI state, not fake progress |
| **Calm through Defaults** | Always Local ON by default; green shield feels protective and premium, not alarming |
| **Flow through Speed** | `Cmd/Ctrl+K` instant response; shortcuts feel responsive; no loading spinners without informational content |
| **Orientation through Context** | Attention cards on launch surface paused/failed work; safe `request_summary` visible; breadcrumbs show navigation state |
| **Empowerment through Clarity** | Errors show what CLI said + suggested fix; not "Something went wrong" but "Patch failed: conflict in file X. Try: ..." |

### Emotional Design Principles

1. **Verification Over Faith**
   - Don't ask users to trust; give them tools to verify
   - "Verify Privacy" button isn't "we promise" - it's "see for yourself"
   - Raw CLI output with highlighted markers builds genuine trust

2. **Green Means Premium Confidence, Not Warning**
   - Always Local indicator should feel like a quality badge
   - "Protected" and "premium" - not paranoid lock-down
   - "Your code never leaves this machine" as a point of pride

3. **Errors Are Partners, Not Enemies**
   - When something fails, UI becomes a collaborative troubleshooter
   - "Here's what happened, here's what you might try"
   - Full stderr + pattern-matched suggested fixes

4. **Returning Users Are Remembered**
   - App greets returning users with context via attention cards
   - Safe `request_summary` or graceful "Context unavailable / redacted"
   - Users feel recognized, not starting from zero

5. **Control Is Instant and Complete**
   - When users cancel, they feel confident it actually stopped
   - No lingering doubt about orphan processes
   - Clear confirmation that cleanup is complete

## UX Pattern Analysis & Inspiration

### Inspiring Products Analysis

| Product | Category | Key UX Strength for aiy Desktop |
|---------|----------|--------------------------------|
| **GitHub Desktop** | Electron CLI wrapper (Git) | Clean workflow states, process management transparency, attention-first dashboard |
| **VS Code** | Developer IDE | Command palette (`Cmd+K`), keyboard-first with discoverable shortcuts |
| **1Password** | Security/privacy tool | Trust-building UX, verification patterns, "nothing leaves device" confidence |
| **Docker Desktop** | CLI wrapper (Docker) | Health indicators, status dashboards, graceful degradation |
| **Pieces.app** | Local-first AI tool | "Local by default" as premium; explicit cloud consent flows |
| **Raycast / Alfred** | Command runners | Command palette UX excellence - fast, fuzzy search, keyboard-driven, extensible |
| **Lens / Rancher Desktop** | K8s desktop wrappers | CLI wrapper + health/status + logs patterns; cluster state visibility |

**GitHub Desktop** - Workflow state management is exemplary: changes → staged → committed maps cleanly to our Ready → Executing → Completed/Failed/Paused. Their attention-first approach (conflicts visible immediately) informs our attention cards. Per-repo Git locking pattern validates our per-repo command queue.

**VS Code** - The gold standard for command palette UX. `Cmd+P`/`Cmd+K` provides power users instant access while menus with visible shortcuts help occasional users learn. Status bar pattern (always visible, key info) directly informs our persistent privacy indicator.

**1Password** - Trust UX for security-conscious users. Biometric feels secure but fast; audit trails visible; CLI integration with verification. Their "see for yourself" approach to security validation directly informs our "Verify Privacy" button showing raw CLI output.

**Docker Desktop** - Health indicators (daemon running/stopped/error) inform our Ollama health indicator. Graceful handling when daemon unavailable teaches us how to handle "Hybrid unavailable" states without degrading local-only experience.

**Pieces.app** - Local-first positioning as a *feature*, not a limitation. Cloud explicitly opt-in. Zero telemetry by default. This validates our "Always Local as premium confidence" emotional design.

**Raycast / Alfred** - Command palette mastery. Sub-100ms response times, fuzzy matching, keyboard-only workflows, extensible actions. Their "type to do anything" mental model informs our `Cmd/Ctrl+K` palette design - fast, fuzzy, comprehensive.

**Lens / Rancher Desktop** - Desktop wrappers around complex CLI tooling (kubectl). Health/status dashboards, integrated logs, context switching. Their "visibility into what the CLI is doing" pattern validates our stage indicators and expandable live output.

### Transferable UX Patterns

**Navigation Patterns:**
| Pattern | Source | Application in aiy Desktop |
|---------|--------|---------------------------|
| Persistent status indicator | 1Password, Docker | Privacy badge (green shield / yellow warning) on ALL screens |
| Command palette as power hub | VS Code, Raycast, Alfred | `Cmd/Ctrl+K` for screens, actions, recent workflows |
| Numbered screen shortcuts | IDEs, Raycast | `Cmd/Ctrl+1-5` for Workflows/Agents/Credentials/Privacy/Logs |
| Fuzzy search everywhere | Raycast, Alfred | Command palette uses fuzzy matching for discoverability |

**Interaction Patterns:**
| Pattern | Source | Application in aiy Desktop |
|---------|--------|---------------------------|
| Attention-first dashboard | GitHub Desktop | Paused/Failed workflows surfaced immediately on launch |
| Progressive disclosure | VS Code settings | Simple view with "expand" for details (logs, errors) |
| Inline shortcuts | VS Code menus | Shortcuts visible on buttons/menus for learning |
| Copy exact command (redacted) | Lens, terminal emulators | "Copy CLI Command" action from any screen/log entry |

**State/Feedback Patterns:**
| Pattern | Source | Application in aiy Desktop |
|---------|--------|---------------------------|
| Health indicators | Docker, Lens | Ollama status (green/red/spinner); CLI binary status |
| Stage progression | GitHub Desktop | Init → Executing → Finalizing with real CLI state mapping |
| Integrated logs | Lens, Rancher Desktop | Logs screen with filtering; expandable output in workflows |
| Process transparency | GitHub Desktop | Cancel is instant + complete; process tree cleanup |

**Trust/Transparency Patterns:**
| Pattern | Source | Application in aiy Desktop |
|---------|--------|---------------------------|
| Verification over faith | 1Password audit | "Verify Privacy" shows raw CLI `[PASS]/[FAIL]` markers |
| Explicit consent | Pieces.app | Each cloud action requires explicit "Send to Cloud" approval |
| Local-first premium | Pieces.app | Green shield as quality badge, not warning |
| CLI command transparency | Lens, terminal tools | "Copy exact CLI command (redacted)" from any action/log for troubleshooting |

### Anti-Patterns to Avoid

| Anti-Pattern | Why Avoid | Our Alternative |
|--------------|-----------|-----------------|
| **Buried privacy settings** | Users can't verify posture easily | Persistent indicator on ALL screens |
| **"Remember this choice" for cloud** | Creates complacency, breaks trust | No "always allow" - each action explicit |
| **Fake progress indicators** | Destroys trust when users notice | Stage indicators map to actual CLI states |
| **Generic error messages** | Helplessness, frustration | Full stderr + suggested fix + copy CLI command |
| **"You're offline" blocking screens** | Punishes local-first users | Local mode is complete, not degraded |
| **Modal overload** | Interrupt-driven anxiety | Toasts for notifications; modals only for consent |
| **Hidden keyboard shortcuts** | Power users never discover them | Shortcuts visible on buttons/menus |
| **Orphaned processes on cancel** | User distrust ("did it stop?") | Process tree cleanup + confirmation |
| **Opaque CLI invocations** | Can't troubleshoot or verify | "Copy CLI Command" shows exact (redacted) invocation |

### Design Inspiration Strategy

**What to Adopt Directly:**
- **Command palette pattern** (VS Code, Raycast, Alfred) - `Cmd/Ctrl+K` as power user hub with fuzzy search
- **Persistent status indicator** (1Password, Docker) - Privacy badge everywhere
- **Attention cards on dashboard** (GitHub Desktop) - Surface issues immediately
- **Health indicators** (Docker, Lens) - Ollama status, binary resolution status
- **Verification button** (1Password) - "Verify Privacy" shows raw CLI output
- **Copy CLI command** (Lens, terminals) - "Copy exact CLI command (redacted)" for transparency
- **Integrated logs** (Lens, Rancher Desktop) - Logs screen with filtering + expandable output

**What to Adapt:**
- **VS Code settings organization** - Simplify for 5 screens instead of hundreds
- **GitHub Desktop workflow states** - Adapt for AI workflow lifecycle (not Git)
- **1Password trust messaging** - Adapt "nothing leaves device" for code context
- **Docker graceful degradation** - Adapt for "Hybrid unavailable" states
- **Lens CLI transparency** - Adapt "copy kubectl command" to "copy aiy command (redacted)"

**What to Avoid:**
- **Slack notification overload** - Keep toasts minimal and actionable
- **Electron bloat** - Stay thin wrapper; CLI does heavy lifting
- **"Premium = cloud" positioning** - Local IS premium here
- **Complex onboarding wizards** - First launch should just work (Always Local default)
- **Hidden CLI commands** - Every action should have "Copy CLI Command" option

## Design System Foundation

### Design System Choice

**Selected:** shadcn/ui + Tailwind CSS (Radix UI primitives)

This combination provides keyboard-first, accessible, lightweight components with full customization control for privacy/status indicators. Components are copied into the codebase (ownership model), not imported as dependencies.

### Rationale for Selection

| Factor | How shadcn/ui + Tailwind Addresses It |
|--------|--------------------------------------|
| **Keyboard-first** | Radix primitives have excellent keyboard navigation and focus management built-in |
| **Lightweight** | No heavy runtime; just Tailwind utility classes compiled at build time |
| **Customization** | Full control over privacy badges, stage indicators, attention cards |
| **Command Palette** | `cmdk` library integrates perfectly (same Radix/Vercel ecosystem) |
| **Developer Aesthetic** | Clean, functional look standard in modern dev tools |
| **Dark Mode** | Tailwind's `dark:` prefix with CSS variables for seamless theming |
| **Offline-First** | No CDN dependencies; everything bundled locally |

### Implementation Approach

**Component Strategy:**

| Component Type | Implementation |
|----------------|----------------|
| **Core UI** | shadcn/ui primitives: Button, Input, Select, Dialog, Toast, Table |
| **Command Palette** | `cmdk` library with custom styling |
| **Privacy Badge** | Custom: green shield / yellow warning with icon + label |
| **Stage Indicator** | Custom: stepped progress reflecting CLI workflow states |
| **Attention Cards** | Custom: alert-style cards with action buttons |
| **Health Indicator** | Custom: icon + label (green/red/spinner) for Ollama status |

**Offline-First Asset Constraints:**

| Asset Type | Approach |
|------------|----------|
| **Fonts** | System font stack only (`-apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif`); no remote font loading |
| **Icons** | Bundle all icons locally (Lucide React or similar); no icon CDN |
| **Images** | All assets embedded in app bundle; no runtime fetches |
| **Styles** | Tailwind compiled at build time; no remote stylesheets |

### Accessibility Requirements

**Core Accessibility Standards:**

| Requirement | Implementation |
|-------------|----------------|
| **Focus Rings** | Strong, visible focus rings on all interactive elements; never `outline: none` without replacement |
| **Keyboard Navigation** | Full keyboard nav for all screens; tab order logical; skip links if needed |
| **Color Independence** | Never rely on color alone: privacy badge = icon + label ("Local" / "Hybrid"); health = icon + label ("Connected" / "Error") |
| **Motion Sensitivity** | Respect `prefers-reduced-motion`: disable animations, transitions, and spinners when enabled |
| **Contrast** | WCAG AA+ minimum (4.5:1 for text, 3:1 for UI components) in both light and dark modes |
| **Screen Readers** | Semantic HTML; ARIA labels where needed; live regions for status changes |

**Accessibility Component Examples:**

```
Privacy Badge:
- Icon: Shield (filled green or outlined yellow)
- Label: "Always Local" or "Hybrid Mode"
- aria-label: "Privacy mode: Always Local" / "Privacy mode: Hybrid"

Health Indicator:
- Icon: Circle (green) / AlertCircle (red) / Loader (spinning)
- Label: "Connected" / "Error" / "Checking..."
- aria-live="polite" for status changes
```

### Design Tokens (CSS Variables)

**Semantic Token Structure:**

Tokens defined via CSS custom properties for light/dark mode theming:

```css
:root {
  /* Privacy Status */
  --color-privacy-local: theme('colors.green.600');
  --color-privacy-local-bg: theme('colors.green.50');
  --color-privacy-hybrid: theme('colors.amber.500');
  --color-privacy-hybrid-bg: theme('colors.amber.50');

  /* Health Status */
  --color-health-ok: theme('colors.green.500');
  --color-health-error: theme('colors.red.500');
  --color-health-checking: theme('colors.blue.500');

  /* Workflow Status */
  --color-status-running: theme('colors.blue.500');
  --color-status-paused: theme('colors.amber.500');
  --color-status-completed: theme('colors.green.500');
  --color-status-failed: theme('colors.red.500');

  /* Surfaces */
  --color-surface: theme('colors.white');
  --color-surface-muted: theme('colors.slate.50');
  --color-border: theme('colors.slate.200');
  --color-text: theme('colors.slate.900');
  --color-text-muted: theme('colors.slate.500');

  /* Focus */
  --color-focus-ring: theme('colors.blue.500');
}

.dark {
  /* Privacy Status (dark mode) */
  --color-privacy-local: theme('colors.green.400');
  --color-privacy-local-bg: theme('colors.green.950');
  --color-privacy-hybrid: theme('colors.amber.400');
  --color-privacy-hybrid-bg: theme('colors.amber.950');

  /* Surfaces (dark mode) */
  --color-surface: theme('colors.slate.900');
  --color-surface-muted: theme('colors.slate.800');
  --color-border: theme('colors.slate.700');
  --color-text: theme('colors.slate.50');
  --color-text-muted: theme('colors.slate.400');
}
```

### Customization Strategy

**Custom Component Development:**

1. **Privacy Badge** - Shield icon + text label; green (local) / yellow (hybrid); clickable → navigates to Privacy Mode screen
2. **Stage Indicator** - Horizontal stepper: Init → Executing → Finalizing; highlights current stage; expandable for live output
3. **Attention Cards** - Alert card with icon, title, description, primary action button; surfaces paused/failed workflows
4. **Health Indicator** - Small badge with icon + label for Ollama status; tooltip with details
5. **Copy CLI Command Button** - Button with copy icon; shows redacted command; copies to clipboard with confirmation

**Tailwind Extensions:**

```js
// tailwind.config.js
module.exports = {
  theme: {
    extend: {
      colors: {
        privacy: {
          local: 'var(--color-privacy-local)',
          'local-bg': 'var(--color-privacy-local-bg)',
          hybrid: 'var(--color-privacy-hybrid)',
          'hybrid-bg': 'var(--color-privacy-hybrid-bg)',
        },
        health: {
          ok: 'var(--color-health-ok)',
          error: 'var(--color-health-error)',
          checking: 'var(--color-health-checking)',
        },
      },
    },
  },
}
```

## Defining Core Experience

### The Defining Experience

**For aiy Desktop, the defining experience is:**

> *"Ask the AI to help with your code, with verifiable privacy confidence."*

Or expressed as the core interaction:

> *"Execute → Watch → Trust (with evidence)"*

**The "elevator pitch" users would tell friends:**
- "It's a GUI for aiy that lets you run AI workflows on your code, but you can *verify* it's all local - there's this green shield showing your privacy posture, and you can click 'Verify' to see the raw proof."

### User Mental Model

**How users currently solve this problem:**

| User Type | Current Approach | Pain Points |
|-----------|------------------|-------------|
| **Alex (Security)** | Avoids cloud AI tools; uses local models only; reads source code before trusting | "I can't *verify* what these tools are doing with my code" |
| **Jordan (Power)** | Uses multiple AI tools; context-switches; worries about NDAs | "Too many clicks; I want keyboard shortcuts; I don't trust 'we promise' privacy policies" |
| **Sam (Occasional)** | Tries tools once, forgets context, struggles to resume | "I don't remember where I left off; errors are confusing" |

**Mental model users bring:**
- *"AI tools send my code to the cloud unless I explicitly stop them"*
- *"Privacy settings are buried and I can't verify they work"*
- *"Cloud = faster/better; Local = slower/worse"* (we flip this)

**Expectation for how it should work:**
1. I see immediately what my privacy posture is (Always Local vs Hybrid)
2. I select my repo/working directory
3. I type what I want the AI to do
4. I watch it work
5. I get my result, confident because I can *verify* nothing leaked

### Success Criteria

| Criteria | Measurement |
|----------|-------------|
| **Privacy Confidence** | User can answer "what's my privacy posture?" by looking at screen (green shield = Always Local) |
| **Repo Awareness** | User always knows which repo/workdir this window is operating on; changing repos is explicit (v1: open a new window) |
| **Execution Speed** | Workflow starts within 500ms of Execute; stage indicator visible immediately |
| **Progress Transparency** | User always knows what stage they're in; can expand to see live output |
| **Error Clarity** | On failure, user can read error + see suggested fix within 5 seconds |
| **Resume Ease** | Returning user sees attention card + resumes paused workflow in 1 click |
| **Verification** | Alex can click "Verify Privacy" and see raw CLI `[PASS]` markers as evidence |

**"This just works" moments:**
- Green shield visible on first launch - no setup required
- Active repo/workdir shown in header at all times (v1: single repo per window)
- `Cmd/Ctrl+Shift+E` → type → Execute → done
- Error happens → toast → click → see full stderr + suggestion → fix → retry
- Return after a week → attention card says "Workflow X paused" → click → resume

### Novel UX Patterns

**Established Patterns (familiar, no learning curve):**

| Pattern | Source | How We Use It |
|---------|--------|---------------|
| Command palette | VS Code, Raycast | `Cmd/Ctrl+K` for all actions |
| Stage/progress indicator | GitHub Desktop, wizards | Init → Executing → Finalizing |
| Toast notifications | Every modern app | Success/error notifications |
| Attention cards | GitHub Desktop, dashboards | Surface paused/failed workflows |
| Tables with filtering | Logs tools, IDEs | Logs screen |
| Repo/workdir context indicator | IDEs | Active repo/workdir shown in header (v1: single repo per window) |

**Novel/Adapted Patterns (unique to aiy Desktop):**

| Pattern | What's Novel | Why It Matters |
|---------|--------------|----------------|
| **Privacy badge as premium feature** | Most apps hide privacy in settings; we make it the hero, showing current posture | Trust is the product's core value |
| **"Verify Privacy" with raw CLI output** | Usually apps say "trust us"; we say "here's the evidence" | Builds verifiable confidence for skeptics (Alex) |
| **Hybrid confirmation with payload preview** | Most cloud confirmations are vague; we show exact data | Informed consent, no surprises |
| **Copy CLI Command (redacted)** | Transparency into what the GUI is doing | Builds trust; aids troubleshooting |
| **Local-first as confident premium** | Usually "offline mode" feels degraded | We invert: green shield = quality |
| **Per-repo context** | Repo selector always visible; state isolated per `.aiy/` directory | Prevents cross-repo confusion |

### Experience Mechanics

**Core Flow: Execute Workflow**

```
┌─────────────────────────────────────────────────────────────────┐
│ 1. INITIATION                                                   │
├─────────────────────────────────────────────────────────────────┤
│ CONTEXT VISIBLE:                                                │
│   • Active Repo / Workdir: Always visible in header             │
│     - Displays repo name + truncated path                       │
│     - v1: single repo per window (no in-window switching)       │
│     - "Open Repo…" opens a new window                           │
│   • Privacy Badge: Green shield "Always Local" in header        │
│                                                                 │
│ TRIGGER: `Cmd/Ctrl+Shift+E` OR click "New Workflow" button      │
│                                                                 │
│ UI: Multi-line text area opens for request entry                │
│ Hint: Placeholder: "What would you like the AI to do?"          │
│ Note: Command queued to active repo's queue                     │
└─────────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────────┐
│ 2. INTERACTION                                                  │
├─────────────────────────────────────────────────────────────────┤
│ User types: "Refactor the auth module to use JWT tokens"        │
│ UI shows: Request text area + Execute button + Cancel link      │
│ Active repo: Still visible in header (confirms context)         │
│ Privacy badge: Still visible, green, "Always Local"             │
│ Keyboard: Cmd/Ctrl+Enter to Execute, Escape to Cancel           │
└─────────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────────┐
│ 3. FEEDBACK (During Execution)                                  │
├─────────────────────────────────────────────────────────────────┤
│ Stage indicator: [Init] → [•Executing] → [Finalizing]           │
│ Live output: Expandable panel shows streaming stdout/stderr     │
│ Privacy badge: Still green, unchanged - reinforces posture      │
│ Active repo: Still visible - confirms which repo is running     │
│ Cancel: Button visible, instant response (SIGTERM + cleanup)    │
│ Duration: Timer shows elapsed time                              │
│ Queue: Other ops for this repo wait; read-only ops proceed      │
└─────────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────────┐
│ 4. COMPLETION                                                   │
├─────────────────────────────────────────────────────────────────┤
│ Success: Toast "Workflow completed" + stage shows [Completed ✓] │
│          Full output visible in expandable panel                │
│          "Copy CLI Command" button available                    │
│                                                                 │
│ Failure: Toast "Workflow failed - click for details"            │
│          Error panel: Full stderr + exit code + duration        │
│          Suggested fix: Pattern-matched recommendation          │
│          "Copy CLI Command" + "Retry" buttons                   │
│                                                                 │
│ Paused:  Toast "Workflow paused"                                │
│          Stage shows [Paused ⏸]                                 │
│          "Resume" button prominent                              │
└─────────────────────────────────────────────────────────────────┘
```

**Active Repo / Workdir Details (v1):**

| Element | Behavior |
|---------|----------|
| **Location** | Left side of header bar, always visible |
| **Display** | Repo name (bold) + truncated path (muted) |
| **Click** | Optional: opens "Open Repo…" (new window) |
| **Keyboard** | Optional: `Cmd/Ctrl+O` opens "Open Repo…" (new window) |
| **State Isolation** | Each repo has its own `.aiy/` directory, command queue, workflow state |
| **Switching** | v1: no in-window switching; open another repo in a new window (future enhancement: in-window switching) |
| **No Repo** | On launch, if no repo configured: prompt to open a repo before workflows can run |

**Core Flow: Privacy Verification (Alex)**

```
┌─────────────────────────────────────────────────────────────────┐
│ 1. INITIATION                                                   │
├─────────────────────────────────────────────────────────────────┤
│ Trigger: Click green shield badge OR navigate to Privacy screen │
│ Context: User wants evidence of privacy posture, not promises   │
└─────────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────────┐
│ 2. INTERACTION                                                  │
├─────────────────────────────────────────────────────────────────┤
│ Privacy Mode screen shows current posture (Always Local/Hybrid) │
│ User clicks "Verify Privacy Configuration" button               │
└─────────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────────┐
│ 3. FEEDBACK                                                     │
├─────────────────────────────────────────────────────────────────┤
│ Spinner while CLI runs `aiy privacy check`                      │
│ Raw CLI output displayed in code block:                         │
│   [PASS] No cloud provider configured ← highlighted green       │
│   [PASS] Ollama endpoint is local     ← highlighted green       │
│   [PASS] No telemetry enabled         ← highlighted green       │
│ Parse markers for highlighting (ignore exit code - known quirk) │
└─────────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────────┐
│ 4. COMPLETION                                                   │
├─────────────────────────────────────────────────────────────────┤
│ Summary: "All checks passed - your code stays local"            │
│ Copy button: Copy raw output to clipboard for audit             │
│ User feels: Verifiable confidence - saw the evidence            │
└─────────────────────────────────────────────────────────────────┘
```

**Core Flow: Resume from Attention Card (Sam)**

```
┌─────────────────────────────────────────────────────────────────┐
│ 1. INITIATION                                                   │
├─────────────────────────────────────────────────────────────────┤
│ Trigger: App launch after days away                             │
│ Context: Sam forgot what they were doing                        │
│ Active repo: This window is scoped to a single repo/workdir      │
└─────────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────────┐
│ 2. INTERACTION                                                  │
├─────────────────────────────────────────────────────────────────┤
│ Dashboard shows Attention Card:                                 │
│   ⏸ "Workflow paused: Refactor auth module..." (repo-name)     │
│   [Resume] [View Details] [Dismiss]                             │
│ Card shows which repo the workflow belongs to                   │
│ Sam clicks [Resume]                                             │
└─────────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────────┐
│ 3. FEEDBACK                                                     │
├─────────────────────────────────────────────────────────────────┤
│ Workflow screen opens with context restored                     │
│ Stage indicator shows current position                          │
│ CLI command enqueued to resume                                  │
└─────────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────────┐
│ 4. COMPLETION                                                   │
├─────────────────────────────────────────────────────────────────┤
│ Workflow continues from where it left off                       │
│ Sam feels: Recognized, oriented, not starting over              │
└─────────────────────────────────────────────────────────────────┘
```

## Visual Design Foundation

### Color System

**Brand posture:** No formal brand guide, but a clear direction:
- Developer-focused aesthetic: clean, functional, not consumer-flashy
- Privacy as premium: green shield = confidence, not warning
- Inspiration: VS Code, Raycast, GitHub Desktop, 1Password

**Core palette (Tailwind-based):**

| Category | Light Mode | Dark Mode | Usage |
|----------|------------|-----------|-------|
| Surface | slate-50 / white | slate-900 / slate-800 | Backgrounds |
| Text | slate-900 / slate-500 | slate-50 / slate-400 | Primary / muted |
| Border | slate-200 | slate-700 | Dividers, outlines |
| Accent | blue-600 | blue-400 | Primary actions, focus |
| Destructive | red-600 | red-400 | Delete, cancel, errors |

**Semantic status colors (always icon + label, never color-only):**

| Status | Light | Dark | Icon + Label |
|--------|-------|------|--------------|
| Privacy: Local | green-600 on green-50 | green-400 on green-950 | Shield + "Always Local" |
| Privacy: Hybrid | amber-500 on amber-50 | amber-400 on amber-950 | ShieldAlert + "Hybrid Mode" |
| Health: OK | green-500 | green-400 | CheckCircle + "Connected" |
| Health: Error | red-500 | red-400 | AlertCircle + "Error" |
| Health: Checking | blue-500 | blue-400 | Loader + "Checking..." |
| Workflow: Running | blue-500 | blue-400 | Loader + stage name |
| Workflow: Paused | amber-500 | amber-400 | Pause + "Paused" |
| Workflow: Completed | green-500 | green-400 | CheckCircle + "Completed" |
| Workflow: Failed | red-500 | red-400 | XCircle + "Failed" |

**Token implementation:** Use semantic CSS variables (see “Design Tokens (CSS Variables)”) and map Tailwind theme colors to those variables for light/dark modes.

### Typography System

**Offline-first font stacks:**

- `--font-sans`: `-apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif;`
- `--font-mono`: `ui-monospace, SFMono-Regular, 'SF Mono', Menlo, Consolas, 'Liberation Mono', monospace;`

**Type scale (16px root):**

| Token | Size | Weight | Line Height | Usage |
|------|------|--------|-------------|-------|
| text-xs | 0.75rem (12px) | 400 | 1rem | Badges, captions, timestamps |
| text-sm | 0.875rem (14px) | 400 | 1.25rem | Secondary text, table cells |
| text-base | 1rem (16px) | 400 | 1.5rem | Body text, form inputs |
| text-lg | 1.125rem (18px) | 500 | 1.75rem | Card titles, section headers |
| text-xl | 1.25rem (20px) | 600 | 1.75rem | Page titles |
| text-2xl | 1.5rem (24px) | 600 | 2rem | Screen titles |
| text-mono | 0.875rem (14px) | 400 | 1.5rem | CLI output, code blocks, paths |

### Spacing & Layout Foundation

**Base unit:** 4px (Tailwind default)

| Token | Value | Usage |
|------|-------|-------|
| space-1 | 4px | Tight gaps (icon to label) |
| space-2 | 8px | Small gaps (between related items) |
| space-3 | 12px | Component internal padding |
| space-4 | 16px | Standard padding, gaps between cards |
| space-6 | 24px | Section spacing |
| space-8 | 32px | Major section breaks |
| space-12 | 48px | Page-level margins |

**Layout structure (desktop):**

```
┌────────────────────────────────────────────────────────────┐
│ HEADER (h-12, 48px)                                        │
│ [Repo / Workdir] [Privacy Badge] [Ollama Health] [Settings] │
├────────────────────────────────────────────────────────────┤
│ SIDEBAR (w-48, 192px)  │  MAIN CONTENT AREA               │
│ [Workflows]            │  (fluid, max-width ~1200px)       │
│ [Agents]               │                                   │
│ [Credentials]          │                                   │
│ [Privacy Mode]         │                                   │
│ [Logs]                 │                                   │
└────────────────────────────────────────────────────────────┘
```

**v1 repo scope:** single repo/workdir per window. “Open Repo…” opens a new window (no in-window multi-repo management).

### Accessibility Considerations

- Focus rings are always visible; never remove outlines without a clear replacement.
- Contrast targets: WCAG AA+ (4.5:1 text, 3:1 UI components) in light and dark themes.
- Do not rely on color alone: every status uses icon + label (Local/Hybrid/Connected/Error/etc.).
- Respect `prefers-reduced-motion`: disable non-essential animations; provide non-animated alternatives to spinners.
- Avoid runtime remote assets (fonts/icons/CDNs); bundle locally for offline-first and deterministic rendering.

### Component Styling Patterns (Tailwind + shadcn/ui)

**Buttons:**

| Variant | Style | Usage |
|---------|-------|-------|
| Primary | `bg-blue-600 text-white hover:bg-blue-700` | Main actions (Execute, Save) |
| Secondary | `bg-slate-100 text-slate-900 hover:bg-slate-200` | Secondary actions (Cancel, View) |
| Ghost | `bg-transparent text-slate-600 hover:bg-slate-100` | Tertiary actions, icon buttons |
| Destructive | `bg-red-600 text-white hover:bg-red-700` | Delete, Stop |

**Cards (conceptual):**

```css
.card {
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: 8px;
  padding: 16px;
}

.card-attention {
  border-left: 4px solid var(--color-status-paused); /* or failed */
}
```

**Inputs (conceptual):**

```css
.input {
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: 6px;
  padding: 8px 12px;
  font-size: 14px;
}

.input:focus {
  outline: none;
  border-color: var(--color-focus-ring);
  box-shadow: 0 0 0 2px rgba(59, 130, 246, 0.2);
}
```

### Dark Mode Strategy

Approach: system preference with manual override.

```css
@media (prefers-color-scheme: dark) {
  :root { /* dark tokens */ }
}

.dark { /* dark tokens */ }
```

Toggle location: Settings screen (and optional quick toggle in header as P2).

## Design Directions (Step 9)

**Artifact:** `_bmad-output/planning-artifacts/ux-design-directions.html`

Step 9 produced a standalone HTML mockup exploring 6 design direction concepts, each demonstrating the same 5 core screens (Workflows, Agents, Credentials, Privacy Mode, Logs) with different visual and interaction approaches.

### Direction Summary

| Direction | Focus | Best For | Trust | Speed |
|-----------|-------|----------|-------|-------|
| **A: Privacy-First** | Maximum trust through constant visual reassurance | Alex (Security) | ★★★★★ | ★★★ |
| **B: Workflow-Centric** | Fast execution with workflow cards as heroes | Jordan (Power) | ★★★ | ★★★★★ |
| **C: Command Palette Native** | Keyboard-first with omnipresent Cmd+K | Jordan (Power) | ★★★ | ★★★★★ |
| **D: VS Code Familiar** | Developer-native patterns, dark mode aesthetic | Jordan (Power) | ★★★ | ★★★★ |
| **E: Dashboard-First** | At-a-glance status with metrics overview | Sam (Occasional) | ★★★★ | ★★★ |
| **F: Log-Transparent** | Real-time streaming logs as primary UI | Alex (Security) | ★★★★★ | ★★★ |

### Recommended Combined Approach

Based on "Trust + Speed" goal and the 3 personas, the recommended approach combines:

- **Direction B layout** — workflow cards as heroes for fast scanning
- **Direction C keyboard-first** — Cmd+K command palette for power users
- **Direction A privacy** — persistent privacy badge + inline verification
- **Direction F transparency** — real-time log view during execution

### Coverage Verified

The HTML mockup explicitly represents:

- **Workflow states:** Running, Paused, Failed, Idle
- **Privacy verification:** [PASS]/[FAIL] markers in log timeline
- **Logs filter/search:** Search input + level filter dropdown
- **Copy CLI Command (redacted):** Explicit affordance for debugging
- **Accessibility:** Focus-visible styling, prefers-reduced-motion support
- **Offline-first:** No external fonts/icons/CDNs (system font stack only)

## User Journey Flows (Step 10)

This section maps the step-by-step paths users take through aiy Desktop for key tasks, optimized for the three personas (Alex, Jordan, Sam).

---

### Journey 1: First Launch & Onboarding

**Trigger:** User opens aiy Desktop for the first time (no prior config).

```
┌─────────────────────────────────────────────────────────────────┐
│ STEP 1: App Launch                                              │
│ ─────────────────────────────────────────────────────────────── │
│ • App attempts to resolve `aiy` CLI binary                      │
│ • Checks: bundled → user path (settings) → PATH lookup          │
│                                                                 │
│ [CLI Found?]                                                    │
│    YES → Continue to Step 2                                     │
│    NO  → Show "CLI Not Found" dialog with:                      │
│          - "Download aiy CLI" link (opens browser)              │
│          - "Browse..." to manually locate binary                │
│          - "Check PATH" to retry PATH resolution                │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│ STEP 2: Repository Selection                                    │
│ ─────────────────────────────────────────────────────────────── │
│ • Show "Open Repository" dialog (native folder picker)          │
│ • User selects a directory containing a project                 │
│                                                                 │
│ [Valid Repo?]                                                   │
│    YES → Continue to Step 3                                     │
│    NO  → Show inline warning: "No .aiy/ found. Initialize?"     │
│          - "Initialize aiy" button (runs `aiy privacy init`)    │
│          - "Open Anyway" (limited functionality warning)        │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│ STEP 3: Privacy Posture Check                                   │
│ ─────────────────────────────────────────────────────────────── │
│ • Auto-run `aiy privacy check` (parse [PASS]/[FAIL] markers)    │
│ • Display result in header privacy badge                        │
│                                                                 │
│ Badge shows current mode (persisted setting):                   │
│   🛡️ "Always Local" (green) — default on first launch           │
│   ⚠️ "Hybrid Mode" (amber) — only when user explicitly enables  │
│                                                                 │
│ [Ollama Check]                                                  │
│    Connected → Show "● Ollama Running" (green)                  │
│    Not Found → Show "○ Ollama Not Found" (muted) + tooltip      │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│ STEP 4: Main Interface                                          │
│ ─────────────────────────────────────────────────────────────── │
│ • Show Workflows screen (default landing)                       │
│ • Display available workflows from CLI                          │
│ • Privacy badge visible in header (persistent)                  │
│                                                                 │
│ First-time hint (dismissible):                                  │
│   "Press ⌘K to open command palette for quick actions"          │
│                                                                 │
│ Sam persona: sees "Getting Started" card if no recent activity  │
│ Jordan persona: immediately uses ⌘K                             │
│ Alex persona: clicks privacy badge to verify                    │
└─────────────────────────────────────────────────────────────────┘
```

**Success criteria:** User lands on Workflows screen with privacy posture visible and understood.

---

### Journey 2: Execute Workflow (Core Flow)

**Trigger:** User wants to run an AI-assisted workflow (e.g., "create-feature").

**Persona paths:**
- **Jordan (Power):** ⌘K → type "create" → Enter → provide input → Execute
- **Sam (Occasional):** Click workflow card → Read description → Click "Execute" → provide input
- **Alex (Security):** Same as Sam, but clicks privacy badge first to verify

```
┌─────────────────────────────────────────────────────────────────┐
│ STEP 1: Select Workflow                                         │
│ ─────────────────────────────────────────────────────────────── │
│ Via Command Palette (⌘K):                                       │
│   • Type workflow name → fuzzy match → ↑↓ to select → Enter     │
│                                                                 │
│ Via Workflow Card Click:                                        │
│   • Click card → Opens workflow detail/execution panel          │
│                                                                 │
│ Keyboard shortcut (if configured):                              │
│   • ⌘⇧E on selected workflow card                               │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│ STEP 2: Provide Input (if required)                             │
│ ─────────────────────────────────────────────────────────────── │
│ • Show input form based on workflow requirements                │
│ • Text area for request/prompt                                  │
│ • Optional: file picker for context files                       │
│                                                                 │
│ Privacy reminder (inline, non-blocking):                        │
│   "This request will be processed locally by Ollama"            │
│   or "This request requires cloud API (will prompt for consent)"│
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│ STEP 3: Pre-Execution Privacy Check                             │
│ ─────────────────────────────────────────────────────────────── │
│ [Privacy Mode = Always Local?]                                  │
│    YES → Continue to Step 4 (no prompt)                         │
│    NO  → Show Cloud Consent Modal (Journey 5)                   │
│          - Must approve per-action (no "always allow")          │
│          - Cancel returns to input step                         │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│ STEP 4: Execution                                               │
│ ─────────────────────────────────────────────────────────────── │
│ • CLI spawned via child_process.spawn (no shell)                │
│ • Stage indicator: Init → Executing → Finalizing                │
│ • Real-time log output (expandable panel)                       │
│ • Cancel button visible: "Cancel (⌘.)"                          │
│                                                                 │
│ UI state:                                                       │
│   • Workflow card shows "Running" badge (blue)                  │
│   • Header shows active workflow indicator                      │
│   • Other workflows remain accessible (queued if same repo)     │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│ STEP 5: Completion                                              │
│ ─────────────────────────────────────────────────────────────── │
│ [Exit Code = 0?]                                                │
│    SUCCESS:                                                     │
│      • Badge changes to "Completed" (green)                     │
│      • Toast notification: "create-feature completed"           │
│      • Log shows final output with syntax highlighting          │
│      • "Copy CLI Command (redacted)" available for debugging    │
│                                                                 │
│    FAILURE:                                                     │
│      • Badge changes to "Failed" (red)                          │
│      • Error panel with:                                        │
│        - Raw CLI error output                                   │
│        - Suggested fixes (if parseable)                         │
│        - "Retry" button                                         │
│        - "Copy Log" for support                                 │
└─────────────────────────────────────────────────────────────────┘
```

**Success criteria:** Workflow executes with full transparency; user knows exactly what happened.

---

### Journey 3: Privacy Verification (Alex Persona)

**Trigger:** Security-conscious user wants to verify their privacy posture before trusting the app.

```
┌─────────────────────────────────────────────────────────────────┐
│ STEP 1: Click Privacy Badge                                     │
│ ─────────────────────────────────────────────────────────────── │
│ • User clicks the privacy badge in header (🛡️ Always Local)     │
│ • Opens Privacy Mode screen                                     │
│                                                                 │
│ Alternative: Sidebar → Privacy Mode                             │
│ Keyboard: ⌘4 (if shortcuts configured)                          │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│ STEP 2: Privacy Mode Screen                                     │
│ ─────────────────────────────────────────────────────────────── │
│ Shows:                                                          │
│   • Current posture: "Always Local" or "Hybrid Mode"            │
│   • Ollama connection status with endpoint                      │
│   • Last privacy check timestamp                                │
│   • "Verify Privacy" button (prominent)                         │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│ STEP 3: Click "Verify Privacy"                                  │
│ ─────────────────────────────────────────────────────────────── │
│ • Runs `aiy privacy check`                                      │
│ • Shows raw CLI output in expandable panel                      │
│ • Parses [PASS]/[FAIL] markers (exit code unreliable)           │
│                                                                 │
│ Output display:                                                 │
│   [PASS] highlighted in green                                   │
│   [FAIL] highlighted in red                                     │
│   Full CLI text visible for manual inspection                   │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│ STEP 4: Verification Result                                     │
│ ─────────────────────────────────────────────────────────────── │
│ [All Checks PASS?]                                              │
│    YES:                                                         │
│      • Badge remains in current mode (unchanged)                │
│      • Confidence message: "All privacy checks passed"          │
│      • Timestamp updated                                        │
│                                                                 │
│    NO (some FAIL):                                              │
│      • Badge remains in current mode (unchanged)                │
│      • Show "Verification failed" warning overlay on badge      │
│      • Warning panel with specific failures listed              │
│      • "How to fix" links for each failure                      │
│      • Option to continue with warnings acknowledged            │
└─────────────────────────────────────────────────────────────────┘
```

**Success criteria:** Alex can verify, not just believe, that their data stays local.

---

### Journey 4: Session Recovery (Sam Persona)

**Trigger:** User returns to aiy Desktop after hours/days away and needs context.

```
┌─────────────────────────────────────────────────────────────────┐
│ STEP 1: App Launch (Returning User)                             │
│ ─────────────────────────────────────────────────────────────── │
│ • App detects previous session state                            │
│ • Loads last-used repository automatically                      │
│ • Checks for interrupted/failed workflows                       │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│ STEP 2: Attention Cards (if applicable)                         │
│ ─────────────────────────────────────────────────────────────── │
│ Display priority cards at top of Workflows screen:              │
│                                                                 │
│ ┌─ ATTENTION CARD (amber border) ────────────────────────────┐  │
│ │ ⚠️ Workflow paused: "fix-bug"                              │  │
│ │ Paused 2 hours ago awaiting confirmation                   │  │
│ │ [Resume] [View Details] [Dismiss]                          │  │
│ └────────────────────────────────────────────────────────────┘  │
│                                                                 │
│ ┌─ ATTENTION CARD (red border) ──────────────────────────────┐  │
│ │ ❌ Workflow failed: "refactor-component"                   │  │
│ │ Failed yesterday at step "Apply patches"                   │  │
│ │ [View Error] [Retry] [Dismiss]                             │  │
│ └────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│ STEP 3: Context Resurrection                                    │
│ ─────────────────────────────────────────────────────────────── │
│ Recent Activity section shows:                                  │
│   • Last 5 workflow executions with timestamps                  │
│   • Success/failure status for each                             │
│   • Quick actions: "Run again", "View logs"                     │
│                                                                 │
│ Sam thinks: "Oh right, I was working on that bug fix"           │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│ STEP 4: Resume or Start Fresh                                   │
│ ─────────────────────────────────────────────────────────────── │
│ [Paused Workflow?]                                              │
│    • Click "Resume" → Workflow continues from checkpoint        │
│                                                                 │
│ [Failed Workflow?]                                              │
│    • Click "View Error" → See what went wrong                   │
│    • Click "Retry" → Re-run with same parameters                │
│                                                                 │
│ [Start Fresh?]                                                  │
│    • Click any workflow card or use ⌘K                          │
└─────────────────────────────────────────────────────────────────┘
```

**Success criteria:** Sam knows immediately what they were doing and can continue seamlessly.

---

### Journey 5: Cloud Consent (Hybrid Action)

**Trigger:** User attempts an action that requires cloud API when in Hybrid mode.

```
┌─────────────────────────────────────────────────────────────────┐
│ STEP 1: Action Triggers Cloud Requirement                       │
│ ─────────────────────────────────────────────────────────────── │
│ • User executes workflow that needs cloud API                   │
│ • System detects Hybrid mode / cloud-required operation         │
│ • Execution pauses before any data leaves machine               │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│ STEP 2: Cloud Consent Modal                                     │
│ ─────────────────────────────────────────────────────────────── │
│ ┌─ MODAL ────────────────────────────────────────────────────┐  │
│ │ ⚠️ Cloud Action Required                                   │  │
│ │                                                            │  │
│ │ This workflow requires sending data to:                    │  │
│ │   api.anthropic.com (Claude API)                           │  │
│ │                                                            │  │
│ │ What will be sent:                                         │  │
│ │ ┌──────────────────────────────────────────────────────┐   │  │
│ │ │ • Request summary (redacted metadata only)           │   │  │
│ │ │ • No file contents, paths, or code                   │   │  │
│ │ │ • No dependency trees or stack traces                │   │  │
│ │ └──────────────────────────────────────────────────────┘   │  │
│ │                                                            │  │
│ │ [Preview Payload] (expandable)                             │  │
│ │                                                            │  │
│ │ ☐ "Remember this choice" ← NOT AVAILABLE (always ask)      │  │
│ │                                                            │  │
│ │ [Cancel]                    [Send to Cloud]                │  │
│ └────────────────────────────────────────────────────────────┘  │
│                                                                 │
│ IMPORTANT: No "Always Allow" or "Remember" option.              │
│ Every cloud action requires explicit per-action consent.        │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│ STEP 3: User Decision                                           │
│ ─────────────────────────────────────────────────────────────── │
│ [Cancel]:                                                       │
│    • Modal closes                                               │
│    • Workflow does not execute                                  │
│    • No data sent anywhere                                      │
│    • User returned to input step                                │
│                                                                 │
│ [Send to Cloud]:                                                │
│    • Explicit consent logged locally                            │
│    • Workflow proceeds with cloud API call                      │
│    • Transient "cloud action in progress" indicator shown       │
│    • Mode remains unchanged after completion (persisted setting)│
└─────────────────────────────────────────────────────────────────┘
```

**Success criteria:** User makes informed decision; no data leaves without explicit consent.

---

### Journey 6: Error Recovery

**Trigger:** A workflow fails and user needs to understand and fix the problem.

```
┌─────────────────────────────────────────────────────────────────┐
│ STEP 1: Failure Detected                                        │
│ ─────────────────────────────────────────────────────────────── │
│ • CLI exits with non-zero code                                  │
│ • Workflow badge changes to "Failed" (red)                      │
│ • Error toast appears (non-blocking)                            │
│ • Sound/vibration feedback (if enabled)                         │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│ STEP 2: Error Panel                                             │
│ ─────────────────────────────────────────────────────────────── │
│ Shows:                                                          │
│   • Error summary (parsed from CLI output)                      │
│   • Raw CLI output (expandable, syntax highlighted)             │
│   • Timestamp and duration                                      │
│                                                                 │
│ Actions:                                                        │
│   • [Copy Log] - Copy full output for debugging                 │
│   • [Copy CLI Command (redacted)] - Safe shareable command      │
│   • [Retry] - Re-run with same parameters                       │
│   • [View in Logs] - Jump to Logs screen with filter            │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│ STEP 3: Suggested Fixes (if parseable)                          │
│ ─────────────────────────────────────────────────────────────── │
│ Common error patterns with actionable suggestions:              │
│                                                                 │
│ "Ollama not responding":                                        │
│   → "Start Ollama" button (runs `ollama serve`)                 │
│   → "Check Ollama docs" link                                    │
│                                                                 │
│ "Model not found":                                              │
│   → "Pull model" button (runs `ollama pull <model>`)            │
│   → List available models                                       │
│                                                                 │
│ "Permission denied":                                            │
│   → Show path that failed                                       │
│   → Suggest chmod/ownership fix                                 │
│                                                                 │
│ Unknown error:                                                  │
│   → "Search docs" link (sanitized error code only; preview first)│
│   → "Report issue" link (no auto-fill; offer "Copy sanitized    │
│      diagnostics" with explicit preview/consent before sharing) │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│ STEP 4: Resolution                                              │
│ ─────────────────────────────────────────────────────────────── │
│ [User fixes issue externally]:                                  │
│    • Click "Retry" → Workflow re-runs                           │
│    • Success → Badge turns green, attention card clears         │
│                                                                 │
│ [User gives up]:                                                │
│    • Click "Dismiss" → Attention card hidden                    │
│    • Error remains in Logs for future reference                 │
│    • Can access via Logs screen anytime                         │
└─────────────────────────────────────────────────────────────────┘
```

**Success criteria:** User understands what failed and has clear path to resolution.

---

### Keyboard Shortcuts Summary

| Action | Shortcut | Context |
|--------|----------|---------|
| Open Command Palette | ⌘K | Global |
| Navigate to Workflows | ⌘1 | Global |
| Navigate to Agents | ⌘2 | Global |
| Navigate to Credentials | ⌘3 | Global |
| Navigate to Privacy Mode | ⌘4 | Global |
| Navigate to Logs | ⌘5 | Global |
| Execute Selected Workflow | ⌘⇧E | Workflow selected |
| Cancel Running Workflow | ⌘. | Workflow running |
| Open Settings | ⌘, | Global |
| Toggle Dark Mode | ⌘⇧D | Global (P2) |
| Search Logs | ⌘F | Logs screen |
| Copy Log Entry | ⌘C | Log entry selected |

---

### Journey State Machine

```
                    ┌──────────────┐
                    │   LAUNCH     │
                    └──────┬───────┘
                           │
              ┌────────────┼────────────┐
              │            │            │
              ▼            ▼            ▼
        ┌──────────┐ ┌──────────┐ ┌──────────┐
        │ CLI NOT  │ │ NO REPO  │ │  READY   │
        │  FOUND   │ │ SELECTED │ │          │
        └────┬─────┘ └────┬─────┘ └────┬─────┘
             │            │            │
             │    fix     │   select   │
             └────────────┴────────────┘
                           │
                           ▼
                    ┌──────────────┐
              ┌─────│    IDLE      │◄────────────────┐
              │     └──────┬───────┘                 │
              │            │ execute                 │
              │            ▼                         │
              │     ┌──────────────┐                 │
              │     │   RUNNING    │─────┐           │
              │     └──────┬───────┘     │           │
              │            │             │ cancel    │
              │     ┌──────┴──────┐      │           │
              │     ▼             ▼      ▼           │
              │ ┌────────┐   ┌────────┐  ┌────────┐  │
              │ │COMPLETE│   │ FAILED │  │CANCELED│  │
              │ └───┬────┘   └───┬────┘  └───┬────┘  │
              │     │            │           │       │
              │     └────────────┴───────────┘       │
              │                  │                   │
              │                  │ dismiss/retry     │
              └──────────────────┴───────────────────┘
```

## Component Strategy (Step 11)

This section defines the reusable UI component architecture for aiy Desktop, built on shadcn/ui + Tailwind CSS with strict adherence to privacy non-negotiables.

---

### Component Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                      COMPONENT LAYERS                           │
├─────────────────────────────────────────────────────────────────┤
│ SCREENS (Pages)                                                 │
│   WorkflowsScreen, AgentsScreen, CredentialsScreen,             │
│   PrivacyModeScreen, LogsScreen, SettingsScreen                 │
├─────────────────────────────────────────────────────────────────┤
│ FEATURE COMPONENTS (Domain-specific)                            │
│   WorkflowCard, WorkflowExecutionPanel, AgentCard,              │
│   CredentialRow, PrivacyBadge, LogEntry, LogToolbar,            │
│   AttentionCard, CloudConsentModal                              │
├─────────────────────────────────────────────────────────────────┤
│ LAYOUT COMPONENTS (Structure)                                   │
│   AppShell, Header, Sidebar, MainContent, CommandPalette        │
├─────────────────────────────────────────────────────────────────┤
│ SHARED COMPONENTS (Primitives from shadcn/ui)                   │
│   Button, Badge, Input, Select, Checkbox, Dialog, Toast,        │
│   Tooltip, Card, ScrollArea, Separator                          │
└─────────────────────────────────────────────────────────────────┘
```

**Design System:** shadcn/ui (copy-paste components, not npm dependency) + Tailwind CSS
**Icon Library:** Lucide React (bundled, no CDN)
**Font Stack:** System fonts only (offline-first)

---

### Shared Components (shadcn/ui Base)

#### Button

| Variant | Class | Usage |
|---------|-------|-------|
| `default` | `bg-primary text-primary-foreground` | Primary actions (Execute, Save) |
| `secondary` | `bg-secondary text-secondary-foreground` | Secondary actions (Cancel, View) |
| `ghost` | `bg-transparent hover:bg-accent` | Tertiary actions, icon buttons |
| `destructive` | `bg-destructive text-destructive-foreground` | Delete, Stop, Cancel workflow |
| `outline` | `border border-input bg-background` | Form actions, filters |

**Keyboard:** All buttons must be focusable and activatable via Enter/Space.

#### Badge (Status Indicators)

| Variant | Visual | Usage |
|---------|--------|-------|
| `privacy-local` | Green background + shield icon | "Always Local" mode indicator |
| `privacy-hybrid` | Amber background + shield-alert icon | "Hybrid Mode" indicator |
| `status-running` | Blue background + loader icon | Workflow running |
| `status-paused` | Amber background + pause icon | Workflow paused |
| `status-completed` | Green background + check icon | Workflow completed |
| `status-failed` | Red background + x-circle icon | Workflow failed |
| `status-idle` | Gray background | Workflow idle |
| `health-ok` | Green dot | Ollama connected |
| `health-error` | Red dot | Ollama error |
| `health-checking` | Blue loader | Ollama checking |

**Accessibility:** All badges include icon + text label (never color-only).

#### Input

```tsx
interface InputProps {
  type: 'text' | 'password' | 'search';
  placeholder?: string;
  disabled?: boolean;
  error?: string;
  // Focus styling built-in (visible ring, no outline:none without replacement)
}
```

**Focus behavior:** `:focus-visible` ring always visible (2px solid accent, offset 2px).

#### Dialog / Modal

```tsx
interface DialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  title: string;
  description?: string;
  children: React.ReactNode;
  // Escape key closes, focus trap enabled, portal to body
}
```

**Accessibility:** Focus trap, Escape to close, aria-labelledby/describedby.

---

### Layout Components

#### AppShell

The root layout wrapper containing Header, Sidebar, and MainContent.

```tsx
interface AppShellProps {
  repo: RepoContext | null;        // Current repo/workdir
  privacyMode: 'always-local' | 'hybrid';  // Persisted mode (not auto-detected)
  ollamaStatus: HealthStatus;
  activeWorkflow: WorkflowExecution | null;
  children: React.ReactNode;
}
```

**Structure:**
```
┌─────────────────────────────────────────────────────────┐
│ Header (h-12)                                           │
│ [Repo Path] [PrivacyBadge] [OllamaHealth] [SettingsBtn] │
├────────────┬────────────────────────────────────────────┤
│ Sidebar    │ MainContent                                │
│ (w-48)     │ (flex-1, scrollable)                       │
│            │                                            │
│ [Nav Items]│ [Screen Content]                           │
│            │                                            │
└────────────┴────────────────────────────────────────────┘
```

#### Header

```tsx
interface HeaderProps {
  repoPath: string;                // e.g., "~/projects/my-app"
  privacyMode: 'always-local' | 'hybrid';
  ollamaStatus: HealthStatus;
  onPrivacyBadgeClick: () => void; // Navigate to Privacy Mode screen
  onSettingsClick: () => void;
}
```

**Privacy Badge in Header:**
- Always visible (persistent indicator)
- Clickable → navigates to Privacy Mode screen
- Shows current persisted mode (not detection result)
- Mode only changes via explicit user toggle in Privacy Mode screen

#### Sidebar

```tsx
interface SidebarProps {
  activeScreen: ScreenId;
  onNavigate: (screen: ScreenId) => void;
}

type ScreenId = 'workflows' | 'agents' | 'credentials' | 'privacy' | 'logs';
```

**Navigation Items:**
| Screen | Icon | Shortcut |
|--------|------|----------|
| Workflows | ClipboardList | ⌘1 |
| Agents | Bot | ⌘2 |
| Credentials | Key | ⌘3 |
| Privacy Mode | Shield | ⌘4 |
| Logs | FileText | ⌘5 |

#### CommandPalette

Global command palette (⌘K) for keyboard-first interaction.

```tsx
interface CommandPaletteProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onExecute: (command: Command) => void;
}

interface Command {
  id: string;
  label: string;
  description?: string;
  icon?: React.ReactNode;
  shortcut?: string;
  category: 'workflow' | 'navigation' | 'action';
}
```

**Features:**
- Fuzzy search across workflows, agents, navigation
- Category grouping (Workflows, Navigation, Actions)
- Keyboard navigation (↑↓ to select, Enter to execute, Escape to close)
- Recent commands shown first

---

### Feature Components

#### PrivacyBadge

The persistent privacy mode indicator.

```tsx
interface PrivacyBadgeProps {
  mode: 'always-local' | 'hybrid';  // Persisted setting (user-toggled only)
  verificationStatus?: 'passed' | 'failed' | 'pending' | null;
  onClick?: () => void;
}
```

**Critical behavior (preserving non-negotiables):**
- **Mode reflects persisted user setting only** — never auto-detects or auto-flips
- `'always-local'` is default on first launch
- `'hybrid'` only when user explicitly enables via Privacy Mode screen toggle
- Verification status (`passed`/`failed`) is shown as overlay/indicator, **does not change mode**
- Mode persists across sessions (Electron local storage)

**Variants:**
```
┌─────────────────────────┐     ┌─────────────────────────┐
│ 🛡️ Always Local         │     │ ⚠️ Hybrid Mode          │
│ (green bg, white text)  │     │ (amber bg, dark text)   │
└─────────────────────────┘     └─────────────────────────┘

With verification overlay:
┌─────────────────────────┐
│ 🛡️ Always Local    ⚠️   │  ← Small warning icon if verification failed
└─────────────────────────┘
```

#### WorkflowCard

Displays a single workflow with status and actions.

```tsx
interface WorkflowCardProps {
  workflow: Workflow;
  status: 'idle' | 'running' | 'paused' | 'completed' | 'failed';
  isSelected?: boolean;
  onExecute: () => void;
  onSelect?: () => void;
}

interface Workflow {
  id: string;
  name: string;
  description: string;
}
```

**States:**
| Status | Badge | Primary Action |
|--------|-------|----------------|
| idle | Gray "Idle" | "Execute (⌘⇧E)" |
| running | Blue "Running" | "Cancel (⌘.)" |
| paused | Amber "Paused" | "Resume" |
| completed | Green "Completed" | "Run Again" |
| failed | Red "Failed" | "Retry" |

#### WorkflowExecutionPanel

Real-time execution output panel.

```tsx
interface WorkflowExecutionPanelProps {
  workflow: Workflow;
  stage: 'init' | 'executing' | 'finalizing' | 'complete' | 'failed';
  output: LogLine[];
  onCancel: () => void;
  onCopyLog: () => void;
  onCopyCliCommand: () => void;  // Copies redacted CLI command
}
```

**Features:**
- Stage indicator (Init → Executing → Finalizing)
- Real-time streaming log output
- Auto-scroll toggle
- Cancel button (⌘.)
- Copy Log button
- **Copy CLI Command (redacted)** button — safe for sharing

#### LogEntry

Single log line in the Logs view.

```tsx
interface LogEntryProps {
  timestamp: string;
  level: 'info' | 'warn' | 'error';
  message: string;
  isPrivacyMarker?: boolean;  // Highlight [PASS]/[FAIL]
}
```

**Privacy marker highlighting:**
- `[PASS]` → green background highlight
- `[FAIL]` → red background highlight

#### LogToolbar

Toolbar for filtering and searching logs.

```tsx
interface LogToolbarProps {
  searchQuery: string;
  onSearchChange: (query: string) => void;
  levelFilter: 'all' | 'info' | 'warn' | 'error';
  onLevelFilterChange: (level: string) => void;
  autoScroll: boolean;
  onAutoScrollChange: (enabled: boolean) => void;
  onCopyLog: () => void;
  onCopyCliCommand: () => void;  // Redacted
}
```

#### AttentionCard

Priority card for returning users (session recovery).

```tsx
interface AttentionCardProps {
  type: 'paused' | 'failed';
  workflowName: string;
  timestamp: string;
  message: string;
  onPrimaryAction: () => void;   // Resume or Retry
  onSecondaryAction: () => void; // View Details or View Error
  onDismiss: () => void;
}
```

**Variants:**
- `paused` → amber border, "Resume" primary action
- `failed` → red border, "Retry" primary action

#### CloudConsentModal

Explicit per-action consent for cloud operations.

```tsx
interface CloudConsentModalProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  targetService: string;         // e.g., "api.anthropic.com"
  payloadSummary: string[];      // What will be sent (redacted summary)
  onCancel: () => void;
  onConfirm: () => void;
  // NO "remember this choice" option — always ask
}
```

**Critical behavior (preserving non-negotiables):**
- **No "Remember this choice" checkbox** — every cloud action requires explicit consent
- **No "Always allow"** option
- Payload preview shows sanitized summary only
- Cancel = no data sent, workflow does not execute
- Confirm = explicit consent logged locally, workflow proceeds
- **Mode remains unchanged after completion** — does not auto-revert

```
┌─────────────────────────────────────────────────────────┐
│ ⚠️ Cloud Action Required                                │
├─────────────────────────────────────────────────────────┤
│ This workflow requires sending data to:                 │
│   api.anthropic.com (Claude API)                        │
│                                                         │
│ What will be sent:                                      │
│ ┌─────────────────────────────────────────────────────┐ │
│ │ • Request summary (redacted metadata only)          │ │
│ │ • No file contents, paths, or code                  │ │
│ │ • No dependency trees or stack traces               │ │
│ └─────────────────────────────────────────────────────┘ │
│                                                         │
│ [▶ Preview Full Payload]                                │
│                                                         │
│ ☐ Remember this choice ← DISABLED / NOT SHOWN           │
│                                                         │
│              [Cancel]    [Send to Cloud]                │
└─────────────────────────────────────────────────────────┘
```

#### ErrorPanel

Error display with suggested fixes.

```tsx
interface ErrorPanelProps {
  error: ParsedError;
  onRetry: () => void;
  onCopyLog: () => void;
  onCopyCliCommand: () => void;  // Redacted
  onSearchDocs: () => void;      // Opens sanitized search (preview first)
  onReportIssue: () => void;     // Opens GitHub (no auto-fill)
}

interface ParsedError {
  summary: string;
  rawOutput: string;
  suggestedFixes?: SuggestedFix[];
}

interface SuggestedFix {
  description: string;
  action?: () => void;  // e.g., "Start Ollama" button
}
```

**External link behavior (preserving non-negotiables):**
- **"Search docs"** → Opens browser with sanitized query (error code only); shows preview modal first
- **"Report issue"** → Opens GitHub issue page without auto-filled text; offers "Copy sanitized diagnostics" with explicit preview/consent before user includes anything

---

### Privacy Mode Screen Components

#### PrivacyModeToggle

The explicit toggle for switching between Always Local and Hybrid.

```tsx
interface PrivacyModeToggleProps {
  currentMode: 'always-local' | 'hybrid';
  onModeChange: (mode: 'always-local' | 'hybrid') => void;
}
```

**Critical behavior:**
- This is the **only** way to change privacy mode (no auto-detection)
- Switching to Hybrid requires confirmation dialog
- Mode persists in Electron local storage
- No "auto-revert" after cloud actions

#### PrivacyVerificationPanel

Shows `aiy privacy check` results.

```tsx
interface PrivacyVerificationPanelProps {
  status: 'idle' | 'running' | 'passed' | 'failed';
  lastCheckTimestamp?: string;
  rawOutput?: string;
  failures?: string[];
  onVerify: () => void;  // Runs `aiy privacy check`
}
```

**Critical behavior:**
- Verification result **does not change mode**
- PASS = show success message + timestamp
- FAIL = show warning + failures + "How to fix" links; mode unchanged

---

### Initialization Components

#### CliNotFoundDialog

Shown when `aiy` CLI binary cannot be resolved.

```tsx
interface CliNotFoundDialogProps {
  open: boolean;
  onBrowse: () => void;       // Open file picker
  onCheckPath: () => void;    // Retry PATH resolution
  onDownload: () => void;     // Open download page in browser
}
```

#### RepoInitDialog

Shown when selected directory has no `.aiy/` folder.

```tsx
interface RepoInitDialogProps {
  open: boolean;
  repoPath: string;
  onInitialize: () => void;   // Runs `aiy privacy init`
  onOpenAnyway: () => void;   // Continue with limited functionality
  onCancel: () => void;
}
```

**Critical behavior:**
- Initialize button runs **`aiy privacy init`** (no top-level init subcommand exists)
- Can optionally pass `--path <repoPath>` if not running with cwd

---

### External Link Components

#### SanitizedSearchModal

Preview modal before opening external search.

```tsx
interface SanitizedSearchModalProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  sanitizedQuery: string;      // Error code only, no paths/stack traces
  searchUrl: string;           // Full URL that will be opened
  onConfirm: () => void;       // User approves, opens browser
  onCancel: () => void;
}
```

**Display:**
```
┌─────────────────────────────────────────────────────────┐
│ Search Documentation                                    │
├─────────────────────────────────────────────────────────┤
│ The following will be searched:                         │
│                                                         │
│ ┌─────────────────────────────────────────────────────┐ │
│ │ aiy error E1234                                     │ │
│ └─────────────────────────────────────────────────────┘ │
│                                                         │
│ This will open your browser to:                         │
│ docs.example.com/search?q=aiy+error+E1234               │
│                                                         │
│              [Cancel]    [Open in Browser]              │
└─────────────────────────────────────────────────────────┘
```

#### ReportIssueModal

Preview modal before opening GitHub issue page.

```tsx
interface ReportIssueModalProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  sanitizedDiagnostics: string;  // Opt-in copy text
  onCopyDiagnostics: () => void; // Copy to clipboard
  onOpenGitHub: () => void;      // Opens blank issue template
  onCancel: () => void;
}
```

**Critical behavior:**
- **No auto-fill** of issue body with error text
- User must explicitly copy diagnostics and paste if desired
- GitHub opens with blank template

---

### State Management Patterns

#### Privacy Mode State

```tsx
interface PrivacyState {
  mode: 'always-local' | 'hybrid';           // Persisted, user-toggled only
  verificationStatus: 'idle' | 'running' | 'passed' | 'failed';
  lastVerificationTimestamp: string | null;
  verificationFailures: string[];
}

// Mode changes ONLY via explicit user action:
function setPrivacyMode(mode: 'always-local' | 'hybrid'): void;

// Verification NEVER changes mode:
function runPrivacyVerification(): Promise<VerificationResult>;
```

#### Workflow Execution State

```tsx
interface WorkflowExecutionState {
  activeWorkflow: string | null;
  stage: 'idle' | 'init' | 'executing' | 'finalizing' | 'complete' | 'failed' | 'canceled';
  output: LogLine[];
  cloudConsentPending: boolean;
}
```

---

### Accessibility Patterns (Built Into Components)

| Pattern | Implementation |
|---------|----------------|
| Focus visible | All interactive elements have `:focus-visible` ring (2px solid accent) |
| Color + icon | Status never indicated by color alone; always icon + text label |
| Keyboard navigation | All actions accessible via keyboard (Tab, Enter, Space, Escape) |
| Focus trap | Modals trap focus until dismissed |
| Reduced motion | `prefers-reduced-motion` disables animations |
| Screen reader | ARIA labels on icons, live regions for status updates |

---

### Component File Structure

```
src/
├── components/
│   ├── ui/                    # shadcn/ui primitives
│   │   ├── button.tsx
│   │   ├── badge.tsx
│   │   ├── input.tsx
│   │   ├── dialog.tsx
│   │   ├── toast.tsx
│   │   └── ...
│   ├── layout/
│   │   ├── app-shell.tsx
│   │   ├── header.tsx
│   │   ├── sidebar.tsx
│   │   └── command-palette.tsx
│   ├── privacy/
│   │   ├── privacy-badge.tsx
│   │   ├── privacy-mode-toggle.tsx
│   │   ├── privacy-verification-panel.tsx
│   │   └── cloud-consent-modal.tsx
│   ├── workflows/
│   │   ├── workflow-card.tsx
│   │   ├── workflow-execution-panel.tsx
│   │   └── attention-card.tsx
│   ├── logs/
│   │   ├── log-entry.tsx
│   │   ├── log-toolbar.tsx
│   │   └── error-panel.tsx
│   └── external/
│       ├── sanitized-search-modal.tsx
│       └── report-issue-modal.tsx
├── screens/
│   ├── workflows-screen.tsx
│   ├── agents-screen.tsx
│   ├── credentials-screen.tsx
│   ├── privacy-mode-screen.tsx
│   ├── logs-screen.tsx
│   └── settings-screen.tsx
└── hooks/
    ├── use-privacy-mode.ts    # Persisted mode state
    ├── use-workflow-execution.ts
    ├── use-command-palette.ts
    └── use-keyboard-shortcuts.ts
```

## Implementation Decisions (v1 Lock-In)

This section documents the key implementation decisions locked in for v1 shipping. These decisions are final and should not be revisited during implementation without explicit stakeholder approval.

---

### Decision A: Design Direction

**Final Selection:** Ship the **Recommended Combined Approach (B + C + A + F)**

| Direction | Contribution |
|-----------|--------------|
| **B (Workflow-Centric)** | Layout structure — workflow cards as heroes for fast scanning |
| **C (Command Palette Native)** | Keyboard-first interaction — ⌘K command palette for power users |
| **A (Privacy-First)** | Privacy posture — persistent badge + inline verification |
| **F (Log-Transparent)** | Transparency — real-time log view during execution |

**Note:** Step 9 design direction variants (A–F individually) are exploratory only. Implementation follows the combined approach exclusively.

---

### Decision B: Platform Scope

**v1 Required Platforms:**

| Platform | Minimum Version | Required in v1 |
|----------|-----------------|----------------|
| **macOS** | 12.0 (Monterey)+ | ✓ Yes |
| **Windows** | 10 (1903+) | ✓ Yes |
| **Linux** | Ubuntu 20.04+ (or equivalent glibc) | ✓ Yes |

**Implications:**
- All three platforms ship simultaneously in v1 (no phased rollout)
- Platform-specific behaviors (native menus, file dialogs, notifications) must be tested on all three
- CI/CD must produce signed builds for all platforms

---

### Decision C: CLI Binary Strategy

**v1 Approach:** Bundle a **pinned `aiy` binary** inside the app package (per-platform).

**Binary Resolution Order:**
```
1. Bundled binary (inside app resources)     ← Default, version-locked
2. User-specified path (from Settings)       ← Override for development/testing
3. PATH lookup                               ← Fallback if bundled missing
```

**Rationale:**
- Version-lock mitigates CLI output format drift risk
- Bundled binary ensures offline-first functionality
- User override allows advanced users to test newer CLI versions

**Packaging Requirements:**
- Bundle `aiy` binary for each target platform (macOS arm64/x64, Windows x64, Linux x64)
- Binary must be signed (macOS notarization, Windows Authenticode)
- App version must document bundled CLI version in release notes

**Update Coordination:**
- App updates may include updated CLI binary
- CLI version bump requires regression testing of all parsers

---

### Decision D: Auto-Update Strategy

**v1 Approach:** Include auto-updater, but **opt-in (default OFF)**.

| Aspect | Decision |
|--------|----------|
| **Default state** | OFF (no update checks unless user enables) |
| **Settings toggle** | "Check for updates automatically" in Settings screen |
| **Telemetry** | None — no analytics, no usage tracking, no crash reporting |
| **Update check** | When enabled, checks for updates on app launch (configurable frequency) |
| **Download** | Requires explicit user confirmation ("Update available. Download?") |
| **Install** | Requires explicit user confirmation ("Install and restart?") |
| **Signing** | All updates must be code-signed (platform-appropriate) |

**Update Flow (when enabled):**
```
┌─────────────────────────────────────────────────────────────────┐
│ App Launch                                                      │
│ ─────────────────────────────────────────────────────────────── │
│ [Auto-update enabled in Settings?]                              │
│    NO  → Skip update check                                      │
│    YES → Check update server (HTTPS, no telemetry payload)      │
│                                                                 │
│ [Update available?]                                             │
│    NO  → Continue to main app                                   │
│    YES → Show non-blocking notification:                        │
│          "Update available (v1.2.3). [Download] [Later]"        │
│                                                                 │
│ [User clicks Download]                                          │
│    → Download in background                                     │
│    → Verify signature                                           │
│    → Show: "Ready to install. [Install & Restart] [Later]"      │
│                                                                 │
│ [User clicks Install & Restart]                                 │
│    → Apply update                                               │
│    → Restart app                                                │
└─────────────────────────────────────────────────────────────────┘
```

**Privacy Constraints:**
- Update check sends only: app version, platform, architecture
- No user identifiers, no usage data, no crash reports
- Update server logs may record IP (standard HTTPS) but app sends no tracking payload

---

### Non-Negotiables Summary (Preserved)

These constraints remain in force throughout implementation:

| Constraint | Enforcement |
|------------|-------------|
| **Always Local default** | Electron enforces on first launch; persisted in local storage |
| **Hybrid via explicit toggle only** | Only `PrivacyModeToggle` component can change mode |
| **Mode persists (no auto-revert)** | Mode stored in Electron local storage; survives sessions |
| **Per-action cloud consent** | `CloudConsentModal` required for every cloud action; no "remember" |
| **Zero telemetry by default** | No analytics SDK; auto-updater opt-in with minimal payload |
| **Offline-first** | No runtime remote assets; bundled CLI; system fonts |
| **Single repo per window** | No in-window repo switching; "Open Repo" opens new window |

---

### Decision Log

| Date | Decision | Stakeholder |
|------|----------|-------------|
| 2026-01-18 | Lock design direction: Combined B+C+A+F | Product/UX |
| 2026-01-18 | Lock platform scope: macOS + Windows + Linux all v1 | Product |
| 2026-01-18 | Lock CLI strategy: Bundle pinned binary | Engineering |
| 2026-01-18 | Lock update strategy: Opt-in, no telemetry, signed | Product/Security |
