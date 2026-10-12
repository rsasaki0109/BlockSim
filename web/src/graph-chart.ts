import type { GraphResult } from './graph-model';
export interface Series { key:string; label:string; values:number[]; color:string }
const COLORS=['#63e3bc','#ffa96b','#b3a1ff','#67c8ff','#f5d66c','#fa87ac','#e9f0f9','#78d381'];
export function seriesFrom(result:GraphResult):Series[] {
  const duplicates=new Set(result.traces.filter((t,i,list)=>list.some((other,j)=>i!==j&&other.label===t.label)).map(t=>t.label));
  return result.traces.flatMap(t=>t.values[0].map((_,i)=>({key:`${t.id}:${i}`,label:`${t.label}${duplicates.has(t.label)?` (${t.id})`:''} [${i}]`,values:t.values.map(v=>v[i]),color:''}))).map((s,i)=>({...s,color:COLORS[i%COLORS.length]}));
}
export function plotGraph(host:HTMLElement,result:GraphResult,selected:Set<string>) {
  const list=seriesFrom(result).filter(s=>selected.has(s.key)).slice(0,16);
  const ns='http://www.w3.org/2000/svg';
  const svg=document.createElementNS(ns,'svg');svg.setAttribute('viewBox','0 0 1000 310');svg.setAttribute('role','img');svg.setAttribute('aria-label','Scope 信号グラフ');
  const make=(tag:string,attrs:Record<string,string>)=>{const el=document.createElementNS(ns,tag);for(const [k,v]of Object.entries(attrs))el.setAttribute(k,v);return el;};
  let lo=0,hi=0;for(const s of list)for(const v of s.values){lo=Math.min(lo,v);hi=Math.max(hi,v);}
  const span=hi-lo||1;lo-=span*.06;hi+=span*.06;
  const x=(t:number)=>65+915*t/result.times.at(-1)!,y=(v:number)=>270-(v-lo)/(hi-lo)*245;
  for(let i=0;i<=4;i++) {
    const v=lo+(hi-lo)*i/4;
    svg.append(make('line',{x1:'65',x2:'980',y1:String(y(v)),y2:String(y(v)),stroke:'#29374d'}));
    const text=make('text',{x:'55',y:String(y(v)+4),'text-anchor':'end',fill:'#9facc5','font-size':'12'});text.textContent=v.toFixed(3);svg.append(text);
  }
  for(let i=0;i<=5;i++){const t=result.times.at(-1)!*i/5;const text=make('text',{x:String(x(t)),y:'298',fill:'#9facc5','text-anchor':'middle','font-size':'12'});text.textContent=t.toFixed(2)+'s';svg.append(text);}
  for(const s of list) {
    const stride=Math.max(1,Math.ceil(result.times.length/2000));
    const points=result.times.map((t,i)=>({t,v:s.values[i],i})).filter(p=>p.i%stride===0||p.i===result.times.length-1);
    svg.append(make('path',{d:points.map((p,i)=>`${i?'L':'M'}${x(p.t).toFixed(3)},${y(p.v).toFixed(3)}`).join(' '),fill:'none',stroke:s.color,'stroke-width':'2','data-series':s.key}));
  }
  host.replaceChildren(svg);
}
