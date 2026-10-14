import {decodeMatrixPlan,isMatrixSequence,matrixInputAt,type MatrixReport} from './matrix-comparison';
import {differenceMetrics} from './graph-comparison';
export const MATRIX_RESULT_LIMIT=8*1024*1024;
export function checkMatrixResultSize(bytes:number){if(bytes>MATRIX_RESULT_LIMIT)throw new Error('行列比較結果は8 MiBまでです');}
function object(value:unknown,keys:string[],name:string,optional:string[]=[]):Record<string,unknown>{
 if(!value||typeof value!=='object'||Array.isArray(value)||Object.keys(value).some(k=>!keys.includes(k)&&!optional.includes(k))||keys.some(k=>!Object.hasOwn(value,k)))throw new Error(name+'の保存形式が不正です');return value as Record<string,unknown>;
}
function same(a:unknown,b:unknown):boolean {
 if(a===b)return true;if(Array.isArray(a)&&Array.isArray(b))return a.length===b.length&&a.every((v,i)=>same(v,b[i]));
 if(a&&b&&typeof a==='object'&&typeof b==='object'&&!Array.isArray(a)&&!Array.isArray(b)){const x=a as Record<string,unknown>,y=b as Record<string,unknown>;return Object.keys(x).length===Object.keys(y).length&&Object.keys(x).every(k=>Object.hasOwn(y,k)&&same(x[k],y[k]));}return false;
}
export function decodeMatrixResult(text:string):MatrixReport {
 checkMatrixResultSize(text.length);checkMatrixResultSize(new TextEncoder().encode(text).length);
 const raw=object(JSON.parse(text),['format','version','plan','conversion','times','input','continuous','discrete','metrics'],'比較結果',['post_change_metrics','change_time']);
 if(raw.format!=='blocksim.matrix-comparison-result'||raw.version!==1)throw new Error('行列比較結果の形式が違います');
 const plan=decodeMatrixPlan(JSON.stringify(raw.plan)),n=plan.request.a.length,m=plan.request.d[0].length,p=plan.request.c.length,count=plan.samples;
 const matrix=(v:unknown,rows:number,cols:number)=>Array.isArray(v)&&v.length===rows&&v.every(row=>Array.isArray(row)&&row.length===cols&&row.every(x=>typeof x==='number'&&Number.isFinite(x)));
 const times=raw.times as number[];if(!Array.isArray(times)||times.length!==count||times.some((t,k)=>typeof t!=='number'||!Number.isFinite(t)||t!==k*10*(plan.request.sample_time/10)||(k>0&&t<=times[k-1])))throw new Error('比較結果の取得時刻が違います');
 if(!matrix(raw.input,count,m)||!matrix(raw.continuous,count,p)||!matrix(raw.discrete,count,p))throw new Error('比較結果の点数・信号幅・有限値が違います');
 const input=raw.input as number[][],continuous=raw.continuous as number[][],discrete=raw.discrete as number[][];
 if(input.some((row,k)=>!same(row,matrixInputAt(plan,k))))throw new Error('比較結果の実入力が実行条件と違います');
 const metrics=(v:unknown,start:number)=>{if(!Array.isArray(v)||v.length!==p)throw new Error('比較結果の差分指標数が違います');return v.map((value,j)=>{const stored=object(value,['maxAbs','rms','final','area'],'差分指標'),computed=differenceMetrics(times.slice(start),discrete.slice(start).map(row=>row[j]),continuous.slice(start).map(row=>row[j]));if(!computed||!same(stored,computed))throw new Error('比較結果の差分指標が取得値と一致しません');return computed;});};
 const full=metrics(raw.metrics,0),conversion=object(raw.conversion,['request','a','b','c','d','initial','mapping_scale','squarings','terms','residual','method'],'変換結果');
 const q=decodeMatrixPlan(JSON.stringify({...plan,request:conversion.request})).request;
 if(!same(q,plan.request)||!matrix(conversion.a,n,n)||!matrix(conversion.b,n,m)||!matrix(conversion.c,p,n)||!matrix(conversion.d,p,m)||!Array.isArray(conversion.initial)||conversion.initial.length!==n||!conversion.initial.every(Number.isFinite)||typeof conversion.method!=='string'||conversion.method.length===0||conversion.method.length>500||!(conversion.mapping_scale===null||typeof conversion.mapping_scale==='number'&&Number.isFinite(conversion.mapping_scale))||!Number.isSafeInteger(conversion.squarings)||Number(conversion.squarings)<0||!Number.isSafeInteger(conversion.terms)||Number(conversion.terms)<0||typeof conversion.residual!=='number'||!Number.isFinite(conversion.residual)||conversion.residual<0)throw new Error('比較結果の変換行列・条件が不正です');
 let step:Pick<MatrixReport,'post_change_metrics'|'change_time'>={};if(isMatrixSequence(plan.input)){if('post_change_metrics' in raw||'change_time' in raw)throw new Error('系列結果にstep指標は含められません');}else{if(raw.change_time!==times[plan.input.change_sample])throw new Error('比較結果の入力変更時刻が違います');step={change_time:raw.change_time as number,post_change_metrics:metrics(raw.post_change_metrics,plan.input.change_sample)};}
 return structuredClone({plan,conversion:{...conversion,request:q} as unknown as MatrixReport['conversion'],times,input,continuous,discrete,metrics:full,...step});
}
export function verifyMatrixResult(stored:MatrixReport,computed:MatrixReport):void {if(!same(stored,computed))throw new Error('保存した比較結果が現在のWASMの再計算と一致しません');}
