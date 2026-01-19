import React from 'react';
import { Button } from '../common/Button';
import { LogsPanel } from '../logs';

/**
 * Main Content Area Component
 *
 * This is the primary workspace area where the main UI content is rendered.
 * Layout includes:
 * - Top section (2/3): Main workspace with welcome screen
 * - Bottom section (1/3): Logs panel for CLI output
 */
export function MainContent(): React.ReactElement {
  return (
    <main className="flex flex-1 flex-col overflow-hidden bg-aiy-background">
      {/* Main workspace area - top 2/3 */}
      <div className="flex-1 overflow-auto p-6" style={{ minHeight: '40%' }}>
        {/* Welcome card */}
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
              <h2 className="text-2xl font-bold text-aiy-text-primary">
                Welcome to AIY Desktop
              </h2>
              <p className="mt-2 text-aiy-text-secondary">
                Privacy-first AI-powered code assistance. Your code stays local by default.
              </p>
            </div>

            {/* Quick actions */}
            <div className="space-y-3">
              <h3 className="text-sm font-medium text-aiy-text-muted">Quick Actions</h3>
              <div className="grid gap-3 sm:grid-cols-2">
                <Button variant="secondary" className="justify-start">
                  <svg className="mr-2 h-5 w-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path
                      strokeLinecap="round"
                      strokeLinejoin="round"
                      strokeWidth={2}
                      d="M12 10v6m0 0l-3-3m3 3l3-3m2 8H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"
                    />
                  </svg>
                  Open Repository
                </Button>
                <Button variant="secondary" className="justify-start">
                  <svg className="mr-2 h-5 w-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path
                      strokeLinecap="round"
                      strokeLinejoin="round"
                      strokeWidth={2}
                      d="M12 6v6m0 0v6m0-6h6m-6 0H6"
                    />
                  </svg>
                  Initialize Privacy Mode
                </Button>
              </div>
            </div>

            {/* Status section */}
            <div className="rounded-lg border border-aiy-border bg-aiy-background p-4">
              <div className="flex items-center justify-between">
                <div>
                  <p className="text-sm font-medium text-aiy-text-primary">Privacy Mode</p>
                  <p className="text-xs text-aiy-text-muted">
                    All AI processing happens locally by default
                  </p>
                </div>
                <div className="flex items-center gap-2 rounded-full bg-privacy-local/20 px-3 py-1">
                  <span className="h-2 w-2 rounded-full bg-privacy-local" />
                  <span className="text-sm font-medium text-privacy-local">Always Local</span>
                </div>
              </div>
            </div>

            {/* Placeholder for recent activity */}
            <div>
              <h3 className="mb-3 text-sm font-medium text-aiy-text-muted">Recent Activity</h3>
              <div className="flex items-center justify-center rounded-lg border border-dashed border-aiy-border py-8">
                <p className="text-sm text-aiy-text-muted">No recent activity</p>
              </div>
            </div>
          </div>
        </div>
      </div>

      {/* Logs panel - bottom 1/3 */}
      <div className="h-1/3 min-h-[200px] border-t border-aiy-border p-3">
        <LogsPanel />
      </div>
    </main>
  );
}
