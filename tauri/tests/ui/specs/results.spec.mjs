import { test, expect, hit, evidence, fileRanking } from '../support/fixtures.mjs';

test('U10: Unicode強調と通常・あいまい・日時・エラー併存', async ({ ui, page }) => {
  await ui.open();
  const search = await ui.search();
  await ui.started(search);
  const issue = { type: 'issue', issue: { stage: 'read', path: 'C:\\fixtures\\sample.txt', code: 'unreadable', reason: '一部の部品を読めません' } };
  // Issue arrives before its result, which must still receive the error badge.
  await ui.emit(search, [issue]);
  await ui.finish(search, [hit(1, { previewText: '😀beacon 日本語', matchRanges: [[1, 7]], sourceMatchRanges: [[1, 7]] }), hit(2, { matchCategory: 'fuzzy', matchType: 'editDistance', previewText: 'beacno' })], [], 'completed');
  await expect(page.locator('.preview').first().locator('mark')).toHaveText('beacon');
  await expect(page.locator('.preview').first()).toHaveText('😀beacon 日本語');
  await expect(page.locator('.match-category')).toHaveText(['一致検索', 'あいまい検索']);
  await expect(page.locator('.match-type').last()).toHaveText('タイプミス候補');
  await expect(page.locator('.result')).toHaveClass(/has-error/);
  await expect(page.locator('.error-badge')).toHaveText('エラーあり');
  await expect(page.locator('.modified-at')).toHaveText('更新 2023/11/15 07:13:20（Asia/Tokyo）');
  await expect(page.locator('#status')).toHaveText('完了（一部エラーあり）');
  await page.locator('#issues-panel summary').click();
  await expect(page.locator('.issue-reason')).toHaveText('一部の部品を読めません');
});

test('U11: 同名別パスの階層は独立、親の再展開で子の状態を保持', async ({ ui, page }) => {
  await ui.open();
  const search = await ui.search();
  await ui.started(search);
  await ui.finish(search, [hit(1, { filePath: 'C:\\fixtures\\left\\same\\sample.txt' }), hit(2, { filePath: 'C:\\fixtures\\right\\same\\sample.txt' })]);
  const left = page.locator('.directory-result').filter({ has: page.locator('.directory-heading strong', { hasText: 'C:\\fixtures\\left\\same' }) });
  const right = page.locator('.directory-result').filter({ has: page.locator('.directory-heading strong', { hasText: 'C:\\fixtures\\right\\same' }) });
  const child = left.locator('.result-toggle');
  await child.focus();
  await child.press('Enter');
  await expect(child).toHaveAttribute('aria-expanded', 'false');
  await expect(right.locator('.result-toggle')).toHaveAttribute('aria-expanded', 'true');
  const parent = left.locator('.directory-heading button');
  await parent.click();
  await expect(left.locator('.directory-files')).toBeHidden();
  await parent.press('Space');
  await expect(left.locator('.directory-files')).toBeVisible();
  await expect(child).toHaveAttribute('aria-expanded', 'false');
  await left.locator('.file-name').click();
  expect(await ui.lastCall('open_result')).toEqual({ path: 'C:\\fixtures\\left\\same\\sample.txt' });
  await expect(child).toHaveAttribute('aria-expanded', 'false');
});

test('U12: 描画追加・終端・絞り込みを経ても折りたたみを保持、新検索で解除', async ({ ui, page }) => {
  await ui.open();
  const search = await ui.search();
  await ui.started(search);
  await ui.emit(search, [{ type: 'result', hit: hit(1) }]);
  await page.locator('.result-toggle').click();
  await ui.finish(search, [hit(2)]);
  await expect(page.locator('.result-toggle')).toHaveAttribute('aria-expanded', 'false');
  await expect(page.locator('.match')).toHaveCount(2);
  await page.locator('#result-filter-text').fill('sample');
  await expect(page.locator('.result-toggle')).toHaveAttribute('aria-expanded', 'false');
  await page.locator('#result-filter-clear').click();
  await expect(page.locator('.result-toggle')).toHaveAttribute('aria-expanded', 'false');
  const next = await ui.search();
  await ui.started(next);
  await ui.finish(next, [hit(1)]);
  await expect(page.locator('.result-toggle')).toHaveAttribute('aria-expanded', 'true');
});

