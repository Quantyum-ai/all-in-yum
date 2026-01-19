/**
 * Renderer-specific type definitions
 *
 * This file re-exports shared types and defines renderer-specific types
 * used throughout the React application.
 */

// Re-export shared types
export type {
  OutputChunk,
  JobStatus,
  JobInfo,
  PrivacyMode,
  AppSettings,
} from '../../shared/ipc-channels';

export { DEFAULT_SETTINGS } from '../../shared/ipc-channels';

/**
 * Navigation item for sidebar
 */
export interface NavItem {
  id: string;
  label: string;
  icon: React.ReactNode;
  path?: string;
  badge?: string | number;
}

/**
 * Repository info for display
 */
export interface RepoInfo {
  path: string;
  name: string;
  branch?: string;
  isPrivacyEnabled?: boolean;
  lastAccessed?: Date;
}

/**
 * Theme options
 */
export type Theme = 'light' | 'dark' | 'system';

/**
 * View/route definitions
 */
export type AppView =
  | 'dashboard'
  | 'privacy'
  | 'repos'
  | 'agents'
  | 'logs'
  | 'settings';

/**
 * Terminal output line with metadata
 */
export interface TerminalLine {
  id: string;
  type: 'stdout' | 'stderr' | 'info' | 'error' | 'command';
  content: string;
  timestamp: Date;
}

/**
 * Command history entry
 */
export interface CommandHistoryEntry {
  id: string;
  command: string;
  args: string[];
  timestamp: Date;
  exitCode: number | null;
  duration?: number;
}

/**
 * Modal/dialog state
 */
export interface ModalState {
  isOpen: boolean;
  title?: string;
  content?: React.ReactNode;
  onConfirm?: () => void;
  onCancel?: () => void;
}

/**
 * Dropdown/menu option
 */
export interface MenuOption {
  id: string;
  label: string;
  icon?: React.ReactNode;
  shortcut?: string;
  disabled?: boolean;
  danger?: boolean;
  onClick?: () => void;
}
