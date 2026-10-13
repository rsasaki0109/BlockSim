// Sample rows only: time, period and repeat remain model settings.
const MAX_BYTES=1_000_000;
const numeric=/^[+-]?(?:\d+(?:\.\d*)?|\.\d+)(?:[eE][+-]?\d+)?$/;
function checkWidth(width:number){if(!Number.isInteger(width)||width<1||width>64)throw new Error('信号幅は1〜64の整数です');}
function checkSize(text:string){if(new TextEncoder().encode(text).length>MAX_BYTES)throw new Error('入力表CSVは1 MBまでです');}
function header(width:number){return width===1?['input']:Array.from({length:width},(_,i)=>`input_${i}`);}
export function validateSampleTable(value:unknown,width:number):number[][] {
 checkWidth(width);
 if(!Array.isArray(value)||value.length<1||value.length>2001||value.some(row=>!Array.isArray(row)||row.length!==width||row.some(v=>typeof v!=='number'||!Number.isFinite(v))))throw new Error('入力表は1〜2001行、信号幅と同じ列数の有限数値が必要です');
 return value as number[][];
}
export function decodeSampleTableCsv(text:string,width:number):number[][] {
 checkWidth(width);checkSize(text);
 const rows=text.replace(/^\uFEFF/,'').trim().split(/\r?\n/).map(row=>row.split(',').map(cell=>cell.trim()));
 if(rows[0][0]==='input'||rows[0][0]==='input_0'){
  const wanted=header(width);if(rows[0].length!==width||!rows[0].every((v,i)=>v===wanted[i]))throw new Error('CSV見出しはinput、またはinput_0から順番に指定してください');rows.shift();
 }
 if(rows.length<1||rows.length>2001||rows.some(row=>row.length!==width||row.some(v=>!numeric.test(v)||!Number.isFinite(Number(v)))))throw new Error('入力表CSVは1〜2001行、信号幅と同じ列数の数値です。空行・引用符・時刻列は使えません');
 return rows.map(row=>row.map(Number));
}
export function sampleTableCsv(value:unknown,width:number):string {
 const rows=validateSampleTable(value,width);const text=[header(width).join(','),...rows.map(row=>row.join(','))].join('\r\n');checkSize(text);return text;
}
