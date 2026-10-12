import {describe,it,expect} from 'vitest';
import {decodeConversionPlan,conversionReport,conversionModels,conversionCsv,type ConversionPlan} from '../src/discretize-comparison';
import type {GraphResult} from '../src/graph-model';
const plan:ConversionPlan={format:'blocksim.conversion-comparison',version:1,method:'zoh',request:{numerator:[1],denominator:[1,1],sample_time:.2},samples:3};
const converted={request:plan.request,numerator:[0,1-Math.exp(-.2)],denominator:[1,-Math.exp(-.2)],method:'zoh'};
const result=(value:(t:number)=>number):GraphResult=>({times:Array.from({length:21},(_,i)=>i*.02),traces:[{id:'output',label:'output',values:Array.from({length:21},(_,i)=>[value(i*.02)])}],solver:'rk45'});
describe('conversion comparison',()=>{
 it('rejects unbounded, ambiguous and invalid plans',()=>{
  expect(decodeConversionPlan(JSON.stringify(plan))).toEqual(plan);
  for(const patch of [{version:2},{samples:2002},{samples:1},{samples:2.1},{extra:true},{request:{...plan.request,sample_time:0}},{request:{...plan.request,numerator:[1,2,3]}},{request:{...plan.request,denominator:[0,1]}},{request:{...plan.request,prewarp_omega:1}}])expect(()=>decodeConversionPlan(JSON.stringify({...plan,...patch}))).toThrow();
  expect(()=>decodeConversionPlan(' '.repeat(100001))).toThrow(/100 KB/);
 });
 it('builds independent isolated zero state models with a held unit step',()=>{
  const before=structuredClone(plan),[a,b]=conversionModels(plan,converted);expect(plan).toEqual(before);expect(a.blocks[0].params).toEqual({kind:'constant',value:[1]});expect(a.dt).toBe(.02);expect(a.duration).toBe(.4);expect(b.blocks[1].params.sample_time).toBe(.2);b.blocks[0].params.value=[9];expect(a.blocks[0].params.value).toEqual([1]);
 });
 it('compares only conversion instants and exports every matching sample',()=>{
  const a=result(t=>1-Math.exp(-t)),b=result(t=>1-Math.exp(-Math.floor((t+1e-10)/.2)*.2));
  const r=conversionReport(plan,converted,a,b);expect(r.times).toEqual([0,.2,.4]);expect(r.metrics.maxAbs).toBeLessThan(1e-14);expect(conversionCsv(r).split('\r\n')).toHaveLength(4);a.traces[0].values[0][0]=9;plan.samples=4;expect(r.continuous[0]).toBe(0);expect(r.plan.samples).toBe(3);plan.samples=3;
 });
 it('includes the initial direct-term difference and signs final difference',()=>{
  const r=conversionReport(plan,converted,result(()=>0),result(t=>.1-t));expect(r.metrics.maxAbs).toBeCloseTo(.3);expect(r.metrics.final).toBeCloseTo(-.3);expect(r.metrics.rms).toBeCloseTo(Math.sqrt(.11/3));
 });
 it('refuses misaligned grids, missing endpoints, nonfinite results and vector scopes',()=>{
  for(const edit of [(r:GraphResult)=>{r.times[10]+=.001;},(r:GraphResult)=>{r.times.pop();},(r:GraphResult)=>{r.traces[0].values[10]=[NaN];},(r:GraphResult)=>{r.traces[0].values[10]=[1,2];}]){const b=result(()=>1);edit(b);expect(()=>conversionReport(plan,converted,result(()=>0),b)).toThrow();}
 });
});
