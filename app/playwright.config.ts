import { defineConfig, devices } from '@playwright/test';

// End-to-end tests run against the production build (`npm run build`).
// Set CHROMIUM to use an already installed Chromium binary.
const executablePath = process.env.CHROMIUM || undefined;

export default defineConfig({
  testDir: 'e2e',
  timeout: 180_000,
  expect: { timeout: 20_000 },
  fullyParallel: false,
  reporter: [['list']],
  use: {
    baseURL: 'http://localhost:4173',
    trace: 'retain-on-failure',
    launchOptions: { executablePath },
    // Screens fade in; screenshots should not catch them halfway.
    contextOptions: { reducedMotion: 'reduce' },
  },
  projects: [
    { name: 'ipad-landscape', use: { ...devices['iPad Pro 11 landscape'], browserName: 'chromium' } },
    { name: 'desktop', use: { viewport: { width: 1440, height: 900 }, browserName: 'chromium' } },
  ],
  webServer: {
    command: 'npx vite preview --port 4173 --strictPort',
    port: 4173,
    reuseExistingServer: !process.env.CI,
  },
});
