import {type GraphModel} from './graph-model';
import {type FrequencyPoint} from './frequency';
import {type StateInfo,type StateSpaceStructure,type StateSpaceResult} from './state-space';
export interface Channel {block:string;component:number}
export interface MimoRequest {inputs:Channel[];outputs:Channel[];discrete:boolean;min_omega:number;max_omega:number;points:number}
export interface MimoPlan {format:'blocksim.mimo';version:1;model:GraphModel;request:MimoRequest}
export interface MimoResult {request:MimoRequest;check:{variables:number;states:number;inputs:number;outputs:number;sample_time:number|null};states:StateInfo[];a:number[][];b:number[][];c:number[][];d:number[][];poles:StateSpaceResult['poles'];matrix_residuals:number[];stability:StateSpaceResult['stability'];structure:StateSpaceStructure;responses:{input:number;output:number;points:FrequencyPoint[]}[];method:string}
function object(x:unknown,keys:string[]):Record<string,unknown>{if(!x||typeof x!=='object'||Array.isArray(x)||Object.keys(x).length!==keys.length||keys.some(k=>!Object.hasOwn(x,k)))throw Error('MIMOの保存形式が不正です');return x as Record<string,unknown>;}
export function validateMimoPlan(value:unknown,normalize:(s:string)=>string,check:(m:string,q:string)=>string):MimoPlan{
 const p=object(value,['format','version','model','request']);if(p.format!=='blocksim.mimo'||p.version!==1)throw Error('未対応のMIMO形式です');const q=object(p.request,['inputs','outputs','discrete','min_omega','max_omega','points']);
 for(const name of ['inputs','outputs']){const channels=q[name];if(!Array.isArray(channels)||channels.length<1||channels.length>8)throw Error('入力・出力はそれぞれ1〜8成分です');const seen=new Set<string>();for(const ch of channels){const c=object(ch,['block','component']);if(typeof c.block!=='string'||!c.block||!Number.isSafeInteger(c.component)||(c.component as number)<0)throw Error('信号と0以上の整数成分を選択してください');const key=JSON.stringify([c.block,c.component]);if(seen.has(key))throw Error('同じ成分の重複は指定できません');seen.add(key);}}
 if(typeof q.discrete!=='boolean'||typeof q.min_omega!=='number'||typeof q.max_omega!=='number'||q.min_omega<1e-9||q.max_omega>1e9||!Number.isFinite(q.min_omega)||!Number.isFinite(q.max_omega)||q.max_omega<=q.min_omega||!Number.isSafeInteger(q.points)||(q.points as number)<2||(q.points as number)>512)throw Error('周波数範囲と2〜512点を指定してください');
 const model=JSON.parse(normalize(JSON.stringify(p.model))) as GraphModel;const request=structuredClone(q) as unknown as MimoRequest;check(JSON.stringify(model),JSON.stringify(request));return {format:'blocksim.mimo',version:1,model,request};
}
export function encodeMimoPlan(p:MimoPlan){const s=JSON.stringify(p);if(new TextEncoder().encode(s).length>1_000_000)throw Error('MIMO設定は1 MB以下にしてください');return s;}
export function decodeMimoPlan(text:string,normalize:(s:string)=>string,check:(m:string,q:string)=>string){if(new TextEncoder().encode(text).length>1_000_000)throw Error('MIMO設定は1 MB以下にしてください');return validateMimoPlan(JSON.parse(text),normalize,check);}
const quote=(s:string)=>'"'+(/^\s*[=+\-@]/.test(s)?"'":'')+s.replaceAll('"','""')+'"';
export function mimoCsv(r:MimoResult){const context=quote(JSON.stringify(r.request)),rows:Array<Array<number|string>>=[];const add=(kind:string,i:number|string,j:number|string,value:number|string='',omega:number|string='',real:number|string='',imag:number|string='',db:number|string='',phase:number|string='',residual:number|string='',note='')=>rows.push([context,quote(kind),i,j,value,omega,real,imag,db,phase,residual,quote(note)]);
 for(const name of ['inputs','outputs'] as const)r.request[name].forEach((s,i)=>add(name,i,s.component,'','','','','','','',s.block));
 r.states.forEach((s,i)=>add('state',i,s.component,'','','','','','','',`${s.block}.state[${s.state}] · ${s.description}`));
 for(const name of ['a','b','c','d'] as const)r[name].forEach((row,i)=>row.forEach((v,j)=>add(name.toUpperCase(),i,j,v)));
 r.poles.roots.forEach((p,i)=>add('eigenvalue',i,'','','',p.real,p.imag,'','',p.relative_residual,`matrix residual=${r.matrix_residuals[i]}`));add('assessment','','','','','','','','','',`${r.stability.classification} | ${r.poles.status} | ${r.stability.notes.join(' | ')}`);
 for(const name of ['controllability','observability'] as const){const s=r.structure[name];add(name+'_rank',s.dimension,'',s.numerical_rank??'','','','','','',s.tolerance,`${s.status} | ${s.notes.join(' | ')}`);}
 if(r.check.sample_time!==null)add('sample_time','','',r.check.sample_time);
 for(const pair of r.responses)for(const p of pair.points)add('frequency',pair.output,pair.input,p.magnitude??'',p.omega,p.real??'',p.imag??'',p.magnitude_db??'',p.phase_deg??'',p.relative_residual??'',p.error??(p.magnitude===0?'zero response; dB and phase undefined':''));
 return '\uFEFF'+[['request','record','row_or_output','column_or_input','value_or_magnitude','omega_rad_s','real','imag','magnitude_db','phase_deg','relative_residual','note'].map(quote).join(','),...rows.map(r=>r.join(','))].join('\r\n');
}
