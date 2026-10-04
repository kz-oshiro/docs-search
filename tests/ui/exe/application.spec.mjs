import { test, expect } from './fixtures.mjs';
import fs from 'node:fs/promises';
import path from 'node:path';

test('E01: 配布Release資材と実Tauri購読が起動する', async ({ app }) => {
  await expect(app.page.locator('#root')).toBeEditable();
  await expect(app.page.locator('#query')).toBeEditable();
  await expect(app.page.locator('#status')).toHaveText('待機中');
  expect(await app.events()).toEqual([]);
  expect(await app.page.evaluate(() => ({
    invoke: typeof window.__TAURI__.core.invoke, listen: typeof window.__TAURI__.event.listen,
    mock: typeof window.__docsSearchTest,
  }))).toEqual({ invoke: 'function', listen: 'function', mock: 'undefined' });
});

test('E02: Unicode検索と実InputErrorの往復', async ({ app }) => {
  const sample = await app.copy('common', 'search/notes/research-log.txt', '日本語 sample 😀.txt');
  const search = await app.search(app.documents, 'CAFÉ');
  expect(search.request.rootDirectory).toBe(app.documents);
  expect(search.request.query).toBe('CAFÉ');
  const { hits, finish } = await app.finished(search);
  expect(hits).toHaveLength(1);
  expect(hits[0].filePath).toBe(sample);
  expect(hits[0].location.lineNumber).toBe(71);
  await expect(app.page.locator('.preview')).toContainText('cafe\u0301');
  await expect(app.page.locator('#counts')).toContainText(`結果 ${finish.counts.resultCount}`);
  const offset = (await app.events()).length;
  await app.page.locator('#root').fill(path.join(app.documents, '存在しない'));
  await app.page.locator('#search').click();
  await expect(app.page.locator('#root-error')).not.toBeEmpty();
  await expect(app.page.locator('#status')).toHaveText('入力を確認してください。');
  await expect(app.page.locator('#search')).toBeEnabled();
  expect((await app.events()).slice(offset)).toEqual([]);
});

test('E04: 索引削除コマンドが試験用DBへ到達する', async ({ app }) => {
  await app.copy('common', 'search/notes/research-log.txt', 'sample.txt');
  await app.page.locator('#use-index').check();
  await app.finished(await app.search(app.documents, 'CAFÉ'));
  const database = path.join(app.localAppData, 'docs-search', 'search-index.sqlite3');
  expect((await fs.stat(database)).size).toBeGreaterThan(0);
  await app.page.locator('#clear-index').click();
  await expect(app.page.locator('#copy-status')).toContainText('索引を削除しました');
  await expect.poll(async () => fs.access(database).then(() => true, error => {
    if (error.code === 'ENOENT') return false;
    throw error;
  })).toBe(false);
});

test('E06: 実一括プレビューと語ID別通知の接続', async ({ app }) => {
  await app.copy('office', 'search', 'office');
  await app.page.locator('#search-mode').selectOption('batch');
  await app.page.locator('#batch-match-mode').selectOption('text');
  await app.page.locator('#batch-terms').fill('TAB_01\nBODY_SIGNAL');
  await expect(app.page.locator('#batch-count')).toHaveText('2 語（空行・重複を除去）');
  const search = await app.search(path.join(app.documents, 'office'), '', { mode: 'batch' });
  const { events, hits } = await app.finished(search);
  const summaries = events.filter(event => event.type === 'batchSummary');
  expect(summaries).toHaveLength(1);
  expect(summaries[0].terms.map(term => ({ termId: term.termId, term: term.term }))).toEqual([
    { termId: 'batch:1', term: 'TAB_01' }, { termId: 'batch:2', term: 'BODY_SIGNAL' },
  ]);
  // This representative count comes from the fixed PPT body and Word paragraph,
  // not a CLI result or a second implementation of matching rules.
  expect(summaries[0].terms[1].hitCount).toBe(2);
  expect(summaries[0].terms[1].fileCount).toBe(2);
  for (const term of summaries[0].terms) {
    const matches = hits.filter(hit => hit.termId === term.termId);
    expect(matches.length).toBeGreaterThan(0);
    expect(matches.every(hit => hit.term === term.term)).toBe(true);
    expect(term.hitCount).toBe(matches.length);
    await expect(app.page.locator('#batch-summary-list')).toContainText(term.term);
  }
});

test('E07: 実検索スレッド中断と終了後の次検索受付', async ({ app }) => {
  await app.copy('common', 'load', 'load');
  const sample = await app.copy('common', 'search/notes/research-log.txt', 'next/sample.txt');
  // Trigger the real cancel button at the first delivered result rather than
  // racing an arbitrary delay against the Rust worker. No event is injected.
  await app.page.evaluate(() => { window.__docsSearchExeObservation.cancelOnResult = true; });
  const search = await app.search(path.join(app.documents, 'load'), 'beacon');
  const { events, hits } = await app.finished(search, 'cancelled');
  expect(events.some(event => event.type === 'progress')).toBe(true);
  expect(hits.length).toBeGreaterThan(0);
  await expect(app.page.locator('#status')).toHaveText('中断');
  const next = await app.search(path.dirname(sample), 'CAFÉ');
  expect(next.searchId).not.toBe(search.searchId);
  expect((await app.finished(next)).hits).toHaveLength(1);
});

