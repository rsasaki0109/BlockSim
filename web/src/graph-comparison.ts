import { resultCsv, type GraphModel, type GraphResult } from './graph-model';

export interface RunSnapshot { model: GraphModel; result: GraphResult }
export interface DifferenceMetrics { maxAbs: number; rms: number; final: number; area: number }
export interface ComparisonRow {
  key: string; id: string; component: number; label: string; referenceLabel: string;
  metrics: DifferenceMetrics|null; status: 'ok'|'time_grid'|'overflow';
}
export interface ComparisonReport { sameTimes: boolean; rows: ComparisonRow[]; notices: string[] }
export function snapshotRun(model:GraphModel,result:GraphResult):RunSnapshot { return structuredClone({model,result}); }
export function sameTimeGrid(a:number[],b:number[]) { return a.length===b.length&&a.every((t,i)=>t===b[i]); }
export function sameTraceWidth(a:GraphResult['traces'][number],b:GraphResult['traces'][number]) { return a.values[0].length===b.values[0].length; }

// Normalize before squaring/summing so finite, large differences do not overflow
// merely because their square exceeds f64. Real out-of-range differences remain errors.
export function differenceMetrics(times:number[],current:number[],reference:number[]):DifferenceMetrics|null {
  let maxAbs=0,final=0;
  for(let i=0;i<times.length;i++){
    const delta=current[i]-reference[i];if(!Number.isFinite(delta))return null;
    maxAbs=Math.max(maxAbs,Math.abs(delta));final=delta;
  }
  if(maxAbs===0)return {maxAbs:0,rms:0,final:0,area:0};
  let square=0,area=0,previous=0;
  for(let i=0;i<times.length;i++){
    const normalized=Math.abs((current[i]-reference[i])/maxAbs);square+=normalized*normalized;
    if(i)area+=(previous+normalized)/2*(times[i]-times[i-1]);previous=normalized;
  }
  const metrics={maxAbs,rms:maxAbs*Math.sqrt(square/times.length),final,area:maxAbs*area};
  return Object.values(metrics).every(Number.isFinite)?metrics:null;
}
export function compareResults(current:GraphResult,reference:GraphResult):ComparisonReport {
  const sameTimes=sameTimeGrid(current.times,reference.times),rows:ComparisonRow[]=[],notices:string[]=[];
  if(!sameTimes)notices.push('記録時刻が異なるため、数値差分は比較できません。補間は行いません。波形はそれぞれの時刻で表示します。');
  const previous=new Map(reference.traces.map(t=>[t.id,t])),ids=new Set(current.traces.map(t=>t.id));
  for(const trace of current.traces){
    const baseline=previous.get(trace.id);
    if(!baseline){notices.push(`${trace.id}: 基準にない Scope です。`);continue;}
    if(!sameTraceWidth(trace,baseline)){notices.push(`${trace.id}: 信号幅が異なります（基準 ${baseline.values[0].length} → 現行 ${trace.values[0].length}）。`);continue;}
    for(let component=0;component<trace.values[0].length;component++){
      const metrics=sameTimes?differenceMetrics(current.times,trace.values.map(v=>v[component]),baseline.values.map(v=>v[component])):null;
      rows.push({key:`${trace.id}:${component}`,id:trace.id,component,label:trace.label,referenceLabel:baseline.label,metrics,status:!sameTimes?'time_grid':metrics?'ok':'overflow'});
    }
  }
  for(const trace of reference.traces)if(!ids.has(trace.id))notices.push(`${trace.id}: 現行にない Scope です。`);
  return {sameTimes,rows,notices};
}
export function differenceCsv(current:GraphResult,reference:GraphResult,report:ComparisonReport):string {
  if(!report.rows.length||report.rows.some(r=>r.status!=='ok'))throw new Error('同じ記録時刻・Scope・信号幅で、有限の差分がある場合に書き出せます');
  const baseline=new Map(reference.traces.map(t=>[t.id,t]));
  const paired=new Set(report.rows.map(row=>row.id));
  return resultCsv({times:current.times,solver:'difference',traces:current.traces.filter(t=>paired.has(t.id)).map(t=>({
    id:t.id,label:`Δ ${t.label}`,values:t.values.map((v,i)=>v.map((value,c)=>value-baseline.get(t.id)!.values[i][c])),
  }))});
}
