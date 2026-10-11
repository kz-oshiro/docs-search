import { test, expect } from '../support/fixtures.mjs';

const historyKey = 'docs-search.folder-history.v1';
const foldersKey = 'docs-search.folders.v1';
const values = page => page.locator('.folder-history-select').allTextContents();
const stored = page => page.evaluate(key => JSON.parse(localStorage.getItem(key) ?? '[]'), historyKey);
const openHistory = (page, index = 0) => page.locator('#root-folders .folder-history').nth(index).click();
async function pick(ui, page, path, index = 0) {
  await ui.queue('pick_folder', { value: path });
  await page.locator('#root-folders .folder-browse').nth(index).click();
  if (path) await expect(page.locator('#root-folders input').nth(index)).toHaveValue(path);
}
async function reload(page) {
  await page.reload();
  await page.waitForFunction(() => window.__docsSearchTest?.ready);
  await expect(page.locator('#search')).toBeEnabled();
}

test('U50: A→B→履歴Aを主・追加欄に反映し、他の入力や検索を変えない', async ({ ui, page }) => {
  await ui.open();
  await page.locator('#add-root').click();
  await page.locator('#root-folders input').nth(1).fill('C:\\additional');
  await page.locator('#add-excluded').click();
  await page.locator('#excluded-folders input').fill('C:\\skip');
  await pick(ui, page, 'C:\\A');
  await pick(ui, page, 'C:\\B');
  const trigger = page.locator('#root-folders .folder-history').first();
  await trigger.focus(); await trigger.press('Enter');
  await expect(page.getByRole('dialog', { name: '検索フォルダの履歴' })).toBeVisible();
  expect(await values(page)).toEqual(['C:\\B', 'C:\\A']);
  await page.getByRole('button', { name: 'C:\\A', exact: true }).click();
  await expect(page.locator('#root')).toHaveValue('C:\\A');
  await expect(page.locator('#root')).toBeFocused();
  expect(await ui.lastCall('validate_folder_paths')).toEqual({ paths: ['C:\\A'] });
  await openHistory(page, 1);
  await expect(page.locator('#folder-history-target')).toHaveText('検索フォルダ 2');
  await page.getByRole('button', { name: 'C:\\B', exact: true }).focus();
  await page.getByRole('button', { name: 'C:\\B', exact: true }).press('Enter');
  await expect(page.locator('#root-folders input').nth(1)).toHaveValue('C:\\B');
  await expect(page.locator('#root')).toHaveValue('C:\\A');
  await expect(page.locator('#excluded-folders input')).toHaveValue('C:\\skip');
  expect(await stored(page)).toEqual(['C:\\B', 'C:\\A']);
  expect(await page.evaluate(key => JSON.parse(localStorage.getItem(key)), foldersKey)).toEqual({ roots: ['C:\\A', 'C:\\B'], excluded: ['C:\\skip'] });
  await openHistory(page, 1);
  await page.locator('#folder-history-close').press('Escape');
  await expect(page.locator('#root-folders .folder-history').nth(1)).toBeFocused();
  expect(await ui.calls('start_search')).toEqual([]);
});

test('U51: 最近順・Windows表記差の重複排除・上限と取消・対象外を扱う', async ({ ui, page }) => {
  const initial = Array.from({ length: 20 }, (_, index) => `C:\\old${index}`);
  await ui.open({ storage: { [historyKey]: JSON.stringify(initial) } });
  await pick(ui, page, 'C:\\New');
  await pick(ui, page, 'c:/NEW/./child/../');
  expect(await stored(page)).toEqual(['c:/NEW/./child/../', ...initial.slice(0, 19)]);
  await pick(ui, page, '\\\\Server\\Share\\Team');
  await pick(ui, page, '\\\\?\\UNC\\server\\share\\TEAM\\');
  await pick(ui, page, '\\\\?\\C:\\NEW\\');
  expect((await stored(page)).slice(0, 2)).toEqual(['\\\\?\\C:\\NEW\\', '\\\\?\\UNC\\server\\share\\TEAM\\']);
  expect(await stored(page)).toHaveLength(20);
  const before = await stored(page);
  await pick(ui, page, null);
  await pick(ui, page, '');
  await ui.queue('pick_folder', { error: 'denied' });
  await page.locator('#root-folders .folder-browse').click();
  await expect(page.locator('#general-error')).toContainText('フォルダを選択できません');
  await page.locator('#add-excluded').click();
  await ui.queue('pick_folder', { value: 'C:\\excluded-only' });
  await page.locator('#excluded-folders .folder-browse').click();
  await expect(page.locator('#excluded-folders input')).toHaveValue('C:\\excluded-only');
  await page.locator('#root').fill('C:\\not-confirmed');
  expect(await stored(page)).toEqual(before);
  await openHistory(page);
  expect(await values(page)).toEqual(before);
  await page.locator('#folder-history-close').click();
});

