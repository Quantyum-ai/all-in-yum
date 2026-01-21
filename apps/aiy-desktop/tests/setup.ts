/**
 * Vitest Test Setup
 *
 * This file configures the test environment for the aiy Desktop application.
 * It sets up jsdom, mocks Electron APIs, and configures testing utilities.
 */

import '@testing-library/jest-dom/vitest';
import { vi } from 'vitest';

// Mock electron API for renderer tests
const mockElectronAPI = {
  cli: {
    run: vi.fn().mockResolvedValue('mock-job-id'),
    cancel: vi.fn().mockResolvedValue(undefined),
    onOutput: vi.fn().mockReturnValue(() => {}),
    onExit: vi.fn().mockReturnValue(() => {}),
  },
  settings: {
    get: vi.fn().mockImplementation((key: string) => {
      const defaults: Record<string, unknown> = {
        privacyMode: 'always-local',
        theme: 'dark',
        fontSize: 14,
      };
      return Promise.resolve(defaults[key]);
    }),
    set: vi.fn().mockResolvedValue(undefined),
    onChange: vi.fn().mockReturnValue(() => {}),
  },
  app: {
    openRepoWindow: vi.fn().mockResolvedValue(undefined),
    openExternal: vi.fn().mockResolvedValue({ success: true }),
    getVersion: vi.fn().mockResolvedValue('0.1.0'),
    selectDirectory: vi.fn().mockResolvedValue({ cancelled: true }),
    selectCliBinary: vi.fn().mockResolvedValue({ cancelled: true }),
    openLogsFolder: vi.fn().mockResolvedValue({ success: true }),
  },
};

// Expose mock API globally
Object.defineProperty(window, 'electronAPI', {
  value: mockElectronAPI,
  writable: true,
});

// Reset mocks before each test
beforeEach(() => {
  vi.clearAllMocks();
});

// Export mock for test access
export { mockElectronAPI };
