import { test, expect, hit } from '../support/fixtures.mjs';

async function completed(ui) {
  await ui.open();
  const search = await ui.search();
  await ui.started(search);
  await ui.finish(search, [hit(1, { previewText: 'beacon keep' }), hit(2, { filePath: 'C:\\fixtures\\other.md', fileType: 'md', previewText: 'beacon skip' })]);
  return search;
}

test('U27: 絞り込み/全結果とCSV・TSV・JSONの出力要求', async ({ ui, page }) => {
  const search = await completed(ui);
  await page.locator('#result-filter-include').fill('keep');
  await expect(page.locator('#export-summary')).toHaveText('1 ファイル · 1 件を対象');
  for (const [index, id, format] of [[1, 'save-csv', 'csv'], [2, 'save-tsv', 'tsv'], [3, 'save-report', 'json']]) {
    await ui.queue('save_report', { value: `C:\\saved.${format}` });
    await page.locator(`#${id}`).click();
    const args = await ui.lastCall('save_report', index);
    expect(args.format).toBe(format);
    expect(args.report).toMatchObject({ searchId: search.searchId, scope: 'filtered', finishedReason: 'completed', selectedFileCount: 1, filterInclude: 'keep', request: search.request });
    expect(args.report.rows.map(row => row.resultId)).toEqual([1]);
    await expect(page.locator('#copy-status')).toContainText('保存しました');
  }
  await page.locator('#export-scope').selectOption('all');
  await expect(page.locator('#export-summary')).toHaveText('2 ファイル · 2 件を対象');
  await ui.queue('save_report', { value: 'C:\\all.json' });
  await page.locator('#save-report').click();
  const { report } = await ui.lastCall('save_report', 4);
  expect(report.scope).toBe('all');
  expect(report.filterInclude).toBe('');
  expect(report.rows.map(row => row.resultId)).toEqual([1, 2]);
});

test('U28: 一括コピーはフォーマット応答を渡し、コピー拒否を表示', async ({ ui, page }) => {
  await completed(ui);
  const content = 'file\tlocation\n"日本語"\t行 1\n';
  await ui.queue('format_report', { value: content });
  await page.locator('#copy-results').click();
  expect((await ui.lastCall('format_report')).report.rows).toHaveLength(2);
  await expect(page.locator('#copy-status')).toHaveText('2 件をTSVでコピーしました。');
  expect(await page.evaluate(() => window.__docsSearchTest.clipboard)).toEqual([content]);
  await page.evaluate(() => { window.__docsSearchTest.clipboardError = true; });
  await ui.queue('format_report', { value: content });
  await page.locator('#copy-results').click();
  await expect(page.locator('#general-error')).toHaveText('Test clipboard denied');
  await expect(page.locator('#copy-results')).toBeEnabled();
});

test('U29: 保存処理中の二重操作・取消・失敗で結果を失わない', async ({ ui, page }) => {
  await completed(ui);
  await ui.queue('save_report', { deferred: 'save' });
  await page.locator('#save-csv').click();
  await ui.pending('save');
  for (const id of ['save-csv', 'save-tsv', 'save-report', 'copy-results']) await expect(page.locator(`#${id}`)).toBeDisabled();
  await ui.resolve('save', { value: null });
  await expect(page.locator('#save-csv')).toBeEnabled();
  await expect(page.locator('#copy-status')).toBeEmpty();
  await expect(page.locator('.result')).toHaveCount(2);
  await ui.queue('save_report', { error: { message: '保存先へ書き込めません' } });
  await page.locator('#save-csv').click();
  await expect(page.locator('#general-error')).toHaveText('保存先へ書き込めません');
  await expect(page.locator('.result')).toHaveCount(2);
  await expect(page.locator('#save-csv')).toBeEnabled();
});

test('U30: 一括入力の識別子警告・文字列への切替・境界のプレビュー', async ({ ui, page }) => {
  await ui.open();
  await page.locator('#search-mode').selectOption('batch');
  await expect(page.locator('#batch-input')).toBeVisible();
  await page.locator('#batch-terms').fill('顧客\n顧客');
  await expect(page.locator('#batch-error')).toContainText('識別子として扱えません');
  expect(await ui.calls('preview_batch')).toEqual([]);
  await ui.queue('preview_batch', { value: [{ termId: 't1', term: '顧客' }] });
  await page.locator('#batch-match-mode').selectOption('text');
  await expect(page.locator('#batch-count')).toHaveText('1 語（空行・重複を除去）');
  await expect(page.locator('#batch-error')).toBeEmpty();
  expect(await ui.lastCall('preview_batch')).toEqual({ querySpec: { mode: 'batch', terms: ['顧客', '顧客'], matchMode: 'text' } });
  const search = await ui.search();
  expect(search.request.querySpec).toEqual({ mode: 'batch', terms: ['顧客', '顧客'], matchMode: 'text' });
});

test('U31: 語別件数・未確定0件・詳細・行列と行列保存要求', async ({ ui, page }) => {
  await ui.open();
  await page.locator('#search-mode').selectOption('batch');
  await ui.queue('preview_batch', { value: [{ termId: 't1', term: 'TAB_01' }, { termId: 't2', term: 'TAB_02' }] });
  await page.locator('#batch-terms').fill('TAB_01\nTAB_02');
  await expect(page.locator('#batch-count')).toContainText('2 語');
  expect(await ui.calls('preview_batch')).toHaveLength(1);
  const search = await ui.search();
  await ui.started(search);
  await expect(page.locator('#save-matrix-csv')).toBeVisible();
  await expect(page.locator('#save-matrix-csv')).toBeDisabled();
  await ui.finish(search, [hit(1, { termId: 't1', term: 'TAB_01', previewText: 'TAB_01', matchType: 'identifier' })], [
    { type: 'batchSummary', terms: [
      { termId: 't1', term: 'TAB_01', fileCount: 1, hitCount: 1, status: '未確定（中断）' },
      { termId: 't2', term: 'TAB_02', fileCount: 0, hitCount: 0, status: '未確定（中断）' },
    ] },
  ], 'cancelled');
  await expect(page.locator('#batch-summary-list summary')).toHaveText(['TAB_01 · 1 ファイル / 1 件 · 未確定（中断）', 'TAB_02 · 0 ファイル / 0 件 · 未確定（中断）']);
  await page.locator('#batch-summary-list summary').first().click();
  await expect(page.locator('#batch-summary-list .match')).toHaveCount(1);
  await page.locator('#batch-matrix summary').click();
  await expect(page.locator('#batch-matrix-content td')).toHaveText(['1', '0']);
  for (const [count, id, format] of [[1, 'save-matrix-csv', 'matrixcsv'], [2, 'save-matrix-tsv', 'matrixtsv']]) {
    await ui.queue('save_report', { value: 'C:\\matrix' });
    await page.locator(`#${id}`).click();
    const args = await ui.lastCall('save_report', count);
    expect(args.format).toBe(format);
    expect(args.report.batchSummary).toHaveLength(2);
    expect(args.report.finishedReason).toBe('cancelled');
    await expect(page.locator('#copy-status')).toContainText('保存しました');
  }
});