test('U52: 手入力は受理された検索のStartedだけを記録し、旧検索や入力拒否を無視', async ({ ui, page }) => {
  await ui.open();
  await page.locator('#root').fill('C:\\typed');
  await page.locator('#search').click();
  await expect(page.locator('#query-error')).not.toBeEmpty();
  expect(await stored(page)).toEqual([]);
  await ui.queue('start_search', { error: { field: 'rootDirectory', message: '存在しないフォルダです' } });
  await page.locator('#query').fill('beacon'); await page.locator('#search').click();
  await expect(page.locator('#root-error')).toHaveText('存在しないフォルダです');
  expect(await stored(page)).toEqual([]);
  await page.locator('#add-root').click();
  await page.locator('#root-folders input').nth(1).fill('C:\\additional');
  await page.locator('#search').click();
  const accepted = await ui.lastCall('start_search', 2);
  expect(await stored(page)).toEqual([]);
  await ui.started(accepted);
  await expect.poll(() => stored(page)).toEqual(['C:\\typed', 'C:\\additional']);
  await ui.finish(accepted);
  await page.locator('#root').fill('C:\\fresh'); await page.locator('#search').click();
  const fresh = await ui.lastCall('start_search', 3);
  await ui.emit(accepted, [{ type: 'started', request: { ...accepted.request, rootDirectory: 'C:\\stale' } }]);
  await ui.started(fresh); await ui.finish(fresh);
  expect(await stored(page)).toEqual(['C:\\fresh', 'C:\\additional', 'C:\\typed']);
});

test('U53: 復元と個別・全件削除は入力欄を変えず、再読込で履歴を復活させない', async ({ ui, page }) => {
  const inputs = { roots: ['C:\\current', 'C:\\other'], excluded: ['C:\\skip'] };
  await ui.open({ storage: { [foldersKey]: JSON.stringify(inputs), [historyKey]: JSON.stringify(['C:\\A', 'C:\\B']) } });
  await openHistory(page);
  expect(await values(page)).toEqual(['C:\\A', 'C:\\B']);
  await page.getByRole('button', { name: 'C:\\A を履歴から削除', exact: true }).click();
  await expect(page.getByRole('button', { name: 'C:\\B', exact: true })).toBeFocused();
  await page.locator('#folder-history-close').click(); await reload(page); await openHistory(page);
  expect(await values(page)).toEqual(['C:\\B']);
  await page.locator('#folder-history-clear').click();
  await expect(page.locator('#folder-history-state')).toHaveText('履歴はありません。');
  await expect(page.locator('#folder-history-clear')).toBeDisabled();
  await page.locator('#folder-history-close').click(); await reload(page); await openHistory(page);
  expect(await values(page)).toEqual([]);
  await page.locator('#folder-history-close').click();
  expect(await page.locator('#root-folders input').evaluateAll(nodes => nodes.map(node => node.value))).toEqual(inputs.roots);
  await expect(page.locator('#excluded-folders input')).toHaveValue(inputs.excluded[0]);
  expect(await ui.calls('start_search')).toEqual([]);
});

