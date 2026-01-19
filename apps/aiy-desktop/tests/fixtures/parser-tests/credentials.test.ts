/**
 * Credentials Parser Tests
 */

import { describe, it, expect } from 'vitest';
import { readFileSync } from 'fs';
import { join } from 'path';
import {
  parseCredentialsStatus,
  getMissingProviders,
  getProviderDisplayName,
  isProviderConfigured,
  getCredentialsSummary,
} from '../../../src/shared/parsers/credentials';

const fixturesPath = join(__dirname, '../cli-outputs');

function loadFixture(name: string): string {
  return readFileSync(join(fixturesPath, name), 'utf-8');
}

describe('parseCredentialsStatus', () => {
  it('should parse full credentials status correctly', () => {
    const output = loadFixture('credentials-status-full.txt');
    const result = parseCredentialsStatus(output);

    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.data.storedCount).toBe(4);
      expect(result.data.providers.length).toBe(4);
      expect(result.data.backend).toContain('EncryptedFile');
    }
  });

  it('should parse partial credentials status correctly', () => {
    const output = loadFixture('credentials-status-partial.txt');
    const result = parseCredentialsStatus(output);

    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.data.storedCount).toBe(2);
      expect(result.data.providers.length).toBe(2);

      // Check specific providers
      const xai = result.data.providers.find((p) => p.provider === 'xai');
      const anthropic = result.data.providers.find((p) => p.provider === 'anthropic');
      expect(xai?.configured).toBe(true);
      expect(anthropic?.configured).toBe(true);
    }
  });

  it('should include all known providers in allProviders', () => {
    const output = loadFixture('credentials-status-partial.txt');
    const result = parseCredentialsStatus(output);

    expect(result.success).toBe(true);
    if (result.success) {
      // allProviders should include both configured and unconfigured
      expect(result.data.allProviders.length).toBeGreaterThanOrEqual(4);

      const google = result.data.allProviders.find((p) => p.provider === 'google');
      expect(google?.configured).toBe(false);
    }
  });

  it('should handle empty input gracefully', () => {
    const result = parseCredentialsStatus('');

    expect(result.success).toBe(false);
    if (!result.success) {
      expect(result.error.type).toBe('empty_input');
    }
  });
});

describe('getMissingProviders', () => {
  it('should return unconfigured providers', () => {
    const output = loadFixture('credentials-status-partial.txt');
    const result = parseCredentialsStatus(output);

    expect(result.success).toBe(true);
    if (result.success) {
      const missing = getMissingProviders(result.data);
      expect(missing.length).toBeGreaterThan(0);
      expect(missing.every((p) => !p.configured)).toBe(true);
    }
  });

  it('should return empty array when all configured', () => {
    const output = loadFixture('credentials-status-full.txt');
    const result = parseCredentialsStatus(output);

    expect(result.success).toBe(true);
    if (result.success) {
      const missing = getMissingProviders(result.data);
      expect(missing.length).toBe(0);
    }
  });
});

describe('getProviderDisplayName', () => {
  it('should return display name for known providers', () => {
    expect(getProviderDisplayName('xai')).toContain('xAI');
    expect(getProviderDisplayName('anthropic')).toContain('Claude');
    expect(getProviderDisplayName('google')).toContain('Gemini');
    expect(getProviderDisplayName('openai')).toContain('Codex');
  });

  it('should return provider ID for unknown providers', () => {
    expect(getProviderDisplayName('unknown')).toBe('unknown');
  });

  it('should be case-insensitive', () => {
    expect(getProviderDisplayName('XAI')).toContain('xAI');
  });
});

describe('isProviderConfigured', () => {
  it('should return true for configured providers', () => {
    const output = loadFixture('credentials-status-partial.txt');
    const result = parseCredentialsStatus(output);

    expect(result.success).toBe(true);
    if (result.success) {
      expect(isProviderConfigured(result.data, 'xai')).toBe(true);
      expect(isProviderConfigured(result.data, 'anthropic')).toBe(true);
    }
  });

  it('should return false for unconfigured providers', () => {
    const output = loadFixture('credentials-status-partial.txt');
    const result = parseCredentialsStatus(output);

    expect(result.success).toBe(true);
    if (result.success) {
      expect(isProviderConfigured(result.data, 'google')).toBe(false);
      expect(isProviderConfigured(result.data, 'openai')).toBe(false);
    }
  });

  it('should be case-insensitive', () => {
    const output = loadFixture('credentials-status-partial.txt');
    const result = parseCredentialsStatus(output);

    expect(result.success).toBe(true);
    if (result.success) {
      expect(isProviderConfigured(result.data, 'XAI')).toBe(true);
      expect(isProviderConfigured(result.data, 'GOOGLE')).toBe(false);
    }
  });
});

describe('getCredentialsSummary', () => {
  it('should return appropriate summary for partial configuration', () => {
    const output = loadFixture('credentials-status-partial.txt');
    const result = parseCredentialsStatus(output);

    expect(result.success).toBe(true);
    if (result.success) {
      const summary = getCredentialsSummary(result.data);
      expect(summary).toContain('2/4');
    }
  });

  it('should indicate all configured when complete', () => {
    const output = loadFixture('credentials-status-full.txt');
    const result = parseCredentialsStatus(output);

    expect(result.success).toBe(true);
    if (result.success) {
      const summary = getCredentialsSummary(result.data);
      expect(summary).toContain('All credentials configured');
    }
  });
});
