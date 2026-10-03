import { test as base, expect } from '@playwright/test';
import { installTauriMock } from './tauri-mock.mjs';

export { expect };
export const test = base.extend({
  ui: async ({ page }, use, testInfo) => {
    const browserErrors = [];
    page.on('crash', () => browserErrors.push('Browser page crashed'));
    page.on('pageerror', error => browserErrors.push(`pageerror: ${error.message}`));
    page.on('console', message => { if (message.type() === 'error') browserErrors.push(`console: ${message.text()}`); });
    page.on('requestfailed', request => browserErrors.push(`request: ${request.url()} ${request.failure()?.errorText}`));
    page.on('response', response => { if (response.status() >= 400) browserErrors.push(`HTTP ${response.status()}: ${response.url()}`); });

    const calls = command => page.evaluate(command => window.__docsSearchTest.calls.filter(call => call.command === command), command);
    const ui = {
      page,
      async open(options = {}, { waitForRestore = true } = {}) {
        await page.addInitScript(installTauriMock, options);
        await page.goto('/');
        await page.waitForFunction(() => window.__docsSearchTest?.ready);
        if (waitForRestore) await expect(page.locator('#search')).toBeEnabled();
        await expect(page.locator('#status')).toHaveText('待機中');
      },
      calls,
      async queue(command, response) {
        await page.evaluate(({ command, response }) => window.__docsSearchTest.queue(command, response), { command, response });
      },
      async resolve(key, response) {
        await page.evaluate(({ key, response }) => window.__docsSearchTest.resolve(key, response), { key, response });
      },
      async pending(key) {
        await expect.poll(() => page.evaluate(() => window.__docsSearchTest.pending())).toContain(key);
      },
      async lastCall(command, count = 1) {
        await expect.poll(async () => (await calls(command)).length).toBe(count);
        return (await calls(command)).at(-1).args;
      },
      async search(query = 'beacon') {
        const count = (await calls('start_search')).length + 1;
        await page.locator('#root').fill('C:\\fixtures');
        if (await page.locator('#search-mode').inputValue() === 'standard') await page.locator('#query').fill(query);
        await page.locator('#search').click();
        return ui.lastCall('start_search', count);
      },
      async emit(search, items) {
        await page.evaluate(({ id, items }) => window.__docsSearchTest.emit(items, id), { id: search.searchId, items });
      },
      async started(search) { await ui.emit(search, [{ type: 'started', request: search.request }]); },
      async finish(search, hits = [], extra = [], reason = 'completed') {
        const prior = await page.evaluate(id => window.__docsSearchTest.events.filter(event => event.searchId === id), search.searchId);
        const allHits = [...prior.filter(event => event.type === 'result').map(event => event.hit), ...hits];
        const paths = [...new Set(allHits.map(hit => hit.filePath))];
        const issueCount = [...prior, ...extra].filter(event => event.type === 'issue').length;
        await ui.emit(search, [
          ...hits.map(hit => ({ type: 'result', hit })),
          { type: 'rankingSummary', files: paths },
          ...extra,
          { type: 'finished', reason, counts: counts(allHits.length, paths.length, issueCount) },
        ]);
        await expect(page.locator('#search')).toBeEnabled();
      },
    };
    try { await use(ui); }
    finally {
      const state = await page.evaluate(() => {
        const mock = window.__docsSearchTest;
        return {
          calls: mock?.calls ?? [], events: mock?.events ?? [], faults: mock?.faults ?? [],
          pending: mock?.pending() ?? [], clipboard: mock?.clipboard ?? [],
          status: document.getElementById('status')?.textContent,
          summary: document.getElementById('result-filter-summary')?.textContent,
          errors: [...document.querySelectorAll('[role="alert"]')].map(node => ({ id: node.id, text: node.textContent })),
          results: [...document.querySelectorAll('.result')].slice(0, 5).map(node => node.textContent.slice(0, 1600)),
          activeElement: document.activeElement?.id,
        };
      }).catch(error => ({ faults: [], pending: [], diagnosticError: error.message }));
      const failures = [...browserErrors, ...state.faults, ...state.pending.map(key => `Unresolved mock response: ${key}`)];
      if (state.diagnosticError) failures.push(`Could not inspect final UI state: ${state.diagnosticError}`);
      const failed = testInfo.status !== testInfo.expectedStatus || failures.length > 0;
      if (failed) {
        // Bounded text only: full result/event streams are intentionally not attached.
        const record = {
          test: testInfo.title, browserErrors, ...state,
          calls: (state.calls ?? []).slice(-40), events: (state.events ?? []).slice(-25),
          assertions: testInfo.errors.map(error => error.message),
        };
        await testInfo.attach('ui-failure.txt', { body: Buffer.from(JSON.stringify(record, null, 2).slice(0, 64_000)), contentType: 'text/plain' });
      }
      expect(failures, 'Browser, resource, and Tauri mock errors must be absent').toEqual([]);
    }
  },
});

