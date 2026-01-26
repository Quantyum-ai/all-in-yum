/**
 * Workflow Status Output Parser
 *
 * Parses JSON output from `aiy privacy workflow-status --format json` command.
 *
 * Expected JSON format (from Rust workflow/state.rs):
 * ```json
 * {
 *   "state": "ready",
 *   "created_at": 1705680000,
 *   "updated_at": 1705680100,
 *   "total_tasks": 5,
 *   "tasks_completed": 3,
 *   "tasks_failed": 0,
 *   "total_repairs": 2,
 *   "files_modified_count": 4,
 *   "chunks_indexed": 150,
 *   "cloud_requests": 1,
 *   "local_executions": 4,
 *   "duration_ms": 45000,
 *   "error_message": null
 * }
 * ```
 *
 * State values: uninitialized, ready, executing, paused, completed, failed, cancelled
 */

import type { WorkflowState, WorkflowStatusResult, ParseResult } from './types';

/**
 * Valid workflow states
 */
const VALID_STATES: WorkflowState[] = [
  'uninitialized',
  'ready',
  'executing',
  'paused',
  'completed',
  'failed',
  'cancelled',
];

/**
 * Raw JSON structure from CLI
 */
interface RawWorkflowStatusJson {
  state?: string;
  created_at?: number;
  updated_at?: number;
  total_tasks?: number;
  tasks_completed?: number;
  tasks_failed?: number;
  total_repairs?: number;
  files_modified_count?: number;
  chunks_indexed?: number;
  cloud_requests?: number;
  local_executions?: number;
  duration_ms?: number;
  error_message?: string | null;
}

/**
 * Validate and normalize workflow state
 */
function normalizeState(state: string | undefined): WorkflowState {
  if (!state) return 'uninitialized';

  const normalized = state.toLowerCase().trim();

  // Map variations to canonical states
  const stateMap: Record<string, WorkflowState> = {
    uninitialized: 'uninitialized',
    not_initialized: 'uninitialized',
    ready: 'ready',
    idle: 'ready',
    executing: 'executing',
    running: 'executing',
    in_progress: 'executing',
    paused: 'paused',
    suspended: 'paused',
    completed: 'completed',
    done: 'completed',
    success: 'completed',
    failed: 'failed',
    error: 'failed',
    cancelled: 'cancelled',
    canceled: 'cancelled',
    aborted: 'cancelled',
  };

  return stateMap[normalized] || 'uninitialized';
}

/**
 * Parse workflow status JSON output.
 *
 * @param jsonOutput - Raw JSON output from `aiy privacy workflow-status --format json`
 * @returns Parsed result with workflow state and progress
 *
 * @example
 * ```typescript
 * const result = parseWorkflowStatus(cliOutput);
 * if (result.success) {
 *   console.log(`State: ${result.data.state}`);
 *   console.log(`Progress: ${result.data.tasksCompleted}/${result.data.totalTasks}`);
 * }
 * ```
 */
