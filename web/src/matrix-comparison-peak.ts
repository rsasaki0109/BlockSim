import type {MatrixReport} from './matrix-comparison';
export interface MatrixPeak {sample:number;component:number;time:number;delta:number;maxAbs:number}
// Compare raw stored deltas. Ties prefer the first sample, then the lowest output.
export function matrixPeak(report:MatrixReport,start:number,end:number,component?:number):MatrixPeak {
 if(!Number.isInteger(start)||!Number.isInteger(end)||start<0||end<start||end>=report.times.length)throw new Error('比較区間は0〜最終点の整数で、開始≦終了です');
 const width=report.metrics.length;
 if(!width||component!==undefined&&(!Number.isInteger(component)||component<0||component>=width))throw new Error('出力成分が範囲外です');
 let peak:MatrixPeak|undefined;
 for(let sample=start;sample<=end;sample++)for(let j=component??0;j<(component===undefined?width:component+1);j++){
  const delta=report.discrete[sample][j]-report.continuous[sample][j],maxAbs=Math.abs(delta);
  if(!Number.isFinite(delta))throw new Error('差分が有限の範囲を超えています');
  if(!peak||maxAbs>peak.maxAbs)peak={sample,component:j,time:report.times[sample],delta,maxAbs};
 }
 return peak!;
}
