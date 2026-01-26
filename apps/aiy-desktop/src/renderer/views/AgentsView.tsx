import React from 'react';
import { Button } from '../components/common/Button';
import { parseAgentsList, parseAgentsStatus } from '../../shared/parsers';
import type { AgentInfo, AgentStatusEntry } from '../../shared/parsers/types';

/**
 * Agents View
 *
 * Provides UI for managing AI agents:
 * - List all available agents with their status
 * - Show credential configuration status
 * - Enable/Disable individual agents
 * - Refresh agent list and status
 */
export function AgentsView(): React.ReactElement {
  const [agents, setAgents] = React.useState<AgentInfo[]>([]);
  const [statusEntries, setStatusEntries] = React.useState<AgentStatusEntry[]>([]);
  const [loading, setLoading] = React.useState(false);
  const [actionLoading, setActionLoading] = React.useState<string | null>(null);

  /**
   * Load agents list from CLI
   */
  const loadAgentsList = React.useCallback(async (): Promise<void> => {
    setLoading(true);
    try {
      const jobId = await window.electronAPI.cli.run('agents', ['list']);

      const outputs: string[] = [];
      const cleanupOutput = window.electronAPI.cli.onOutput((jid, chunk) => {
        if (jid === jobId) outputs.push(chunk.text);
      });

      const cleanupExit = window.electronAPI.cli.onExit((jid) => {
        if (jid === jobId) {
          const fullOutput = outputs.join('');
          const result = parseAgentsList(fullOutput);
          if (result.success) {
            setAgents(result.data.agents);
          }
          cleanupOutput();
          cleanupExit();
          setLoading(false);
        }
      });
    } catch (error) {
      console.error('Failed to load agents:', error);
      setLoading(false);
    }
  }, []);

  /**
   * Load agents status from CLI
   */
  const loadAgentsStatus = React.useCallback(async (): Promise<void> => {
    try {
      const jobId = await window.electronAPI.cli.run('agents', ['status']);

      const outputs: string[] = [];
      const cleanupOutput = window.electronAPI.cli.onOutput((jid, chunk) => {
        if (jid === jobId) outputs.push(chunk.text);
      });

      const cleanupExit = window.electronAPI.cli.onExit((jid) => {
        if (jid === jobId) {
          const fullOutput = outputs.join('');
          const result = parseAgentsStatus(fullOutput);
          if (result.success) {
            setStatusEntries(result.data.entries);
          }
          cleanupOutput();
          cleanupExit();
        }
      });
    } catch (error) {
      console.error('Failed to load agent status:', error);
    }
  }, []);

  /**
   * Load agents on mount
   */
  React.useEffect(() => {
    loadAgentsList();
    loadAgentsStatus();
  }, [loadAgentsList, loadAgentsStatus]);

  /**
   * Toggle agent enabled/disabled state
   */
  const handleToggleAgent = async (
    agentId: string,
    currentlyEnabled: boolean
  ): Promise<void> => {
    setActionLoading(agentId);
    try {
      const subcommand = currentlyEnabled ? 'disable' : 'enable';
      const jobId = await window.electronAPI.cli.run('agents', [subcommand, agentId]);

      const cleanupExit = window.electronAPI.cli.onExit((jid) => {
        if (jid === jobId) {
          loadAgentsList();
          loadAgentsStatus();
          setActionLoading(null);
          cleanupExit();
        }
      });
    } catch (error) {
      console.error(`Failed to toggle agent ${agentId}:`, error);
      setActionLoading(null);
    }
  };

  /**
   * Refresh both agents list and status
   */
  const handleRefresh = async (): Promise<void> => {
    await Promise.all([loadAgentsList(), loadAgentsStatus()]);
  };

  return (
    <div className="mx-auto max-w-4xl space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-xl font-bold text-aiy-text-primary">AI Agents</h2>
          <p className="mt-1 text-sm text-aiy-text-secondary">
            Manage AI agents and their credentials
          </p>
        </div>
        <Button onClick={handleRefresh} size="sm" variant="secondary" disabled={loading}>
          {loading ? 'Refreshing...' : 'Refresh'}
        </Button>
      </div>

      {/* Agents List */}
      <div className="card">
        {loading && agents.length === 0 ? (
          <p className="text-aiy-text-muted">Loading agents...</p>
        ) : agents.length === 0 ? (
          <p className="text-aiy-text-muted">No agents found</p>
        ) : (
          <div className="space-y-2">
            {agents.map((agent) => {
              const statusEntry = statusEntries.find((s) => s.id === agent.id);

              return (
                <div
                  key={agent.id}
                  className="flex items-center justify-between border-b border-aiy-border pb-3 last:border-0 last:pb-0"
                >
                  <div className="flex-1">
                    <div className="flex items-center gap-2">
                      <p className="font-medium text-aiy-text-primary">{agent.name}</p>
                      <span
                        className={`rounded px-2 py-0.5 text-xs ${
                          agent.enabled
                            ? 'bg-green-500/20 text-green-400'
                            : 'bg-gray-500/20 text-gray-400'
                        }`}
                      >
                        {agent.enabled ? 'Enabled' : 'Disabled'}
                      </span>
                      <span
                        className={`rounded px-2 py-0.5 text-xs ${
                          agent.hasCredentials
                            ? 'bg-blue-500/20 text-blue-400'
                            : 'bg-red-500/20 text-red-400'
                        }`}
                      >
                        {agent.hasCredentials ? 'Configured' : 'Missing Creds'}
                      </span>
                    </div>
                    {statusEntry && !statusEntry.ready && statusEntry.message && (
                      <p className="mt-1 text-xs text-aiy-text-muted">{statusEntry.message}</p>
                    )}
                  </div>
                  <Button
                    size="sm"
                    variant="secondary"
                    onClick={() => handleToggleAgent(agent.id, agent.enabled)}
                    disabled={actionLoading === agent.id}
                  >
                    {actionLoading === agent.id
                      ? 'Loading...'
                      : agent.enabled
                        ? 'Disable'
                        : 'Enable'}
                  </Button>
                </div>
              );
            })}
          </div>
        )}
      </div>

      {/* Credentials Help */}
      <div className="card">
        <h3 className="text-sm font-medium text-aiy-text-muted">Setting Up Credentials</h3>
        <p className="mt-2 text-sm text-aiy-text-secondary">
          To configure credentials for an agent, use the CLI command:
        </p>
        <pre className="mt-2 rounded bg-aiy-surface p-3 font-mono text-xs text-aiy-text-primary">
          aiy credentials set {'<provider>'}
        </pre>
        <p className="mt-2 text-xs text-aiy-text-muted">
          Available providers: xai, anthropic, google, openai
        </p>
      </div>
    </div>
  );
}
