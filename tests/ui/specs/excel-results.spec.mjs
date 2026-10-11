import { test, expect, hit, evidence, excelHit } from '../support/fixtures.mjs';

test('U45: シート名は構造化された位置で各箇所を絞り込み、全文や非Excelを混同しない', async ({ ui, page }) => {
  await ui.open();
  const search = await ui.search(); await ui.started(search);
  const matches = ['cell', 'formula', 'excelRow', 'shape', 'excelComment'].map((sourceKind, i) => excelHit(i + 1, {
    sourceKind, location: { sheetName: 'Résumé 集計', cellAddress: 'A1', row: 1, anchor: 'C8' },
  }));
  await ui.finish(search, [...matches,
    excelHit(6, { location: { sheetName: 'other', cellAddress: 'A1' }, previewText: 'Résumé 集計' }),
    excelHit(7, { sourceKind: 'fileName', location: {}, filePath: 'C:\\fixtures\\Résumé 集計.xlsx' }),
    hit(8, { previewText: 'Résumé 集計', location: { sheetName: 'Résumé 集計' } }),
    excelHit(9, { filePath: 'C:\\fixtures\\second.xlsm', fileType: 'xlsm', location: { sheetName: 'Résumé 集計', cellAddress: 'B2' } }),
  ]);
  await page.locator('#result-filter-sheet').fill('RE\u0301SUME\u0301');
  await expect(page.locator('.match')).toHaveCount(6);
  await expect(page.locator('#result-filter-summary')).toHaveText('絞り込み後 2 / 4 ファイル · 6 / 9 件');
  await page.locator('#result-filter-text').fill('layout');
  await page.locator('#result-filter-extension').selectOption('xlsx');
  await page.locator('#result-filter-include').fill('beacon');
  await page.locator('#result-filter-exclude').fill('skip');
  await expect(page.locator('.match')).toHaveCount(5);
  await page.locator('#result-filter-sheet').fill('absent');
  await expect(page.locator('.match')).toHaveCount(0);
  await page.locator('#result-filter-clear').click();
  await expect(page.locator('.match')).toHaveCount(9);
  expect(await ui.calls('start_search')).toHaveLength(1);
});

test('U46: 別シートの根拠でもfileMatchを残し、全根拠と元の検索条件を保持する', async ({ ui, page }) => {
  await ui.open();
  const search = await ui.search(); await ui.started(search);
  const sources = [evidence('alpha', 1, { sourceKind: 'cell', location: { sheetName: '他', cellAddress: 'A1' } }),
    evidence('beta', 2, { sourceKind: 'formula', location: { sheetName: '集計', cellAddress: 'D2' } })];
  await ui.finish(search, [excelHit(1, { sourceKind: 'fileMatch', location: {}, evidence: sources })]);
  await page.locator('#result-filter-sheet').fill('集計');
  await expect(page.locator('.match')).toHaveCount(1);
  await expect(page.locator('.evidence-list li')).toHaveCount(2);
  await ui.queue('save_report', { value: true }); await page.locator('#save-report').click();
  const { report } = await ui.lastCall('save_report');
  expect(report.filterSheet).toBe('集計');
  expect(report.request).toEqual(search.request);
  expect(report.rows[0].evidence.map(item => item.location.sheetName)).toEqual(['他', '集計']);
});

test('U47: シート条件を新着・未描画・折りたたみ・各出力へ適用し、新検索で解除', async ({ ui, page }) => {
  await ui.open();
  const search = await ui.search(); await ui.started(search);
  await page.locator('#result-filter-sheet').fill('集計');
  const hits = Array.from({ length: 205 }, (_, i) => excelHit(i + 1, {
    filePath: `C:\\fixtures\\${i}.xlsx`, location: { sheetName: '集計', cellAddress: 'A1' },
  }));
  await ui.emit(search, [{ type: 'result', hit: hits[0] }]);
  await expect(page.locator('.match')).toHaveCount(1);
  await page.locator('.result-toggle').click();
  await ui.finish(search, [...hits.slice(1), excelHit(206, { filePath: 'C:\\fixtures\\other.xlsx' })]);
  await expect(page.locator('.result')).toHaveCount(200);
  await expect(page.locator('.result-toggle').first()).toHaveAttribute('aria-expanded', 'false');
  await expect(page.locator('#result-filter-summary')).toHaveText('絞り込み後 205 / 206 ファイル · 205 / 206 件');
  await ui.queue('format_report', { value: 'fixed TSV response' });
  await page.locator('#copy-results').click();
  const copied = (await ui.lastCall('format_report')).report;
  expect(copied.rows).toHaveLength(205); expect(copied.filterSheet).toBe('集計');
  await expect(page.locator('#copy-results')).toBeEnabled();
  for (const [i, button] of ['save-csv', 'save-tsv', 'save-report'].entries()) {
    await ui.queue('save_report', { value: true }); await page.locator(`#${button}`).click();
    const { report } = await ui.lastCall('save_report', i + 1);
    expect(report.rows).toHaveLength(205); expect(report.filterSheet).toBe('集計');
    expect(new Set(report.rows.map(row => row.resultId)).size).toBe(205);
    await expect(page.locator(`#${button}`)).toBeEnabled();
  }
  await page.locator('#export-scope').selectOption('all');
  await ui.queue('save_report', { value: true }); await page.locator('#save-report').click();
  const { report } = await ui.lastCall('save_report', 4);
  expect(report.rows).toHaveLength(206); expect(report.filterSheet).toBe('');
  const next = await ui.search(); await ui.started(next);
  await expect(page.locator('#result-filter-sheet')).toHaveValue('');
});

