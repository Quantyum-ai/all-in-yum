/**
 * CLI Service - Spawns and manages CLI processes
 *
 * This service is responsible for:
 * - Resolving the aiy binary location
 * - Spawning CLI commands as child processes
 * - Managing process lifecycle (start, cancel, cleanup)
 * - Streaming stdout/stderr to the renderer via IPC
 * - Enforcing security constraints (shell is always disabled)
 * - Implementing cross-platform process tree kill
 * - JSONL logging with sensitive data scrubbing
 *
 * SECURITY: All process spawning MUST use shell: false
 */

	import { ChildProcess, spawn } from 'child_process';
	import { EventEmitter } from 'events';
	import * as fs from 'fs';
	import * as path from 'path';
	import * as os from 'os';
import {
  BinaryResolution,
  BinarySource,
  CommandCategory,
  COMMAND_CATEGORIES,
  DEFAULT_TIMEOUTS,
  QueuedJob,
  OutputChunk,
  JobCompleteEvent,
  CLIServiceConfig,
  DEFAULT_CLI_CONFIG,
  RunCommandResult,
  CancelCommandResult,
  ICLIService,
  KILL_STRATEGIES,
} from '../shared/cli-service-types';
import {
  CommandLogEntry,
  LOG_ROTATION_CONFIG,
  scrubSensitivePatterns,
  truncateForLog,
} from '../shared/log-schema';

// Dynamically import electron app to handle test environments
let electronApp: typeof import('electron').app | undefined;
try {
  // eslint-disable-next-line @typescript-eslint/no-require-imports
  electronApp = require('electron').app;
} catch {
  // Not in Electron context (e.g., running tests)
}

/**
 * Internal job tracking with process handle
 */
interface InternalJob extends QueuedJob {
  process?: ChildProcess;
  stdoutBuffer: string;
  stderrBuffer: string;
  outputListeners: Set<(chunk: OutputChunk) => void>;
  completeListeners: Set<(event: JobCompleteEvent) => void>;
}