test('U54: 無効・検証拒否・取消後の遅延応答でも入力を保持し、通常参照を続行', async ({ ui, page }) => {
  await ui.open({ storage: { [historyKey]: JSON.stringify(['C:\\missing']) } });
  await page.locator('#root').fill('C:\\keep'); await openHistory(page);
  await ui.queue('validate_folder_paths', { value: [''] });
  await page.locator('.folder-history-select').click();
  await expect(page.locator('#folder-history-error')).toContainText('フォルダが見つからないか、利用できません');
  await expect(page.locator('#root')).toHaveValue('C:\\keep');
  await ui.queue('validate_folder_paths', { error: 'denied' });
  await page.locator('.folder-history-select').click();
  await expect(page.locator('#folder-history-error')).toContainText('フォルダを確認できません');
  await ui.queue('validate_folder_paths', { deferred: 'history-old' });
  await page.locator('.folder-history-select').click(); await ui.pending('history-old');
  await expect(page.locator('.folder-history-select')).toBeDisabled();
  await expect(page.locator('#folder-history-clear')).toBeDisabled();
  await page.locator('#folder-history-close').press('Escape');
  await page.locator('#root').fill('C:\\new-input'); await openHistory(page);
  await ui.resolve('history-old', { value: ['C:\\missing'] });
  await expect(page.locator('#root')).toHaveValue('C:\\new-input');
  await expect(page.locator('#folder-history-error')).toBeEmpty();
  await page.locator('#folder-history-close').click(); await pick(ui, page, 'C:\\picked');
  await expect(page.locator('#root')).toHaveValue('C:\\picked');
  expect(await ui.calls('start_search')).toEqual([]);
});

for (const saved of ['{broken', '{"paths":[]}', '[null,7,"", "C:\\\\valid", "c:/VALID/"]']) {
  test(`U55: 不正履歴 ${saved} でも既存入力の復元と参照を継続`, async ({ ui, page }) => {
    await ui.open({ storage: { [historyKey]: saved, [foldersKey]: JSON.stringify({ roots: ['C:\\existing'], excluded: [] }) } });
    await expect(page.locator('#root')).toHaveValue('C:\\existing');
    await expect(page.locator('#folder-history-save-status')).not.toHaveText('最近使った検索フォルダを20件まで保存します。');
    await openHistory(page);
    expect(await values(page)).toEqual(saved.startsWith('[null') ? ['C:\\valid'] : []);
    await page.locator('#folder-history-close').click(); await pick(ui, page, 'C:\\new');
    expect((await stored(page))[0]).toBe('C:\\new');
  });
}

for (const failure of ['storageReadError', 'storageWriteError']) {
  test(`U56: ${failure}でも起動中の履歴選択・削除・検索を使える`, async ({ ui, page }) => {
    await ui.open({ [failure]: true });
    await pick(ui, page, 'C:\\A'); await pick(ui, page, 'C:\\B'); await openHistory(page);
    expect(await values(page)).toEqual(['C:\\B', 'C:\\A']);
    await page.getByRole('button', { name: 'C:\\A', exact: true }).click();
    await expect(page.locator('#root')).toHaveValue('C:\\A');
    if (failure === 'storageWriteError') await expect(page.locator('#folder-history-save-status')).toContainText('この起動中だけ');
    await openHistory(page); await page.locator('#folder-history-clear').click();
    await page.locator('#folder-history-close').click();
    const search = await ui.search(); await ui.started(search); await ui.finish(search);
    await openHistory(page); expect(await values(page)).toEqual(['C:\\fixtures']);
    await page.locator('#folder-history-close').click();
  });
}

for (const width of [760, 390]) {
  test(`U57: ${width}pxで同名別パス・長い全文・TabとEscapeを扱う`, async ({ ui, page }) => {
    const longPath = `C:\\${'長いフォルダ'.repeat(60)}\\同名`;
    const paths = [longPath, 'D:\\同名'];
    await page.setViewportSize({ width, height: 900 });
    await ui.open({ storage: { [historyKey]: JSON.stringify(paths) } });
    const root = page.locator('#root'); await root.focus(); await root.press('Tab');
    await expect(page.locator('#root-folders .folder-browse')).toBeFocused();
    await page.locator('#root-folders .folder-browse').press('Tab');
    const trigger = page.locator('#root-folders .folder-history'); await expect(trigger).toBeFocused();
    await trigger.press('Enter');
    expect(await values(page)).toEqual(paths);
    await expect(page.locator('.folder-history-select').first()).toBeFocused();
    await page.locator('.folder-history-select').first().press('Tab');
    await expect(page.locator('.folder-history-remove').first()).toBeFocused();
    const dimensions = await page.locator('#folder-history-dialog').evaluate(node => ({ scroll: node.scrollWidth, client: node.clientWidth, right: node.getBoundingClientRect().right }));
    expect(dimensions.scroll).toBeLessThanOrEqual(dimensions.client);
    expect(dimensions.right).toBeLessThanOrEqual(width);
    await page.locator('.folder-history-remove').first().press('Escape');
    await expect(trigger).toBeFocused();
    expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(width);
  });
}
