/**
 * IPC Handlers - Bridge between main and renderer processes
 *
 * All communication between processes goes through these handlers.
 * This file registers all IPC channels and their handlers.
 *
 * SECURITY REQUIREMENTS (from PRP):
 * - ALL IPC messages must be validated in main process
 * - Schema validation + allowlisted commands only
 * - NO direct ipcRenderer exposure in renderer
 * - contextBridge is the ONLY communication method
 */

import { ipcMain, BrowserWindow, shell, app, dialog } from 'electron';
import Store from 'electron-store';
import {
  IPC_CHANNELS,
  DEFAULT_SETTINGS,
  CLIRunRequest,
  CLIRunResponse,
  CLICancelRequest,
  CLICancelResponse,
  SettingsSetResponse,
  OpenExternalResponse,
  AppPathsResponse,
  PrivacyMode,
  PrivacyGetModeResponse,
  SelectDirectoryResponse,
  AppSettings,
  OutputChunk as SharedOutputChunk,
} from '../shared/ipc-channels';
import {
  COMMAND_CATEGORIES,
  OutputChunk,
  JobCompleteEvent,
} from '../shared/cli-service-types';
import { cliService } from './cli-service';

// ============================================
// Command Allowlist (SECURITY)
// ============================================

/**
 * Only these commands are allowed to be executed.
 * Any command not in this list will be rejected.
 */
const ALLOWED_COMMANDS = new Set(Object.keys(COMMAND_CATEGORIES));

/**
 * Validate that a command is in the allowlist
 */
function isCommandAllowed(command: string): boolean {
  // Check exact match first
  if (ALLOWED_COMMANDS.has(command)) {
    return true;
  }
  // Check if command starts with any allowed command prefix
  for (const allowed of ALLOWED_COMMANDS) {
    if (command.startsWith(allowed)) {
      return true;
    }
  }
  return false;
}

// ============================================
// Input Validation Helpers
// ============================================

/**
 * Validate CLI run request
 */
function validateCLIRunRequest(request: unknown): {
  valid: boolean;
  error?: string;
  data?: CLIRunRequest;
} {
  if (!request || typeof request !== 'object') {
    return { valid: false, error: 'Request must be an object' };
  }

  const req = request as Record<string, unknown>;

  // Validate command
  if (typeof req.command !== 'string' || req.command.trim() === '') {
    return { valid: false, error: 'Command must be a non-empty string' };
  }

  // Validate args
  if (!Array.isArray(req.args)) {
    return { valid: false, error: 'Args must be an array' };
  }

  // Validate all args are strings
  for (const arg of req.args) {
    if (typeof arg !== 'string') {
      return { valid: false, error: 'All args must be strings' };
    }
  }

  // Validate repoPath if provided
  if (req.repoPath !== undefined && typeof req.repoPath !== 'string') {
    return { valid: false, error: 'repoPath must be a string if provided' };
  }

  // Check command allowlist
  if (!isCommandAllowed(req.command)) {
    return { valid: false, error: `Command "${req.command}" is not allowed` };
  }

  return {
    valid: true,
    data: {
      command: req.command.trim(),
      args: req.args as string[],
      repoPath: req.repoPath as string | undefined,
    },
  };
}

/**
 * Validate job ID
 */
function validateJobId(jobId: unknown): { valid: boolean; error?: string; data?: string } {
  if (typeof jobId !== 'string' || jobId.trim() === '') {
    return { valid: false, error: 'Job ID must be a non-empty string' };
  }
  return { valid: true, data: jobId.trim() };
}

/**
 * Validate settings key
 */
function validateSettingsKey(key: unknown): { valid: boolean; error?: string; data?: string } {
  if (typeof key !== 'string' || key.trim() === '') {
    return { valid: false, error: 'Settings key must be a non-empty string' };
  }
  // Only allow known setting keys
  const allowedKeys = new Set([
    'privacyMode',
    'cliBinaryPath',
    'defaultTimeoutMs',
    'autoUpdateEnabled',
    'lastUpdateCheck',
    'theme',
    'fontSize',
  ]);
  if (!allowedKeys.has(key)) {
    return { valid: false, error: `Unknown settings key: ${key}` };
  }
  return { valid: true, data: key };
}

