import {validateSampleTable} from './sampled-input-csv';
import type {GraphModel,GraphResult} from './graph-model';
export interface InputPreviewRequest {values:unknown;width:number;sample_time:number;repeat:boolean;record_dt:number;cycles:number}
export interface InputPreviewReport {format:'blocksim.input-preview';version:1;request:InputPreviewRequest;model:GraphModel;result:GraphResult}
export function inputPreviewModels(request:InputPreviewRequest):{request:InputPreviewRequest;guard:GraphModel;model:GraphModel} {
 const q=structuredClone(request),rows=validateSampleTable(q.values,q.width);
 if(!Number.isFinite(q.sample_time)||q.sample_time<=0||!Number.isFinite(q.record_dt)||q.record_dt<=0||typeof q.repeat!=='boolean'||!Number.isInteger(q.cycles)||q.cycles<1||q.cycles>3)throw new Error('周期・モデル刻みは正の有限値、表示周期数は1〜3の整数です');
 q.cycles=q.repeat?q.cycles:1;const duration=rows.length*q.cycles*q.sample_time;
 if(!Number.isFinite(duration)||duration<=0)throw new Error('プレビューの時間が有限の範囲を超えています');
 const model:GraphModel={version:1,dt:q.sample_time,duration,blocks:[{id:'source',label:'Draft input',width:q.width,position:[30,100],params:{kind:'sampled_input',values:rows,sample_time:q.sample_time,...(q.repeat?{repeat:true}:{})}},{id:'output',label:'Input',width:q.width,position:[260,100],params:{kind:'scope'}}],edges:[{from:'source',from_port:0,to:'output',to_port:0}]};
 // Validate the source period against the real model clock before recording the
 // isolated input once per acquisition. No connected plant is simulated here.
 const guard=structuredClone(model);guard.dt=q.record_dt;guard.duration=q.record_dt;
 return {request:q,guard,model};
}
export function inputPreviewReport(request:InputPreviewRequest,model:GraphModel,result:GraphResult):InputPreviewReport {
 const rows=validateSampleTable(request.values,request.width),count=rows.length*request.cycles+1,trace=result.traces[0];
 if(result.times.length!==count||result.traces.length!==1||trace?.id!=='output'||trace.values.length!==count)throw new Error('入力プレビューの記録形式が違います');
 for(let k=0;k<count;k++){
  const t=result.times[k],expected=k*request.sample_time;
  if(!Number.isFinite(t)||Math.abs(t-expected)>1e-10*Math.max(Math.abs(expected),Number.MIN_VALUE)||k>0&&t<=result.times[k-1])throw new Error('入力プレビューの時計が違います');
  const index=request.repeat?k%rows.length:Math.min(k,rows.length-1),v=trace.values[k];if(v.length!==request.width||v.some((x,j)=>!Number.isFinite(x)||x!==rows[index][j]))throw new Error('実記録が指定した保持入力と違います');
 }
 return structuredClone({format:'blocksim.input-preview',version:1,request,model,result});
}
export function inputPreviewPosition(report:InputPreviewReport,sample:number) {
 const rows=(report.request.values as number[][]).length;
 if(!Number.isInteger(sample)||sample<0||sample>=report.result.times.length||rows<1)throw new Error('取得サンプルが範囲外です');
 return {row:report.request.repeat?sample%rows:Math.min(sample,rows-1),cycle:report.request.repeat?Math.floor(sample/rows):0,end:sample===report.result.times.length-1,time:report.result.times[sample],next:report.result.times[sample+1]??null};
}
export function inputPreviewSampleForRow(report:InputPreviewReport,sample:number,row:number) {
 const rows=(report.request.values as number[][]).length,p=inputPreviewPosition(report,sample);
 if(!Number.isInteger(row)||row<0||row>=rows)throw new Error('表の行番号が範囲外です');
 if(!report.request.repeat)return p.end&&row===rows-1?sample:row;
 const target=p.cycle*rows+row;
 return target<report.result.times.length?target:(report.request.cycles-1)*rows+row;
}
const COLORS=['#63e3bc','#ffa96b','#b3a1ff','#67c8ff','#f5d66c','#fa87ac','#e9f0f9','#78d381'];
function previewGeometry(report:InputPreviewReport,components:number[]) {
 const {times,traces}=report.result,rows=traces[0].values;
 let low=0,high=0;for(const row of rows)for(const j of components){low=Math.min(low,row[j]);high=Math.max(high,row[j]);}
 const scale=Math.max(Math.abs(low),Math.abs(high))||1;low/=scale;high/=scale;const pad=(high-low||1)*.06;low-=pad;high+=pad;
 const end=times.at(-1)!,x=(t:number)=>65+915*(t/end),y=(v:number)=>270-(v/scale-low)/(high-low)*245;
 return {scale,low,high,end,x,y};
}
export function plotInputPreview(host:HTMLElement,report:InputPreviewReport,components:number[]){
 const {times,traces}=report.result,rows=traces[0].values,ns='http://www.w3.org/2000/svg',svg=document.createElementNS(ns,'svg');svg.setAttribute('viewBox','0 0 1000 310');svg.setAttribute('role','img');svg.setAttribute('aria-label','入力プレビュー（取得時刻間で保持）');
 const make=(tag:string,attrs:Record<string,string>)=>{const e=document.createElementNS(ns,tag);for(const [k,v]of Object.entries(attrs))e.setAttribute(k,v);return e;};
 const {scale,low,high,end,x,y}=previewGeometry(report,components);
 for(let i=0;i<=4;i++){const normalized=low+(high-low)*i/4,v=Math.max(-Number.MAX_VALUE,Math.min(Number.MAX_VALUE,normalized*scale)),py=270-(normalized-low)/(high-low)*245;svg.append(make('line',{x1:'65',x2:'980',y1:String(py),y2:String(py),stroke:'#29374d'}));const text=make('text',{x:'55',y:String(py+4),'text-anchor':'end',fill:'#9facc5','font-size':'12'});text.textContent=v===0?'0':v.toPrecision(4);svg.append(text);}
 for(let i=0;i<=5;i++){const t=end*(i/5),text=make('text',{x:String(x(t)),y:'298',fill:'#9facc5','text-anchor':'middle','font-size':'12'});text.textContent=t.toPrecision(4)+'s';svg.append(text);}
 for(const j of components.slice(0,16)){
  let d=`M${x(times[0]).toFixed(3)},${y(rows[0][j]).toFixed(3)}`;
  for(let k=1;k<times.length;k++)d+=` H${x(times[k]).toFixed(3)} V${y(rows[k][j]).toFixed(3)}`;
  svg.append(make('path',{d,fill:'none',stroke:COLORS[j%COLORS.length],'stroke-width':'2','data-component':String(j)}));
 }
 host.replaceChildren(svg);
}

