/**
 * IPC Channel Definitions for aiy Desktop
 *
 * All IPC communication between main and renderer processes goes through
 * these strictly-typed channels. The preload script exposes only these
 * channels via contextBridge.
 *
 * SECURITY REQUIREMENTS (from PRP):
 * - All IPC messages must be validated in main process
 * - Schema validation + allowlisted commands only
 * - NO direct ipcRenderer exposure in renderer
 * - contextBridge is the ONLY way to communicate
 */

/**
 * Request-response channels (invoke pattern)
 * Renderer calls: window.electronAPI.channel(args)
 * Main handles: ipcMain.handle(channel, handler)
 */
export const IPC_CHANNELS = {
  // ============================================
  // CLI Execution Channels
  // ============================================

  /**
   * Start a CLI command execution
   * @request { command: string, args: string[], repoPath?: string }
   * @response { jobId: string, queued: boolean, queuePosition?: number }
   */
  CLI_RUN: 'cli:run',

  /**
   * Cancel a running or queued CLI command
   * @request { jobId: string }
   * @response { success: boolean, error?: string, wasRunning: boolean }
   */
  CLI_CANCEL: 'cli:cancel',

  /**
   * Get status of a specific job
   * @request { jobId: string }
   * @response QueuedJob | null
   */
  CLI_GET_JOB_STATUS: 'cli:get-job-status',

  /**
   * Get all jobs for current window
   * @request void
   * @response QueuedJob[]
   */
  CLI_GET_JOBS: 'cli:get-jobs',

  /**
   * Get CLI binary resolution info
   * @request void
   * @response BinaryResolution
   */
  CLI_GET_BINARY_INFO: 'cli:get-binary-info',

  // ============================================
  // CLI Event Channels (streaming from main to renderer)
  // ============================================

  /**
   * Stream output chunks from CLI process
   * @event { jobId: string, stream: 'stdout' | 'stderr', text: string, timestamp: string }
   */
  CLI_OUTPUT: 'cli:output',
  CLI_ON_OUTPUT: 'cli:output', // Alias for event subscription

  /**
   * Job completion event
   * @event { jobId: string, exitCode: number | null, cancelled: boolean, error?: string, durationMs: number }
   */
  CLI_EXIT: 'cli:exit',
  CLI_ON_EXIT: 'cli:exit', // Alias for event subscription

  // ============================================
  // Settings Channels
  // ============================================

  /**
   * Get a settings value
   * @request { key: string }
   * @response unknown (type depends on key)
   */
  SETTINGS_GET: 'settings:get',

  /**
   * Set a settings value
   * @request { key: string, value: unknown }
   * @response { success: boolean, error?: string }
   */
  SETTINGS_SET: 'settings:set',

  /**
   * Get all settings
   * @request void
   * @response Record<string, unknown>
   */
  SETTINGS_GET_ALL: 'settings:get-all',

  // ============================================
  // App Channels
  // ============================================

  /**
   * Open a new window for a repository
   * @request { repoPath: string }
   * @response { success: boolean, windowId?: string, error?: string }
   */
  APP_OPEN_REPO_WINDOW: 'app:open-repo-window',

  /**
   * Open an external URL in default browser
   * SECURITY: URL must pass protocol allowlist validation
   * @request { url: string }
   * @response { success: boolean, error?: string }
   */
  APP_OPEN_EXTERNAL: 'app:open-external',

  /**
   * Get app version
   * @request void
   * @response string
   */
  APP_GET_VERSION: 'app:get-version',

  /**
   * Get app paths (userData, logs, etc.)
   * @request void
   * @response { userData: string, logs: string, temp: string }
   */
  APP_GET_PATHS: 'app:get-paths',

  /**
   * Show native file dialog for selecting a directory
   * @request { title?: string, defaultPath?: string }
   * @response { cancelled: boolean, path?: string }
   */
  APP_SELECT_DIRECTORY: 'app:select-directory',

  /**
   * Show native file dialog for selecting the CLI binary
   * @request { title?: string }
   * @response { cancelled: boolean, path?: string }
   */
  APP_SELECT_CLI_BINARY: 'app:select-cli-binary',

  /**
   * Open logs folder in system file manager
   * @request void
   * @response { success: boolean, error?: string }
   */
  APP_OPEN_LOGS_FOLDER: 'app:open-logs-folder',

  /**
   * Get current window ID
   * @request void
   * @response string
   */
  APP_GET_WINDOW_ID: 'app:get-window-id',

  // ============================================
  // Privacy Mode Channels
  // ============================================

  /**
   * Get current privacy mode
   * @request void
   * @response { mode: 'always-local' | 'hybrid' }
   */
  PRIVACY_GET_MODE: 'privacy:get-mode',

  /**
   * Set privacy mode (requires explicit user action)
   * @request { mode: 'always-local' | 'hybrid' }
   * @response { success: boolean, error?: string }
   */
  PRIVACY_SET_MODE: 'privacy:set-mode',

  // ============================================
  // Update Channels (opt-in only)
  // ============================================

  /**
   * Check for updates (manual trigger)
   * @request void
   * @response { available: boolean, version?: string, releaseNotes?: string }
   */
  UPDATE_CHECK: 'update:check',

  /**
   * Download available update
   * @request void
   * @response { success: boolean, error?: string }
   */
  UPDATE_DOWNLOAD: 'update:download',

  /**
   * Install downloaded update
   * @request void
   * @response void (app will restart)
   */
  UPDATE_INSTALL: 'update:install',

  /**
   * Update download progress event
   * @event { percent: number, transferred: number, total: number }
   */
  UPDATE_PROGRESS: 'update:progress',
} as const;

/**
 * Channel type for TypeScript
 */
export type IPCChannel = (typeof IPC_CHANNELS)[keyof typeof IPC_CHANNELS];

