import {generateSampleTable,type InputShape} from './sampled-input-generator';
import {encodeInputGenerator,decodeInputGenerator,INPUT_GENERATOR_FILE_LIMIT} from './sampled-input-generator-file';
export function mountInputGenerator(host:HTMLElement,options:{values:HTMLTextAreaElement;width:HTMLInputElement;busy:()=>boolean;ioEpoch:()=>number;download:(text:string,name:string,type:string)=>void}) {
 host.id='graph-input-generator';host.innerHTML=`<details><summary>波形から入力表を生成</summary><fieldset id="graph-input-generator-fields"><label>生成する波形<select id="graph-input-generator-shape"><option value="constant">定数</option><option value="step">ステップ</option><option value="ramp">ランプ</option><option value="sine">正弦波</option></select></label><label>生成行数<input id="graph-input-generator-rows" type="text" inputmode="numeric" value="11"></label><label>基準値（数値または成分順JSON）<input id="graph-input-generator-base" type="text" value="0"></label><label id="graph-input-generator-delta-label">変化量（数値または成分順JSON）<input id="graph-input-generator-delta" type="text" value="1"></label><label id="graph-input-generator-step-label">切り替え行（0から）<input id="graph-input-generator-step" type="text" inputmode="numeric" value="5"></label><label id="graph-input-generator-cycles-label">表内の正弦波周期数<input id="graph-input-generator-cycles" type="text" inputmode="numeric" value="1"></label><label id="graph-input-generator-phase-label">初期位相（rad / 数値または成分順JSON）<input id="graph-input-generator-phase" type="text" value="0"></label><button id="graph-input-generator-run" type="button">入力表を生成して置換</button><div class="buttons"><button id="graph-input-generator-export" type="button">生成条件JSONを保存</button><label class="file-button">生成条件JSONを読み込む<input id="graph-input-generator-import" type="file" accept=".json,application/json"></label></div></fieldset><p class="hint">数値は全成分に共通、配列は信号幅と同じ長さ。ステップは切り替え行から基準値＋変化量、ランプは最終行までに変化量を加算、正弦波は変化量が振幅、初期位相は−2π〜2π radです。生成条件JSONは100 KBまで。読み込みは条件だけを復元し、信号幅は現在の設定欄と同じ値が必要です。周期・繰り返しはファイルに含みません。生成は設定欄の表を置換します。周期・繰り返しは保持し、プレビューで確認後にブロック設定を適用してください。</p></details><p id="graph-input-generator-state" role="status" aria-live="polite"></p>`;
 const get=<T extends HTMLElement>(id:string)=>host.querySelector<T>('#graph-input-generator-'+id)!;
 const message=(text:string,error=false)=>{get('state').textContent=text;get('state').classList.toggle('error',error);};
 let revision=0,readRequest=0;
 const file=get<HTMLInputElement>('import'),form=host.closest('form')!;
 const changed=(event:Event)=>{if(event.target!==file)revision++;};form.addEventListener('input',changed);form.addEventListener('change',changed);
 function sync(){const shape=get<HTMLSelectElement>('shape').value;for(const [field,visible] of [['delta',shape!=='constant'],['step',shape==='step'],['cycles',shape==='sine'],['phase',shape==='sine']] as const){get(field+'-label').hidden=!visible;get<HTMLInputElement>(field).disabled=!visible;}get<HTMLFieldSetElement>('fields').disabled=options.busy();}
 get('shape').addEventListener('change',sync);
 host.addEventListener('keydown',event=>{if(event.key==='Enter'&&event.target instanceof HTMLInputElement)event.preventDefault();});
 function read(){
  const shape=get<HTMLSelectElement>('shape').value as InputShape;
  const number=(id:string)=>{const value=get<HTMLInputElement>(id).value;return /^[+-]?\d+$/.test(value.trim())?Number(value):NaN;};
  const vector=(id:string)=>{const text=get<HTMLInputElement>(id).value.trim();return /^[+-]?(?:\d+(?:\.\d*)?|\.\d+)(?:[eE][+-]?\d+)?$/.test(text)?Number(text):JSON.parse(text);};
  return {shape,width:Number(options.width.value),rows:number('rows'),base:vector('base'),delta:shape==='constant'?undefined:vector('delta'),step:number('step'),cycles:number('cycles'),phase:shape==='sine'?vector('phase'):undefined};
 }
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
   get<HTMLInputElement>('step').value=String(request.step??5);get<HTMLInputElement>('cycles').value=String(request.cycles??1);get<HTMLInputElement>('phase').value=JSON.stringify(request.phase??0);sync();get('shape').dispatchEvent(new Event('input',{bubbles:true}));
   message('生成条件JSONを読み込みました。入力表は保持しています。「入力表を生成して置換」で反映してください。');
  }catch(error){if(active()&&version===revision&&epoch===options.ioEpoch())message(String(error),true);}
  finally{if(id===readRequest)file.value='';}
 });
 sync();return {sync};
}
