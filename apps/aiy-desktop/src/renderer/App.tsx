import React, { useEffect } from 'react';
import { Header } from './components/layout/Header';
import { Sidebar } from './components/layout/Sidebar';
import { MainContent } from './components/layout/MainContent';
import { usePrivacyStore } from './stores/privacy-store';

/**
 * Root Application Component
 *
 * Provides the main layout structure with:
 * - Header (with privacy badge)
 * - Sidebar (navigation)
 * - Main content area
 */
function App(): React.ReactElement {
  const { loadPrivacyMode } = usePrivacyStore();

  // Load privacy mode from settings on mount
  useEffect(() => {
    loadPrivacyMode();
  }, [loadPrivacyMode]);

  return (
    <div className="flex h-screen flex-col overflow-hidden">
      {/* Header - Fixed at top */}
      <Header />

      {/* Main layout - Sidebar + Content */}
      <div className="flex flex-1 overflow-hidden">
        {/* Sidebar - Fixed width */}
        <Sidebar />

        {/* Main content - Flexible */}
        <MainContent />
      </div>
    </div>
  );
}

export default App;
