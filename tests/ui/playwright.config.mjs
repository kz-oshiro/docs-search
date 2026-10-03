import { defineConfig } from '@playwright/test';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { validateWorkerPlan } from './workers.mjs';

const here = path.dirname(fileURLToPath(import.meta.url));
const artifacts = process.env.DOCS_SEARCH_UI_ARTIFACTS;
if (!artifacts || !process.env.DOCS_SEARCH_UI_SITE) {
  throw new Error('Run cargo xtask ui to stage the real frontend and allocate a new output directory.');
}
// The preflight measures once. Every worker reads the same immutable decision.
const workers = validateWorkerPlan(JSON.parse(fs.readFileSync(path.join(artifacts, 'workers.json'), 'utf8')));

export default defineConfig({
  testDir: './specs',
  testMatch: '**/*.spec.mjs',
  fullyParallel: false,
  workers,
  retries: 0,
  timeout: 30_000,
  expect: { timeout: 5_000 },
  forbidOnly: true,
  outputDir: path.join(artifacts, 'test-results'),
  reporter: [['list'], ['json', { outputFile: path.join(artifacts, 'results.json') }]],
  use: {
    browserName: 'chromium',
    headless: true,
    baseURL: 'http://127.0.0.1:43861',
    viewport: { width: 1100, height: 800 },
    locale: 'ja-JP',
    timezoneId: 'Asia/Tokyo',
    screenshot: 'off',
    video: 'off',
    trace: 'off',
  },
  captureGitInfo: { commit: false, diff: false },
  webServer: {
    command: 'node server.mjs',
    cwd: here,
    url: 'http://127.0.0.1:43861/__health',
    reuseExistingServer: false,
    timeout: 15_000,
  },
});
