import { graphContext, type GraphModel, type GraphResult, type SolverStatistics } from './graph-model';
import { solverStatistics } from './graph-statistics';
export interface SweepParameter { key:string;address:string;label:string;path:Array<string|number>;kind:'number'|'boolean';value:number }
export interface SweepMetric { scope:string;component:number;reference:number;start:number;end:number }
export interface SweepPlan { format:'blocksim.sweep';version:1;model:GraphModel;parameter:string;values:number[];metric:SweepMetric }
export interface SweepMetrics { min:number;max:number;final:number;iae:number;ise:number }
export interface SweepRow { value:number;status:'pending'|'ok'|'error'|'cancelled';model?:GraphModel;result?:GraphResult;statistics?:SolverStatistics;metrics?:SweepMetrics;error?:string;stage?:'validation'|'execution'|'metrics' }
const KEYS=new Set(['numerator','denominator','value','before','after','step_time','amplitude','frequency','phase','offset','gain','tau','initial','signs','sample_time','kp','ki','kd','derivative_tau','anti_windup','lower','upper']);
export function sweepParameters(model:GraphModel,prefix:string[]=[]):SweepParameter[]{
 return model.blocks.flatMap(block=>{
  const address=[...prefix,block.id].join('/');if(block.params.kind==='subsystem')return sweepParameters(block.params.model as GraphModel,[...prefix,block.id]);
  const entries:SweepParameter[]=[];
  const visit=(value:unknown,path:Array<string|number>)=>{
   const name=String(path.at(-1));if(typeof value==='number'||typeof value==='boolean'||value===null&&['lower','upper'].includes(name)){entries.push({key:`${address}:${path.join('.')}`,address,label:block.label,path,kind:typeof value==='boolean'?'boolean':'number',value:typeof value==='boolean'?Number(value):Number(value??0)});}
   else if(Array.isArray(value))value.forEach((v,i)=>visit(v,[...path,i]));else if(value&&typeof value==='object')for(const [key,v]of Object.entries(value))if(KEYS.has(key)||['config','limits'].includes(key))visit(v,[...path,key]);
  };visit(block.params,[]);return entries;
 });
}
export function sweepScopes(model:GraphModel,prefix:string[]=[]):Array<{id:string,label:string,width:number}>{return model.blocks.flatMap(b=>b.params.kind==='scope'?[{id:[...prefix,b.id].join('/'),label:b.label,width:b.width}]:b.params.kind==='subsystem'?sweepScopes(b.params.model as GraphModel,[...prefix,b.id]):[]);}
function object(value:unknown,keys:string[]):Record<string,unknown>{if(!value||typeof value!=='object'||Array.isArray(value)||Object.keys(value).some(k=>!keys.includes(k))||keys.some(k=>!Object.hasOwn(value,k)))throw new Error('掃引の保存形式が不正です');return value as Record<string,unknown>;}
export function validateSweepPlan(value:unknown,normalize:(model:string)=>string):SweepPlan{
 const raw=object(value,['format','version','model','parameter','values','metric']);if(raw.format!=='blocksim.sweep'||raw.version!==1)throw new Error('未対応の掃引形式です');
 const modelJson=JSON.stringify(raw.model);if(new TextEncoder().encode(modelJson).length>1_000_000)throw new Error('モデルは1 MB以下にしてください');const model=JSON.parse(normalize(modelJson)) as GraphModel;
 const parameter=sweepParameters(model).find(p=>p.key===raw.parameter);if(!parameter)throw new Error('掃引するパラメータがありません');
 if(!Array.isArray(raw.values)||!raw.values.length||raw.values.length>32||raw.values.some(v=>typeof v!=='number'||!Number.isFinite(v)||parameter.kind==='boolean'&&v!==0&&v!==1))throw new Error('掃引値は1〜32個の有限な数値、真偽値は0/1で指定してください');
 const metric=object(raw.metric,['scope','component','reference','start','end']);const scope=sweepScopes(model).find(s=>s.id===metric.scope);
 if(!scope||!Number.isInteger(metric.component)||(metric.component as number)<0||(metric.component as number)>=scope.width||['reference','start','end'].some(k=>typeof metric[k]!=='number'||!Number.isFinite(metric[k])))throw new Error('評価する Scope・成分・目標値・時間が不正です');
 const start=(metric.start as number)/model.dt,end=(metric.end as number)/model.dt,steps=Math.round(model.duration/model.dt);
 if(Math.abs(start-Math.round(start))>1e-8||Math.abs(end-Math.round(end))>1e-8||Math.round(start)<0||Math.round(end)>steps||Math.round(end)<=Math.round(start))throw new Error('評価窓は記録時計に揃え、開始 < 終了を指定してください');
 const work=(g:GraphModel):number=>g.blocks.reduce((n,b)=>n+(b.params.kind==='subsystem'?work(b.params.model as GraphModel):b.width),0);
 if((steps+1)*sweepScopes(model).reduce((n,s)=>n+s.width,0)*raw.values.length>2_000_000||(steps+1)*work(model)*raw.values.length>20_000_000)throw new Error('掃引全体が記録量・計算量の上限を超えています。値の数や終了時刻を減らしてください');
 return {format:'blocksim.sweep',version:1,model,parameter:parameter.key,values:raw.values as number[],metric:{scope:scope.id,component:metric.component as number,reference:metric.reference as number,start:Math.round(start)*model.dt,end:Math.round(end)*model.dt}};
}
export function sweepModel(plan:SweepPlan,value:number,normalize:(model:string)=>string):GraphModel{
 const model=structuredClone(plan.model),parameter=sweepParameters(model).find(p=>p.key===plan.parameter)!;const path=parameter.address.split('/'),id=path.pop()!;const block=graphContext(model,path).blocks.find(b=>b.id===id)!;
 let target:unknown=block.params;for(const key of parameter.path.slice(0,-1))target=Reflect.get(target as object,key);Reflect.set(target as object,parameter.path.at(-1)!,parameter.kind==='boolean'?Boolean(value):value);
 return JSON.parse(normalize(JSON.stringify(model))) as GraphModel;
}
export function sweepMetrics(result:GraphResult,metric:SweepMetric):SweepMetrics{
 const trace=result.traces.find(t=>t.id===metric.scope);if(!trace)throw new Error('評価する Scope が結果にありません');
 const first=result.times.indexOf(metric.start),last=result.times.indexOf(metric.end);if(first<0||last<=first)throw new Error('評価窓が結果の記録時刻と一致しません');
 let min=Infinity,max=-Infinity,scale=0;for(let i=first;i<=last;i++){const value=trace.values[i][metric.component],error=value-metric.reference;if(!Number.isFinite(error))throw new Error('評価差が有限値の範囲外です');min=Math.min(min,value);max=Math.max(max,value);scale=Math.max(scale,Math.abs(error));}
 let iae=0,ise=0;if(scale)for(let i=first+1;i<=last;i++){const a=Math.abs((trace.values[i-1][metric.component]-metric.reference)/scale),b=Math.abs((trace.values[i][metric.component]-metric.reference)/scale),h=result.times[i]-result.times[i-1];iae+=(a+b)*h/2;ise+=(a*a+b*b)*h/2;}
 const metrics={min,max,final:trace.values[last][metric.component],iae:scale*iae,ise:(scale*ise)*scale};if(Object.values(metrics).some(v=>!Number.isFinite(v)))throw new Error('評価指標が有限値の範囲外です');return metrics;
}
export function sweepRow(plan:SweepPlan,value:number,normalize:(model:string)=>string,run:(json:string)=>GraphResult):SweepRow{
 let model:GraphModel,result:GraphResult;
 try{model=sweepModel(plan,value,normalize);}catch(error){return {value,status:'error',stage:'validation',error:String(error)};}
 try{result=run(JSON.stringify(model));}catch(error){return {value,status:'error',stage:'execution',error:String(error)};}
 const statistics=solverStatistics(model,result);
 try{return {value,status:'ok',model,result,metrics:sweepMetrics(result,plan.metric),statistics};}catch(error){return {value,status:'error',stage:'metrics',statistics,error:String(error)};}
}
export function encodeSweepPlan(plan:SweepPlan):string{const text=JSON.stringify(plan);if(new TextEncoder().encode(text).length>1_000_000)throw new Error('掃引ファイルは1 MB以下にしてください');return text;}
export function decodeSweepPlan(text:string,normalize:(model:string)=>string):SweepPlan{if(new TextEncoder().encode(text).length>1_000_000)throw new Error('掃引ファイルは1 MB以下にしてください');return validateSweepPlan(JSON.parse(text),normalize);}
export function sweepCsv(plan:SweepPlan,rows:SweepRow[]):string{
 const quote=(s:string)=>'"'+(/^\s*[=+\-@]/.test(s)?"'":'')+s.replaceAll('"','""')+'"';
 const header=['parameter','scope','component','reference','start','end','value','status','min','max','final','IAE','ISE','rate_evaluations','error'].map(quote).join(',');
 return '\uFEFF'+[header,...rows.map(row=>[quote(plan.parameter),quote(plan.metric.scope),plan.metric.component,plan.metric.reference,plan.metric.start,plan.metric.end,row.value,quote(row.status),...['min','max','final','iae','ise'].map(k=>row.metrics?.[k as keyof SweepMetrics]??''),row.statistics?.rate_evaluations??'',quote(row.error||'')].join(','))].join('\r\n');
}
