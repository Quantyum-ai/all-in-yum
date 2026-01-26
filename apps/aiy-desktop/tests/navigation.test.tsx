/**
 * Navigation Tests
 *
 * Tests for navigation routing between views in the application.
 * Verifies that clicking sidebar items and other navigation triggers
 * correctly routes to the appropriate views.
 */

import { describe, it, expect, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import App from '../src/renderer/App';
import { useAppStore } from '../src/renderer/stores/app-store';
import { usePrivacyStore } from '../src/renderer/stores/privacy-store';
import { useLogsStore } from '../src/renderer/stores/logs-store';

describe('Navigation', () => {
  beforeEach(() => {
    // Reset app store to initial state before each test
    useAppStore.setState({
      currentView: 'dashboard',
      isGlobalLoading: false,
      loadingMessage: undefined,
      error: null,
      toasts: [],
      appVersion: null,
    });

    // Reset privacy store to initial state (complete state to avoid loading)
    usePrivacyStore.setState({
      mode: 'always-local',
      isLoading: false,
      isChanging: false,
      error: null,
    });

    // Reset logs store to initial state
    useLogsStore.setState({
      lines: [],
      currentJobId: null,
      isRunning: false,
      searchFilter: '',
      levelFilter: 'all',
      autoScrollEnabled: true,
    });
  });

  it('should start on dashboard view', async () => {
    render(<App />);

    // Dashboard is the default view - check for Dashboard heading
    await waitFor(() => {
      expect(screen.getByText('Dashboard')).toBeInTheDocument();
    });
  });

  it('should navigate to Privacy view when sidebar item clicked', async () => {
    render(<App />);

    const privacyButton = screen.getByRole('button', { name: /privacy mode/i });
    fireEvent.click(privacyButton);

    await waitFor(() => {
      // Privacy view has unique description text
      expect(screen.getByText(/Configure local-only AI execution/i)).toBeInTheDocument();
    });
  });

  it('should navigate to Repositories view', async () => {
    render(<App />);

    const reposButton = screen.getByRole('button', { name: /repositories/i });
    fireEvent.click(reposButton);

    await waitFor(() => {
      expect(screen.getByRole('heading', { name: /repositories/i })).toBeInTheDocument();
    });
  });

  it('should navigate to Agents view', async () => {
    render(<App />);

    const agentsButton = screen.getByRole('button', { name: /agents/i });
    fireEvent.click(agentsButton);

    await waitFor(() => {
      expect(screen.getByRole('heading', { name: /agents/i })).toBeInTheDocument();
    });
  });

  it('should navigate to Logs view', async () => {
    render(<App />);

    const logsButton = screen.getByRole('button', { name: /^logs$/i });
    fireEvent.click(logsButton);

    await waitFor(() => {
      expect(screen.getByText('Command Logs')).toBeInTheDocument();
      expect(screen.getByText('Open Logs Folder')).toBeInTheDocument();
    });
  });

  it('should navigate when PrivacyBadge is clicked', async () => {
    render(<App />);

    // Wait for privacy badge to load and render
    const badge = await waitFor(() => screen.getByText(/Always Local|Hybrid/));
    fireEvent.click(badge);

    await waitFor(() => {
      // Should navigate to privacy view
      expect(screen.getByText(/Configure local-only AI execution/i)).toBeInTheDocument();
    });
  });

  it('should highlight active navigation item', async () => {
    render(<App />);

    // Dashboard should be highlighted initially
    const dashboardButton = screen.getByRole('button', { name: /dashboard/i });
    expect(dashboardButton).toHaveClass('bg-aiy-primary/10');

    // Navigate to Logs
    const logsButton = screen.getByRole('button', { name: /^logs$/i });
    fireEvent.click(logsButton);

    await waitFor(() => {
      // Logs should now be highlighted
      expect(logsButton).toHaveClass('bg-aiy-primary/10');
      // Dashboard should no longer be highlighted
      expect(dashboardButton).not.toHaveClass('bg-aiy-primary/10');
    });
  });
});
