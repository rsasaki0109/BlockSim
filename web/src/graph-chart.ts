import type { GraphResult } from './graph-model';
import { sameTraceWidth } from './graph-comparison';
export interface Series { key:string; label:string; values:number[]; color:string }
const COLORS=['#63e3bc','#ffa96b','#b3a1ff','#67c8ff','#f5d66c','#fa87ac','#e9f0f9','#78d381'];
export function seriesFrom(result:GraphResult):Series[] {
  const duplicates=new Set(result.traces.filter((t,i,list)=>list.some((other,j)=>i!==j&&other.label===t.label)).map(t=>t.label));
  return result.traces.flatMap(t=>t.values[0].map((_,i)=>({key:`${t.id}:${i}`,label:`${t.label}${duplicates.has(t.label)?` (${t.id})`:''} [${i}]`,values:t.values.map(v=>v[i]),color:''}))).map((s,i)=>({...s,color:COLORS[i%COLORS.length]}));
}
export function plotGraph(host:HTMLElement,result:GraphResult,selected:Set<string>,reference?:GraphResult) {
  const list=seriesFrom(result).filter(s=>selected.has(s.key)).slice(0,16);
  const current=new Map(result.traces.map(t=>[t.id,t]));
  const previous=reference?seriesFrom({...reference,traces:reference.traces.filter(t=>current.has(t.id)&&sameTraceWidth(current.get(t.id)!,t))}).filter(s=>list.some(c=>c.key===s.key)):[];
  const ns='http://www.w3.org/2000/svg';
  const svg=document.createElementNS(ns,'svg');svg.setAttribute('viewBox','0 0 1000 310');svg.setAttribute('role','img');svg.setAttribute('aria-label',previous.length?'Scope 信号グラフ（現行と基準）':'Scope 信号グラフ');
  const make=(tag:string,attrs:Record<string,string>)=>{const el=document.createElementNS(ns,tag);for(const [k,v]of Object.entries(attrs))el.setAttribute(k,v);return el;};
  let lo=0,hi=0;for(const s of [...list,...previous])for(const v of s.values){lo=Math.min(lo,v);hi=Math.max(hi,v);}
  const scale=Math.max(Math.abs(lo),Math.abs(hi))||1;lo/=scale;hi/=scale;
  const span=hi-lo||1;lo=Math.max(-Number.MAX_VALUE/scale,lo-span*.06);hi=Math.min(Number.MAX_VALUE/scale,hi+span*.06);
  const end=Math.max(result.times.at(-1)!,previous.length?reference!.times.at(-1)!:0);
  const x=(t:number)=>65+915*(t/end),y=(v:number)=>270-(v-lo)/(hi-lo)*245;
  for(let i=0;i<=4;i++) {
    const v=lo+(hi-lo)*i/4;
    svg.append(make('line',{x1:'65',x2:'980',y1:String(y(v)),y2:String(y(v)),stroke:'#29374d'}));
    const text=make('text',{x:'55',y:String(y(v)+4),'text-anchor':'end',fill:'#9facc5','font-size':'12'});text.textContent=Math.max(-Number.MAX_VALUE,Math.min(Number.MAX_VALUE,v*scale)).toFixed(3);svg.append(text);
  }
  for(let i=0;i<=5;i++){const t=end*(i/5);const text=make('text',{x:String(x(t)),y:'298',fill:'#9facc5','text-anchor':'middle','font-size':'12'});text.textContent=t.toFixed(2)+'s';svg.append(text);}
  for(const [snapshot,series,times]of [['reference',previous,reference?.times],['current',list,result.times]] as const)for(const s of series){
    const stride=Math.max(1,Math.ceil(times!.length/2000));
    const points=times!.map((t,i)=>({t,v:s.values[i],i})).filter(p=>p.i%stride===0||p.i===times!.length-1);
    const color=list.find(c=>c.key===s.key)!.color;
    const path=make('path',{d:points.map((p,i)=>`${i?'L':'M'}${x(p.t).toFixed(3)},${y(p.v/scale).toFixed(3)}`).join(' '),fill:'none',stroke:color,'stroke-width':'2','data-series':s.key,'data-snapshot':snapshot});
    if(snapshot==='reference'){path.setAttribute('stroke-dasharray','8 5');path.setAttribute('opacity','.7');}svg.append(path);
  }
  host.replaceChildren(svg);
}
