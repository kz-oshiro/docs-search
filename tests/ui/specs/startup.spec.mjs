import { test, expect } from '../support/fixtures.mjs';
import { installTauriMock } from '../support/tauri-mock.mjs';

test('U40: real frontend starts and requests no WASM resources', async ({ ui, page }) => {
  const requests = [];
  page.on('request', request => requests.push(request.url()));
  await ui.open();
  const search = await ui.search();
  await ui.started(search);
  await ui.finish(search);
  expect(requests.some(url => /\.wasm(?:[?#]|$)|\/pkg\//i.test(url))).toBe(false);
  for (const name of ['boot.js', 'app.js', 'view.js', 'tauri.js']) expect(requests.some(url => url.endsWith(`/${name}`))).toBe(true);
});

test('U41: module load failure leaves a visible startup error', async ({ page }, testInfo) => {
  // This case deliberately blocks a resource; it does not use the ui error-free fixture.
  await page.addInitScript(installTauriMock, {});
  await page.route('**/app.js', route => route.abort('failed'));
  await page.goto('/');
  await expect(page.locator('#status')).toContainText('画面を起動できません:');
  await expect(page.locator('#search')).toBeDisabled();
  await testInfo.attach('startup-state.txt', { body: Buffer.from(await page.locator('#status').textContent()), contentType: 'text/plain' });
});

test('U42: CRLF and trailing newline preserve condition request lines', async ({ ui, page }) => {
  await ui.open();
  await page.locator('#search-mode').selectOption('conditions');
  await page.locator('#all-terms').fill('first\r\nsecond\r\n');
  await page.locator('#any-terms').fill('');
  const search = await ui.search();
  expect(search.request.querySpec.all).toEqual(['first', 'second']);
  expect(search.request.querySpec.any).toEqual([]);
  await ui.finish(search);
});
