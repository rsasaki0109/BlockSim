import {type GraphModel} from './graph-model';
export type FrequencyDomain='continuous'|'discrete';
export interface FrequencyRequest { source:string;source_component:number;scope:string;scope_component:number;min_omega:number;max_omega:number;points:number;open_loop?:boolean }
export interface FrequencyCheck { source_label:string;scope_label:string;variables:number }
export interface FrequencyPoint { omega:number;real:number|null;imag:number|null;magnitude:number|null;magnitude_db:number|null;phase_deg:number|null;relative_residual:number|null;error:string|null }
export interface GainCrossing {omega:number;phase_deg:number;phase_margin_deg:number;relative_residual:number}
export interface PhaseCrossing {omega:number;phase_level_deg:number;magnitude_db:number;gain_margin_db:number;gain_margin:number|null;relative_residual:number}
export interface MarginIssue {kind:string;min_omega:number;max_omega:number;message:string}
export interface MarginAnalysis {gain_crossings:GainCrossing[];phase_crossings:PhaseCrossing[];issues:MarginIssue[];additional_evaluations:number;method:string}
export interface FrequencyResult { sample_time?:number;nyquist_omega?:number;states?:number;margins?:MarginAnalysis;request:FrequencyRequest;check:FrequencyCheck;points:FrequencyPoint[];method:string }
export interface FrequencyPlan { format:'blocksim.frequency';version:1;domain?:FrequencyDomain;model:GraphModel;request:FrequencyRequest }
export type FrequencyValidator=(model:string,request:string,domain?:FrequencyDomain)=>string;
export function frequencySources(model:GraphModel,prefix:string[]=[]):Array<{id:string,label:string,width:number}>{return model.blocks.flatMap(b=>['constant','sampled_input','step','sine'].includes(b.params.kind)?[{id:[...prefix,b.id].join('/'),label:b.label,width:b.width}]:b.params.kind==='subsystem'?frequencySources(b.params.model as GraphModel,[...prefix,b.id]):[]);}
function object(value:unknown,keys:string[],optional:string[]=[]){if(!value||typeof value!=='object'||Array.isArray(value)||Object.keys(value).some(k=>!keys.includes(k)&&!optional.includes(k))||keys.some(k=>!Object.hasOwn(value,k)))throw new Error('周波数解析の保存形式が不正です');return value as Record<string,unknown>;}
export function validateFrequencyPlan(value:unknown,normalize:(model:string)=>string,check:FrequencyValidator):FrequencyPlan{
 const raw=object(value,['format','version','model','request'],['domain']);if(Object.hasOwn(raw,'domain')&&raw.domain!=='continuous'&&raw.domain!=='discrete')throw new Error('周波数解析の解析領域が不正です');if(raw.format!=='blocksim.frequency'||raw.version!==1)throw new Error('未対応の周波数解析形式です');
 const json=JSON.stringify(raw.model);if(new TextEncoder().encode(json).length>1_000_000)throw new Error('モデルは1 MB以下にしてください');const model=JSON.parse(normalize(json)) as GraphModel;
 const q=object(raw.request,['source','source_component','scope','scope_component','min_omega','max_omega','points'],['open_loop']);
 if(Object.hasOwn(q,'open_loop')&&typeof q.open_loop!=='boolean')throw new Error('開ループの指定は有効／無効で指定してください');
 if(typeof q.source!=='string'||typeof q.scope!=='string'||['source_component','scope_component','points'].some(k=>!Number.isSafeInteger(q[k])||(q[k] as number)<0)||['min_omega','max_omega'].some(k=>typeof q[k]!=='number'||!Number.isFinite(q[k]))||(q.min_omega as number)<1e-9||(q.max_omega as number)>1e9||(q.max_omega as number)<=(q.min_omega as number)||(q.points as number)<2||(q.points as number)>2048)throw new Error('解析条件は1e-9 ≤ 最小 < 最大 ≤ 1e9 rad/s、2〜2048点で指定してください');
 if((Math.log(q.max_omega as number)-Math.log(q.min_omega as number))/((q.points as number)-1)<=64*Number.EPSILON)throw new Error('周波数範囲が浮動小数点の分解能に対して狭すぎます');
 const request=Object.fromEntries(Object.keys(q).map(k=>[k,q[k]])) as unknown as FrequencyRequest;check(JSON.stringify(model),JSON.stringify(request),raw.domain==='discrete'?'discrete':'continuous');return {format:'blocksim.frequency',version:1,...(raw.domain==='discrete'?{domain:'discrete' as const}:{}),model,request};
}
export function encodeFrequencyPlan(plan:FrequencyPlan){const text=JSON.stringify(plan);if(new TextEncoder().encode(text).length>1_000_000)throw new Error('周波数解析設定は1 MB以下にしてください');return text;}
export function decodeFrequencyPlan(text:string,normalize:(model:string)=>string,check:FrequencyValidator){if(new TextEncoder().encode(text).length>1_000_000)throw new Error('周波数解析設定は1 MB以下にしてください');return validateFrequencyPlan(JSON.parse(text),normalize,check);}
const csvQuote=(s:string)=>'"'+(/^\s*[=+\-@]/.test(s)?"'":'')+s.replaceAll('"','""')+'"';
export function frequencyCsv(data:FrequencyResult){
 const discrete=data.sample_time!==undefined,quote=csvQuote;const q=data.request;return '\uFEFF'+[['source','input_component','scope','output_component','omega_rad_s','real','imag','magnitude','magnitude_db','phase_deg','status','relative_residual','error',...(discrete?['sample_time_s','nyquist_rad_s','omega_rad_sample']:[])].map(quote).join(','),...data.points.map(p=>[quote(q.source),q.source_component,quote(q.scope),q.scope_component,p.omega,p.real??'',p.imag??'',p.magnitude??'',p.magnitude_db??'',p.phase_deg??'',quote(p.error?'error':p.magnitude===0?'zero':'ok'),p.relative_residual??'',quote(p.error||''),...(discrete?[data.sample_time!,data.nyquist_omega!,p.omega*data.sample_time!]:[])].join(','))].join('\r\n');
}
export function marginsCsv(data:FrequencyResult){
 if(!data.margins||!data.request.open_loop)throw new Error('開ループ余裕解析の結果がありません');
 const q=data.request,m=data.margins,quote=csvQuote,discrete=data.sample_time!==undefined;
 const prefix=[quote(q.source),q.source_component,quote(q.scope),q.scope_component,q.min_omega,q.max_omega];
 const headers=['source','input_component','scope','output_component','range_min_rad_s','range_max_rad_s','record','omega_rad_s','phase_deg','phase_margin_deg','phase_level_deg','gain_margin_db','gain_margin_ratio','relative_residual','interval_min_rad_s','interval_max_rad_s','note',...(discrete?['sample_time_s','nyquist_rad_s','omega_rad_sample']:[])];
 const rows:Array<Array<string|number>>=[];
 for(const p of m.gain_crossings)rows.push([...prefix,quote('gain_crossing'),p.omega,p.phase_deg,p.phase_margin_deg,'','','',p.relative_residual,'','',quote('PM = 180 + locally unwrapped phase; no stability inference')]);
 for(const p of m.phase_crossings)rows.push([...prefix,quote('phase_crossing'),p.omega,'','',p.phase_level_deg,p.gain_margin_db,p.gain_margin??'',p.relative_residual,'','',quote('Unit negative feedback; no stability inference')]);
 for(const p of m.issues)rows.push([...prefix,quote(p.kind),'','','','','','','',p.min_omega,p.max_omega,quote(p.message)]);
 for(const [name,list] of [['gain_crossing',m.gain_crossings],['phase_crossing',m.phase_crossings]] as const)if(!list.length)rows.push([...prefix,quote(name+'_not_detected'),'','','','','','','','','',quote('No isolated crossing detected in sampled finite range; not infinite margin')]);
 return '\uFEFF'+[headers.map(quote).join(','),...rows.map(r=>[...r,...(discrete?[data.sample_time!,data.nyquist_omega!,typeof r[7]==='number'?r[7]*data.sample_time!:'']:[])].join(','))].join('\r\n');
}
export function frequencyGeometry(data:FrequencyResult,field:'magnitude_db'|'phase_deg'){
 const values=data.points.flatMap(p=>p[field]!==null&&!p.error?[p[field]!]:[]);const lo=values.length?Math.min(...values):0,hi=values.length?Math.max(...values):1;const span=hi-lo,padding=(span<=1e-9*Math.max(1,Math.abs(lo),Math.abs(hi))?1:span)*.08;const min=lo-padding,max=hi+padding;
 const first=Math.log10(data.points[0].omega),last=Math.log10(data.points.at(-1)!.omega);
 if(!Number.isFinite(first)||!Number.isFinite(last)||last<=first)throw new Error('周波数の対数軸を描画できません');
 const x=(omega:number)=>65+710*(Math.log10(omega)-first)/(last-first),y=(v:number)=>175-150*(v-min)/(max-min);
 const segments:string[]=[];let current='';for(const p of data.points){const v=p[field];if(v===null||p.error){if(current)segments.push(current);current='';}else current+=(current?' L':'M')+x(p.omega).toFixed(3)+','+y(v).toFixed(3);}if(current)segments.push(current);
 return {segments,min,max,x,y,empty:!values.length};
}
export function plotFrequency(host:HTMLElement,data:FrequencyResult,field:'magnitude_db'|'phase_deg'){
 const geometry=frequencyGeometry(data,field),ns='http://www.w3.org/2000/svg';
 const make=(tag:string,attrs:Record<string,string>)=>{const el=document.createElementNS(ns,tag);for(const [k,v]of Object.entries(attrs))el.setAttribute(k,v);return el;};
 const svg=make('svg',{viewBox:'0 0 800 215',role:'img','aria-label':field==='magnitude_db'?'Bode 振幅 (dB)':'Bode 位相 (度)'});
 for(let i=0;i<=4;i++){const v=geometry.min+(geometry.max-geometry.min)*i/4,y=geometry.y(v);svg.append(make('line',{x1:'65',x2:'775',y1:String(y),y2:String(y),stroke:'#29374d'}));const text=make('text',{x:'57',y:String(y+4),fill:'#9facc5','text-anchor':'end','font-size':'11'});text.textContent=v.toPrecision(4);svg.append(text);}
 const first=data.request.min_omega,last=data.request.max_omega;for(let i=0;i<=4;i++){const w=10**(Math.log10(first)+(Math.log10(last)-Math.log10(first))*i/4),text=make('text',{x:String(geometry.x(w)),y:'201',fill:'#9facc5','text-anchor':'middle','font-size':'11'});text.textContent=w.toPrecision(3);svg.append(text);}
 for(const d of geometry.segments)svg.append(make('path',{d,fill:'none',stroke:field==='magnitude_db'?'#63e3bc':'#ffa96b','stroke-width':'2'}));
 if(data.margins&&!geometry.empty){
  const levels=field==='magnitude_db'?[0]:Array.from({length:9},(_,i)=>-180+360*(i-4));
  for(const v of levels)if(v>=geometry.min&&v<=geometry.max)svg.append(make('line',{x1:'65',x2:'775',y1:String(geometry.y(v)),y2:String(geometry.y(v)),stroke:'#8eaac5','stroke-dasharray':'4 5','data-threshold':String(v)}));
  for(const [kind,points]of [['gain',data.margins.gain_crossings],['phase',data.margins.phase_crossings]] as const)for(const p of points){const v=kind==='gain'?(field==='magnitude_db'?0:(p as GainCrossing).phase_deg):(field==='magnitude_db'?-(p as PhaseCrossing).gain_margin_db:(p as PhaseCrossing).phase_level_deg);if(v<geometry.min||v>geometry.max)continue;const dot=make('circle',{cx:String(geometry.x(p.omega)),cy:String(geometry.y(v)),r:'4',fill:kind==='gain'?'#63e3bc':'#ffa96b',stroke:'#101828','data-crossing':kind});const title=make('title',{});title.textContent=`${kind==='gain'?'ゲイン交差':'位相交差'} ω=${p.omega.toPrecision(6)} rad/s`;dot.append(title);svg.append(dot);}
 }
 if(geometry.empty){const text=make('text',{x:'400',y:'95',fill:'#9facc5','text-anchor':'middle','font-size':'13'});text.textContent='この範囲に描画可能な値がありません';svg.append(text);}host.replaceChildren(svg);
}
