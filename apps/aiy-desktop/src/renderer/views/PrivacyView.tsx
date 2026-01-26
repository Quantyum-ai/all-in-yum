import React from 'react';
import { Button } from '../components/common/Button';
import { usePrivacyStore } from '../stores/privacy-store';
import { useRepoStore } from '../stores/repo-store';
import { parsePrivacyStatus, parsePrivacyCheck } from '../../shared/parsers';

/**
 * Privacy Mode View
 *
 * Provides UI for managing privacy mode settings:
 * - UI privacy mode toggle (Always Local / Hybrid) persisted via privacy-store
 * - CLI privacy commands (status, check, config, enable, disable)
 * - Real-time output display from CLI commands
 */
export function PrivacyView(): React.ReactElement {
  const { mode, setMode } = usePrivacyStore();
  const { repoPath } = useRepoStore();
  const [status, setStatus] = React.useState<string>('');
  const [checkOutput, setCheckOutput] = React.useState<string>('');
  const [config, setConfig] = React.useState<string>('');
  const [loading, setLoading] = React.useState<string | null>(null);

  /**
   * Run a privacy CLI command and collect output
   */
  const runCommand = async (
    subcommand: string,
    args: string[] = []
  ): Promise<string | null> => {
    if (!repoPath) {
      return null;
    }

    try {
      const jobId = await window.electronAPI.cli.run(
        'privacy',
        [subcommand, ...args],
        { repoPath }
      );

      return new Promise<string>((resolve) => {
        const outputs: string[] = [];

        const cleanupOutput = window.electronAPI.cli.onOutput((jid, chunk) => {
          if (jid === jobId) {
            outputs.push(chunk.text);
          }
        });

        const cleanupExit = window.electronAPI.cli.onExit((jid) => {
          if (jid === jobId) {
            cleanupOutput();
            cleanupExit();
            resolve(outputs.join(''));
          }
        });
      });
    } catch (error) {
      console.error(`Failed to run privacy ${subcommand}:`, error);
      return null;
    }
  };

  /**
   * Fetch and display privacy status
   */
  const handleStatus = async (): Promise<void> => {
    setLoading('status');
    const output = await runCommand('status');
    if (output) {
      setStatus(output);
      const parsed = parsePrivacyStatus(output);
      if (parsed.success) {
        console.log('Privacy status:', parsed.data);
      }
    }
    setLoading(null);
  };

  /**
   * Run privacy verification check
   */
  const handleCheck = async (): Promise<void> => {
    setLoading('check');
    const output = await runCommand('check');
    if (output) {
      setCheckOutput(output);
      const parsed = parsePrivacyCheck(output);
      if (parsed.success) {
        console.log(
          `Privacy check: ${parsed.data.passed ? 'PASS' : 'FAIL'}`,
          parsed.data.summary
        );
      }
    }
    setLoading(null);
  };

  /**
   * Show privacy configuration
   */
  const handleShowConfig = async (): Promise<void> => {
    setLoading('config');
    const output = await runCommand('config', ['show']);
    if (output) {
      setConfig(output);
    }
    setLoading(null);
  };

  /**
   * Enable privacy mode with default Ollama settings
   */
  const handleEnable = async (): Promise<void> => {
    setLoading('enable');
    await runCommand('enable', [
      '--ollama-url',
      'http://127.0.0.1:11434',
      '--model',
      'codellama:7b-instruct',
    ]);
    await handleStatus();
    setLoading(null);
  };

  /**
   * Disable privacy mode
   */
  const handleDisable = async (): Promise<void> => {
    setLoading('disable');
    await runCommand('disable');
    await handleStatus();
    setLoading(null);
  };

  return (
    <div className="mx-auto max-w-3xl space-y-6">
      {/* Header */}
      <div>
        <h2 className="text-xl font-bold text-aiy-text-primary">Privacy Mode</h2>
        <p className="mt-1 text-sm text-aiy-text-secondary">
          Configure local-only AI execution and verify privacy posture
        </p>
      </div>

      {/* UI Mode Toggle (persisted in Electron) */}
      <div className="card space-y-3">
        <h3 className="text-sm font-medium text-aiy-text-muted">UI Privacy Mode</h3>
        <div className="flex items-center gap-3">
          <button
            onClick={() => setMode('always-local')}
            className={`flex-1 rounded-lg border p-3 transition-colors ${
              mode === 'always-local'
                ? 'border-privacy-local bg-privacy-local/10'
                : 'border-aiy-border hover:border-aiy-border/80'
            }`}
          >
            <div className="text-sm font-medium text-aiy-text-primary">Always Local</div>
            <div className="text-xs text-aiy-text-muted">No cloud access</div>
          </button>
          <button
            onClick={() => setMode('hybrid')}
            className={`flex-1 rounded-lg border p-3 transition-colors ${
              mode === 'hybrid'
                ? 'border-yellow-500 bg-yellow-500/10'
                : 'border-aiy-border hover:border-aiy-border/80'
            }`}
          >
            <div className="text-sm font-medium text-aiy-text-primary">Hybrid</div>
            <div className="text-xs text-aiy-text-muted">Local + cloud with consent</div>
          </button>
        </div>
      </div>

      {/* CLI Actions */}
      <div className="card space-y-3">
        <h3 className="text-sm font-medium text-aiy-text-muted">CLI Privacy Mode</h3>
        {!repoPath && (
          <p className="text-sm text-yellow-500">
            Select a repository first (Repositories tab)
          </p>
        )}
        <div className="flex flex-wrap gap-2">
          <Button
            onClick={handleStatus}
            disabled={!repoPath || loading === 'status'}
            size="sm"
          >
            {loading === 'status' ? 'Loading...' : 'Show Status'}
          </Button>
          <Button
            onClick={handleCheck}
            disabled={!repoPath || loading === 'check'}
            size="sm"
          >
            {loading === 'check' ? 'Checking...' : 'Verify'}
          </Button>
          <Button
            onClick={handleShowConfig}
            disabled={!repoPath || loading === 'config'}
            size="sm"
            variant="secondary"
          >
            {loading === 'config' ? 'Loading...' : 'Show Config'}
          </Button>
          <Button
            onClick={handleEnable}
            disabled={!repoPath || loading === 'enable'}
            size="sm"
            variant="secondary"
          >
            {loading === 'enable' ? 'Enabling...' : 'Enable'}
          </Button>
          <Button
            onClick={handleDisable}
            disabled={!repoPath || loading === 'disable'}
            size="sm"
            variant="secondary"
          >
            {loading === 'disable' ? 'Disabling...' : 'Disable'}
          </Button>
        </div>

        {/* Output displays */}
        {status && (
          <details className="rounded-lg border border-aiy-border bg-aiy-surface p-3">
            <summary className="cursor-pointer text-sm font-medium text-aiy-text-primary">
              Status Output
            </summary>
            <pre className="mt-2 whitespace-pre-wrap font-mono text-xs text-aiy-text-secondary">
              {status}
            </pre>
          </details>
        )}
        {checkOutput && (
          <details
            className="rounded-lg border border-aiy-border bg-aiy-surface p-3"
            open
          >
            <summary className="cursor-pointer text-sm font-medium text-aiy-text-primary">
              Privacy Check Output
            </summary>
            <pre className="mt-2 whitespace-pre-wrap font-mono text-xs text-aiy-text-secondary">
              {checkOutput}
            </pre>
          </details>
        )}
        {config && (
          <details className="rounded-lg border border-aiy-border bg-aiy-surface p-3">
            <summary className="cursor-pointer text-sm font-medium text-aiy-text-primary">
              Config Output
            </summary>
            <pre className="mt-2 whitespace-pre-wrap font-mono text-xs text-aiy-text-secondary">
              {config}
            </pre>
          </details>
        )}
      </div>
    </div>
  );
}
