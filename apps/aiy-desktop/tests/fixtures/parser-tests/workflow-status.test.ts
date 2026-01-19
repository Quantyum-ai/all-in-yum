/**
 * Workflow Status Parser Tests
 */

import { describe, it, expect } from 'vitest';
import { readFileSync } from 'fs';
import { join } from 'path';
import {
  parseWorkflowStatus,
  isTerminalState,
  isActiveState,
  getProgressPercent,
  formatDuration,
  getWorkflowSummary,
} from '../../../src/shared/parsers/workflow-status';

const fixturesPath = join(__dirname, '../cli-outputs');

function loadFixture(name: string): string {
  return readFileSync(join(fixturesPath, name), 'utf-8');
}

describe('parseWorkflowStatus', () => {
  it('should parse ready state correctly', () => {
    const output = loadFixture('workflow-status-ready.json');
    const result = parseWorkflowStatus(output);

    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.data.state).toBe('ready');
      expect(result.data.totalTasks).toBe(0);
      expect(result.data.errorMessage).toBeNull();
    }
  });

  it('should parse executing state with progress', () => {
    const output = loadFixture('workflow-status-executing.json');
    const result = parseWorkflowStatus(output);

    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.data.state).toBe('executing');
      expect(result.data.totalTasks).toBe(5);
      expect(result.data.tasksCompleted).toBe(2);
      expect(result.data.cloudRequests).toBe(1);
      expect(result.data.localExecutions).toBe(2);
    }
  });

  it('should parse paused state', () => {
    const output = loadFixture('workflow-status-paused.json');
    const result = parseWorkflowStatus(output);

    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.data.state).toBe('paused');
      expect(result.data.tasksCompleted).toBe(3);
    }
  });

  it('should parse completed state', () => {
    const output = loadFixture('workflow-status-completed.json');
    const result = parseWorkflowStatus(output);

    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.data.state).toBe('completed');
      expect(result.data.tasksCompleted).toBe(5);
      expect(result.data.totalTasks).toBe(5);
      expect(result.data.durationMs).toBe(45000);
    }
  });

  it('should parse failed state with error message', () => {
    const output = loadFixture('workflow-status-failed.json');
    const result = parseWorkflowStatus(output);

    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.data.state).toBe('failed');
      expect(result.data.errorMessage).toContain('Task 4 failed');
      expect(result.data.tasksFailed).toBe(1);
    }
  });

  it('should handle empty input gracefully', () => {
    const result = parseWorkflowStatus('');

    expect(result.success).toBe(false);
    if (!result.success) {
      expect(result.error.type).toBe('empty_input');
    }
  });

  it('should handle invalid JSON gracefully', () => {
    const result = parseWorkflowStatus('{ invalid json }');

    expect(result.success).toBe(false);
    if (!result.success) {
      expect(result.error.type).toBe('json_parse_error');
    }
  });

  it('should handle JSON with surrounding text', () => {
    const output = 'Some prefix text\n' + loadFixture('workflow-status-ready.json') + '\nSome suffix';
    const result = parseWorkflowStatus(output);

    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.data.state).toBe('ready');
    }
  });
});

describe('isTerminalState', () => {
  it('should identify completed as terminal', () => {
    expect(isTerminalState('completed')).toBe(true);
  });

  it('should identify failed as terminal', () => {
    expect(isTerminalState('failed')).toBe(true);
  });

  it('should identify cancelled as terminal', () => {
    expect(isTerminalState('cancelled')).toBe(true);
  });

  it('should not identify executing as terminal', () => {
    expect(isTerminalState('executing')).toBe(false);
  });

  it('should not identify ready as terminal', () => {
    expect(isTerminalState('ready')).toBe(false);
  });
});

describe('isActiveState', () => {
  it('should identify executing as active', () => {
    expect(isActiveState('executing')).toBe(true);
  });

  it('should identify paused as active', () => {
    expect(isActiveState('paused')).toBe(true);
  });

  it('should not identify completed as active', () => {
    expect(isActiveState('completed')).toBe(false);
  });

  it('should not identify ready as active', () => {
    expect(isActiveState('ready')).toBe(false);
  });
});

describe('getProgressPercent', () => {
  it('should calculate progress correctly', () => {
    const output = loadFixture('workflow-status-executing.json');
    const result = parseWorkflowStatus(output);

    expect(result.success).toBe(true);
    if (result.success) {
      const progress = getProgressPercent(result.data);
      expect(progress).toBe(40); // 2/5 = 40%
    }
  });

  it('should return 0 for ready state', () => {
    const output = loadFixture('workflow-status-ready.json');
    const result = parseWorkflowStatus(output);

    expect(result.success).toBe(true);
    if (result.success) {
      const progress = getProgressPercent(result.data);
      expect(progress).toBe(0);
    }
  });

  it('should return 100 for completed state', () => {
    const output = loadFixture('workflow-status-completed.json');
    const result = parseWorkflowStatus(output);

    expect(result.success).toBe(true);
    if (result.success) {
      const progress = getProgressPercent(result.data);
      expect(progress).toBe(100);
    }
  });
});

describe('formatDuration', () => {
  it('should format milliseconds', () => {
    expect(formatDuration(500)).toBe('500ms');
  });

  it('should format seconds', () => {
    expect(formatDuration(5000)).toBe('5.0s');
  });

  it('should format minutes and seconds', () => {
    expect(formatDuration(90000)).toBe('1m 30s');
  });
});

describe('getWorkflowSummary', () => {
  it('should generate summary for executing state', () => {
    const output = loadFixture('workflow-status-executing.json');
    const result = parseWorkflowStatus(output);

    expect(result.success).toBe(true);
    if (result.success) {
      const summary = getWorkflowSummary(result.data);
      expect(summary).toContain('Executing');
      expect(summary).toContain('2/5');
    }
  });

  it('should include error message for failed state', () => {
    const output = loadFixture('workflow-status-failed.json');
    const result = parseWorkflowStatus(output);

    expect(result.success).toBe(true);
    if (result.success) {
      const summary = getWorkflowSummary(result.data);
      expect(summary).toContain('Failed');
      expect(summary).toContain('Error');
    }
  });
});