// Fixtures mirror core/src/lib.rs, edit.rs, context.rs and ranking.rs wire fields.
// These describe supplied backend responses, not expected search engine behavior.
export const counts = (resultCount = 0, files = 0, issueCount = 0) => ({ resultCount, discoveredFiles: files, processedFiles: files, issueCount });
export function hit(resultId, overrides = {}) {
  return {
    resultId, filePath: 'C:\\fixtures\\sample.txt', fileType: 'txt', sourceKind: 'textLine',
    unitKey: String(resultId), partKey: 'text', groupKey: 'text', row: null, column: null,
    contentClass: 'body', anchor: null, location: { lineNumber: resultId },
    previewText: 'beacon', previewTruncated: false, matchRanges: [[0, 6]],
    sourceMatchRanges: [[0, 6]], documentOrder: [1, resultId], modifiedAt: 1700000000000,
    matchType: 'substring', matchCategory: 'standard', score: 70, evidence: [],
    ...overrides,
  };
}
export function editableHit(resultId = 1, overrides = {}) {
  return hit(resultId, {
    editAnchor: { lineNumber: resultId, range: [0, 6], sourceRevision: 'fixture-source', lineRevision: 'fixture-line' },
    ...overrides,
  });
}
export function evidence(term, lineNumber, overrides = {}) {
  const length = Array.from(term).length;
  return {
    termId: `t${lineNumber}`, term, sourceKind: 'textLine', contentClass: 'body',
    unitKey: String(lineNumber), partKey: 'text', row: null, column: null,
    location: { lineNumber }, previewText: term, previewTruncated: false,
    matchRanges: [[0, length]], sourceMatchRanges: [[0, length]], documentOrder: [1, lineNumber],
    ...overrides,
  };
}
export function fileRanking(filePath, resultIds, reasons = []) {
  return {
    filePath, resultIds, reasons,
    evidence: { quality: 70, matchedTerms: 1, sameRowTerms: 0, bodyEvidence: Math.min(resultIds.length, 5), distinctEvidence: Math.min(resultIds.length, 5) },
  };
}
export function editResponse(token = 'edit-token', selectedText = 'beacon') {
  return { token, view: { filePath: 'C:\\fixtures\\sample.txt', lineNumber: 1, encoding: 'UTF-8', before: 'before ', selectedText, after: ' after' } };
}
export function excelHit(resultId = 1, overrides = {}) {
  return hit(resultId, {
    filePath: 'C:\\fixtures\\layout.xlsx', fileType: 'xlsx', sourceKind: 'cell',
    partKey: 'xl/worksheets/sheet1.xml', groupKey: 'sheet:0', row: 1, column: 1,
    location: { sheetName: 'Sheet1', cellAddress: 'A1' }, ...overrides,
  });
}
export function contextResponse(overrides = {}) {
  return {
    sheetName: 'Sheet1', focusAddress: 'A1', focusKind: 'cell', firstRow: 1, firstColumn: 1,
    rowCount: 5, columnCount: 5, hiddenRows: [2], hiddenColumns: [3], evidenceAddresses: [],
    cells: [{ row: 1, column: 1, address: 'A1', text: 'beacon', truncated: false, matchRanges: [[0, 6]] }],
    merges: [{ firstRow: 1, lastRow: 1, firstColumn: 1, lastColumn: 2, anchorAddress: 'A1', anchorText: 'beacon', anchorTruncated: false }],
    layout: { defaultRowHeight: null, defaultColumnWidth: null, rows: [], columns: [], notes: ['書式は保存済みの情報です。'], cells: [{ row: 1, column: 1, style: { horizontal: null, vertical: null, wrap: false, fontName: null, fontSize: null, bold: true, italic: false, underline: false, strike: false, color: '#112233', fill: '#ffeecc', borders: [null, null, null, null] } }] },
    ...overrides,
  };
}
