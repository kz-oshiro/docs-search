import { test, expect, hit, editableHit } from '../support/fixtures.mjs';

const folderKey = 'docs-search.folders.v1';
const folders = { roots: ['C:\\first', 'C:\\missing', 'C:\\last'], excluded: ['C:\\skip'] };

for (const theme of ['sun', 'amber', 'sunset', 'teal', 'blue', 'forest']) {
  test(`U32: ${theme}テーマは即時反映し再読込で復元`, async ({ ui, page }) => {
    await ui.open();
    await expect(page.locator('html')).toHaveAttribute('data-theme', 'sun');
    await page.locator('#appearance-settings summary').click();
    await page.locator('#theme-color').selectOption(theme);
    await expect(page.locator('html')).toHaveAttribute('data-theme', theme);
    expect(await page.evaluate(() => localStorage.getItem('docs-search.theme'))).toBe(theme);
    await page.reload();
    await page.waitForFunction(() => window.__docsSearchTest?.ready);
    await expect(page.locator('html')).toHaveAttribute('data-theme', theme);
    await expect(page.locator('#theme-color')).toHaveValue(theme);
    expect(await ui.calls('start_search')).toEqual([]);
  });
}

for (const [saved, expected] of [['teal', 'teal'], ['invalid', 'sun']]) {
  test(`U33: 保存色 ${saved} の継承・不正値フォールバック`, async ({ ui, page }) => {
    await ui.open({ storage: { 'docs-search.theme': saved } });
    await expect(page.locator('html')).toHaveAttribute('data-theme', expected);
    await expect(page.locator('#theme-color')).toHaveValue(expected);
  });
}

test('U34: 検索中のテーマ変更は結果・条件・絞り込みを保持', async ({ ui, page }) => {
  await ui.open();
  const search = await ui.search();
  await ui.started(search);
  await ui.emit(search, [{ type: 'result', hit: hit(1) }]);
  await page.locator('#result-filter-include').fill('beacon');
  await page.locator('#appearance-settings summary').click();
  await page.locator('#theme-color').selectOption('blue');
  await expect(page.locator('.match')).toHaveCount(1);
  await expect(page.locator('#query')).toHaveValue('beacon');
  await expect(page.locator('#result-filter-include')).toHaveValue('beacon');
  await expect(page.locator('#cancel')).toBeEnabled();
  expect(await ui.calls('start_search')).toHaveLength(1);
  await page.locator('#theme-color').press('Escape');
  await expect(page.locator('#appearance-settings')).not.toHaveAttribute('open', '');
  await expect(page.locator('#appearance-settings summary')).toBeFocused();
});

for (const failure of ['storageReadError', 'storageWriteError']) {
  test(`U35: ${failure}でも画面起動と検索・一時テーマ変更が可能`, async ({ ui, page }) => {
    await ui.open({ [failure]: true });
    await page.locator('#appearance-settings summary').click();
    if (failure === 'storageReadError') await expect(page.locator('#theme-save-status')).toContainText('この起動中だけ');
    await page.locator('#theme-color').selectOption('forest');
    await expect(page.locator('html')).toHaveAttribute('data-theme', 'forest');
    if (failure === 'storageWriteError') await expect(page.locator('#theme-save-status')).toContainText('この起動中だけ');
    const search = await ui.search();
    await ui.started(search);
    await ui.finish(search, [hit(1)]);
    await expect(page.locator('#status')).toHaveText('完了');
  });
}

test('U36: フォルダーの欄数・順序を復元、無効欄だけ空にして検索しない', async ({ ui, page }) => {
  await ui.open({ storage: { [folderKey]: JSON.stringify(folders) }, responses: { validate_folder_paths: [{ value: ['C:\\first', '', 'C:\\last', 'C:\\skip'] }] } });
  expect(await page.locator('#root-folders input').evaluateAll(nodes => nodes.map(node => node.value))).toEqual(['C:\\first', '', 'C:\\last']);
  await expect(page.locator('#excluded-folders input')).toHaveValue('C:\\skip');
  await expect(page.locator('#folder-save-status')).toContainText('無効になったフォルダー欄を空欄');
  expect(await ui.calls('start_search')).toEqual([]);
  expect(await ui.lastCall('validate_folder_paths')).toEqual({ paths: [...folders.roots, ...folders.excluded] });
  await page.locator('#root-folders input').nth(1).fill('C:\\new');
  expect(await page.evaluate(key => JSON.parse(localStorage.getItem(key)), folderKey)).toEqual({ roots: ['C:\\first', 'C:\\new', 'C:\\last'], excluded: ['C:\\skip'] });
});

