import {generateSampleTable,isTableWave,type InputGeneration} from './sampled-input-generator';
export interface GenerationChannel {component:number;min:number;max:number;duty?:{requested:number;effective:number;on_rows:number;off_rows:number;on_time:number}}
export interface GenerationPlan {values:number[][];rows:number;width:number;sample_time:number;repeat:boolean;table_time:number;last_sample_time:number;period?:{rows:number;time:number;frequency:number};step_time?:number;ramp_end_time?:number;channels:GenerationChannel[]}
export function planGeneratedInput(request:InputGeneration,sample_time:number,repeat:boolean):GenerationPlan {
 const values=generateSampleTable(request),rows=values.length,width=request.width;
 if(!Number.isFinite(sample_time)||sample_time<=0||typeof repeat!=='boolean')throw new Error('生成予定の周期は正の有限値、繰り返しはbooleanが必要です');
 const table_time=rows*sample_time,last_sample_time=(rows-1)*sample_time;
 const periodic=request.shape==='sine'||isTableWave(request.shape),period=periodic?{rows:rows/request.cycles!,time:table_time/request.cycles!,frequency:request.cycles!/table_time}:undefined;
 if(!Number.isFinite(table_time)||!Number.isFinite(last_sample_time)||period&&(!Number.isFinite(period.time)||period.time<=0||!Number.isFinite(period.frequency)||period.frequency<=0))throw new Error('生成予定の時間・周波数が有限の範囲を超えています');
 const channels=Array.from({length:width},(_,j):GenerationChannel=>{
  let min=values[0][j],max=min;for(const row of values){min=Math.min(min,row[j]);max=Math.max(max,row[j]);}
  const channel:GenerationChannel={component:j,min,max};
  if(request.shape==='square'||request.shape==='pulse'){
   const requested=(Array.isArray(request.duty)?request.duty[j]:request.duty===undefined?50:request.duty) as number,periodRows=period!.rows;
   let on_rows=0;for(let r=0;r<periodRows;r++)if(r*100<requested*periodRows)on_rows++;
   channel.duty={requested,effective:100*on_rows/periodRows,on_rows,off_rows:periodRows-on_rows,on_time:on_rows*sample_time};
  }
  return channel;
 });
 return {values,rows,width,sample_time,repeat,table_time,last_sample_time,...(period?{period}:{}),...(request.shape==='step'?{step_time:request.step!*sample_time}:{}),...(request.shape==='ramp'?{ramp_end_time:last_sample_time}:{}),channels};
}
