import { test, expect, hit, counts } from '../support/fixtures.mjs';

test('U01: 初期選択と拡張子の全体・分類操作', async ({ ui, page }) => {
  await ui.open();
  await expect(page.locator('input[name="extension"]')).toHaveCount(93);
  await expect(page.locator('input[name="extension"]:checked')).toHaveCount(93);
  await expect(page.locator('#selected-extension-count')).toHaveText('93 種類選択中');
  for (const id of ['use-index', 'fuzzy-search', 'include-notes', 'include-formulas']) await expect(page.locator(`#${id}`)).not.toBeChecked();
  await page.locator('.extension-picker > summary').click();
  await page.locator('#clear-all-extensions').click();
  await expect(page.locator('input[name="extension"]:checked')).toHaveCount(0);
  const group = page.locator('.extension-group').first();
  await group.locator('.group-toggle').click();
  await expect(group.locator('input:checked')).toHaveCount(await group.locator('input').count());
  await group.locator('.group-toggle').click();
  await expect(group.locator('input:checked')).toHaveCount(0);
  await page.locator('#select-all-extensions').click();
  await expect(page.locator('input[name="extension"]:checked')).toHaveCount(93);
});

test('U02: 必須入力と拡張子0件では要求を送らない', async ({ ui, page }) => {
  await ui.open();
  await page.locator('#query').fill('beacon');
  await page.locator('#search').click();
  await expect(page.locator('#root-error')).not.toBeEmpty();
  await page.locator('#root').fill('C:\\fixtures');
  await page.locator('#query').fill('');
  await page.locator('#search').click();
  await expect(page.locator('#query-error')).not.toBeEmpty();
  await page.locator('#query').fill('beacon');
  await page.locator('.extension-picker > summary').click();
  await page.locator('#clear-all-extensions').click();
  await page.locator('#search').click();
  await expect(page.locator('#extensions-error')).not.toBeEmpty();
  expect(await ui.calls('start_search')).toEqual([]);
});

test('U03: 複数ルート・除外・独立オプションを境界へ渡す', async ({ ui, page }) => {
  await ui.open();
  await page.locator('#add-root').click();
  await page.locator('#root-folders input').nth(1).fill('C:\\other');
  await page.locator('#add-excluded').click();
  await page.locator('#excluded-folders input').fill('C:\\fixtures\\skip');
  await page.locator('#fuzzy-search').check();
  await page.locator('#include-notes').check();
  const first = await ui.search(' Café ');
  expect(first.request).toMatchObject({ rootDirectory: 'C:\\fixtures', additionalDirectories: ['C:\\other'], excludedDirectories: ['C:\\fixtures\\skip'], query: ' Café ', querySpec: null, recursive: true, useIndex: false, fuzzySearch: true, includeNotes: true, includeFormulas: false });
  expect(first.request.extensions).toHaveLength(93);
  await ui.started(first);
  await ui.finish(first);
  await page.locator('#fuzzy-search').uncheck();
  await page.locator('#use-index').check();
  await page.locator('#include-formulas').check();
  await page.locator('#root-folders .folder-remove').click();
  await page.locator('#excluded-folders .folder-remove').click();
  const second = await ui.search();
  expect(second.request).toMatchObject({ additionalDirectories: [], excludedDirectories: [], useIndex: true, fuzzySearch: false, includeFormulas: true });
});

test('U04: 高度な条件の行と判定範囲を保持する', async ({ ui, page }) => {
  await ui.open();
  await page.locator('#search-mode').selectOption('conditions');
  await expect(page.locator('#normal-query-field')).toBeHidden();
  await expect(page.locator('#advanced-conditions')).toBeVisible();
  await page.locator('#all-terms').fill('alpha\nbeta');
  await page.locator('#any-terms').fill('keep');
  await page.locator('#not-terms').fill('skip');
  await page.locator('#condition-scope').selectOption('file');
  const search = await ui.search();
  expect(search.request).toMatchObject({ query: '', querySpec: { mode: 'conditions', scope: 'file', all: ['alpha', 'beta'], any: ['keep'], not: ['skip'] } });
});

