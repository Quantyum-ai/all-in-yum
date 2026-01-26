import React from 'react';
import { Button } from '../components/common/Button';
import { useAppStore } from '../stores/app-store';
import { useRepoStore } from '../stores/repo-store';
import { parsePrivacyCheck, getCheckSummary } from '../../shared/parsers';

/**
 * Dashboard View
 *
 * The main landing view for the application, showing:
 * - Current repository status
 * - Quick actions for common tasks
 * - Privacy verification functionality
 * - Smoke test for CLI connectivity
 */
export function DashboardView(): React.ReactElement {
  const { setCurrentView, addToast } = useAppStore();
  const { repoPath } = useRepoStore();
  const [checking, setChecking] = React.useState(false);
  const [checkResult, setCheckResult] = React.useState<{
    passed: boolean;
    summary: string;
  } | null>(null);

  /**
   * Run a smoke test by executing 'aiy version' command
   */
  const handleSmokeTest = async () => {
    try {
      const jobId = await window.electronAPI.cli.run('version', []);
      console.log(`Started version check: ${jobId}`);
      addToast({
        type: 'info',
        title: 'Smoke Test Started',
        message: `Running aiy version (Job: ${jobId})`,
      });
    } catch (error) {
      console.error('Failed to run version:', error);
      addToast({
        type: 'error',
        title: 'Smoke Test Failed',
        message: error instanceof Error ? error.message : 'Unknown error',
      });
    }
  };

  /**
   * Verify privacy mode by running 'aiy privacy check'
   */
  const handleVerifyPrivacy = async () => {
    if (!repoPath) {
      addToast({
        type: 'warning',
        title: 'No Repository Selected',
        message: 'Please select a repository first',
      });
      return;
    }

    setChecking(true);
    setCheckResult(null);

    try {
      const jobId = await window.electronAPI.cli.run('privacy', ['check']);
      console.log(`Started privacy check: ${jobId}`);

      // Listen for output
      const outputs: string[] = [];
      const cleanupOutput = window.electronAPI.cli.onOutput((jid, chunk) => {
        if (jid === jobId) {
          outputs.push(chunk.text);
        }
      });

      // Wait for completion
      const cleanupExit = window.electronAPI.cli.onExit((jid, code) => {
        if (jid === jobId) {
          const fullOutput = outputs.join('');
          const result = parsePrivacyCheck(fullOutput);

          if (result.success) {
            const summary = getCheckSummary(result.data);
            setCheckResult({
              passed: result.data.passed,
              summary: result.data.passed
                ? `All privacy checks passed! (${summary})`
                : `Some checks failed: ${summary}`,
            });
          } else {
            setCheckResult({
              passed: false,
              summary: `Failed to parse output: ${result.error?.message || 'Unknown error'}`,
            });
          }

          setChecking(false);
          cleanupOutput();
          cleanupExit();

          // Show toast based on result
          if (result.success && result.data.passed) {
            addToast({
              type: 'success',
              title: 'Privacy Check Passed',
              message: 'All privacy checks passed successfully',
            });
          } else {
            addToast({
              type: 'warning',
              title: 'Privacy Check Completed',
              message: result.success
                ? `${result.data.summary.fail} check(s) failed`
                : 'Failed to parse results',
            });
          }
        }
      });
    } catch (error) {
      const message = error instanceof Error ? error.message : 'Unknown error';
      setCheckResult({
        passed: false,
        summary: `Error: ${message}`,
      });
      setChecking(false);
      addToast({
        type: 'error',
        title: 'Privacy Check Failed',
        message,
      });
    }
  };

  return (
    <div className="mx-auto max-w-2xl">
      <div className="card space-y-6">
        {/* Header */}
        <div className="text-center">
          <div className="mx-auto mb-4 flex h-16 w-16 items-center justify-center rounded-full bg-aiy-primary/20">
            <svg
              className="h-8 w-8 text-aiy-primary"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M13 10V3L4 14h7v7l9-11h-7z"
              />
            </svg>
          </div>
          <h2 className="text-2xl font-bold text-aiy-text-primary">Welcome to AIY Desktop</h2>
          <p className="mt-2 text-aiy-text-secondary">
            Privacy-first AI-powered code assistance. Your code stays local by default.
          </p>
        </div>

        {/* Current Repo */}
        <div className="rounded-lg border border-aiy-border bg-aiy-surface p-4">
          <p className="text-sm font-medium text-aiy-text-primary">Current Repository</p>
          <p className="mt-1 font-mono text-xs text-aiy-text-muted">
            {repoPath || 'No repository selected'}
          </p>
        </div>

        {/* Quick Actions */}
        <div className="space-y-3">
          <h3 className="text-sm font-medium text-aiy-text-muted">Quick Actions</h3>
          <div className="grid gap-3 sm:grid-cols-2">
            <Button variant="secondary" onClick={() => setCurrentView('repos')}>
              Open Repository...
            </Button>
            <Button
              variant="secondary"
              onClick={handleVerifyPrivacy}
              disabled={!repoPath || checking}
              isLoading={checking}
            >
              {checking ? 'Checking...' : 'Verify Privacy'}
            </Button>
            <Button variant="secondary" onClick={() => setCurrentView('privacy')}>
              Privacy Settings
            </Button>
            <Button variant="secondary" onClick={handleSmokeTest}>
              Smoke Test (aiy version)
            </Button>
          </div>
        </div>

        {/* Check Result */}
        {checkResult && (
          <div
            className={`rounded-lg border p-3 ${
              checkResult.passed
                ? 'border-green-500/50 bg-green-500/10 text-green-300'
                : 'border-red-500/50 bg-red-500/10 text-red-300'
            }`}
          >
            <div className="flex items-start gap-2">
              {checkResult.passed ? (
                <svg className="mt-0.5 h-5 w-5 flex-shrink-0" fill="currentColor" viewBox="0 0 20 20">
                  <path
                    fillRule="evenodd"
                    d="M10 18a8 8 0 100-16 8 8 0 000 16zm3.707-9.293a1 1 0 00-1.414-1.414L9 10.586 7.707 9.293a1 1 0 00-1.414 1.414l2 2a1 1 0 001.414 0l4-4z"
                    clipRule="evenodd"
                  />
                </svg>
              ) : (
                <svg className="mt-0.5 h-5 w-5 flex-shrink-0" fill="currentColor" viewBox="0 0 20 20">
                  <path
                    fillRule="evenodd"
                    d="M10 18a8 8 0 100-16 8 8 0 000 16zM8.707 7.293a1 1 0 00-1.414 1.414L8.586 10l-1.293 1.293a1 1 0 101.414 1.414L10 11.414l1.293 1.293a1 1 0 001.414-1.414L11.414 10l1.293-1.293a1 1 0 00-1.414-1.414L10 8.586 8.707 7.293z"
                    clipRule="evenodd"
                  />
                </svg>
              )}
              <p className="text-sm">{checkResult.summary}</p>
            </div>
          </div>
        )}

        {/* Version Info */}
        <div className="text-center text-xs text-aiy-text-muted">
          <p>AIY Desktop - Privacy Mode Enabled</p>
        </div>
      </div>
    </div>
  );
}
