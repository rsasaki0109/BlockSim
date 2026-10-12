import { test, expect, type Page } from '@playwright/test';
import sample from '../../samples/pid-recovery.json' with { type: 'json' };
import { STORAGE_KEY } from '../src/document';
async function ready(page: Page) {
  await page.goto('/');
  await expect(page.locator('#status')).toContainText('完了');
}
test('real WASM runs continuous and discrete comparison, bounded outputs and unlimited equivalence', async ({ page }) => {
  await ready(page);
  const result = await page.evaluate(async (sample) => {
    // Import the generated module, instantiate its binary in Chromium, exercise exports.
    const path = '/src/wasm/blocksim.js';
    const module = await import(/* @vite-ignore */ path);
    await module.default();
    const reports = [];
    for (const mode of ['continuous', 'discrete']) {
      const cfg = { ...sample, mode };
      const comparison = JSON.parse(module.compare_model(JSON.stringify(cfg)));
      const unlimited = { ...cfg, pid: { ...cfg.pid, limits: { lower: null, upper: null } } };
      const baseline = JSON.parse(module.compare_model(JSON.stringify(unlimited)));
      reports.push({
        enabled: comparison.enabled.metrics, disabled: comparison.disabled.metrics,
        count: comparison.enabled.samples.length,
        bounds: [comparison.enabled, comparison.disabled].every((run: { samples: { control: number }[] }) => run.samples.every((s: { control: number }) => s.control >= 0 && s.control <= 1)),
        unchanged: JSON.stringify(baseline.enabled) === JSON.stringify(baseline.disabled),
      });
    }
    return { reports, vector: module.saturate_vector('[-2,0,0.5,2]', '{"lower":0,"upper":1}') };
  }, sample);
  for (const report of result.reports) {
    expect(report.bounds).toBe(true); expect(report.unchanged).toBe(true);
    expect(report.count).toBe(4001);
    expect(report.enabled.recovery_iae).toBeLessThan(report.disabled.recovery_iae * 0.25);
    expect(report.enabled.upper_release_seconds).toBe(0);
  }
  expect(JSON.parse(result.vector)).toEqual([0, 0, 0.5, 1]);
  await expect(page.locator('#output-chart path')).toHaveCount(3);
  await expect(page.locator('#control-chart path')).toHaveCount(2);
  await expect(page.locator('footer')).toContainText('30%');
  await expect(page.locator('footer')).toContainText('主観的な開発評価');
});
test('GUI switches PID mode and AW, persists and restores all settings including vector block', async ({ page }) => {
  await ready(page);
  await page.getByLabel('更新方式').selectOption('discrete');
  await page.getByLabel('anti-windup を有効にする').uncheck();
  await page.getByLabel('PID 下限', { exact: true }).fill('-0.25');
  await page.getByLabel('PID 上限', { exact: true }).fill('0.8');
  await page.getByLabel('入力ベクトル (JSON)').fill('[-3,0,4]');
  await page.getByLabel('Saturation 下限').fill('-1');
  await page.getByLabel('Saturation 上限').fill('2');
  await page.getByRole('button', { name: 'シミュレーション実行' }).click();
  await expect(page.locator('#active-mode')).toHaveText('離散 PID');
  await expect(page.locator('tr[data-aw=disabled]')).toHaveClass('active');
  await expect(page.locator('#vector-output')).toHaveText('[-1.0,0.0,2.0]');
  await page.getByRole('button', { name: 'ブラウザに保存' }).click();
  const saved = await page.evaluate(key => JSON.parse(localStorage.getItem(key)!), STORAGE_KEY);
  expect(saved.model.pid).toMatchObject({ anti_windup: false, limits: { lower: -0.25, upper: 0.8 } });
  await page.reload();
  await expect(page.locator('#status')).toContainText('自動復元');
  await expect(page.getByLabel('更新方式')).toHaveValue('discrete');
  await expect(page.getByLabel('anti-windup を有効にする')).not.toBeChecked();
  await expect(page.getByLabel('PID 下限', { exact: true })).toHaveValue('-0.25');
  await expect(page.getByLabel('PID 上限', { exact: true })).toHaveValue('0.8');
  await expect(page.getByLabel('入力ベクトル (JSON)')).toHaveValue('[-3,0,4]');
  await page.getByRole('button', { name: '復帰比較サンプル' }).click();
  await expect(page.getByLabel('更新方式')).toHaveValue('continuous');
  await page.getByRole('button', { name: '保存設定を復元' }).click();
  await expect(page.getByLabel('更新方式')).toHaveValue('discrete');
});
test('JSON export/import restores bounds and AW, invalid import leaves configuration intact', async ({ page }) => {
  await ready(page);
  await page.getByLabel('PID 下限', { exact: true }).fill('');
  await page.getByLabel('anti-windup を有効にする').uncheck();
  const downloadPromise = page.waitForEvent('download');
  await page.getByRole('button', { name: 'JSON を書き出す' }).click();
  const download = await downloadPromise;
  const filePath = await download.path(); expect(filePath).not.toBeNull();
  await page.getByRole('button', { name: '復帰比較サンプル' }).click();
  await page.locator('#import').setInputFiles(filePath!);
  await expect(page.locator('#status')).toContainText('JSON 設定を復元');
  await expect(page.getByLabel('PID 下限', { exact: true })).toHaveValue('');
  await expect(page.getByLabel('anti-windup を有効にする')).not.toBeChecked();
  const invalid = { ...sample, pid: { ...sample.pid, limits: { lower: 2, upper: 1 } } };
  await page.locator('#import').setInputFiles({ name: 'bad.json', mimeType: 'application/json', buffer: Buffer.from(JSON.stringify(invalid)) });
  await expect(page.locator('#status')).toContainText('Lower limit');
  await expect(page.getByLabel('PID 下限', { exact: true })).toHaveValue('');
  // Legacy fields omitted: normalized by Rust to unlimited and AW disabled.
  const { limits: _limits, anti_windup: _aw, ...legacyPid } = sample.pid;
  await page.locator('#import').setInputFiles({ name: 'legacy.json', mimeType: 'application/json', buffer: Buffer.from(JSON.stringify({ ...sample, pid: legacyPid })) });
  await expect(page.locator('#status')).toContainText('JSON 設定を復元');
  await expect(page.getByLabel('PID 上限', { exact: true })).toHaveValue('');
  await expect(page.getByLabel('anti-windup を有効にする')).not.toBeChecked();
});
test('invalid bounds and corrupted saved settings produce visible errors, no successful run claim', async ({ page }) => {
  await ready(page);
  await page.getByLabel('PID 下限', { exact: true }).fill('2');
  await page.getByRole('button', { name: 'シミュレーション実行' }).click();
  await expect(page.locator('#status')).toContainText('Lower limit');
  await expect(page.locator('#status')).toHaveClass('error');
  await page.getByRole('button', { name: '復帰比較サンプル' }).click();
  await page.getByLabel('入力ベクトル (JSON)').fill('[1,null]');
  await page.getByRole('button', { name: 'ブロックを実行' }).click();
  await expect(page.locator('#status')).toContainText('有限数値');
  await page.evaluate(key => localStorage.setItem(key, '{bad'), STORAGE_KEY);
  await page.reload();
  await expect(page.locator('#status')).toContainText('読み込みに失敗');
  // Default remains usable; bad saved file isn't silently overwritten.
  await page.getByRole('button', { name: '復帰比較サンプル' }).click();
  await expect(page.locator('#status')).toContainText('完了');
  expect(await page.evaluate(key => localStorage.getItem(key), STORAGE_KEY)).toBe('{bad');
});
test('vector block supports one-sided, equal and absent bounds', async ({ page }) => {
  await ready(page);
  await page.getByLabel('Saturation 下限').fill('');
  await page.getByLabel('Saturation 上限').fill('1');
  await page.getByRole('button', { name: 'ブロックを実行' }).click();
  await expect(page.locator('#vector-output')).toHaveText('[-2.0,0.0,0.5,1.0]');
  await page.getByLabel('Saturation 上限').fill('');
  await page.getByRole('button', { name: 'ブロックを実行' }).click();
  await expect(page.locator('#vector-output')).toHaveText('[-2.0,0.0,0.5,2.0]');
  await page.getByLabel('Saturation 下限').fill('0.5');
  await page.getByLabel('Saturation 上限').fill('0.5');
  await page.getByRole('button', { name: 'ブロックを実行' }).click();
  await expect(page.locator('#vector-output')).toHaveText('[0.5,0.5,0.5,0.5]');
});
