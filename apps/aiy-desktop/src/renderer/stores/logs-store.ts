import { create } from 'zustand';
import type { OutputChunk } from '../../shared/ipc-channels';

/**
 * Log line with unique ID for rendering
 */
export interface LogLine {
  /** Unique identifier for the log line */
  id: string;
  /** Stream source (stdout or stderr) */
  stream: 'stdout' | 'stderr';
  /** Text content of the line */
  text: string;
  /** ISO timestamp when received */
  timestamp: string;
  /** Job ID this line belongs to */
  jobId: string;
}

/**
 * Level filter options
 */
export type LevelFilter = 'all' | 'stdout' | 'stderr';

/**
 * Logs Panel Store
 *
 * Manages log output state including:
 * - Log lines from CLI processes
 * - Current running job
 * - Search/filter state
 * - Auto-scroll state
 */
export interface LogsState {
  /** All log lines */
  lines: LogLine[];

  /** Currently running job ID (null if none) */
  currentJobId: string | null;

  /** Whether a job is currently running */
  isRunning: boolean;

  /** Search filter text */
  searchFilter: string;

  /** Level filter (all, stdout, stderr) */
  levelFilter: LevelFilter;

  /** Whether auto-scroll is enabled */
  autoScrollEnabled: boolean;

  // Actions
  addLine: (chunk: OutputChunk, jobId: string) => void;
  addLines: (chunks: OutputChunk[], jobId: string) => void;
  clear: () => void;
  setJobId: (id: string | null) => void;
  setIsRunning: (running: boolean) => void;
  setSearchFilter: (filter: string) => void;
  setLevelFilter: (filter: LevelFilter) => void;
  setAutoScrollEnabled: (enabled: boolean) => void;
  getFilteredLines: () => LogLine[];
}

/**
 * Generate unique ID for log lines
 */
let lineCounter = 0;
function generateLineId(): string {
  return `log-${Date.now()}-${++lineCounter}`;
}

/**
 * Maximum number of log lines to keep in memory
 * Prevents memory issues with long-running processes
 */
const MAX_LOG_LINES = 10000;

/**
 * Logs store with state management for CLI output
 */
export const useLogsStore = create<LogsState>((set, get) => ({
  // Initial state
  lines: [],
  currentJobId: null,
  isRunning: false,
  searchFilter: '',
  levelFilter: 'all',
  autoScrollEnabled: true,

  /**
   * Add a single log line from CLI output
   */
  addLine: (chunk: OutputChunk, jobId: string) => {
    const newLine: LogLine = {
      id: generateLineId(),
      stream: chunk.stream,
      text: chunk.text,
      timestamp: chunk.timestamp,
      jobId,
    };

    set((state) => {
      const newLines = [...state.lines, newLine];
      // Trim to max lines if exceeded
      if (newLines.length > MAX_LOG_LINES) {
        return { lines: newLines.slice(-MAX_LOG_LINES) };
      }
      return { lines: newLines };
    });
  },

  /**
   * Add multiple log lines at once (for batched output)
   */
  addLines: (chunks: OutputChunk[], jobId: string) => {
    const newLines: LogLine[] = chunks.map((chunk) => ({
      id: generateLineId(),
      stream: chunk.stream,
      text: chunk.text,
      timestamp: chunk.timestamp,
      jobId,
    }));

    set((state) => {
      const combined = [...state.lines, ...newLines];
      // Trim to max lines if exceeded
      if (combined.length > MAX_LOG_LINES) {
        return { lines: combined.slice(-MAX_LOG_LINES) };
      }
      return { lines: combined };
    });
  },

  /**
   * Clear all log lines
   */
  clear: () => {
    set({ lines: [] });
  },

  /**
   * Set the current job ID
   */
  setJobId: (id: string | null) => {
    set({ currentJobId: id });
  },

  /**
   * Set whether a job is running
   */
  setIsRunning: (running: boolean) => {
    set({ isRunning: running });
  },

  /**
   * Set search filter text
   */
  setSearchFilter: (filter: string) => {
    set({ searchFilter: filter });
  },

  /**
   * Set level filter
   */
  setLevelFilter: (filter: LevelFilter) => {
    set({ levelFilter: filter });
  },

  /**
   * Enable/disable auto-scroll
   */
  setAutoScrollEnabled: (enabled: boolean) => {
    set({ autoScrollEnabled: enabled });
  },

  /**
   * Get filtered log lines based on search and level filters
   */
  getFilteredLines: (): LogLine[] => {
    const { lines, searchFilter, levelFilter } = get();

    return lines.filter((line) => {
      // Level filter
      if (levelFilter !== 'all' && line.stream !== levelFilter) {
        return false;
      }

      // Search filter (case-insensitive)
      if (searchFilter) {
        const searchLower = searchFilter.toLowerCase();
        return line.text.toLowerCase().includes(searchLower);
      }

      return true;
    });
  },
}));