/**
 * Validate privacy mode value
 */
function validatePrivacyMode(mode: unknown): { valid: boolean; error?: string; data?: PrivacyMode } {
  if (mode !== 'always-local' && mode !== 'hybrid') {
    return { valid: false, error: 'Privacy mode must be "always-local" or "hybrid"' };
  }
  return { valid: true, data: mode };
}

// ============================================
// Store Configuration
// ============================================

// Typed electron-store for settings
const store = new Store<AppSettings>({
  name: 'aiy-settings',
  defaults: {
    ...DEFAULT_SETTINGS,
  },
});

// ============================================
// URL Allowlist for External Links
// ============================================

const ALLOWED_EXTERNAL_DOMAINS = [
  'github.com',
  'docs.allyum.dev',
  'allyum.dev',
  'docs.anthropic.com',
  'ollama.ai',
  'huggingface.co',
];

const ALLOWED_URL_PROTOCOLS = ['https:', 'http:'];

// ============================================
// Event Subscription Tracking
// ============================================

// Track subscriptions for cleanup
const outputSubscriptions: Map<string, () => void> = new Map();
const completeSubscriptions: Map<string, () => void> = new Map();

/**
 * Subscribe to CLI service events for a job and forward to renderer
 */
function subscribeToJobEvents(jobId: string): void {
  // Subscribe to output events
  const unsubOutput = cliService.onOutput(jobId, (chunk: OutputChunk) => {
    const windows = BrowserWindow.getAllWindows();
    const payload: SharedOutputChunk = {
      stream: chunk.stream,
      text: chunk.text,
      timestamp: chunk.timestamp,
    };
    windows.forEach((win) => {
      if (!win.isDestroyed()) {
        win.webContents.send(IPC_CHANNELS.CLI_OUTPUT, jobId, payload);
      }
    });
  });
  outputSubscriptions.set(jobId, unsubOutput);

  // Subscribe to completion events
  const unsubComplete = cliService.onComplete(jobId, (event: JobCompleteEvent) => {
    const windows = BrowserWindow.getAllWindows();
    windows.forEach((win) => {
      if (!win.isDestroyed()) {
        win.webContents.send(IPC_CHANNELS.CLI_EXIT, {
          jobId: event.jobId,
          exitCode: event.exitCode,
          cancelled: event.cancelled,
          durationMs: event.durationMs,
          error: event.error,
        });
      }
    });

    // Cleanup subscriptions after completion
    cleanupJobSubscriptions(jobId);
  });
  completeSubscriptions.set(jobId, unsubComplete);
}

/**
 * Cleanup subscriptions for a job
 */
function cleanupJobSubscriptions(jobId: string): void {
  const unsubOutput = outputSubscriptions.get(jobId);
  if (unsubOutput) {
    unsubOutput();
    outputSubscriptions.delete(jobId);
  }

  const unsubComplete = completeSubscriptions.get(jobId);
  if (unsubComplete) {
    unsubComplete();
    completeSubscriptions.delete(jobId);
  }
}

// ============================================
// IPC Handler Registration
// ============================================

/**
 * Register all IPC handlers
 * Call this once during app initialization
 */
