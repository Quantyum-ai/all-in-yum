/**
 * JSONL Log Schema for aiy Desktop
 *
 * CRITICAL REQUIREMENTS (from PRP):
 * - NEVER log credentials, secrets, API keys, or actual code content
 * - NEVER persist full request text (may contain code)
 * - Only store: redacted/truncated request_summary + optional request_hash + timestamps
 * - Scrub known sensitive patterns before writing
 *
 * Log location: ~/.config/aiy-desktop/logs/commands.jsonl
 * Rotation: 50MB max per file, keep 5 files
 */

/**
 * Sensitive patterns to scrub before logging.
 * These are replaced with [REDACTED] in all log output.
 */
export const SENSITIVE_PATTERNS = [
  // API Keys (various formats)
  /(?:api[_-]?key|apikey|api_secret|secret_key|auth_token|bearer)\s*[:=]\s*['"]?[\w-]{20,}['"]?/gi,
  // Generic secrets
  /(?:password|passwd|pwd|secret|token|credential)\s*[:=]\s*['"]?[^\s'"]+['"]?/gi,
  // AWS keys
  /AKIA[0-9A-Z]{16}/g,
  // GitHub tokens
  /gh[pousr]_[A-Za-z0-9_]{36,}/g,
  // Anthropic API keys
  /sk-ant-[A-Za-z0-9-_]{40,}/g,
  // OpenAI API keys
  /sk-[A-Za-z0-9]{48}/g,
  // xAI/Grok API keys
  /xai-[A-Za-z0-9-_]{40,}/g,
  // Google API keys
  /AIza[0-9A-Za-z-_]{35}/g,
  // Base64-encoded secrets (common pattern)
  /(?:basic|bearer)\s+[A-Za-z0-9+/=]{40,}/gi,
  // Environment variable assignments with secrets
  /(?:export\s+)?[A-Z_]*(?:KEY|SECRET|TOKEN|PASSWORD|CREDENTIAL)[A-Z_]*\s*=\s*['"]?[^\s'"]+['"]?/gi,
];

/**
 * Output chunk from CLI process (stdout or stderr)
 */
export interface OutputChunk {
  /** Stream source */
  stream: 'stdout' | 'stderr';
  /** Text content (already scrubbed of sensitive patterns) */
  text: string;
  /** Timestamp when received */
  timestamp: string;
}

/**
 * A single command log entry in JSONL format.
 * Each line in commands.jsonl is one of these objects.
 */
export interface CommandLogEntry {
  /** Unique job identifier */
  jobId: string;

  /** ISO 8601 timestamp when command started */
  startedAt: string;

  /** ISO 8601 timestamp when command completed */
  completedAt: string;

  /** Duration in milliseconds */
  durationMs: number;

  /** The CLI command that was executed (e.g., "aiy privacy check") */
  command: string;

  /** Command arguments (sensitive args like credential values are NOT logged) */
  args: string[];

  /** Process exit code */
  exitCode: number | null;

  /**
   * SAFE summary of stdout.
   * - Truncated to max 10KB
   * - All sensitive patterns scrubbed
   * - NEVER contains full request text or code
   */
  stdoutSummary: string;

  /**
   * SAFE summary of stderr.
   * - Truncated to max 10KB
   * - All sensitive patterns scrubbed
   */
  stderrSummary: string;

  /** True if output was truncated */
  truncated: boolean;

  /** Window/repo context */
  context: {
    /** Repository path this command was executed against */
    repoPath?: string;
    /** Window ID that initiated the command */
    windowId: string;
  };

  /**
   * For workflow execute commands only:
   * A SAFE summary of the request (never full text).
   * Max 100 characters, no code content.
   */
  requestSummary?: string;

  /**
   * Optional hash of the original request for correlation.
   * Allows linking logs without storing actual request.
   */
  requestHash?: string;

  /** True if command was cancelled by user */
  cancelled: boolean;

  /** Error message if command failed (already scrubbed) */
  error?: string;
}

/**
 * Log rotation configuration
 */
export const LOG_ROTATION_CONFIG = {
  /** Maximum size per log file in bytes (50MB) */
  maxFileSize: 50 * 1024 * 1024,
  /** Maximum number of log files to keep */
  maxFiles: 5,
  /** Log file name pattern */
  filePattern: 'commands.jsonl',
  /** Rotated file pattern */
  rotatedPattern: 'commands.{n}.jsonl',
} as const;

/**
 * Scrub sensitive patterns from text.
 * @param text The text to scrub
 * @returns Text with sensitive patterns replaced by [REDACTED]
 */
export function scrubSensitivePatterns(text: string): string {
  let result = text;
  for (const pattern of SENSITIVE_PATTERNS) {
    result = result.replace(pattern, '[REDACTED]');
  }
  return result;
}

/**
 * Truncate text to a maximum length.
 * @param text The text to truncate
 * @param maxLength Maximum length in characters
 * @returns Truncated text with indicator if truncated
 */
export function truncateForLog(text: string, maxLength = 10240): { text: string; truncated: boolean } {
  if (text.length <= maxLength) {
    return { text, truncated: false };
  }
  return {
    text: text.slice(0, maxLength) + '\n[...TRUNCATED...]',
    truncated: true,
  };
}

/**
 * Create a safe request summary from a full request.
 * NEVER returns actual code or full request text.
 * @param request The full request text
 * @returns A safe summary (max 100 chars)
 */
export function createRequestSummary(request: string): string {
  // Extract first line only, remove any code-like content
  const firstLine = request.split('\n')[0].trim();

  // Remove anything that looks like code (braces, brackets, semicolons in sequence)
  const safeText = firstLine
    .replace(/[{}\[\];]+/g, ' ')
    .replace(/\s+/g, ' ')
    .trim();

  // Truncate to 100 chars
  if (safeText.length > 100) {
    return safeText.slice(0, 97) + '...';
  }

  return safeText || '[request]';
}

/**
 * Create a hash of the request for correlation without storing content.
 * Uses a simple hash that's not reversible.
 * @param request The full request text
 * @returns A hex hash string
 */
export function createRequestHash(request: string): string {
  // Simple non-cryptographic hash for correlation
  let hash = 0;
  for (let i = 0; i < request.length; i++) {
    const char = request.charCodeAt(i);
    hash = ((hash << 5) - hash) + char;
    hash = hash & hash; // Convert to 32-bit integer
  }
  return Math.abs(hash).toString(16).padStart(8, '0');
}
