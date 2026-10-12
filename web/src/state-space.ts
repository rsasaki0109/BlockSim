import {type GraphModel} from './graph-model';
import {type PolynomialRoots,type Stability} from './pole-zero';
export type StateSpaceDomain='continuous'|'discrete';
export interface StateSpaceRequest {source:string;source_component:number;scope:string;scope_component:number}
export interface StateInfo {block:string;label:string;component:number;state:number;description:string}
export interface StateSpaceResult {request:StateSpaceRequest;check:{source_label:string;scope_label:string;variables:number;states:number;source_related:boolean};states:StateInfo[];a:number[][];b:number[];c:number[];d:number;characteristic:number[]|null;pole_scale:number;characteristic_residual:number|null;poles:PolynomialRoots;matrix_residuals:number[];stability:Stability & {outside_unit_circle_poles?:number|null};sample_time?:number;method:string}
export interface StateSpacePlan {format:'blocksim.state-space';version:1;domain?:StateSpaceDomain;model:GraphModel;request:StateSpaceRequest}
export type StateSpaceValidator=(model:string,request:string,domain?:StateSpaceDomain)=>string;
function object(value:unknown,keys:string[]){if(!value||typeof value!=='object'||Array.isArray(value)||Object.keys(value).length!==keys.length||keys.some(k=>!Object.hasOwn(value,k)))throw new Error('状態空間の保存形式が不正です');return value as Record<string,unknown>;}
export function validateStateSpacePlan(value:unknown,normalize:(s:string)=>string,check:StateSpaceValidator):StateSpacePlan {
 const hasDomain=!!value&&typeof value==='object'&&Object.hasOwn(value,'domain');const p=object(value,['format','version','model','request',...(hasDomain?['domain']:[])]);if(hasDomain&&p.domain!=='continuous'&&p.domain!=='discrete')throw new Error('状態空間の解析領域が不正です');if(p.format!=='blocksim.state-space'||p.version!==1)throw new Error('未対応の状態空間解析形式です');const q=object(p.request,['source','source_component','scope','scope_component']);if(typeof q.source!=='string'||!q.source||typeof q.scope!=='string'||!q.scope||!Number.isSafeInteger(q.source_component)||!Number.isSafeInteger(q.scope_component)||(q.source_component as number)<0||(q.scope_component as number)<0)throw new Error('信号源・Scopeと0以上の整数の成分を指定してください');
 const json=JSON.stringify(p.model);if(new TextEncoder().encode(json).length>1_000_000)throw new Error('モデルは1 MB以下にしてください');const model=JSON.parse(normalize(json)) as GraphModel;const request={...q} as unknown as StateSpaceRequest;check(JSON.stringify(model),JSON.stringify(request),p.domain==='discrete'?'discrete':'continuous');return {format:'blocksim.state-space',version:1,...(p.domain==='discrete'?{domain:'discrete' as const}:{}),model,request};
}
export function encodeStateSpacePlan(p:StateSpacePlan){const text=JSON.stringify(p);if(new TextEncoder().encode(text).length>1_000_000)throw new Error('状態空間設定は1 MB以下にしてください');return text;}
export function decodeStateSpacePlan(text:string,normalize:(s:string)=>string,check:StateSpaceValidator){if(new TextEncoder().encode(text).length>1_000_000)throw new Error('状態空間設定は1 MB以下にしてください');return validateStateSpacePlan(JSON.parse(text),normalize,check);}
const quote=(s:string)=>'"'+(/^\s*[=+\-@]/.test(s)?"'":'')+s.replaceAll('"','""')+'"';
export function stateSpaceCsv(r:StateSpaceResult){
 const discrete=r.sample_time!==undefined;const q=r.request,prefix=[quote(q.source),q.source_component,quote(q.scope),q.scope_component,quote(r.stability.classification)];const rows:Array<Array<string|number>>=[];
 // Every row uses the executed request, never the current form's values.
 rows.push([...prefix,quote('assessment'),'','','','','','',quote(r.stability.notes.join(' | ')),(discrete?r.stability.outside_unit_circle_poles:r.stability.right_half_plane_poles)??'']);
 for(const [i,s]of r.states.entries())rows.push([...prefix,quote('state'),i,'','','','','',quote(`${s.block}[${s.component}].state[${s.state}] · ${s.label} · ${s.description}`),'']);
 for(const [i,row]of r.a.entries())for(const [j,value]of row.entries())rows.push([...prefix,quote('A'),i,j,value,'','','',quote(discrete?'x[k+1]=Ax[k]+Bu[k]':'xdot=Ax+Bu'),'']);
 for(const [name,values]of [['B',r.b],['C',r.c]] as const)for(const [i,value]of values.entries())rows.push([...prefix,quote(name),name==='B'?i:0,name==='B'?0:i,value,'','','',quote('selected source/Scope components'),'']);
 rows.push([...prefix,quote('D'),0,0,r.d,'','','',quote('direct feedthrough'),'']);
 for(const [i,p]of r.poles.roots.entries())rows.push([...prefix,quote('eigenvalue'),i,'','',p.real,p.imag,p.relative_residual,quote(`matrix determinant residual=${r.matrix_residuals[i]} | ${r.poles.warnings.join(' | ')}`),'']);
 if(!r.poles.roots.length)rows.push([...prefix,quote('eigenvalue_set'),'','','','','','',quote(r.poles.status+(r.check.states===0?' · no dynamic states':' · '+r.poles.warnings.join(' | '))),'']);
 rows.push([...prefix,quote('characteristic'),'','','','','',r.characteristic_residual??'',quote(`det(zI-A/scale), scale=${r.pole_scale}, coefficients=${JSON.stringify(r.characteristic)}`),'']);
 if(discrete)rows.push([...prefix,quote('sample_time'),'','',r.sample_time!,'','','',quote('seconds; states before sample update, output after PID update and before next Delay commit'),'']);
 const headers=['source','input_component','scope','output_component','stability','record','row_or_index','column','value',discrete?'real_dimensionless':'real_per_s',discrete?'imag_dimensionless':'imag_rad_s','relative_residual','note',discrete?'outside_unit_circle_poles':'right_half_plane_poles'];
 return '\uFEFF'+[headers.map(quote).join(','),...rows.map(x=>x.join(','))].join('\r\n');
}
