import { create } from 'zustand';
import type { PrivacyMode } from '../../shared/ipc-channels';

/**
 * Privacy Mode Store
 *
 * Manages privacy mode state. The privacy mode determines how AI processing
 * is handled:
 *
 * - "always-local" (DEFAULT): All AI processing happens locally. No data
 *   is sent to external AI providers. This is the most private option.
 *
 * - "hybrid": Local models are used when possible, but external AI providers
 *   may be used for certain operations with user consent.
 *
 * CRITICAL: The default mode MUST be "always-local". This ensures user privacy
 * by default and requires explicit user action to enable hybrid mode.
 */

export interface PrivacyState {
  /** Current privacy mode - DEFAULTS TO 'always-local' */
  mode: PrivacyMode;

  /** Whether the privacy mode is being loaded from settings */
  isLoading: boolean;

  /** Whether a mode change is in progress */
  isChanging: boolean;

  /** Error from last operation */
  error: string | null;

  // Actions
  setMode: (mode: PrivacyMode) => Promise<void>;
  loadPrivacyMode: () => Promise<void>;
}

/**
 * Privacy mode store
 *
 * IMPORTANT: Default mode is 'always-local' for maximum privacy by default.
 * Users must explicitly opt-in to hybrid mode.
 */
export const usePrivacyStore = create<PrivacyState>((set, get) => ({
  // Initial state - CRITICAL: Default to 'always-local'
  mode: 'always-local',
  isLoading: true,
  isChanging: false,
  error: null,

  /**
   * Load privacy mode from persistent settings
   */
  loadPrivacyMode: async () => {
    set({ isLoading: true, error: null });

    try {
      const savedMode = await window.electronAPI.settings.get<PrivacyMode>('privacyMode');

      // If no saved mode, keep the default ('always-local')
      // This ensures new users start with maximum privacy
      if (savedMode === 'always-local' || savedMode === 'hybrid') {
        set({ mode: savedMode, isLoading: false });
      } else {
        // Invalid or missing mode - default to 'always-local'
        console.warn(`Invalid privacy mode in settings: ${savedMode}, defaulting to 'always-local'`);
        set({ mode: 'always-local', isLoading: false });

        // Persist the default
        await window.electronAPI.settings.set('privacyMode', 'always-local');
      }
    } catch (error) {
      console.error('Failed to load privacy mode:', error);
      set({
        isLoading: false,
        error: 'Failed to load privacy settings',
        // Keep default mode on error
        mode: 'always-local',
      });
    }
  },

  /**
   * Change privacy mode (requires explicit user action)
   */
  setMode: async (newMode: PrivacyMode) => {
    const currentMode = get().mode;

    // No-op if mode is the same
    if (currentMode === newMode) {
      return;
    }

    set({ isChanging: true, error: null });

    try {
      // Persist to settings
      await window.electronAPI.settings.set('privacyMode', newMode);

      // Update local state
      set({ mode: newMode, isChanging: false });

      console.log(`Privacy mode changed: ${currentMode} -> ${newMode}`);
    } catch (error) {
      console.error('Failed to change privacy mode:', error);
      set({
        isChanging: false,
        error: 'Failed to change privacy mode',
      });
    }
  },
}));
