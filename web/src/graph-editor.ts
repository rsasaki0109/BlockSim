import { normalize_graph, diagnose_graph, validate_graph, validate_frequency, validate_discrete_frequency, validate_pole_zero, validate_state_space, validate_discrete_state_space, discretize_tustin, discretize_zoh } from './wasm/blocksim';
import {mountConversionComparison} from './discretize-comparison-ui';
import {tustinRequest,tustinModel,type TustinResult,type ZohResult} from './discretize';
import { parseNumber } from './document';
import {decodeSampleTableCsv,sampleTableCsv} from './sampled-input-csv';
import { copySelection, pasteSelection, connectPorts, mergeGraph, ungroupBlock, inputWidth, nodeHeight, portY, wirePath } from './graph-edit';
import {
  GraphHistory, graphContext, newBlock, groupBlocks, deleteBlocks, inputCount, outputCount, outputWidth, setClock, setSolver,
  KIND_NAMES, GRAPH_STORAGE_KEY, decodeGraphDocument, encodeGraphDocument, resultCsv,
  type GraphModel, type GraphBlock, type GraphResult, type Kind,
} from './graph-model';
import { plotGraph, seriesFrom } from './graph-chart';
import { autoLayout } from './graph-layout';
import { searchGraph, type GraphSearchHit } from './graph-search';
import { diagnosticLinks, ISSUE_HINTS, type GraphDiagnostic, type GraphValidation } from './graph-diagnostics';
import { snapshotRun, compareResults, differenceCsv, type RunSnapshot, type ComparisonReport } from './graph-comparison';
import { checkArchiveSize, decodeRunArchive, encodeRunArchive, verifyArchivedResult } from './graph-run-archive';
import { solverStatistics } from './graph-statistics';
import { mountSweep } from './graph-sweep-ui';
import { mountFrequency } from './frequency-ui';
import { mountPoleZero } from './pole-zero-ui';
import { mountStateSpace } from './state-space-ui';
import inputSequence from '../../samples/discrete/sequence.json';
import repeatInput from '../../samples/discrete/repeat.json';
import vectorInput from '../../samples/discrete/vector-input.json';
import heldSine from '../../samples/discrete/held-sine.json';
import zohSample from '../../samples/discrete/zoh.json';
import tustinSample from '../../samples/discrete/tustin.json';
import discreteTransfer from '../../samples/discrete/transfer.json';
import discreteOpenLoop from '../../samples/discrete/open-loop.json';
import discreteFeedback from '../../samples/discrete/feedback.json';
import connectedFeedback from '../../samples/state-space/feedback.json';
import lowpass from '../../samples/frequency/lowpass.json';
import openLoop from '../../samples/frequency/open-loop.json';
import linearFeedback from '../../samples/frequency/feedback.json';
import firstOrder from '../../samples/numerics/first-order.json';
import oscillator from '../../samples/numerics/oscillator.json';
import offGridStep from '../../samples/numerics/off-grid-step.json';
import adaptiveMixed from '../../samples/numerics/mixed-pid.json';
import continuous from '../../samples/graphs/continuous-pid.json';
import mixed from '../../samples/graphs/mixed-pid.json';
import vector from '../../samples/graphs/vector.json';
import subsystem from '../../samples/graphs/subsystem.json';

