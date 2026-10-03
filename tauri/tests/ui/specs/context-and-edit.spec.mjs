import { test, expect, editableHit, editResponse, excelHit, contextResponse, hit, evidence } from '../support/fixtures.mjs';

async function editableSearch(ui) {
  await ui.open();
  const search = await ui.search();
  await ui.started(search);
  await ui.finish(search, [editableHit()]);
  return search;
}

test('U18: Excel周辺の取得・結合・非表示・書式反映と列移動', async ({ ui, page }) => {
  await ui.open();
  const search = await ui.search();
  await ui.started(search);
  await ui.finish(search, [excelHit()]);
  await ui.queue('get_result_context', { value: contextResponse() });
  await page.getByRole('button', { name: '周辺を表示', exact: true }).click();
  expect(await ui.lastCall('get_result_context')).toEqual({ searchId: search.searchId, resultId: 1, range: null });
  const cell = page.locator('.excel-context-table td').first();
  await expect(cell.locator('mark')).toHaveText('beacon');
  await expect(cell).toHaveAttribute('colspan', '2');
  await expect(cell).toHaveCSS('font-weight', '700');
  await expect(cell).toHaveCSS('color', 'rgb(17, 34, 51)');
  await expect(cell).toHaveCSS('background-color', 'rgb(255, 238, 204)');
  await expect(page.locator('.excel-context-table')).toContainText('C (非表示)');
  await expect(page.locator('.excel-context-table')).toContainText('2 (非表示)');
  await expect(page.getByRole('button', { name: '← 前の列', exact: true })).toBeDisabled();
  await ui.queue('get_result_context', { value: contextResponse({ firstColumn: 6, cells: [], merges: [] }) });
  await page.getByRole('button', { name: '次の列 →', exact: true }).click();
  expect((await ui.lastCall('get_result_context', 2)).range).toEqual({ firstRow: 1, firstColumn: 6, rowCount: 5, columnCount: 5 });
  await expect(page.locator('.excel-context-controls')).toContainText('F〜J 列');
  await ui.queue('get_result_context', { value: contextResponse() });
  await page.getByRole('button', { name: '← 前の列', exact: true }).click();
  expect((await ui.lastCall('get_result_context', 3)).range.firstColumn).toBe(1);
  await page.getByRole('button', { name: '周辺を閉じる', exact: true }).click();
  await expect(page.locator('.excel-context')).toBeHidden();
});

test('U19: 周辺失敗を局所表示、アンカーなし図形は要求しない', async ({ ui, page }) => {
  await ui.open();
  const search = await ui.search();
  await ui.started(search);
  await ui.finish(search, [excelHit(), excelHit(2, { sourceKind: 'shape', anchor: null, location: { sheetName: 'Sheet1', shapeName: '図形1' } })]);
  await expect(page.locator('.excel-context-unavailable')).toContainText('アンカー位置がない');
  await expect(page.locator('.excel-context-toggle')).toHaveCount(1);
  await ui.queue('get_result_context', { error: { code: 'changedSinceSearch', message: '検索後に変更されました。再検索してください。' } });
  await page.locator('.excel-context-toggle').click();
  await expect(page.locator('.excel-context-error')).toContainText('再検索してください');
  await expect(page.locator('#status')).toHaveText('完了');
});

test('U20: 閉じた周辺窓へ遅延応答を描画しない', async ({ ui, page }) => {
  await ui.open();
  const search = await ui.search();
  await ui.started(search);
  await ui.finish(search, [excelHit()]);
  await ui.queue('get_result_context', { deferred: 'context' });
  await page.locator('.excel-context-toggle').click();
  await ui.pending('context');
  await page.locator('.excel-context-toggle').click();
  await ui.resolve('context', { value: contextResponse() });
  await expect(page.locator('.excel-context')).toBeHidden();
  await expect(page.locator('.excel-context-table')).toHaveCount(0);
});

test('U21: 別検索開始後の遅延周辺応答は新結果を変更しない', async ({ ui, page }) => {
  await ui.open();
  const old = await ui.search();
  await ui.started(old);
  await ui.finish(old, [excelHit()]);
  await ui.queue('get_result_context', { deferred: 'old-context' });
  await page.locator('.excel-context-toggle').click();
  await ui.pending('old-context');
  const current = await ui.search();
  await ui.started(current);
  await ui.finish(current, [hit(2)]);
  await ui.resolve('old-context', { value: contextResponse() });
  await expect(page.locator('.excel-context-table')).toHaveCount(0);
  await expect(page.locator('.file-name')).toHaveText('sample.txt');
});

test('U22: 編集読込・現在ID/tokenの保存要求と再検索案内', async ({ ui, page }) => {
  const search = await editableSearch(ui);
  await page.locator('.edit-result').click();
  await expect(page.getByRole('dialog')).toBeVisible();
  await expect(page.locator('#edit-save')).toBeDisabled();
  await ui.queue('prepare_result_edit', { value: editResponse('token-current') });
  await page.locator('#edit-load').click();
  expect(await ui.lastCall('prepare_result_edit')).toEqual({ searchId: search.searchId, resultId: 1, evidenceIndex: null });
  await expect(page.locator('#edit-context')).toHaveText('before 【beacon】 after');
  await expect(page.locator('#edit-location')).toContainText('UTF-8');
  await expect(page.locator('#search')).toBeDisabled();
  await page.locator('#edit-replacement').fill('置換 😀');
  await ui.queue('save_result_edit', { value: null });
  await page.locator('#edit-save').click();
  expect(await ui.lastCall('save_result_edit')).toEqual({ searchId: search.searchId, token: 'token-current', replacement: '置換 😀' });
  await expect(page.getByRole('dialog')).toBeHidden();
  await expect(page.locator('.edited-notice')).toContainText('再検索が必要');
  await expect(page.locator('.edit-result')).toHaveCount(0);
  await expect(page.locator('#search')).toBeEnabled();
});

