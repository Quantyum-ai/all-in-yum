/**
 * Agents Parser Tests
 */

import { describe, it, expect } from 'vitest';
import { readFileSync } from 'fs';
import { join } from 'path';
import {
  parseAgentsList,
  parseAgentsStatus,
  getAgentsNeedingCredentials,
  getEnabledAgents,
  getAgentById,
} from '../../../src/shared/parsers/agents';

const fixturesPath = join(__dirname, '../cli-outputs');

function loadFixture(name: string): string {
  return readFileSync(join(fixturesPath, name), 'utf-8');
}

describe('parseAgentsList', () => {
  it('should parse agents list correctly', () => {
    const output = loadFixture('agents-list.txt');
    const result = parseAgentsList(output);

    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.data.agents.length).toBeGreaterThan(0);
      expect(result.data.rawOutput).toBe(output);

      // Check specific agents
      const grok = result.data.agents.find((a) => a.id === 'grok');
      expect(grok).toBeDefined();
      expect(grok?.enabled).toBe(true);
      expect(grok?.hasCredentials).toBe(true);
      expect(grok?.name).toContain('Grok');
    }
  });

  it('should identify enabled vs disabled agents', () => {
    const output = loadFixture('agents-list.txt');
    const result = parseAgentsList(output);

    expect(result.success).toBe(true);
    if (result.success) {
      const enabledAgents = result.data.agents.filter((a) => a.enabled);
      const disabledAgents = result.data.agents.filter((a) => !a.enabled);

      expect(enabledAgents.length).toBeGreaterThan(0);
      expect(disabledAgents.length).toBeGreaterThan(0);
    }
  });

  it('should identify configured vs missing credentials', () => {
    const output = loadFixture('agents-list.txt');
    const result = parseAgentsList(output);

    expect(result.success).toBe(true);
    if (result.success) {
      const configured = result.data.agents.filter((a) => a.hasCredentials);
      const missing = result.data.agents.filter((a) => !a.hasCredentials);

      expect(configured.length).toBeGreaterThan(0);
      expect(missing.length).toBeGreaterThan(0);
    }
  });

  it('should include credential provider mapping', () => {
    const output = loadFixture('agents-list.txt');
    const result = parseAgentsList(output);

    expect(result.success).toBe(true);
    if (result.success) {
      const grok = result.data.agents.find((a) => a.id === 'grok');
      expect(grok?.credentialProvider).toBe('xai');

      const claude = result.data.agents.find((a) => a.id === 'claude');
      expect(claude?.credentialProvider).toBe('anthropic');
    }
  });

  it('should handle empty input gracefully', () => {
    const result = parseAgentsList('');

    expect(result.success).toBe(false);
    if (!result.success) {
      expect(result.error.type).toBe('empty_input');
    }
  });
});

describe('parseAgentsStatus', () => {
  it('should parse all-configured status correctly', () => {
    const output = loadFixture('agents-status-all-configured.txt');
    const result = parseAgentsStatus(output);

    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.data.entries.every((e) => e.ready)).toBe(true);
      expect(result.data.summary.ready).toBe(result.data.summary.total);
    }
  });

  it('should parse partial status correctly', () => {
    const output = loadFixture('agents-status-partial.txt');
    const result = parseAgentsStatus(output);

    expect(result.success).toBe(true);
    if (result.success) {
      const readyAgents = result.data.entries.filter((e) => e.ready);
      const notReady = result.data.entries.filter((e) => !e.ready);

      expect(readyAgents.length).toBeGreaterThan(0);
      expect(notReady.length).toBeGreaterThan(0);
      expect(result.data.summary.ready).toBe(1);
      expect(result.data.summary.total).toBe(3);
    }
  });

  it('should extract agent names from status entries', () => {
    const output = loadFixture('agents-status-all-configured.txt');
    const result = parseAgentsStatus(output);

    expect(result.success).toBe(true);
    if (result.success) {
      const hasGrok = result.data.entries.some((e) => e.name.includes('Grok'));
      expect(hasGrok).toBe(true);
    }
  });
});

describe('getAgentsNeedingCredentials', () => {
  it('should return enabled agents without credentials', () => {
    const output = loadFixture('agents-list.txt');
    const result = parseAgentsList(output);

    expect(result.success).toBe(true);
    if (result.success) {
      const needingCreds = getAgentsNeedingCredentials(result.data);

      // All returned agents should be enabled but missing credentials
      expect(needingCreds.every((a) => a.enabled && !a.hasCredentials)).toBe(true);
    }
  });
});

describe('getEnabledAgents', () => {
  it('should return only enabled agents', () => {
    const output = loadFixture('agents-list.txt');
    const result = parseAgentsList(output);

    expect(result.success).toBe(true);
    if (result.success) {
      const enabled = getEnabledAgents(result.data);
      expect(enabled.every((a) => a.enabled)).toBe(true);
    }
  });
});

describe('getAgentById', () => {
  it('should find agent by ID (case-insensitive)', () => {
    const output = loadFixture('agents-list.txt');
    const result = parseAgentsList(output);

    expect(result.success).toBe(true);
    if (result.success) {
      const grok = getAgentById(result.data, 'GROK');
      expect(grok).toBeDefined();
      expect(grok?.id).toBe('grok');
    }
  });

  it('should return undefined for non-existent agent', () => {
    const output = loadFixture('agents-list.txt');
    const result = parseAgentsList(output);

    expect(result.success).toBe(true);
    if (result.success) {
      const notFound = getAgentById(result.data, 'nonexistent');
      expect(notFound).toBeUndefined();
    }
  });
});
