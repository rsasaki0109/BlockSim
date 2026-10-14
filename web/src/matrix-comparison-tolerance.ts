import type {MatrixReport} from './matrix-comparison';
import {matrixRange} from './matrix-comparison-range';
export interface MatrixTolerance {absolute:number;relative:number}
export interface MatrixToleranceSettings extends MatrixTolerance {enabled:boolean}
export interface MatrixToleranceOutput {component:number;passed:boolean;exceedances:number;first_exceedance:number|null;last_exceedance:number|null}
export interface MatrixToleranceAssessment {start_sample:number;end_sample:number;start_time:number;end_time:number;samples:number;passed:boolean;exceedances:number;first_exceedance:{sample:number;component:number}|null;outputs:MatrixToleranceOutput[]}
export const MATRIX_TOLERANCE_CRITERION='abs(discrete - continuous) <= absolute + relative * abs(continuous)';
export function matrixTolerancePolicy(value:unknown):MatrixTolerance {
 if(!value||typeof value!=='object'||Array.isArray(value))throw new Error('許容差は非負の有限値で指定してください');
 const v=value as MatrixTolerance;
 if(typeof v.absolute!=='number'||!Number.isFinite(v.absolute)||v.absolute<0||typeof v.relative!=='number'||!Number.isFinite(v.relative)||v.relative<0)throw new Error('許容差は非負の有限値で指定してください');
 return {absolute:v.absolute,relative:v.relative};
}
export function matrixToleranceAccepts(continuous:number,discrete:number,policy:MatrixTolerance):boolean {
 const {absolute,relative}=matrixTolerancePolicy(policy);
 if(!Number.isFinite(continuous)||!Number.isFinite(discrete))throw new Error('取得値が有限値ではありません');
 const delta=Math.abs(discrete-continuous),bound=absolute+relative*Math.abs(continuous);
 if(Number.isFinite(delta)&&Number.isFinite(bound))return delta<=bound;
 // Normalize only when subtraction or the tolerance bound overflows.
 // Do not turn Infinity <= Infinity into an automatic pass.
 const scale=Math.max(Math.abs(continuous),Math.abs(discrete),absolute);
 return Math.abs(discrete/scale-continuous/scale)<=absolute/scale+relative*(Math.abs(continuous)/scale);
}
export function matrixTolerance(report:MatrixReport,start:number,end:number,policy:MatrixTolerance):MatrixToleranceAssessment {
 const p=matrixTolerancePolicy(policy),range=matrixRange(report,start,end);
 const outputs:MatrixToleranceOutput[]=report.metrics.map((_,component)=>({component,passed:true,exceedances:0,first_exceedance:null,last_exceedance:null}));
 let first:MatrixToleranceAssessment['first_exceedance']=null;
 for(let sample=start;sample<=end;sample++)for(const output of outputs)if(!matrixToleranceAccepts(report.continuous[sample][output.component],report.discrete[sample][output.component],p)){
  output.passed=false;output.exceedances++;output.first_exceedance??=sample;output.last_exceedance=sample;first??={sample,component:output.component};
 }
 return {start_sample:start,end_sample:end,start_time:range.start_time,end_time:range.end_time,samples:range.samples,passed:outputs.every(o=>o.passed),exceedances:outputs.reduce((s,o)=>s+o.exceedances,0),first_exceedance:first,outputs};
}
function checkSize(bytes:number){if(bytes>100_000)throw new Error('許容差設定は100 KBまでです');}
export function decodeMatrixTolerance(text:string):MatrixToleranceSettings {
 checkSize(text.length);checkSize(new TextEncoder().encode(text).length);const raw=JSON.parse(text);
 if(!raw||typeof raw!=='object'||Array.isArray(raw)||raw.format!=='blocksim.matrix-comparison-tolerance-settings'||raw.version!==1||typeof raw.enabled!=='boolean'||Object.keys(raw).length!==5||Object.keys(raw).some(k=>!['format','version','enabled','absolute','relative'].includes(k)))throw new Error('許容差設定の形式が違います');
 return {enabled:raw.enabled,...matrixTolerancePolicy(raw)};
}
export function matrixToleranceSettingsJson(settings:MatrixToleranceSettings):string {
 return JSON.stringify({format:'blocksim.matrix-comparison-tolerance-settings',version:1,...decodeMatrixTolerance(JSON.stringify({format:'blocksim.matrix-comparison-tolerance-settings',version:1,...settings}))},null,2);
}
export function matrixToleranceJson(report:MatrixReport,start:number,end:number,policy:MatrixTolerance):string {
 const checked=matrixTolerancePolicy(policy),assessment=matrixTolerance(report,start,end,checked);
 return JSON.stringify({format:'blocksim.matrix-comparison-tolerance-result',version:1,criterion:MATRIX_TOLERANCE_CRITERION,tolerance:checked,plan:report.plan,conversion:report.conversion,assessment,sample_indices:Array.from({length:assessment.samples},(_,i)=>start+i),times:report.times.slice(start,end+1),input:report.input.slice(start,end+1),continuous:report.continuous.slice(start,end+1),discrete:report.discrete.slice(start,end+1)},null,2);
}