export function registerIpcHandlers(): void {
  // ============================================
  // CLI Handlers
  // ============================================

  /**
   * cli:run - Start a CLI command
   * Returns a job ID for tracking
   *
   * Supports two formats:
   * 1. Object format: { command: string, args: string[], repoPath?: string }
   * 2. Positional format: (command: string, args: string[]) - for backwards compatibility
   */
  ipcMain.handle(
    IPC_CHANNELS.CLI_RUN,
    async (event, arg1: unknown, arg2?: unknown): Promise<CLIRunResponse | string> => {
      let command: string;
      let args: string[];
      let repoPath: string | undefined;

      // Detect format: object vs positional arguments
      if (typeof arg1 === 'object' && arg1 !== null && 'command' in arg1) {
        // Object format: { command, args, repoPath? }
        const validation = validateCLIRunRequest(arg1);
        if (!validation.valid || !validation.data) {
          console.error(`[IPC] cli:run validation failed: ${validation.error}`);
          throw new Error(validation.error);
        }
        command = validation.data.command;
        args = validation.data.args;
        repoPath = validation.data.repoPath;
      } else if (typeof arg1 === 'string' && Array.isArray(arg2)) {
        // Positional format: (command, args)
        command = arg1.trim();
        args = arg2;

        // Validate command is allowed
        if (!isCommandAllowed(command)) {
          const error = `Command "${command}" is not allowed`;
          console.error(`[IPC] cli:run validation failed: ${error}`);
          throw new Error(error);
        }

        // Validate all args are strings
        for (const arg of args) {
          if (typeof arg !== 'string') {
            throw new Error('All args must be strings');
          }
        }
      } else {
        throw new Error('Invalid cli:run arguments. Expected (command, args) or { command, args }');
      }

      // Get window ID from sender
      const window = BrowserWindow.fromWebContents(event.sender);
      const windowId = window?.id.toString() ?? 'unknown';

      try {
        // Run command through CLI service
        const result = await cliService.run(command, args, {
          repoPath,
          windowId,
        });

        // Subscribe to events for this job
        subscribeToJobEvents(result.jobId);

        // Return format depends on input format for backwards compatibility
        if (typeof arg1 === 'object') {
          return {
            jobId: result.jobId,
            queued: result.queued,
            queuePosition: result.queuePosition,
          };
        } else {
          // Legacy format: return just jobId as string
          return result.jobId;
        }
      } catch (err) {
        const message = err instanceof Error ? err.message : 'Unknown error';
        console.error(`[IPC] cli:run error: ${message}`);
        throw new Error(`Failed to start command: ${message}`);
      }
    }
  );

  /**
   * cli:cancel - Cancel a running or queued command
   */
  ipcMain.handle(
    IPC_CHANNELS.CLI_CANCEL,
    async (_event, request: unknown): Promise<CLICancelResponse> => {
      // Handle both object format and direct jobId string
      let jobId: string;
      if (typeof request === 'string') {
        const validation = validateJobId(request);
        if (!validation.valid || !validation.data) {
          return { success: false, error: validation.error, wasRunning: false };
        }
        jobId = validation.data;
      } else if (request && typeof request === 'object' && 'jobId' in request) {
        const validation = validateJobId((request as CLICancelRequest).jobId);
        if (!validation.valid || !validation.data) {
          return { success: false, error: validation.error, wasRunning: false };
        }
        jobId = validation.data;
      } else {
        return { success: false, error: 'Invalid request format', wasRunning: false };
      }

      try {
        const result = await cliService.cancel(jobId);

        // Cleanup subscriptions
        cleanupJobSubscriptions(jobId);

        return result;
      } catch (err) {
        const message = err instanceof Error ? err.message : 'Unknown error';
        console.error(`[IPC] cli:cancel error: ${message}`);
        return { success: false, error: message, wasRunning: false };
      }
    }
  );

  /**
   * cli:get-job-status - Get status of a specific job
   */
  ipcMain.handle(IPC_CHANNELS.CLI_GET_JOB_STATUS, async (_event, request: unknown) => {
    // Handle both object format and direct jobId string
    let jobId: string;
    if (typeof request === 'string') {
      const validation = validateJobId(request);
      if (!validation.valid || !validation.data) {
        return null;
      }
      jobId = validation.data;
    } else if (request && typeof request === 'object' && 'jobId' in request) {
      const validation = validateJobId((request as { jobId: unknown }).jobId);
      if (!validation.valid || !validation.data) {
        return null;
      }
      jobId = validation.data;
    } else {
      return null;
    }

    return cliService.getJobStatus(jobId) ?? null;
  });

  /**
   * cli:get-jobs - Get all jobs for current window
   */
  ipcMain.handle(IPC_CHANNELS.CLI_GET_JOBS, async (event) => {
    const window = BrowserWindow.fromWebContents(event.sender);
    const windowId = window?.id.toString() ?? 'unknown';
    return cliService.getJobsForWindow(windowId);
  });

  /**
   * cli:get-binary-info - Get CLI binary resolution info
   */
  ipcMain.handle(IPC_CHANNELS.CLI_GET_BINARY_INFO, async () => {
    return cliService.getBinaryInfo();
  });

  // ============================================
  // Settings Handlers
  // ============================================

  /**
   * settings:get - Get a setting value
   */
  ipcMain.handle(IPC_CHANNELS.SETTINGS_GET, async (_event, key: unknown) => {
    const validation = validateSettingsKey(key);
    if (!validation.valid || !validation.data) {
      console.warn(`[IPC] settings:get invalid key: ${validation.error}`);
      return undefined;
    }
    return store.get(validation.data);
  });

  /**
   * settings:set - Set a setting value
   */
  ipcMain.handle(
    IPC_CHANNELS.SETTINGS_SET,
    async (_event, key: unknown, value: unknown): Promise<SettingsSetResponse> => {
      const validation = validateSettingsKey(key);
      if (!validation.valid || !validation.data) {
        return { success: false, error: validation.error };
      }

      try {
        store.set(validation.data, value);

        // Broadcast setting change to all windows
        const windows = BrowserWindow.getAllWindows();
        windows.forEach((win) => {
          if (!win.isDestroyed()) {
            win.webContents.send('settings:changed', validation.data, value);
          }
        });

        return { success: true };
      } catch (err) {
        const message = err instanceof Error ? err.message : 'Unknown error';
        return { success: false, error: message };
      }
    }
  );

  /**
   * settings:get-all - Get all settings
   */
  ipcMain.handle(IPC_CHANNELS.SETTINGS_GET_ALL, async () => {
    return store.store;
  });

  // ============================================
  // Privacy Mode Handlers
  // ============================================

  /**
   * privacy:get-mode - Get current privacy mode
   */
  ipcMain.handle(IPC_CHANNELS.PRIVACY_GET_MODE, async (): Promise<PrivacyGetModeResponse> => {
    const mode = store.get('privacyMode', 'always-local') as PrivacyMode;
    return { mode };
  });

  /**
   * privacy:set-mode - Set privacy mode (requires explicit user action)
   */
  ipcMain.handle(
    IPC_CHANNELS.PRIVACY_SET_MODE,
    async (_event, request: unknown): Promise<SettingsSetResponse> => {
      // Handle both object format and direct mode string
      let mode: PrivacyMode;
      if (typeof request === 'string') {
        const validation = validatePrivacyMode(request);
        if (!validation.valid || !validation.data) {
          return { success: false, error: validation.error };
        }
        mode = validation.data;
      } else if (request && typeof request === 'object' && 'mode' in request) {
        const validation = validatePrivacyMode((request as { mode: unknown }).mode);
        if (!validation.valid || !validation.data) {
          return { success: false, error: validation.error };
        }
        mode = validation.data;
      } else {
        return { success: false, error: 'Invalid request format' };
      }

      try {
        store.set('privacyMode', mode);

        // Broadcast change to all windows
        const windows = BrowserWindow.getAllWindows();
        windows.forEach((win) => {
          if (!win.isDestroyed()) {
            win.webContents.send('settings:changed', 'privacyMode', mode);
          }
        });

        return { success: true };
      } catch (err) {
        const message = err instanceof Error ? err.message : 'Unknown error';
        return { success: false, error: message };
      }
    }
  );

  // ============================================
  // App Handlers
  // ============================================

  /**
   * app:get-version - Get app version
   */
  ipcMain.handle(IPC_CHANNELS.APP_GET_VERSION, async (): Promise<string> => {
    return app.getVersion();
  });

  /**
   * app:get-paths - Get app paths
   */
  ipcMain.handle(IPC_CHANNELS.APP_GET_PATHS, async (): Promise<AppPathsResponse> => {
    return {
      userData: app.getPath('userData'),
      logs: app.getPath('logs'),
      temp: app.getPath('temp'),
    };
  });

  /**
   * app:get-window-id - Get current window ID
   */
  ipcMain.handle(IPC_CHANNELS.APP_GET_WINDOW_ID, async (event): Promise<string> => {
    const window = BrowserWindow.fromWebContents(event.sender);
    return window?.id.toString() ?? 'unknown';
  });

  /**
   * app:open-repo-window - Open a new window for a repository
   */
  ipcMain.handle(IPC_CHANNELS.APP_OPEN_REPO_WINDOW, async (_event, repoPath: unknown) => {
    if (typeof repoPath !== 'string' || repoPath.trim() === '') {
      return { success: false, error: 'Repository path must be a non-empty string' };
    }

    // TODO: Implement multi-window support via WindowManager
    console.log(`[IPC] Would open repo window for: ${repoPath}`);
    return { success: true, windowId: 'placeholder' };
  });

  /**
   * app:open-external - Open external URL (with allowlist validation)
   */
  ipcMain.handle(
    IPC_CHANNELS.APP_OPEN_EXTERNAL,
    async (_event, url: unknown): Promise<OpenExternalResponse> => {
      if (typeof url !== 'string') {
        return { success: false, error: 'URL must be a string' };
      }

      try {
        const urlObj = new URL(url);

        // Validate protocol
        if (!ALLOWED_URL_PROTOCOLS.includes(urlObj.protocol)) {
          console.warn(`[IPC] Blocked URL with disallowed protocol: ${url}`);
          return {
            success: false,
            error: `Protocol "${urlObj.protocol}" is not allowed`,
          };
        }

        // Check if domain is in allowlist
        const isAllowed = ALLOWED_EXTERNAL_DOMAINS.some(
          (domain) =>
            urlObj.hostname === domain || urlObj.hostname.endsWith(`.${domain}`)
        );

        if (isAllowed) {
          await shell.openExternal(url);
          return { success: true };
        } else {
          console.warn(`[IPC] Blocked external URL (not in allowlist): ${url}`);
          return { success: false, error: 'URL domain not in allowlist' };
        }
      } catch (err) {
        const message = err instanceof Error ? err.message : 'Invalid URL';
        console.error(`[IPC] Invalid URL: ${url}`, err);
        return { success: false, error: message };
      }
    }
  );

  /**
   * app:select-directory - Show native directory picker
   */
  ipcMain.handle(
    IPC_CHANNELS.APP_SELECT_DIRECTORY,
    async (event, request: unknown): Promise<SelectDirectoryResponse> => {
      const window = BrowserWindow.fromWebContents(event.sender);
      if (!window) {
        return { cancelled: true };
      }

      // Parse optional title and defaultPath
      let title = 'Select Directory';
      let defaultPath: string | undefined;

      if (request && typeof request === 'object') {
        const req = request as Record<string, unknown>;
        if (typeof req.title === 'string') {
          title = req.title;
        }
        if (typeof req.defaultPath === 'string') {
          defaultPath = req.defaultPath;
        }
      }

      try {
        const result = await dialog.showOpenDialog(window, {
          title,
          defaultPath,
          properties: ['openDirectory', 'createDirectory'],
        });

        if (result.canceled || result.filePaths.length === 0) {
          return { cancelled: true };
        }

        return { cancelled: false, path: result.filePaths[0] };
      } catch (err) {
        console.error('[IPC] Directory selection error:', err);
        return { cancelled: true };
      }
    }
  );

  /**
   * app:select-cli-binary - Show native file picker for CLI binary
   */
  ipcMain.handle(
    IPC_CHANNELS.APP_SELECT_CLI_BINARY,
    async (event, request: unknown): Promise<SelectDirectoryResponse> => {
      const window = BrowserWindow.fromWebContents(event.sender);
      if (!window) {
        return { cancelled: true };
      }

      let title = 'Select AIY CLI Binary';
      if (request && typeof request === 'object') {
        const req = request as Record<string, unknown>;
        if (typeof req.title === 'string') {
          title = req.title;
        }
      }

      try {
        const result = await dialog.showOpenDialog(window, {
          title,
          properties: ['openFile'],
          filters: [
            { name: 'Executable', extensions: process.platform === 'win32' ? ['exe'] : ['*'] },
          ],
        });

        if (result.canceled || result.filePaths.length === 0) {
          return { cancelled: true };
        }

        return { cancelled: false, path: result.filePaths[0] };
      } catch (err) {
        console.error('[IPC] Binary selection error:', err);
        return { cancelled: true };
      }
    }
  );

  /**
   * app:open-logs-folder - Open logs folder in system file manager
   */
  ipcMain.handle(
    IPC_CHANNELS.APP_OPEN_LOGS_FOLDER,
    async (): Promise<{ success: boolean; error?: string }> => {
      try {
        const logsPath = app.getPath('logs');
        await shell.openPath(logsPath);
        return { success: true };
      } catch (err) {
        const message = err instanceof Error ? err.message : 'Unknown error';
        return { success: false, error: message };
      }
    }
  );

  // ============================================
  // Legacy Event Forwarding (for placeholder CLI service)
  // ============================================

  // The placeholder CLI service uses EventEmitter pattern
  // Forward events from CLI service to renderer
  cliService.on('output', (jobId: string, chunk: OutputChunk) => {
    const windows = BrowserWindow.getAllWindows();
    const payload: SharedOutputChunk = {
      stream: chunk.stream,
      text: chunk.text,
      timestamp: chunk.timestamp,
    };
    windows.forEach((win) => {
      if (!win.isDestroyed()) {
        win.webContents.send(IPC_CHANNELS.CLI_ON_OUTPUT, jobId, payload);
      }
    });
  });

  cliService.on('exit', (jobId: string, code: number) => {
    const windows = BrowserWindow.getAllWindows();
    windows.forEach((win) => {
      if (!win.isDestroyed()) {
        win.webContents.send(IPC_CHANNELS.CLI_ON_EXIT, jobId, code);
      }
    });
  });

  console.log('[IPC] All handlers registered');
}

