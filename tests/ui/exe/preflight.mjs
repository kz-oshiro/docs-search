import fs from 'node:fs';
import path from 'node:path';
import { createRequire } from 'node:module';
import { execFileSync } from 'node:child_process';
import os from 'node:os';
const require = createRequire(import.meta.url);
const record = { platform: process.platform, windows: { release: os.release(), version: os.version() }, node: process.version, ready: false };
try {
  if (process.platform !== 'win32') throw new Error('Windows with WebView2 is required');
  const expected = require('../package.json').devDependencies['@playwright/test'];
  record.playwright = require('@playwright/test/package.json').version;
  if (record.playwright !== expected) throw new Error('Playwright version mismatch; run cargo xtask setup');
  fs.accessSync(process.env.DOCS_SEARCH_EXE, fs.constants.R_OK);
  record.sessionId = Number(execFileSync('powershell.exe', ['-NoProfile', '-NonInteractive', '-Command', '(Get-Process -Id $PID).SessionId'], { windowsHide: true, encoding: 'utf8' }).trim());
  if (!record.sessionId || process.env.SESSIONNAME === 'Services') throw new Error('An interactive Windows session is required');
  // A fixed/runtime override is accepted only when its actual executable exists.
  const roots = [process.env.WEBVIEW2_BROWSER_EXECUTABLE_FOLDER,
    ...[process.env['ProgramFiles(x86)'], process.env.ProgramFiles, process.env.LOCALAPPDATA]
      .filter(Boolean).map(root => path.join(root, 'Microsoft', 'EdgeWebView', 'Application'))].filter(Boolean);
  record.webview2 = roots.flatMap(root => {
    if (!fs.existsSync(root)) return [];
    return [root, ...fs.readdirSync(root, { withFileTypes: true }).filter(item => item.isDirectory()).map(item => path.join(root, item.name))]
      .filter(folder => fs.existsSync(path.join(folder, 'msedgewebview2.exe')));
  });
  if (!record.webview2.length) throw new Error('WebView2 runtime executable was not found');
  record.ready = true;
} catch (error) {
  record.reason = error.message;
  process.exitCode = 1;
}
fs.writeFileSync(path.join(process.env.DOCS_SEARCH_EXE_ARTIFACTS, 'preflight.json'), JSON.stringify(record, null, 2) + '\n');
console.log(JSON.stringify(record));