// ============================================
// Request/Response Types
// ============================================

/**
 * CLI Run request payload
 */
export interface CLIRunRequest {
  command: string;
  args: string[];
  repoPath?: string;
}

/**
 * CLI Run response payload
 */
export interface CLIRunResponse {
  jobId: string;
  queued: boolean;
  queuePosition?: number;
}

/**
 * CLI Cancel request payload
 */
export interface CLICancelRequest {
  jobId: string;
}

/**
 * CLI Cancel response payload
 */
export interface CLICancelResponse {
  success: boolean;
  error?: string;
  wasRunning: boolean;
}

/**
 * Settings Get request payload
 */
export interface SettingsGetRequest {
  key: string;
}

/**
 * Settings Set request payload
 */
export interface SettingsSetRequest {
  key: string;
  value: unknown;
}

/**
 * Settings Set response payload
 */
export interface SettingsSetResponse {
  success: boolean;
  error?: string;
}

/**
 * Open Repo Window request payload
 */
export interface OpenRepoWindowRequest {
  repoPath: string;
}

/**
 * Open Repo Window response payload
 */
export interface OpenRepoWindowResponse {
  success: boolean;
  windowId?: string;
  error?: string;
}

/**
 * Open External URL request payload
 */
export interface OpenExternalRequest {
  url: string;
}

/**
 * Open External response payload
 */
export interface OpenExternalResponse {
  success: boolean;
  error?: string;
}

/**
 * Select Directory request payload
 */
export interface SelectDirectoryRequest {
  title?: string;
  defaultPath?: string;
}

/**
 * Select Directory response payload
 */
export interface SelectDirectoryResponse {
  cancelled: boolean;
  path?: string;
}

/**
 * Privacy Mode type
 */
export type PrivacyMode = 'always-local' | 'hybrid';

/**
 * Privacy Get Mode response payload
 */
export interface PrivacyGetModeResponse {
  mode: PrivacyMode;
}

/**
 * Privacy Set Mode request payload
 */
export interface PrivacySetModeRequest {
  mode: PrivacyMode;
}

/**
 * App Paths response payload
 */
export interface AppPathsResponse {
  userData: string;
  logs: string;
  temp: string;
}

/**
 * Update Check response payload
 */
export interface UpdateCheckResponse {
  available: boolean;
  version?: string;
  releaseNotes?: string;
}

/**
 * Update Progress event payload
 */
export interface UpdateProgressEvent {
  percent: number;
  transferred: number;
  total: number;
}

// ============================================
// URL Allowlist for External Links
// ============================================

/**
 * Allowed URL protocols for opening external links.
 * SECURITY: Only these protocols are allowed.
 */
export const ALLOWED_URL_PROTOCOLS = ['https:', 'http:'] as const;

/**
 * Allowed URL domains for opening external links.
 * SECURITY: URLs to other domains require user confirmation.
 */
export const TRUSTED_URL_DOMAINS = [
  'github.com',
  'docs.anthropic.com',
  'ollama.ai',
  'huggingface.co',
] as const;

/**
 * Validate a URL for external opening
 * @param url The URL to validate
 * @returns { valid: boolean, reason?: string, requiresConfirmation: boolean }
 */
export function validateExternalUrl(url: string): {
  valid: boolean;
  reason?: string;
  requiresConfirmation: boolean;
} {
  try {
    const parsed = new URL(url);

    // Check protocol
    if (!ALLOWED_URL_PROTOCOLS.includes(parsed.protocol as 'https:' | 'http:')) {
      return {
        valid: false,
        reason: `Protocol "${parsed.protocol}" is not allowed. Only https and http are permitted.`,
        requiresConfirmation: false,
      };
    }

    // Check if trusted domain
    const hostname = parsed.hostname.toLowerCase();
    const isTrusted = TRUSTED_URL_DOMAINS.some(
      (domain) => hostname === domain || hostname.endsWith(`.${domain}`)
    );

    return {
      valid: true,
      requiresConfirmation: !isTrusted,
    };
  } catch {
    return {
      valid: false,
      reason: 'Invalid URL format',
      requiresConfirmation: false,
    };
  }
}

// ============================================
// Additional Types for Renderer
// ============================================

/**
 * Output chunk from CLI process (for renderer use)
 */
export interface OutputChunk {
  /** Stream source */
  stream: 'stdout' | 'stderr';
  /** Text content */
  text: string;
  /** Timestamp when received */
  timestamp: string;
}

/**
 * Job status type
 */
export type JobStatus = 'queued' | 'running' | 'completed' | 'failed' | 'cancelled';

/**
 * Job information for display in renderer
 */
export interface JobInfo {
  /** Unique job identifier */
  id: string;
  /** Command being executed */
  command: string;
  /** Command arguments */
  args: string[];
  /** Current status */
  status: JobStatus;
  /** Start time (ISO string) */
  startedAt?: string;
  /** End time (ISO string) */
  completedAt?: string;
  /** Duration in milliseconds */
  durationMs?: number;
  /** Exit code if completed */
  exitCode?: number | null;
  /** Error message if failed */
  error?: string;
}

/**
 * App settings interface
 */
export interface AppSettings {
  /** Privacy mode setting */
  privacyMode: PrivacyMode;
  /** Custom CLI binary path (optional) */
  cliBinaryPath?: string;
  /** Default command timeout in ms */
  defaultTimeoutMs: number;
  /** Auto-update enabled */
  autoUpdateEnabled: boolean;
  /** Last update check timestamp */
  lastUpdateCheck?: number;
}

/**
 * Default settings values
 */
export const DEFAULT_SETTINGS: AppSettings = {
  privacyMode: 'always-local',
  defaultTimeoutMs: 30000,
  autoUpdateEnabled: false,
};
