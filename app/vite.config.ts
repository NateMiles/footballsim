import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';

// Relative asset paths, so the same build works from any web host, from
// a subdirectory, and inside the Tauri shell.
export default defineConfig({
  base: './',
  plugins: [svelte()],
  worker: { format: 'es' },
  build: { target: 'safari16', sourcemap: true },
  server: { port: 5173, strictPort: true },
  clearScreen: false,
  test: { include: ['src/**/*.test.ts'] },
});