export function parseWorkflowStatus(jsonOutput: string): ParseResult<WorkflowStatusResult> {
  // Handle empty input
  if (!jsonOutput || jsonOutput.trim().length === 0) {
    return {
      success: false,
      error: {
        type: 'empty_input',
        message: 'Workflow status output is empty',
      },
      rawOutput: jsonOutput || '',
    };
  }

  // Try to extract JSON from output (may have non-JSON prefix/suffix)
  let jsonStr = jsonOutput.trim();

  // Find JSON object boundaries
  const jsonStart = jsonStr.indexOf('{');
  const jsonEnd = jsonStr.lastIndexOf('}');

  if (jsonStart === -1 || jsonEnd === -1 || jsonEnd <= jsonStart) {
    return {
      success: false,
      error: {
        type: 'invalid_format',
        message: 'No valid JSON object found in output',
        input: jsonOutput.slice(0, 200),
      },
      rawOutput: jsonOutput,
    };
  }

  jsonStr = jsonStr.slice(jsonStart, jsonEnd + 1);

  // Parse JSON
  let parsed: RawWorkflowStatusJson;
  try {
    parsed = JSON.parse(jsonStr);
  } catch (e) {
    return {
      success: false,
      error: {
        type: 'json_parse_error',
        message: `Failed to parse JSON: ${e instanceof Error ? e.message : 'Unknown error'}`,
        input: jsonStr.slice(0, 200),
      },
      rawOutput: jsonOutput,
    };
  }

  // Build result
  const result: WorkflowStatusResult = {
    state: normalizeState(parsed.state),
    rawOutput: jsonOutput,
  };

  // Copy optional fields if present
  if (typeof parsed.created_at === 'number') {
    result.createdAt = parsed.created_at;
  }
  if (typeof parsed.updated_at === 'number') {
    result.updatedAt = parsed.updated_at;
  }
  if (typeof parsed.total_tasks === 'number') {
    result.totalTasks = parsed.total_tasks;
  }
  if (typeof parsed.tasks_completed === 'number') {
    result.tasksCompleted = parsed.tasks_completed;
  }
  if (typeof parsed.tasks_failed === 'number') {
    result.tasksFailed = parsed.tasks_failed;
  }
  if (typeof parsed.total_repairs === 'number') {
    result.totalRepairs = parsed.total_repairs;
  }
  if (typeof parsed.files_modified_count === 'number') {
    result.filesModifiedCount = parsed.files_modified_count;
  }
  if (typeof parsed.chunks_indexed === 'number') {
    result.chunksIndexed = parsed.chunks_indexed;
  }
  if (typeof parsed.cloud_requests === 'number') {
    result.cloudRequests = parsed.cloud_requests;
  }
  if (typeof parsed.local_executions === 'number') {
    result.localExecutions = parsed.local_executions;
  }
  if (typeof parsed.duration_ms === 'number') {
    result.durationMs = parsed.duration_ms;
  }
  if ('error_message' in parsed) {
    result.errorMessage = parsed.error_message;
  }

  return {
    success: true,
    data: result,
  };
}

/**
 * Check if workflow is in a terminal state
 */
export function isTerminalState(state: WorkflowState): boolean {
  return ['completed', 'failed', 'cancelled'].includes(state);
}

/**
 * Check if workflow is in an active/running state
 */
export function isActiveState(state: WorkflowState): boolean {
  return ['executing', 'paused'].includes(state);
}

/**
 * Get progress percentage (0-100)
 */
export function getProgressPercent(result: WorkflowStatusResult): number {
  if (!result.totalTasks || result.totalTasks === 0) {
    return 0;
  }

  const completed = result.tasksCompleted || 0;
  return Math.round((completed / result.totalTasks) * 100);
}

/**
 * Format duration for display
 */
export function formatDuration(ms: number): string {
  if (ms < 1000) return `${ms}ms`;
  if (ms < 60000) return `${(ms / 1000).toFixed(1)}s`;
  const minutes = Math.floor(ms / 60000);
  const seconds = Math.round((ms % 60000) / 1000);
  return `${minutes}m ${seconds}s`;
}

/**
 * Get a human-readable summary of workflow status
 */
export function getWorkflowSummary(result: WorkflowStatusResult): string {
  const state = result.state.charAt(0).toUpperCase() + result.state.slice(1);

  if (result.state === 'uninitialized') {
    return 'Workflow not initialized';
  }

  const parts = [state];

  if (result.totalTasks !== undefined && result.tasksCompleted !== undefined) {
    parts.push(`${result.tasksCompleted}/${result.totalTasks} tasks`);
  }

  if (result.durationMs !== undefined) {
    parts.push(formatDuration(result.durationMs));
  }

  if (result.errorMessage) {
    parts.push(`Error: ${result.errorMessage}`);
  }

  return parts.join(' | ');
}
