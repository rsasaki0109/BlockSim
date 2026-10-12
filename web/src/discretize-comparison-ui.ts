import {decodeConversionPlan,conversionCsv,type ConversionPlan,type ConversionReport} from './discretize-comparison';
import {plotGraph} from './graph-chart';
export function mountConversionComparison(host:HTMLElement,options:{plan:()=>ConversionPlan;restore:(p:ConversionPlan)=>void;busy:()=>boolean;onBusy:()=>void;download:(text:string,name:string,type:string)=>void}) {
 host.innerHTML=`<h4>単位ステップ応答の比較</h4><p class="hint">選択した伝達関数を単独・初期状態0・入力1で比較します。元モデルの接続や入力は使いません。連続はRK45、離散は保持出力。TsごとのΔ=離散−連続を表示します。ZOHの一致はこの保持入力のサンプル時刻についての数値検証です。</p><label>比較点数（初回を含む）<input id="conversion-samples" type="number" min="2" max="2001" step="1" value="101"></label><div class="buttons"><button id="conversion-run" type="button">応答を比較</button><button id="conversion-cancel" type="button" disabled>比較を中止</button><button id="conversion-plan" type="button">比較設定 JSON</button><label class="file-button">比較設定を読み込む<input id="conversion-import" type="file" accept=".json,application/json"></label><button id="conversion-csv" type="button" disabled>比較 CSV</button><button id="conversion-report" type="button" disabled>比較結果 JSON</button></div><p id="conversion-state" role="status" aria-live="polite"></p><p id="conversion-metrics"></p><p id="conversion-caption" class="hint"></p><div id="conversion-plot"></div>`;
 const get=<T extends HTMLElement>(id:string)=>host.querySelector<T>('#'+id)!;
 let worker:Worker|null=null,id=0,report:ConversionReport|null=null;
 const state=(message:string,error=false)=>{get('conversion-state').textContent=message;get('conversion-state').classList.toggle('error',error);};
 const read=()=>decodeConversionPlan(JSON.stringify({...options.plan(),samples:Number(get<HTMLInputElement>('conversion-samples').value)}));
 const sync=()=>{const busy=!!worker||options.busy();for(const key of ['conversion-run','conversion-plan'])get<HTMLButtonElement>(key).disabled=busy;get<HTMLInputElement>('conversion-import').disabled=busy;get<HTMLInputElement>('conversion-samples').disabled=busy;get<HTMLButtonElement>('conversion-cancel').disabled=!worker;for(const key of ['conversion-csv','conversion-report'])get<HTMLButtonElement>(key).disabled=busy||!report;};
 const cancel=()=>{id++;worker?.terminate();worker=null;sync();options.onBusy();};
 const clear=()=>{cancel();report=null;get('conversion-metrics').textContent='';get('conversion-caption').textContent='';get('conversion-plot').replaceChildren();sync();};
 const guard=(action:()=>void)=>{try{if(worker||options.busy())throw new Error('実行・解析の完了を待ってください');action();}catch(error){state(String(error),true);}};
 get('conversion-run').addEventListener('click',()=>guard(()=>{
  const plan=read(),request=++id;
  worker=new Worker(new URL('./discretize-comparison-worker.ts',import.meta.url),{type:'module'});sync();options.onBusy();state('単位ステップ応答を比較中…');
  worker.onmessage=(event:MessageEvent<{id:number;result?:ConversionReport;error?:string}>)=>{if(request!==id||event.data.id!==request)return;
   cancel();if(event.data.error){state(event.data.error,true);return;}if(!event.data.result){state('比較結果がありません',true);return;}
   report=event.data.result;const r=report,m=r.metrics;
   get('conversion-metrics').textContent=`最大 |Δ|=${m.maxAbs.toPrecision(7)} · RMS=${m.rms.toPrecision(7)} · 終点 Δ=${m.final.toPrecision(7)}`;
   get('conversion-caption').textContent=`実行時: ${r.plan.method.toUpperCase()} · Ts=${r.plan.request.sample_time}s · ${r.times.length}点 · 分子=${JSON.stringify(r.plan.request.numerator)} · 分母=${JSON.stringify(r.plan.request.denominator)} · プリワープ=${r.plan.request.prewarp_omega??'なし'} · 実線=離散 / 破線=連続。表示線はサンプル点を結びます。`;
   const graph=(values:number[])=>({times:r.times,solver:'conversion',traces:[{id:'response',label:'Unit step',values:values.map(v=>[v])}]});
   plotGraph(get('conversion-plot'),graph(r.discrete),new Set(['response:0']),graph(r.continuous));state(`比較完了 · ${r.times.length} サンプル`);sync();
  };
  worker.onerror=event=>{if(request!==id)return;cancel();state(`比較ワーカーの起動に失敗: ${event.message}`,true);};worker.postMessage({id:request,json:JSON.stringify(plan)});
 }));
 get('conversion-cancel').addEventListener('click',()=>{cancel();state('比較を中止しました');});
 get('conversion-plan').addEventListener('click',()=>guard(()=>options.download(JSON.stringify(read(),null,2),'blocksim-conversion-plan.json','application/json')));
 get('conversion-import').addEventListener('change',async()=>{
  const input=get<HTMLInputElement>('conversion-import'),file=input.files?.[0];let request=id;
  try{if(!file)return;if(worker||options.busy())throw new Error('実行・解析の完了を待ってください');request=++id;if(file.size>100_000)throw new Error('比較設定は100 KBまでです');const p=decodeConversionPlan(await file.text());if(request!==id||!host.isConnected)return;if(worker||options.busy())throw new Error('実行・解析の完了を待ってください');options.restore(p);clear();get<HTMLInputElement>('conversion-samples').value=String(p.samples);state('比較設定を読み込みました');}catch(error){if(request===id&&host.isConnected)state(String(error),true);}finally{input.value='';}
 });
 get('conversion-csv').addEventListener('click',()=>guard(()=>{if(report)options.download(conversionCsv(report),'blocksim-conversion.csv','text/csv');}));
 get('conversion-report').addEventListener('click',()=>guard(()=>{if(report)options.download(JSON.stringify({format:'blocksim.conversion-result',version:1,...report},null,2),'blocksim-conversion-result.json','application/json');}));
 get('conversion-samples').addEventListener('input',()=>state('点数を変更しました。表示は前回の実行条件です。'));
 sync();return {running:()=>!!worker,sync,cancel,clear};
}
