/**
 * Parser Type Definitions
 *
 * Common types used by all CLI output parsers.
 * Based on actual output formats from aiy CLI Rust implementation.
 */

/**
 * Check marker types as output by CLI
 * Colors in CLI:
 * - PASS: green
 * - FAIL: red
 * - WARN: yellow
 * - SKIP: yellow
 * - INFO: blue
 */
export type CheckMarker = 'PASS' | 'FAIL' | 'WARN' | 'SKIP' | 'INFO';

/**
 * A single check result from privacy check output
 */
export interface CheckResult {
  /** The marker type (PASS, FAIL, etc.) */
  marker: CheckMarker;
  /** The check description/name */
  name: string;
  /** Additional context message if present */
  message?: string;
  /** Indentation level (for nested checks) */
  level: number;
}

/**
 * Privacy check command result
 */
export interface PrivacyCheckResult {
  /** True if ALL checks passed (no FAIL markers) */
  passed: boolean;
  /** Individual check results */
  checks: CheckResult[];
  /** Count of each marker type */
  summary: {
    pass: number;
    fail: number;
    warn: number;
    skip: number;
    info: number;
  };
  /** Original raw output for display */
  rawOutput: string;
}

/**
 * Privacy status result (from `aiy privacy status`)
 */
export interface PrivacyStatusResult {
  /** Whether privacy mode is enabled */
  enabled: boolean;
  /** Local executor configuration */
  localExecutor?: {
    ollamaUrl: string;
    model: string;
    contextSize?: number;
    timeout?: number;
  };
  /** RAG settings if configured */
  ragSettings?: {
    tokenBudget: number;
    topK: number;
    minSimilarity: number;
  };
  /** Verification limits */
  verificationLimits?: {
    maxFmtRepairs: number;
    maxClippyRepairs: number;
    maxTestRepairs: number;
    maxGlobalRepairs: number;
  };
  /** Exclude patterns */
  excludePatterns?: string[];
  /** Original raw output */
  rawOutput: string;
}

/**
 * Workflow state values from workflow-state.json
 */
export type WorkflowState =
  | 'uninitialized'
  | 'ready'
  | 'executing'
  | 'paused'
  | 'completed'
  | 'failed'
  | 'cancelled';

/**
 * Workflow status result (from `aiy privacy workflow-status --format json`)
 */
export interface WorkflowStatusResult {
  /** Current workflow state */
  state: WorkflowState;
  /** Unix timestamp when workflow was created */
  createdAt?: number;
  /** Unix timestamp when workflow was last updated */
  updatedAt?: number;
  /** Total number of tasks */
  totalTasks?: number;
  /** Number of completed tasks */
  tasksCompleted?: number;
  /** Number of failed tasks */
  tasksFailed?: number;
  /** Total repair attempts */
  totalRepairs?: number;
  /** Count of modified files */
  filesModifiedCount?: number;
  /** Number of chunks indexed for RAG */
  chunksIndexed?: number;
  /** Number of cloud API requests made */
  cloudRequests?: number;
  /** Number of local executions */
  localExecutions?: number;
  /** Total duration in milliseconds */
  durationMs?: number;
  /** Error message if failed */
  errorMessage?: string | null;
  /** Original raw output/JSON */
  rawOutput: string;
}

/**
 * Known agent IDs from the CLI registry
 */
export type KnownAgentId = 'grok' | 'claude' | 'gemini' | 'codex' | 'ollama';

/**
 * Agent information from agents list/status
 */
export interface AgentInfo {
  /** Agent identifier (e.g., "grok", "claude") */
  id: string;
  /** Display name (e.g., "Grok (xAI)") */
  name: string;
  /** Whether the agent is enabled */
  enabled: boolean;
  /** Whether credentials are configured */
  hasCredentials: boolean;
  /** Credential provider ID (e.g., "xai", "anthropic") */
  credentialProvider?: string;
}

/**
 * Agents list result
 */
export interface AgentsListResult {
  /** List of all agents */
  agents: AgentInfo[];
  /** Original raw output */
  rawOutput: string;
}

/**
 * Agent status entry (from status command)
 */
export interface AgentStatusEntry {
  /** Agent ID */
  id: string;
  /** Agent display name */
  name: string;
  /** Whether agent is ready (credentials configured) */
  ready: boolean;
  /** Status message */
  message?: string;
}

/**
 * Agents status result
 */
export interface AgentsStatusResult {
  /** Status entries for enabled agents */
  entries: AgentStatusEntry[];
  /** Summary counts */
  summary: {
    ready: number;
    total: number;
  };
  /** Original raw output */
  rawOutput: string;
}

/**
 * Known credential providers
 */
export type CredentialProvider = 'xai' | 'anthropic' | 'google' | 'openai';

/**
 * Credential status for a provider
 */
export interface CredentialStatus {
  /** Provider identifier */
  provider: string;
  /** Whether credentials are configured */
  configured: boolean;
}

/**
 * Credentials status result
 */
export interface CredentialsStatusResult {
  /** Storage backend description */
  backend: string;
  /** Number of stored credentials */
  storedCount: number;
  /** List of configured providers */
  providers: CredentialStatus[];
  /** All known providers with their status */
  allProviders: CredentialStatus[];
  /** Original raw output */
  rawOutput: string;
}

/**
 * Parser error - returned when parsing fails instead of throwing
 */
export interface ParserError {
  /** Error type */
  type: 'empty_input' | 'invalid_format' | 'missing_required_field' | 'json_parse_error';
  /** Error message */
  message: string;
  /** The problematic input (truncated) */
  input?: string;
}

/**
 * Result wrapper that includes potential errors
 */
export type ParseResult<T> =
  | { success: true; data: T }
  | { success: false; error: ParserError; rawOutput: string };
