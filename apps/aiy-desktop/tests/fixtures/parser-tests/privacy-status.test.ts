/**
 * Privacy Status Parser Tests
 */

import { describe, it, expect } from 'vitest';
import { readFileSync } from 'fs';
import { join } from 'path';
import {
  parsePrivacyStatus,
  isOllamaConfigured,
  getStatusSummary,
} from '../../../src/shared/parsers/privacy-status';

const fixturesPath = join(__dirname, '../cli-outputs');

function loadFixture(name: string): string {
  return readFileSync(join(fixturesPath, name), 'utf-8');
}

describe('parsePrivacyStatus', () => {
  it('should parse enabled status correctly', () => {
    const output = loadFixture('privacy-status-enabled.txt');
    const result = parsePrivacyStatus(output);

    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.data.enabled).toBe(true);
      expect(result.data.rawOutput).toBe(output);
    }
  });

  it('should parse disabled status correctly', () => {
    const output = loadFixture('privacy-status-disabled.txt');
    const result = parsePrivacyStatus(output);

    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.data.enabled).toBe(false);
    }
  });

  it('should extract local executor config when enabled', () => {
    const output = loadFixture('privacy-status-enabled.txt');
    const result = parsePrivacyStatus(output);

    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.data.localExecutor).toBeDefined();
      expect(result.data.localExecutor?.ollamaUrl).toBe('http://127.0.0.1:11434');
      expect(result.data.localExecutor?.model).toBe('qwen2.5-coder:14b');
      expect(result.data.localExecutor?.contextSize).toBe(32768);
      expect(result.data.localExecutor?.timeout).toBe(120000);
    }
  });

  it('should extract RAG settings', () => {
    const output = loadFixture('privacy-status-enabled.txt');
    const result = parsePrivacyStatus(output);

    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.data.ragSettings).toBeDefined();
      expect(result.data.ragSettings?.tokenBudget).toBe(4096);
      expect(result.data.ragSettings?.topK).toBe(10);
      expect(result.data.ragSettings?.minSimilarity).toBe(0.7);
    }
  });

  it('should extract verification limits', () => {
    const output = loadFixture('privacy-status-enabled.txt');
    const result = parsePrivacyStatus(output);

    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.data.verificationLimits).toBeDefined();
      expect(result.data.verificationLimits?.maxFmtRepairs).toBe(3);
      expect(result.data.verificationLimits?.maxClippyRepairs).toBe(3);
      expect(result.data.verificationLimits?.maxTestRepairs).toBe(3);
      expect(result.data.verificationLimits?.maxGlobalRepairs).toBe(10);
    }
  });

  it('should extract exclude patterns', () => {
    const output = loadFixture('privacy-status-enabled.txt');
    const result = parsePrivacyStatus(output);

    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.data.excludePatterns).toBeDefined();
      expect(result.data.excludePatterns?.length).toBeGreaterThan(0);
      expect(result.data.excludePatterns).toContain('target/**');
      expect(result.data.excludePatterns).toContain('node_modules/**');
    }
  });

  it('should handle empty input gracefully', () => {
    const result = parsePrivacyStatus('');

    expect(result.success).toBe(false);
    if (!result.success) {
      expect(result.error.type).toBe('empty_input');
    }
  });

  it('should handle whitespace-only input', () => {
    const result = parsePrivacyStatus('   \n\n   ');

    expect(result.success).toBe(false);
    if (!result.success) {
      expect(result.error.type).toBe('empty_input');
    }
  });
});

describe('isOllamaConfigured', () => {
  it('should return true when Ollama is configured', () => {
    const output = loadFixture('privacy-status-enabled.txt');
    const result = parsePrivacyStatus(output);

    expect(result.success).toBe(true);
    if (result.success) {
      expect(isOllamaConfigured(result.data)).toBe(true);
    }
  });

  it('should return false when privacy mode is disabled', () => {
    const output = loadFixture('privacy-status-disabled.txt');
    const result = parsePrivacyStatus(output);

    expect(result.success).toBe(true);
    if (result.success) {
      expect(isOllamaConfigured(result.data)).toBe(false);
    }
  });
});

describe('getStatusSummary', () => {
  it('should generate summary for enabled status', () => {
    const output = loadFixture('privacy-status-enabled.txt');
    const result = parsePrivacyStatus(output);

    expect(result.success).toBe(true);
    if (result.success) {
      const summary = getStatusSummary(result.data);
      expect(summary).toContain('enabled');
      expect(summary).toContain('qwen2.5-coder:14b');
    }
  });

  it('should generate summary for disabled status', () => {
    const output = loadFixture('privacy-status-disabled.txt');
    const result = parsePrivacyStatus(output);

    expect(result.success).toBe(true);
    if (result.success) {
      const summary = getStatusSummary(result.data);
      expect(summary).toContain('disabled');
    }
  });
});
