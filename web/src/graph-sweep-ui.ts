import {type GraphModel,type GraphResult} from './graph-model';
import {sweepParameters,sweepScopes,validateSweepPlan,decodeSweepPlan,encodeSweepPlan,sweepCsv,type SweepPlan,type SweepRow} from './graph-sweep';
interface Options {
 model:()=>GraphModel;normalize:(json:string)=>string;busy:()=>boolean;onBusy:()=>void;
 reference:(model:GraphModel,result:GraphResult)=>void;apply:(model:GraphModel,result:GraphResult)=>void;
 replace:(model:GraphModel)=>boolean;
 download:(text:string,name:string,type:string)=>void;
}
export function mountSweep(host:HTMLElement,options:Options){
 host.innerHTML=`<details id="graph-sweep" class="panel"><summary>パラメータ掃引</summary>
 <p class="hint">1パラメータを最大32通りで計算します。元のモデルと結果を保持し、各ケースを独立に実行。真偽値は0/1です。</p>
 <form id="graph-sweep-form"><fieldset id="graph-sweep-fields"><legend>掃引と評価</legend>
 <label>対象ブロック<select id="graph-sweep-block"></select></label><label>対象パラメータ<select id="graph-sweep-parameter"></select></label>
 <label>掃引値 (JSON 配列)<textarea id="graph-sweep-values" rows="2" required></textarea></label>
 <label>評価 Scope<select id="graph-sweep-scope"></select></label><label>評価成分<input id="graph-sweep-component" type="number" min="0" step="1" value="0" required></label>
 <label>評価目標値<input id="graph-sweep-reference" type="number" step="any" value="1" required></label>
 <label>評価開始時刻 (s)<input id="graph-sweep-start" type="number" step="any" value="0" required></label><label>評価終了時刻 (s)<input id="graph-sweep-end" type="number" step="any" required></label>
 </fieldset><button id="graph-sweep-run" class="primary">掃引を実行</button></form>
 <div class="buttons"><button id="graph-sweep-cancel" disabled>掃引を中止</button><button id="graph-sweep-export">掃引設定を保存</button><label class="file-button">掃引設定を読み込む<input id="graph-sweep-import" type="file" accept=".json,application/json"></label><button id="graph-sweep-csv" disabled>掃引 CSV</button></div>
 <p id="graph-sweep-state" role="status" aria-live="polite">評価窓は記録時計に揃えて指定してください。</p>
 <p class="hint">IAE = ∫|出力 − 目標|dt、ISE = ∫(出力 − 目標)²dt。指定窓の全記録点を台形則で評価します。設定ファイルはモデルも含みます（1 MBまで）。読み込みはモデルを置き換えます。記録量は全ケースで200万セル、実際のブロック成分評価は2000万回まで。実行エラー後のケースは未実行とします。</p>
 <p id="graph-sweep-caption" class="hint"></p><div class="table-wrap"><table aria-label="掃引結果"><thead><tr><th>値</th><th>状態</th><th>終点</th><th>最小</th><th>最大</th><th>IAE</th><th>ISE</th><th>微分評価</th><th>結果を使う</th></tr></thead><tbody id="graph-sweep-rows"></tbody></table></div></details>`;
 const get=<T extends HTMLElement=HTMLElement>(id:string)=>host.querySelector<T>('#'+id)!;
 const value=(id:string)=>get<HTMLInputElement>('graph-sweep-'+id).value;
 const state=(message:string,error=false)=>{get('graph-sweep-state').textContent=message;get('graph-sweep-state').classList.toggle('error',error);};
 let worker:Worker|null=null,generation=0,importGeneration=0,plan:SweepPlan|null=null,rows:SweepRow[]=[],modelKey='';
 let parameters= sweepParameters(options.model());
 function sync(){const busy=!!worker||options.busy();get<HTMLButtonElement>('graph-sweep-run').disabled=busy;get<HTMLButtonElement>('graph-sweep-cancel').disabled=!worker;get<HTMLFieldSetElement>('graph-sweep-fields').disabled=busy;get<HTMLButtonElement>('graph-sweep-export').disabled=busy;get<HTMLInputElement>('graph-sweep-import').disabled=busy;get<HTMLButtonElement>('graph-sweep-csv').disabled=!plan||!!worker||!rows.length;for(const button of get('graph-sweep-rows').querySelectorAll<HTMLButtonElement>('button'))button.disabled=busy;}
 function parameterOptions(reset=false){
  const select=get<HTMLSelectElement>('graph-sweep-parameter'),before=select.value;select.replaceChildren();
  for(const p of parameters.filter(p=>p.address===value('block')))select.add(new Option(p.path.join('.')+(p.kind==='boolean'?' (0/1)':''),p.key));
  if([...select.options].some(o=>o.value===before))select.value=before;
  if(reset||select.value!==before||!value('values')){const p=parameters.find(p=>p.key===select.value);get<HTMLTextAreaElement>('graph-sweep-values').value=JSON.stringify(p?.kind==='boolean'?[0,1]:p?.value?[p.value*.5,p.value,p.value*1.5]:[-1,0,1]);}
 }
 function refresh(){
  const model=options.model(),key=JSON.stringify(model);if(key!==modelKey){
   importGeneration++;parameters=sweepParameters(model);const blocks=get<HTMLSelectElement>('graph-sweep-block'),before=blocks.value;blocks.replaceChildren();
   for(const address of [...new Set(parameters.map(p=>p.address))]){const p=parameters.find(p=>p.address===address)!;blocks.add(new Option(`${p.label} (${address})`,address));}
   if([...blocks.options].some(o=>o.value===before))blocks.value=before;parameterOptions();
   const scope=get<HTMLSelectElement>('graph-sweep-scope'),selected=scope.value;scope.replaceChildren();for(const s of sweepScopes(model))scope.add(new Option(`${s.label} (${s.id}) [幅${s.width}]`,s.id));if([...scope.options].some(o=>o.value===selected))scope.value=selected;
   const old=modelKey?JSON.parse(modelKey) as GraphModel:null;if(!old||old.dt!==model.dt||old.duration!==model.duration){get<HTMLInputElement>('graph-sweep-start').value='0';get<HTMLInputElement>('graph-sweep-end').value=String(Math.round(model.duration/model.dt)*model.dt);}
   modelKey=key;
  }sync();
 }
 const numeric=(key:string)=>{if(!value(key).trim())throw new Error('評価条件を入力してください');return Number(value(key));};
 function readPlan(){return validateSweepPlan({format:'blocksim.sweep',version:1,model:options.model(),parameter:value('parameter'),values:JSON.parse(value('values')),metric:{scope:value('scope'),component:numeric('component'),reference:numeric('reference'),start:numeric('start'),end:numeric('end')}},options.normalize);}
 function renderRows(){get('graph-sweep-caption').textContent=plan?`実行時の設定: ${plan.parameter} · ${plan.metric.scope}[${plan.metric.component}] · 目標 ${plan.metric.reference} · 評価窓 ${plan.metric.start}〜${plan.metric.end} s · 記録 dt=${plan.model.dt} s。モデル編集後もこの設定の結果を保持します。`:'';const table=get('graph-sweep-rows');table.replaceChildren();rows.forEach((row,index)=>{
  const tr=document.createElement('tr');tr.dataset.index=String(index);tr.dataset.status=row.status;
  const cell=(text:string,key?:string)=>{const td=document.createElement('td');td.textContent=text;if(key)td.dataset.metric=key;tr.append(td);};
  cell(String(row.value));cell(({pending:'待機',ok:'完了',error:'エラー',cancelled:'未実行（中止）'} as const)[row.status]+(row.error?': '+row.error:''));
  for(const key of ['final','min','max','iae','ise'] as const)cell(row.metrics?.[key].toPrecision(6)??'—',key);cell(row.statistics?String(row.statistics.rate_evaluations):'—','rates');
  const actions=document.createElement('td');if(row.status==='ok'&&row.model&&row.result){for(const [text,callback] of [['基準にする',options.reference],['設定を適用',options.apply]] as const){const button=document.createElement('button');button.type='button';button.textContent=text;button.addEventListener('click',()=>{if(worker||options.busy())return;try{callback(structuredClone(row.model!),structuredClone(row.result!));}catch(error){state(String(error),true);}});actions.append(button);}}tr.append(actions);table.append(tr);
 });sync();}
 function finish(){generation++;worker?.terminate();worker=null;renderRows();options.onBusy();}
 function cancel(){importGeneration++;if(!worker)return;rows=rows.map(row=>row.status==='pending'?{value:row.value,status:'cancelled'}:row);finish();state('掃引を中止しました。完了したケースは保持しています。');}
 get('graph-sweep-block').addEventListener('change',()=>parameterOptions(true));get('graph-sweep-parameter').addEventListener('change',()=>parameterOptions(true));
 get('graph-sweep-cancel').addEventListener('click',cancel);
 get('graph-sweep-form').addEventListener('submit',event=>{
  event.preventDefault();if(worker||options.busy())return;
  try{
   const candidate=readPlan();importGeneration++;plan=candidate;rows=plan.values.map(value=>({value,status:'pending'}));const id=++generation;
   worker=new Worker(new URL('./graph-sweep-worker.ts',import.meta.url),{type:'module'});renderRows();options.onBusy();state(`掃引を計算中 · 0 / ${rows.length}`);
   const failed=(error:string)=>{rows=rows.map(row=>row.status==='pending'?{value:row.value,status:'error',error:'未実行: '+error}:row);finish();state(error,true);};
   worker.onmessage=(message:MessageEvent<{id:number,index?:number,row?:SweepRow,done?:boolean,error?:string}>)=>{
    if(id!==generation||message.data.id!==id)return;const data=message.data;
    if(data.error){failed(data.error);return;}if(data.index!==undefined&&data.row){rows[data.index]=data.row;renderRows();state(`掃引を計算中 · ${rows.filter(r=>r.status!=='pending').length} / ${rows.length}`);}
    if(data.done){finish();state(`掃引完了 · ${rows.filter(r=>r.status==='ok').length} 成功 / ${rows.filter(r=>r.status==='error').length} エラー`);}
   };
   worker.onerror=event=>{if(id===generation)failed('掃引ワーカーの起動に失敗: '+event.message);};worker.postMessage({id,plan});
  }catch(error){if(worker){rows=rows.map(r=>r.status==='pending'?{value:r.value,status:'error',error:'未実行: '+String(error)}:r);finish();}state(String(error),true);}
 });
 get('graph-sweep-export').addEventListener('click',()=>{try{options.download(encodeSweepPlan(readPlan()),'blocksim-sweep.json','application/json');}catch(error){state(String(error),true);}});
 get('graph-sweep-csv').addEventListener('click',()=>{if(plan&&!worker)options.download(sweepCsv(plan,rows),'blocksim-sweep.csv','text/csv');});
 get<HTMLInputElement>('graph-sweep-import').addEventListener('change',async()=>{
  const input=get<HTMLInputElement>('graph-sweep-import'),file=input.files?.[0];if(!file||worker||options.busy())return;const id=++importGeneration;
  try{if(file.size>1_000_000)throw new Error('掃引ファイルは1 MB以下にしてください');const text=await file.text();if(id!==importGeneration)throw new Error('読み込み中のモデル変更または別の操作のため中止しました');
   const imported=decodeSweepPlan(text,options.normalize);
   if(!options.replace(imported.model)){state('掃引設定の読み込みを中止しました');return;}refresh();
   get<HTMLSelectElement>('graph-sweep-block').value=parameters.find(p=>p.key===imported.parameter)!.address;parameterOptions();get<HTMLSelectElement>('graph-sweep-parameter').value=imported.parameter;
   get<HTMLTextAreaElement>('graph-sweep-values').value=JSON.stringify(imported.values);for(const [key,v]of Object.entries(imported.metric))get<HTMLInputElement>('graph-sweep-'+key).value=String(v);
   state('掃引設定を読み込みました。実行は「掃引を実行」から行ってください。');
  }catch(error){state(String(error),true);}finally{input.value='';}
 });
 refresh();return {refresh,sync,cancel,running:()=>!!worker};
}
