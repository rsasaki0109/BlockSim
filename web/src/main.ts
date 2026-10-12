import { mountEditor } from './graph-editor';
import init, { default_model, validate_model, run_model, compare_model, saturate_vector } from './wasm/blocksim';
import { decodeDocument, encodeDocument, parseLimit, parseNumber, parseVector, STORAGE_KEY } from './document';
import { drawChart } from './chart';
import type { AppDocument, Comparison, ModelConfig, SimulationResult } from './types';
import recoverySample from '../../samples/pid-recovery.json';
import './style.css';

const app = document.querySelector<HTMLDivElement>('#app')!;
app.innerHTML = `
  <header><a class="brand" href="./">▦ <span>BlockSim</span></a><span class="pill">CONTROL WORKSPACE · 0.1</span></header>
  <nav class="app-tabs" role="tablist" aria-label="開発画面"><button id="pid-tab" role="tab" aria-selected="true" aria-controls="pid-main">PID Lab</button><button id="editor-tab" role="tab" aria-selected="false" aria-controls="graph-main" disabled>ブロック図エディタ</button></nav>
  <main id="graph-main" role="tabpanel" aria-labelledby="editor-tab" hidden></main>
  <main id="pid-main" role="tabpanel" aria-labelledby="pid-tab">
    <section class="intro"><p class="eyebrow">CONTROL SYSTEMS / 実験室</p><h1>飽和から、すばやく戻る。</h1>
      <p>出力制限と条件付き積分で PID の復帰を比較。Rust + WASM で計算します。</p></section>
    <div class="workspace"><aside>
      <form id="model-form" class="panel">
        <h2>PID コントローラ</h2>
        <label>更新方式<select name="mode"><option value="continuous">連続 PID / RK4</option><option value="discrete">離散 PID</option></select></label>
        <div class="fields three"><label>Kp<input name="kp" type="number" step="any" required></label><label>Ki<input name="ki" type="number" step="any" required></label><label>Kd<input name="kd" type="number" step="any" required></label></div>
        <label>微分フィルタ時定数 (s)<input name="derivative_tau" type="number" step="any" required></label>
        <h3>出力制限</h3><div class="fields"><label>PID 下限<input name="lower" type="number" step="any" placeholder="制限なし"></label><label>PID 上限<input name="upper" type="number" step="any" placeholder="制限なし"></label></div>
        <p class="hint">空欄は制限なし。片側だけの制限も可能です。</p>
        <label class="check"><input name="anti_windup" type="checkbox">anti-windup を有効にする</label>
        <p class="hint">飽和を悪化させる積分を停止。戻す方向の積分は継続します。</p>
        <h3>目標値とプラント</h3>
        <div class="fields"><label>変更前の目標値<input name="initial_reference" type="number" step="any" required></label><label>変更後の目標値<input name="final_reference" type="number" step="any" required></label></div>
        <label>目標変更時刻 (s)<input name="step_time" type="number" step="any" required></label>
        <div class="fields"><label>プラントゲイン<input name="plant_gain" type="number" step="any" required></label><label>プラント時定数 (s)<input name="plant_tau" type="number" step="any" required></label></div>
        <details><summary>シミュレーション設定</summary>
          <div class="fields"><label>刻み幅 (s)<input name="dt" type="number" step="any" required></label><label>終了時刻 (s)<input name="duration" type="number" step="any" required></label></div>
          <div class="fields"><label>初期プラント出力<input name="initial_output" type="number" step="any" required></label><label>整定許容誤差<input name="settling_tolerance" type="number" step="any" required></label></div>
        </details>
        <button type="submit" class="primary" disabled>シミュレーション実行</button>
      </form>
      <section class="panel"><h2>設定を管理</h2><div class="buttons"><button id="sample" disabled>復帰比較サンプル</button><button id="save" disabled>ブラウザに保存</button><button id="restore" disabled>保存設定を復元</button><button id="export" disabled>JSON を書き出す</button><label class="file-button">JSON を読み込む<input id="import" type="file" accept="application/json,.json" disabled></label></div></section>
    </aside><article>
      <div id="status" role="status" aria-label="実行状態" aria-live="polite">WASM を初期化しています…</div>
      <section class="panel flow" aria-label="シミュレーション構成"><div>目標値<br><small>3 → 0.5</small></div><span>→</span><div>Σ 誤差</div><span>→</span><div>PID + 制限<br><small>条件付き積分</small></div><span>→</span><div>一次遅れ<br><small>フィードバック ↩</small></div></section>
      <section class="panel"><div class="section-top"><h2>復帰の比較</h2><span id="active-mode" class="pill"></span></div>
        <p class="hint">同じモデルで有効／無効を計算。選択した設定の線を太く表示します。</p>
        <div class="table-wrap"><table><thead><tr><th>anti-windup</th><th>復帰 IAE</th><th>整定時間 (s)</th><th>上限解除 (s)</th><th>最大 |I|</th></tr></thead><tbody id="metrics"></tbody></table></div>
        <p class="hint">時間は目標変更を観測した時点から。— は観測期間中に未達／上限制限なし。IAE は変更後の絶対誤差積分です。</p>
      </section>
      <section class="panel charts"><div class="legend"><span class="enabled">● AW 有効</span><span class="disabled">● AW 無効</span><span class="reference">┄ 目標値</span></div>
        <h3>プラント出力</h3><div id="output-chart" data-title="プラント出力の比較"></div>
        <h3>PID 出力</h3><div id="control-chart" data-title="PID 出力制限の比較"></div>
        <h3>積分項</h3><div id="integral-chart" data-title="積分項の比較"></div>
      </section>
      <section class="panel"><h2>Saturation ブロック</h2><p class="hint">スカラーは1要素の配列として入力できます。各要素を独立に制限します。</p>
        <label>入力ベクトル (JSON)<textarea id="vector" rows="2">[-2, 0, 0.5, 2]</textarea></label>
        <div class="fields"><label>Saturation 下限<input id="sat-lower" type="number" step="any" value="0" placeholder="制限なし"></label><label>Saturation 上限<input id="sat-upper" type="number" step="any" value="1" placeholder="制限なし"></label></div>
        <button id="saturate" disabled>ブロックを実行</button><output id="vector-output" aria-label="Saturation 出力" aria-live="polite"></output>
      </section>
    </article></div>
    <footer>成熟度比較：Simulink 比 30% を目標としています。現時点の実装は基本的なブロック図と PID 実験環境です。この比較は主観的な開発評価であり、機能網羅率や客観的なベンチマークではありません。</footer>
  </main>`;
