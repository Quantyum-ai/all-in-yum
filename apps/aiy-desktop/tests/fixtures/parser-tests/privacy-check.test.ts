/**
 * Privacy Check Parser Tests
 */

import { describe, it, expect } from 'vitest';
import { readFileSync } from 'fs';
import { join } from 'path';
import {
  parsePrivacyCheck,
  extractFailures,
  extractWarnings,
  getCheckSummary,
} from '../../../src/shared/parsers/privacy-check';

const fixturesPath = join(__dirname, '../cli-outputs');

function loadFixture(name: string): string {
  return readFileSync(join(fixturesPath, name), 'utf-8');
}

describe('parsePrivacyCheck', () => {
  it('should parse all-pass output correctly', () => {
    const output = loadFixture('privacy-check-pass.txt');
    const result = parsePrivacyCheck(output);

    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.data.passed).toBe(true);
      expect(result.data.summary.fail).toBe(0);
      expect(result.data.summary.pass).toBeGreaterThan(0);
      expect(result.data.rawOutput).toBe(output);
    }
  });

  it('should parse failed output correctly', () => {
    const output = loadFixture('privacy-check-fail.txt');
    const result = parsePrivacyCheck(output);

    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.data.passed).toBe(false);
      expect(result.data.summary.fail).toBeGreaterThan(0);
    }
  });

  it('should parse mixed output with warnings', () => {
    const output = loadFixture('privacy-check-mixed.txt');
    const result = parsePrivacyCheck(output);

    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.data.passed).toBe(true); // No FAIL markers
      expect(result.data.summary.warn).toBeGreaterThan(0);
      expect(result.data.summary.pass).toBeGreaterThan(0);
    }
  });

  it('should handle empty input gracefully', () => {
    const result = parsePrivacyCheck('');

    expect(result.success).toBe(false);
    if (!result.success) {
      expect(result.error.type).toBe('empty_input');
    }
  });

  it('should handle whitespace-only input', () => {
    const result = parsePrivacyCheck('   \n\n   \t   ');

    expect(result.success).toBe(false);
    if (!result.success) {
      expect(result.error.type).toBe('empty_input');
    }
  });

  it('should parse check names correctly', () => {
    const output = loadFixture('privacy-check-pass.txt');
    const result = parsePrivacyCheck(output);

    expect(result.success).toBe(true);
    if (result.success) {
      const passChecks = result.data.checks.filter((c) => c.marker === 'PASS');
      expect(passChecks.some((c) => c.name.includes('Privacy mode'))).toBe(true);
      expect(passChecks.some((c) => c.name.includes('Ollama'))).toBe(true);
    }
  });

  it('should detect indentation levels for nested checks', () => {
    const output = loadFixture('privacy-check-pass.txt');
    const result = parsePrivacyCheck(output);

    expect(result.success).toBe(true);
    if (result.success) {
      // Top-level checks should have level 0
      const topLevel = result.data.checks.filter((c) => c.level === 0);
      expect(topLevel.length).toBeGreaterThan(0);

      // Nested checks (under "Ollama Connectivity:") should have level > 0
      const nested = result.data.checks.filter((c) => c.level > 0);
      expect(nested.length).toBeGreaterThan(0);
    }
  });
});

describe('extractFailures', () => {
  it('should extract only FAIL markers', () => {
    const output = loadFixture('privacy-check-fail.txt');
    const result = parsePrivacyCheck(output);

    expect(result.success).toBe(true);
    if (result.success) {
      const failures = extractFailures(result.data);
      expect(failures.length).toBeGreaterThan(0);
      expect(failures.every((f) => f.marker === 'FAIL')).toBe(true);
    }
  });

  it('should return empty array for all-pass output', () => {
    const output = loadFixture('privacy-check-pass.txt');
    const result = parsePrivacyCheck(output);

    expect(result.success).toBe(true);
    if (result.success) {
      const failures = extractFailures(result.data);
      expect(failures.length).toBe(0);
    }
  });
});

describe('extractWarnings', () => {
  it('should extract only WARN markers', () => {
    const output = loadFixture('privacy-check-mixed.txt');
    const result = parsePrivacyCheck(output);

    expect(result.success).toBe(true);
    if (result.success) {
      const warnings = extractWarnings(result.data);
      expect(warnings.length).toBeGreaterThan(0);
      expect(warnings.every((w) => w.marker === 'WARN')).toBe(true);
    }
  });
});

describe('getCheckSummary', () => {
  it('should generate human-readable summary', () => {
    const output = loadFixture('privacy-check-pass.txt');
    const result = parsePrivacyCheck(output);

    expect(result.success).toBe(true);
    if (result.success) {
      const summary = getCheckSummary(result.data);
      expect(summary).toContain('passed');
      expect(typeof summary).toBe('string');
    }
  });

  it('should include failure count when present', () => {
    const output = loadFixture('privacy-check-fail.txt');
    const result = parsePrivacyCheck(output);

    expect(result.success).toBe(true);
    if (result.success) {
      const summary = getCheckSummary(result.data);
      expect(summary).toContain('failed');
    }
  });
});
