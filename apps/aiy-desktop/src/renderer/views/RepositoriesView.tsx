import React from 'react';
import { Button } from '../components/common/Button';
import { useRepoStore } from '../stores/repo-store';
import { useAppStore } from '../stores/app-store';

/**
 * Repositories View
 *
 * Manages repository selection and CLI binary configuration:
 * - Select a repository directory via native file picker
 * - Auto-initialize privacy mode when selecting a new repo
 * - Configure custom CLI binary path
 */
export function RepositoriesView(): React.ReactElement {
  const { repoPath, setRepoPath } = useRepoStore();
  const { addToast } = useAppStore();
  const [isInitializing, setIsInitializing] = React.useState(false);
  const [binaryPath, setBinaryPath] = React.useState<string | null>(null);
  const [initStatus, setInitStatus] = React.useState<{
    success: boolean;
    message: string;
  } | null>(null);

  // Load binary path from settings on mount
  React.useEffect(() => {
    window.electronAPI.settings.get<string>('cliBinaryPath').then((path) => {
      setBinaryPath(path || null);
    });
  }, []);

  /**
   * Open native directory picker and select a repository
   */
  const handleOpenRepo = async () => {
    try {
      const result = await window.electronAPI.app.selectDirectory({
        title: 'Select Repository',
      });

      if (!result.cancelled && result.path) {
        setRepoPath(result.path);
        setInitStatus(null);

        // Auto-init privacy mode in the repo
        setIsInitializing(true);
        try {
          const jobId = await window.electronAPI.cli.run('privacy', ['init']);

          // Collect output for status message
          const outputs: string[] = [];
          const cleanupOutput = window.electronAPI.cli.onOutput((jid, chunk) => {
            if (jid === jobId) {
              outputs.push(chunk.text);
            }
          });

          // Wait for completion
          const cleanupExit = window.electronAPI.cli.onExit((jid, code) => {
            if (jid === jobId) {
              setIsInitializing(false);
              cleanupOutput();
              cleanupExit();

              if (code === 0) {
                setInitStatus({
                  success: true,
                  message: 'Privacy mode initialized successfully',
                });
                addToast({
                  type: 'success',
                  title: 'Repository Ready',
                  message: `Privacy mode initialized in ${result.path}`,
                });
              } else {
                setInitStatus({
                  success: false,
                  message: `Privacy init exited with code ${code}`,
                });
                addToast({
                  type: 'warning',
                  title: 'Privacy Init Warning',
                  message: `Privacy init exited with code ${code}`,
                });
              }
            }
          });
        } catch (error) {
          setIsInitializing(false);
          const message = error instanceof Error ? error.message : 'Unknown error';
          setInitStatus({
            success: false,
            message: `Failed to initialize privacy: ${message}`,
          });
          addToast({
            type: 'error',
            title: 'Privacy Init Failed',
            message,
          });
        }
      }
    } catch (error) {
      console.error('Failed to open repository:', error);
      addToast({
        type: 'error',
        title: 'Failed to Open Repository',
        message: error instanceof Error ? error.message : 'Unknown error',
      });
    }
  };

  /**
   * Open native file picker and select a CLI binary
   */
  const handleSelectBinary = async () => {
    try {
      const result = await window.electronAPI.app.selectCliBinary({
        title: 'Select aiy CLI Binary',
      });

      if (!result.cancelled && result.path) {
        await window.electronAPI.settings.set('cliBinaryPath', result.path);
        setBinaryPath(result.path);
        addToast({
          type: 'success',
          title: 'CLI Binary Updated',
          message: `Using binary at: ${result.path}`,
        });
      }
    } catch (error) {
      console.error('Failed to select binary:', error);
      addToast({
        type: 'error',
        title: 'Failed to Select Binary',
        message: error instanceof Error ? error.message : 'Unknown error',
      });
    }
  };

  /**
   * Reset binary path to auto-detect
   */
  const handleResetBinary = async () => {
    try {
      await window.electronAPI.settings.set('cliBinaryPath', null);
      setBinaryPath(null);
      addToast({
        type: 'info',
        title: 'CLI Binary Reset',
        message: 'Using auto-detected binary',
      });
    } catch (error) {
      console.error('Failed to reset binary:', error);
    }
  };

  return (
    <div className="mx-auto max-w-2xl">
      <div className="card space-y-6">
        {/* Header */}
        <div>
          <h2 className="text-xl font-bold text-aiy-text-primary">Repositories</h2>
          <p className="mt-1 text-sm text-aiy-text-secondary">
            Manage repository selection and CLI configuration
          </p>
        </div>

        {/* Current Repository */}
        <div className="space-y-3">
          <h3 className="text-sm font-medium text-aiy-text-muted">Current Repository</h3>
          <div className="rounded-lg border border-aiy-border bg-aiy-surface p-4">
            <p className="font-mono text-sm text-aiy-text-primary">
              {repoPath || 'No repository selected'}
            </p>
            {repoPath && (
              <p className="mt-2 text-xs text-aiy-text-muted">
                This repository will be used as the working directory for CLI commands.
              </p>
            )}
          </div>
          <div className="flex gap-2">
            <Button onClick={handleOpenRepo} disabled={isInitializing} isLoading={isInitializing}>
              {isInitializing ? 'Initializing...' : 'Open Repository...'}
            </Button>
            {repoPath && (
              <Button variant="ghost" onClick={() => setRepoPath(null)}>
                Clear
              </Button>
            )}
          </div>
        </div>

        {/* Init Status */}
        {initStatus && (
          <div
            className={`rounded-lg border p-3 ${
              initStatus.success
                ? 'border-green-500/50 bg-green-500/10 text-green-300'
                : 'border-red-500/50 bg-red-500/10 text-red-300'
            }`}
          >
            <div className="flex items-center gap-2">
              {initStatus.success ? (
                <svg className="h-5 w-5 flex-shrink-0" fill="currentColor" viewBox="0 0 20 20">
                  <path
                    fillRule="evenodd"
                    d="M10 18a8 8 0 100-16 8 8 0 000 16zm3.707-9.293a1 1 0 00-1.414-1.414L9 10.586 7.707 9.293a1 1 0 00-1.414 1.414l2 2a1 1 0 001.414 0l4-4z"
                    clipRule="evenodd"
                  />
                </svg>
              ) : (
                <svg className="h-5 w-5 flex-shrink-0" fill="currentColor" viewBox="0 0 20 20">
                  <path
                    fillRule="evenodd"
                    d="M18 10a8 8 0 11-16 0 8 8 0 0116 0zm-7 4a1 1 0 11-2 0 1 1 0 012 0zm-1-9a1 1 0 00-1 1v4a1 1 0 102 0V6a1 1 0 00-1-1z"
                    clipRule="evenodd"
                  />
                </svg>
              )}
              <p className="text-sm">{initStatus.message}</p>
            </div>
          </div>
        )}

        {/* CLI Binary Configuration */}
        <div className="space-y-3">
          <h3 className="text-sm font-medium text-aiy-text-muted">CLI Binary</h3>
          <div className="rounded-lg border border-aiy-border bg-aiy-surface p-4">
            <p className="font-mono text-sm text-aiy-text-primary">
              {binaryPath || 'Auto-detect (cargo build -> PATH lookup)'}
            </p>
            <p className="mt-2 text-xs text-aiy-text-muted">
              {binaryPath
                ? 'Using custom binary path. Click "Reset" to auto-detect.'
                : 'The CLI binary is auto-detected from bundled binary, then PATH.'}
            </p>
          </div>
          <div className="flex gap-2">
            <Button variant="secondary" onClick={handleSelectBinary}>
              Select CLI Binary...
            </Button>
            {binaryPath && (
              <Button variant="ghost" onClick={handleResetBinary}>
                Reset to Auto-detect
              </Button>
            )}
          </div>
        </div>

        {/* Help Section */}
        <div className="rounded-lg border border-aiy-border bg-aiy-surface/50 p-4">
          <h4 className="text-sm font-medium text-aiy-text-primary">Getting Started</h4>
          <ul className="mt-2 space-y-1 text-xs text-aiy-text-muted">
            <li>1. Click "Open Repository..." to select your project folder</li>
            <li>2. Privacy mode will be automatically initialized</li>
            <li>3. Use the Dashboard to verify privacy settings</li>
            <li>4. (Optional) Select a custom CLI binary if needed</li>
          </ul>
        </div>
      </div>
    </div>
  );
}