export function markInputPreview(host:HTMLElement,report:InputPreviewReport,components:number[],sample:number) {
 const p=inputPreviewPosition(report,sample),svg=host.querySelector('svg');if(!svg)return;
 svg.querySelectorAll('[data-preview-marker]').forEach(e=>e.remove());
 const {x,y}=previewGeometry(report,components),ns='http://www.w3.org/2000/svg',group=document.createElementNS(ns,'g');group.setAttribute('data-preview-marker','selection');
 const make=(tag:string,attrs:Record<string,string>)=>{const e=document.createElementNS(ns,tag);for(const [k,v]of Object.entries(attrs))e.setAttribute(k,v);return e;};
 if(p.next!==null)group.append(make('rect',{x:String(x(p.time)),y:'25',width:String(x(p.next)-x(p.time)),height:'245',fill:'#63e3bc',opacity:'.08','data-preview-interval':'true'}));
 group.append(make('line',{x1:String(x(p.time)),x2:String(x(p.time)),y1:'25',y2:'270',stroke:'#e9f0f9','stroke-width':'1.5','stroke-dasharray':'4 4','data-preview-cursor':'true'}));
 for(const j of components.slice(0,16))group.append(make('circle',{cx:String(x(p.time)),cy:String(y(report.result.traces[0].values[sample][j])),r:'4',fill:COLORS[j%COLORS.length],stroke:'#0b1220','data-preview-point':String(j)}));
 svg.append(group);
}
