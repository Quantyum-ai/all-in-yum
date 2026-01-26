/**
 * Agents Output Parser
 *
 * Parses output from `aiy agents list` and `aiy agents status` commands.
 *
 * Expected output format for `aiy agents list` (from Rust CLI):
 * ```
 * Available Agents
 * ================
 *
 * ID           Name                      Status     Credentials
 * ------------------------------------------------------------
 * grok         Grok (xAI)               enabled    configured
 * claude       Claude (Anthropic)       disabled   missing
 * gemini       Gemini (Google)          disabled   missing
 * codex        Codex (OpenAI)           disabled   missing
 *
 * Tip: To enable an agent:  aiy agents enable <id>
 *      To set credentials: aiy credentials set <provider>
 * ```
 *
 * Expected output format for `aiy agents status` (from Rust CLI):
 * ```
 * Agent Credential Status
 * =======================
 *
 * [OK] Grok (xAI) - Ready
 * [!] Claude (Anthropic) - Missing credentials
 *     Set with: aiy credentials set anthropic
 *
 * [OK] 1 of 2 enabled agents ready
 * ```
 */

import type {
  AgentInfo,
  AgentsListResult,
  AgentStatusEntry,
  AgentsStatusResult,
  ParseResult,
} from './types';

/**
 * Known agent ID to credential provider mapping
 */
const AGENT_PROVIDERS: Record<string, string> = {
  grok: 'xai',
  claude: 'anthropic',
  gemini: 'google',
  codex: 'openai',
  ollama: 'ollama',
};

/**
 * Parse a table row from agents list output
 */
function parseTableRow(line: string): AgentInfo | null {
  // Skip header/separator lines
  if (line.includes('----') || line.includes('ID ') || !line.trim()) {
    return null;
  }

  // Try to parse space-separated columns
  // Format: ID           Name                      Status     Credentials
  const parts = line.trim().split(/\s{2,}/); // Split on 2+ spaces

  if (parts.length < 4) {
    // Try alternative parsing with flexible whitespace
    const match = line.match(/^\s*(\w+)\s+(.+?)\s+(enabled|disabled)\s+(configured|missing)\s*$/i);
    if (match) {
      const [, id, name, status, credentials] = match;
      return {
        id: id.toLowerCase(),
        name: name.trim(),
        enabled: status.toLowerCase() === 'enabled',
        hasCredentials: credentials.toLowerCase() === 'configured',
        credentialProvider: AGENT_PROVIDERS[id.toLowerCase()],
      };
    }
    return null;
  }

  const [id, name, status, credentials] = parts;

  return {
    id: id.toLowerCase().trim(),
    name: name.trim(),
    enabled: status.toLowerCase().trim() === 'enabled',
    hasCredentials: credentials.toLowerCase().trim() === 'configured',
    credentialProvider: AGENT_PROVIDERS[id.toLowerCase().trim()],
  };
}

/**
 * Parse agents list output.
 *
 * @param output - Raw output from `aiy agents list`
 * @returns Parsed result with agent information
 *
 * @example
 * ```typescript
 * const result = parseAgentsList(cliOutput);
 * if (result.success) {
 *   result.data.agents.forEach(agent => {
 *     console.log(`${agent.name}: ${agent.enabled ? 'enabled' : 'disabled'}`);
 *   });
 * }
 * ```
 */
export function parseAgentsList(output: string): ParseResult<AgentsListResult> {
  // Handle empty input
  if (!output || output.trim().length === 0) {
    return {
      success: false,
      error: {
        type: 'empty_input',
        message: 'Agents list output is empty',
      },
      rawOutput: output || '',
    };
  }

  const lines = output.split('\n');
  const agents: AgentInfo[] = [];
  let inTable = false;

  for (const line of lines) {
    // Detect table start (after separator line)
    if (line.includes('----')) {
      inTable = true;
      continue;
    }

    // Detect table end (empty line or tip section)
    if (inTable && (line.trim() === '' || line.includes('Tip:'))) {
      inTable = false;
      continue;
    }

    // Parse table rows
    if (inTable) {
      const agent = parseTableRow(line);
      if (agent) {
        agents.push(agent);
      }
    }
  }

  return {
    success: true,
    data: {
      agents,
      rawOutput: output,
    },
  };
}

/**
 * Parse agents status output.
 *
 * @param output - Raw output from `aiy agents status`
 * @returns Parsed result with agent status entries
 */
export function parseAgentsStatus(output: string): ParseResult<AgentsStatusResult> {
  // Handle empty input
  if (!output || output.trim().length === 0) {
    return {
      success: false,
      error: {
        type: 'empty_input',
        message: 'Agents status output is empty',
      },
      rawOutput: output || '',
    };
  }

  const lines = output.split('\n');
  const entries: AgentStatusEntry[] = [];
  let summaryReady = 0;
  let summaryTotal = 0;

  for (const line of lines) {
    // Parse [OK] or [!] status lines
    const okMatch = line.match(/\[OK\]\s*(.+?)\s*-\s*(.+)/);
    if (okMatch) {
      const [, name, message] = okMatch;
      // Extract agent ID from name (e.g., "Grok (xAI)" -> "grok")
      const idMatch = name.match(/^(\w+)/);
      entries.push({
        id: idMatch ? idMatch[1].toLowerCase() : name.toLowerCase(),
        name: name.trim(),
        ready: true,
        message: message.trim(),
      });
      continue;
    }

    const failMatch = line.match(/\[!\]\s*(.+?)\s*-\s*(.+)/);
    if (failMatch) {
      const [, name, message] = failMatch;
      const idMatch = name.match(/^(\w+)/);
      entries.push({
        id: idMatch ? idMatch[1].toLowerCase() : name.toLowerCase(),
        name: name.trim(),
        ready: false,
        message: message.trim(),
      });
      continue;
    }

    // Parse summary line: [OK] 1 of 2 enabled agents ready
    const summaryMatch = line.match(/\[OK\]\s*(\d+)\s+of\s+(\d+)\s+enabled\s+agents?\s+ready/i);
    if (summaryMatch) {
      summaryReady = parseInt(summaryMatch[1], 10);
      summaryTotal = parseInt(summaryMatch[2], 10);
    }
  }

  return {
    success: true,
    data: {
      entries,
      summary: {
        ready: summaryReady || entries.filter((e) => e.ready).length,
        total: summaryTotal || entries.length,
      },
      rawOutput: output,
    },
  };
}

/**
 * Get agents that need credentials
 */
export function getAgentsNeedingCredentials(result: AgentsListResult): AgentInfo[] {
  return result.agents.filter((a) => a.enabled && !a.hasCredentials);
}

/**
 * Get enabled agents
 */
export function getEnabledAgents(result: AgentsListResult): AgentInfo[] {
  return result.agents.filter((a) => a.enabled);
}

/**
 * Get agent by ID
 */
export function getAgentById(result: AgentsListResult, id: string): AgentInfo | undefined {
  return result.agents.find((a) => a.id.toLowerCase() === id.toLowerCase());
}
