import fs from 'node:fs';
import { createRequire } from 'node:module';
const require = createRequire(import.meta.url);
try {
  const expected = require('./package.json').devDependencies['@playwright/test'];
  if (require('@playwright/test/package.json').version !== expected) throw new Error('Playwright version mismatch');
  const executable = require('@playwright/test').chromium.executablePath();
  if (!fs.existsSync(executable)) throw new Error('Chromium is not installed');
  console.log(`Playwright ${expected}; Chromium ${executable}`);
} catch (error) {
  console.error(error.message);
  process.exitCode = 1;
}
