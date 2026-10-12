import {type GraphModel} from './graph-model';
export type PoleZeroMode='block'|'unit_negative_feedback';
export interface PoleZeroRequest {block:string;mode:PoleZeroMode}
export interface PolynomialRoot {real:number;imag:number;relative_residual:number}
export interface PolynomialRoots {status:'verified_residual'|'identically_zero'|'unresolved';degree:number|null;roots:PolynomialRoot[];reconstruction_error:number|null;warnings:string[]}
export interface Stability {classification:'stable'|'unstable'|'boundary_or_uncertain'|'indeterminate';right_half_plane_poles:number|null;tolerance:number;method:string;notes:string[]}
export interface PoleZeroResult {request:PoleZeroRequest;check:{label:string;width:number;order:number};numerator:number[];denominator:number[];poles:PolynomialRoots;zeros:PolynomialRoots;stability:Stability;method:string}
export interface PoleZeroPlan {format:'blocksim.pole-zero';version:1;model:GraphModel;request:PoleZeroRequest}
export type PoleZeroValidator=(model:string,request:string)=>string;
export const stabilityLabels:Record<Stability['classification'],string>={stable:'漸近安定（数値判定）',unstable:'不安定（数値判定）',boundary_or_uncertain:'境界／近傍（漸近安定と判定せず）',indeterminate:'判定不能'};
export function transferBlocks(model:GraphModel,prefix:string[]=[]):Array<{id:string;label:string;width:number}>{return model.blocks.flatMap(b=>b.params.kind==='transfer_function'?[{id:[...prefix,b.id].join('/'),label:b.label,width:b.width}]:b.params.kind==='subsystem'?transferBlocks(b.params.model as GraphModel,[...prefix,b.id]):[]);}
function record(value:unknown,keys:string[]){if(!value||typeof value!=='object'||Array.isArray(value)||Object.keys(value).length!==keys.length||keys.some(k=>!Object.hasOwn(value,k)))throw new Error('極・零点の保存形式が不正です');return value as Record<string,unknown>;}
export function validatePoleZeroPlan(value:unknown,normalize:(s:string)=>string,check:PoleZeroValidator):PoleZeroPlan{
 const p=record(value,['format','version','model','request']);if(p.format!=='blocksim.pole-zero'||p.version!==1)throw new Error('未対応の極・零点解析形式です');const q=record(p.request,['block','mode']);if(typeof q.block!=='string'||!q.block||!['block','unit_negative_feedback'].includes(q.mode as string))throw new Error('伝達関数と解析方式を指定してください');
 const json=JSON.stringify(p.model);if(new TextEncoder().encode(json).length>1_000_000)throw new Error('モデルは1 MB以下にしてください');const model=JSON.parse(normalize(json)) as GraphModel;const request={block:q.block,mode:q.mode} as PoleZeroRequest;check(JSON.stringify(model),JSON.stringify(request));return {format:'blocksim.pole-zero',version:1,model,request};
}
export function encodePoleZeroPlan(p:PoleZeroPlan){const text=JSON.stringify(p);if(new TextEncoder().encode(text).length>1_000_000)throw new Error('極・零点の設定は1 MB以下にしてください');return text;}
export function decodePoleZeroPlan(text:string,normalize:(s:string)=>string,check:PoleZeroValidator){if(new TextEncoder().encode(text).length>1_000_000)throw new Error('極・零点の設定は1 MB以下にしてください');return validatePoleZeroPlan(JSON.parse(text),normalize,check);}
const quote=(s:string)=>'"'+(/^\s*[=+\-@]/.test(s)?"'":'')+s.replaceAll('"','""')+'"';
export function poleZeroCsv(r:PoleZeroResult){
 const prefix=[quote(r.request.block),quote(r.request.mode),quote(r.stability.classification),r.stability.right_half_plane_poles??''];const polynomials=[quote(JSON.stringify(r.numerator)),quote(JSON.stringify(r.denominator))];
 const rows:Array<Array<string|number>>=[];
 rows.push([...prefix,quote('assessment'),'','','',quote(r.stability.classification),quote(r.stability.notes.join(' | ')),...polynomials]);
 for(const [name,group] of [['pole',r.poles],['zero',r.zeros]] as const){for(const p of group.roots)rows.push([...prefix,quote(name),p.real,p.imag,p.relative_residual,quote(group.status),quote(group.warnings.join(' | ')),...polynomials]);if(!group.roots.length)rows.push([...prefix,quote(name+'_set'),'','','',quote(group.status),quote(group.warnings.join(' | ')||'No finite roots (constant polynomial)'),...polynomials]);}
 return '\uFEFF'+[['block','mode','stability','right_half_plane_poles','record','real_per_s','imag_rad_s','relative_residual','status','note','numerator','denominator'].map(quote).join(','),...rows.map(x=>x.join(','))].join('\r\n');
}
export function poleZeroGeometry(r:{poles:{roots:PolynomialRoot[]};zeros:{roots:PolynomialRoot[]}}){
 const roots=[...r.poles.roots,...r.zeros.roots];if(roots.some(p=>![p.real,p.imag].every(Number.isFinite)))throw new Error('極・零点の座標が有限値ではありません');
 const extent=Math.max(1,...roots.flatMap(p=>[Math.abs(p.real),Math.abs(p.imag)]));
 // Divide before multiplying, including poles near the finite floating-point limit.
 const x=(v:number)=>300+125*(v/extent),y=(v:number)=>165-125*(v/extent);
 return {extent,x,y,empty:!roots.length};
}
export function plotPoleZero(host:HTMLElement,r:{poles:{roots:PolynomialRoot[]};zeros:{roots:PolynomialRoot[]}},discrete=false){
 const g=poleZeroGeometry(r),ns='http://www.w3.org/2000/svg';const make=(tag:string,attrs:Record<string,string>)=>{const e=document.createElementNS(ns,tag);for(const [k,v]of Object.entries(attrs))e.setAttribute(k,v);return e;};
 const svg=make('svg',{viewBox:'0 0 600 340',role:'img','aria-label':discrete?'離散内部状態の極（複素 z 平面・単位円）':'連続伝達関数の極・零点（複素 s 平面）'});
 if(discrete)svg.append(make('circle',{cx:'300',cy:'165',r:String(125/g.extent),fill:'#14362d',opacity:'.7',stroke:'#63e3bc','stroke-width':'1.5','data-unit-circle':'true'}));else {svg.append(make('rect',{x:'150',y:'15',width:'150',height:'300',fill:'#14362d',opacity:'.5'}));svg.append(make('rect',{x:'300',y:'15',width:'150',height:'300',fill:'#48232d',opacity:'.35'}));}
 for(let i=-2;i<=2;i++){const v=g.extent*i/2;const x=g.x(v),y=g.y(v);svg.append(make('line',{x1:String(x),x2:String(x),y1:'15',y2:'315',stroke:i===0?'#9facc5':'#29374d'}));svg.append(make('line',{x1:'150',x2:'450',y1:String(y),y2:String(y),stroke:i===0?'#9facc5':'#29374d'}));const tx=make('text',{x:String(x),y:'331',fill:'#9facc5','text-anchor':'middle','font-size':'11'});tx.textContent=v.toPrecision(4);svg.append(tx);const ty=make('text',{x:'144',y:String(y+4),fill:'#9facc5','text-anchor':'end','font-size':'11'});ty.textContent=v.toPrecision(4);svg.append(ty);}
 for(const [kind,group]of [['pole',r.poles],['zero',r.zeros]] as const)for(const p of group.roots){const x=g.x(p.real),y=g.y(p.imag);const e=kind==='pole'?make('path',{d:`M${x-5},${y-5}L${x+5},${y+5}M${x-5},${y+5}L${x+5},${y-5}`,stroke:'#ffa96b','stroke-width':'2.5','data-root':kind}):make('circle',{cx:String(x),cy:String(y),r:'7',stroke:'#63e3bc','stroke-width':'2',fill:'none','data-root':kind});const title=make('title',{});title.textContent=`${kind==='pole'?'極':'零点'} ${p.real.toPrecision(7)} ${p.imag<0?'−':'+'} j${Math.abs(p.imag).toPrecision(7)}`;e.append(title);svg.append(e);}
 const axis=make('text',{x:'460',y:'165',fill:'#9facc5','font-size':'12'});axis.textContent=discrete?'Re(z) [無次元]':'Re(s) [1/s]';svg.append(axis);const imaginary=make('text',{x:'310',y:'12',fill:'#9facc5','font-size':'12'});imaginary.textContent=discrete?'Im(z) [無次元]':'Im(s) [rad/s]';svg.append(imaginary);
 if(g.empty){const text=make('text',{x:'300',y:'90',fill:'#9facc5','text-anchor':'middle','font-size':'12'});text.textContent='表示できる有限の根がありません';svg.append(text);}host.replaceChildren(svg);
}
