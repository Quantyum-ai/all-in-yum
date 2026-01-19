/**
 * Privacy Check Output Parser
 *
 * Parses output from `aiy privacy check` command.
 *
 * IMPORTANT: Do NOT rely on exit code for success/failure.
 * The CLI may return exit code 0 even when checks fail.
 * Always parse the [PASS]/[FAIL] markers in the output.
 *
 * Expected output format (from Rust CLI):
 * ```
 * Privacy Mode Check
 * ==================
 *
 * [PASS] Privacy mode: enabled
 * [PASS] Ollama URL loopback-only: http://127.0.0.1:11434
 *
 * Ollama Connectivity:
 *   [PASS] Ollama server reachable
 *   [INFO] Available models:
 *     * qwen2.5-coder:14b
 *       llama3.1:8b
 *   [PASS] Configured model 'qwen2.5-coder:14b' is available
 * ```
 */

import type { CheckMarker, CheckResult, PrivacyCheckResult, ParseResult, ParserError } from './types';

/**
 * Regex to match check markers: [PASS], [FAIL], [WARN], [SKIP], [INFO]
 * Captures: leading whitespace (for indentation), marker, and the rest of the line
 */
const MARKER_REGEX = /^(\s*)\[(PASS|FAIL|WARN|SKIP|INFO)\]\s*(.*)$/;

/**
 * Parse a single line for check markers
 */
function parseLine(line: string): CheckResult | null {
  const match = line.match(MARKER_REGEX);
  if (!match) {
    return null;
  }

  const [, indent, marker, rest] = match;
  const level = Math.floor(indent.length / 2); // 2 spaces per level

  return {
    marker: marker as CheckMarker,
    name: rest.trim(),
    level,
  };
}

/**
 * Parse privacy check output.
 *
 * @param output - Raw output from `aiy privacy check`
 * @returns Parsed result with pass/fail status and individual checks
 *
 * @example
 * ```typescript
 * const result = parsePrivacyCheck(cliOutput);
 * if (result.success) {
 *   console.log(result.data.passed ? 'All checks passed' : 'Some checks failed');
 *   console.log(`Failures: ${result.data.summary.fail}`);
 * }
 * ```
 */
export function parsePrivacyCheck(output: string): ParseResult<PrivacyCheckResult> {
  // Handle empty input
  if (!output || output.trim().length === 0) {
    return {
      success: false,
      error: {
        type: 'empty_input',
        message: 'Privacy check output is empty',
      },
      rawOutput: output || '',
    };
  }

  const lines = output.split('\n');
  const checks: CheckResult[] = [];
  const summary = {
    pass: 0,
    fail: 0,
    warn: 0,
    skip: 0,
    info: 0,
  };

  for (const line of lines) {
    const check = parseLine(line);
    if (check) {
      checks.push(check);

      // Update summary counts
      switch (check.marker) {
        case 'PASS':
          summary.pass++;
          break;
        case 'FAIL':
          summary.fail++;
          break;
        case 'WARN':
          summary.warn++;
          break;
        case 'SKIP':
          summary.skip++;
          break;
        case 'INFO':
          summary.info++;
          break;
      }
    }
  }

  // Determine overall pass/fail - passes only if no FAIL markers
  const passed = summary.fail === 0;

  return {
    success: true,
    data: {
      passed,
      checks,
      summary,
      rawOutput: output,
    },
  };
}

/**
 * Extract only FAIL checks for error display
 */
export function extractFailures(result: PrivacyCheckResult): CheckResult[] {
  return result.checks.filter((check) => check.marker === 'FAIL');
}

/**
 * Extract only WARN checks for warning display
 */
export function extractWarnings(result: PrivacyCheckResult): CheckResult[] {
  return result.checks.filter((check) => check.marker === 'WARN');
}

/**
 * Format check results for display (without ANSI colors)
 */
export function formatCheckResult(check: CheckResult): string {
  const indent = '  '.repeat(check.level);
  return `${indent}[${check.marker}] ${check.name}`;
}

/**
 * Get a human-readable summary of the check results
 */
export function getCheckSummary(result: PrivacyCheckResult): string {
  const { summary } = result;
  const parts: string[] = [];

  if (summary.pass > 0) parts.push(`${summary.pass} passed`);
  if (summary.fail > 0) parts.push(`${summary.fail} failed`);
  if (summary.warn > 0) parts.push(`${summary.warn} warnings`);
  if (summary.skip > 0) parts.push(`${summary.skip} skipped`);

  if (parts.length === 0) {
    return 'No checks performed';
  }

  return parts.join(', ');
}
