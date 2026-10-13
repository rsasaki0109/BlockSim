import {generateSampleTable,isTableWave,type InputShape} from './sampled-input-generator';
import {encodeInputGenerator,decodeInputGenerator,INPUT_GENERATOR_FILE_LIMIT} from './sampled-input-generator-file';
import {planGeneratedInput} from './sampled-input-generation-plan';
import {inputPreviewModels} from './sampled-input-preview';
export function mountInputGenerator(host:HTMLElement,options:{values:HTMLTextAreaElement;width:HTMLInputElement;period:HTMLInputElement;repeat:HTMLInputElement;record_dt:()=>number;normalize:(text:string)=>string;busy:()=>boolean;ioEpoch:()=>number;download:(text:string,name:string,type:string)=>void}) {
 host.id='graph-input-generator';host.innerHTML=`<details><summary>波形から入力表を生成</summary><fieldset id="graph-input-generator-fields"><label>生成する波形<select id="graph-input-generator-shape"><option value="constant">定数</option><option value="step">ステップ</option><option value="ramp">ランプ</option><option value="sine">正弦波</option><option value="square">矩形波</option><option value="triangle">三角波</option><option value="pulse">パルス列</option></select></label><label>生成行数<input id="graph-input-generator-rows" type="text" inputmode="numeric" value="11"></label><label>基準値（数値または成分順JSON）<input id="graph-input-generator-base" type="text" value="0"></label><label id="graph-input-generator-delta-label">変化量（数値または成分順JSON）<input id="graph-input-generator-delta" type="text" value="1"></label><label id="graph-input-generator-step-label">切り替え行（0から）<input id="graph-input-generator-step" type="text" inputmode="numeric" value="5"></label><label id="graph-input-generator-cycles-label">表内の周期数<input id="graph-input-generator-cycles" type="text" inputmode="numeric" value="1"></label><label id="graph-input-generator-phase-label">初期位相（rad / 数値または成分順JSON）<input id="graph-input-generator-phase" type="text" value="0"></label><label id="graph-input-generator-duty-label">デューティ比（% / 数値または成分順JSON）<input id="graph-input-generator-duty" type="text" value="50"></label><button id="graph-input-generator-plan" type="button">生成予定の条件を確認</button><button id="graph-input-generator-run" type="button">入力表を生成して置換</button><div class="buttons"><button id="graph-input-generator-export" type="button">生成条件JSONを保存</button><label class="file-button">生成条件JSONを読み込む<input id="graph-input-generator-import" type="file" accept=".json,application/json"></label></div></fieldset><p class="hint">数値は全成分に共通、配列は信号幅と同じ長さ。ステップは切り替え行から基準値＋変化量、ランプは最終行までに変化量を加算、正弦波は変化量が振幅、初期位相は−2π〜2π radです。矩形波・三角波・パルス列は1周期2行以上で行数を周期数で割り切る必要があります。デューティ比は0〜100%。矩形波は基準値±変化量、パルス列は基準値と基準値＋変化量を切り替えます。生成条件JSONは100 KBまで。読み込みは条件だけを復元し、信号幅は現在の設定欄と同じ値が必要です。周期・繰り返しはファイルに含みません。生成は設定欄の表を置換します。周期・繰り返しは保持し、プレビューで確認後にブロック設定を適用してください。</p><section id="graph-input-generator-plan-output" aria-live="polite"></section></details><p id="graph-input-generator-state" role="status" aria-live="polite"></p>`;
 const get=<T extends HTMLElement>(id:string)=>host.querySelector<T>('#graph-input-generator-'+id)!;
 const message=(text:string,error=false)=>{get('state').textContent=text;get('state').classList.toggle('error',error);};
 let revision=0,readRequest=0;
 const file=get<HTMLInputElement>('import'),form=host.closest('form')!;
 const clearPlan=()=>get('plan-output').replaceChildren();
 const changed=(event:Event)=>{if(event.target!==file){revision++;if(event.target instanceof Node&&(host.contains(event.target)||event.target===options.width||event.target===options.period||event.target===options.repeat))clearPlan();}};form.addEventListener('input',changed);form.addEventListener('change',changed);
 function sync(){const shape=get<HTMLSelectElement>('shape').value;for(const [field,visible] of [['delta',shape!=='constant'],['step',shape==='step'],['cycles',shape==='sine'||isTableWave(shape)],['phase',shape==='sine'],['duty',shape==='square'||shape==='pulse']] as const){get(field+'-label').hidden=!visible;get<HTMLInputElement>(field).disabled=!visible;}get<HTMLFieldSetElement>('fields').disabled=options.busy();}
 get('shape').addEventListener('change',sync);
 host.addEventListener('keydown',event=>{if(event.key==='Enter'&&event.target instanceof HTMLInputElement)event.preventDefault();});
 function read(){
  const shape=get<HTMLSelectElement>('shape').value as InputShape;
  const number=(id:string)=>{const value=get<HTMLInputElement>(id).value;return /^[+-]?\d+$/.test(value.trim())?Number(value):NaN;};
  const vector=(id:string)=>{const text=get<HTMLInputElement>(id).value.trim();return /^[+-]?(?:\d+(?:\.\d*)?|\.\d+)(?:[eE][+-]?\d+)?$/.test(text)?Number(text):JSON.parse(text);};
  return {shape,width:Number(options.width.value),rows:number('rows'),base:vector('base'),delta:shape==='constant'?undefined:vector('delta'),step:number('step'),cycles:number('cycles'),phase:shape==='sine'?vector('phase'):undefined,duty:shape==='square'||shape==='pulse'?vector('duty'):undefined};
 }
 get('plan').addEventListener('click',()=>{try{
  if(options.busy())throw new Error('実行・解析の完了を待ってください');
  clearPlan();const request=read(),sample_time=Number(options.period.value),plan=planGeneratedInput(request,sample_time,options.repeat.checked);
  const {guard}=inputPreviewModels({values:plan.values,width:plan.width,sample_time,repeat:plan.repeat,record_dt:options.record_dt(),cycles:1});options.normalize(JSON.stringify(guard));
  const output=get('plan-output'),summary=document.createElement('p'),fmt=(value:number)=>String(Number(value.toPrecision(7)));summary.id='graph-input-generator-plan-summary';
  summary.textContent=`生成予定 · ${plan.rows}行 × ${plan.width}成分 · Ts=${fmt(plan.sample_time)}s · 表1回の保持時間=${fmt(plan.table_time)}s · 最終行の取得=${fmt(plan.last_sample_time)}s · ${plan.repeat?'表を繰り返す':'最終行を保持'}`;
  if(plan.period)summary.textContent+=` · 波形の周期=${fmt(plan.period.rows)}${request.shape==='sine'?'行相当':'行'}/${fmt(plan.period.time)}s · 周波数=${fmt(plan.period.frequency)}Hz`;
  if(plan.step_time!==undefined)summary.textContent+=` · 切り替え=${fmt(plan.step_time)}s`;
  if(plan.ramp_end_time!==undefined)summary.textContent+=` · ランプの到達=${fmt(plan.ramp_end_time)}s`;
  const wrap=document.createElement('div');wrap.className='table-wrap';const table=document.createElement('table');table.id='graph-input-generator-plan-table';table.setAttribute('aria-label','生成予定の成分範囲とオン区間');
  const head=document.createElement('thead'),header=document.createElement('tr'),duty=plan.channels[0].duty!==undefined;
  for(const label of ['成分','予定最小','予定最大',...(duty?['指定比率 (%)','実効比率 (%)','オン行 / 周期行','オン時間 (s)']:[])]){const th=document.createElement('th');th.scope='col';th.textContent=label;header.append(th);}head.append(header);table.append(head);
  const body=document.createElement('tbody');for(const channel of plan.channels){const tr=document.createElement('tr');tr.dataset.component=String(channel.component);const cells=[String(channel.component),fmt(channel.min),fmt(channel.max),...(channel.duty?[fmt(channel.duty.requested),fmt(channel.duty.effective),`${channel.duty.on_rows} / ${plan.period!.rows}`,fmt(channel.duty.on_time)]:[])];cells.forEach((text,k)=>{const cell=document.createElement(k===0?'th':'td');if(k===0)(cell as HTMLTableCellElement).scope='row';cell.textContent=text;tr.append(cell);});body.append(tr);}table.append(body);wrap.append(table);
  const hint=document.createElement('p');hint.className='hint';hint.textContent='予定値は生成表の計算値です。オン区間の比率は保持する行から算出し、プレビューの終端取得点を加算しません。実取得値は生成後のプレビューで確認してください。';output.append(summary,wrap,hint);
  message('生成予定の条件を確認しました。入力表・モデル・結果は保持しています。');
 }catch(error){message(String(error),true);}});
 get('run').addEventListener('click',()=>{try{
  if(options.busy())throw new Error('実行・解析の完了を待ってください');
  const rows=generateSampleTable(read());
  options.values.value=JSON.stringify(rows);options.values.dispatchEvent(new Event('input',{bubbles:true}));
  message(`入力表を生成しました · ${rows.length}行 × ${rows[0].length}列。プレビューで確認し、ブロック設定を適用してください。`);
 }catch(error){message(String(error),true);}});
 get('export').addEventListener('click',()=>{try{
  if(options.busy())throw new Error('実行・解析の完了を待ってください');
  options.download(encodeInputGenerator(read()),'blocksim-input-generator.json','application/json');message('生成条件JSONを保存しました。入力表・モデル・結果は保持します。');
 }catch(error){message(String(error),true);}});
 file.addEventListener('change',async()=>{
  const selected=file.files?.[0];if(!selected)return;const id=++readRequest,version=revision,epoch=options.ioEpoch();const active=()=>id===readRequest&&host.isConnected;
  try{
   if(options.busy())throw new Error('実行・解析の完了を待ってください');
   if(selected.size>INPUT_GENERATOR_FILE_LIMIT)throw new Error('生成条件JSONは100 KBまでです');
   message('生成条件JSONを読み込み中…');const text=await selected.text();if(!active())return;
   if(version!==revision||epoch!==options.ioEpoch()||options.busy()){message('設定変更・実行のため生成条件の読み込みを破棄しました');return;}
   const request=decodeInputGenerator(text);if(request.width!==Number(options.width.value))throw new Error(`生成条件の信号幅${request.width}は設定欄と一致しません。信号幅を確認してください。`);
   get<HTMLSelectElement>('shape').value=request.shape;get<HTMLInputElement>('rows').value=String(request.rows);
   for(const key of ['base','delta'] as const)get<HTMLInputElement>(key).value=JSON.stringify(request[key]??1);
   get<HTMLInputElement>('step').value=String(request.step??5);get<HTMLInputElement>('cycles').value=String(request.cycles??1);get<HTMLInputElement>('phase').value=JSON.stringify(request.phase??0);get<HTMLInputElement>('duty').value=JSON.stringify(request.duty??50);sync();get('shape').dispatchEvent(new Event('input',{bubbles:true}));
   message('生成条件JSONを読み込みました。入力表は保持しています。「入力表を生成して置換」で反映してください。');
  }catch(error){if(active()&&version===revision&&epoch===options.ioEpoch())message(String(error),true);}
  finally{if(id===readRequest)file.value='';}
 });
 sync();return {sync};
}
