import {generateSampleTable,type InputShape} from './sampled-input-generator';
export function mountInputGenerator(host:HTMLElement,options:{values:HTMLTextAreaElement;width:HTMLInputElement;busy:()=>boolean}) {
 host.id='graph-input-generator';host.innerHTML=`<details><summary>波形から入力表を生成</summary><fieldset id="graph-input-generator-fields"><label>生成する波形<select id="graph-input-generator-shape"><option value="constant">定数</option><option value="step">ステップ</option><option value="ramp">ランプ</option><option value="sine">正弦波</option></select></label><label>生成行数<input id="graph-input-generator-rows" type="text" inputmode="numeric" value="11"></label><label>基準値（数値または成分順JSON）<input id="graph-input-generator-base" type="text" value="0"></label><label id="graph-input-generator-delta-label">変化量（数値または成分順JSON）<input id="graph-input-generator-delta" type="text" value="1"></label><label id="graph-input-generator-step-label">切り替え行（0から）<input id="graph-input-generator-step" type="text" inputmode="numeric" value="5"></label><label id="graph-input-generator-cycles-label">表内の正弦波周期数<input id="graph-input-generator-cycles" type="text" inputmode="numeric" value="1"></label><button id="graph-input-generator-run" type="button">入力表を生成して置換</button></fieldset><p class="hint">数値は全成分に共通、配列は信号幅と同じ長さ。ステップは切り替え行から基準値＋変化量、ランプは最終行までに変化量を加算、正弦波は変化量が振幅です。生成は設定欄の表を置換します。周期・繰り返しは保持し、プレビューで確認後にブロック設定を適用してください。</p></details><p id="graph-input-generator-state" role="status" aria-live="polite"></p>`;
 const get=<T extends HTMLElement>(id:string)=>host.querySelector<T>('#graph-input-generator-'+id)!;
 function sync(){const shape=get<HTMLSelectElement>('shape').value;for(const [field,visible] of [['delta',shape!=='constant'],['step',shape==='step'],['cycles',shape==='sine']] as const){get(field+'-label').hidden=!visible;get<HTMLInputElement>(field).disabled=!visible;}get<HTMLFieldSetElement>('fields').disabled=options.busy();}
 get('shape').addEventListener('change',sync);
 host.addEventListener('keydown',event=>{if(event.key==='Enter'&&event.target instanceof HTMLInputElement)event.preventDefault();});
 get('run').addEventListener('click',()=>{try{
  if(options.busy())throw new Error('実行・解析の完了を待ってください');
  const shape=get<HTMLSelectElement>('shape').value as InputShape;
  const number=(id:string)=>{const value=get<HTMLInputElement>(id).value;return /^[+-]?\d+$/.test(value.trim())?Number(value):NaN;};
  const vector=(id:string)=>{const text=get<HTMLInputElement>(id).value.trim();return /^[+-]?(?:\d+(?:\.\d*)?|\.\d+)(?:[eE][+-]?\d+)?$/.test(text)?Number(text):JSON.parse(text);};
  const rows=generateSampleTable({shape,width:Number(options.width.value),rows:number('rows'),base:vector('base'),delta:shape==='constant'?undefined:vector('delta'),step:number('step'),cycles:number('cycles')});
  options.values.value=JSON.stringify(rows);options.values.dispatchEvent(new Event('input',{bubbles:true}));
  get('state').textContent=`入力表を生成しました · ${rows.length}行 × ${rows[0].length}列。プレビューで確認し、ブロック設定を適用してください。`;get('state').classList.remove('error');
 }catch(error){get('state').textContent=String(error);get('state').classList.add('error');}});
 sync();return {sync};
}
