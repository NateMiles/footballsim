import { defineConfig } from 'vitest/config';
import { svelte } from '@sveltejs/vite-plugin-svelte';

// Set by `tauri ios dev` / `tauri android dev` so a device can reach the
// dev server over the network.
const tauriHost = process.env.TAURI_DEV_HOST;

// Relative asset paths, so the same build works from any web host, from
// a subdirectory, and inside the Tauri shell.
export default defineConfig({
  base: './',
  plugins: [svelte()],
  worker: { format: 'es' },
  build: { target: 'safari16', sourcemap: true },
  server: {
    port: 5173,
    strictPort: true,
    host: tauriHost || false,
    hmr: tauriHost ? { protocol: 'ws', host: tauriHost, port: 5174 } : undefined,
    watch: { ignored: ['**/src-tauri/**'] },
  },
  clearScreen: false,
  test: { include: ['src/**/*.test.ts'] },
});
