import {isMatrixSequence,type MatrixReport} from './matrix-comparison';
export interface MatrixCursorPoint {sample:number;component:number;time:number;input:number[];continuous:number;discrete:number;delta:number;position:{kind:'sample_sequence';row:number;cycle:number;rows:number;repeat:boolean;final_hold:boolean}|{kind:'held_step';after_change:boolean}}
export function matrixCursorPoint(report:MatrixReport,sample:number,component:number):MatrixCursorPoint {
 if(!Number.isInteger(sample)||sample<0||sample>=report.times.length||!Number.isInteger(component)||component<0||component>=report.metrics.length)throw new Error('サンプル番号・出力成分が範囲外です');
 const u=report.plan.input,continuous=report.continuous[sample][component],discrete=report.discrete[sample][component];
 return {sample,component,time:report.times[sample],input:[...report.input[sample]],continuous,discrete,delta:discrete-continuous,position:isMatrixSequence(u)?{kind:'sample_sequence',row:u.repeat?sample%u.values.length:Math.min(sample,u.values.length-1),cycle:u.repeat?Math.floor(sample/u.values.length):0,rows:u.values.length,repeat:!!u.repeat,final_hold:!u.repeat&&sample>=u.values.length}:{kind:'held_step',after_change:sample>=u.change_sample}};
}
// Select recorded data, never interpolate values. Equal distances choose the earlier point.
export function nearestMatrixSample(times:number[],time:number):number {
 if(!times.length||!Number.isFinite(time))throw new Error('取得時刻が不正です');
 if(time<=times[0])return 0;if(time>=times[times.length-1])return times.length-1;
 let lo=0,hi=times.length-1;while(hi-lo>1){const middle=Math.floor((lo+hi)/2);if(times[middle]<=time)lo=middle;else hi=middle;}
 return time-times[lo]<=times[hi]-time?lo:hi;
}