for (const close of ['button', 'Escape']) {
  test(`U23: 編集キャンセル ${close} はtokenを破棄し起点へ戻る`, async ({ ui, page }) => {
    const search = await editableSearch(ui);
    await page.locator('.edit-result').click();
    await ui.queue('prepare_result_edit', { value: editResponse() });
    await page.locator('#edit-load').click();
    await expect(page.locator('#edit-replacement')).toBeFocused();
    await page.locator('#edit-replacement').fill('保存しない');
    if (close === 'button') await page.locator('#edit-cancel').click();
    else await page.locator('#edit-replacement').press('Escape');
    expect(await ui.lastCall('cancel_result_edit')).toEqual({ searchId: search.searchId, token: 'edit-token' });
    expect(await ui.calls('save_result_edit')).toEqual([]);
    await expect(page.locator('.edit-result')).toBeFocused();
    await expect(page.getByRole('dialog')).toBeHidden();
    await expect(page.locator('#search')).toBeEnabled();
  });
}

test('U24: 保存拒否は入力・ダイアログを保持し再試行できる', async ({ ui, page }) => {
  await editableSearch(ui);
  await page.locator('.edit-result').click();
  await ui.queue('prepare_result_edit', { value: editResponse() });
  await page.locator('#edit-load').click();
  await expect(page.locator('#edit-save')).toBeEnabled();
  await page.locator('#edit-replacement').fill('retry text');
  await ui.queue('save_result_edit', { error: { message: '外部変更を検出しました。再検索してください。' } });
  await page.locator('#edit-save').click();
  await expect(page.locator('#edit-error')).toContainText('外部変更');
  await expect(page.locator('#edit-replacement')).toHaveValue('retry text');
  await expect(page.getByRole('dialog')).toBeVisible();
  await expect(page.locator('#edit-save')).toBeEnabled();
  await page.locator('#edit-cancel').click();
});

test('U25: 根拠切替で入力を保持、再読込で旧tokenを破棄し選択根拠を渡す', async ({ ui, page }) => {
  await ui.open();
  const search = await ui.search();
  await ui.started(search);
  const entries = ['alpha', 'beta'].map((term, index) => evidence(term, index + 1, {
    editAnchor: { ...editableHit(index + 1).editAnchor, range: [0, Array.from(term).length] },
  }));
  await ui.finish(search, [hit(1, { evidence: entries, sourceKind: 'fileMatch' })]);
  await page.locator('.edit-result').click();
  await ui.queue('prepare_result_edit', { value: editResponse('alpha-token', 'alpha') });
  await page.locator('#edit-load').click();
  await expect(page.locator('#edit-replacement')).toHaveValue('alpha');
  await page.locator('#edit-replacement').fill('retain');
  await page.locator('#edit-evidence').selectOption('1');
  await expect(page.locator('#edit-replacement')).toHaveValue('retain');
  await expect(page.locator('#edit-save')).toBeDisabled();
  await ui.queue('prepare_result_edit', { value: editResponse('beta-token', 'beta') });
  await page.locator('#edit-load').click();
  expect(await ui.lastCall('cancel_result_edit')).toEqual({ searchId: search.searchId, token: 'alpha-token' });
  expect(await ui.lastCall('prepare_result_edit', 2)).toEqual({ searchId: search.searchId, resultId: 1, evidenceIndex: 1 });
  await expect(page.locator('#edit-replacement')).toHaveValue('beta');
  await ui.queue('save_result_edit', { value: null });
  await page.locator('#edit-save').click();
  expect((await ui.lastCall('save_result_edit')).token).toBe('beta-token');
});

test('U26: 編集取得失敗・非対応・検索中の編集は保存要求を送らない', async ({ ui, page }) => {
  await ui.open();
  const search = await ui.search();
  await ui.started(search);
  await ui.emit(search, [{ type: 'result', hit: editableHit() }]);
  await page.locator('.edit-result').click();
  await expect(page.locator('#general-error')).toContainText('検索終了後');
  await expect(page.getByRole('dialog')).toBeHidden();
  await ui.finish(search, [excelHit(2)]);
  await expect(page.locator('.edit-unavailable')).toContainText('Office形式');
  await page.locator('.edit-result').click();
  await ui.queue('prepare_result_edit', { error: { message: '検索後に変更されました' } });
  await page.locator('#edit-load').click();
  await expect(page.locator('#edit-error')).toHaveText('検索後に変更されました');
  await expect(page.locator('#edit-save')).toBeDisabled();
  await page.locator('#edit-cancel').click();
  expect(await ui.calls('save_result_edit')).toEqual([]);
});
