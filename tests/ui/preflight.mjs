import fs from 'node:fs';
import path from 'node:path';
import { createRequire } from 'node:module';
import { measureWorkerPlan } from './workers.mjs';
const require = createRequire(import.meta.url);
try {
  const expected = require('./package.json').devDependencies['@playwright/test'];
  if (require('@playwright/test/package.json').version !== expected) throw new Error('Playwright version mismatch');
  const executable = require('@playwright/test').chromium.executablePath();
  if (!fs.existsSync(executable)) throw new Error('Chromium is not installed');
  console.log(`Playwright ${expected}; Chromium ${executable}`);
  const artifacts = process.env.DOCS_SEARCH_UI_ARTIFACTS;
  if (artifacts) {
    const plan = await measureWorkerPlan();
    fs.writeFileSync(path.join(artifacts, 'workers.json'), JSON.stringify(plan, null, 2) + '\n');
    const load = plan.peakCpuUsage === null ? 'unavailable' : `${(plan.peakCpuUsage * 100).toFixed(1)}%`;
    console.log(`UI workers: ${plan.workers}; peak CPU usage: ${load}; available CPUs: ${plan.availableCpuCount}`);
    if (plan.fallbackReason) console.log(`Worker fallback: ${plan.fallbackReason}`);
  }
} catch (error) {
  console.error(error.message);
  process.exitCode = 1;
}
