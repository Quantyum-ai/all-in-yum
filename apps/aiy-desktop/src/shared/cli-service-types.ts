/**
 * CLI Service Type Definitions for aiy Desktop
 *
 * This module defines the interfaces for the CLI service that runs in the
 * Electron main process. The CLI service is responsible for:
 * - Resolving the aiy binary location
 * - Spawning CLI processes with shell: false
 * - Streaming stdout/stderr to the renderer
 * - Managing the per-repo command queue
 * - Tracking PIDs for cleanup on app exit
 * - Implementing cancel with process tree cleanup
 */

/**
 * Binary resolution order (from PRP):
 * 1. Bundled binary (inside app resources) - Default, version-locked
 * 2. User-specified path (from Settings) - Override for development/testing
 * 3. PATH lookup - Fallback if bundled missing
 */
export type BinarySource = 'bundled' | 'user-specified' | 'path-lookup';

/**
 * CLI binary resolution result
 */
export interface BinaryResolution {
  /** Full path to the aiy binary */
  path: string;
  /** How the binary was resolved */
  source: BinarySource;
  /** Version string from `aiy version` */
  version?: string;
  /** True if version is compatible with this app version */
  compatible: boolean;
  /** Warning message if there's a version mismatch */
  versionWarning?: string;
}

/**
 * Command category determines queue behavior.
 * Mutating commands are serialized per-repo.
 * Read-only commands can run concurrently.
 */
export type CommandCategory = 'mutating' | 'read-only';

/**
 * Map of commands to their categories
 */
export const COMMAND_CATEGORIES: Record<string, CommandCategory> = {
  // Privacy commands
  'privacy init': 'mutating',
  'privacy enable': 'mutating',
  'privacy disable': 'mutating',
  'privacy execute': 'mutating',
  'privacy resume': 'mutating',
  'privacy cancel': 'mutating',
  'privacy status': 'read-only',
  'privacy check': 'read-only',
  'privacy workflow-status': 'read-only',
  'privacy config show': 'read-only',
  'privacy config set': 'mutating',

  // Agent commands
  'agents list': 'read-only',
  'agents status': 'read-only',
  'agents enable': 'mutating',
  'agents disable': 'mutating',

  // Credential commands
  'credentials status': 'read-only',
  'credentials set': 'mutating',
  'credentials delete': 'mutating',
  'credentials get': 'read-only',

  // Other commands
  'version': 'read-only',
  'config show': 'read-only',
  'config path': 'read-only',
  'config set': 'mutating',
  'config reset': 'mutating',
};

/**
 * Job status
 */
export type JobStatus = 'queued' | 'running' | 'completed' | 'failed' | 'cancelled';

/**
 * A job in the command queue
 */
export interface QueuedJob {
  /** Unique job identifier */
  id: string;
  /** Command to execute (e.g., "privacy check") */
  command: string;
  /** Command arguments */
  args: string[];
  /** Command category */
  category: CommandCategory;
  /** Repository path this command targets (for queue serialization) */
  repoPath?: string;
  /** Window ID that initiated the command */
  windowId: string;
  /** Current status */
  status: JobStatus;
  /** When the job was queued */
  queuedAt: Date;
  /** When the job started running */
  startedAt?: Date;
  /** When the job completed */
  completedAt?: Date;
  /** Process ID (set when running) */
  pid?: number;
  /** Exit code (set when completed) */
  exitCode?: number;
  /** Error message if failed */
  error?: string;
  /** Timeout in milliseconds (default varies by command type) */
  timeoutMs: number;
  /** Timeout handle for cleanup */
  timeoutHandle?: NodeJS.Timeout;
}

/**
 * Default timeouts by command type (from PRD)
 */
export const DEFAULT_TIMEOUTS: Record<string, number> = {
  'privacy execute': 60000, // 60 seconds for planning operations
  'privacy resume': 60000,
  default: 30000, // 30 seconds for other operations
};

/**
 * Output chunk sent to renderer during streaming
 */
export interface OutputChunk {
  /** Job ID this output belongs to */
  jobId: string;
  /** Stream source */
  stream: 'stdout' | 'stderr';
  /** Text content */
  text: string;
  /** Timestamp when received */
  timestamp: string;
}

/**
 * Job completion event sent to renderer
 */
