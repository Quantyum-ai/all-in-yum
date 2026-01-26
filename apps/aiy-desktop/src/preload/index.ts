/**
 * Preload Script - Bridge between main process and renderer
 *
 * This script runs in a privileged context with access to Node.js APIs,
 * but exposes only a limited, type-safe API to the renderer process
 * via contextBridge.
 *
 * SECURITY: Only expose what's absolutely necessary
 */

import { contextBridge } from 'electron';
import { electronAPI } from './api';

// Expose the API to the renderer process
// This will be available as window.electronAPI
contextBridge.exposeInMainWorld('electronAPI', electronAPI);

// Type augmentation for window object
declare global {
  interface Window {
    electronAPI: typeof electronAPI;
  }
}