let editorMounted = false;
function selectTab(editor: boolean) {
  document.getElementById('pid-main')!.hidden = editor;
  document.getElementById('graph-main')!.hidden = !editor;
  document.getElementById('pid-tab')!.setAttribute('aria-selected', String(!editor));
  document.getElementById('editor-tab')!.setAttribute('aria-selected', String(editor));
  if (editor && !editorMounted) { mountEditor(document.getElementById('graph-main')!); editorMounted = true; }
  else if (editor) document.getElementById('graph-main')!.dispatchEvent(new Event('graph-visible'));
}
document.getElementById('pid-tab')!.addEventListener('click', () => selectTab(false));
document.getElementById('editor-tab')!.addEventListener('click', () => selectTab(true));
const form = document.querySelector<HTMLFormElement>('#model-form')!;
const status = document.querySelector<HTMLDivElement>('#status')!;
const input = (name: string) => form.elements.namedItem(name) as HTMLInputElement;
const el = <T extends HTMLElement>(id: string) => document.getElementById(id) as T;
let ready = false;
function message(text: string, error = false) { status.textContent = text; status.classList.toggle('error', error); }
function guard(action: () => void) {
  if (!ready) return;
  try { action(); } catch (error) { message(String(error), true); }
}
function readDocument(): AppDocument {
  const number = (name: string) => parseNumber(input(name).value, name);
  const raw = {
    mode: input('mode').value,
    pid: {
      kp: number('kp'), ki: number('ki'), kd: number('kd'), derivative_tau: number('derivative_tau'),
      limits: { lower: parseLimit(input('lower').value, 'PID 下限'), upper: parseLimit(input('upper').value, 'PID 上限') },
      anti_windup: input('anti_windup').checked,
    },
    dt: number('dt'), duration: number('duration'), step_time: number('step_time'),
    initial_reference: number('initial_reference'), final_reference: number('final_reference'),
    plant_gain: number('plant_gain'), plant_tau: number('plant_tau'), initial_output: number('initial_output'),
    settling_tolerance: number('settling_tolerance'),
  };
  const model = JSON.parse(validate_model(JSON.stringify(raw))) as ModelConfig;
  const saturation = readSaturation();
  return { version: 1, model, saturation };
}
function readSaturation(): AppDocument['saturation'] {
  const input = parseVector(el<HTMLTextAreaElement>('vector').value);
  const limits = {
    lower: parseLimit(el<HTMLInputElement>('sat-lower').value, 'Saturation 下限'),
    upper: parseLimit(el<HTMLInputElement>('sat-upper').value, 'Saturation 上限'),
  };
  saturate_vector(JSON.stringify(input), JSON.stringify(limits)); // Validate with the same Rust block.
  return { input, limits };
}
function applyDocument(doc: AppDocument) {
  const model = doc.model;
  for (const [name, value] of Object.entries(model)) if (name !== 'pid') input(name).value = String(value);
  for (const name of ['kp', 'ki', 'kd', 'derivative_tau'] as const) input(name).value = String(model.pid[name]);
  input('lower').value = model.pid.limits.lower === null ? '' : String(model.pid.limits.lower);
  input('upper').value = model.pid.limits.upper === null ? '' : String(model.pid.limits.upper);
  input('anti_windup').checked = model.pid.anti_windup;
  el<HTMLTextAreaElement>('vector').value = JSON.stringify(doc.saturation.input);
  el<HTMLInputElement>('sat-lower').value = doc.saturation.limits.lower === null ? '' : String(doc.saturation.limits.lower);
  el<HTMLInputElement>('sat-upper').value = doc.saturation.limits.upper === null ? '' : String(doc.saturation.limits.upper);
}
function runSaturation() {
  const sat = readSaturation();
  el<HTMLOutputElement>('vector-output').textContent = saturate_vector(JSON.stringify(sat.input), JSON.stringify(sat.limits));
}
function run() {
  const doc = readDocument();
  const json = JSON.stringify(doc.model);
  // Exercise selected mode separately; comparison always uses the identical model.
  const selected = JSON.parse(run_model(json)) as SimulationResult;
  const comparison = JSON.parse(compare_model(json)) as Comparison;
  const fmt = (v: number | null) => v === null ? '—' : v.toFixed(3);
  const tbody = el<HTMLTableSectionElement>('metrics'); tbody.replaceChildren();
  for (const [key, label] of [['enabled', '有効'], ['disabled', '無効']] as const) {
    const tr = document.createElement('tr'); tr.dataset.aw = key;
    const active = (key === 'enabled') === doc.model.pid.anti_windup;
    tr.classList.toggle('active', active);
    const m = active ? selected.metrics : comparison[key].metrics;
    for (const [i, value] of [label + (active ? ' · 選択中' : ''), fmt(m.recovery_iae), fmt(m.settling_seconds), fmt(m.upper_release_seconds), fmt(m.peak_integral)].entries()) {
      const cell = document.createElement('td'); cell.textContent = value;
      if (i === 1) cell.dataset.metric = 'iae';
      tr.append(cell);
    }
    tbody.append(tr);
  }
  for (const field of ['output', 'control', 'integral'] as const) drawChart(el(`${field}-chart`), comparison, field, doc.model.pid.anti_windup);
  document.querySelector('.flow small')!.textContent = `${doc.model.initial_reference} → ${doc.model.final_reference}`;
  el('active-mode').textContent = doc.model.mode === 'continuous' ? '連続 PID' : '離散 PID';
  runSaturation();
  message(`完了 · ${selected.samples.length} 点 · Rust / WASM`);
}
form.addEventListener('submit', event => { event.preventDefault(); guard(run); });
el('saturate').addEventListener('click', () => guard(() => { runSaturation(); message('Saturation ブロックを実行しました'); }));
el('sample').addEventListener('click', () => guard(() => { applyDocument(decodeDocument(JSON.stringify(recoverySample), validate_model)); run(); }));
el('save').addEventListener('click', () => guard(() => {
  localStorage.setItem(STORAGE_KEY, encodeDocument(readDocument())); message('設定をブラウザに保存しました');
}));
el('restore').addEventListener('click', () => guard(() => {
  const saved = localStorage.getItem(STORAGE_KEY);
  if (saved === null) throw new Error('保存済みの設定がありません');
  applyDocument(decodeDocument(saved, validate_model)); run(); message('保存設定を復元しました');
}));
el('export').addEventListener('click', () => guard(() => {
  const url = URL.createObjectURL(new Blob([encodeDocument(readDocument())], { type: 'application/json' }));
  const link = document.createElement('a'); link.href = url; link.download = 'blocksim-model.json'; link.click();
  URL.revokeObjectURL(url); message('設定を JSON に書き出しました');
}));
el<HTMLInputElement>('import').addEventListener('change', async event => {
  const target = event.target as HTMLInputElement, file = target.files?.[0];
  if (!file || !ready) return;
  try {
    if (file.size > 1_000_000) throw new Error('設定ファイルは1 MB以下にしてください');
    const doc = decodeDocument(await file.text(), validate_model);
    applyDocument(doc); run(); message('JSON 設定を復元しました');
  } catch (error) { message(String(error), true); }
  finally { target.value = ''; }
});
async function start() {
  try {
    await init(); ready = true;
    applyDocument(decodeDocument(default_model(), validate_model));
    document.querySelectorAll<HTMLButtonElement | HTMLInputElement>('button[disabled], input[disabled]').forEach(control => control.disabled = false);
    run();
    const saved = localStorage.getItem(STORAGE_KEY);
    if (saved !== null) { applyDocument(decodeDocument(saved, validate_model)); run(); message('保存設定を自動復元しました'); }
  } catch (error) { message(`初期化／保存設定の読み込みに失敗: ${String(error)}`, true); }
}
void start();