export interface JobCompleteEvent {
  /** Job ID */
  jobId: string;
  /** Exit code (null if killed/cancelled) */
  exitCode: number | null;
  /** Whether job was cancelled by user */
  cancelled: boolean;
  /** Error message if any */
  error?: string;
  /** Duration in milliseconds */
  durationMs: number;
}

/**
 * CLI Service configuration
 */
export interface CLIServiceConfig {
  /** User-specified binary path (overrides bundled) */
  customBinaryPath?: string;
  /** Default timeout for commands (milliseconds) */
  defaultTimeoutMs: number;
  /** Maximum concurrent read-only jobs per repo */
  maxConcurrentReadOnly: number;
  /** Whether to enable verbose logging */
  verbose: boolean;
}

/**
 * Default CLI service configuration
 */
export const DEFAULT_CLI_CONFIG: CLIServiceConfig = {
  defaultTimeoutMs: 30000,
  maxConcurrentReadOnly: 3,
  verbose: false,
};

/**
 * Result of attempting to run a command
 */
export interface RunCommandResult {
  /** Job ID for tracking */
  jobId: string;
  /** Whether job was queued (true) or started immediately (false) */
  queued: boolean;
  /** Queue position if queued */
  queuePosition?: number;
}

/**
 * Result of cancelling a command
 */
export interface CancelCommandResult {
  /** Whether cancel was successful */
  success: boolean;
  /** Error message if cancel failed */
  error?: string;
  /** Whether the job was running (vs queued) */
  wasRunning: boolean;
}

/**
 * CLI Service interface exposed to IPC handlers
 */
export interface ICLIService {
  /**
   * Initialize the service - resolve binary, check version
   * @param force If true, reinitialize even if already initialized (e.g., when settings change)
   */
  initialize(force?: boolean): Promise<BinaryResolution>;

  /**
   * Run a CLI command
   * @param command The command (e.g., "privacy check")
   * @param args Additional arguments
   * @param options Execution options
   * @returns Job ID and queue status
   */
  run(
    command: string,
    args: string[],
    options: {
      repoPath?: string;
      windowId: string;
      timeoutMs?: number;
    }
  ): Promise<RunCommandResult>;

  /**
   * Cancel a running or queued command
   * @param jobId The job to cancel
   * @returns Cancel result
   */
  cancel(jobId: string): Promise<CancelCommandResult>;

  /**
   * Get status of a job
   * @param jobId The job ID
   * @returns Job status or undefined if not found
   */
  getJobStatus(jobId: string): QueuedJob | undefined;

  /**
   * Get all jobs for a window
   * @param windowId The window ID
   * @returns All jobs (queued and completed)
   */
  getJobsForWindow(windowId: string): QueuedJob[];

  /**
   * Subscribe to output chunks for a job
   * @param jobId The job ID
   * @param callback Called with each output chunk
   * @returns Unsubscribe function
   */
  onOutput(jobId: string, callback: (chunk: OutputChunk) => void): () => void;

  /**
   * Subscribe to job completion events
   * @param jobId The job ID
   * @param callback Called when job completes
   * @returns Unsubscribe function
   */
  onComplete(jobId: string, callback: (event: JobCompleteEvent) => void): () => void;

  /**
   * Clean up all jobs for a window (called when window closes)
   * @param windowId The window ID
   */
  cleanupWindow(windowId: string): Promise<void>;

  /**
   * Clean up all running processes (called on app exit)
   */
  shutdown(): Promise<void>;

  /**
   * Get current binary resolution info
   */
  getBinaryInfo(): BinaryResolution | undefined;
}

/**
 * Process tree kill strategy by platform (from PRP)
 */
export const KILL_STRATEGIES = {
  /**
   * Windows: Use taskkill with /T flag for tree kill
   * Command: taskkill /PID <pid> /T /F
   */
  win32: {
    command: 'taskkill',
    args: (pid: number) => ['/PID', pid.toString(), '/T', '/F'],
  },

  /**
   * macOS/Linux: Kill process group
   * First try SIGTERM, then SIGKILL after timeout
   */
  unix: {
    signal: 'SIGTERM' as const,
    forceSignal: 'SIGKILL' as const,
    forceTimeout: 5000, // 5 seconds before SIGKILL
  },
} as const;
