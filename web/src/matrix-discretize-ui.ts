import {discretize_matrix,normalize_graph,diagnose_graph} from './wasm/blocksim';
import {graphContext,type GraphModel,type Params} from './graph-model';
import {matrixRequest,matrixModel,type MatrixRequest,type MatrixResult} from './matrix-discretize';
export function mountMatrixDiscretize(host:HTMLElement,options:{model:()=>GraphModel;address:string;busy:()=>boolean;apply:(params:Params,message:string)=>void}) {
 const section=document.createElement('section');section.id='graph-matrix-discretize';
 section.innerHTML=`<h3>行列の連続 → 離散</h3><p class="hint">適用済みの行列を変換します。上の編集は先に適用してください。ZOHは元の状態座標と初期値を引き継ぎ、保持入力のサンプル時刻の応答を合わせます。Tustinは初期状態0のみで、状態座標と直接項が変わります。接続・記録時計・解析領域は保持します。</p><form id="graph-matrix-discretize-form"><fieldset id="graph-matrix-discretize-fields"><label>離散化方式<select id="graph-matrix-discretize-method"><option value="zoh">ゼロ次ホールド（ZOH）</option><option value="tustin">Tustin法</option></select></label><label>変換サンプル周期 (s)<input id="graph-matrix-discretize-period" type="number" step="any" required></label><label>プリワープ角周波数 (rad/s)<input id="graph-matrix-discretize-prewarp" type="number" step="any" placeholder="なし" disabled></label><button id="graph-matrix-discretize-preview" type="submit">変換行列を確認</button><button id="graph-matrix-discretize-apply" type="button" disabled>離散行列へ変換</button></fieldset></form><pre id="graph-matrix-discretize-coefficients" class="hint" style="white-space:pre-wrap;overflow-wrap:anywhere"></pre><p id="graph-matrix-discretize-state" role="status"></p>`;host.append(section);
 const get=<T extends HTMLElement>(id:string)=>section.querySelector<T>('#graph-matrix-discretize-'+id)!;
 const method=get<HTMLSelectElement>('method'),period=get<HTMLInputElement>('period'),prewarp=get<HTMLInputElement>('prewarp'),apply=get<HTMLButtonElement>('apply'),state=get('state'),output=get('coefficients');period.value=String(options.model().dt);
 let preview:{result:MatrixResult;root:string;conditions:string}|null=null;
 const conditions=()=>JSON.stringify([method.value,period.value,prewarp.value]);
 const source=()=>{const path=options.address.split('/'),id=path.pop()!;const b=graphContext(options.model(),path).blocks.find(b=>b.id===id);if(!b)throw new Error('変換対象がありません');return b;};
 const clear=()=>{preview=null;apply.disabled=true;output.textContent='';state.textContent='条件を変更しました。行列を再確認してください。';state.classList.remove('error');};
 const fail=(e:unknown)=>{state.textContent=String(e);state.classList.add('error');};
 get<HTMLFormElement>('form').addEventListener('submit',event=>{event.preventDefault();try{
  if(options.busy())throw new Error('実行・解析の完了を待ってください');preview=null;apply.disabled=true;output.textContent='';
  const q=matrixRequest(source(),Number(period.value),method.value as MatrixRequest['method'],method.value==='tustin'&&prewarp.value.trim()?Number(prewarp.value):undefined);
  const data=JSON.parse(discretize_matrix(JSON.stringify(q))) as MatrixResult;matrixModel(options.model(),options.address,data,normalize_graph,diagnose_graph);
  preview={result:data,root:JSON.stringify(options.model()),conditions:conditions()};output.textContent=['a','b','c','d'].map(k=>`${k.toUpperCase()}d: ${JSON.stringify(data[k as 'a'|'b'|'c'|'d'])}`).join('\n')+`\n初期状態: ${JSON.stringify(data.initial)}\nTs=${q.sample_time}s · 残差=${data.residual}`;state.textContent='変換行列を確認しました。適用は1回のUndoで戻せます。';state.classList.remove('error');apply.disabled=false;
 }catch(e){fail(e);}});
 for(const input of [period,prewarp])input.addEventListener('input',clear);
 method.addEventListener('change',()=>{prewarp.disabled=method.value==='zoh';clear();});
 apply.addEventListener('click',()=>{try{
  if(options.busy())throw new Error('実行・解析の完了を待ってください');if(!preview||preview.root!==JSON.stringify(options.model())||preview.conditions!==conditions())throw new Error('モデルまたは条件が変わりました。再確認してください。');
  const next=matrixModel(options.model(),options.address,preview.result,normalize_graph,diagnose_graph),path=options.address.split('/'),id=path.pop()!;
  options.apply(graphContext(next,path).blocks.find(b=>b.id===id)!.params,`${method.value==='zoh'?'ZOH':'Tustin法'}で離散行列へ変換しました`);
 }catch(e){fail(e);}});
 get<HTMLFieldSetElement>('fields').disabled=options.busy();
}
