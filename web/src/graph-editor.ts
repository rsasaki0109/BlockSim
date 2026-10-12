import { normalize_graph, validate_graph } from './wasm/blocksim';
import { parseNumber } from './document';
import { connectPorts, mergeGraph, ungroupBlock, inputWidth, nodeHeight, portY, wirePath } from './graph-edit';
import {
  GraphHistory, graphContext, newBlock, groupBlocks, deleteBlocks, inputCount, outputCount, outputWidth, setClock,
  KIND_NAMES, GRAPH_STORAGE_KEY, decodeGraphDocument, encodeGraphDocument, resultCsv,
  type GraphModel, type GraphBlock, type GraphResult, type Kind,
} from './graph-model';
import { plotGraph, seriesFrom } from './graph-chart';
import continuous from '../../samples/graphs/continuous-pid.json';
import mixed from '../../samples/graphs/mixed-pid.json';
import vector from '../../samples/graphs/vector.json';
import subsystem from '../../samples/graphs/subsystem.json';

const SAMPLES: Record<string,GraphModel>={continuous:continuous as GraphModel,mixed:mixed as GraphModel,vector:vector as GraphModel,subsystem:subsystem as GraphModel};
function download(text:string,name:string,type:string){const url=URL.createObjectURL(new Blob([text],{type}));const a=document.createElement('a');a.href=url;a.download=name;a.click();URL.revokeObjectURL(url);}
function button(text:string,action:()=>void){const b=document.createElement('button');b.type='button';b.textContent=text;b.addEventListener('click',action);return b;}
export function mountEditor(host:HTMLElement) {
  host.innerHTML=`
    <section class="intro"><p class="eyebrow">MODEL WORKSPACE / ブロック図</p><h1>つなげて、動きを確かめる。</h1><p>連続・離散・ベクトルをひとつのモデルに。Scope で結果を記録します。</p></section>
    <div class="graph-toolbar panel">
      <label>ブロック種類<select id="block-kind"></select></label><button id="block-add">ブロックを追加</button>
      <button id="graph-undo">Undo</button><button id="graph-redo">Redo</button><button id="graph-delete">選択を削除</button><button id="graph-group">サブシステム化</button><button id="graph-ungroup">グループ解除</button><button id="graph-wire-cancel" disabled>配線をキャンセル</button><button id="graph-disconnect" disabled>選択した接続を削除</button>
      <button id="graph-run" class="primary">モデルを実行</button><button id="graph-cancel" disabled>実行を中止</button>
    </div>
    <div id="graph-status" role="status" aria-label="モデル実行状態" aria-live="polite"></div>
    <div class="graph-workspace"><section class="graph-center">
      <div class="graph-topline"><div id="graph-breadcrumb"></div><span id="graph-count" class="pill"></span></div>
      <p class="hint">ブロックはドラッグで移動。チェックで複数選択。出力ポート→入力ポートをクリックして配線。Escape で中止。線を選ぶと削除できます。右のフォームも使えます。</p>
      <div class="graph-viewport"><div id="graph-board" aria-label="ブロック図キャンバス"><svg id="graph-wires" width="1800" height="1000" aria-label="接続線"></svg><div id="graph-nodes"></div></div></div>
      <section class="panel graph-results"><h2>Scope の結果</h2><div id="graph-series" class="legend"></div><p class="hint">表示は最大16系列。CSV は全 Scope・全記録点を出力します。</p><div id="graph-plot"></div>
        <label>記録点カーソル<input id="graph-cursor" type="range" min="0" max="0" value="0" disabled></label><output id="graph-values" aria-live="polite"></output><button id="graph-csv" disabled>CSV を書き出す</button>
      </section>
    </section><aside>
      <section class="panel"><h2>ブロック設定</h2><div id="graph-inspector"><p class="hint">ブロックを選択してください</p></div></section>
      <section class="panel"><h2>接続</h2><form id="graph-connect"><label>接続元<select id="edge-from"></select></label><label>出力ポート<select id="edge-from-port"></select></label><label>接続先<select id="edge-to"></select></label><label>入力ポート<select id="edge-to-port"></select></label><button>接続する</button></form><div id="graph-edges"></div></section>
      <section class="panel"><h2>シミュレーション設定</h2><form id="graph-clock"><label>グラフ刻み幅 (s)<input id="graph-dt" type="number" step="any" required></label><label>グラフ終了時刻 (s)<input id="graph-duration" type="number" step="any" required></label><button>時間設定を適用</button></form><p class="hint">離散の周期は刻み幅の整数倍。内部モデルも同じ基準時計を使います。</p></section>
      <section class="panel"><h2>モデルとサンプル</h2><label>モデルサンプル<select id="graph-sample"><option value="continuous">連続 PID の復帰</option><option value="mixed">離散 PID + 連続プラント</option><option value="vector">ベクトル + Saturation</option><option value="subsystem">Sine + Subsystem</option></select></label>
        <label>読み込み方法<select id="graph-import-mode"><option value="replace">モデルを置き換える</option><option value="merge">既存モデルに追加</option></select></label><div class="buttons"><button id="graph-load-sample">サンプルを読み込む</button><button id="graph-empty">新しいモデル</button><button id="graph-save">モデルを保存</button><button id="graph-restore">モデルを復元</button><button id="graph-export">モデル JSON</button><label class="file-button">モデルを読み込む<input id="graph-import" type="file" accept=".json,application/json"></label></div><p class="hint">未接続の作成途中モデルも保存できます。追加読み込みでは既存の時計・終了時刻を保持し、同じ刻み幅のモデルを取り込めます。既存の PID Lab 設定は別に保持します。</p>
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
  const cancel=()=>{request++;worker?.terminate();worker=null;get<HTMLButtonElement>('graph-run').disabled=false;get<HTMLButtonElement>('graph-cancel').disabled=true;};
  const invalidate=()=>{cancel();pendingWire=null;previewPoint=null;selectedEdge=null;result=null;get('graph-plot').replaceChildren();get('graph-series').replaceChildren();get('graph-values').textContent='モデル変更後は再実行してください';get<HTMLInputElement>('graph-cursor').disabled=true;get<HTMLButtonElement>('graph-csv').disabled=true;};
  const current=()=>graphContext(history.graph,path);
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
    const labels:Record<string,string>={value:'値 (JSON 配列)',before:'変更前 (JSON 配列)',after:'変更後 (JSON 配列)',step_time:'変更時刻 (s)',gain:'ゲイン',tau:'時定数 (s)',initial:'初期値 (JSON 配列)',signs:'入力係数 (JSON 配列)',inputs:'入力数',indices:'抽出インデックス (JSON 配列)',sample_time:'サンプル周期 (s)',frequency:'周波数 (Hz)',phase:'位相 (rad)',amplitude:'振幅',offset:'オフセット',port:'境界ポート番号',mode:'PID 更新方式'};
    for(const [key,value]of Object.entries(b.params)) {
      if(['kind','model','config','limits'].includes(key))continue;
      field(`p.${key}`,labels[key]||key,value,key==='mode'?'mode':undefined);
    }
    if(b.params.config){const cfg=b.params.config as Record<string,unknown>;for(const [key,value]of Object.entries(cfg))if(key!=='limits')field(`c.${key}`,key==='anti_windup'?'anti-windup':key,value);
      const limits=cfg.limits as Record<string,unknown>;field('c.lower','PID 出力下限',limits.lower);field('c.upper','PID 出力上限',limits.upper);}
    if(b.params.limits){const limits=b.params.limits as Record<string,unknown>;field('p.lower','出力下限',limits.lower);field('p.upper','出力上限',limits.upper);}
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
      for(const [key]of Object.entries(target.params))if(!['kind','model','config','limits'].includes(key))target.params[key]=read(`p.${key}`);
      if(target.params.config){const cfg=target.params.config as Record<string,unknown>;for(const key of Object.keys(cfg))if(key!=='limits')cfg[key]=read(`c.${key}`);cfg.limits={lower:read('c.lower'),upper:read('c.upper')};}
      if(target.params.limits)target.params.limits={lower:read('p.lower'),upper:read('p.upper')};
    },'ブロック設定を適用しました');});
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
    get<HTMLButtonElement>('graph-disconnect').disabled=selectedEdge===null;
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
      const node=document.createElement('div');node.className='graph-node';node.classList.toggle('selected',selected.has(b.id));node.dataset.block=b.id;node.style.left=b.position[0]+'px';node.style.top=b.position[1]+'px';node.style.minHeight=nodeHeight(b)+'px';node.tabIndex=0;node.setAttribute('role','button');node.setAttribute('aria-label',`ブロック ${b.id}`);
      const check=document.createElement('input');check.type='checkbox';check.checked=selected.has(b.id);check.setAttribute('aria-label',`選択 ${b.id}`);
      check.addEventListener('pointerdown',e=>e.stopPropagation());check.addEventListener('keydown',e=>{if(e.key==='Enter'||e.key===' ')e.stopPropagation();});check.addEventListener('click',e=>e.stopPropagation());check.addEventListener('change',()=>{selectedEdge=null;if(check.checked)selected.add(b.id);else selected.delete(b.id);render();});
      const title=document.createElement('strong');title.textContent=b.label;const small=document.createElement('small');small.textContent=`${KIND_NAMES[b.params.kind]} · 幅${b.width}`;
      node.append(check,title,small);
      for(const direction of ['in','out'] as const)for(let port=0;port<(direction==='in'?inputCount(b):outputCount(b));port++){
        const endpoint=document.createElement('button');endpoint.type='button';endpoint.className=`node-port port-${direction}`;endpoint.style.top=(portY(port)-11)+'px';endpoint.textContent=String(port);
        const label=`${direction==='in'?'入力':'出力'} ${b.id}:${port}`;endpoint.setAttribute('aria-label',label);endpoint.title=`${label} · 幅${direction==='in'?inputWidth(b,port):outputWidth(b,port)}`;
        endpoint.id=`port-${direction}-${b.id}-${port}`;endpoint.dataset.port=String(port);endpoint.dataset.direction=direction;
        if(direction==='out'){const active=pendingWire?.from===b.id&&pendingWire.from_port===port;endpoint.classList.toggle('port-pending',active);endpoint.setAttribute('aria-pressed',String(active));}
        else endpoint.classList.toggle('port-connected',current().edges.some(e=>e.to===b.id&&e.to_port===port));
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
        node.setPointerCapture(event.pointerId);const start=[event.clientX,event.clientY];let position:[number,number]=[...b.position];let moved=false;
        const move=(e:PointerEvent)=>{if(Math.abs(e.clientX-start[0])+Math.abs(e.clientY-start[1])>3)moved=true;if(!moved)return;
          position=[Math.max(0,Math.min(10000,b.position[0]+e.clientX-start[0])),Math.max(0,Math.min(10000,b.position[1]+e.clientY-start[1]))];node.style.left=position[0]+'px';node.style.top=position[1]+'px';};
        const finish=()=>{node.removeEventListener('pointermove',move);node.removeEventListener('pointerup',finish);node.removeEventListener('pointercancel',abort);if(moved)edit(g=>{g.blocks.find(n=>n.id===b.id)!.position=position;},'ブロックを移動しました');else syncSelection();};
        const abort=()=>{node.removeEventListener('pointermove',move);node.removeEventListener('pointerup',finish);node.removeEventListener('pointercancel',abort);render();};
        node.addEventListener('pointermove',move);node.addEventListener('pointerup',finish);node.addEventListener('pointercancel',abort);
      });nodes.append(node);
    }paintPreview();
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
    renderPorts();renderNodes();renderInspector();
    const edges=get('graph-edges');edges.replaceChildren();current().edges.forEach((e,i)=>{
      const row=document.createElement('div');row.className='edge-row';const text=document.createElement('span');text.textContent=`${e.from}:${e.from_port} → ${e.to}:${e.to_port}`;
      const remove=button('×',()=>edit(g=>{g.edges.splice(i,1);},'接続を削除しました'));remove.setAttribute('aria-label',`接続を削除 ${e.from} ${e.to} ${e.to_port}`);row.append(text,remove);edges.append(row);
    });
    get<HTMLInputElement>('graph-dt').value=String(history.graph.dt);get<HTMLInputElement>('graph-duration').value=String(history.graph.duration);
  }
  function cursor(){if(!result)return;const index=Number(get<HTMLInputElement>('graph-cursor').value);get('graph-values').textContent=`t=${result.times[index].toFixed(4)}s · `+seriesFrom(result).filter(s=>visible.has(s.key)).map(s=>`${s.label}=${s.values[index].toPrecision(6)}`).join(' / ');}
  function showResult(data:GraphResult){
    result=data;visible=new Set(seriesFrom(data).slice(0,16).map(s=>s.key));const area=get('graph-series');area.replaceChildren();
    for(const series of seriesFrom(data)){const label=document.createElement('label');label.className='check';label.style.color=series.color;const check=document.createElement('input');check.type='checkbox';check.checked=visible.has(series.key);check.setAttribute('aria-label',`表示 ${series.label}`);
      check.addEventListener('change',()=>{if(check.checked&&visible.size>=16){check.checked=false;say('表示は最大16系列です',true);return;}if(check.checked)visible.add(series.key);else visible.delete(series.key);plotGraph(get('graph-plot'),data,visible);cursor();});label.append(check,document.createTextNode(series.label));area.append(label);}
    const slider=get<HTMLInputElement>('graph-cursor');slider.disabled=false;slider.max=String(data.times.length-1);slider.value=String(data.times.length-1);get<HTMLButtonElement>('graph-csv').disabled=false;
    plotGraph(get('graph-plot'),data,visible);cursor();say(`完了 · ${data.times.length} 点 · ${data.traces.length} Scope · Rust / WASM`);
  }
  function run(){guard(()=>{
    const json=validate_graph(JSON.stringify(history.graph));cancel();const id=++request;
    get<HTMLButtonElement>('graph-run').disabled=true;get<HTMLButtonElement>('graph-cancel').disabled=false;say('モデルを計算中…');
    worker=new Worker(new URL('./graph-worker.ts',import.meta.url),{type:'module'});
    worker.onmessage=(event:MessageEvent<{id:number,result?:GraphResult,error?:string}>)=>{if(event.data.id!==request)return;cancel();if(event.data.error)say(event.data.error,true);else if(event.data.result)showResult(event.data.result);};
    worker.onerror=event=>{if(id!==request)return;cancel();say(`計算ワーカーの起動に失敗: ${event.message}`,true);};worker.postMessage({id,json});
  });}
  get('block-add').addEventListener('click',()=>edit(g=>{const block=newBlock(get<HTMLSelectElement>('block-kind').value as Kind,g);g.blocks.push(block);selected=new Set([block.id]);},'ブロックを追加しました'));
  get('graph-delete').addEventListener('click',()=>edit(g=>deleteBlocks(g,selected),'選択と接続を削除しました'));
  get('graph-ungroup').addEventListener('click',()=>edit(g=>{if(selected.size!==1)throw new Error('解除するサブシステムを1つ選択してください');selected=new Set(ungroupBlock(g,[...selected][0]));},'サブシステムを解除しました'));
  get('graph-group').addEventListener('click',()=>edit(g=>{selected=new Set([groupBlocks(g,selected)]);},'サブシステムを作成しました'));
  const undo=()=>{guard(()=>{history.undo();invalidate();render();say('Undo');});},redo=()=>{guard(()=>{history.redo();invalidate();render();say('Redo');});};
  get('graph-undo').addEventListener('click',undo);get('graph-redo').addEventListener('click',redo);
  const cancelWire=()=>{pendingWire=null;previewPoint=null;render();say('配線をキャンセルしました');};
  get('graph-wire-cancel').addEventListener('click',cancelWire);
  const disconnect=()=>{if(selectedEdge!==null){const index=selectedEdge;edit(g=>{g.edges.splice(index,1);},'接続を削除しました');}};
  get('graph-disconnect').addEventListener('click',disconnect);
  get('graph-board').addEventListener('pointermove',event=>{if(!pendingWire)return;const rect=get('graph-board').getBoundingClientRect(),scale=rect.width/get('graph-board').clientWidth;previewPoint=[(event.clientX-rect.left)/scale,(event.clientY-rect.top)/scale];paintPreview();});
  document.addEventListener('keydown',event=>{if(host.hidden)return;if(event.key==='Escape'&&pendingWire){event.preventDefault();cancelWire();return;}if(event.target instanceof HTMLElement&&['INPUT','TEXTAREA','SELECT'].includes(event.target.tagName))return;if(event.key==='Delete'||event.key==='Backspace'){if(selectedEdge!==null){event.preventDefault();disconnect();}return;}if(!event.ctrlKey&&!event.metaKey)return;if(event.target instanceof HTMLElement&&['INPUT','TEXTAREA','SELECT'].includes(event.target.tagName))return;if(event.key.toLowerCase()==='z'){event.preventDefault();if(event.shiftKey)redo();else undo();}else if(event.key.toLowerCase()==='y'){event.preventDefault();redo();}});
  get('graph-run').addEventListener('click',run);get('graph-cancel').addEventListener('click',()=>{cancel();say('実行を中止しました');});
  get('edge-from').addEventListener('change',renderPorts);get('edge-to').addEventListener('change',renderPorts);
  get<HTMLFormElement>('graph-connect').addEventListener('submit',event=>{event.preventDefault();edit(g=>{
    const from=get<HTMLSelectElement>('edge-from').value,to=get<HTMLSelectElement>('edge-to').value;
    if(!get<HTMLSelectElement>('edge-from-port').value||!get<HTMLSelectElement>('edge-to-port').value)throw new Error('有効なポートを指定してください');
    connectPorts(g,{from,from_port:Number(get<HTMLSelectElement>('edge-from-port').value),to,to_port:Number(get<HTMLSelectElement>('edge-to-port').value)});
  },'接続を追加しました');});
  get<HTMLFormElement>('graph-clock').addEventListener('submit',event=>{event.preventDefault();guard(()=>{history.apply(root=>setClock(root,parseNumber(get<HTMLInputElement>('graph-dt').value,'dt'),parseNumber(get<HTMLInputElement>('graph-duration').value,'duration')));invalidate();render();say('時間設定を適用しました');});});
  function replace(graph:GraphModel){history.replace(graph);path=[];selected.clear();invalidate();render();}
  get('graph-load-sample').addEventListener('click',()=>guard(()=>{replace(SAMPLES[get<HTMLSelectElement>('graph-sample').value]);run();}));
  get('graph-empty').addEventListener('click',()=>guard(()=>{replace({version:1,dt:0.01,duration:5,blocks:[],edges:[]});say('新しいモデルを作成しました');}));
  get('graph-save').addEventListener('click',()=>guard(()=>{localStorage.setItem(GRAPH_STORAGE_KEY,encodeGraphDocument(history.graph));say('モデルを保存しました');}));
  get('graph-restore').addEventListener('click',()=>guard(()=>{const text=localStorage.getItem(GRAPH_STORAGE_KEY);if(text===null)throw new Error('保存済みモデルがありません');replace(decodeGraphDocument(text,normalize_graph));say('モデルを復元しました');}));
  get('graph-export').addEventListener('click',()=>download(encodeGraphDocument(history.graph),'blocksim-graph.json','application/json'));
  get('graph-csv').addEventListener('click',()=>{if(result)download(resultCsv(result),'blocksim-signals.csv','text/csv');});
  get<HTMLInputElement>('graph-import').addEventListener('change',async event=>{
    const target=event.target as HTMLInputElement,file=target.files?.[0];if(!file)return;
    try{if(file.size>1_000_000)throw new Error('モデルは1 MB以下にしてください');const incoming=decodeGraphDocument(await file.text(),normalize_graph);if(get<HTMLSelectElement>('graph-import-mode').value==='merge'){edit(g=>{selected=new Set(mergeGraph(g,incoming));},'モデルを追加しました（既存の時計・終了時刻を保持）');}else{replace(incoming);say('モデル JSON を読み込みました');}}
    catch(error){say(String(error),true);}finally{target.value='';}
  });
  get('graph-cursor').addEventListener('input',cursor);
  let savedError:string|null=null;
  try{const text=localStorage.getItem(GRAPH_STORAGE_KEY);if(text!==null)history.replace(decodeGraphDocument(text,normalize_graph));}catch(error){savedError=String(error);}
  render();if(savedError)say(`保存モデルの読み込みに失敗: ${savedError}`,true);else run();
}
