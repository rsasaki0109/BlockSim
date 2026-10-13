import {differenceMetrics,type DifferenceMetrics} from './graph-comparison';
import type {GraphModel,GraphResult} from './graph-model';
import type {MatrixRequest,MatrixResult} from './matrix-discretize';
export interface MatrixPlan {format:'blocksim.matrix-comparison';version:1;request:MatrixRequest;samples:number;input:{before:number[];after:number[];change_sample:number}}
export interface MatrixReport {plan:MatrixPlan;conversion:MatrixResult;times:number[];input:number[][];continuous:number[][];discrete:number[][];metrics:DifferenceMetrics[];post_change_metrics:DifferenceMetrics[];change_time:number}
export function decodeMatrixPlan(text:string):MatrixPlan {
 if(new TextEncoder().encode(text).length>100_000)throw new Error('行列比較設定は100 KBまでです');
 const p=JSON.parse(text) as MatrixPlan,q=p?.request;
 if(!p||p.format!=='blocksim.matrix-comparison'||p.version!==1||Object.keys(p).some(k=>!['format','version','request','samples','input'].includes(k)))throw new Error('行列比較設定の形式が違います');
 if(!Number.isInteger(p.samples)||p.samples<2||p.samples>2001)throw new Error('比較点数は2〜2001の整数です');
 if(!q||!['zoh','tustin'].includes(q.method)||Object.keys(q).some(k=>!['a','b','c','d','initial','sample_time','method','prewarp_omega'].includes(k))||!Number.isFinite(q.sample_time)||q.sample_time<=0)throw new Error('比較行列・方式・周期が不正です');
 const n=q.a?.length,m=q.d?.[0]?.length,outputs=q.c?.length;
 const matrix=(a:unknown,rows:number,cols:number)=>Array.isArray(a)&&a.length===rows&&a.every(row=>Array.isArray(row)&&row.length===cols&&row.every(Number.isFinite));
 if(!Number.isInteger(n)||n>16||!Number.isInteger(m)||m<1||m>8||!Number.isInteger(outputs)||outputs<1||outputs>8||!matrix(q.a,n,n)||!matrix(q.b,n,m)||!matrix(q.c,outputs,n)||!matrix(q.d,outputs,m)||!Array.isArray(q.initial)||q.initial.length!==n||!q.initial.every(Number.isFinite))throw new Error('行列はA n×n、B n×m、C p×n、D p×m、初期状態nです（n=0〜16、m/p=1〜8）');
 if(q.method==='tustin'&&q.initial.some(v=>v!==0))throw new Error('Tustinは初期状態0のみです');
 if('prewarp_omega' in q&&(q.method!=='tustin'||!Number.isFinite(q.prewarp_omega)||q.prewarp_omega!<=0||q.prewarp_omega!*q.sample_time>=Math.PI))throw new Error('プリワープはTustinの0〜Nyquistの間だけです');
 const u=p.input;
 if(!u||Object.keys(u).some(k=>!['before','after','change_sample'].includes(k))||![u.before,u.after].every(row=>Array.isArray(row)&&row.length===m&&row.every(Number.isFinite))||!Number.isInteger(u.change_sample)||u.change_sample<1||u.change_sample>=p.samples)throw new Error('保持入力は入力数に一致する有限配列と1〜点数−1の変更サンプルです');
 const dt=q.sample_time/10,duration=(p.samples-1)*q.sample_time;
 if(dt<=0||!Number.isFinite(duration)||duration<=0)throw new Error('比較時間が有限の範囲を超えています');
 return structuredClone({format:p.format,version:1,request:{a:q.a,b:q.b,c:q.c,d:q.d,initial:q.initial,sample_time:q.sample_time,method:q.method,...(q.prewarp_omega===undefined?{}:{prewarp_omega:q.prewarp_omega})},samples:p.samples,input:{before:u.before,after:u.after,change_sample:u.change_sample}});
}
export function matrixComparisonModels(plan:MatrixPlan,r:MatrixResult):[GraphModel,GraphModel] {
 if(JSON.stringify(plan.request)!==JSON.stringify(r.request))throw new Error('実行した変換条件が違います');
 const q=plan.request,p=q.c.length,m=q.d[0].length,dt=q.sample_time/10;
 const a:GraphModel={version:1,dt,duration:(plan.samples-1)*q.sample_time,solver:{method:'rk45',abs_tolerance:1e-11,rel_tolerance:1e-11,min_step:Math.max(Number.MIN_VALUE,q.sample_time*1e-12),max_step:dt},blocks:[
 {id:'source',label:'Held vector input',width:m,position:[30,100],params:{kind:'step',before:structuredClone(plan.input.before),after:structuredClone(plan.input.after),step_time:(plan.input.change_sample*10)*dt}},
 {id:'plant',label:'Continuous matrix',width:p,position:[250,100],params:{kind:'state_space',a:structuredClone(q.a),b:structuredClone(q.b),c:structuredClone(q.c),d:structuredClone(q.d),initial:structuredClone(q.initial),mode:'continuous',sample_time:q.sample_time}},
 {id:'output',label:'Response',width:p,position:[490,100],params:{kind:'scope'}},
 {id:'input_record',label:'Actual input',width:m,position:[250,250],params:{kind:'scope'}}],edges:[{from:'source',from_port:0,to:'plant',to_port:0},{from:'plant',from_port:0,to:'output',to_port:0},{from:'source',from_port:0,to:'input_record',to_port:0}]};
 const b=structuredClone(a);b.blocks[1].params={kind:'state_space',a:structuredClone(r.a),b:structuredClone(r.b),c:structuredClone(r.c),d:structuredClone(r.d),initial:structuredClone(r.initial),mode:'discrete',sample_time:q.sample_time};return [a,b];
}
export function matrixComparisonReport(plan:MatrixPlan,conversion:MatrixResult,a:GraphResult,b:GraphResult):MatrixReport {
 const expected=(plan.samples-1)*10+1,width=plan.request.c.length,m=plan.request.d[0].length;
 const trace=(r:GraphResult,id:string,w:number)=>{const t=r.traces.find(t=>t.id===id);if(r.traces.length!==2||!t||t.values.length!==expected||t.values.some(row=>row.length!==w||!row.every(Number.isFinite)))throw new Error('比較結果のScope・幅・有限値が違います');return t;};
 if(a.times.length!==expected||b.times.length!==expected||!a.times.every((t,i)=>Number.isFinite(t)&&t===b.times[i]&&Math.abs(t-i*(plan.request.sample_time/10))<=1e-10*Math.max(1,Math.abs(t))))throw new Error('比較結果の記録時計が違います');
 const av=trace(a,'output',width),bv=trace(b,'output',width),ai=trace(a,'input_record',m),bi=trace(b,'input_record',m);
 for(let i=0;i<expected;i++){const u=Math.floor(i/10)<plan.input.change_sample?plan.input.before:plan.input.after;if(!ai.values[i].every((v,j)=>v===bi.values[i][j]&&v===u[j]))throw new Error('両モデルの実入力が指定した保持系列と違います');}
 const times=Array.from({length:plan.samples},(_,k)=>a.times[k*10]),continuous=times.map((_,k)=>av.values[k*10]),discrete=times.map((_,k)=>bv.values[k*10]),input=times.map((_,k)=>ai.values[k*10]);
 const metrics=(start:number)=>Array.from({length:width},(_,j)=>{const result=differenceMetrics(times.slice(start),discrete.slice(start).map(row=>row[j]),continuous.slice(start).map(row=>row[j]));if(!result)throw new Error('差分が有限の範囲を超えています');return result;});
 return structuredClone({plan,conversion,times,input,continuous,discrete,metrics:metrics(0),post_change_metrics:metrics(plan.input.change_sample),change_time:times[plan.input.change_sample]});
}
export function matrixComparisonCsv(r:MatrixReport):string {
 return ['method,sample_time_s,prewarp_omega,change_sample,time_s,output_component,continuous,discrete,delta,input_json',...r.times.flatMap((t,k)=>r.metrics.map((_,j)=>[r.plan.request.method,r.plan.request.sample_time,r.plan.request.prewarp_omega??'',r.plan.input.change_sample,t,j,r.continuous[k][j],r.discrete[k][j],r.discrete[k][j]-r.continuous[k][j],'"'+JSON.stringify(r.input[k]).replaceAll('"','""')+'"'].join(',')))].join('\r\n');
}
