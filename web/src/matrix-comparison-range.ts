import {differenceMetrics,type DifferenceMetrics} from './graph-comparison';
import {matrixComparisonCsv,type MatrixReport} from './matrix-comparison';
export interface MatrixRange {start_sample:number;end_sample:number;start_time:number;end_time:number;samples:number;metrics:(DifferenceMetrics|null)[]}
export function matrixRange(report:MatrixReport,start:number,end:number):MatrixRange {
 if(!Number.isInteger(start)||!Number.isInteger(end)||start<0||end<start||end>=report.times.length)throw new Error('比較区間は0〜最終点の整数で、開始≦終了です');
 const times=report.times.slice(start,end+1);
 return {start_sample:start,end_sample:end,start_time:times[0],end_time:times.at(-1)!,samples:times.length,metrics:report.metrics.map((_,j)=>differenceMetrics(times,report.discrete.slice(start,end+1).map(row=>row[j]),report.continuous.slice(start,end+1).map(row=>row[j])))};
}
export function matrixRangeJson(report:MatrixReport,start:number,end:number):string {
 const range=matrixRange(report,start,end);if(range.metrics.some(m=>m===null))throw new Error('区間の差分指標が有限の範囲を超えています');
 return JSON.stringify({format:'blocksim.matrix-comparison-range',version:1,plan:report.plan,conversion:report.conversion,range,sample_indices:Array.from({length:range.samples},(_,i)=>start+i),times:report.times.slice(start,end+1),input:report.input.slice(start,end+1),continuous:report.continuous.slice(start,end+1),discrete:report.discrete.slice(start,end+1)},null,2);
}
export function matrixRangeCsv(report:MatrixReport,start:number,end:number):string {
 const range=matrixRange(report,start,end);if(range.metrics.some(m=>m===null))throw new Error('区間の差分指標が有限の範囲を超えています');
 const [header,...rows]=matrixComparisonCsv(report).split('\r\n'),width=report.metrics.length;
 return ['range_start_sample,range_end_sample,range_sample_index,'+header,...rows.slice(start*width,(end+1)*width).map((row,i)=>[start,end,start+Math.floor(i/width),row].join(','))].join('\r\n');
}
