import { test as base, expect } from '@playwright/test';
import fs from 'node:fs/promises';
import path from 'node:path';
import net from 'node:net';
import { spawn, execFile } from 'node:child_process';
import { promisify } from 'node:util';
import { fileURLToPath } from 'node:url';

const execute = promisify(execFile);
const processScript = fileURLToPath(new URL('./process.ps1', import.meta.url));
export { expect };

async function freePort() {
  const server = net.createServer();
  await new Promise((resolve, reject) => { server.once('error', reject); server.listen(0, '127.0.0.1', resolve); });
  const port = server.address().port;
  await new Promise((resolve, reject) => server.close(error => error ? reject(error) : resolve()));
  return port;
}

export const test = base.extend({
  app: async ({ playwright }, use, testInfo) => {
    const artifacts = process.env.DOCS_SEARCH_EXE_ARTIFACTS;
    const prerequisite = JSON.parse(await fs.readFile(path.join(artifacts, 'preflight.json'), 'utf8'));
    testInfo.skip(!prerequisite.ready, prerequisite.reason);
    const id = testInfo.title.match(/^E\d+/)[0];
    const directory = path.join(artifacts, id);
    await fs.mkdir(directory); // Existing case data is never silently reused.
    const documents = path.join(directory, 'documents 日本語 😀');
    const localAppData = path.join(directory, 'local-app-data');
    const profile = path.join(directory, 'webview2');
    await Promise.all([documents, localAppData, profile].map(folder => fs.mkdir(folder)));
    const corpus = JSON.parse(await fs.readFile(path.join(artifacts, 'corpus.json'), 'utf8'));
    const errors = [];
    const sessions = [];
    let child, browser, page;
    let stopping = false;
    const app = {
      documents, localAppData, profile, directory,
      get page() { return page; },
      async copy(kind, relative, destination = '') {
        const target = path.join(documents, destination);
        await fs.cp(path.join(corpus[kind], relative), target, { recursive: true, force: false, errorOnExist: true });
        return target;
      },
      async events() { return page.evaluate(() => window.__docsSearchExeObservation.events); },
      async batches() { return page.evaluate(() => window.__docsSearchExeObservation.batches); },
      async search(root, query, { mode = 'standard' } = {}) {
        await page.locator('#root').fill(root);
        await page.locator('#search-mode').selectOption(mode);
        if (mode === 'standard') await page.locator('#query').fill(query);
        const offset = (await app.events()).length;
        await page.locator('#search').click();
        await expect.poll(async () => (await app.events()).slice(offset).some(event => event.type === 'started')).toBe(true);
        return (await app.events()).slice(offset).find(event => event.type === 'started');
      },
      async finished(search, reason = 'completed') {
        await expect.poll(async () => (await app.events()).some(event => event.searchId === search.searchId && event.type === 'finished')).toBe(true);
        const events = (await app.events()).filter(event => event.searchId === search.searchId);
        const finish = events.at(-1);
        expect(finish.type).toBe('finished');
        expect(finish.reason).toBe(reason);
        expect(events.map(event => event.sequence)).toEqual(events.map((_, index) => index + 1));
        expect(events.filter(event => event.type === 'started')).toHaveLength(1);
        expect(events.filter(event => event.type === 'finished')).toHaveLength(1);
        expect(finish.counts.resultCount).toBe(events.filter(event => event.type === 'result').length);
        expect(finish.counts.issueCount).toBe(events.filter(event => event.type === 'issue').length);
        await expect(page.locator('#search')).toBeEnabled();
        return { events, finish, hits: events.filter(event => event.type === 'result').map(event => event.hit) };
      },
      async stop({ graceful = false } = {}) {
        if (!child) return;
        stopping = true;
        const env = { ...process.env, DOCS_SEARCH_CHILD_PID: String(child.pid),
          DOCS_SEARCH_CHILD_CREATED: sessions.at(-1).portOwner?.rootCreated ?? '' };
        const snapshotPath = path.join(directory, `processes-${sessions.length}.json`);
        // Snapshot identities before closing so orphaned WebView2 children can be
        // terminated without touching other apps or a later reuse of their PIDs.
        if (child.pid) {
          const { stdout } = await execute('powershell.exe', ['-NoProfile', '-NonInteractive', '-ExecutionPolicy', 'Bypass', '-File', processScript, 'snapshot'], { env, windowsHide: true, timeout: 10_000 });
          await fs.writeFile(snapshotPath, stdout);
          const cleanupEnv = { ...env, DOCS_SEARCH_CHILD_SNAPSHOT: snapshotPath };
          try {
            if (graceful && child.exitCode === null && child.signalCode === null) {
              // Lifecycle only: close the owned native window without adding a
              // product invoke/permission or an OS UI automation dependency.
              const { stdout } = await execute('powershell.exe', ['-NoProfile', '-NonInteractive', '-ExecutionPolicy', 'Bypass', '-File', processScript, 'close'], { env: cleanupEnv, windowsHide: true, timeout: 10_000 });
              sessions.at(-1).close = JSON.parse(stdout);
              await expect.poll(() => child.exitCode !== null || child.signalCode !== null, { timeout: 10_000 }).toBe(true);
            }
          } finally {
            await execute('powershell.exe', ['-NoProfile', '-NonInteractive', '-ExecutionPolicy', 'Bypass', '-File', processScript, 'cleanup'], {
              env: cleanupEnv, windowsHide: true, timeout: 10_000,
            });
          }
        }
        if (browser?.isConnected()) await browser.close();
        browser = null; child = null; page = null;
      },
      async launch() {
        stopping = false;
        const port = await freePort();
        const session = { pid: null, port, profile, localAppData, executable: process.env.DOCS_SEARCH_EXE, stdout: '', stderr: '' };
        sessions.push(session);
        child = spawn(process.env.DOCS_SEARCH_EXE, [], {
          cwd: path.dirname(process.env.DOCS_SEARCH_EXE), windowsHide: true,
          env: { ...process.env, LOCALAPPDATA: localAppData,
            WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${port} --remote-debugging-address=127.0.0.1`,
            WEBVIEW2_USER_DATA_FOLDER: profile },
          stdio: ['ignore', 'pipe', 'pipe'],
        });
        session.pid = child.pid;
        let launchError;
        child.on('error', error => { launchError = error; });
        child.stdout.on('data', chunk => { session.stdout = (session.stdout + chunk).slice(-16_000); });
        child.stderr.on('data', chunk => { session.stderr = (session.stderr + chunk).slice(-16_000); });
        child.on('exit', (code, signal) => {
          session.exit = { code, signal };
          if (!stopping) errors.push(`Unexpected EXE exit: ${code ?? signal}`);
        });
        const deadline = Date.now() + 30_000;
        const remaining = () => Math.max(1, deadline - Date.now());
        try {
          // Read readiness only; do not attach until port ownership is established.
          await expect.poll(async () => {
            if (launchError) throw launchError;
            if (child.exitCode !== null) throw new Error(`EXE exited during startup: ${child.exitCode}`);
            try {
              const response = await fetch(`http://127.0.0.1:${port}/json/version`, { signal: AbortSignal.timeout(Math.min(1000, remaining())) });
              if (!response.ok) return false;
              session.cdp = await response.json();
              return Boolean(session.cdp.webSocketDebuggerUrl);
            } catch { return false; }
          }, { timeout: remaining(), message: 'Release WebView2 CDP readiness' }).toBe(true);
          const { stdout } = await execute('powershell.exe', ['-NoProfile', '-NonInteractive', '-ExecutionPolicy', 'Bypass', '-File', processScript, 'owner'], {
            env: { ...process.env, DOCS_SEARCH_CHILD_PID: String(child.pid), DOCS_SEARCH_CHILD_PORT: String(port), DOCS_SEARCH_CHILD_PROFILE: profile },
            windowsHide: true, timeout: remaining(),
          });
          session.portOwner = JSON.parse(stdout);
          browser = await playwright.chromium.connectOverCDP(`http://127.0.0.1:${port}`, { timeout: remaining() });
          session.webview2Version = browser.version();
          await expect.poll(() => browser.contexts().flatMap(context => context.pages()).length, { timeout: remaining() }).toBeGreaterThan(0);
          const pages = browser.contexts().flatMap(context => context.pages());
          expect(pages, 'Only the launched single-window app is attached').toHaveLength(1);
          page = pages[0];
        } catch (error) {
          session.connectionError = error.message;
          await app.stop();
          // Unavailable real connection is incomplete evidence, never a mock pass.
          testInfo.skip(true, `Real EXE/CDP connection unavailable: ${error.message}`);
        }
        page.on('pageerror', error => errors.push(`pageerror: ${error.message}`));
        page.on('crash', () => errors.push('WebView2 page crashed'));
        page.on('console', message => { if (message.type() === 'error') errors.push(`console: ${message.text()}`); });
        page.on('requestfailed', request => errors.push(`request: ${request.url()} ${request.failure()?.errorText}`));
        page.on('response', response => { if (response.status() >= 400) errors.push(`HTTP ${response.status()}: ${response.url()}`); });
        const logs = await page.context().newCDPSession(page);
        logs.on('Log.entryAdded', ({ entry }) => { if (entry.level === 'error') errors.push(`WebView2 log: ${entry.text}`); });
        await logs.send('Log.enable'); // Includes retained startup/resource errors.
        await expect(page.locator('#search')).toBeEnabled({ timeout: remaining() });
        await expect(page.locator('#status')).toHaveText('待機中', { timeout: remaining() });
        await page.evaluate(async () => {
          if (typeof window.__TAURI__?.core?.invoke !== 'function') throw new Error('Real Tauri invoke is missing');
          window.__docsSearchExeObservation = { events: [], batches: [] };
          await window.__TAURI__.event.listen('search-events', notification => {
            const record = window.__docsSearchExeObservation;
            record.batches.push(notification.payload.length);
            record.events.push(...notification.payload);
            if (record.cancelOnResult && notification.payload.some(event => event.type === 'result')) {
              record.cancelOnResult = false;
              document.getElementById('cancel').click();
            }
          });
        });
      },
    };
    try {
      await app.launch();
      await use(app);
      expect(errors, 'EXE, resource and browser errors must be absent').toEqual([]);
    } finally {
      const state = page && !page.isClosed() ? await page.evaluate(() => ({
        status: document.getElementById('status')?.textContent,
        counts: document.getElementById('counts')?.textContent,
        alerts: [...document.querySelectorAll('[role="alert"]')].map(node => ({ id: node.id, text: node.textContent })),
        batches: window.__docsSearchExeObservation?.batches,
        events: window.__docsSearchExeObservation?.events.slice(-25),
      })).catch(error => ({ diagnosticError: error.message })) : null;
      try { await app.stop(); }
      finally {
        await fs.writeFile(path.join(directory, 'session.json'), JSON.stringify({ prerequisite, sessions, errors }, null, 2) + '\n');
        if (testInfo.status !== testInfo.expectedStatus || errors.length) {
          await testInfo.attach('exe-failure.txt', { body: Buffer.from(JSON.stringify({ state, errors, sessions, assertions: testInfo.errors.map(error => error.message) }, null, 2).slice(0, 64_000)), contentType: 'text/plain' });
        }
      }
    }
  },
});
