import assert from 'node:assert/strict';
import { existsSync } from 'node:fs';
import { chromium } from '@playwright/test';
const browser = await chromium.launch({
  executablePath: process.env.CHROMIUM_PATH || (existsSync('/usr/bin/chromium') ? '/usr/bin/chromium' : undefined),
  args: ['--no-sandbox'],
});
try {
  const page = await browser.newPage({ viewport: { width: 1440, height: 1100 } });
  const errors = [];
  page.on('pageerror', error => errors.push(String(error)));
  await page.goto(process.argv[2] || 'http://127.0.0.1:5173');
  await page.locator('#status').filter({ hasText: '完了' }).waitFor();
  assert.equal(await page.locator('#output-chart path').count(), 3);
  const enabled = Number(await page.locator('tr[data-aw=enabled] [data-metric=iae]').textContent());
  const disabled = Number(await page.locator('tr[data-aw=disabled] [data-metric=iae]').textContent());
  assert.ok(enabled > 0 && enabled < disabled * 0.25);
  assert.equal(await page.locator('#vector-output').textContent(), '[0.0,0.0,0.5,1.0]');
  await page.getByRole('tab', { name: 'ブロック図エディタ' }).click();
  await page.locator('#graph-status').filter({ hasText: '完了' }).waitFor();
  assert.equal(await page.locator('#graph-nodes .graph-node').count(), 6);
  assert.equal(await page.locator('#graph-plot path').count(), 2);
  await page.getByLabel('モデルサンプル').selectOption('vector');
  await page.getByRole('button', { name: 'サンプルを読み込む', exact: true }).click();
  await page.locator('#graph-status').filter({ hasText: '完了' }).waitFor();
  assert.equal(await page.locator('#graph-plot path').count(), 4);
  assert.ok((await page.locator('#graph-values').textContent()).includes('vector-output [0]=-1.00000'));
  assert.deepEqual(errors, []);
  if (process.env.BLOCKSIM_SCREENSHOT_PATH) await page.screenshot({ path: process.env.BLOCKSIM_SCREENSHOT_PATH, fullPage: true });
  console.log(`WASM GUI smoke passed: enabled IAE=${enabled}, disabled IAE=${disabled}, vector=[0,0,0.5,1], graph editor + worker + 4 vector traces`);
} finally { await browser.close(); }
