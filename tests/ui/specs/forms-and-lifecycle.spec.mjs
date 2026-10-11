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
  await expect(page.locator('label[for="query"]')).toHaveText('検索語');
  await expect(page.locator('#query')).toHaveAttribute('aria-describedby', 'query-error');
  await page.locator('#query').press('Enter');
  await expect(page.locator('#query-error')).not.toBeEmpty();
  expect(await ui.calls('start_search')).toEqual([]);
  await page.locator('#query').fill('beacon');
  await page.locator('.extension-picker > summary').click();
  await page.locator('#clear-all-extensions').click();
  await page.locator('#search').click();
  await expect(page.locator('#extensions-error')).not.toBeEmpty();
  expect(await ui.calls('start_search')).toEqual([]);
  await page.locator('#select-all-extensions').click();
  await page.locator('#query').press('Enter');
  const search = await ui.lastCall('start_search');
  expect(search.request).toMatchObject({ query: 'beacon', querySpec: null });
  await ui.started(search);
  await ui.finish(search);
  expect(await ui.calls('start_search')).toHaveLength(1);
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
  await page.locator('#query').fill('保持する検索語');
  await page.locator('#search-mode').selectOption('conditions');
  await expect(page.locator('#normal-query-field')).toBeHidden();
  await expect(page.locator('#query')).toBeDisabled();
  await expect(page.locator('#advanced-conditions')).toBeVisible();
  await expect(page.locator('#batch-input')).toBeHidden();
  await page.locator('#search-mode').selectOption('batch');
  await expect(page.locator('#normal-query-field')).toBeHidden();
  await expect(page.locator('#query')).toBeDisabled();
  await expect(page.locator('#advanced-conditions')).toBeHidden();
  await expect(page.locator('#batch-input')).toBeVisible();
  await page.locator('#search-mode').selectOption('standard');
  await expect(page.locator('#normal-query-field')).toBeVisible();
  await expect(page.locator('#query')).toBeEnabled();
  await expect(page.locator('#query')).toHaveValue('保持する検索語');
  await expect(page.locator('#advanced-conditions')).toBeHidden();
  await expect(page.locator('#batch-input')).toBeHidden();
  await page.locator('#search-mode').selectOption('conditions');
  const scopeWidth = await page.locator('#condition-scope').evaluate(node => ({
    select: node.getBoundingClientRect().width, field: node.parentElement.getBoundingClientRect().width,
  }));
  expect(scopeWidth.select).toBeCloseTo(scopeWidth.field, 0);
  await page.locator('#all-terms').fill('alpha\nbeta');
  await page.locator('#any-terms').fill('keep');
  await page.locator('#not-terms').fill('skip');
  await page.locator('#condition-scope').selectOption('file');
  const search = await ui.search();
  expect(search.request).toMatchObject({ query: '', querySpec: { mode: 'conditions', scope: 'file', all: ['alpha', 'beta'], any: ['keep'], not: ['skip'] } });
});