test('U36b: 入力したルート・除外欄の件数と順序を再読込で復元', async ({ ui, page }) => {
  await ui.open();
  await page.locator('#root').fill('C:\\first');
  await page.locator('#add-root').click();
  await page.locator('#root-folders input').nth(1).fill('C:\\new');
  await page.locator('#add-excluded').click();
  await page.locator('#excluded-folders input').fill('C:\\skip');
  await page.reload();
  await page.waitForFunction(() => window.__docsSearchTest?.ready);
  await expect(page.locator('#search')).toBeEnabled();
  expect(await page.locator('#root-folders input').evaluateAll(nodes => nodes.map(node => node.value))).toEqual(['C:\\first', 'C:\\new']);
  await expect(page.locator('#excluded-folders input')).toHaveValue('C:\\skip');
  expect(await ui.calls('start_search')).toEqual([]);
});

test('U37: 復元中に入力したパスを遅延検証で上書きしない', async ({ ui, page }) => {
  await ui.open({ storage: { [folderKey]: JSON.stringify(folders) }, responses: { validate_folder_paths: [{ deferred: 'folders' }] } }, { waitForRestore: false });
  await ui.pending('folders');
  await expect(page.locator('#search')).toBeDisabled();
  await page.locator('#root').fill('C:\\typed');
  await ui.resolve('folders', { value: [...folders.roots, ...folders.excluded] });
  await expect(page.locator('#search')).toBeEnabled();
  await expect(page.locator('#root')).toHaveValue('C:\\typed');
  await expect(page.locator('#root-folders input')).toHaveCount(1);
});

test('U38: 壊れたフォルダー保存値は入力継続を妨げない', async ({ ui, page }) => {
  await ui.open({ storage: { [folderKey]: '{broken' } });
  await expect(page.locator('#folder-save-status')).toContainText('読み取れません');
  await page.locator('#root').fill('C:\\typed');
  await expect(page.locator('#root')).toHaveValue('C:\\typed');
});

test('U38b: フォルダー検証拒否後にパスを入力できる', async ({ ui, page }) => {
  await ui.open({ storage: { [folderKey]: JSON.stringify(folders) }, responses: { validate_folder_paths: [{ error: { message: 'denied' } }] } });
  await expect(page.locator('#search')).toBeEnabled();
  await expect(page.locator('#folder-save-status')).toContainText('確認できません');
  await page.locator('#root').fill('C:\\typed');
  await expect(page.locator('#root')).toHaveValue('C:\\typed');
});

test('U39: 最小幅で主要入力・設定・編集ダイアログが横にはみ出さない', async ({ ui, page }) => {
  await page.setViewportSize({ width: 760, height: 900 });
  await ui.open();
  const search = await ui.search();
  await ui.started(search);
  await ui.finish(search, [editableHit()]);
  await page.locator('#appearance-settings summary').click();
  for (const selector of ['#root', '#query', '#appearance-settings .appearance-panel', '#search', '#cancel']) {
    const element = page.locator(selector);
    await expect(element).toBeVisible();
    const box = await element.boundingBox();
    expect(box.x, selector).toBeGreaterThanOrEqual(0);
    expect(box.x + box.width, selector).toBeLessThanOrEqual(760);
  }
  await page.locator('#theme-color').press('Escape');
  await page.locator('.edit-result').click();
  const box = await page.getByRole('dialog').boundingBox();
  expect(box.x).toBeGreaterThanOrEqual(0);
  expect(box.x + box.width).toBeLessThanOrEqual(760);
  await page.locator('#edit-cancel').press('Enter');
  await expect(page.locator('.edit-result')).toBeFocused();
});
