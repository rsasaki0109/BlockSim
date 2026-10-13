import {it,expect} from 'vitest';
import {inputPreviewModels,inputPreviewReport,type InputPreviewRequest} from '../src/sampled-input-preview';
import type {GraphResult} from '../src/graph-model';
const q:InputPreviewRequest={values:[[1,-1],[.5,2],[0,-.5]],width:2,sample_time:.1,repeat:true,record_dt:.01,cycles:2};
it('validates the original clock separately and builds bounded acquisition records without modifying the draft',()=>{
 const old=structuredClone(q),r=inputPreviewModels(q);expect(q).toEqual(old);expect(r.guard.dt).toBe(.01);expect(r.guard.duration).toBe(.01);expect(r.model.dt).toBe(.1);expect(r.model.duration).toBeCloseTo(.6);expect(r.model.blocks).toHaveLength(2);expect(r.model.blocks[0].params.values).toEqual(q.values);
 const hold=inputPreviewModels({...q,repeat:false,cycles:3});expect(hold.request.cycles).toBe(1);expect(hold.model.blocks[0].params.repeat).toBeUndefined();expect(hold.model.duration).toBeCloseTo(.3);
});
it('refuses bad cells, widths, cycles, periods, and unbounded preview time',()=>{
 for(const patch of [{values:[]},{values:[[1,null]]},{width:65},{cycles:0},{cycles:4},{cycles:1.5},{sample_time:0},{record_dt:0},{repeat:'true'},{sample_time:Number.MAX_VALUE}])expect(()=>inputPreviewModels({...q,...patch} as InputPreviewRequest)).toThrow();
});
it('checks actual clock and input provenance including endpoint wrap without hiding missing or altered values',()=>{
 const built=inputPreviewModels(q),result:GraphResult={solver:'rk4',times:Array.from({length:7},(_,k)=>k*.1),traces:[{id:'output',label:'Input',values:Array.from({length:7},(_,k)=>(q.values as number[][])[k%3])}]};
 const r=inputPreviewReport(built.request,built.model,result);expect(r.result.traces[0].values[6]).toEqual([1,-1]);result.traces[0].values[0]=[99,99];expect(r.result.traces[0].values[0]).toEqual([1,-1]);
 for(const edit of [(r:GraphResult)=>r.times.pop(),(r:GraphResult)=>{r.times[3]+=.001;},(r:GraphResult)=>{r.traces[0].values[3]=[0,0];},(r:GraphResult)=>{r.traces[0].values[3]=[NaN,0];}]){const bad=structuredClone(r.result);edit(bad);expect(()=>inputPreviewReport(built.request,built.model,bad)).toThrow();}
});
