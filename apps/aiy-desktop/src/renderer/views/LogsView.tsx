import React from 'react';
import { LogsPanel } from '../components/logs';
import { Button } from '../components/common/Button';

/**
 * Logs View Component
 *
 * Full-height view for viewing CLI command logs with:
 * - Header with title and Open Logs Folder button
 * - Full-height logs panel with search/filter controls
 */
export function LogsView(): React.ReactElement {
  const handleOpenLogsFolder = async (): Promise<void> => {
    try {
      await window.electronAPI.app.openLogsFolder();
    } catch (error) {
      console.error('Failed to open logs folder:', error);
    }
  };

  return (
    <div className="flex h-full flex-col">
      {/* Header bar */}
      <div className="border-b border-aiy-border bg-aiy-surface px-4 py-3">
        <div className="flex items-center justify-between">
          <div>
            <h2 className="text-lg font-semibold text-aiy-text-primary">Command Logs</h2>
            <p className="text-xs text-aiy-text-muted">Real-time CLI output and history</p>
          </div>
          <Button size="sm" variant="secondary" onClick={handleOpenLogsFolder}>
            <svg className="mr-2 h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z"
              />
            </svg>
            Open Logs Folder
          </Button>
        </div>
      </div>

      {/* Full-height logs panel */}
      <div className="flex-1 overflow-hidden p-3">
        <LogsPanel />
      </div>
    </div>
  );
}
