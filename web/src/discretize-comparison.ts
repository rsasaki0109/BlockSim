import {differenceMetrics,type DifferenceMetrics} from './graph-comparison';
import type {GraphModel,GraphResult} from './graph-model';
import type {ConversionResult,TustinRequest} from './discretize';

export interface HeldStepInput {kind:'held_step';before:number;after:number;change_sample:number}
export interface ConversionPlan {format:'blocksim.conversion-comparison';version:1;method:'tustin'|'zoh';request:TustinRequest;samples:number;input?:HeldStepInput}
export interface ConversionReport {plan:ConversionPlan;conversion:ConversionResult;times:number[];continuous:number[];discrete:number[];metrics:DifferenceMetrics;input?:number[];change_time?:number;post_change_metrics?:DifferenceMetrics}
export function decodeConversionPlan(text:string):ConversionPlan {
 if(new TextEncoder().encode(text).length>100_000)throw new Error('比較設定は100 KBまでです');
 const p=JSON.parse(text) as ConversionPlan;
 if(!p||p.format!=='blocksim.conversion-comparison'||p.version!==1||!['tustin','zoh'].includes(p.method)||Object.keys(p).some(k=>!['format','version','method','request','samples','input'].includes(k)))throw new Error('比較設定の形式が違います');
 if(!Number.isInteger(p.samples)||p.samples<2||p.samples>2001)throw new Error('比較点数は2〜2001の整数です');
 if('input' in p){const u=p.input;if(!u||u.kind!=='held_step'||Object.keys(u).some(k=>!['kind','before','after','change_sample'].includes(k))||!Number.isFinite(u.before)||!Number.isFinite(u.after)||!Number.isInteger(u.change_sample)||u.change_sample<1||u.change_sample>=p.samples)throw new Error('保持入力は有限値と、1〜点数−1の整数の変更サンプルが必要です');}
 const q=p.request;
 if(!q||Object.keys(q).some(k=>!['numerator','denominator','sample_time','prewarp_omega'].includes(k))||!Number.isFinite(q.sample_time)||q.sample_time<=0||![q.numerator,q.denominator].every(a=>Array.isArray(a)&&a.length>=1&&a.length<=9&&a.every(Number.isFinite))||q.numerator.length>q.denominator.length||q.denominator[0]===0||('prewarp_omega' in q&&(!Number.isFinite(q.prewarp_omega)||q.prewarp_omega!<=0||p.method==='zoh')))throw new Error('比較係数・周期・プリワープが不正です');
 const dt=q.sample_time/10,duration=(p.samples-1)*q.sample_time;
 if(dt<=0||!Number.isFinite(duration)||duration<=0)throw new Error('比較時間が有限の範囲を超えています');
 return structuredClone(p);
}
// An isolated scalar transfer, not the connected source model. The optional
// Step changes exactly at a conversion instant and is held on each interval.
export function conversionModels(plan:ConversionPlan,conversion:ConversionResult):[GraphModel,GraphModel] {
 const q=plan.request;
 const continuous:GraphModel={version:1,dt:q.sample_time/10,duration:(plan.samples-1)*q.sample_time,solver:{method:'rk45',abs_tolerance:1e-11,rel_tolerance:1e-11,min_step:Math.max(Number.MIN_VALUE,q.sample_time*1e-12),max_step:q.sample_time/10},blocks:[
  {id:'source',label:plan.input?'Held step':'Unit step',width:1,position:[30,100],params:plan.input?{kind:'step',before:[plan.input.before],after:[plan.input.after],step_time:(plan.input.change_sample*10)*(q.sample_time/10)}:{kind:'constant',value:[1]}},
  {id:'plant',label:'Continuous',width:1,position:[260,100],params:{kind:'transfer_function',numerator:q.numerator,denominator:q.denominator,initial:Array(q.denominator.length-1).fill(0)}},
  {id:'output',label:'Response',width:1,position:[500,100],params:{kind:'scope'}},
 ],edges:[{from:'source',from_port:0,to:'plant',to_port:0},{from:'plant',from_port:0,to:'output',to_port:0}]};
 const discrete=structuredClone(continuous);discrete.blocks[1].params={kind:'discrete_transfer_function',numerator:conversion.numerator,denominator:conversion.denominator,initial:Array(conversion.denominator.length-1).fill(0),sample_time:q.sample_time};
 return [continuous,discrete];
}
export function conversionReport(plan:ConversionPlan,conversion:ConversionResult,a:GraphResult,b:GraphResult):ConversionReport {
 const expected=(plan.samples-1)*10+1;
 if(a.times.length!==expected||b.times.length!==expected||!a.times.every((t,i)=>Number.isFinite(t)&&t===b.times[i])||a.traces.length!==1||b.traces.length!==1||a.traces[0].values.length!==expected||b.traces[0].values.length!==expected)throw new Error('比較結果の記録時計・Scopeが違います');
 const times:number[]=[],continuous:number[]=[],discrete:number[]=[];
 for(let k=0;k<plan.samples;k++){
  const i=k*10,t=a.times[i],target=k*plan.request.sample_time;
  if(Math.abs(t-target)>1e-10*Math.max(1,Math.abs(target)))throw new Error('サンプル時刻が一致しません');
  const av=a.traces[0].values[i],bv=b.traces[0].values[i];
  if(av.length!==1||bv.length!==1||!Number.isFinite(av[0])||!Number.isFinite(bv[0]))throw new Error('比較応答が有限の単一成分ではありません');
  times.push(t);continuous.push(av[0]);discrete.push(bv[0]);
 }
 const metrics=differenceMetrics(times,discrete,continuous);if(!metrics)throw new Error('差分が有限の範囲を超えています');
 let held:{}|{input:number[];change_time:number;post_change_metrics:DifferenceMetrics}={};
 if(plan.input){const start=plan.input.change_sample,post=differenceMetrics(times.slice(start),discrete.slice(start),continuous.slice(start));if(!post)throw new Error('変更後の差分が有限の範囲を超えています');held={input:times.map((_,k)=>k<start?plan.input!.before:plan.input!.after),change_time:times[start],post_change_metrics:post};}
 return structuredClone({plan,conversion,times,continuous,discrete,metrics,...held});
}
export function conversionCsv(r:ConversionReport):string {
 const u=r.plan.input;
 return ['method,sample_time_s,time_s,continuous,discrete,delta'+(u?',input,change_sample,before,after':''),...r.times.map((t,i)=>[r.plan.method,r.plan.request.sample_time,t,r.continuous[i],r.discrete[i],r.discrete[i]-r.continuous[i],...(u?[r.input![i],u.change_sample,u.before,u.after]:[])].join(','))].join('\r\n');
}
