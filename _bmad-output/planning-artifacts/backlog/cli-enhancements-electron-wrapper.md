# CLI Enhancement Backlog (Post-PRD): Electron Wrapper Support

Goal: Reduce Electron-side text parsing and strengthen defense-in-depth by making the `aiy` CLI more machine-consumable while keeping the CLI as the source of truth.

## Guiding Principles

- Maintain backwards compatibility where feasible (additive flags/fields).
- When `--format json` is used, **all outputs (success + error) are valid JSON**.
- Prefer stable schemas with explicit versioning (`schema_version`) to avoid fragile UI parsers.
- Preserve privacy posture: no code/paths/diffs/stack traces/dependency trees to cloud.

## Proposed Backlog

### CLI-1: Add `--format json` to `aiy agents` commands

**Applies to:** `aiy agents list`, `aiy agents status`, `aiy agents enable`, `aiy agents disable`

- **Problem:** Electron v1 must parse human text output for agents.
- **Proposal:** Support `--format json` for list/status and return structured results for enable/disable.
- **Acceptance criteria:**
  - `aiy agents list --format json` returns an array of agents with stable fields (id, enabled, provider, ready, credential_status).
  - `aiy agents enable|disable <id> --format json` returns `{ok, agent_id, enabled}` and uses non-zero exit code on failure.

### CLI-2: Add `--format json` to `aiy credentials` commands

**Applies to:** `aiy credentials status`, `aiy credentials set`, `aiy credentials get`, `aiy credentials delete`

- **Problem:** Electron v1 must parse text output and avoid logging secrets.
- **Proposal:** Support `--format json` for status and return structured results for set/get/delete (with secret-safe fields).
- **Acceptance criteria:**
  - `aiy credentials status --format json` returns per-provider `{configured: bool}` (never includes the secret value).
  - `aiy credentials set|delete <provider> --format json` returns `{ok, provider}` and a structured error on failure.

### CLI-3: Add `--format json` (and correct exit codes) for `aiy privacy check`

**Applies to:** `aiy privacy check`

- **Problem:** Exit code may be 0 even when output contains `[FAIL]`; UI must parse text markers.
- **Proposal:** Make exit code reliable and add JSON output.
- **Acceptance criteria:**
  - If any check fails, exit code is non-zero.
  - `aiy privacy check --format json` returns `{overall: "pass"|"fail", checks:[{id, status, message}]}`.
  - Text output remains unchanged for default format to avoid breaking users/scripts.

### CLI-4: Add `--format json` to `aiy privacy` config commands

**Applies to:** `aiy privacy enable`, `aiy privacy disable`, `aiy privacy config show|get|set|reset`

- **Problem:** Electron v1 must parse text output to render config state and actions.
- **Proposal:** Provide a JSON config view with stable keys and explicit types.
- **Acceptance criteria:**
  - `aiy privacy config show --format json` returns `{config:{...}}` with explicit booleans/strings (no “pretty text”).
  - Mutating commands return `{ok, changed:[...], config:{...}}` on success.

### CLI-5: Standardize JSON error envelopes (all commands)

- **Problem:** Even when JSON success exists, failures often appear as unstructured stderr.
- **Proposal:** When `--format json` is used, failures return a single JSON object such as:
  - `{ok:false, error:{code, message, details?}}`
- **Acceptance criteria:**
  - For every command supporting `--format json`, success and failure are both valid JSON.
  - Error object never includes secrets; includes actionable `code` values for UI mapping.

### CLI-6: Add defense-in-depth CLI-level “Always Local” enforcement

- **Problem:** v1 “Always Local” is UI-enforced only; direct CLI usage can bypass UI guardrails.
- **Proposal:** Add a CLI config flag (e.g., `privacy.mode=always_local|hybrid`) or equivalent that, when set to Always Local, causes the CLI to refuse cloud-planning/execution operations with a clear error.
- **Acceptance criteria:**
  - When Always Local is active, any cloud operation is rejected regardless of invocation path.
  - Error is explicit (distinct code like `ALWAYS_LOCAL_ENFORCED`) and machine-readable under `--format json`.

### CLI-7: Add `cloud_payload_preview` to `aiy privacy execute --format json` (if absent)

- **Problem:** Electron’s per-action modal (Hybrid mode) needs a CLI-provided preview to avoid UI-side redaction logic.
- **Proposal:** Ensure `aiy privacy execute --format json` includes a `cloud_payload_preview` field when a cloud call is requested, plus a `cloud_destination` identifier.
- **Acceptance criteria:**
  - Preview content is redacted/safe by CLI policy.
  - UI can render preview without any additional transformation.

### CLI-8: Add a safe `request_summary` to `aiy privacy workflow-status --format json`

- **Problem:** Workflow status is metadata-only; UI reconstructs context from logs/memory.
- **Proposal:** Include a short, redacted, truncated summary for display purposes in `workflow-status` JSON output (not persisted to `.aiy/workflow-state.json`).
- **Acceptance criteria:**
  - `workflow-status --format json` returns `{request_summary}` with a defined max length.
  - Summary is never the full request and is safe by redaction policy.

### CLI-9: Wire `CloudAgent` to existing adapters (hybrid planning gap)

- **Problem:** Privacy orchestration defines a `CloudAgent` interface but has no in-repo implementation connecting to provider adapters.
- **Proposal:** Implement a CloudAgent that calls existing provider adapters for planning-only operations under strict redaction and explicit approval gates.
- **Acceptance criteria:**
  - Cloud planning can be invoked through privacy orchestration with a clear audit trail.
  - Redaction boundary is enforced in one place (CLI), not duplicated in UI.

