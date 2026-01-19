import { app, BrowserWindow, shell } from 'electron';
import path from 'path';
import { registerIpcHandlers, registerAppLifecycleHandlers } from './ipc-handlers';
import { WindowManager } from './window-manager';

// Security: Disable hardware acceleration (optional, can be removed if GPU rendering is needed)
// app.disableHardwareAcceleration();

let windowManager: WindowManager | null = null;

function createWindow(): void {
  // Create the browser window with secure settings
  const mainWindow = new BrowserWindow({
    width: 1200,
    height: 800,
    minWidth: 800,
    minHeight: 600,
    show: false,
    autoHideMenuBar: true,
    title: 'AIY Desktop',
    backgroundColor: '#0f172a',
    webPreferences: {
      // CRITICAL SECURITY SETTINGS
      nodeIntegration: false,
      contextIsolation: true,
      sandbox: true,
      preload: path.join(__dirname, '../preload/index.js'),
      // Additional security hardening
      webSecurity: true,
      allowRunningInsecureContent: false,
      experimentalFeatures: false,
    },
  });

  // Initialize window manager
  windowManager = new WindowManager(mainWindow);

  // Register IPC handlers
  registerIpcHandlers();

  // Register app lifecycle handlers (cleanup on quit, etc.)
  registerAppLifecycleHandlers();

  // Prevent navigation to external URLs
  mainWindow.webContents.on('will-navigate', (event, url) => {
    // Allow only internal navigation
    if (!url.startsWith('file://') && !url.startsWith('http://localhost')) {
      event.preventDefault();
      console.warn(`Blocked navigation to: ${url}`);
    }
  });

  // Handle external links securely
  mainWindow.webContents.setWindowOpenHandler(({ url }) => {
    // Only allow opening URLs from allowlist
    const allowedDomains = [
      'github.com',
      'docs.allyum.dev',
    ];

    try {
      const urlObj = new URL(url);
      if (allowedDomains.some(domain => urlObj.hostname.endsWith(domain))) {
        shell.openExternal(url);
      } else {
        console.warn(`Blocked external URL: ${url}`);
      }
    } catch {
      console.warn(`Invalid URL: ${url}`);
    }

    return { action: 'deny' };
  });

  // Show window when ready
  mainWindow.on('ready-to-show', () => {
    mainWindow.show();
  });

  // Load the app
  if (process.env.ELECTRON_RENDERER_URL) {
    mainWindow.loadURL(process.env.ELECTRON_RENDERER_URL);
  } else {
    mainWindow.loadFile(path.join(__dirname, '../renderer/index.html'));
  }

  // Open DevTools in development
  if (process.env.NODE_ENV === 'development') {
    mainWindow.webContents.openDevTools();
  }
}

// This method will be called when Electron has finished initialization
app.whenReady().then(() => {
  createWindow();

  // On macOS, re-create window when dock icon is clicked
  app.on('activate', () => {
    if (BrowserWindow.getAllWindows().length === 0) {
      createWindow();
    }
  });
});

// Quit when all windows are closed (except on macOS)
app.on('window-all-closed', () => {
  if (process.platform !== 'darwin') {
    app.quit();
  }
});

// Security: Prevent new window creation from renderer
app.on('web-contents-created', (_, contents) => {
  contents.on('will-attach-webview', (event) => {
    event.preventDefault();
    console.warn('Blocked webview creation');
  });
});