test('U13: 本文含む/除外・パス・拡張子を併用、解除は再検索しない', async ({ ui, page }) => {
  await ui.open();
  const search = await ui.search();
  await ui.started(search);
  await ui.finish(search, [
    hit(1, { previewText: 'BEACON KEEP' }), hit(2, { previewText: 'beacon keep skip' }),
    hit(3, { previewText: 'beacon other' }), hit(4, { filePath: 'C:\\fixtures\\other.md', fileType: 'md', previewText: 'beacon keep' }),
  ]);
  await page.locator('#result-filter-include').fill('keep');
  await page.locator('#result-filter-exclude').fill('SKIP');
  await page.locator('#result-filter-text').fill('SAMPLE');
  await page.locator('#result-filter-extension').selectOption('txt');
  await expect(page.locator('.match')).toHaveCount(1);
  await expect(page.locator('.preview')).toHaveText('BEACON KEEP');
  await expect(page.locator('#result-filter-summary')).toHaveText('絞り込み後 1 / 2 ファイル · 1 / 4 件');
  await page.locator('#result-filter-clear').click();
  await expect(page.locator('.match')).toHaveCount(4);
  expect(await ui.calls('start_search')).toHaveLength(1);
  const next = await ui.search();
  await ui.started(next);
  await expect(page.locator('#result-filter-include')).toHaveValue('');
  await expect(page.locator('#result-filter-text')).toHaveValue('');
});

test('U14: 高度な検索は根拠の抜粋を本文フィルターと強調へ使う', async ({ ui, page }) => {
  await ui.open();
  const search = await ui.search();
  await ui.started(search);
  await ui.finish(search, [hit(1, { sourceKind: 'fileMatch', evidence: [
    evidence('alpha', 2, { previewText: 'alpha KEEP' }),
    evidence('beta', 4),
  ] })]);
  await page.locator('#result-filter-include').fill('keep');
  await expect(page.locator('.evidence-list li')).toHaveCount(2);
  await expect(page.locator('.evidence-heading')).toHaveText(['alpha · 行 2', 'beta · 行 4']);
  await expect(page.locator('.evidence-preview mark')).toHaveText(['alpha', 'beta']);
  await page.locator('#result-filter-exclude').fill('beta');
  await expect(page.locator('.result')).toHaveCount(0);
});

test('U15: コアのファイル順位と文書位置順を到着順から独立して表示', async ({ ui, page }) => {
  await ui.open();
  const search = await ui.search();
  await ui.started(search);
  const firstPath = 'C:\\fixtures\\first.txt', secondPath = 'C:\\fixtures\\second.txt';
  await ui.finish(search, [hit(8, { filePath: secondPath }), hit(10, { filePath: firstPath, documentOrder: [1, 10] }), hit(2, { filePath: firstPath, documentOrder: [1, 2] })], [
    { type: 'fileRanked', ranking: fileRanking(firstPath, [2, 10], ['本文に複数の根拠']) },
    { type: 'rankingSummary', files: [firstPath, secondPath] },
  ]);
  await expect(page.locator('.file-name')).toHaveText(['first.txt', 'second.txt']);
  await expect(page.locator('.result').first().locator('.location')).toHaveText(['行 2', '行 10']);
  await expect(page.locator('.ranking-reasons')).toHaveText('本文に複数の根拠');
});

test('U16: 205ファイルの追加表示と全結果出力は欠落・重複なし', async ({ ui, page }) => {
  await ui.open();
  const search = await ui.search();
  await ui.started(search);
  const hits = Array.from({ length: 205 }, (_, index) => hit(index + 1, { filePath: `C:\\fixtures\\pages\\${String(index).padStart(3, '0')}.txt` }));
  await ui.finish(search, hits);
  await expect(page.locator('.result')).toHaveCount(200);
  await page.locator('.result-toggle').first().click();
  await expect(page.locator('#result-filter-summary')).toHaveText('絞り込み後 205 / 205 ファイル · 205 / 205 件');
  await ui.queue('save_report', { value: 'C:\\saved.csv' });
  await page.locator('#save-csv').click();
  const { report } = await ui.lastCall('save_report');
  expect(report.rows).toHaveLength(205);
  expect(new Set(report.rows.map(row => row.resultId)).size).toBe(205);
  await page.locator('#more').click();
  await expect(page.locator('.result')).toHaveCount(205);
  await expect(page.locator('.result-toggle').first()).toHaveAttribute('aria-expanded', 'false');
  await expect(page.locator('#more')).toBeHidden();
  const displayed = await page.locator('.path').allTextContents();
  expect(new Set(displayed).size).toBe(205);
  expect(displayed).toEqual(hits.map(item => item.filePath));
});

test('U17: 元ファイル起動・場所コピーの要求と失敗表示', async ({ ui, page }) => {
  await ui.open();
  const search = await ui.search();
  await ui.started(search);
  await ui.finish(search, [hit(3)]);
  await page.locator('.copy-location').click();
  expect(await page.evaluate(() => window.__docsSearchTest.clipboard)).toEqual(['C:\\fixtures\\sample.txt\n行 3']);
  await ui.queue('open_result', { error: { message: '関連付けがありません' } });
  await page.locator('.file-name').click();
  await expect(page.locator('#general-error')).toHaveText('関連付けがありません');
  await page.evaluate(() => { window.__docsSearchTest.clipboardError = true; });
  await page.locator('.copy-location').click();
  await expect(page.locator('#general-error')).toContainText('コピーできませんでした');
});