test('U05: 境界の入力拒否を適切な欄へ返し再検索可能にする', async ({ ui, page }) => {
  await ui.open();
  await ui.queue('start_search', { error: { field: 'rootDirectory', message: '存在しないフォルダです' } });
  await ui.search();
  await expect(page.locator('#root-error')).toHaveText('存在しないフォルダです');
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

test('U09: 操作した行の現在パスで参照し選択・取消・失敗を扱う', async ({ ui, page }) => {
  const folderKey = 'docs-search.folders.v1';
  await ui.open({ storage: { [folderKey]: JSON.stringify({ roots: ['C:\\復元 空白', 'C:\\other'], excluded: ['C:\\skip'] }) } });
  await ui.queue('pick_folder', { value: null });
  await page.locator('#root-folders .folder-browse').first().click();
  expect(await ui.lastCall('pick_folder')).toEqual({ initialDirectory: 'C:\\復元 空白' });
  await expect(page.locator('#root')).toHaveValue('C:\\復元 空白');
  await page.locator('#root-folders input').nth(1).fill('C:\\手入力 空白');
  await ui.queue('pick_folder', { value: 'C:\\picked' });
  await page.locator('#root-folders .folder-browse').nth(1).click();
  expect(await ui.lastCall('pick_folder', 2)).toEqual({ initialDirectory: 'C:\\手入力 空白' });
  await expect(page.locator('#root-folders input').nth(1)).toHaveValue('C:\\picked');
  await expect(page.locator('#root')).toHaveValue('C:\\復元 空白');
  await expect(page.locator('#excluded-folders input')).toHaveValue('C:\\skip');
  expect(await page.evaluate(key => JSON.parse(localStorage.getItem(key)), folderKey)).toEqual({ roots: ['C:\\復元 空白', 'C:\\picked'], excluded: ['C:\\skip'] });
  await ui.queue('pick_folder', { value: null });
  await page.locator('#root-folders .folder-browse').nth(1).click();
  await ui.lastCall('pick_folder', 3);
  await expect(page.locator('#root-folders input').nth(1)).toHaveValue('C:\\picked');
  await ui.queue('pick_folder', { error: 'picker denied' });
  await page.locator('#root-folders .folder-browse').first().click();
  await expect(page.locator('#general-error')).toContainText('フォルダを選択できません');
  await expect(page.locator('#root')).toHaveValue('C:\\復元 空白');
  await ui.queue('pick_folder', { value: 'C:\\excluded' });
  await page.locator('#excluded-folders .folder-browse').click();
  expect(await ui.lastCall('pick_folder', 5)).toEqual({ initialDirectory: 'C:\\skip' });
  await expect(page.locator('#excluded-folders input')).toHaveValue('C:\\excluded');
  await page.locator('#add-root').click();
  await ui.queue('pick_folder', { value: null });
  await page.locator('#root-folders .folder-browse').last().click();
  expect(await ui.lastCall('pick_folder', 6)).toEqual({ initialDirectory: '' });
  await expect(page.locator('#root-folders input').last()).toHaveValue('');
  await page.locator('#clear-index').click();
  await ui.lastCall('clear_search_index');
  await expect(page.locator('#copy-status')).toContainText('索引');
});

for (const mode of ['standard', 'conditions', 'batch']) {
  test(`U43: ${mode}の検索で空欄を整理し入力順・操作・保存復元を保つ`, async ({ ui, page }) => {
    await ui.open();
    const roots = page.locator('#root-folders input');
    for (const value of ['C:\\日本語 空白', '', ' \t ', 'C:\\last', '']) {
      await page.locator('#add-root').click();
      await roots.last().fill(value);
    }
    await page.locator('#search-mode').selectOption(mode);
    if (mode === 'standard') await page.locator('#query').fill('beacon');
    if (mode === 'conditions') await page.locator('#all-terms').fill('beacon');
    if (mode === 'batch') await page.locator('#batch-terms').fill('beacon');
    // Enter and the Search button share the same cleanup path.
    if (mode === 'standard') await page.locator('#query').press('Enter');
    else await page.locator('#search').click();
    const search = await ui.lastCall('start_search');
    expect(search.request).toMatchObject({ rootDirectory: 'C:\\日本語 空白', additionalDirectories: ['C:\\last'] });
    expect(await roots.evaluateAll(nodes => nodes.map(node => node.value))).toEqual(['C:\\日本語 空白', 'C:\\last']);
    await expect(page.locator('#root')).toHaveValue('C:\\日本語 空白');
    await expect(roots.nth(1)).toHaveAttribute('aria-label', '検索フォルダ 2');
    await expect(page.locator('#root-folders .folder-browse').nth(1)).toHaveAttribute('aria-label', '検索フォルダ 2 を参照');
    await expect(page.locator('#root-folders .folder-remove')).toHaveAttribute('aria-label', '検索フォルダ 2 を削除');
    await ui.started(search);
    await ui.finish(search);
    await page.locator('#root').focus();
    await page.locator('#root').press('Tab');
    await expect(page.locator('#root-folders .folder-browse').first()).toBeFocused();
    await page.locator('#root-folders .folder-browse').first().press('Tab');
    await expect(page.locator('#root-folders .folder-history').first()).toBeFocused();
    await page.locator('#root-folders .folder-history').first().press('Tab');
    await expect(roots.nth(1)).toBeFocused();
    await page.locator('#add-root').click();
    await expect(roots).toHaveCount(3);
    await expect(roots.last()).toBeFocused();
    await ui.queue('pick_folder', { value: 'C:\\new' });
    await page.locator('#root-folders .folder-browse').last().click();
    expect(await ui.lastCall('pick_folder')).toEqual({ initialDirectory: '' });
    await expect(roots.last()).toHaveValue('C:\\new');
    await page.locator('#root-folders .folder-remove').last().click();
    await page.locator('#add-root').click();
    await page.locator('#search').click();
    const repeated = await ui.lastCall('start_search', 2);
    expect(repeated.request).toMatchObject({ rootDirectory: 'C:\\日本語 空白', additionalDirectories: ['C:\\last'] });
    await ui.started(repeated);
    await ui.finish(repeated);
    expect(await page.evaluate(() => JSON.parse(localStorage.getItem('docs-search.folders.v1')))).toEqual({ roots: ['C:\\日本語 空白', 'C:\\last'], excluded: [] });
    await page.reload();
    await page.waitForFunction(() => window.__docsSearchTest?.ready);
    await expect(page.locator('#search')).toBeEnabled();
    expect(await roots.evaluateAll(nodes => nodes.map(node => node.value))).toEqual(['C:\\日本語 空白', 'C:\\last']);
    expect(await ui.calls('start_search')).toEqual([]);
  });
}

test('U44: 全空欄は1欄で指定を促し、他の入力拒否より先に空欄を整理する', async ({ ui, page }) => {
  await ui.open();
  const roots = page.locator('#root-folders input');
  await page.locator('#search').click();
  await expect(page.locator('#root-error')).toHaveText('検索フォルダを指定してください。');
  for (const value of ['', ' \t ']) {
    await page.locator('#add-root').click();
    await roots.last().fill(value);
  }
  await page.locator('#root').fill(' \t ');
  await page.locator('#search').click();
  await expect(roots).toHaveCount(1);
  await expect(page.locator('#root')).toHaveValue('');
  await expect(page.locator('#root-error')).toHaveText('検索フォルダを指定してください。');
  expect(await ui.calls('start_search')).toEqual([]);
  // A nonempty invalid path must remain available for correction.
  await page.locator('#root').fill('C:\\missing');
  await page.locator('#add-root').click();
  await page.locator('#query').press('Enter');
  await expect(roots).toHaveCount(1);
  await expect(page.locator('#root')).toHaveValue('C:\\missing');
  await expect(page.locator('#query-error')).not.toBeEmpty();
  await page.locator('#query').fill('beacon');
  await page.locator('#add-excluded').click();
  await page.locator('#search').click();
  await expect(page.locator('#excluded-error')).not.toBeEmpty();
  await expect(page.locator('#excluded-folders input')).toHaveCount(1);
  expect(await ui.calls('start_search')).toEqual([]);
  await page.locator('#excluded-folders .folder-remove').click();
  await ui.queue('start_search', { error: { field: 'rootDirectory', message: '存在するフォルダを指定してください。' } });
  await page.locator('#search').click();
  expect((await ui.lastCall('start_search')).request.rootDirectory).toBe('C:\\missing');
  await expect(page.locator('#root-error')).toHaveText('存在するフォルダを指定してください。');
  await expect(page.locator('#root')).toHaveValue('C:\\missing');
  await page.locator('#root').fill('C:\\valid');
  await page.locator('#search').click();
  const accepted = await ui.lastCall('start_search', 2);
  expect(accepted.request.rootDirectory).toBe('C:\\valid');
  await ui.started(accepted);
  await ui.finish(accepted);
});
