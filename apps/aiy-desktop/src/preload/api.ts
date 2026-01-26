/**
 * Preload API - Type-safe API exposed to renderer
 *
 * This defines the exact shape of the API available to the renderer.
 * All communication with the main process goes through this interface.
 */

import { ipcRenderer, IpcRendererEvent } from 'electron';
import { IPC_CHANNELS, OutputChunk } from '../shared/ipc-channels';

/**
 * Options for running CLI commands
 */
interface CLIRunOptions {
  /** Repository path to use as working directory */
  repoPath?: string;
}

/**
 * CLI API - For running and managing CLI commands
 */
const cliAPI = {
  /**
   * Run a CLI command
   * @param command - The command to run
   * @param args - Command arguments
   * @param options - Optional settings including repoPath
   * @returns Job ID for tracking
   */
  run: (command: string, args: string[], options?: CLIRunOptions): Promise<string> => {
    if (options?.repoPath) {
      // Use object format when repoPath is provided
      return ipcRenderer
        .invoke(IPC_CHANNELS.CLI_RUN, {
          command,
          args,
          repoPath: options.repoPath,
        })
        .then((result) => {
          // Handle both string (legacy) and object response formats
          return typeof result === 'string' ? result : result.jobId;
        });
    }
    // Use legacy positional format
    return ipcRenderer.invoke(IPC_CHANNELS.CLI_RUN, command, args);
  },

  /**
   * Cancel a running command
   * @param jobId - The job ID to cancel
   */
  cancel: (jobId: string): Promise<void> => {
    return ipcRenderer.invoke(IPC_CHANNELS.CLI_CANCEL, jobId);
  },

  /**
   * Subscribe to command output
   * @param callback - Called when output is received
   * @returns Cleanup function to unsubscribe
   */
  onOutput: (callback: (jobId: string, chunk: OutputChunk) => void): (() => void) => {
    const handler = (_event: IpcRendererEvent, jobId: string, chunk: OutputChunk): void => {
      callback(jobId, chunk);
    };
    ipcRenderer.on(IPC_CHANNELS.CLI_ON_OUTPUT, handler);
    return () => {
      ipcRenderer.removeListener(IPC_CHANNELS.CLI_ON_OUTPUT, handler);
    };
  },

  /**
   * Subscribe to command exit events
   * @param callback - Called when command exits
   * @returns Cleanup function to unsubscribe
   */
  onExit: (callback: (jobId: string, code: number) => void): (() => void) => {
    const handler = (_event: IpcRendererEvent, jobId: string, code: number): void => {
      callback(jobId, code);
    };
    ipcRenderer.on(IPC_CHANNELS.CLI_ON_EXIT, handler);
    return () => {
      ipcRenderer.removeListener(IPC_CHANNELS.CLI_ON_EXIT, handler);
    };
  },

  /**
   * Get CLI binary resolution info
   * @returns Binary path, source, version, and compatibility info
   */
  getBinaryInfo: (): Promise<{
    path: string;
    source: 'bundled' | 'user-specified' | 'path-lookup';
    version?: string;
    compatible: boolean;
    versionWarning?: string;
  } | undefined> => {
    return ipcRenderer.invoke(IPC_CHANNELS.CLI_GET_BINARY_INFO);
  },
};

/**
 * Settings API - For getting and setting app settings
 */
const settingsAPI = {
  /**
   * Get a setting value
   * @param key - Setting key
   */
  get: <T>(key: string): Promise<T | undefined> => {
    return ipcRenderer.invoke(IPC_CHANNELS.SETTINGS_GET, key);
  },

  /**
   * Set a setting value
   * @param key - Setting key
   * @param value - New value
   */
  set: <T>(key: string, value: T): Promise<void> => {
    return ipcRenderer.invoke(IPC_CHANNELS.SETTINGS_SET, key, value);
  },

  /**
   * Subscribe to setting changes
   * @param callback - Called when a setting changes
   * @returns Cleanup function to unsubscribe
   */
  onChange: (callback: (key: string, value: unknown) => void): (() => void) => {
    const handler = (_event: IpcRendererEvent, key: string, value: unknown): void => {
      callback(key, value);
    };
    ipcRenderer.on('settings:changed', handler);
    return () => {
      ipcRenderer.removeListener('settings:changed', handler);
    };
  },
};

/**
 * App API - For app-level operations
 */
const appAPI = {
  /**
   * Open a new window for a repository
   * @param repoPath - Path to the repository
   */
  openRepoWindow: (repoPath: string): Promise<void> => {
    return ipcRenderer.invoke(IPC_CHANNELS.APP_OPEN_REPO_WINDOW, repoPath);
  },

  /**
   * Open an external URL (must be in allowlist)
   * @param url - URL to open
   */
  openExternal: (url: string): Promise<{ success: boolean; error?: string }> => {
    return ipcRenderer.invoke(IPC_CHANNELS.APP_OPEN_EXTERNAL, url);
  },

  /**
   * Get the app version
   */
  getVersion: (): Promise<string> => {
    return ipcRenderer.invoke(IPC_CHANNELS.APP_GET_VERSION);
  },

  /**
   * Show native file dialog for selecting a directory
   * @param options - Dialog options (title, defaultPath)
   */
  selectDirectory: (options?: {
    title?: string;
    defaultPath?: string;
  }): Promise<{ cancelled: boolean; path?: string }> => {
    return ipcRenderer.invoke(IPC_CHANNELS.APP_SELECT_DIRECTORY, options ?? {});
  },

  /**
   * Show native file dialog for selecting the CLI binary
   * @param options - Dialog options (title)
   */
  selectCliBinary: (options?: {
    title?: string;
  }): Promise<{ cancelled: boolean; path?: string }> => {
    return ipcRenderer.invoke(IPC_CHANNELS.APP_SELECT_CLI_BINARY, options ?? {});
  },

  /**
   * Open logs folder in system file manager
   */
  openLogsFolder: (): Promise<{ success: boolean; error?: string }> => {
    return ipcRenderer.invoke(IPC_CHANNELS.APP_OPEN_LOGS_FOLDER);
  },
};

/**
 * Complete Electron API exposed to renderer
 */
export const electronAPI = {
  cli: cliAPI,
  settings: settingsAPI,
  app: appAPI,
};

/**
 * Type definitions for the API
 */
export type ElectronAPI = typeof electronAPI;
