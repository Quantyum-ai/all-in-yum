import { create } from 'zustand';

/**
 * Application-wide state store
 *
 * Manages global application state including:
 * - Current view/route
 * - Loading states
 * - Error states
 * - Toast/notification queue
 */

export interface Toast {
  id: string;
  type: 'info' | 'success' | 'warning' | 'error';
  title: string;
  message?: string;
  duration?: number;
}

export interface AppState {
  // Current active view/route
  currentView: string;

  // Global loading state
  isGlobalLoading: boolean;
  loadingMessage?: string;

  // Error state
  error: Error | null;

  // Toast notifications
  toasts: Toast[];

  // App version
  appVersion: string | null;

  // Actions
  setCurrentView: (view: string) => void;
  setGlobalLoading: (loading: boolean, message?: string) => void;
  setError: (error: Error | null) => void;
  addToast: (toast: Omit<Toast, 'id'>) => void;
  removeToast: (id: string) => void;
  clearToasts: () => void;
  loadAppVersion: () => Promise<void>;
}

/**
 * Generate unique ID for toasts
 */
function generateId(): string {
  return `toast-${Date.now()}-${Math.random().toString(36).slice(2, 9)}`;
}

/**
 * App store with global state management
 */
export const useAppStore = create<AppState>((set, get) => ({
  // Initial state
  currentView: 'dashboard',
  isGlobalLoading: false,
  loadingMessage: undefined,
  error: null,
  toasts: [],
  appVersion: null,

  // Actions
  setCurrentView: (view) => {
    set({ currentView: view });
  },

  setGlobalLoading: (loading, message) => {
    set({
      isGlobalLoading: loading,
      loadingMessage: message,
    });
  },

  setError: (error) => {
    set({ error });
    if (error) {
      get().addToast({
        type: 'error',
        title: 'Error',
        message: error.message,
      });
    }
  },

  addToast: (toast) => {
    const id = generateId();
    const newToast: Toast = {
      ...toast,
      id,
      duration: toast.duration ?? 5000,
    };

    set((state) => ({
      toasts: [...state.toasts, newToast],
    }));

    // Auto-remove toast after duration
    if (newToast.duration && newToast.duration > 0) {
      setTimeout(() => {
        get().removeToast(id);
      }, newToast.duration);
    }
  },

  removeToast: (id) => {
    set((state) => ({
      toasts: state.toasts.filter((t) => t.id !== id),
    }));
  },

  clearToasts: () => {
    set({ toasts: [] });
  },

  loadAppVersion: async () => {
    try {
      const version = await window.electronAPI.app.getVersion();
      set({ appVersion: version });
    } catch (error) {
      console.error('Failed to load app version:', error);
    }
  },
}));
