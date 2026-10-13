import {differenceMetrics,type DifferenceMetrics} from './graph-comparison';
import type {GraphModel,GraphResult} from './graph-model';
import type {ConversionResult,TustinRequest} from './discretize';

export interface HeldStepInput {kind:'held_step';before:number;after:number;change_sample:number}
export interface SampledSineInput {kind:'sampled_sine';frequency_hz:number;amplitude:number;phase_rad:number;offset:number}
export interface SampleSequenceInput {kind:'sample_sequence';values:number[]}
export interface ConversionPlan {format:'blocksim.conversion-comparison';version:1;method:'tustin'|'zoh';request:TustinRequest;samples:number;input?:HeldStepInput|SampledSineInput|SampleSequenceInput}
export interface ConversionReport {plan:ConversionPlan;conversion:ConversionResult;times:number[];continuous:number[];discrete:number[];metrics:DifferenceMetrics;input?:number[];change_time?:number;post_change_metrics?:DifferenceMetrics}
export function decodeConversionPlan(text:string):ConversionPlan {
 if(new TextEncoder().encode(text).length>100_000)throw new Error('比較設定は100 KBまでです');
 const p=JSON.parse(text) as ConversionPlan;
 if(!p||p.format!=='blocksim.conversion-comparison'||p.version!==1||!['tustin','zoh'].includes(p.method)||Object.keys(p).some(k=>!['format','version','method','request','samples','input'].includes(k)))throw new Error('比較設定の形式が違います');
 if(!Number.isInteger(p.samples)||p.samples<2||p.samples>2001)throw new Error('比較点数は2〜2001の整数です');
 const q=p.request;
 if(!q||Object.keys(q).some(k=>!['numerator','denominator','sample_time','prewarp_omega'].includes(k))||!Number.isFinite(q.sample_time)||q.sample_time<=0||![q.numerator,q.denominator].every(a=>Array.isArray(a)&&a.length>=1&&a.length<=9&&a.every(Number.isFinite))||q.numerator.length>q.denominator.length||q.denominator[0]===0||('prewarp_omega' in q&&(!Number.isFinite(q.prewarp_omega)||q.prewarp_omega!<=0||p.method==='zoh')))throw new Error('比較係数・周期・プリワープが不正です');
 if('input' in p){const u=p.input;
  if(u?.kind==='held_step'){
   if(Object.keys(u).some(k=>!['kind','before','after','change_sample'].includes(k))||!Number.isFinite(u.before)||!Number.isFinite(u.after)||!Number.isInteger(u.change_sample)||u.change_sample<1||u.change_sample>=p.samples)throw new Error('保持入力は有限値と、1〜点数−1の整数の変更サンプルが必要です');
  }else if(u?.kind==='sampled_sine'){
   if(Object.keys(u).some(k=>!['kind','frequency_hz','amplitude','phase_rad','offset'].includes(k))||![u.frequency_hz,u.amplitude,u.phase_rad,u.offset].every(Number.isFinite)||u.frequency_hz<0||!Number.isFinite(2*Math.PI*u.frequency_hz)||u.frequency_hz*q.sample_time>.5)throw new Error('保持正弦波は有限値と、0〜Nyquist（1/(2Ts) Hz）の周波数が必要です');
  }else if(u?.kind==='sample_sequence'){
   if(Object.keys(u).some(k=>!['kind','values'].includes(k))||!Array.isArray(u.values)||u.values.length!==p.samples||!u.values.every(Number.isFinite))throw new Error('入力系列は比較点数と同じ長さの有限値配列が必要です');
  }else throw new Error('比較入力の形式が違います');
 }
 const dt=q.sample_time/10,duration=(p.samples-1)*q.sample_time;
 if(dt<=0||!Number.isFinite(duration)||duration<=0)throw new Error('比較時間が有限の範囲を超えています');
 return structuredClone(p);
}
// An isolated scalar transfer, not the connected source model. The optional
// Step changes exactly at a conversion instant; sine goes through an actual
// zero-order discrete transfer, with its held output recorded independently.
export function conversionModels(plan:ConversionPlan,conversion:ConversionResult):[GraphModel,GraphModel] {
 const q=plan.request;
 const continuous:GraphModel={version:1,dt:q.sample_time/10,duration:(plan.samples-1)*q.sample_time,solver:{method:'rk45',abs_tolerance:1e-11,rel_tolerance:1e-11,min_step:Math.max(Number.MIN_VALUE,q.sample_time*1e-12),max_step:q.sample_time/10},blocks:[
  {id:'source',label:plan.input?'Held input':'Unit step',width:1,position:[30,100],params:plan.input?.kind==='held_step'?{kind:'step',before:[plan.input.before],after:[plan.input.after],step_time:(plan.input.change_sample*10)*(q.sample_time/10)}:plan.input?.kind==='sampled_sine'?{kind:'sine',amplitude:plan.input.amplitude,frequency:plan.input.frequency_hz,phase:plan.input.phase_rad,offset:plan.input.offset}:{kind:'constant',value:[1]}},
  {id:'plant',label:'Continuous',width:1,position:[260,100],params:{kind:'transfer_function',numerator:q.numerator,denominator:q.denominator,initial:Array(q.denominator.length-1).fill(0)}},
  {id:'output',label:'Response',width:1,position:[500,100],params:{kind:'scope'}},
 ],edges:[{from:'source',from_port:0,to:'plant',to_port:0},{from:'plant',from_port:0,to:'output',to_port:0}]};
 if(plan.input?.kind==='sampled_sine'){
  continuous.blocks.push({id:'hold',label:'Sample and hold',width:1,position:[140,100],params:{kind:'discrete_transfer_function',numerator:[1],denominator:[1],initial:[],sample_time:q.sample_time}},{id:'input_record',label:'Actual held input',width:1,position:[260,230],params:{kind:'scope'}});
  continuous.edges[0].from='hold';continuous.edges.push({from:'source',from_port:0,to:'hold',to_port:0},{from:'hold',from_port:0,to:'input_record',to_port:0});
 }
 if(plan.input?.kind==='sample_sequence'){
  continuous.blocks[0].params={kind:'sampled_input',values:plan.input.values.map(v=>[v]),sample_time:q.sample_time};
  continuous.blocks.push({id:'input_record',label:'Actual held input',width:1,position:[260,230],params:{kind:'scope'}});
  continuous.edges.push({from:'source',from_port:0,to:'input_record',to_port:0});
 }
 const discrete=structuredClone(continuous);discrete.blocks[1].params={kind:'discrete_transfer_function',numerator:conversion.numerator,denominator:conversion.denominator,initial:Array(conversion.denominator.length-1).fill(0),sample_time:q.sample_time};
 return [continuous,discrete];
}
export function conversionReport(plan:ConversionPlan,conversion:ConversionResult,a:GraphResult,b:GraphResult):ConversionReport {
 const expected=(plan.samples-1)*10+1;
 const count=plan.input?.kind==='sampled_sine'||plan.input?.kind==='sample_sequence'?2:1;
 const avTrace=a.traces.find(t=>t.id==='output'),bvTrace=b.traces.find(t=>t.id==='output');
 if(a.times.length!==expected||b.times.length!==expected||!a.times.every((t,i)=>Number.isFinite(t)&&t===b.times[i])||a.traces.length!==count||b.traces.length!==count||avTrace?.values.length!==expected||bvTrace?.values.length!==expected)throw new Error('比較結果の記録時計・Scopeが違います');
 const times:number[]=[],continuous:number[]=[],discrete:number[]=[];
 for(let k=0;k<plan.samples;k++){
  const i=k*10,t=a.times[i],target=k*plan.request.sample_time;
  if(Math.abs(t-target)>1e-10*Math.max(1,Math.abs(target)))throw new Error('サンプル時刻が一致しません');
  const av=avTrace.values[i],bv=bvTrace.values[i];
  if(av.length!==1||bv.length!==1||!Number.isFinite(av[0])||!Number.isFinite(bv[0]))throw new Error('比較応答が有限の単一成分ではありません');
  times.push(t);continuous.push(av[0]);discrete.push(bv[0]);
 }
 const metrics=differenceMetrics(times,discrete,continuous);if(!metrics)throw new Error('差分が有限の範囲を超えています');
 let held:Partial<Pick<ConversionReport,'input'|'change_time'|'post_change_metrics'>>={};
 if(plan.input?.kind==='held_step'){const u=plan.input,start=u.change_sample,post=differenceMetrics(times.slice(start),discrete.slice(start),continuous.slice(start));if(!post)throw new Error('変更後の差分が有限の範囲を超えています');held={input:times.map((_,k)=>k<start?u.before:u.after),change_time:times[start],post_change_metrics:post};}
 if(plan.input?.kind==='sampled_sine'||plan.input?.kind==='sample_sequence'){
  const ai=a.traces.find(t=>t.id==='input_record'),bi=b.traces.find(t=>t.id==='input_record');
  if(ai?.values.length!==expected||bi?.values.length!==expected)throw new Error('保持入力の記録がありません');
  for(let i=0;i<expected;i++){const v=ai.values[i],w=bi.values[i];if(v.length!==1||w.length!==1||!Number.isFinite(v[0])||v[0]!==w[0]||v[0]!==ai.values[Math.floor(i/10)*10][0])throw new Error('両モデルの入力が同じ保持系列ではありません');}
  held={input:times.map((_,k)=>ai.values[k*10][0])};
  if(plan.input.kind==='sample_sequence'){const expected=plan.input.values;if(!held.input!.every((v,k)=>v===expected[k]))throw new Error('実入力が指定した系列と違います');}
 }
 return structuredClone({plan,conversion,times,continuous,discrete,metrics,...held});
}
export function conversionCsv(r:ConversionReport):string {
 const u=r.plan.input;
 const columns=u?.kind==='held_step'?',input,change_sample,before,after':u?.kind==='sampled_sine'?',input,frequency_hz,amplitude,phase_rad,offset':u?.kind==='sample_sequence'?',input,sample_index':'';
 return ['method,sample_time_s,time_s,continuous,discrete,delta'+columns,...r.times.map((t,i)=>[r.plan.method,r.plan.request.sample_time,t,r.continuous[i],r.discrete[i],r.discrete[i]-r.continuous[i],...(u?.kind==='held_step'?[r.input![i],u.change_sample,u.before,u.after]:u?.kind==='sampled_sine'?[r.input![i],u.frequency_hz,u.amplitude,u.phase_rad,u.offset]:u?.kind==='sample_sequence'?[r.input![i],i]:[])].join(','))].join('\r\n');
}
export function conversionInputLabel(r:ConversionReport):string {
 const u=r.plan.input;
 return u?.kind==='held_step'?`入力${u.before}→${u.after}、変更サンプル${u.change_sample} (t=${r.change_time}s)`:u?.kind==='sampled_sine'?`保持正弦波 ${u.frequency_hz}Hz · 振幅${u.amplitude} · 位相${u.phase_rad}rad · オフセット${u.offset} · Nyquist=${.5/r.plan.request.sample_time}Hz`:u?.kind==='sample_sequence'?`任意の保持入力 ${u.values.length}サンプル · 初回${u.values[0]} · 最終${u.values.at(-1)}`:'入力1';
}
export function decodeInputCsv(text:string):number[] {
 if(new TextEncoder().encode(text).length>100_000)throw new Error('入力CSVは100 KBまでです');
 const rows=text.replace(/^\uFEFF/,'').trim().split(/\r?\n/).map(v=>v.trim());if(rows[0]==='input')rows.shift();
 if(rows.length<2||rows.length>2001||rows.some(v=>!/^[+-]?(?:\d+(?:\.\d*)?|\.\d+)(?:[eE][+-]?\d+)?$/.test(v)||!Number.isFinite(Number(v))))throw new Error('入力CSVは2〜2001行の数値1列です。任意の見出しはinputのみです');
 return rows.map(Number);
}
