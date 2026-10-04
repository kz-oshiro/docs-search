import { defineConfig } from '@playwright/test';
import path from 'node:path';

const artifacts = process.env.DOCS_SEARCH_EXE_ARTIFACTS;
if (!artifacts || !process.env.DOCS_SEARCH_EXE) {
  throw new Error('Run cargo xtask exe, test or ci to build and allocate this run.');
}

export default defineConfig({
  testDir: './exe',
  testMatch: '**/*.spec.mjs',
  fullyParallel: false,
  workers: 1,
  retries: 0,
  timeout: 60_000,
  expect: { timeout: 10_000 },
  forbidOnly: true,
  outputDir: path.join(artifacts, 'test-results'),
  reporter: [['list'], ['json', { outputFile: path.join(artifacts, 'results.json') }]],
  use: { screenshot: 'off', video: 'off', trace: 'off' },
  captureGitInfo: { commit: false, diff: false },
});
