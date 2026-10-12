import type { PidConfig } from './types';
export type Kind = 'constant'|'step'|'sine'|'gain'|'sum'|'product'|'saturation'|'integrator'|'first_order'|'transfer_function'|'discrete_transfer_function'|'delay'|'pid'|'mux'|'selector'|'scope'|'input'|'output'|'subsystem';
export interface Params { kind: Kind; [key: string]: unknown }
export interface GraphBlock { id: string; label: string; width: number; position: [number,number]; params: Params }
export interface Edge { from: string; from_port: number; to: string; to_port: number }
export type SolverConfig = {method:'rk4'}|{method:'rk45';abs_tolerance:number;rel_tolerance:number;min_step:number;max_step:number};
export interface SolverStatistics { accepted_steps:number;rejected_steps:number;rate_evaluations:number;output_evaluations:number;min_step:number;max_step:number;max_error_ratio:number }
export interface GraphModel { version: 1; dt: number; duration: number; solver?:SolverConfig; blocks: GraphBlock[]; edges: Edge[] }
export interface GraphDocument { version: 2; graph: GraphModel }
export interface GraphTrace { id: string; label: string; values: number[][] }
export interface GraphResult { times: number[]; traces: GraphTrace[]; solver: string; statistics?:SolverStatistics }
export const GRAPH_STORAGE_KEY = 'blocksim.graph.v2';
export const KIND_NAMES: Record<Kind,string> = {
  constant:'Constant', step:'Step', sine:'Sine', gain:'Gain', sum:'Sum', product:'Product', saturation:'Saturation',
  integrator:'Integrator', first_order:'FirstOrder', delay:'Unit Delay', pid:'PID', mux:'Mux', selector:'Selector',
  transfer_function:'TransferFunction', discrete_transfer_function:'Discrete TransferFunction', scope:'Scope', input:'Input', output:'Output', subsystem:'Subsystem',
};
export function graphContext(root: GraphModel, path: string[]): GraphModel {
  let graph = root;
  for (const id of path) {
    const block = graph.blocks.find(b => b.id === id);
    if (block?.params.kind !== 'subsystem') throw new Error('サブシステムがありません');
    graph = block.params.model as GraphModel;
  }
  return graph;
}
export function interfaceBlocks(block: GraphBlock, kind: 'input'|'output'): GraphBlock[] {
  const model = block.params.model as GraphModel;
  return model.blocks.filter(b => b.params.kind === kind).sort((a,b) => Number(a.params.port)-Number(b.params.port));
}
export function inputCount(block: GraphBlock): number {
  switch(block.params.kind) {
    case 'constant': case 'step': case 'sine': return 0;
    case 'sum': return (block.params.signs as number[]).length;
    case 'mux': case 'product': return block.params.inputs as number;
    case 'subsystem': return interfaceBlocks(block,'input').length;
    default: return 1;
  }
}
export function outputCount(block: GraphBlock): number { return block.params.kind === 'subsystem' ? interfaceBlocks(block,'output').length : 1; }
export function outputWidth(block: GraphBlock, port: number): number {
  return block.params.kind === 'subsystem' ? interfaceBlocks(block,'output')[port].width : block.width;
}
export function newBlock(kind: Kind, graph: GraphModel): GraphBlock {
  let n=1; while(graph.blocks.some(b=>b.id===`${kind}-${n}`)) n++;
  const pid: PidConfig = { kp:2, ki:1, kd:0, derivative_tau:0.05, limits:{ lower:null, upper:null }, anti_windup:false };
  const params: Record<Kind,Params> = {
    constant:{kind,value:[1]}, step:{kind,before:[0],after:[1],step_time:1},
    sine:{kind,amplitude:1,frequency:1,phase:0,offset:0}, gain:{kind,gain:1}, sum:{kind,signs:[1,-1]},
    product:{kind,inputs:2}, saturation:{kind,limits:{lower:0,upper:1}}, integrator:{kind,initial:[0]},
    transfer_function:{kind,numerator:[1],denominator:[1,1],initial:[0]},
    discrete_transfer_function:{kind,numerator:[0.2,0],denominator:[1,-0.8],initial:[0],sample_time:graph.dt},
    first_order:{kind,gain:1,tau:1,initial:[0]}, delay:{kind,initial:[0],sample_time:graph.dt},
    pid:{kind,config:pid,mode:'continuous',sample_time:graph.dt}, mux:{kind,inputs:2}, selector:{kind,indices:[0]},
    scope:{kind}, input:{kind,port:graph.blocks.filter(b=>b.params.kind==='input').length},
    output:{kind,port:graph.blocks.filter(b=>b.params.kind==='output').length}, subsystem:{kind},
  };
  if (kind==='subsystem') {
    params.subsystem.model={version:1,dt:graph.dt,duration:graph.duration,
      blocks:[{id:'in',label:'Input 0',width:1,position:[40,100],params:{kind:'input',port:0}},
        {id:'gain',label:'Gain',width:1,position:[270,100],params:{kind:'gain',gain:1}},
        {id:'out',label:'Output 0',width:1,position:[500,100],params:{kind:'output',port:0}}],
      edges:[{from:'in',from_port:0,to:'gain',to_port:0},{from:'gain',from_port:0,to:'out',to_port:0}] } satisfies GraphModel;
  }
  const width=kind==='mux'?2:1;
  return { id:`${kind}-${n}`, label:KIND_NAMES[kind], width, position:[50+(graph.blocks.length%5)*210,100+Math.floor(graph.blocks.length/5)*150],params:params[kind] };
}
export function setClock(graph: GraphModel, dt: number, duration: number) {
  graph.dt=dt; graph.duration=duration;
  for (const b of graph.blocks) if (b.params.kind==='subsystem') setClock(b.params.model as GraphModel,dt,duration);
}
export function setSolver(graph:GraphModel,solver:SolverConfig){graph.solver=structuredClone(solver);const inherit=(g:GraphModel)=>{for(const b of g.blocks)if(b.params.kind==='subsystem'){const inner=b.params.model as GraphModel;delete inner.solver;inherit(inner);}};inherit(graph);}
export function solverKey(graph:GraphModel):string {const s=graph.solver;return !s||s.method==='rk4'?'rk4':['rk45',s.abs_tolerance,s.rel_tolerance,s.min_step,s.max_step].join('|');}
export function deleteBlocks(graph: GraphModel, ids: Set<string>) {
  graph.blocks=graph.blocks.filter(b=>!ids.has(b.id)); graph.edges=graph.edges.filter(e=>!ids.has(e.from)&&!ids.has(e.to));
}
export function groupBlocks(graph: GraphModel, ids: Set<string>): string {
  if (!ids.size) throw new Error('まとめるブロックを選択してください');
  const selected=graph.blocks.filter(b=>ids.has(b.id));
  if(selected.some(b=>b.params.kind==='input'||b.params.kind==='output')) throw new Error('境界 Input / Output はグループ化できません');
  const sub=newBlock('subsystem',graph); sub.label='Subsystem';
  const incoming=graph.edges.filter(e=>!ids.has(e.from)&&ids.has(e.to));
  const outgoing=graph.edges.filter(e=>ids.has(e.from)&&!ids.has(e.to));
  if (!outgoing.length) throw new Error('外部への出力接続があるブロックを選んでください');
  const inner: GraphModel={version:1,dt:graph.dt,duration:graph.duration,blocks:structuredClone(selected),edges:graph.edges.filter(e=>ids.has(e.from)&&ids.has(e.to)).map(e=>({...e}))};
  const used=new Set(inner.blocks.map(b=>b.id));
  const fresh=(prefix:string)=>{let n=0;while(used.has(`${prefix}${n}`))n++;const id=`${prefix}${n}`;used.add(id);return id;};
  const parent=graph.edges.filter(e=>!ids.has(e.from)&&!ids.has(e.to));
  incoming.forEach((e,port)=>{
    const id=fresh('boundary-in-'), source=graph.blocks.find(b=>b.id===e.from)!;
    inner.blocks.push({id,label:`Input ${port}`,width:outputWidth(source,e.from_port),position:[30,30+port*140],params:{kind:'input',port}});
    inner.edges.push({from:id,from_port:0,to:e.to,to_port:e.to_port});parent.push({...e,to:sub.id,to_port:port});
  });
  const outputs=new Map<string,number>();
  outgoing.forEach(e=>{
    const key=`${e.from}:${e.from_port}`;
    if(!outputs.has(key)) {
      const port=outputs.size;outputs.set(key,port);
      const id=fresh('boundary-out-'), source=graph.blocks.find(b=>b.id===e.from)!;
      inner.blocks.push({id,label:`Output ${port}`,width:outputWidth(source,e.from_port),position:[1300,30+port*140],params:{kind:'output',port}});
      inner.edges.push({...e,to:id,to_port:0});
    }
    parent.push({...e,from:sub.id,from_port:outputs.get(key)!});
  });
  sub.params.model=inner;
  sub.position=[Math.min(...selected.map(b=>b.position[0])),Math.min(...selected.map(b=>b.position[1]))];
  graph.blocks=graph.blocks.filter(b=>!ids.has(b.id));graph.blocks.push(sub);graph.edges=parent;
  return sub.id;
}
export class GraphHistory {
  private past: GraphModel[]=[]; private future: GraphModel[]=[];
  constructor(public graph: GraphModel, private normalize: (json:string)=>string) { this.graph=this.valid(graph); }
  private valid(graph: GraphModel) { return JSON.parse(this.normalize(JSON.stringify(graph))) as GraphModel; }
  apply(edit: (graph:GraphModel)=>void) {
    const next=structuredClone(this.graph);edit(next);const valid=this.valid(next);
    if(JSON.stringify(valid)===JSON.stringify(this.graph))return;
    this.past.push(this.graph);if(this.past.length>50)this.past.shift();this.graph=valid;this.future=[];
  }
  replace(graph:GraphModel) { this.apply(root=>Object.assign(root,graph)); }
  undo() { const previous=this.past.pop();if(previous){this.future.push(this.graph);this.graph=previous;} }
  redo() { const next=this.future.pop();if(next){this.past.push(this.graph);this.graph=next;} }
  get canUndo(){return this.past.length>0;} get canRedo(){return this.future.length>0;}
}
export function decodeGraphDocument(text: string, normalize:(json:string)=>string): GraphModel {
  if(new TextEncoder().encode(text).length>1_000_000)throw new Error('モデルは1 MB以下にしてください');
  const obj=JSON.parse(text) as Record<string,unknown>;
  if(!obj || typeof obj!=='object' || Array.isArray(obj))throw new Error('グラフ形式が不正です');
  if('graph' in obj) {
    if(obj.version!==2 || Object.keys(obj).some(k=>!['version','graph'].includes(k)))throw new Error('未対応のグラフ保存形式です');
    return JSON.parse(normalize(JSON.stringify(obj.graph))) as GraphModel;
  }
  return JSON.parse(normalize(JSON.stringify(obj))) as GraphModel;
}
export function encodeGraphDocument(graph:GraphModel) { return JSON.stringify({version:2,graph} satisfies GraphDocument,null,2); }
export function resultCsv(result:GraphResult):string {
  const cell=(s:string)=>`"${(/^[=+\-@]/.test(s)?"'":"")+s.replaceAll('"','""')}"`;
  const headers=['time',...result.traces.flatMap(t=>t.values[0].map((_,i)=>`${t.label} (${t.id})[${i}]`))].map(cell).join(',');
  const rows=result.times.map((time,i)=>[time,...result.traces.flatMap(t=>t.values[i])].join(','));
  return '\uFEFF'+[headers,...rows].join('\r\n');
}
