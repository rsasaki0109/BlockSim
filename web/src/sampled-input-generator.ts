import {validateSampleTable} from './sampled-input-csv';
export type InputShape='constant'|'step'|'ramp'|'sine'|'square'|'triangle'|'pulse';
export interface InputGeneration {shape:InputShape;width:number;rows:number;base:unknown;delta?:unknown;step?:number;cycles?:number;phase?:unknown;duty?:unknown}
export function isTableWave(shape:unknown){return shape==='square'||shape==='triangle'||shape==='pulse';}
function vector(value:unknown,width:number,label:string):number[] {
 const v=typeof value==='number'?Array(width).fill(value):value;
 if(!Array.isArray(v)||v.length!==width||v.some(x=>typeof x!=='number'||!Number.isFinite(x)))throw new Error(`${label}は有限の数値、または信号幅と同じ長さのJSON配列です`);
 return v as number[];
}
export function generateSampleTable(request:InputGeneration):number[][] {
 const {shape,width,rows}=request,step=request.step??NaN,cycles=request.cycles??NaN;
 if(!Number.isInteger(width)||width<1||width>64)throw new Error('信号幅は1〜64の整数です');
 if(!Number.isInteger(rows)||rows<1||rows>2001)throw new Error('生成行数は1〜2001の整数です');
 if(!['constant','step','ramp','sine','square','triangle','pulse'].includes(shape))throw new Error('入力波形の種類が不正です');
 const base=vector(request.base,width,'基準値'),delta=shape==='constant'?Array(width).fill(0):vector(request.delta,width,'変化量');
 const phase=shape==='sine'?vector(request.phase===undefined?0:request.phase,width,'初期位相'):[];
 const duty=shape==='square'||shape==='pulse'?vector(request.duty===undefined?50:request.duty,width,'デューティ比'):[];
 if(duty.some(v=>v<0||v>100))throw new Error('デューティ比は0〜100%の範囲です');
 if(phase.some(v=>Math.abs(v)>2*Math.PI))throw new Error('初期位相は−2π〜2π radの範囲です');
 if(shape==='step'&&(!Number.isInteger(step)||step<0||step>=rows))throw new Error('切り替え行は0から生成行数−1までの整数です');
 if(shape==='sine'&&(!Number.isInteger(cycles)||cycles<1||cycles>Math.floor(rows/2)))throw new Error('正弦波の周期数は1からfloor(生成行数/2)までの整数です');
 if(isTableWave(shape)&&(!Number.isInteger(cycles)||cycles<1||cycles>Math.floor(rows/2)||rows%cycles!==0))throw new Error('矩形波・三角波・パルス列の周期数は生成行数を割り切る整数です（1周期は2行以上）');
 const values=Array.from({length:rows},(_,k)=>{
  if(isTableWave(shape)){
   const period=rows/cycles,row=k%period;
   return base.map((v,j)=>{const factor=shape==='triangle'?1-4*Math.abs(row/period-.5):row*100<duty[j]*period?1:shape==='square'?-1:0;return v+delta[j]*factor;});
  }
  const factor=shape==='constant'?0:shape==='step'?(k>=step?1:0):shape==='ramp'?(rows===1?0:k/(rows-1)):Math.sin(2*Math.PI*(cycles*k/rows));
  return base.map((v,j)=>v+delta[j]*(shape==='sine'&&phase[j]!==0?Math.sin(2*Math.PI*(cycles*k/rows)+phase[j]):factor));
 });
 validateSampleTable(values,width);
 if(new TextEncoder().encode(JSON.stringify(values)).length>1_000_000)throw new Error('生成した入力表JSONは1 MBまでです。行数や桁数を減らしてください');
 return values;
}
