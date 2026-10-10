import { inTauri } from './api/tauri-transport';

/**
 * Registers the service worker that lets the installed web app (Add to
 * Home Screen on iPad) start offline. Only for production web builds:
 * not in development, and not inside the Tauri shell.
 */
export function registerServiceWorker(): void {
  if (import.meta.env.DEV || inTauri() || !('serviceWorker' in navigator)) return;
  addEventListener('load', () => {
    navigator.serviceWorker.register('./sw.js').catch((err) => console.warn('Service worker failed', err));
  });
}
