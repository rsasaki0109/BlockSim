import {validateSampleTable} from './sampled-input-csv';
export type InputShape='constant'|'step'|'ramp'|'sine';
export interface InputGeneration {shape:InputShape;width:number;rows:number;base:unknown;delta?:unknown;step?:number;cycles?:number}
function vector(value:unknown,width:number,label:string):number[] {
 const v=typeof value==='number'?Array(width).fill(value):value;
 if(!Array.isArray(v)||v.length!==width||v.some(x=>typeof x!=='number'||!Number.isFinite(x)))throw new Error(`${label}は有限の数値、または信号幅と同じ長さのJSON配列です`);
 return v as number[];
}
export function generateSampleTable(request:InputGeneration):number[][] {
 const {shape,width,rows}=request,step=request.step??NaN,cycles=request.cycles??NaN;
 if(!Number.isInteger(width)||width<1||width>64)throw new Error('信号幅は1〜64の整数です');
 if(!Number.isInteger(rows)||rows<1||rows>2001)throw new Error('生成行数は1〜2001の整数です');
 if(!['constant','step','ramp','sine'].includes(shape))throw new Error('入力波形の種類が不正です');
 const base=vector(request.base,width,'基準値'),delta=shape==='constant'?Array(width).fill(0):vector(request.delta,width,'変化量');
 if(shape==='step'&&(!Number.isInteger(step)||step<0||step>=rows))throw new Error('切り替え行は0から生成行数−1までの整数です');
 if(shape==='sine'&&(!Number.isInteger(cycles)||cycles<1||cycles>Math.floor(rows/2)))throw new Error('正弦波の周期数は1からfloor(生成行数/2)までの整数です');
 const values=Array.from({length:rows},(_,k)=>{
  const factor=shape==='constant'?0:shape==='step'?(k>=step?1:0):shape==='ramp'?(rows===1?0:k/(rows-1)):Math.sin(2*Math.PI*(cycles*k/rows));
  return base.map((v,j)=>v+delta[j]*factor);
 });
 validateSampleTable(values,width);
 if(new TextEncoder().encode(JSON.stringify(values)).length>1_000_000)throw new Error('生成した入力表JSONは1 MBまでです。行数や桁数を減らしてください');
 return values;
}
