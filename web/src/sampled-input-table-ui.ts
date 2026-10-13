import {validateSampleTable} from './sampled-input-csv';
const PAGE_ROWS=20;
const decimal=/^[+-]?(?:\d+(?:\.\d*)?|\.\d+)(?:[eE][+-]?\d+)?$/;
function cellValue(text:string):number|null {const s=text.trim();return decimal.test(s)&&Number.isFinite(Number(s))?Number(s):null;}
export function mountSampleTableEditor(host:HTMLElement,options:{values:HTMLTextAreaElement;width:HTMLInputElement;period:HTMLInputElement;busy:()=>boolean}) {
 host.id='graph-input-table';host.innerHTML=`<h3>入力表を編集</h3><fieldset id="graph-input-table-fields"><legend>行=サンプル、列=成分</legend><div class="buttons"><button id="graph-input-table-add" type="button">末尾に行を追加</button><button id="graph-input-table-prev" type="button">前の20行</button><button id="graph-input-table-next" type="button">次の20行</button><label>ページ<input id="graph-input-table-page" type="number" min="1" step="1" value="1"></label><button id="graph-input-table-go" type="button">ページへ移動</button></div><p id="graph-input-table-state" role="status" aria-live="polite"></p><div class="table-wrap"><table><thead></thead><tbody></tbody></table></div></fieldset><p class="hint">サンプル番号は0から。時刻は設定欄の周期×番号で、繰り返す前の1周期を表示します。セル・行の編集は設定欄だけを更新します。モデルへはブロック設定を適用してください。空欄や不正な数値は適用・CSV保存できません。</p>`;
 const get=<T extends HTMLElement>(id:string)=>host.querySelector<T>('#'+id)!;
 const head=host.querySelector('thead')!,body=host.querySelector('tbody')!,state=get('graph-input-table-state');
 let rows:string[][]=[],page=0,writing=false,validShape=false;
 function message(){
  let invalid: [number,number]|null=null;for(let k=0;k<rows.length&&!invalid;k++)for(let j=0;j<rows[k].length;j++)if(cellValue(rows[k][j])===null){invalid=[k,j];break;}
  state.classList.toggle('error',!!invalid);
  state.textContent=invalid?`サンプル ${invalid[0]} 成分 ${invalid[1]}: 有限の数値を入力してください。適用・CSV保存はできません。`:`${rows.length}行 × ${rows[0]?.length??0}列 · ${page+1} / ${Math.ceil(rows.length/PAGE_ROWS)}ページ。編集後にブロック設定を適用してください。`;
 }
 function write(){
  // Invalid tokens become explicit nulls, so model apply and CSV export cannot
  // accidentally use the last valid value. Keep their text for correction here.
  options.values.value=JSON.stringify(rows.map(row=>row.map(cellValue)));
  writing=true;try{options.values.dispatchEvent(new Event('input',{bubbles:true}));}finally{writing=false;}
  message();
 }
 function sync(){
  get<HTMLFieldSetElement>('graph-input-table-fields').disabled=options.busy();
  get<HTMLButtonElement>('graph-input-table-go').disabled=!validShape;
  get<HTMLInputElement>('graph-input-table-page').disabled=!validShape;
  get<HTMLInputElement>('graph-input-table-page').max=String(Math.max(1,Math.ceil(rows.length/PAGE_ROWS)));
  get<HTMLButtonElement>('graph-input-table-add').disabled=!validShape||rows.length>=2001;
  get<HTMLButtonElement>('graph-input-table-prev').disabled=!validShape||page===0;
  get<HTMLButtonElement>('graph-input-table-next').disabled=!validShape||(page+1)*PAGE_ROWS>=rows.length;
 }
 function rowAction(action:()=>void){if(options.busy())return;action();page=Math.max(0,Math.min(page,Math.ceil(rows.length/PAGE_ROWS)-1));write();render();}
 function render(){
  head.replaceChildren();body.replaceChildren();
  if(!validShape){sync();return;}
  get<HTMLInputElement>('graph-input-table-page').value=String(page+1);
  const h=document.createElement('tr');for(const label of ['番号','時刻 (s)',...rows[0].map((_,j)=>`成分 ${j}`),'行の操作']){const th=document.createElement('th');th.scope='col';th.textContent=label;h.append(th);}head.append(h);
  const ts=Number(options.period.value),periodValid=options.period.value.trim()!==''&&Number.isFinite(ts)&&ts>0;
  for(let k=page*PAGE_ROWS;k<Math.min(rows.length,(page+1)*PAGE_ROWS);k++){
   const tr=document.createElement('tr');tr.dataset.sample=String(k);
   const index=document.createElement('th');index.scope='row';index.textContent=String(k);tr.append(index);
   const time=document.createElement('td');const t=k*ts;time.textContent=periodValid&&Number.isFinite(t)?String(Number(t.toPrecision(12))):'—';tr.append(time);
   rows[k].forEach((v,j)=>{const td=document.createElement('td'),input=document.createElement('input');input.type='text';input.inputMode='decimal';input.value=v;input.dataset.sample=String(k);input.dataset.component=String(j);input.setAttribute('aria-label',`サンプル ${k} 成分 ${j}`);input.setAttribute('aria-invalid',String(cellValue(v)===null));
    input.addEventListener('input',()=>{if(options.busy())return;rows[k][j]=input.value;input.setAttribute('aria-invalid',String(cellValue(input.value)===null));write();});input.addEventListener('keydown',e=>{if(e.key==='Enter')e.preventDefault();});td.append(input);tr.append(td);});
   const actions=document.createElement('td');
   for(const [label,disabled,action] of [ ['複製',rows.length>=2001,()=>{rows.splice(k+1,0,[...rows[k]]);}], ['削除',rows.length===1,()=>{rows.splice(k,1);}] ] as const){const b=document.createElement('button');b.type='button';b.textContent=label;b.setAttribute('aria-label',`サンプル ${k} を${label}`);b.disabled=disabled;b.addEventListener('click',()=>rowAction(action));actions.append(b);}tr.append(actions);body.append(tr);
  }
  message();sync();
 }
 function refresh(){
  if(writing)return;
  try{const values=validateSampleTable(JSON.parse(options.values.value),Number(options.width.value));rows=values.map(row=>row.map(String));validShape=true;page=Math.max(0,Math.min(page,Math.ceil(rows.length/PAGE_ROWS)-1));render();}
  catch(error){validShape=false;head.replaceChildren();body.replaceChildren();state.textContent=`${String(error)}。JSONと信号幅を修正してください。`;state.classList.add('error');sync();}
 }
 get('graph-input-table-add').addEventListener('click',()=>{if(validShape&&rows.length<2001)rowAction(()=>{rows.push(Array(rows[0].length).fill('0'));page=Math.floor((rows.length-1)/PAGE_ROWS);});});
 get('graph-input-table-go').addEventListener('click',()=>{if(options.busy()||!validShape)return;const n=Number(get<HTMLInputElement>('graph-input-table-page').value);if(!Number.isInteger(n)||n<1||n>Math.ceil(rows.length/PAGE_ROWS)){state.textContent='ページ番号は範囲内の整数です';state.classList.add('error');return;}page=n-1;render();});
 get('graph-input-table-prev').addEventListener('click',()=>{if(!options.busy()&&page>0){page--;render();}});
 get('graph-input-table-next').addEventListener('click',()=>{if(!options.busy()&&(page+1)*PAGE_ROWS<rows.length){page++;render();}});
 options.values.addEventListener('input',refresh);options.width.addEventListener('input',refresh);options.period.addEventListener('input',render);
 refresh();return {refresh,sync};
}
