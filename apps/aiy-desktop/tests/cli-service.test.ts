/**
 * CLI Service Tests
 *
 * Tests for the CLI service that spawns and manages CLI processes.
 * Uses the fake CLI fixture for testing without requiring the real Rust binary.
 */

import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import * as path from 'path';
import * as fs from 'fs';
import * as os from 'os';
import { CliService } from '../src/main/cli-service';
import { OutputChunk, JobCompleteEvent } from '../src/shared/cli-service-types';

// Path to the fake CLI fixture
const FAKE_CLI_PATH = path.join(__dirname, 'fixtures', 'fake-cli.js');

// Temporary directory for test logs
let testLogDir: string;

describe('CliService', () => {
  let cliService: CliService;

  beforeEach(() => {
    // Create a fresh instance for each test
    cliService = new CliService();

    // Set up test log directory
    testLogDir = path.join(os.tmpdir(), `aiy-test-logs-${Date.now()}`);
    fs.mkdirSync(testLogDir, { recursive: true });
    cliService._setTestLogDir(testLogDir);

    // Set the fake CLI path
    cliService._setTestBinaryPath(FAKE_CLI_PATH);
  });

  afterEach(async () => {
    // Clean up
    await cliService.shutdown();

    // Clean up test log directory
    try {
      const files = fs.readdirSync(testLogDir);
      for (const file of files) {
        fs.unlinkSync(path.join(testLogDir, file));
      }
      fs.rmdirSync(testLogDir);
    } catch {
      // Ignore cleanup errors
    }
  });

  describe('initialize', () => {
    it('should initialize successfully with fake CLI', async () => {
      const resolution = await cliService.initialize();

      expect(resolution).toBeDefined();
      expect(resolution.path).toBe(FAKE_CLI_PATH);
      expect(resolution.compatible).toBe(true);
    });

    it('should return the same resolution on subsequent calls', async () => {
      const resolution1 = await cliService.initialize();
      const resolution2 = await cliService.initialize();

      expect(resolution1).toEqual(resolution2);
    });
  });

  describe('run', () => {
    it('should spawn a process and return a job ID', async () => {
      const result = await cliService.run('version', [], {
        windowId: 'test-window',
      });

      expect(result.jobId).toBeDefined();
      expect(result.jobId).toMatch(/^job-\d+-\d+$/);
      expect(result.queued).toBe(false);
    });

    it('should stream stdout output', async () => {
      const outputs: OutputChunk[] = [];

      // Subscribe to global output events BEFORE running the command
      // This ensures we capture output even if it arrives before run() returns
      const outputHandler = (_jobId: string, chunk: OutputChunk) => {
        outputs.push(chunk);
      };
      cliService.on('output', outputHandler);

      const result = await cliService.run('version', [], {
        windowId: 'test-window',
      });

      // Wait for completion
      await new Promise<void>((resolve) => {
        cliService.onComplete(result.jobId, () => {
          resolve();
        });
      });

      cliService.off('output', outputHandler);

      // Filter to only outputs from this job
      const jobOutputs = outputs.filter((o) => o.jobId === result.jobId);

      expect(jobOutputs.length).toBeGreaterThan(0);
      expect(jobOutputs.some((o) => o.text.includes('aiy 0.1.0'))).toBe(true);
    });

    it('should stream both stdout and stderr', async () => {
      const outputs: OutputChunk[] = [];

      // Subscribe to global output events BEFORE running the command
      const outputHandler = (_jobId: string, chunk: OutputChunk) => {
        outputs.push(chunk);
      };
      cliService.on('output', outputHandler);

      const result = await cliService.run('--stderr', [], {
        windowId: 'test-window',
      });

      // Wait for completion
      await new Promise<void>((resolve) => {
        cliService.onComplete(result.jobId, () => {
          resolve();
        });
      });

      cliService.off('output', outputHandler);

      // Filter to only outputs from this job
      const jobOutputs = outputs.filter((o) => o.jobId === result.jobId);
      const stdoutChunks = jobOutputs.filter((o) => o.stream === 'stdout');
      const stderrChunks = jobOutputs.filter((o) => o.stream === 'stderr');

      expect(stdoutChunks.some((o) => o.text.includes('stdout message'))).toBe(true);
      expect(stderrChunks.some((o) => o.text.includes('stderr message'))).toBe(true);
    });

    it('should emit completion event with exit code', async () => {
      const result = await cliService.run('version', [], {
        windowId: 'test-window',
      });

      const event = await new Promise<JobCompleteEvent>((resolve) => {
        cliService.onComplete(result.jobId, resolve);
      });

      expect(event.jobId).toBe(result.jobId);
      expect(event.exitCode).toBe(0);
      expect(event.cancelled).toBe(false);
      expect(event.durationMs).toBeGreaterThanOrEqual(0);
    });

    it('should handle privacy check command', async () => {
      const outputs: string[] = [];

      // Subscribe to global output events BEFORE running the command
      const outputHandler = (jobId: string, chunk: OutputChunk) => {
        outputs.push(chunk.text);
      };
      cliService.on('output', outputHandler);

      const result = await cliService.run('privacy', ['check'], {
        windowId: 'test-window',
      });

      const event = await new Promise<JobCompleteEvent>((resolve) => {
        cliService.onComplete(result.jobId, resolve);
      });

      cliService.off('output', outputHandler);

      expect(event.exitCode).toBe(0);
      expect(outputs.join('')).toContain('[PASS]');
    });

    it('should handle errors with non-zero exit code', async () => {
      const result = await cliService.run('--error', [], {
        windowId: 'test-window',
      });

      const event = await new Promise<JobCompleteEvent>((resolve) => {
        cliService.onComplete(result.jobId, resolve);
      });

      expect(event.exitCode).toBe(1);
    });

    it('should track job status', async () => {
      const result = await cliService.run('version', [], {
        windowId: 'test-window',
      });

      // Check initial status
      const initialStatus = cliService.getJobStatus(result.jobId);
      expect(initialStatus).toBeDefined();
      expect(initialStatus!.id).toBe(result.jobId);
      expect(initialStatus!.command).toBe('version');

      // Wait for completion
      await new Promise<void>((resolve) => {
        cliService.onComplete(result.jobId, () => resolve());
      });

      // Check final status
      const finalStatus = cliService.getJobStatus(result.jobId);
      expect(finalStatus).toBeDefined();
      expect(finalStatus!.status).toBe('completed');
    });
  });

  describe('cancel', () => {
    it('should cancel a running process', async () => {
      // Start a hanging process
      const result = await cliService.run('--hang', [], {
        windowId: 'test-window',
      });

      // Give it a moment to start
      await new Promise((resolve) => setTimeout(resolve, 100));

      // Cancel it
      const cancelResult = await cliService.cancel(result.jobId);

      expect(cancelResult.success).toBe(true);
      expect(cancelResult.wasRunning).toBe(true);

      // Wait for the exit event
      const event = await new Promise<JobCompleteEvent>((resolve) => {
        cliService.onComplete(result.jobId, resolve);
      });

      expect(event.cancelled).toBe(true);
    });

    it('should return error for non-existent job', async () => {
      const cancelResult = await cliService.cancel('non-existent-job');

      expect(cancelResult.success).toBe(false);
      expect(cancelResult.error).toBe('Job not found');
    });

    it('should mark job status as cancelled', async () => {
      const result = await cliService.run('--hang', [], {
        windowId: 'test-window',
      });

      await new Promise((resolve) => setTimeout(resolve, 100));
      await cliService.cancel(result.jobId);

      // Wait a bit for the status to update
      await new Promise((resolve) => setTimeout(resolve, 100));

      const status = cliService.getJobStatus(result.jobId);
      expect(status?.status).toBe('cancelled');
    });
  });

  describe('getJobsForWindow', () => {
    it('should return all jobs for a window', async () => {
      const windowId = 'test-window-1';

      // Start multiple jobs
      await cliService.run('version', [], { windowId });
      await cliService.run('privacy', ['check'], { windowId });

      const jobs = cliService.getJobsForWindow(windowId);

      expect(jobs.length).toBe(2);
      expect(jobs.every((j) => j.windowId === windowId)).toBe(true);
    });

    it('should not include jobs from other windows', async () => {
      await cliService.run('version', [], { windowId: 'window-1' });
      await cliService.run('version', [], { windowId: 'window-2' });

      const jobs = cliService.getJobsForWindow('window-1');

      expect(jobs.length).toBe(1);
      expect(jobs[0].windowId).toBe('window-1');
    });
  });

  describe('cleanupWindow', () => {
    it('should cancel and remove all jobs for a window', async () => {
      const windowId = 'test-cleanup-window';

      // Start a hanging process
      await cliService.run('--hang', [], { windowId });

      // Wait a moment
      await new Promise((resolve) => setTimeout(resolve, 100));

      // Clean up the window
      await cliService.cleanupWindow(windowId);

      const jobs = cliService.getJobsForWindow(windowId);
      expect(jobs.length).toBe(0);
    });
  });

  describe('shutdown', () => {
    it('should cancel all running jobs', async () => {
      // Start multiple hanging processes
      await cliService.run('--hang', [], { windowId: 'window-1' });
      await cliService.run('--hang', [], { windowId: 'window-2' });

      await new Promise((resolve) => setTimeout(resolve, 100));

      // Shutdown
      await cliService.shutdown();

      // All jobs should be cleared
      const jobs1 = cliService.getJobsForWindow('window-1');
      const jobs2 = cliService.getJobsForWindow('window-2');

      expect(jobs1.length).toBe(0);
      expect(jobs2.length).toBe(0);
    });

    it('should reset initialized state', async () => {
      await cliService.initialize();
      expect(cliService.getBinaryInfo()).toBeDefined();

      await cliService.shutdown();
      expect(cliService.getBinaryInfo()).toBeUndefined();
    });
  });

  describe('timeout', () => {
    it('should timeout long-running processes', async () => {
      const result = await cliService.run('--hang', [], {
        windowId: 'test-window',
        timeoutMs: 500, // Short timeout for testing
      });

      const event = await new Promise<JobCompleteEvent>((resolve) => {
        cliService.onComplete(result.jobId, resolve);
      });

      expect(event.error).toContain('timed out');
    }, 10000);
  });

  describe('logging', () => {
    it('should write log entries to JSONL file', async () => {
      const result = await cliService.run('version', [], {
        windowId: 'test-window',
      });

      // Wait for completion
      await new Promise<void>((resolve) => {
        cliService.onComplete(result.jobId, () => resolve());
      });

      // Wait a bit for log to be written
      await new Promise((resolve) => setTimeout(resolve, 100));

      // Check log file exists and has content
      const logFile = path.join(testLogDir, 'commands.jsonl');
      const logExists = fs.existsSync(logFile);
      expect(logExists).toBe(true);

      if (logExists) {
        const logContent = fs.readFileSync(logFile, 'utf8');
        const lines = logContent.trim().split('\n');
        expect(lines.length).toBeGreaterThan(0);

        const entry = JSON.parse(lines[0]);
        expect(entry.jobId).toBe(result.jobId);
        expect(entry.command).toBe('version');
        expect(entry.exitCode).toBe(0);
      }
    });

    it('should scrub sensitive data from logs', async () => {
      // Run a command that might include sensitive output
      const result = await cliService.run('config', ['show'], {
        windowId: 'test-window',
      });

      await new Promise<void>((resolve) => {
        cliService.onComplete(result.jobId, () => resolve());
      });

      await new Promise((resolve) => setTimeout(resolve, 100));

      const logFile = path.join(testLogDir, 'commands.jsonl');
      if (fs.existsSync(logFile)) {
        const logContent = fs.readFileSync(logFile, 'utf8');

        // The log should not contain any API keys or secrets
        expect(logContent).not.toMatch(/sk-ant-/);
        expect(logContent).not.toMatch(/sk-[A-Za-z0-9]{48}/);
        expect(logContent).not.toMatch(/AKIA[0-9A-Z]{16}/);
      }
    });
  });

  describe('EventEmitter interface', () => {
    it('should emit output events globally', async () => {
      const globalOutputs: OutputChunk[] = [];

      const outputHandler = (_jobId: string, chunk: OutputChunk) => {
        globalOutputs.push(chunk);
      };
      cliService.on('output', outputHandler);

      const result = await cliService.run('version', [], {
        windowId: 'test-window',
      });

      await new Promise<void>((resolve) => {
        cliService.onComplete(result.jobId, () => resolve());
      });

      // Small delay to ensure all output events have been delivered
      await new Promise((resolve) => setTimeout(resolve, 50));

      cliService.off('output', outputHandler);

      // Filter to outputs from this job
      const jobOutputs = globalOutputs.filter((o) => o.jobId === result.jobId);
      expect(jobOutputs.length).toBeGreaterThan(0);
    });

    it('should emit exit events globally', async () => {
      let exitEvent: JobCompleteEvent | undefined;

      cliService.on('exit', (jobId: string, event: JobCompleteEvent) => {
        exitEvent = event;
      });

      const result = await cliService.run('version', [], {
        windowId: 'test-window',
      });

      await new Promise<void>((resolve) => {
        cliService.onComplete(result.jobId, () => resolve());
      });

      expect(exitEvent).toBeDefined();
      expect(exitEvent!.jobId).toBe(result.jobId);
    });
  });

  describe('streaming output', () => {
    it('should stream output incrementally', async () => {
      const outputs: OutputChunk[] = [];
      const timestamps: number[] = [];

      // Subscribe to global output events BEFORE running the command
      const outputHandler = (_jobId: string, chunk: OutputChunk) => {
        outputs.push(chunk);
        timestamps.push(Date.now());
      };
      cliService.on('output', outputHandler);

      const result = await cliService.run('--stream', [], {
        windowId: 'test-window',
      });

      await new Promise<void>((resolve) => {
        cliService.onComplete(result.jobId, () => resolve());
      });

      cliService.off('output', outputHandler);

      // Filter to only outputs from this job
      const jobOutputs = outputs.filter((o) => o.jobId === result.jobId);
      const jobTimestamps = timestamps.slice(0, jobOutputs.length);

      // Should have multiple output chunks
      expect(jobOutputs.length).toBeGreaterThan(1);

      // Chunks should arrive over time (not all at once)
      if (jobTimestamps.length >= 2) {
        const timeDiff = jobTimestamps[jobTimestamps.length - 1] - jobTimestamps[0];
        expect(timeDiff).toBeGreaterThan(100); // At least 100ms between first and last
      }
    });
  });
});

describe('CliService Security', () => {
  it('should never use shell: true', async () => {
    // This is a compile-time guarantee enforced by the implementation
    // The spawn call always uses { shell: false }
    // We verify this by checking the source code contains the correct pattern
    const serviceSource = fs.readFileSync(
      path.join(__dirname, '..', 'src', 'main', 'cli-service.ts'),
      'utf8'
    );

    // Check that shell: false is used in spawn
    expect(serviceSource).toContain('shell: false');

    // Check that shell: true is not used anywhere
    expect(serviceSource).not.toMatch(/shell:\s*true/);
  });

  it('should track PIDs for cleanup', async () => {
    const cliService = new CliService();
    cliService._setTestBinaryPath(FAKE_CLI_PATH);

    const result = await cliService.run('--hang', [], {
      windowId: 'test-window',
    });

    await new Promise((resolve) => setTimeout(resolve, 100));

    const status = cliService.getJobStatus(result.jobId);
    expect(status?.pid).toBeDefined();
    expect(typeof status?.pid).toBe('number');

    await cliService.shutdown();
  });
});
