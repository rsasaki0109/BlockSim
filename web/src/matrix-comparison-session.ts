import {checkMatrixResultSize,decodeMatrixResult} from './matrix-comparison-file';
import type {MatrixReport} from './matrix-comparison';
export interface MatrixView {sample:number;component:number;start_sample:number;end_sample:number}
export function matrixView(report:MatrixReport,value:unknown):MatrixView {
 if(!value||typeof value!=='object'||Array.isArray(value))throw new Error('比較の表示状態が不正です');const v=value as MatrixView,keys=['sample','component','start_sample','end_sample'];
 if(Object.keys(v).length!==4||Object.keys(v).some(k=>!keys.includes(k))||keys.some(k=>!Number.isInteger(v[k as keyof MatrixView]))||v.sample<0||v.sample>=report.times.length||v.component<0||v.component>=report.metrics.length||v.start_sample<0||v.end_sample<v.start_sample||v.end_sample>=report.times.length)throw new Error('カーソル・出力・比較区間が範囲外です');
 return {sample:v.sample,component:v.component,start_sample:v.start_sample,end_sample:v.end_sample};
}
export function matrixSessionJson(report:MatrixReport,view:MatrixView):string {
 const checked=matrixView(report,view),text=JSON.stringify({format:'blocksim.matrix-comparison-session',version:1,report:{format:'blocksim.matrix-comparison-result',version:1,...report},view:checked},null,2);checkMatrixResultSize(text.length);checkMatrixResultSize(new TextEncoder().encode(text).length);return text;
}
export function decodeMatrixFile(text:string):{report:MatrixReport;view?:MatrixView} {
 checkMatrixResultSize(text.length);checkMatrixResultSize(new TextEncoder().encode(text).length);const raw=JSON.parse(text) as Record<string,unknown>;
 if(raw?.format!=='blocksim.matrix-comparison-session')return {report:decodeMatrixResult(text)};
 if(raw.version!==1||Object.keys(raw).length!==4||Object.keys(raw).some(k=>!['format','version','report','view'].includes(k)))throw new Error('比較の作業状態ファイルの形式が違います');
 const report=decodeMatrixResult(JSON.stringify(raw.report));return {report,view:matrixView(report,raw.view)};
}
