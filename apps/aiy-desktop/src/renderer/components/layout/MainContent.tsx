import React from 'react';
import { useAppStore } from '../../stores/app-store';
import { DashboardView } from '../../views/DashboardView';
import { PrivacyView } from '../../views/PrivacyView';
import { RepositoriesView } from '../../views/RepositoriesView';
import { AgentsView } from '../../views/AgentsView';
import { LogsView } from '../../views/LogsView';
import { LogsPanel } from '../logs';

/**
 * Main Content Area Component
 *
 * This is the primary workspace area where the main UI content is rendered.
 * Routes to different views based on currentView from app store.
 *
 * Layout includes:
 * - For logs view: Full height logs panel
 * - For other views: Split layout (content top, logs bottom)
 */
export function MainContent(): React.ReactElement {
  const { currentView } = useAppStore();

  // Logs view takes full height
  if (currentView === 'logs') {
    return (
      <main className="flex flex-1 flex-col overflow-hidden bg-aiy-background">
        <LogsView />
      </main>
    );
  }

  // Other views: split layout (content top, logs bottom)
  return (
    <main className="flex flex-1 flex-col overflow-hidden bg-aiy-background">
      {/* Main workspace area - top 2/3 */}
      <div className="flex-1 overflow-auto p-6" style={{ minHeight: '40%' }}>
        {currentView === 'dashboard' && <DashboardView />}
        {currentView === 'privacy' && <PrivacyView />}
        {currentView === 'repos' && <RepositoriesView />}
        {currentView === 'agents' && <AgentsView />}
      </div>

      {/* Logs panel - bottom 1/3 */}
      <div className="h-1/3 min-h-[200px] border-t border-aiy-border p-3">
        <LogsPanel />
      </div>
    </main>
  );
}