// ============================================
// Cleanup Functions
// ============================================

/**
 * Cleanup function for app shutdown
 * Call this on app 'before-quit' event
 */
export async function cleanupIpcHandlers(): Promise<void> {
  console.log('[IPC] Starting cleanup...');

  // Cleanup all job subscriptions
  for (const [jobId, unsub] of outputSubscriptions) {
    try {
      unsub();
    } catch {
      // Ignore errors during cleanup
    }
    outputSubscriptions.delete(jobId);
  }

  for (const [jobId, unsub] of completeSubscriptions) {
    try {
      unsub();
    } catch {
      // Ignore errors during cleanup
    }
    completeSubscriptions.delete(jobId);
  }

  // Shutdown CLI service
  try {
    await cliService.shutdown();
  } catch (err) {
    console.error('[IPC] Error during CLI service shutdown:', err);
  }

  // Remove all IPC listeners
  ipcMain.removeAllListeners();

  console.log('[IPC] Cleanup complete');
}

/**
 * Register app lifecycle handlers
 * Call this once during app initialization (after registerIpcHandlers)
 */
export function registerAppLifecycleHandlers(): void {
  // Cleanup on app quit
  app.on('before-quit', async (event) => {
    // Prevent immediate quit to allow async cleanup
    event.preventDefault();

    await cleanupIpcHandlers();

    // Now actually quit
    app.exit(0);
  });

  // Cleanup window-specific resources when a window closes
  app.on('browser-window-created', (_event, window) => {
    window.on('closed', async () => {
      const windowId = window.id.toString();
      try {
        await cliService.cleanupWindow(windowId);
      } catch {
        // Window cleanup errors are non-fatal
      }
    });
  });

  console.log('[IPC] App lifecycle handlers registered');
}
