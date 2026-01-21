import { create } from 'zustand';

/**
 * Repository Store
 *
 * Manages the current repository path state. This is used by components
 * to determine which repository to operate on when running CLI commands.
 *
 * The repository path is passed to CLI commands as the working directory.
 */

export interface RepoState {
  /** Current selected repository path */
  repoPath: string | null;

  /** Whether the repo path is being loaded from settings */
  isLoading: boolean;

  /** Error from last operation */
  error: string | null;

  // Actions
  setRepoPath: (path: string | null) => void;
  loadRepoPath: () => Promise<void>;
}

/**
 * Repository store for managing current working directory
 */
export const useRepoStore = create<RepoState>((set) => ({
  // Initial state
  repoPath: null,
  isLoading: true,
  error: null,

  /**
   * Set the current repository path
   */
  setRepoPath: (path: string | null) => {
    set({ repoPath: path, error: null });

    // Persist to settings (fire and forget)
    if (path) {
      window.electronAPI.settings.set('lastRepoPath', path).catch((err) => {
        console.error('Failed to save repo path:', err);
      });
    }
  },

  /**
   * Load repository path from persistent settings
   */
  loadRepoPath: async () => {
    set({ isLoading: true, error: null });

    try {
      const savedPath = await window.electronAPI.settings.get<string>('lastRepoPath');

      if (savedPath && typeof savedPath === 'string') {
        set({ repoPath: savedPath, isLoading: false });
      } else {
        set({ repoPath: null, isLoading: false });
      }
    } catch (error) {
      console.error('Failed to load repo path:', error);
      set({
        isLoading: false,
        error: 'Failed to load repository path',
        repoPath: null,
      });
    }
  },
}));