const SAMPLES: Record<string,GraphModel>={continuous:continuous as GraphModel,mixed:mixed as GraphModel,vector:vector as GraphModel,subsystem:subsystem as GraphModel,firstOrder:firstOrder as GraphModel,oscillator:oscillator as GraphModel,offGridStep:offGridStep as GraphModel,adaptiveMixed:adaptiveMixed as GraphModel,lowpass:lowpass as GraphModel,linearFeedback:linearFeedback as GraphModel,openLoop:openLoop as GraphModel,connectedFeedback:connectedFeedback as GraphModel,discreteFeedback:discreteFeedback as GraphModel,discreteOpenLoop:discreteOpenLoop as GraphModel,discreteTransfer:discreteTransfer as GraphModel,tustin:tustinSample as GraphModel,zoh:zohSample as GraphModel,heldSine:heldSine as GraphModel,inputSequence:inputSequence as GraphModel,repeatInput:repeatInput as GraphModel,vectorInput:vectorInput as GraphModel};
function download(text:string,name:string,type:string){const url=URL.createObjectURL(new Blob([text],{type}));const a=document.createElement('a');a.href=url;a.download=name;a.click();URL.revokeObjectURL(url);}
function button(text:string,action:()=>void){const b=document.createElement('button');b.type='button';b.textContent=text;b.addEventListener('click',action);return b;}
export function mountEditor(host:HTMLElement) {
  host.innerHTML=`
    <section class="intro"><p class="eyebrow">MODEL WORKSPACE / ブロック図</p><h1>つなげて、動きを確かめる。</h1><p>連続・離散・ベクトルをひとつのモデルに。Scope で結果を記録します。</p></section>
    <div class="graph-toolbar panel">
      <label>ブロック種類<select id="block-kind"></select></label><button id="block-add">ブロックを追加</button>
      <button id="graph-select-all">すべて選択</button><button id="graph-copy" disabled>コピー</button><button id="graph-paste" disabled>貼り付け</button><button id="graph-duplicate" disabled>複製</button><button id="graph-undo">Undo</button><button id="graph-redo">Redo</button><button id="graph-delete">選択を削除</button><button id="graph-group">サブシステム化</button><button id="graph-ungroup">グループ解除</button><button id="graph-wire-cancel" disabled>配線をキャンセル</button><button id="graph-disconnect" disabled>選択した接続を削除</button>
      <button id="graph-check-run">モデルを確認</button><button id="graph-run" class="primary">モデルを実行</button><button id="graph-cancel" disabled>実行を中止</button>
    </div>
    <div id="graph-status" role="status" aria-label="モデル実行状態" aria-live="polite"></div>
    <details id="graph-diagnostics" class="panel">
      <summary id="graph-diagnostics-summary">モデル診断</summary><p id="graph-diagnostics-state" role="status" aria-live="polite"></p>
      <ul id="graph-diagnostics-list" aria-label="モデルの問題"></ul>
      <div class="diagnostic-pages"><button id="graph-diagnostics-prev">前の問題</button><span id="graph-diagnostics-page"></span><button id="graph-diagnostics-next">次の問題</button></div>
    </details>
    <details id="graph-search" class="panel">
      <summary>ブロックを検索 <span class="hint">Ctrl/Cmd+K</span></summary>
      <form id="graph-search-form"><label>ブロック検索<input id="graph-query" type="search" maxlength="200" placeholder="名前・ID・種類（空白で複数指定）" autocomplete="off"></label>
        <label>検索範囲<select id="graph-search-scope"><option value="all">モデル全体（内部を含む）</option><option value="current">現在の階層</option></select></label>
        <label>種類で絞り込む<select id="graph-search-kind"><option value="">すべての種類</option></select></label><button id="graph-search-clear" type="button">検索をクリア</button>
      </form><p id="graph-search-count" role="status" aria-live="polite"></p><ul id="graph-search-results" aria-label="ブロック検索結果"></ul>
      <p class="hint">検索結果を選ぶとブロックへ移動し、設定を開きます。Enter で最初の結果へ。検索と移動はモデルを変更しません。</p>
    </details>
    <div class="graph-workspace"><section class="graph-center">
      <div class="graph-topline"><div id="graph-breadcrumb"></div><span id="graph-count" class="pill"></span><span id="graph-dirty" role="status"></span></div>
      <p class="hint">ブロックはドラッグで移動。チェックで複数選択。出力ポート→入力ポートをクリックして配線。Escape で中止。線を選ぶと削除できます。右のフォームも使えます。</p>
      <div class="graph-view-tools"><button id="graph-layout">自動配置</button><button id="graph-zoom-out" aria-label="縮小">−</button><output id="graph-zoom" aria-label="表示倍率">100%</output><button id="graph-zoom-in" aria-label="拡大">＋</button><button id="graph-zoom-reset">100% 表示</button><button id="graph-fit">全体を表示</button></div><div class="graph-viewport"><div id="graph-surface"><div id="graph-board" aria-label="ブロック図キャンバス"><svg id="graph-wires" width="1800" height="1000" aria-label="接続線"></svg><div id="graph-nodes"></div></div></div></div>
      <section class="panel graph-results"><h2>Scope の結果</h2><p id="graph-statistics" class="hint"></p>
        <div class="graph-reference-tools"><button id="graph-reference-set" disabled>基準として保持</button><button id="graph-reference-clear" disabled>基準を解除</button><button id="graph-reference-export" disabled>基準のモデル JSON</button><label class="check"><input id="graph-reference-show" type="checkbox" checked disabled>基準を重ねる</label></div>
        <p id="graph-reference-state" class="hint" role="status"></p>
        <details id="graph-run-files"><summary>実行結果ファイル</summary><div class="buttons"><button id="graph-run-export" disabled>現行の結果を保存</button><button id="graph-reference-save" disabled>基準の結果を保存</button><label class="file-button">基準の結果を読み込む<input id="graph-reference-import" type="file" accept=".json,application/json"></label><button id="graph-reference-cancel" disabled>読み込みを中止</button></div><p id="graph-reference-import-state" role="status" aria-live="polite">保存した設定と結果を、再計算で確認して基準にします。</p><p class="hint">実行時のモデルと全 Scope の結果を JSON に保存。読み込みは64 MiBまで、モデルは1 MBまでです。現在のモデルは保持します。</p></details>
        <div id="graph-series" class="legend"></div><p class="hint">表示は最大16系列。実線が現行、破線が基準。CSV は現行の全 Scope・全記録点を出力します。</p><div id="graph-plot"></div>
        <label>記録点カーソル<input id="graph-cursor" type="range" min="0" max="0" value="0" disabled></label><output id="graph-values" aria-live="polite"></output><output id="graph-reference-values" hidden></output><button id="graph-csv" disabled>CSV を書き出す</button>
        <details id="graph-comparison" hidden><summary id="graph-comparison-summary">比較結果</summary><p class="hint">Δ = 現行 − 基準。同じ Scope ID・幅・記録時刻で比較。RMSE は記録点の平均、∫|Δ|dt は台形則です。比較は表示の選択によらず全対応成分を含みます。</p>
          <ul id="graph-comparison-notices"></ul><div class="table-wrap"><table aria-label="基準との差分"><thead><tr><th>Scope / 成分</th><th>最大 |Δ|</th><th>RMSE</th><th>終点 Δ</th><th>∫|Δ|dt</th></tr></thead><tbody id="graph-comparison-rows"></tbody></table></div>
          <div class="comparison-pages"><button id="graph-comparison-prev">前の系列</button><span id="graph-comparison-page"></span><button id="graph-comparison-next">次の系列</button><button id="graph-difference-csv" disabled>差分 CSV</button></div>
        </details>
      </section>
      <div id="graph-sweep-host"></div><div id="graph-frequency-host"></div><div id="graph-pole-zero-host"></div><div id="graph-state-space-host"></div>
    </section><aside>
      <section class="panel"><h2>ブロック設定</h2><div id="graph-inspector"><p class="hint">ブロックを選択してください</p></div></section>
      <section class="panel"><h2>接続</h2><form id="graph-connect"><label>接続元<select id="edge-from"></select></label><label>出力ポート<select id="edge-from-port"></select></label><label>接続先<select id="edge-to"></select></label><label>入力ポート<select id="edge-to-port"></select></label><button>接続する</button></form><div id="graph-edges"></div></section>
      <section class="panel"><h2>シミュレーション設定</h2><form id="graph-clock"><label>計算方式<select id="graph-solver"><option value="rk4">固定刻み RK4</option><option value="rk45">可変刻み RK45</option></select></label><label>グラフ刻み幅 (s)<input id="graph-dt" type="number" step="any" required></label><label>グラフ終了時刻 (s)<input id="graph-duration" type="number" step="any" required></label><fieldset id="graph-rk45-settings" hidden disabled><legend>RK45 の精度と内部刻み</legend><label>絶対許容誤差<input id="graph-atol" type="number" step="any" required></label><label>相対許容誤差<input id="graph-rtol" type="number" step="any" required></label><label>最小内部刻み (s)<input id="graph-min-step" type="number" step="any" required></label><label>最大内部刻み (s)<input id="graph-max-step" type="number" step="any" required></label></fieldset><button>時間設定を適用</button></form><p class="hint">RK45 ではグラフ刻み幅は記録時計です。離散の周期はこの整数倍。計算方式はモデル全体で共有し、Step と離散更新で積分区間を分けます。</p></section>
      <section class="panel"><h2>モデルとサンプル</h2><label>モデルサンプル<select id="graph-sample"><option value="continuous">連続 PID の復帰</option><option value="mixed">離散 PID + 連続プラント</option><option value="vector">ベクトル + Saturation</option><option value="subsystem">Sine + Subsystem</option><option value="firstOrder">RK45: 一次遅れ解析解</option><option value="oscillator">RK45: 結合振動子</option><option value="offGridStep">RK45: 時計の間の Step</option><option value="adaptiveMixed">RK45: 連続／離散 PID</option><option value="lowpass">伝達関数：2次ローパス</option><option value="linearFeedback">線形 PI 閉ループ</option><option value="openLoop">開ループのゲイン／位相余裕</option><option value="connectedFeedback">接続済み負帰還の状態空間</option><option value="discreteFeedback">離散負帰還・単位円の極</option><option value="vectorInput">ベクトル入力表CSV</option><option value="repeatInput">繰り返す保持入力系列</option><option value="inputSequence">任意の保持入力系列</option><option value="heldSine">保持正弦波の離散化比較</option><option value="zoh">保持入力：ZOH離散化</option><option value="tustin">連続 → 離散：Tustin変換</option><option value="discreteTransfer">離散伝達関数：直接項と1サンプル遅れ</option><option value="discreteOpenLoop">離散開ループ・ゲイン／位相余裕</option></select></label>
        <label>読み込み方法<select id="graph-import-mode"><option value="replace">モデルを置き換える</option><option value="merge">既存モデルに追加</option></select></label><div class="buttons"><button id="graph-load-sample">サンプルを読み込む</button><button id="graph-empty">新しいモデル</button><button id="graph-save">モデルを保存</button><button id="graph-restore">モデルを復元</button><button id="graph-export">モデル JSON</button><label class="file-button">モデルを読み込む<input id="graph-import" type="file" accept=".json,application/json"></label></div><p class="hint">未接続の作成途中モデルも保存できます。コピーはこのタブ内の専用クリップボードを使い、外部の接続と境界 Input / Output は含めません。追加読み込みでは既存の時計・終了時刻を保持し、同じ刻み幅のモデルを取り込めます。既存の PID Lab 設定は別に保持します。</p>
      </section>
    </aside></div><footer>成熟度比較：目標は Simulink 比30%。評価は主観的な開発評価で、機能網羅率や客観的なベンチマークではありません。</footer>`;
  const get=<T extends Element = HTMLElement>(id:string)=>host.querySelector<T>(`#${id}`)!;
  const status=get('graph-status');
  const say=(text:string,error=false)=>{status.textContent=text;status.classList.toggle('error',error);};
  const guard=(action:()=>void)=>{try{action();}catch(e){say(String(e),true);}};
  const history=new GraphHistory(structuredClone(SAMPLES.continuous),normalize_graph);
  let path:string[]=[], selected=new Set<string>(), result:GraphResult|null=null, visible=new Set<string>();
  let pendingWire:{from:string,from_port:number,path:string}|null=null, previewPoint:[number,number]|null=null, selectedEdge:number|null=null;
  let worker:Worker|null=null, request=0;
  let clipboard:GraphModel|null=null, pasteCount=0, zoom=1;
  let resultModel:GraphModel|null=null,referenceRun:RunSnapshot|null=null,comparison:ComparisonReport|null=null,comparisonPage=0;
  let archiveWorker:Worker|null=null,archiveRequest=0,archiveBusy=false;
  let sweep:ReturnType<typeof mountSweep>|undefined;
  let frequency:ReturnType<typeof mountFrequency>|undefined;
  let poleZero:ReturnType<typeof mountPoleZero>|undefined;
  let stateSpace:ReturnType<typeof mountStateSpace>|undefined;
  let conversionComparison:ReturnType<typeof mountConversionComparison>|undefined;
  let sampledInputIoEpoch=0,sampledInputIoBusy=false;
  const modelBusy=()=>!!worker||archiveBusy||!!sweep?.running()||!!frequency?.running()||!!poleZero?.running()||!!stateSpace?.running()||!!conversionComparison?.running();
  let diagnosticSnapshot:string|null=null,diagnosticPage=0,runtimeError:string|null=null;
  let validation:GraphValidation={runnable:false,diagnostics:[]};
  const problemBlocks=new Map<string,number>(),problemPorts=new Set<string>();
  function refreshDiagnostics(force=false){
    const json=JSON.stringify(history.graph);if(!force&&json===diagnosticSnapshot)return;
    try{validation=JSON.parse(diagnose_graph(json)) as GraphValidation;}
    catch(error){validation={runnable:false,diagnostics:[{code:'structure',message:String(error),block_id:null,port:null,source_id:null}]};}
    diagnosticSnapshot=json;diagnosticPage=0;runtimeError=null;problemBlocks.clear();problemPorts.clear();
    for(const issue of validation.diagnostics){
      if(issue.block_id){
        const parts=issue.block_id.split('/');for(let i=1;i<=parts.length;i++){const address=parts.slice(0,i).join('/');problemBlocks.set(address,(problemBlocks.get(address)||0)+1);}
        if(issue.port!==null)problemPorts.add(`${issue.block_id}:${issue.port}`);
      }
      const parent=diagnosticLinks(history.graph,issue).parentInput;if(parent)problemPorts.add(`${parent.target.address}:${parent.port}`);
    }
  }
  function renderDiagnostics(){
    const issues:GraphDiagnostic[]=[...validation.diagnostics];
    if(runtimeError)issues.push({code:'runtime_error',message:runtimeError,block_id:null,port:null,source_id:null});
    const panel=get<HTMLDetailsElement>('graph-diagnostics');panel.classList.toggle('has-issues',issues.length>0);
    get('graph-diagnostics-summary').textContent=`モデル診断 · ${issues.length?issues.length+' 件の問題':'問題なし'}`;
    get('graph-diagnostics-state').textContent=runtimeError?'計算中にエラーが発生しました':validation.runnable?'実行前チェック: 問題なし':`実行前チェック: ${issues.length} 件の問題`;
    const pages=Math.max(1,Math.ceil(issues.length/50));diagnosticPage=Math.max(0,Math.min(diagnosticPage,pages-1));
    get<HTMLButtonElement>('graph-diagnostics-prev').disabled=diagnosticPage===0;get<HTMLButtonElement>('graph-diagnostics-next').disabled=diagnosticPage===pages-1;
    get('graph-diagnostics-page').textContent=issues.length?`${diagnosticPage+1} / ${pages} ページ（全${issues.length}件）`:'';
    const list=get('graph-diagnostics-list');list.replaceChildren();
    for(const issue of issues.slice(diagnosticPage*50,(diagnosticPage+1)*50)){
      const row=document.createElement('li'),title=document.createElement('strong'),message=document.createElement('p'),hint=document.createElement('p'),actions=document.createElement('div');
      title.textContent=ISSUE_HINTS[issue.code][0];message.textContent=issue.message;hint.className='hint';hint.textContent=ISSUE_HINTS[issue.code][1];row.dataset.code=issue.code;
      const links=diagnosticLinks(history.graph,issue);
      if(links.target)actions.append(button(`表示 ${links.target.address}${issue.port!==null?':'+issue.port:''}`,()=>guard(()=>revealBlock(links.target!,issue.port))));
      if(links.parentInput){const parent=links.parentInput;actions.append(button(`親の入力 ${parent.target.address}:${parent.port}`,()=>guard(()=>revealBlock(parent.target,parent.port))));hint.textContent='親階層の対応する入力と、内部の境界接続を確認してください。';}
      if(links.source&&links.source.address!==links.target?.address)actions.append(button(`接続元 ${links.source.address}`,()=>guard(()=>revealBlock(links.source!))));
      if(['empty_graph','missing_scope'].includes(issue.code))actions.append(button('Scope を選ぶ',()=>{get<HTMLSelectElement>('block-kind').value='scope';get('block-add').focus();}));
      if(['work_limit','recording_limit'].includes(issue.code))actions.append(button('時間設定を確認',()=>get('graph-dt').focus()));
      row.append(title,message,hint,actions);list.append(row);
    }
  }
  function renderComparison(){
    const conversionBusy=modelBusy();
    if(conversionBusy!==sampledInputIoBusy){sampledInputIoBusy=conversionBusy;sampledInputIoEpoch++;}
    for(const id of ['graph-input-csv-import','graph-input-csv-export']){const control=host.querySelector<HTMLInputElement|HTMLButtonElement>('#'+id);if(control)control.disabled=conversionBusy;}
    const conversionFields=host.querySelector<HTMLFieldSetElement>('#graph-discretize-fields');if(conversionFields)conversionFields.disabled=conversionBusy;
    sweep?.sync();frequency?.sync();poleZero?.sync();stateSpace?.sync();conversionComparison?.sync();
    get<HTMLButtonElement>('graph-run').disabled=!!worker||archiveBusy||(!!sweep?.running()||(!!frequency?.running()||(!!poleZero?.running()||(!!stateSpace?.running()||!!conversionComparison?.running()))));
    get<HTMLButtonElement>('graph-reference-set').disabled=!result||!resultModel||!!worker||archiveBusy||(!!sweep?.running()||(!!frequency?.running()||(!!poleZero?.running()||(!!stateSpace?.running()||!!conversionComparison?.running()))));
    get('graph-reference-set').textContent=referenceRun?'基準を置き換える':'基準として保持';
    get<HTMLButtonElement>('graph-reference-clear').disabled=!referenceRun||archiveBusy;get<HTMLButtonElement>('graph-reference-export').disabled=!referenceRun;
    get<HTMLButtonElement>('graph-run-export').disabled=!result||!resultModel||!!worker||archiveBusy||(!!sweep?.running()||(!!frequency?.running()||(!!poleZero?.running()||(!!stateSpace?.running()||!!conversionComparison?.running()))));
    get<HTMLButtonElement>('graph-reference-save').disabled=!referenceRun||!!worker||archiveBusy||(!!sweep?.running()||(!!frequency?.running()||(!!poleZero?.running()||(!!stateSpace?.running()||!!conversionComparison?.running()))));
    get<HTMLInputElement>('graph-reference-import').disabled=!!worker||archiveBusy||(!!sweep?.running()||(!!frequency?.running()||(!!poleZero?.running()||(!!stateSpace?.running()||!!conversionComparison?.running()))));
    get<HTMLButtonElement>('graph-reference-cancel').disabled=!archiveBusy;
    get<HTMLInputElement>('graph-reference-show').disabled=!referenceRun;
    get('graph-reference-state').textContent=referenceRun?`基準: dt=${referenceRun.model.dt}s · 終了=${referenceRun.model.duration}s · ${referenceRun.result.traces.length} Scope · ${referenceRun.result.times.length} 点。${result?'設定と結果を保持しています。':'再実行して比較してください。'} このタブ内のみ保持し、再読み込みで解除します。`:'実行結果を基準として保持すると、設定変更後の波形と差分を比較できます。';
    get('graph-comparison').hidden=!referenceRun||!result;get('graph-reference-values').hidden=!referenceRun||!result;
    get<HTMLButtonElement>('graph-difference-csv').disabled=!comparison?.rows.length||comparison.rows.some(r=>r.status!=='ok');
    get('graph-comparison-summary').textContent=`比較結果 · ${comparison?.rows.length||0} 対応系列`;
    const notices=get('graph-comparison-notices');notices.replaceChildren();for(const notice of comparison?.notices||[]){const li=document.createElement('li');li.textContent=notice;notices.append(li);}
    if(comparison&&!comparison.rows.length){const li=document.createElement('li');li.textContent='同じ Scope ID と信号幅の系列がありません。';notices.append(li);}
    const pages=Math.max(1,Math.ceil((comparison?.rows.length||0)/50));comparisonPage=Math.max(0,Math.min(comparisonPage,pages-1));
    get<HTMLButtonElement>('graph-comparison-prev').disabled=comparisonPage===0;get<HTMLButtonElement>('graph-comparison-next').disabled=comparisonPage===pages-1;
    get('graph-comparison-page').textContent=comparison?.rows.length?`${comparisonPage+1} / ${pages} ページ（全${comparison.rows.length}系列）`:'';
    const rows=get('graph-comparison-rows');rows.replaceChildren();
    for(const row of comparison?.rows.slice(comparisonPage*50,(comparisonPage+1)*50)||[]){
      const tr=document.createElement('tr');tr.dataset.series=row.key;const label=document.createElement('td');label.textContent=`${row.label} (${row.id}) [${row.component}]`;label.title=`基準の表示名: ${row.referenceLabel}`;tr.append(label);
      if(row.metrics)for(const key of ['maxAbs','rms','final','area'] as const){const cell=document.createElement('td');cell.dataset.metric=key;cell.textContent=row.metrics[key].toPrecision(6);tr.append(cell);}
      else {const cell=document.createElement('td');cell.colSpan=4;cell.textContent=row.status==='time_grid'?'記録時刻が異なります':'差分の計算が有限値の範囲外です';tr.append(cell);}
      rows.append(tr);
    }
  }
  function plotResult(){if(result)plotGraph(get('graph-plot'),result,visible,get<HTMLInputElement>('graph-reference-show').checked?referenceRun?.result:undefined);}
  let baseline=JSON.stringify(history.graph);
  const dirty=()=>JSON.stringify(history.graph)!==baseline;
  function syncDirty(){get('graph-dirty').textContent=dirty()?'未保存の変更あり':'変更なし';}
  window.addEventListener('beforeunload',event=>{if(dirty()){event.preventDefault();event.returnValue='';}});
  function syncCopy(){for(const id of ['graph-copy','graph-duplicate'])get<HTMLButtonElement>(id).disabled=!selected.size;get<HTMLButtonElement>('graph-paste').disabled=!clipboard;}
  function applyZoom(){
    const board=get('graph-board'),surface=get('graph-surface');board.style.transform=`scale(${zoom})`;
    board.style.left=32*zoom+'px';board.style.top=32*zoom+'px';
    surface.style.width=(parseFloat(board.style.width)+64)*zoom+'px';surface.style.height=(parseFloat(board.style.height)+64)*zoom+'px';
    get('graph-zoom').textContent=`${Math.round(zoom*100)}%`;
    get<HTMLButtonElement>('graph-zoom-out').disabled=zoom<=.02;get<HTMLButtonElement>('graph-zoom-in').disabled=zoom>=2;
  }
  function setZoom(next:number){
    const viewport=host.querySelector<HTMLElement>('.graph-viewport')!,old=zoom;
    const center=[(viewport.scrollLeft+viewport.clientWidth/2)/old,(viewport.scrollTop+viewport.clientHeight/2)/old];
    zoom=Math.max(.02,Math.min(2,next));applyZoom();
    viewport.scrollLeft=center[0]*zoom-viewport.clientWidth/2;viewport.scrollTop=center[1]*zoom-viewport.clientHeight/2;
  }
  function fit(){
    const blocks=current().blocks,viewport=host.querySelector<HTMLElement>('.graph-viewport')!;
    if(!blocks.length){setZoom(1);viewport.scrollTo(0,0);return;}
    const bounds=get<SVGSVGElement>('graph-wires').getBBox();
    const left=Math.min(...blocks.map(b=>b.position[0]-30),bounds.x),top=Math.min(...blocks.map(b=>b.position[1]-20),bounds.y);
    const right=Math.max(...blocks.map(b=>b.position[0]+200),bounds.x+bounds.width),bottom=Math.max(...blocks.map(b=>b.position[1]+nodeHeight(b)+20),bounds.y+bounds.height);
    setZoom(Math.min(1,(viewport.clientWidth-50)/(right-left),(viewport.clientHeight-50)/(bottom-top)));
    viewport.scrollLeft=Math.max(0,(left+32)*zoom-25);viewport.scrollTop=Math.max(0,(top+32)*zoom-25);
  }
  const cancel=()=>{request++;worker?.terminate();worker=null;get<HTMLButtonElement>('graph-run').disabled=false;get<HTMLButtonElement>('graph-cancel').disabled=true;renderComparison();};
  const invalidate=()=>{conversionComparison?.clear();frequency?.invalidate();poleZero?.invalidate();stateSpace?.invalidate();sweep?.cancel();cancel();pendingWire=null;previewPoint=null;selectedEdge=null;result=null;resultModel=null;comparison=null;get('graph-statistics').textContent='';get('graph-plot').replaceChildren();get('graph-series').replaceChildren();get('graph-values').textContent='モデル変更後は再実行してください';get('graph-reference-values').textContent='';get<HTMLInputElement>('graph-cursor').disabled=true;get<HTMLButtonElement>('graph-csv').disabled=true;renderComparison();};
  const current=()=>graphContext(history.graph,path);
  // Transfer metadata uses the root solver while retaining the context arrays.
  const transfer=(graph:GraphModel)=>({...graph,solver:history.graph.solver});
  let searchHits:GraphSearchHit[]=[];
  function renderSearch(){
    const query=get<HTMLInputElement>('graph-query').value,kind=get<HTMLSelectElement>('graph-search-kind').value as Kind|'';
    const scope=get<HTMLSelectElement>('graph-search-scope').value;
    searchHits=query.trim()||kind?searchGraph(history.graph,query,{kind,path:scope==='current'?path:[],recursive:scope==='all'}):[];
    get('graph-search-count').textContent=query.trim()||kind?`${searchHits.length} 件`:'名前・ID・種類を入力するか、種類で絞り込んでください';
    const results=get('graph-search-results');results.replaceChildren();
    for(const hit of searchHits){
      const row=document.createElement('li'),target=button('',()=>guard(()=>revealBlock(hit)));
      const title=document.createElement('strong'),detail=document.createElement('small');title.textContent=hit.label;detail.textContent=`${KIND_NAMES[hit.kind]} · ${hit.address}`;
      target.append(title,detail);target.setAttribute('aria-label',`表示 ${hit.label} (${hit.address})`);
      target.classList.toggle('search-current',hit.path.join('/')===path.join('/')&&selected.has(hit.id));row.append(target);results.append(row);
    }
  }
  function revealBlock(hit:GraphSearchHit,port:number|null=null){
    const graph=graphContext(history.graph,hit.path),block=graph.blocks.find(b=>b.id===hit.id);
    if(!block){renderSearch();throw new Error('ブロックがありません。検索し直してください');}
    path=[...hit.path];selected=new Set([hit.id]);pendingWire=null;previewPoint=null;selectedEdge=null;render();
    const viewport=host.querySelector<HTMLElement>('.graph-viewport')!,node=get('graph-nodes').querySelector<HTMLElement>(`[data-block="${hit.id}"]`)!,height=node.offsetHeight;
    setZoom(Math.min(1,(viewport.clientWidth-48)/190,(viewport.clientHeight-48)/height));
    viewport.scrollLeft=(block.position[0]+32+83)*zoom-viewport.clientWidth/2;
    viewport.scrollTop=(block.position[1]+32+height/2)*zoom-viewport.clientHeight/2;
    viewport.scrollIntoView({block:'nearest',inline:'nearest'});
    node.focus({preventScroll:true});
    if(port!==null)get<HTMLButtonElement>(`port-in-${hit.id}-${port}`)?.focus({preventScroll:true});
  }
  const edit=(action:(graph:GraphModel)=>void,message='モデルを変更しました')=>{
    const previousSelection=new Set(selected);
    try { history.apply(root=>action(graphContext(root,path)));invalidate();render();say(message);return true; }
    catch(error){ selected=previousSelection;say(String(error),true);return false; }
  };
  function renderPorts() {
    for(const [blockId,portId,direction] of [['edge-from','edge-from-port','out'],['edge-to','edge-to-port','in']] as const){
      const b=current().blocks.find(b=>b.id===get<HTMLSelectElement>(blockId).value);const select=get<HTMLSelectElement>(portId);select.replaceChildren();
      const count=b?(direction==='out'?outputCount(b):inputCount(b)):0;
      for(let p=0;p<count;p++)select.add(new Option(String(p),String(p)));
    }
  }
  function renderInspector() {
    const previousComparison=conversionComparison;conversionComparison=undefined;previousComparison?.cancel();
    const area=get('graph-inspector');area.replaceChildren();
    const b=current().blocks.find(b=>selected.has(b.id));
    if(!b){const p=document.createElement('p');p.className='hint';p.textContent='ブロックを選択してください';area.append(p);return;}
    const info=document.createElement('p');info.className='hint';info.textContent=`${b.id} · ${KIND_NAMES[b.params.kind]} · 選択${selected.size}個`;area.append(info);
    const form=document.createElement('form');form.id='block-properties';area.append(form);
    const controls=new Map<string,HTMLInputElement|HTMLTextAreaElement|HTMLSelectElement>();
    function field(name:string,label:string,value:unknown,type?:string) {
      const l=document.createElement('label');l.textContent=label;
      let control:HTMLInputElement|HTMLTextAreaElement|HTMLSelectElement;
      if(Array.isArray(value)){control=document.createElement('textarea');control.rows=2;control.value=JSON.stringify(value);control.dataset.type='json';}
      else if(type==='mode'){control=document.createElement('select');control.add(new Option('連続 PID','continuous'));control.add(new Option('離散 PID','discrete'));control.value=String(value);}
      else {control=document.createElement('input');control.type=typeof value==='boolean'?'checkbox':type==='text'?'text':'number';
        if(control.type==='checkbox')control.checked=Boolean(value);else{control.value=value===null?'':String(value);control.step='any';}
        if(value===null)control.placeholder='制限なし';if(control.type==='checkbox')l.className='check';}
      control.name=name;controls.set(name,control);l.append(control);form.append(l);
    }
    field('label','表示名',b.label,'text');field('width','信号幅',b.width);field('x','位置 X',b.position[0]);field('y','位置 Y',b.position[1]);
    const labels:Record<string,string>={numerator:'分子係数 (降べき順 JSON)',denominator:'分母係数 (降べき順 JSON)',value:'値 (JSON 配列)',values:'入力系列（行=サンプル、列=成分 / JSON）',before:'変更前 (JSON 配列)',after:'変更後 (JSON 配列)',step_time:'変更時刻 (s)',gain:'ゲイン',tau:'時定数 (s)',initial:'初期値 (JSON 配列)',signs:'入力係数 (JSON 配列)',inputs:'入力数',indices:'抽出インデックス (JSON 配列)',sample_time:'サンプル周期 (s)',frequency:'周波数 (Hz)',phase:'位相 (rad)',amplitude:'振幅',offset:'オフセット',port:'境界ポート番号',mode:'PID 更新方式'};
    for(const [key,value]of Object.entries(b.params)) {
      if(['kind','model','config','limits','repeat'].includes(key))continue;
      field(`p.${key}`,labels[key]||key,value,key==='mode'?'mode':undefined);
    }
    if(b.params.kind==='sampled_input')field('p.repeat','入力系列を繰り返す',b.params.repeat??false);
    if(b.params.kind==='sampled_input'){
      const section=document.createElement('section');section.id='graph-input-csv';
      section.innerHTML=`<h3>入力表CSV</h3><div class="buttons"><label class="file-button">入力表CSVを読み込む<input id="graph-input-csv-import" type="file" accept=".csv,text/csv"></label><button id="graph-input-csv-export" type="button">入力表CSVを保存</button></div><p class="hint">1行=サンプル、1列=成分。列数は上の信号幅と一致させます（1〜64列、1〜2001行、1 MBまで）。見出しなし、または1列はinput、複数列はinput_0,input_1,…。時刻・周期・繰り返し設定はCSVに含みません。読み込み後に「ブロック設定を適用」で反映します。保存は現在の設定欄の入力表です。</p><p id="graph-input-csv-state" role="status" aria-live="polite"></p>`;
      form.append(section);
      const input=section.querySelector<HTMLInputElement>('#graph-input-csv-import')!,exportButton=section.querySelector<HTMLButtonElement>('#graph-input-csv-export')!,state=section.querySelector<HTMLElement>('#graph-input-csv-state')!;
      const values=controls.get('p.values')!,width=controls.get('width')!;
      let readRequest=0,revision=0;
      const message=(text:string,error=false)=>{state.textContent=text;state.classList.toggle('error',error);};
      const changed=(event:Event)=>{if(event.target!==input){revision++;message('設定欄を変更しました。CSV保存は現在の入力表を使用します。');}};
      form.addEventListener('input',changed);form.addEventListener('change',changed);
      input.disabled=exportButton.disabled=modelBusy();
      input.addEventListener('change',async()=>{
        const file=input.files?.[0];if(!file)return;const id=++readRequest,version=revision,epoch=sampledInputIoEpoch;
        const active=()=>id===readRequest&&form.isConnected;
        try{
          if(modelBusy())throw new Error('実行・解析の完了を待ってください');
          if(file.size>1_000_000)throw new Error('入力表CSVは1 MBまでです');
          const expectedWidth=parseNumber(width.value,'信号幅');message('入力表CSVを読み込み中…');
          const text=await file.text();if(!active())return;
          if(version!==revision||epoch!==sampledInputIoEpoch||modelBusy()){message('設定変更・実行のためCSV読み込みを破棄しました');return;}
          const rows=decodeSampleTableCsv(text,expectedWidth);values.value=JSON.stringify(rows);revision++;
          message(`入力表CSVを読み込みました · ${rows.length}行 × ${expectedWidth}列。ブロック設定を適用してください。`);
        }catch(error){if(active()&&version===revision&&epoch===sampledInputIoEpoch)message(String(error),true);}
        finally{if(id===readRequest)input.value='';}
      });
      exportButton.addEventListener('click',()=>{try{
        if(modelBusy())throw new Error('実行・解析の完了を待ってください');
        download(sampleTableCsv(JSON.parse(values.value),parseNumber(width.value,'信号幅')),'blocksim-input-table.csv','text/csv');message('設定欄の入力表CSVを保存しました');
      }catch(error){message(String(error),true);}});
    }
    if(b.params.config){const cfg=b.params.config as Record<string,unknown>;for(const [key,value]of Object.entries(cfg))if(key!=='limits')field(`c.${key}`,key==='anti_windup'?'anti-windup':key,value);
      const limits=cfg.limits as Record<string,unknown>;field('c.lower','PID 出力下限',limits.lower);field('c.upper','PID 出力上限',limits.upper);}
    if(b.params.limits){const limits=b.params.limits as Record<string,unknown>;field('p.lower','出力下限',limits.lower);field('p.upper','出力上限',limits.upper);}
    if(b.params.kind==='discrete_transfer_function'){const hint=document.createElement('p');hint.className='hint';hint.textContent='係数は z の降べき順、分子次数 ≤ 分母次数 ≤ 8。短い分子は高次側を0で補います（[0.2] / [1,-0.8] は1サンプル遅れ）。初期値は成分ごとのcompanion状態。周期は記録刻みの整数倍。サンプル時刻で出力を計算し、サンプル間は保持します。';form.append(hint);}
    if(b.params.kind==='transfer_function'){const hint=document.createElement('p');hint.className='hint';hint.textContent='係数は s の降べき順、分子次数 ≤ 分母次数 ≤ 8。初期値は各信号成分につき分母次数個を、成分ごとに並べます。分母の先頭係数は0以外。';form.append(hint);}
    const submit=document.createElement('button');submit.textContent='ブロック設定を適用';form.append(submit);
    form.addEventListener('submit',event=>{event.preventDefault();edit(graph=>{
      const target=graph.blocks.find(n=>n.id===b.id)!;
      const read=(name:string)=>{
        const c=controls.get(name)!;
        if(c instanceof HTMLInputElement&&c.type==='checkbox')return c.checked;
        if(c.dataset.type==='json')return JSON.parse(c.value) as unknown;
        if(c instanceof HTMLSelectElement||name==='label')return c.value;
        if((name.endsWith('.lower')||name.endsWith('.upper'))&&!c.value.trim())return null;
        return parseNumber(c.value,name);
      };
      target.label=String(read('label'));target.width=Number(read('width'));target.position=[Number(read('x')),Number(read('y'))];
      for(const [key]of Object.entries(target.params))if(!['kind','model','config','limits','repeat'].includes(key))target.params[key]=read(`p.${key}`);
      if(target.params.kind==='sampled_input')target.params.repeat=read('p.repeat');
      if(target.params.config){const cfg=target.params.config as Record<string,unknown>;for(const key of Object.keys(cfg))if(key!=='limits')cfg[key]=read(`c.${key}`);cfg.limits={lower:read('c.lower'),upper:read('c.upper')};}
      if(target.params.limits)target.params.limits={lower:read('p.lower'),upper:read('p.upper')};
    },'ブロック設定を適用しました');});
    if(b.params.kind==='transfer_function'){
      const section=document.createElement('section');section.id='graph-discretize';section.innerHTML=`<h3>連続 → 離散</h3><p class="hint">適用済みの係数を変換します。上の変更は先に適用してください。初期状態0のみ。Tustinは周波数を写像し、プリワープで指定周波数を一致させます。ZOHは一定の保持入力に対するサンプル時刻の応答を合わせます。接続・記録時計・解析領域は保持します。</p><form id="graph-discretize-form"><fieldset id="graph-discretize-fields"><label>離散化方式<select id="graph-discretize-method"><option value="tustin">Tustin法</option><option value="zoh">ゼロ次ホールド（ZOH）</option></select></label><label>変換サンプル周期 (s)<input id="graph-discretize-period" type="number" step="any" value="${history.graph.dt}" required></label><label>プリワープ角周波数 (rad/s)<input id="graph-discretize-prewarp" type="number" step="any" placeholder="なし"></label><button id="graph-discretize-preview" type="submit">変換係数を確認</button><button id="graph-discretize-apply" type="button" disabled>離散ブロックへ変換</button></fieldset></form><pre id="graph-discretize-coefficients" class="hint"></pre><p id="graph-discretize-state" role="status"></p>`;area.append(section);
      let preview:{result:TustinResult|ZohResult;root:string;conditions:string}|null=null;
      const period=section.querySelector<HTMLInputElement>('#graph-discretize-period')!,prewarp=section.querySelector<HTMLInputElement>('#graph-discretize-prewarp')!,apply=section.querySelector<HTMLButtonElement>('#graph-discretize-apply')!,state=section.querySelector<HTMLElement>('#graph-discretize-state')!,output=section.querySelector<HTMLElement>('#graph-discretize-coefficients')!;
      const busy=()=>!!worker||archiveBusy||!!sweep?.running()||!!frequency?.running()||!!poleZero?.running()||(!!stateSpace?.running()||!!conversionComparison?.running());
      const mode=section.querySelector<HTMLSelectElement>('#graph-discretize-method')!;
      const conditions=()=>JSON.stringify([mode.value,period.value,prewarp.value]);
      const address=[...path,b.id].join('/');
      const fail=(e:unknown)=>{state.textContent=String(e);state.classList.add('error');};
      section.querySelector<HTMLFormElement>('#graph-discretize-form')!.addEventListener('submit',event=>{event.preventDefault();try{
        if(busy())throw new Error('実行・解析の完了を待ってください');preview=null;apply.disabled=true;output.textContent='';
        const q=tustinRequest(b,parseNumber(period.value,'変換周期'),mode.value==='tustin'&&prewarp.value.trim()?parseNumber(prewarp.value,'プリワープ'):undefined);
        const data=JSON.parse((mode.value==='zoh'?discretize_zoh:discretize_tustin)(JSON.stringify(q))) as TustinResult|ZohResult;tustinModel(history.graph,address,data,normalize_graph,diagnose_graph);
        preview={result:data,root:JSON.stringify(history.graph),conditions:conditions()};output.textContent=`z分子: ${JSON.stringify(data.numerator)}\nz分母: ${JSON.stringify(data.denominator)}\nTs=${data.request.sample_time}s · ${'mapping_scale' in data?`Tustin k=${data.mapping_scale}`:`ZOH · 再構成残差=${data.reconstruction_residual}`}`;state.textContent='変換係数を確認しました。適用すると1回のUndoで元へ戻せます。';state.classList.remove('error');apply.disabled=false;
      }catch(e){fail(e);}});
      for(const input of [period,prewarp])input.addEventListener('input',()=>{preview=null;apply.disabled=true;output.textContent='';state.textContent='条件を変更しました。係数を再確認してください。';state.classList.remove('error');});
      mode.addEventListener('change',()=>{prewarp.disabled=mode.value==='zoh';preview=null;apply.disabled=true;output.textContent='';state.textContent='方式を変更しました。係数を再確認してください。';state.classList.remove('error');});
      apply.addEventListener('click',()=>{try{if(busy())throw new Error('実行・解析の完了を待ってください');if(!preview||preview.root!==JSON.stringify(history.graph)||preview.conditions!==conditions())throw new Error('モデルまたは条件が変わりました。再確認してください。');const data=tustinModel(history.graph,address,preview.result,normalize_graph,diagnose_graph);edit(g=>{g.blocks.find(n=>n.id===b.id)!.params=structuredClone(graphContext(data,path).blocks.find(n=>n.id===b.id)!.params);},`${mode.value==='zoh'?'ZOH':'Tustin法'}で離散伝達関数へ変換しました`);}catch(e){fail(e);}});
      const comparisonHost=document.createElement('section');comparisonHost.id='graph-conversion-comparison';section.append(comparisonHost);
      conversionComparison=mountConversionComparison(comparisonHost,{
        plan:()=>({format:'blocksim.conversion-comparison',version:1,method:mode.value==='zoh'?'zoh':'tustin',samples:101,request:tustinRequest(b,parseNumber(period.value,'変換周期'),mode.value==='tustin'&&prewarp.value.trim()?parseNumber(prewarp.value,'プリワープ'):undefined)}),
        restore:p=>{const q=tustinRequest(b,p.request.sample_time,p.request.prewarp_omega);if(JSON.stringify([q.numerator,q.denominator])!==JSON.stringify([p.request.numerator,p.request.denominator]))throw new Error('選択した伝達関数と比較設定の係数が違います');(p.method==='zoh'?discretize_zoh:discretize_tustin)(JSON.stringify(q));mode.value=p.method;period.value=String(q.sample_time);prewarp.value=q.prewarp_omega===undefined?'':String(q.prewarp_omega);prewarp.disabled=p.method==='zoh';preview=null;apply.disabled=true;output.textContent='';state.textContent='比較設定を読み込みました。係数を再確認してください。';state.classList.remove('error');},
        busy:()=>!!worker||archiveBusy||!!sweep?.running()||!!frequency?.running()||!!poleZero?.running()||!!stateSpace?.running(),onBusy:renderComparison,download,
      });
      section.querySelector<HTMLFieldSetElement>('#graph-discretize-fields')!.disabled=busy();
    }
    if(b.params.kind==='subsystem')area.append(button('内部を編集',()=>{path.push(b.id);selected.clear();render();say('サブシステムの内部を表示しています');}));
  }
  function paintPreview() {
    get('graph-wire-preview')?.remove();
    if(!pendingWire)return;
    const source=current().blocks.find(b=>b.id===pendingWire!.from);if(!source)return;
    const x=source.position[0]+166,y=source.position[1]+portY(pendingWire.from_port);
    const [endX,endY]=previewPoint||[x+65,y];
    const line=document.createElementNS('http://www.w3.org/2000/svg','path');line.id='graph-wire-preview';line.setAttribute('class','wire-preview');
    line.setAttribute('d',`M${x},${y} C${x+70},${y} ${endX-70},${endY} ${endX},${endY}`);get('graph-wires').append(line);
  }
  function chooseEdge(index:number) {
    pendingWire=null;previewPoint=null;selectedEdge=index;
    renderNodes();get<HTMLButtonElement>('graph-wire-cancel').disabled=true;get<HTMLButtonElement>('graph-disconnect').disabled=false;
    say('接続を選択しました。削除ボタンまたは Delete キーで削除できます');
  }
  function syncSelection() {
    for(const card of get('graph-nodes').children){const id=(card as HTMLElement).dataset.block!;card.classList.toggle('selected',selected.has(id));const check=card.querySelector<HTMLInputElement>('input[type=checkbox]');if(check)check.checked=selected.has(id);}
    for(const line of get('graph-wires').querySelectorAll<SVGPathElement>('.wire'))line.classList.toggle('wire-selected',selectedEdge===Number(line.dataset.edge));
    get<HTMLButtonElement>('graph-disconnect').disabled=selectedEdge===null;syncCopy();renderSearch();
  }
  function renderNodes() {
    const nodes=get('graph-nodes');nodes.replaceChildren();const wires=get<SVGSVGElement>('graph-wires');wires.replaceChildren();
    const board=get('graph-board');const width=Math.max(1800,...current().blocks.map(b=>b.position[0]+230)),height=Math.max(1000,...current().blocks.map(b=>b.position[1]+nodeHeight(b)+140));
    board.style.width=width+'px';board.style.height=height+'px';wires.setAttribute('width',String(width));wires.setAttribute('height',String(height));
    const ns='http://www.w3.org/2000/svg',defs=document.createElementNS(ns,'defs'),marker=document.createElementNS(ns,'marker'),arrow=document.createElementNS(ns,'polygon');
    marker.id='graph-arrow';for(const [key,value]of Object.entries({viewBox:'0 0 10 10',refX:'9',refY:'5',markerWidth:'5',markerHeight:'5',orient:'auto'}))marker.setAttribute(key,value);
    arrow.setAttribute('points','0,0 10,5 0,10');arrow.setAttribute('fill','#63e3bc');marker.append(arrow);defs.append(marker);wires.append(defs);
    current().edges.forEach((e,i)=>{const a=current().blocks.find(b=>b.id===e.from)!,b=current().blocks.find(b=>b.id===e.to)!;
      const curve=document.createElementNS(ns,'path');curve.setAttribute('d',wirePath(a,e.from_port,b,e.to_port));curve.setAttribute('class','wire');curve.classList.toggle('wire-selected',selectedEdge===i);
      curve.setAttribute('marker-end','url(#graph-arrow)');curve.dataset.from=e.from;curve.dataset.to=e.to;curve.dataset.edge=String(i);curve.setAttribute('role','button');curve.setAttribute('tabindex','0');curve.setAttribute('aria-label',`接続 ${e.from}:${e.from_port} → ${e.to}:${e.to_port}`);
      curve.addEventListener('click',()=>chooseEdge(i));curve.addEventListener('keydown',event=>{if(event.key==='Enter'||event.key===' '){event.preventDefault();chooseEdge(i);}});wires.append(curve);
    });
    for(const b of current().blocks){
      const node=document.createElement('div');node.className='graph-node';const address=[...path,b.id].join('/');node.classList.toggle('problem',problemBlocks.has(address));node.classList.toggle('selected',selected.has(b.id));node.dataset.block=b.id;node.style.left=b.position[0]+'px';node.style.top=b.position[1]+'px';node.style.minHeight=nodeHeight(b)+'px';node.tabIndex=0;node.setAttribute('role','button');node.setAttribute('aria-label',`ブロック ${b.id}`);
      const check=document.createElement('input');check.type='checkbox';check.checked=selected.has(b.id);check.setAttribute('aria-label',`選択 ${b.id}`);
      check.addEventListener('pointerdown',e=>e.stopPropagation());check.addEventListener('keydown',e=>{if(e.key==='Enter'||e.key===' ')e.stopPropagation();});check.addEventListener('click',e=>e.stopPropagation());check.addEventListener('change',()=>{selectedEdge=null;if(check.checked)selected.add(b.id);else selected.delete(b.id);render();});
      const title=document.createElement('strong');title.textContent=b.label;const small=document.createElement('small');small.textContent=`${KIND_NAMES[b.params.kind]} · 幅${b.width}`;
      node.append(check,title,small);
      if(problemBlocks.has(address)){const badge=document.createElement('span');badge.className='problem-badge';badge.textContent=String(problemBlocks.get(address));badge.title='このブロックまたは内部に診断があります';badge.setAttribute('aria-hidden','true');node.append(badge);}
      for(const direction of ['in','out'] as const)for(let port=0;port<(direction==='in'?inputCount(b):outputCount(b));port++){
        const endpoint=document.createElement('button');endpoint.type='button';endpoint.className=`node-port port-${direction}`;endpoint.style.top=(portY(port)-11)+'px';endpoint.textContent=String(port);
        const label=`${direction==='in'?'入力':'出力'} ${b.id}:${port}`;endpoint.setAttribute('aria-label',label);endpoint.title=`${label} · 幅${direction==='in'?inputWidth(b,port):outputWidth(b,port)}`;
        endpoint.id=`port-${direction}-${b.id}-${port}`;endpoint.dataset.port=String(port);endpoint.dataset.direction=direction;
        if(direction==='out'){const active=pendingWire?.from===b.id&&pendingWire.from_port===port;endpoint.classList.toggle('port-pending',active);endpoint.setAttribute('aria-pressed',String(active));}
        else {endpoint.classList.toggle('port-connected',current().edges.some(e=>e.to===b.id&&e.to_port===port));endpoint.classList.toggle('port-problem',problemPorts.has(`${address}:${port}`));}
        endpoint.addEventListener('pointerdown',event=>event.stopPropagation());endpoint.addEventListener('keydown',event=>{if(event.key==='Enter'||event.key===' ')event.stopPropagation();});
        endpoint.addEventListener('click',event=>{event.stopPropagation();if(direction==='out'){
          pendingWire={from:b.id,from_port:port,path:path.join('/')};previewPoint=null;selectedEdge=null;render();get<HTMLButtonElement>(`port-out-${b.id}-${port}`).focus();say(`${label} を選択しました。接続先の入力ポートをクリックしてください`);
        }else if(pendingWire){const edge={from:pendingWire.from,from_port:pendingWire.from_port,to:b.id,to_port:port};edit(g=>connectPorts(g,edge),'ポートを接続しました');}
        else say('先に出力ポートを選択してください',true);});node.append(endpoint);
      }
      const select=(multi=false)=>{selectedEdge=null;if(multi){if(selected.has(b.id))selected.delete(b.id);else selected.add(b.id);}else selected=new Set([b.id]);};
      node.addEventListener('keydown',e=>{if(e.target===node&&(e.key==='Enter'||e.key===' ')){e.preventDefault();select(e.shiftKey);renderInspector();syncSelection();}});
      node.addEventListener('dblclick',event=>{if(event.target instanceof Element&&event.target.closest('button,input'))return;if(b.params.kind==='subsystem'){path.push(b.id);selected.clear();render();}});
      node.addEventListener('pointerdown',event=>{
        if(event.button!==0)return;select(event.shiftKey||event.ctrlKey||event.metaKey);renderInspector();
        syncSelection();
        node.setPointerCapture(event.pointerId);const scale=board.getBoundingClientRect().width/board.clientWidth,start=[event.clientX,event.clientY];let position:[number,number]=[...b.position];let moved=false;
        const move=(e:PointerEvent)=>{if(Math.abs(e.clientX-start[0])+Math.abs(e.clientY-start[1])>3)moved=true;if(!moved)return;
          position=[Math.max(0,Math.min(10000,b.position[0]+(e.clientX-start[0])/scale)),Math.max(0,Math.min(10000,b.position[1]+(e.clientY-start[1])/scale))];node.style.left=position[0]+'px';node.style.top=position[1]+'px';};
        const finish=()=>{node.removeEventListener('pointermove',move);node.removeEventListener('pointerup',finish);node.removeEventListener('pointercancel',abort);if(moved)edit(g=>{g.blocks.find(n=>n.id===b.id)!.position=position;},'ブロックを移動しました');else syncSelection();};
        const abort=()=>{node.removeEventListener('pointermove',move);node.removeEventListener('pointerup',finish);node.removeEventListener('pointercancel',abort);render();};
        node.addEventListener('pointermove',move);node.addEventListener('pointerup',finish);node.addEventListener('pointercancel',abort);
      });nodes.append(node);
    }paintPreview();
    const nodeBottom=Math.max(0,...Array.from(nodes.children,n=>(n as HTMLElement).offsetTop+(n as HTMLElement).offsetHeight+40));
    const bounds=wires.getBBox(),fullWidth=Math.ceil(Math.max(width,bounds.x+bounds.width+40)),fullHeight=Math.ceil(Math.max(height,bounds.y+bounds.height+40,nodeBottom));
    board.style.width=fullWidth+'px';board.style.height=fullHeight+'px';wires.setAttribute('width',String(fullWidth));wires.setAttribute('height',String(fullHeight));applyZoom();
  }
  function render() {
    // Undoing a group may invalidate navigation; return safely to root.
    try{current();}catch{path=[];selected.clear();}
    if(pendingWire?.path!==path.join('/')){pendingWire=null;previewPoint=null;}
    get<HTMLButtonElement>('graph-wire-cancel').disabled=!pendingWire;get<HTMLButtonElement>('graph-disconnect').disabled=selectedEdge===null;
    selected=new Set([...selected].filter(id=>current().blocks.some(b=>b.id===id)));
    const crumb=get('graph-breadcrumb');crumb.replaceChildren(button('ルート',()=>{path=[];selected.clear();render();}));
    path.forEach((id,i)=>crumb.append(button(id,()=>{path=path.slice(0,i+1);selected.clear();render();})));
    get('graph-count').textContent=`${current().blocks.length} blocks · ${current().edges.length} wires`;
    get<HTMLButtonElement>('graph-undo').disabled=!history.canUndo;get<HTMLButtonElement>('graph-redo').disabled=!history.canRedo;
    const kinds=get<HTMLSelectElement>('block-kind');const previous=kinds.value;kinds.replaceChildren();
    for(const kind of Object.keys(KIND_NAMES) as Kind[])if(path.length||!['input','output'].includes(kind))kinds.add(new Option(KIND_NAMES[kind],kind));if([...kinds.options].some(o=>o.value===previous))kinds.value=previous;
    for(const id of ['edge-from','edge-to']){const select=get<HTMLSelectElement>(id),previous=select.value;select.replaceChildren();for(const b of current().blocks)select.add(new Option(`${b.label} (${b.id})`,b.id));if(current().blocks.some(b=>b.id===previous))select.value=previous;}
    refreshDiagnostics();renderPorts();renderNodes();renderInspector();syncCopy();syncDirty();renderSearch();renderDiagnostics();renderComparison();sweep?.refresh();frequency?.refresh();poleZero?.refresh();stateSpace?.refresh();
    const edges=get('graph-edges');edges.replaceChildren();current().edges.forEach((e,i)=>{
      const row=document.createElement('div');row.className='edge-row';const text=document.createElement('span');text.textContent=`${e.from}:${e.from_port} → ${e.to}:${e.to_port}`;
      const remove=button('×',()=>edit(g=>{g.edges.splice(i,1);},'接続を削除しました'));remove.setAttribute('aria-label',`接続を削除 ${e.from} ${e.to} ${e.to_port}`);row.append(text,remove);edges.append(row);
    });
    get<HTMLInputElement>('graph-dt').value=String(history.graph.dt);get<HTMLInputElement>('graph-duration').value=String(history.graph.duration);
    const settings=history.graph.solver;get<HTMLSelectElement>('graph-solver').value=settings?.method||'rk4';
    const rk45=settings?.method==='rk45'?settings:{abs_tolerance:1e-8,rel_tolerance:1e-6,min_step:1e-10,max_step:.5};
    for(const [id,value] of [['graph-atol',rk45.abs_tolerance],['graph-rtol',rk45.rel_tolerance],['graph-min-step',rk45.min_step],['graph-max-step',rk45.max_step]] as const)get<HTMLInputElement>(id).value=String(value);
    solverFields();
  }
  function cursor(){if(!result)return;const index=Number(get<HTMLInputElement>('graph-cursor').value);get('graph-values').textContent=`t=${result.times[index].toFixed(4)}s · `+seriesFrom(result).filter(s=>visible.has(s.key)).map(s=>`${s.label}=${s.values[index].toPrecision(6)}`).join(' / ');
    if(referenceRun&&comparison){
      const previous=new Map(referenceRun.result.traces.map(t=>[t.id,t])),current=new Map(result.traces.map(t=>[t.id,t]));
      get('graph-reference-values').textContent=comparison.sameTimes?comparison.rows.filter(r=>visible.has(r.key)).map(r=>{const a=current.get(r.id)!.values[index][r.component],b=previous.get(r.id)!.values[index][r.component],delta=a-b;return `${r.label} (${r.id}) [${r.component}]: 基準=${b.toPrecision(6)} · Δ=${Number.isFinite(delta)?delta.toPrecision(6):'範囲外'}`;}).join(' / '):'記録時刻が異なるため、カーソルの数値差分は比較できません。';
    }else get('graph-reference-values').textContent='';
  }
  function showResult(data:GraphResult,model:GraphModel){
    result=data;resultModel=model;comparison=referenceRun?compareResults(data,referenceRun.result):null;comparisonPage=0;visible=new Set(seriesFrom(data).slice(0,16).map(s=>s.key));const area=get('graph-series');area.replaceChildren();
    for(const series of seriesFrom(data)){const label=document.createElement('label');label.className='check';label.style.color=series.color;const check=document.createElement('input');check.type='checkbox';check.checked=visible.has(series.key);check.setAttribute('aria-label',`表示 ${series.label}`);
      check.addEventListener('change',()=>{if(check.checked&&visible.size>=16){check.checked=false;say('表示は最大16系列です',true);return;}if(check.checked)visible.add(series.key);else visible.delete(series.key);plotResult();cursor();});label.append(check,document.createTextNode(series.label));area.append(label);}
    const slider=get<HTMLInputElement>('graph-cursor');slider.disabled=false;slider.max=String(data.times.length-1);slider.value=String(data.times.length-1);get<HTMLButtonElement>('graph-csv').disabled=false;
    const stats=solverStatistics(model,data);get('graph-statistics').textContent=`${model.solver?.method==='rk45'?'RK45':'RK4'} · 積分 ${stats.accepted_steps} 回 · 棄却 ${stats.rejected_steps} 回 · 微分評価 ${stats.rate_evaluations} 回 · 記録評価 ${stats.output_evaluations} 回 · 内部刻み ${stats.min_step.toExponential(3)}〜${stats.max_step.toExponential(3)} s${model.solver?.method==='rk45'?` · 最大推定誤差比 ${stats.max_error_ratio.toPrecision(4)}`:''}`;
    renderComparison();plotResult();cursor();say(`完了 · ${data.times.length} 点 · ${data.traces.length} Scope · Rust / WASM`);
  }
  function run(){guard(()=>{
    if(archiveBusy||(sweep?.running()||frequency?.running()||poleZero?.running()||stateSpace?.running()||conversionComparison?.running()))throw new Error('実行結果ファイルの確認が終わるか、中止してから実行してください');
    refreshDiagnostics(true);renderNodes();renderDiagnostics();
    if(!validation.runnable){get<HTMLDetailsElement>('graph-diagnostics').open=true;throw new Error(validation.diagnostics[0].message);}
    const json=JSON.stringify(history.graph);cancel();const id=++request;
    get<HTMLButtonElement>('graph-run').disabled=true;get<HTMLButtonElement>('graph-cancel').disabled=false;say('モデルを計算中…');
    worker=new Worker(new URL('./graph-worker.ts',import.meta.url),{type:'module'});
    worker.onmessage=(event:MessageEvent<{id:number,result?:GraphResult,error?:string}>)=>{if(event.data.id!==request)return;cancel();if(event.data.error){runtimeError=event.data.error;renderDiagnostics();get<HTMLDetailsElement>('graph-diagnostics').open=true;say(event.data.error,true);}else if(event.data.result)showResult(event.data.result,JSON.parse(json) as GraphModel);};
    worker.onerror=event=>{if(id!==request)return;cancel();runtimeError=`計算ワーカーの起動に失敗: ${event.message}`;renderDiagnostics();get<HTMLDetailsElement>('graph-diagnostics').open=true;say(runtimeError,true);};worker.postMessage({id,json});renderComparison();
  });}
  get('block-add').addEventListener('click',()=>edit(g=>{const block=newBlock(get<HTMLSelectElement>('block-kind').value as Kind,g);g.blocks.push(block);selected=new Set([block.id]);},'ブロックを追加しました'));
  get('graph-delete').addEventListener('click',()=>edit(g=>deleteBlocks(g,selected),'選択と接続を削除しました'));
  get('graph-ungroup').addEventListener('click',()=>edit(g=>{if(selected.size!==1)throw new Error('解除するサブシステムを1つ選択してください');selected=new Set(ungroupBlock(g,[...selected][0]));},'サブシステムを解除しました'));
  get('graph-group').addEventListener('click',()=>edit(g=>{selected=new Set([groupBlocks(g,selected)]);},'サブシステムを作成しました'));
  const undo=()=>{guard(()=>{history.undo();invalidate();render();say('Undo');});},redo=()=>{guard(()=>{history.redo();invalidate();render();say('Redo');});};
  const copy=()=>guard(()=>{clipboard=copySelection(transfer(current()),selected);pasteCount=0;syncCopy();say(`${clipboard.blocks.length} 個のブロックと内部接続をコピーしました`);});
  const paste=()=>{if(clipboard&&edit(g=>{selected=new Set(pasteSelection(transfer(g),clipboard!,40*(pasteCount+1)));},'ブロックを貼り付けました'))pasteCount++;};
  const duplicate=()=>{if(edit(g=>{selected=new Set(pasteSelection(transfer(g),copySelection(transfer(g),selected)));},'選択したブロックを複製しました'))syncCopy();};
  const selectAll=()=>{selected=new Set(current().blocks.map(b=>b.id));selectedEdge=null;render();};
  get('graph-select-all').addEventListener('click',selectAll);
  get('graph-copy').addEventListener('click',copy);get('graph-paste').addEventListener('click',paste);get('graph-duplicate').addEventListener('click',duplicate);
  get('graph-zoom-in').addEventListener('click',()=>setZoom(zoom+.1));get('graph-zoom-out').addEventListener('click',()=>setZoom(zoom-.1));
  get('graph-zoom-reset').addEventListener('click',()=>setZoom(1));get('graph-fit').addEventListener('click',fit);
  get('graph-layout').addEventListener('click',()=>guard(()=>{
    const planned=structuredClone(current()),layout=autoLayout(planned);
    if(!layout.changed){say('すでに自動配置されています');return;}
    if(edit(g=>{g.blocks.forEach((b,i)=>{b.position=planned.blocks[i].position;});},layout.mode==='grid'?'モデルをコンパクトな格子に自動配置しました':'モデルを信号の流れに沿って自動配置しました'))fit();
  }));
  get('graph-undo').addEventListener('click',undo);get('graph-redo').addEventListener('click',redo);
  const cancelWire=()=>{pendingWire=null;previewPoint=null;render();say('配線をキャンセルしました');};
  get('graph-wire-cancel').addEventListener('click',cancelWire);
  const disconnect=()=>{if(selectedEdge!==null){const index=selectedEdge;edit(g=>{g.edges.splice(index,1);},'接続を削除しました');}};
  get('graph-disconnect').addEventListener('click',disconnect);
  get('graph-board').addEventListener('pointermove',event=>{if(!pendingWire)return;const rect=get('graph-board').getBoundingClientRect(),scale=rect.width/get('graph-board').clientWidth;previewPoint=[(event.clientX-rect.left)/scale,(event.clientY-rect.top)/scale];paintPreview();});
  const searchPanel=get<HTMLDetailsElement>('graph-search');
  const openSearch=()=>{searchPanel.open=true;const input=get<HTMLInputElement>('graph-query');input.focus();input.select();};
  for(const kind of Object.keys(KIND_NAMES) as Kind[])get<HTMLSelectElement>('graph-search-kind').add(new Option(KIND_NAMES[kind],kind));
  get('graph-query').addEventListener('input',renderSearch);get('graph-search-kind').addEventListener('change',renderSearch);get('graph-search-scope').addEventListener('change',renderSearch);
  get('graph-search-clear').addEventListener('click',()=>{get<HTMLInputElement>('graph-query').value='';get<HTMLSelectElement>('graph-search-kind').value='';renderSearch();get('graph-query').focus();});
  get('graph-search-form').addEventListener('submit',event=>{event.preventDefault();if(searchHits.length)guard(()=>revealBlock(searchHits[0]));});
  document.addEventListener('keydown',event=>{if(host.hidden)return;
    const typing=event.target instanceof HTMLElement&&['INPUT','TEXTAREA','SELECT'].includes(event.target.tagName);
    if((event.ctrlKey||event.metaKey)&&event.key.toLowerCase()==='k'&&(!typing||event.target===get('graph-query'))){event.preventDefault();openSearch();return;}
    if(event.key==='Escape'&&!pendingWire&&searchPanel.open){event.preventDefault();const fromSearch=event.target instanceof Node&&searchPanel.contains(event.target);searchPanel.open=false;if(fromSearch)searchPanel.querySelector<HTMLElement>('summary')!.focus();return;}
if(event.key==='Escape'&&pendingWire){event.preventDefault();cancelWire();return;}if(event.target instanceof HTMLElement&&['INPUT','TEXTAREA','SELECT'].includes(event.target.tagName))return;if(event.key==='Delete'||event.key==='Backspace'){if(selectedEdge!==null){event.preventDefault();disconnect();}return;}if(!event.ctrlKey&&!event.metaKey)return;if(event.target instanceof HTMLElement&&['INPUT','TEXTAREA','SELECT'].includes(event.target.tagName))return;if(event.key.toLowerCase()==='a'){event.preventDefault();selectAll();}else if(event.key.toLowerCase()==='c'){event.preventDefault();copy();}else if(event.key.toLowerCase()==='v'){event.preventDefault();paste();}else if(event.key.toLowerCase()==='d'){event.preventDefault();duplicate();}else if(event.key.toLowerCase()==='z'){event.preventDefault();if(event.shiftKey)redo();else undo();}else if(event.key.toLowerCase()==='y'){event.preventDefault();redo();}});
  get('graph-check-run').addEventListener('click',()=>{refreshDiagnostics(true);renderNodes();renderDiagnostics();get<HTMLDetailsElement>('graph-diagnostics').open=true;});
  get('graph-diagnostics-prev').addEventListener('click',()=>{diagnosticPage--;renderDiagnostics();});get('graph-diagnostics-next').addEventListener('click',()=>{diagnosticPage++;renderDiagnostics();});
  get('graph-run').addEventListener('click',run);get('graph-cancel').addEventListener('click',()=>{cancel();say('実行を中止しました');});
  get('edge-from').addEventListener('change',renderPorts);get('edge-to').addEventListener('change',renderPorts);
  get<HTMLFormElement>('graph-connect').addEventListener('submit',event=>{event.preventDefault();edit(g=>{
    const from=get<HTMLSelectElement>('edge-from').value,to=get<HTMLSelectElement>('edge-to').value;
    if(!get<HTMLSelectElement>('edge-from-port').value||!get<HTMLSelectElement>('edge-to-port').value)throw new Error('有効なポートを指定してください');
    connectPorts(g,{from,from_port:Number(get<HTMLSelectElement>('edge-from-port').value),to,to_port:Number(get<HTMLSelectElement>('edge-to-port').value)});
  },'接続を追加しました');});
  function solverFields(){const field=get<HTMLFieldSetElement>('graph-rk45-settings');field.hidden=get<HTMLSelectElement>('graph-solver').value!=='rk45';field.disabled=field.hidden;}
  get('graph-solver').addEventListener('change',solverFields);
  get<HTMLFormElement>('graph-clock').addEventListener('submit',event=>{event.preventDefault();guard(()=>{history.apply(root=>{
    setClock(root,parseNumber(get<HTMLInputElement>('graph-dt').value,'dt'),parseNumber(get<HTMLInputElement>('graph-duration').value,'duration'));
    if(get<HTMLSelectElement>('graph-solver').value==='rk45')setSolver(root,{method:'rk45',abs_tolerance:parseNumber(get<HTMLInputElement>('graph-atol').value,'atol'),rel_tolerance:parseNumber(get<HTMLInputElement>('graph-rtol').value,'rtol'),min_step:parseNumber(get<HTMLInputElement>('graph-min-step').value,'min_step'),max_step:parseNumber(get<HTMLInputElement>('graph-max-step').value,'max_step')});
    else if(root.solver)setSolver(root,{method:'rk4'});
  });invalidate();render();say('時間設定を適用しました');});});
  function replace(graph:GraphModel){
    if(dirty()&&JSON.stringify(graph)!==JSON.stringify(history.graph)&&!window.confirm('未保存の変更があります。変更を破棄してモデルを置き換えますか？')){say('モデルの置き換えを中止しました');return false;}
    history.replace(graph);path=[];selected.clear();invalidate();render();return true;
  }
  get('graph-load-sample').addEventListener('click',()=>guard(()=>{if(replace(SAMPLES[get<HTMLSelectElement>('graph-sample').value]))run();}));
  get('graph-empty').addEventListener('click',()=>guard(()=>{if(replace({version:1,dt:0.01,duration:5,blocks:[],edges:[]}))say('新しいモデルを作成しました');}));
  get('graph-save').addEventListener('click',()=>guard(()=>{localStorage.setItem(GRAPH_STORAGE_KEY,encodeGraphDocument(history.graph));baseline=JSON.stringify(history.graph);syncDirty();say('モデルを保存しました');}));
  get('graph-restore').addEventListener('click',()=>guard(()=>{const text=localStorage.getItem(GRAPH_STORAGE_KEY);if(text===null)throw new Error('保存済みモデルがありません');if(replace(decodeGraphDocument(text,normalize_graph))){baseline=JSON.stringify(history.graph);syncDirty();say('モデルを復元しました');}}));
  get('graph-export').addEventListener('click',()=>download(encodeGraphDocument(history.graph),'blocksim-graph.json','application/json'));
  get('graph-csv').addEventListener('click',()=>{if(result)download(resultCsv(result),'blocksim-signals.csv','text/csv');});
  get('graph-reference-set').addEventListener('click',()=>guard(()=>{if(!result||!resultModel||worker||archiveBusy||(sweep?.running()||frequency?.running()||poleZero?.running()||stateSpace?.running()||conversionComparison?.running()))throw new Error('実行・読み込みが完了してから基準を保持してください');referenceRun=snapshotRun(resultModel,result);comparison=compareResults(result,referenceRun.result);comparisonPage=0;get<HTMLDetailsElement>('graph-comparison').open=true;renderComparison();plotResult();cursor();}));
  get('graph-reference-clear').addEventListener('click',()=>{if(archiveBusy)return;referenceRun=null;comparison=null;comparisonPage=0;renderComparison();plotResult();cursor();});
  get('graph-reference-export').addEventListener('click',()=>{if(referenceRun)download(encodeGraphDocument(referenceRun.model),'blocksim-reference-model.json','application/json');});
  get('graph-reference-show').addEventListener('change',plotResult);
  get('graph-comparison-prev').addEventListener('click',()=>{comparisonPage--;renderComparison();});get('graph-comparison-next').addEventListener('click',()=>{comparisonPage++;renderComparison();});
  get('graph-difference-csv').addEventListener('click',()=>guard(()=>{if(result&&referenceRun&&comparison)download(differenceCsv(result,referenceRun.result,comparison),'blocksim-difference.csv','text/csv');}));
  get('graph-run-export').addEventListener('click',()=>guard(()=>{if(result&&resultModel&&!worker&&!archiveBusy)download(encodeRunArchive({model:resultModel,result}),'blocksim-run.json','application/json');}));
  get('graph-reference-save').addEventListener('click',()=>guard(()=>{if(referenceRun&&!worker&&!archiveBusy)download(encodeRunArchive(referenceRun),'blocksim-reference-run.json','application/json');}));
  const archiveState=(message:string,error=false)=>{const state=get('graph-reference-import-state');state.textContent=message;state.classList.toggle('error',error);};
  const finishArchive=()=>{archiveRequest++;archiveWorker?.terminate();archiveWorker=null;archiveBusy=false;get<HTMLInputElement>('graph-reference-import').value='';renderComparison();};
  get('graph-reference-cancel').addEventListener('click',()=>{finishArchive();archiveState('実行結果ファイルの読み込みを中止しました');});
  get<HTMLInputElement>('graph-reference-import').addEventListener('change',async event=>{
    const target=event.target as HTMLInputElement,file=target.files?.[0];if(!file)return;
    if(worker||archiveBusy||(sweep?.running()||frequency?.running()||poleZero?.running()||stateSpace?.running()||conversionComparison?.running())){target.value='';return;}
    const id=++archiveRequest;archiveBusy=true;get<HTMLDetailsElement>('graph-run-files').open=true;archiveState('実行結果ファイルを読み込み中…');renderComparison();
    try{
      checkArchiveSize(file.size);const text=await file.text();if(id!==archiveRequest)return;
      const candidate=decodeRunArchive(text,validate_graph);archiveState('保存したモデルを WASM で再計算して確認中…');
      archiveWorker=new Worker(new URL('./graph-worker.ts',import.meta.url),{type:'module'});
      archiveWorker.onmessage=(message:MessageEvent<{id:number,result?:GraphResult,error?:string}>)=>{
        if(id!==archiveRequest||message.data.id!==id)return;
        try{
          if(message.data.error)throw new Error(`保存したモデルの再計算に失敗: ${message.data.error}`);
          if(!message.data.result)throw new Error('保存したモデルの再計算結果がありません');
          verifyArchivedResult(candidate.result,message.data.result);
          referenceRun={model:candidate.model,result:message.data.result};comparison=result?compareResults(result,referenceRun.result):null;comparisonPage=0;get<HTMLDetailsElement>('graph-comparison').open=true;
          finishArchive();plotResult();cursor();archiveState(`再計算と全記録点の一致を確認し、基準を読み込みました · ${referenceRun.result.times.length} 点`);
        }catch(error){finishArchive();archiveState(String(error),true);}
      };
      archiveWorker.onerror=error=>{if(id!==archiveRequest)return;finishArchive();archiveState(`確認用ワーカーの起動に失敗: ${error.message}`,true);};
      archiveWorker.postMessage({id,json:JSON.stringify(candidate.model)});
    }catch(error){if(id===archiveRequest){finishArchive();archiveState(String(error),true);}}
    finally{if(id===archiveRequest)target.value='';}
  });
  get<HTMLInputElement>('graph-import').addEventListener('change',async event=>{
    const target=event.target as HTMLInputElement,file=target.files?.[0];if(!file)return;
    try{if(file.size>1_000_000)throw new Error('モデルは1 MB以下にしてください');const incoming=decodeGraphDocument(await file.text(),normalize_graph);if(get<HTMLSelectElement>('graph-import-mode').value==='merge'){edit(g=>{selected=new Set(mergeGraph(transfer(g),incoming));},'モデルを追加しました（既存の時計・終了時刻を保持）');}else if(replace(incoming))say('モデル JSON を読み込みました');}
    catch(error){say(String(error),true);}finally{target.value='';}
  });
  get('graph-cursor').addEventListener('input',cursor);
  // File reads can finish while PID Lab is visible. Measure wires after revealing the editor.
  host.addEventListener('graph-visible',renderNodes);
  let savedError:string|null=null;
  try{const text=localStorage.getItem(GRAPH_STORAGE_KEY);if(text!==null)history.replace(decodeGraphDocument(text,normalize_graph));}catch(error){savedError=String(error);}
  sweep=mountSweep(get('graph-sweep-host'),{model:()=>history.graph,normalize:validate_graph,busy:()=>!!worker||archiveBusy||(!!frequency?.running()||(!!poleZero?.running()||(!!stateSpace?.running()||!!conversionComparison?.running()))),onBusy:renderComparison,download,replace,apply:(model,data)=>{if(replace(model))showResult(data,structuredClone(history.graph));},reference:(model,data)=>{referenceRun=snapshotRun(model,data);comparison=result?compareResults(result,data):null;comparisonPage=0;get<HTMLDetailsElement>('graph-comparison').open=true;renderComparison();plotResult();cursor();say('掃引結果を基準として保持しました');}});
  frequency=mountFrequency(get('graph-frequency-host'),{model:()=>history.graph,normalize:validate_graph,check:(model,request,domain)=>domain==='discrete'?validate_discrete_frequency(model,request):validate_frequency(model,request),busy:()=>!!worker||archiveBusy||!!sweep?.running()||(!!poleZero?.running()||(!!stateSpace?.running()||!!conversionComparison?.running())),onBusy:renderComparison,download,replace});
  poleZero=mountPoleZero(get('graph-pole-zero-host'),{model:()=>history.graph,normalize:validate_graph,check:validate_pole_zero,busy:()=>!!worker||archiveBusy||!!sweep?.running()||!!frequency?.running()||(!!stateSpace?.running()||!!conversionComparison?.running()),onBusy:renderComparison,download,replace});
  stateSpace=mountStateSpace(get('graph-state-space-host'),{model:()=>history.graph,normalize:validate_graph,check:(model,request,domain)=>domain==='discrete'?validate_discrete_state_space(model,request):validate_state_space(model,request),busy:()=>!!worker||archiveBusy||!!sweep?.running()||!!frequency?.running()||!!poleZero?.running()||!!conversionComparison?.running(),onBusy:renderComparison,download,replace});
  baseline=JSON.stringify(history.graph);render();if(savedError)say(`保存モデルの読み込みに失敗: ${savedError}`,true);else run();
}