test('U49: 一括検索の行列出力にもシート名と出力範囲を反映する', async ({ ui, page }) => {
  await ui.open();
  await page.locator('#search-mode').selectOption('batch');
  await ui.queue('preview_batch', { value: [{ termId: 't1', term: 'TAB_01' }] });
  await page.locator('#batch-terms').fill('TAB_01');
  await expect(page.locator('#batch-count')).toContainText('1 語');
  const search = await ui.search(); await ui.started(search);
  await ui.finish(search, [
    excelHit(1, { termId: 't1', term: 'TAB_01', location: { sheetName: '集計', cellAddress: 'A1' } }),
    excelHit(2, { termId: 't1', term: 'TAB_01', filePath: 'C:\\fixtures\\other.xlsx' }),
  ], [{ type: 'batchSummary', terms: [{ termId: 't1', term: 'TAB_01', fileCount: 2, hitCount: 2, status: '一致あり' }] }]);
  await page.locator('#result-filter-sheet').fill('集計');
  for (const [i, button] of ['save-matrix-csv', 'save-matrix-tsv'].entries()) {
    await ui.queue('save_report', { value: true }); await page.locator(`#${button}`).click();
    const { report } = await ui.lastCall('save_report', i + 1);
    expect(report.rows.map(row => row.resultId)).toEqual([1]); expect(report.filterSheet).toBe('集計');
    expect(report.batchSummary[0].hitCount).toBe(2);
    await expect(page.locator(`#${button}`)).toBeEnabled();
  }
  await page.locator('#export-scope').selectOption('all');
  await ui.queue('save_report', { value: true }); await page.locator('#save-matrix-csv').click();
  const { report } = await ui.lastCall('save_report', 3);
  expect(report.rows).toHaveLength(2); expect(report.filterSheet).toBe('');
});

test('U48: Excelの選択ヒット・根拠を指定し、二重操作・失敗・遅延応答を扱う', async ({ ui, page }) => {
  await ui.open();
  const search = await ui.search(); await ui.started(search);
  await ui.finish(search, [excelHit(1), excelHit(2, { location: { sheetName: '別の表', cellAddress: 'D900' } }),
    excelHit(3, { sourceKind: 'excelRow', location: { sheetName: '行の表', row: 2 }, evidence: [
      evidence('alpha', 2, { sourceKind: 'cell', location: { sheetName: '行の表', cellAddress: 'B2' } }),
      evidence('beta', 2, { sourceKind: 'formula', location: { sheetName: '行の表', cellAddress: 'F2' } }),
    ] }), excelHit(4, { sourceKind: 'shape', location: { sheetName: '図', anchor: 'C8' } }),
    excelHit(5, { sourceKind: 'excelComment', location: { sheetName: 'コメント', cellAddress: 'G4' } }),
    excelHit(6, { sourceKind: 'fileName', location: {} }), hit(7)]);
  await expect(page.locator('.excel-open')).toHaveCount(6);
  await expect(page.locator('.excel-open-unavailable')).toContainText('セル位置を特定できません');
  await ui.queue('open_excel_result', { value: null });
  await page.getByRole('button', { name: 'layout.xlsx、別の表!D900をExcelで開く', exact: true }).press('Enter');
  expect(await ui.lastCall('open_excel_result')).toEqual({ searchId: search.searchId, resultId: 2, evidenceIndex: null });
  await ui.queue('open_excel_result', { error: '非表示シートには移動できません。' });
  await page.getByRole('button', { name: 'layout.xlsx、行の表!F2をExcelで開く', exact: true }).click();
  expect(await ui.lastCall('open_excel_result', 2)).toEqual({ searchId: search.searchId, resultId: 3, evidenceIndex: 1 });
  await expect(page.locator('.excel-open-status').filter({ hasText: '非表示シート' })).toContainText('行の表!F2');
  await page.locator('.file-name').first().click();
  expect(await ui.lastCall('open_result')).toEqual({ path: 'C:\\fixtures\\layout.xlsx' });
  await ui.queue('open_excel_result', { deferred: 'excel' });
  const button = page.getByRole('button', { name: 'layout.xlsx、Sheet1!A1をExcelで開く', exact: true });
  await button.click(); await ui.pending('excel'); await expect(button).toBeDisabled();
  const next = await ui.search(); await ui.started(next);
  await ui.resolve('excel', { error: { message: '古い結果' } });
  await expect(page.locator('#general-error')).toBeEmpty();
  await expect(page.locator('.excel-open-status')).toHaveCount(0);
});