test('U05: 境界の入力拒否を適切な欄へ返し再検索可能にする', async ({ ui, page }) => {
  await ui.open();
  await ui.queue('start_search', { error: { field: 'rootDirectory', message: '存在しないフォルダーです' } });
  await ui.search();
  await expect(page.locator('#root-error')).toHaveText('存在しないフォルダーです');
  await expect(page.locator('#status')).toHaveText('入力を確認してください。');
  await expect(page.locator('#search')).toBeEnabled();
  await expect(page.locator('#cancel')).toBeDisabled();
  const accepted = await ui.search();
  await ui.started(accepted);
  await ui.finish(accepted);
  await expect(page.locator('#root-error')).toBeEmpty();
});

test('U06: バッチ進捗・完了・出力可否と二重送信防止', async ({ ui, page }) => {
  await ui.open();
  const search = await ui.search();
  await ui.started(search);
  await expect(page.locator('#search')).toBeDisabled();
  await expect(page.locator('#cancel')).toBeEnabled();
  await expect(page.locator('#save-csv')).toBeDisabled();
  await page.locator('#search-form').dispatchEvent('submit');
  expect(await ui.calls('start_search')).toHaveLength(1);
  await ui.emit(search, [{ type: 'progress', phase: 'discovery', counts: { ...counts(), discoveredFiles: 3 } }]);
  await expect(page.locator('#counts')).toHaveText('結果 0 · 発見 3 · 処理 0 · エラー 0');
  await ui.emit(search, [{ type: 'progress', phase: 'search', counts: counts(1, 3) }]);
  await expect(page.locator('#status')).toHaveText('検索中…');
  await ui.finish(search, [hit(1)]);
  await expect(page.locator('#status')).toHaveText('完了');
  await expect(page.locator('#cancel')).toBeDisabled();
  await expect(page.locator('#save-csv')).toBeEnabled();
});

for (const [reason, status, state] of [['completed', '完了 · 該当なし', 'complete'], ['cancelled', '中断', 'paused'], ['failed', '全体失敗', 'failed']]) {
  test(`U07: 0件の終端 ${reason}`, async ({ ui, page }) => {
    await ui.open();
    const search = await ui.search();
    await ui.started(search);
    await ui.finish(search, [], [], reason);
    await expect(page.locator('#status')).toHaveText(status);
    await expect(page.locator('.summary')).toHaveAttribute('data-state', state);
    await expect(page.locator('#empty-state')).not.toBeEmpty();
    await expect(page.locator('#search')).toBeEnabled();
  });
}

test('U08: 中断要求は現在IDで1回、終端後の古い通知は次の検索に混ざらない', async ({ ui, page }) => {
  await ui.open();
  const old = await ui.search();
  await ui.started(old);
  await page.locator('#cancel').click();
  expect(await ui.lastCall('cancel_search')).toEqual({ searchId: old.searchId });
  await expect(page.locator('#cancel')).toBeDisabled();
  await ui.finish(old, [hit(1)], [], 'cancelled');
  const current = await ui.search('fresh');
  expect(current.searchId).not.toBe(old.searchId);
  await ui.started(current);
  await ui.emit(old, [{ type: 'result', hit: hit(9) }, { type: 'finished', reason: 'failed', counts: counts(1, 1) }]);
  await expect(page.locator('.result')).toHaveCount(0);
  await expect(page.locator('#search')).toBeDisabled();
  await ui.finish(current, [hit(2, { previewText: 'fresh', matchRanges: [[0, 5]], sourceMatchRanges: [[0, 5]] })]);
  await expect(page.locator('.preview')).toHaveText('fresh');
});

test('U09: フォルダー選択の取消・失敗と索引削除', async ({ ui, page }) => {
  await ui.open();
  await ui.queue('pick_folder', { value: 'C:\\picked' });
  await page.locator('#root-folders .folder-browse').click();
  await expect(page.locator('#root')).toHaveValue('C:\\picked');
  await ui.queue('pick_folder', { value: null });
  await page.locator('#root-folders .folder-browse').click();
  await ui.lastCall('pick_folder', 2);
  await expect(page.locator('#root')).toHaveValue('C:\\picked');
  await ui.queue('pick_folder', { error: 'picker denied' });
  await page.locator('#root-folders .folder-browse').click();
  await expect(page.locator('#general-error')).toContainText('フォルダーを選択できません');
  await page.locator('#clear-index').click();
  await ui.lastCall('clear_search_index');
  await expect(page.locator('#copy-status')).toContainText('索引');
});
