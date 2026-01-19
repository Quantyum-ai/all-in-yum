/**
 * Window Manager - Manages application windows lifecycle
 *
 * Responsibilities:
 * - Track open windows
 * - Handle window state (minimize, maximize, close)
 * - Manage multi-window scenarios (e.g., per-repo windows)
 * - Persist window bounds/state
 */

import { BrowserWindow, screen } from 'electron';
import Store from 'electron-store';

interface WindowState {
  x: number;
  y: number;
  width: number;
  height: number;
  isMaximized: boolean;
}

const windowStateStore = new Store<{ windowState: WindowState }>({
  name: 'window-state',
  defaults: {
    windowState: {
      x: 0,
      y: 0,
      width: 1200,
      height: 800,
      isMaximized: false,
    },
  },
});

export class WindowManager {
  private mainWindow: BrowserWindow;
  private repoWindows: Map<string, BrowserWindow> = new Map();

  constructor(mainWindow: BrowserWindow) {
    this.mainWindow = mainWindow;
    this.setupWindowStateHandlers();
    this.restoreWindowState();
  }

  /**
   * Setup handlers to save window state on changes
   */
  private setupWindowStateHandlers(): void {
    // Save state when window is moved or resized
    const saveState = (): void => {
      if (this.mainWindow.isDestroyed()) return;

      const bounds = this.mainWindow.getBounds();
      const isMaximized = this.mainWindow.isMaximized();

      windowStateStore.set('windowState', {
        ...bounds,
        isMaximized,
      });
    };

    this.mainWindow.on('resize', saveState);
    this.mainWindow.on('move', saveState);
    this.mainWindow.on('close', saveState);
  }

  /**
   * Restore window state from storage
   */
  private restoreWindowState(): void {
    const state = windowStateStore.get('windowState');

    // Ensure window is on screen
    const displays = screen.getAllDisplays();
    const isOnScreen = displays.some((display) => {
      const { x, y, width, height } = display.bounds;
      return (
        state.x >= x &&
        state.y >= y &&
        state.x + state.width <= x + width &&
        state.y + state.height <= y + height
      );
    });

    if (isOnScreen) {
      this.mainWindow.setBounds({
        x: state.x,
        y: state.y,
        width: state.width,
        height: state.height,
      });
    }

    if (state.isMaximized) {
      this.mainWindow.maximize();
    }
  }

  /**
   * Get the main window
   */
  getMainWindow(): BrowserWindow {
    return this.mainWindow;
  }

  /**
   * Open a new window for a repository
   *
   * TODO: Implement full repo window creation
   */
  openRepoWindow(repoPath: string): BrowserWindow | null {
    // Check if window already exists for this repo
    if (this.repoWindows.has(repoPath)) {
      const existingWindow = this.repoWindows.get(repoPath)!;
      existingWindow.focus();
      return existingWindow;
    }

    // TODO: Create new window with repo-specific context
    // const repoWindow = new BrowserWindow({
    //   width: 1200,
    //   height: 800,
    //   webPreferences: {
    //     nodeIntegration: false,
    //     contextIsolation: true,
    //     sandbox: true,
    //     preload: path.join(__dirname, '../preload/index.js'),
    //   },
    // });

    console.log(`[WindowManager] Would open window for repo: ${repoPath}`);
    return null;
  }

  /**
   * Close a repo window
   */
  closeRepoWindow(repoPath: string): void {
    const window = this.repoWindows.get(repoPath);
    if (window && !window.isDestroyed()) {
      window.close();
    }
    this.repoWindows.delete(repoPath);
  }

  /**
   * Get all open windows
   */
  getAllWindows(): BrowserWindow[] {
    const windows = [this.mainWindow, ...this.repoWindows.values()];
    return windows.filter((w) => !w.isDestroyed());
  }

  /**
   * Focus the main window
   */
  focusMainWindow(): void {
    if (!this.mainWindow.isDestroyed()) {
      if (this.mainWindow.isMinimized()) {
        this.mainWindow.restore();
      }
      this.mainWindow.focus();
    }
  }
}