/**
 * CLI Service implementation
 *
 * Handles spawning, streaming, and managing CLI processes.
 * Implements all security requirements from PRP.
 */
	export class CliService extends EventEmitter implements ICLIService {
	  private jobs: Map<string, InternalJob> = new Map();
	  private jobCounter = 0;
	  private config: CLIServiceConfig;
	  private binaryResolution?: BinaryResolution;
	  private logDir: string;
	  private pendingLogWrites: Set<Promise<void>> = new Set();
	  private initialized = false;

  constructor(config: Partial<CLIServiceConfig> = {}) {
    super();
    this.config = { ...DEFAULT_CLI_CONFIG, ...config };

    // Set up log directory
    const configDir = this.getConfigDir();
    this.logDir = path.join(configDir, 'logs');
  }

  /**
   * Get the config directory path
   */
  private getConfigDir(): string {
    // Use Electron's app.getPath if available, otherwise fallback
    try {
      if (electronApp && typeof electronApp.getPath === 'function') {
        return electronApp.getPath('userData');
      }
    } catch {
      // app not ready or not in Electron context
    }

    // Fallback for testing or non-Electron context
    const home = os.homedir();
    if (process.platform === 'win32') {
      return path.join(home, 'AppData', 'Roaming', 'aiy-desktop');
    } else if (process.platform === 'darwin') {
      return path.join(home, 'Library', 'Application Support', 'aiy-desktop');
    } else {
      return path.join(home, '.config', 'aiy-desktop');
    }
  }

  /**
   * Generate a unique job ID
   */
  private generateJobId(): string {
    this.jobCounter++;
    return `job-${Date.now()}-${this.jobCounter}`;
  }

  /**
   * Get command category for queue management
   */
  private getCommandCategory(command: string): CommandCategory {
    return COMMAND_CATEGORIES[command] ?? 'read-only';
  }

  /**
   * Get timeout for a command
   */
  private getTimeout(command: string, customTimeout?: number): number {
    if (customTimeout !== undefined) {
      return customTimeout;
    }
    return DEFAULT_TIMEOUTS[command] ?? DEFAULT_TIMEOUTS.default;
  }

  /**
   * Resolve the binary path
   */
  private async resolveBinary(): Promise<BinaryResolution> {
    // Priority 1: User-specified path (from settings or config)
    const customPath = await this.getCustomBinaryPath();
    if (customPath) {
      const exists = await this.checkBinaryExists(customPath);
      if (exists) {
        return {
          path: customPath,
          source: 'user-specified' as BinarySource,
          compatible: true,
        };
      }
    }

    // Also check config for custom path (backwards compatibility)
    if (this.config.customBinaryPath) {
      const exists = await this.checkBinaryExists(this.config.customBinaryPath);
      if (exists) {
        return {
          path: this.config.customBinaryPath,
          source: 'user-specified' as BinarySource,
          compatible: true,
        };
      }
    }

    // Priority 2: Cargo build output (workspace root)
    const cargoBinaryPath = this.getCargoBinaryPath();
    if (cargoBinaryPath) {
      const exists = await this.checkBinaryExists(cargoBinaryPath);
      if (exists) {
        return {
          path: cargoBinaryPath,
          source: 'bundled' as BinarySource, // Treat cargo build as "bundled"
          compatible: true,
        };
      }
    }

    // Priority 3: Bundled binary (in Electron resources)
    const bundledPath = this.getBundledBinaryPath();
    if (bundledPath) {
      const exists = await this.checkBinaryExists(bundledPath);
      if (exists) {
        return {
          path: bundledPath,
          source: 'bundled' as BinarySource,
          compatible: true,
        };
      }
    }

    // Priority 4: PATH lookup
    const pathBinary = await this.findInPath('aiy');
    if (pathBinary) {
      return {
        path: pathBinary,
        source: 'path-lookup' as BinarySource,
        compatible: true,
      };
    }

    // Fallback: Return fake CLI path for testing (Milestone 1)
    const fakeCLIPath = this.getFakeCLIPath();
    if (fs.existsSync(fakeCLIPath)) {
      return {
        path: fakeCLIPath,
        source: 'path-lookup' as BinarySource,
        compatible: true,
        versionWarning: 'Using fake CLI for testing',
      };
    }

    throw new Error('No aiy binary found. Please build with cargo or set a custom path.');
  }

  /**
   * Get the path to the fake CLI for testing
   */
  private getFakeCLIPath(): string {
    // In development/testing, use the fake CLI fixture
    // This path works when running from the project root
    const possiblePaths = [
      path.join(process.cwd(), 'tests', 'fixtures', 'fake-cli.js'),
      path.join(__dirname, '..', '..', 'tests', 'fixtures', 'fake-cli.js'),
      path.join(__dirname, '..', '..', '..', 'tests', 'fixtures', 'fake-cli.js'),
    ];

    for (const p of possiblePaths) {
      if (fs.existsSync(p)) {
        return p;
      }
    }

    // Default fallback
    return possiblePaths[0];
  }

  /**
   * Get bundled binary path
   */
  private getBundledBinaryPath(): string | null {
    try {
      if (electronApp && typeof electronApp.getPath === 'function') {
        const resourcesPath = path.join(electronApp.getPath('exe'), '..', 'resources');
        const binaryName = process.platform === 'win32' ? 'aiy.exe' : 'aiy';
        return path.join(resourcesPath, 'bin', binaryName);
      }
    } catch {
      // Not in Electron context
    }
    return null;
  }

  /**
   * Get cargo build output path (workspace root)
   * This is used during development when the binary is built with cargo
   */
  private getCargoBinaryPath(): string | null {
    // Try to find workspace root by looking for Cargo.toml
    const possibleRoots = [
      // From aiy-desktop directory, workspace root is 2 levels up
      path.resolve(__dirname, '..', '..', '..', '..'),
      // From process.cwd() if running from workspace root
      process.cwd(),
      // Common workspace root for monorepo
      path.resolve(process.cwd(), '..', '..'),
    ];

    const binaryName = process.platform === 'win32' ? 'aiy.exe' : 'aiy';

    for (const root of possibleRoots) {
      const cargoPath = path.join(root, 'Cargo.toml');
      const debugBinary = path.join(root, 'target', 'debug', binaryName);
      const releaseBinary = path.join(root, 'target', 'release', binaryName);

      // Check if this looks like the workspace root
      if (fs.existsSync(cargoPath)) {
        // Prefer release build if available, otherwise debug
        if (fs.existsSync(releaseBinary)) {
          return releaseBinary;
        }
        if (fs.existsSync(debugBinary)) {
          return debugBinary;
        }
      }
    }

    return null;
  }

  /**
   * Get custom binary path from settings store
   */
  private async getCustomBinaryPath(): Promise<string | undefined> {
    try {
      // Try to use electron-store to get user settings
      // This is async to handle cases where store isn't ready
      const Store = (await import('electron-store')).default;
      const store = new Store();
      const customPath = store.get('cliBinaryPath') as string | undefined;
      return customPath;
    } catch {
      // Not in Electron context or store not available
      return undefined;
    }
  }

  /**
   * Check if a binary exists
   */
  private async checkBinaryExists(binaryPath: string): Promise<boolean> {
    return new Promise((resolve) => {
      fs.access(binaryPath, fs.constants.X_OK, (err) => {
        resolve(!err);
      });
    });
  }

  /**
   * Find a binary in PATH
   */
  private async findInPath(name: string): Promise<string | null> {
    const pathEnv = process.env.PATH || '';
    const pathSeparator = process.platform === 'win32' ? ';' : ':';
    const paths = pathEnv.split(pathSeparator);
    const extensions = process.platform === 'win32' ? ['.exe', '.cmd', '.bat', ''] : [''];

    for (const dir of paths) {
      for (const ext of extensions) {
        const fullPath = path.join(dir, name + ext);
        if (await this.checkBinaryExists(fullPath)) {
          return fullPath;
        }
      }
    }
    return null;
  }

  /**
   * Initialize the CLI service
   * @param force If true, reinitialize even if already initialized (e.g., when settings change)
   */
  async initialize(force = false): Promise<BinaryResolution> {
    if (!force && this.initialized && this.binaryResolution) {
      return this.binaryResolution;
    }

    // Reset state if forcing reinitialization
    if (force) {
      this.initialized = false;
      this.binaryResolution = undefined;
    }

    // Ensure log directory exists
    await this.ensureLogDirectory();

    // Resolve binary
    this.binaryResolution = await this.resolveBinary();

    // Get version if binary exists
    if (this.binaryResolution.path) {
      try {
        const version = await this.getBinaryVersion(this.binaryResolution.path);
        if (version) {
          this.binaryResolution.version = version;
        }
      } catch {
        // Version check failed, continue anyway
      }
    }

    this.initialized = true;
    return this.binaryResolution;
  }

  /**
   * Get binary version
   */
  private async getBinaryVersion(binaryPath: string): Promise<string | null> {
    return new Promise((resolve) => {
      // For fake CLI, use node to run it
      const isNodeScript = binaryPath.endsWith('.js');
      const cmd = isNodeScript ? process.execPath : binaryPath;
      const args = isNodeScript ? [binaryPath, 'version'] : ['version'];

      const child = spawn(cmd, args, {
        shell: false,
        stdio: ['pipe', 'pipe', 'pipe'],
        timeout: 5000,
      });

      let output = '';
      child.stdout?.on('data', (data) => {
        output += data.toString();
      });

      child.on('close', (code) => {
        if (code === 0 && output) {
          // Extract version from output (e.g., "aiy 0.1.0" -> "0.1.0")
          const match = output.match(/\d+\.\d+\.\d+/);
          resolve(match ? match[0] : output.trim());
        } else {
          resolve(null);
        }
      });

      child.on('error', () => {
        resolve(null);
      });
    });
  }

  /**
   * Ensure log directory exists
   */
  private async ensureLogDirectory(): Promise<void> {
    return new Promise((resolve, reject) => {
      fs.mkdir(this.logDir, { recursive: true }, (err) => {
        if (err && err.code !== 'EEXIST') {
          reject(err);
        } else {
          resolve();
        }
      });
    });
  }

  /**
   * Run a CLI command
   */
  async run(
    command: string,
    args: string[],
    options: {
      repoPath?: string;
      windowId: string;
      timeoutMs?: number;
    }
  ): Promise<RunCommandResult> {
    // Ensure initialized
    if (!this.binaryResolution) {
      await this.initialize();
    }

    const jobId = this.generateJobId();
    const category = this.getCommandCategory(command);
    const timeoutMs = this.getTimeout(command, options.timeoutMs);

    // Create job entry
    const job: InternalJob = {
      id: jobId,
      command,
      args,
      category,
      repoPath: options.repoPath,
      windowId: options.windowId,
      status: 'running',
      queuedAt: new Date(),
      startedAt: new Date(),
      timeoutMs,
      stdoutBuffer: '',
      stderrBuffer: '',
      outputListeners: new Set(),
      completeListeners: new Set(),
    };

    this.jobs.set(jobId, job);

    // Spawn the process
    this.spawnProcess(job);

    return {
      jobId,
      queued: false,
    };
  }

  /**
   * Spawn a process for a job
   */
	  private spawnProcess(job: InternalJob): void {
    const binaryPath = this.binaryResolution!.path;

    // Determine how to spawn: if it's a .js file, run with node
    const isNodeScript = binaryPath.endsWith('.js');
    const cmd = isNodeScript ? process.execPath : binaryPath;

    // Build arguments: for node scripts, include the script path
    let spawnArgs: string[];
    if (isNodeScript) {
      spawnArgs = [binaryPath, job.command, ...job.args];
    } else {
      spawnArgs = [job.command, ...job.args];
    }

    // SECURITY: Shell is always disabled to prevent command injection
    const child = spawn(cmd, spawnArgs, {
      shell: false,
      stdio: ['pipe', 'pipe', 'pipe'],
      env: { ...process.env },
      cwd: job.repoPath || process.cwd(),
    });

    job.process = child;
    job.pid = child.pid;

    // Set up timeout
    if (job.timeoutMs > 0) {
      job.timeoutHandle = setTimeout(() => {
        this.handleTimeout(job.id);
      }, job.timeoutMs);
    }

    // Handle stdout
    child.stdout?.on('data', (data: Buffer) => {
      const text = data.toString();
      job.stdoutBuffer += text;

      const chunk: OutputChunk = {
        jobId: job.id,
        stream: 'stdout',
        text,
        timestamp: new Date().toISOString(),
      };

      // Emit to global listeners
      this.emit('output', job.id, chunk);

      // Emit to job-specific listeners
      for (const listener of job.outputListeners) {
        listener(chunk);
      }
    });

	    // Handle stderr
	    child.stderr?.on('data', (data: Buffer) => {
      const text = data.toString();
      job.stderrBuffer += text;

      const chunk: OutputChunk = {
        jobId: job.id,
        stream: 'stderr',
        text,
        timestamp: new Date().toISOString(),
      };

      // Emit to global listeners
      this.emit('output', job.id, chunk);

      // Emit to job-specific listeners
      for (const listener of job.outputListeners) {
        listener(chunk);
      }
	    });

	    // Handle exit (wait for stdio to fully drain)
	    child.on('close', (code, signal) => {
	      this.handleProcessExit(job.id, code, signal);
	    });

    // Handle error
    child.on('error', (err: Error) => {
      this.handleProcessError(job.id, err);
    });
  }

  /**
   * Handle process timeout
   */
  private handleTimeout(jobId: string): void {
    const job = this.jobs.get(jobId);
    if (!job || job.status !== 'running') {
      return;
    }

    job.error = 'Process timed out';
    this.killProcess(job);
  }

  /**
   * Handle process exit
   */
	  private handleProcessExit(jobId: string, code: number | null, signal: string | null): void {
	    const job = this.jobs.get(jobId);
	    if (!job) {
	      return;
	    }

    // Clear timeout
    if (job.timeoutHandle) {
      clearTimeout(job.timeoutHandle);
      job.timeoutHandle = undefined;
    }

    // Update job status
    job.completedAt = new Date();
    job.exitCode = code ?? undefined;

    if (job.status === 'cancelled') {
      // Already marked as cancelled
    } else if (code === 0) {
      job.status = 'completed';
    } else {
      job.status = 'failed';
      // Only set signal error if there isn't already an error (e.g., from timeout)
      if (signal && !job.error) {
        job.error = `Process killed by signal: ${signal}`;
      }
    }

    const durationMs = job.completedAt.getTime() - (job.startedAt?.getTime() ?? job.queuedAt.getTime());

    // Create completion event
	    const event: JobCompleteEvent = {
	      jobId: job.id,
	      exitCode: code,
	      cancelled: job.status === 'cancelled',
	      error: job.error,
	      durationMs,
	    };

	    // Log to JSONL
	    const logPromise = this.logCommand(job, durationMs);
	    this.pendingLogWrites.add(logPromise);
	    logPromise.finally(() => {
	      this.pendingLogWrites.delete(logPromise);
	    });

    // Emit to global listeners
    this.emit('exit', jobId, event);

    // Emit to job-specific listeners
    for (const listener of job.completeListeners) {
      listener(event);
    }

    // Clean up listeners but keep job info for status queries
    job.outputListeners.clear();
    job.completeListeners.clear();
    job.process = undefined;
  }

  /**
   * Handle process error
   */
  private handleProcessError(jobId: string, err: Error): void {
    const job = this.jobs.get(jobId);
    if (!job) {
      return;
    }

    job.error = err.message;
    job.status = 'failed';

    // Emit error as stderr
    const chunk: OutputChunk = {
      jobId: job.id,
      stream: 'stderr',
      text: `Process error: ${err.message}\n`,
      timestamp: new Date().toISOString(),
    };

    this.emit('output', jobId, chunk);
    for (const listener of job.outputListeners) {
      listener(chunk);
    }

    // Trigger exit handling
    this.handleProcessExit(jobId, 1, null);
  }

  /**
   * Kill a process using platform-specific strategy
   */
	  private killProcess(job: InternalJob): void {
	    if (!job.process || !job.pid) {
	      return;
	    }

    const pid = job.pid;

	    if (process.platform === 'win32') {
	      // Windows: Use taskkill for tree kill
	      try {
	        const killArgs = KILL_STRATEGIES.win32.args(pid);
	        const killer = spawn(KILL_STRATEGIES.win32.command, killArgs, {
	          shell: false,
	          stdio: 'ignore',
	          windowsHide: true,
	        });
	        killer.on('error', () => {
	          // Process may already be dead, or taskkill may be unavailable
	        });
	        killer.unref();
	      } catch {
	        // Process may already be dead
	      }
	    } else {
      // Unix: Use SIGTERM first, then SIGKILL after timeout
      try {
        // Kill process group if possible
        process.kill(-pid, KILL_STRATEGIES.unix.signal);
      } catch {
        // Try killing just the process
        try {
          job.process.kill(KILL_STRATEGIES.unix.signal);
        } catch {
          // Process may already be dead
        }
      }

      // Schedule SIGKILL if process doesn't exit
      setTimeout(() => {
        if (job.process && !job.process.killed) {
          try {
            process.kill(-pid, KILL_STRATEGIES.unix.forceSignal);
          } catch {
            try {
              job.process?.kill(KILL_STRATEGIES.unix.forceSignal);
            } catch {
              // Process is already dead
            }
          }
        }
      }, KILL_STRATEGIES.unix.forceTimeout);
    }
  }

  /**
   * Cancel a running or queued command
   */
  async cancel(jobId: string): Promise<CancelCommandResult> {
    const job = this.jobs.get(jobId);
    if (!job) {
      return {
        success: false,
        error: 'Job not found',
        wasRunning: false,
      };
    }

    const wasRunning = job.status === 'running';

    // Clear timeout
    if (job.timeoutHandle) {
      clearTimeout(job.timeoutHandle);
      job.timeoutHandle = undefined;
    }

    // Mark as cancelled
    job.status = 'cancelled';

    // Kill the process if running
    if (wasRunning && job.process) {
      this.killProcess(job);
    }

    return {
      success: true,
      wasRunning,
    };
  }

  /**
   * Get status of a job
   */
  getJobStatus(jobId: string): QueuedJob | undefined {
    const job = this.jobs.get(jobId);
    if (!job) {
      return undefined;
    }

    // Return a copy without internal fields
    const { process: _p, stdoutBuffer: _sb, stderrBuffer: _stb, outputListeners: _ol, completeListeners: _cl, timeoutHandle: _th, ...publicJob } = job;
    return publicJob;
  }

  /**
   * Get all jobs for a window
   */
  getJobsForWindow(windowId: string): QueuedJob[] {
    const jobs: QueuedJob[] = [];
    for (const job of this.jobs.values()) {
      if (job.windowId === windowId) {
        const { process: _p, stdoutBuffer: _sb, stderrBuffer: _stb, outputListeners: _ol, completeListeners: _cl, timeoutHandle: _th, ...publicJob } = job;
        jobs.push(publicJob);
      }
    }
    return jobs;
  }

  /**
   * Subscribe to output chunks for a job
   */
  onOutput(jobId: string, callback: (chunk: OutputChunk) => void): () => void {
    const job = this.jobs.get(jobId);
    if (!job) {
      return () => {};
    }

    job.outputListeners.add(callback);

    return () => {
      job.outputListeners.delete(callback);
    };
  }

  /**
   * Subscribe to job completion events
   */
  onComplete(jobId: string, callback: (event: JobCompleteEvent) => void): () => void {
    const job = this.jobs.get(jobId);
    if (!job) {
      return () => {};
    }

    // If already completed, call immediately
    if (job.status === 'completed' || job.status === 'failed' || job.status === 'cancelled') {
      const durationMs = (job.completedAt?.getTime() ?? Date.now()) - (job.startedAt?.getTime() ?? job.queuedAt.getTime());
      callback({
        jobId: job.id,
        exitCode: job.exitCode ?? null,
        cancelled: job.status === 'cancelled',
        error: job.error,
        durationMs,
      });
      return () => {};
    }

    job.completeListeners.add(callback);

    return () => {
      job.completeListeners.delete(callback);
    };
  }

  /**
   * Clean up all jobs for a window
   */
  async cleanupWindow(windowId: string): Promise<void> {
    const windowJobs = Array.from(this.jobs.entries()).filter(
      ([, job]) => job.windowId === windowId
    );

    for (const [jobId, job] of windowJobs) {
      if (job.status === 'running' || job.status === 'queued') {
        await this.cancel(jobId);
      }
      this.jobs.delete(jobId);
    }
  }

  /**
   * Clean up all running processes (for app exit)
   */
	  async shutdown(): Promise<void> {
	    const runningJobs = Array.from(this.jobs.entries()).filter(
	      ([, job]) => job.status === 'running'
	    );

	    // Cancel all running jobs
	    await Promise.all(runningJobs.map(([jobId]) => this.cancel(jobId)));

	    // Flush any pending log writes before clearing jobs (helps tests & clean shutdown)
	    await Promise.allSettled(Array.from(this.pendingLogWrites));
	    this.pendingLogWrites.clear();

	    // Clear all jobs
	    this.jobs.clear();

    // Reset state
    this.initialized = false;
    this.binaryResolution = undefined;
  }

  /**
   * Get current binary resolution info
   */
  getBinaryInfo(): BinaryResolution | undefined {
    return this.binaryResolution;
  }

  /**
   * Legacy cleanup method for backwards compatibility
   * @deprecated Use shutdown() instead
   */
  cleanup(): void {
    this.shutdown().catch((err) => {
      console.error('[CliService] Error during cleanup:', err);
    });
  }

  /**
   * Log a command to JSONL
   */
  private async logCommand(job: InternalJob, durationMs: number): Promise<void> {
    try {
      // Create log entry
      const stdoutResult = truncateForLog(scrubSensitivePatterns(job.stdoutBuffer));
      const stderrResult = truncateForLog(scrubSensitivePatterns(job.stderrBuffer));

      const entry: CommandLogEntry = {
        jobId: job.id,
        startedAt: job.startedAt?.toISOString() ?? job.queuedAt.toISOString(),
        completedAt: job.completedAt?.toISOString() ?? new Date().toISOString(),
        durationMs,
        command: job.command,
        args: this.scrubArgs(job.args),
        exitCode: job.exitCode ?? null,
        stdoutSummary: stdoutResult.text,
        stderrSummary: stderrResult.text,
        truncated: stdoutResult.truncated || stderrResult.truncated,
        context: {
          repoPath: job.repoPath,
          windowId: job.windowId,
        },
        cancelled: job.status === 'cancelled',
        error: job.error ? scrubSensitivePatterns(job.error) : undefined,
      };

      // Write to log file
      await this.writeLogEntry(entry);
    } catch (err) {
      // Log writing should never crash the app
      console.error('[CliService] Failed to write log entry:', err);
    }
  }

  /**
   * Scrub sensitive arguments
   */
  private scrubArgs(args: string[]): string[] {
    return args.map((arg) => {
      // Don't log values for sensitive arguments
      const lowerArg = arg.toLowerCase();
      if (
        lowerArg.includes('password') ||
        lowerArg.includes('secret') ||
        lowerArg.includes('token') ||
        lowerArg.includes('key') ||
        lowerArg.includes('credential')
      ) {
        return '[REDACTED]';
      }
      return scrubSensitivePatterns(arg);
    });
  }

  /**
   * Write a log entry to the JSONL file
   */
  private async writeLogEntry(entry: CommandLogEntry): Promise<void> {
    const logFile = path.join(this.logDir, LOG_ROTATION_CONFIG.filePattern);

    // Check if rotation is needed
    await this.rotateLogsIfNeeded(logFile);

    // Append entry
    const line = JSON.stringify(entry) + '\n';
    return new Promise((resolve, reject) => {
      fs.appendFile(logFile, line, { encoding: 'utf8' }, (err) => {
        if (err) {
          reject(err);
        } else {
          resolve();
        }
      });
    });
  }

  /**
   * Rotate logs if needed
   */
  private async rotateLogsIfNeeded(logFile: string): Promise<void> {
    return new Promise((resolve) => {
      fs.stat(logFile, (err, stats) => {
        if (err || stats.size < LOG_ROTATION_CONFIG.maxFileSize) {
          resolve();
          return;
        }

        // Need to rotate
        this.rotateLogs(logFile).then(resolve).catch(() => resolve());
      });
    });
  }

  /**
   * Rotate log files
   */
  private async rotateLogs(logFile: string): Promise<void> {
    // Move existing rotated files
    for (let i = LOG_ROTATION_CONFIG.maxFiles - 1; i >= 1; i--) {
      const oldFile = path.join(
        this.logDir,
        LOG_ROTATION_CONFIG.rotatedPattern.replace('{n}', String(i))
      );
      const newFile = path.join(
        this.logDir,
        LOG_ROTATION_CONFIG.rotatedPattern.replace('{n}', String(i + 1))
      );

      try {
        await this.moveFile(oldFile, newFile);
      } catch {
        // File may not exist
      }
    }

    // Move current log to .1
    const rotatedFile = path.join(
      this.logDir,
      LOG_ROTATION_CONFIG.rotatedPattern.replace('{n}', '1')
    );

    try {
      await this.moveFile(logFile, rotatedFile);
    } catch {
      // Log file may not exist
    }

    // Delete oldest if we have too many
    const oldestFile = path.join(
      this.logDir,
      LOG_ROTATION_CONFIG.rotatedPattern.replace('{n}', String(LOG_ROTATION_CONFIG.maxFiles + 1))
    );
    try {
      await this.deleteFile(oldestFile);
    } catch {
      // File may not exist
    }
  }

  /**
   * Move a file
   */
  private moveFile(src: string, dest: string): Promise<void> {
    return new Promise((resolve, reject) => {
      fs.rename(src, dest, (err) => {
        if (err) {
          reject(err);
        } else {
          resolve();
        }
      });
    });
  }

  /**
   * Delete a file
   */
  private deleteFile(filePath: string): Promise<void> {
    return new Promise((resolve, reject) => {
      fs.unlink(filePath, (err) => {
        if (err) {
          reject(err);
        } else {
          resolve();
        }
      });
    });
  }

  /**
   * For testing: Set the binary resolution directly
   */
  _setTestBinaryPath(binaryPath: string): void {
    this.binaryResolution = {
      path: binaryPath,
      source: 'user-specified',
      compatible: true,
    };
    this.initialized = true;
  }

  /**
   * For testing: Set the log directory
   */
  _setTestLogDir(logDir: string): void {
    this.logDir = logDir;
  }
}

// Singleton instance
export const cliService = new CliService();
