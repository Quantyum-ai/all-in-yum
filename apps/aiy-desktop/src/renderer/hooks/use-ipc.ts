import { useCallback, useEffect, useRef, useState } from 'react';
import type { OutputChunk } from '../../shared/ipc-channels';

/**
 * Hook for managing IPC communication with the main process
 *
 * Provides:
 * - CLI command execution with streaming output
 * - Settings management
 * - Event subscription with automatic cleanup
 */

export interface CliJobState {
  jobId: string | null;
  isRunning: boolean;
  output: OutputChunk[];
  exitCode: number | null;
  error: string | null;
}

/**
 * Hook for running CLI commands with streaming output
 *
 * @example
 * ```tsx
 * const { run, cancel, state } = useCliCommand();
 *
 * // Run a command
 * await run('privacy', ['check']);
 *
 * // Access output
 * console.log(state.output);
 *
 * // Cancel if needed
 * cancel();
 * ```
 */
export function useCliCommand() {
  const [state, setState] = useState<CliJobState>({
    jobId: null,
    isRunning: false,
    output: [],
    exitCode: null,
    error: null,
  });

  // Refs for cleanup functions
  const outputCleanup = useRef<(() => void) | null>(null);
  const exitCleanup = useRef<(() => void) | null>(null);

  // Cleanup on unmount
  useEffect(() => {
    return () => {
      outputCleanup.current?.();
      exitCleanup.current?.();
    };
  }, []);

  /**
   * Run a CLI command
   */
  const run = useCallback(async (command: string, args: string[] = []): Promise<string> => {
    // Clean up previous subscriptions
    outputCleanup.current?.();
    exitCleanup.current?.();

    // Reset state
    setState({
      jobId: null,
      isRunning: true,
      output: [],
      exitCode: null,
      error: null,
    });

    try {
      // Start the command
      const jobId = await window.electronAPI.cli.run(command, args);

      setState((prev) => ({ ...prev, jobId }));

      // Subscribe to output
      outputCleanup.current = window.electronAPI.cli.onOutput((id, chunk) => {
        if (id === jobId) {
          setState((prev) => ({
            ...prev,
            output: [...prev.output, chunk],
          }));
        }
      });

      // Subscribe to exit
      exitCleanup.current = window.electronAPI.cli.onExit((id, code) => {
        if (id === jobId) {
          setState((prev) => ({
            ...prev,
            isRunning: false,
            exitCode: code,
          }));
        }
      });

      return jobId;
    } catch (error) {
      setState((prev) => ({
        ...prev,
        isRunning: false,
        error: error instanceof Error ? error.message : 'Unknown error',
      }));
      throw error;
    }
  }, []);

  /**
   * Cancel the current command
   */
  const cancel = useCallback(async () => {
    const { jobId } = state;
    if (!jobId) return;

    try {
      await window.electronAPI.cli.cancel(jobId);
      setState((prev) => ({
        ...prev,
        isRunning: false,
        exitCode: -1,
      }));
    } catch (error) {
      console.error('Failed to cancel job:', error);
    }
  }, [state.jobId]);

  /**
   * Clear the output buffer
   */
  const clearOutput = useCallback(() => {
    setState((prev) => ({
      ...prev,
      output: [],
    }));
  }, []);

  return {
    state,
    run,
    cancel,
    clearOutput,
  };
}

/**
 * Hook for managing a setting value
 *
 * @example
 * ```tsx
 * const [fontSize, setFontSize, isLoading] = useSetting<number>('fontSize', 14);
 * ```
 */
export function useSetting<T>(key: string, defaultValue: T) {
  const [value, setValue] = useState<T>(defaultValue);
  const [isLoading, setIsLoading] = useState(true);

  // Load initial value
  useEffect(() => {
    let mounted = true;

    async function loadSetting() {
      try {
        const savedValue = await window.electronAPI.settings.get<T>(key);
        if (mounted && savedValue !== undefined) {
          setValue(savedValue);
        }
      } catch (error) {
        console.error(`Failed to load setting ${key}:`, error);
      } finally {
        if (mounted) {
          setIsLoading(false);
        }
      }
    }

    loadSetting();

    return () => {
      mounted = false;
    };
  }, [key]);

  // Subscribe to setting changes
  useEffect(() => {
    const cleanup = window.electronAPI.settings.onChange((changedKey, newValue) => {
      if (changedKey === key) {
        setValue(newValue as T);
      }
    });

    return cleanup;
  }, [key]);

  // Update setting
  const updateValue = useCallback(
    async (newValue: T) => {
      try {
        await window.electronAPI.settings.set(key, newValue);
        setValue(newValue);
      } catch (error) {
        console.error(`Failed to update setting ${key}:`, error);
        throw error;
      }
    },
    [key]
  );

  return [value, updateValue, isLoading] as const;
}
