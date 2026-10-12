import type { RunSnapshot } from './graph-comparison';
import type { GraphModel, GraphResult, GraphTrace } from './graph-model';

export const MAX_RUN_ARCHIVE_BYTES=64*1024*1024;
export interface RunArchive extends RunSnapshot { format:'blocksim.run'; version:1 }
export function checkArchiveSize(bytes:number){if(bytes>MAX_RUN_ARCHIVE_BYTES)throw new Error('実行結果ファイルは64 MiB以下にしてください');}
function object(value:unknown,keys:string[],name:string):Record<string,unknown>{
  if(!value||typeof value!=='object'||Array.isArray(value)||Object.keys(value).some(key=>!keys.includes(key))||keys.some(key=>!Object.hasOwn(value,key)))throw new Error(`${name}: 保存形式が不正です`);
  return value as Record<string,unknown>;
}
function scopes(model:GraphModel,prefix=''):Array<{id:string,label:string,width:number}>{
  return model.blocks.flatMap(block=>block.params.kind==='scope'?[{id:prefix+block.id,label:block.label,width:block.width}]:block.params.kind==='subsystem'?scopes(block.params.model as GraphModel,prefix+block.id+'/'):[]);
}
function validateResult(value:unknown,model:GraphModel):GraphResult{
  const result=object(value,['times','traces','solver'],'計算結果'),expected=scopes(model),samples=Math.round(model.duration/model.dt)+1;
  if(!Number.isSafeInteger(samples)||samples<2||samples>100001||!expected.length||expected.length>256||expected.reduce((sum,s)=>sum+s.width,0)*samples>2_000_000)throw new Error('計算結果の記録数が上限を超えています');
  if(!Array.isArray(result.times)||result.times.length!==samples||!result.times.every((t,i)=>typeof t==='number'&&Number.isFinite(t)&&t===i*model.dt))throw new Error('計算結果の記録時刻がモデルと一致しません');
  if(typeof result.solver!=='string'||!result.solver.length||result.solver.length>200)throw new Error('計算結果のソルバ情報が不正です');
  if(!Array.isArray(result.traces)||result.traces.length!==expected.length)throw new Error('計算結果の Scope 数がモデルと一致しません');
  const traces:GraphTrace[]=result.traces.map((value,index)=>{
    const trace=object(value,['id','label','values'],'Scope'),scope=expected[index];
    if(trace.id!==scope.id||trace.label!==scope.label)throw new Error('計算結果の Scope ID・名前・順序がモデルと一致しません');
    if(!Array.isArray(trace.values)||trace.values.length!==samples||!trace.values.every(row=>Array.isArray(row)&&row.length===scope.width&&row.every(v=>typeof v==='number'&&Number.isFinite(v))))throw new Error(`${scope.id}: 計算結果の信号幅・記録数・数値が不正です`);
    return {id:scope.id,label:scope.label,values:trace.values as number[][]};
  });
  return {times:result.times as number[],traces,solver:result.solver};
}
export function decodeRunArchive(text:string,validate:(model:string)=>string):RunSnapshot{
  checkArchiveSize(text.length);checkArchiveSize(new TextEncoder().encode(text).length);
  const archive=object(JSON.parse(text),['format','version','model','result'],'実行結果ファイル');
  if(archive.format!=='blocksim.run'||archive.version!==1)throw new Error('未対応の実行結果ファイルです');
  const json=JSON.stringify(archive.model);if(new TextEncoder().encode(json).length>1_000_000)throw new Error('保存したモデルは1 MB以下にしてください');
  const model=JSON.parse(validate(json)) as GraphModel;
  return {model,result:validateResult(archive.result,model)};
}
export function encodeRunArchive(snapshot:RunSnapshot):string{
  const model=JSON.stringify(snapshot.model);if(new TextEncoder().encode(model).length>1_000_000)throw new Error('保存したモデルは1 MB以下にしてください');
  const archive:RunArchive={format:'blocksim.run',version:1,model:snapshot.model,result:validateResult(snapshot.result,snapshot.model)};
  const text=JSON.stringify(archive);checkArchiveSize(text.length);checkArchiveSize(new TextEncoder().encode(text).length);return text;
}
// Compare every cell without building additional, potentially large JSON strings.
export function verifyArchivedResult(stored:GraphResult,computed:GraphResult):void{
  if(stored.solver!==computed.solver||stored.times.length!==computed.times.length||stored.traces.length!==computed.traces.length||stored.times.some((t,i)=>t!==computed.times[i]))throw new Error('保存結果が現在の WASM の再計算と一致しません');
  for(let i=0;i<stored.traces.length;i++){
    const a=stored.traces[i],b=computed.traces[i];
    if(a.id!==b.id||a.label!==b.label||a.values.length!==b.values.length||a.values.some((row,t)=>row.length!==b.values[t].length||row.some((v,c)=>v!==b.values[t][c])))throw new Error(`${a.id}: 保存結果が現在の WASM の再計算と一致しません`);
  }
}