test('E08: 実結果IDからExcel周辺と変更拒否を取得する', async ({ app }) => {
  const workbook = await app.copy('context', 'row-window.xlsx', 'row-window.xlsx');
  const { hits } = await app.finished(await app.search(app.documents, 'context-needle'));
  expect(hits).toHaveLength(1);
  expect(hits[0].location.cellAddress).toBe('B12');
  await app.page.locator('.excel-context-toggle').click();
  await expect(app.page.locator('.excel-context-table')).toContainText('context-needle');
  await app.page.getByRole('button', { name: '次の列 →', exact: true }).click();
  await expect(app.page.locator('.excel-context-table')).toContainText('L'.repeat(20));
  // Change size as well as mtime so no timestamp resolution assumption is needed.
  await fs.appendFile(workbook, Buffer.from('external change'));
  await app.page.getByRole('button', { name: '← 前の列', exact: true }).click();
  await expect(app.page.locator('.excel-context-error')).toContainText('検索後');
  await expect(app.page.locator('#status')).toHaveText('完了');
});

async function loadEdit(app) {
  await app.page.locator('.edit-result').first().click();
  await app.page.locator('#edit-load').click();
  await expect(app.page.locator('#edit-save')).toBeEnabled();
  await expect(app.page.locator('#edit-replacement')).toHaveValue('対象');
}

test('E09: 実編集tokenの取消・保存と外部変更拒否', async ({ app }) => {
  const sample = await app.copy('issues', 'left/same/multiple.txt', 'sample.txt');
  const original = await fs.readFile(sample);
  await app.finished(await app.search(app.documents, '対象'));
  await loadEdit(app);
  await app.page.locator('#edit-replacement').fill('保存しない');
  await app.page.locator('#edit-cancel').click();
  await expect(app.page.getByRole('dialog')).toBeHidden();
  expect(await fs.readFile(sample)).toEqual(original);
  await loadEdit(app);
  await app.page.locator('#edit-replacement').fill('置換 😀');
  await app.page.locator('#edit-save').click();
  await expect(app.page.getByRole('dialog')).toBeHidden();
  expect(await fs.readFile(sample)).toEqual(Buffer.from('置換 😀 keep\r\n対象 skip\r\n対象 keep skip\r\n対象 KEEP\r\n'));
  await app.finished(await app.search(app.documents, '対象'));
  await loadEdit(app);
  const external = Buffer.from('外部の変更\r\n対象 external\r\n');
  await fs.writeFile(sample, external);
  await app.page.locator('#edit-replacement').fill('保持する入力');
  await app.page.locator('#edit-save').click();
  await expect(app.page.locator('#edit-error')).not.toBeEmpty();
  await expect(app.page.getByRole('dialog')).toBeVisible();
  await expect(app.page.locator('#edit-replacement')).toHaveValue('保持する入力');
  expect(await fs.readFile(sample)).toEqual(external);
  await app.page.locator('#edit-cancel').click();
});

test('E10: EXE再起動後に実プロファイル復元と無効パス検証', async ({ app }) => {
  const removed = path.join(app.documents, '終了中に削除');
  await fs.mkdir(removed);
  await app.page.locator('#root').fill(app.documents);
  await app.page.locator('#add-root').click();
  await app.page.locator('#root-folders input').nth(1).fill(removed);
  await app.page.locator('#add-excluded').click();
  await app.page.locator('#excluded-folders input').fill(app.localAppData);
  await app.page.locator('#appearance-settings summary').click();
  await app.page.locator('#theme-color').selectOption('teal');
  await app.stop({ graceful: true });
  await fs.rmdir(removed); // Only this explicitly created empty folder is removed.
  await app.launch();
  await expect(app.page.locator('#root-folders input')).toHaveCount(2);
  await expect(app.page.locator('#root-folders input').nth(0)).toHaveValue(app.documents);
  await expect(app.page.locator('#root-folders input').nth(1)).toHaveValue('');
  await expect(app.page.locator('#excluded-folders input')).toHaveValue(app.localAppData);
  await expect(app.page.locator('html')).toHaveAttribute('data-theme', 'teal');
  await expect(app.page.locator('#folder-save-status')).toContainText('無効になった');
  await expect(app.page.locator('#status')).toHaveText('待機中');
  expect(await app.events()).toEqual([]);
});

test('E11: 実複数バッチの205結果に欠落・重複・終端先行がない', async ({ app }) => {
  await app.copy('issues', 'pages', 'pages');
  const root = path.join(app.documents, 'pages');
  const expected = Array.from({ length: 205 }, (_, index) => path.join(root, `${String(index).padStart(3, '0')}.txt`)).sort();
  const { hits, finish } = await app.finished(await app.search(root, '対象'));
  expect(hits.map(hit => hit.filePath).sort()).toEqual(expected);
  expect(new Set(hits.map(hit => hit.resultId)).size).toBe(205);
  expect(finish.counts.resultCount).toBe(205);
  expect((await app.batches()).filter(size => size > 1).length).toBeGreaterThan(1);
  await expect(app.page.locator('#counts')).toContainText('結果 205');
  await expect(app.page.locator('#result-filter-summary')).toContainText('205 / 205 件');
  await expect(app.page.locator('.result')).toHaveCount(200);
  await app.page.locator('#more').click();
  await expect(app.page.locator('.result')).toHaveCount(205);
  expect((await app.page.locator('.path').allTextContents()).sort()).toEqual(expected);
});
