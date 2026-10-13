import {describe,it,expect} from 'vitest';
import {decodeConversionPlan,conversionReport,conversionModels,conversionCsv,conversionInputLabel,decodeInputCsv,type ConversionPlan} from '../src/discretize-comparison';
import type {GraphResult} from '../src/graph-model';
const plan:ConversionPlan={format:'blocksim.conversion-comparison',version:1,method:'zoh',request:{numerator:[1],denominator:[1,1],sample_time:.2},samples:3};
const converted={request:plan.request,numerator:[0,1-Math.exp(-.2)],denominator:[1,-Math.exp(-.2)],method:'zoh'};
const result=(value:(t:number)=>number):GraphResult=>({times:Array.from({length:21},(_,i)=>i*.02),traces:[{id:'output',label:'output',values:Array.from({length:21},(_,i)=>[value(i*.02)])}],solver:'rk45'});
describe('conversion comparison',()=>{
 it('accepts strict scalar CSV with optional input header and rejects ambiguous or excessive files',()=>{
  expect(decodeInputCsv('\uFEFFinput\r\n0\r\n+1.25\r\n-2e-3\r\n')).toEqual([0,1.25,-.002]);expect(decodeInputCsv('.5\n1.')).toEqual([.5,1]);
  for(const s of ['1','time,input\n0,1\n1,2','0\n\n1','0\nNaN','0\nInfinity','0\n0x10','0\n"1"','0\n1e999','input\n'+Array(2002).fill('0').join('\n'),' '.repeat(100001)])expect(()=>decodeInputCsv(s)).toThrow();
 });
 it('requires exact series length and builds bounded actual SampledInput graphs',()=>{
  const p:ConversionPlan={...plan,input:{kind:'sample_sequence',values:[1,-.5,0]}};expect(decodeConversionPlan(JSON.stringify(p))).toEqual(p);const [a,b]=conversionModels(p,converted);expect(a.blocks[0].params).toEqual({kind:'sampled_input',values:[[1],[-.5],[0]],sample_time:.2});expect(b.blocks).toHaveLength(4);
  for(const values of [[1],[1,2,3,4],[1,null,0],[1,'2',0],[[1],[2],[3]]])expect(()=>decodeConversionPlan(JSON.stringify({...p,input:{kind:'sample_sequence',values}}))).toThrow();
 });
 it('checks actual series equality and exports each sample index without changing old CSV',()=>{
  const p:ConversionPlan={...plan,input:{kind:'sample_sequence',values:[0,1,2]}},a=result(()=>0);a.traces.push({id:'input_record',label:'actual',values:a.times.map((_,i)=>[Math.floor(i/10)])});const b=structuredClone(a),r=conversionReport(p,converted,a,b);expect(r.input).toEqual([0,1,2]);expect(conversionCsv(r).split('\r\n')[2].split(',').slice(6)).toEqual(['1','1']);expect(conversionInputLabel(r)).toContain('3サンプル');
  expect(()=>conversionReport({...p,input:{kind:'sample_sequence',values:[0,9,2]}},converted,a,b)).toThrow(/指定した系列/);
 });
 it('bounds sampled-sine fields, Nyquist and ambiguous input formats',()=>{
  const input={kind:'sampled_sine',frequency_hz:2.5,amplitude:-1,phase_rad:.4,offset:.25};
  expect(decodeConversionPlan(JSON.stringify({...plan,input})).input).toEqual(input);
  for(const patch of [{frequency_hz:2.500001},{frequency_hz:-1},{amplitude:null},{phase_rad:'0'},{offset:null},{extra:true},{change_sample:1}])expect(()=>decodeConversionPlan(JSON.stringify({...plan,input:{...input,...patch}}))).toThrow();
 });
 it('uses actual sample/hold and records the supplied input on both sides',()=>{
  const p:ConversionPlan={...plan,input:{kind:'sampled_sine',frequency_hz:.5,amplitude:1,phase_rad:.4,offset:.25}},[a,b]=conversionModels(p,converted);
  expect(a.blocks.find(v=>v.id==='hold')!.params).toEqual({kind:'discrete_transfer_function',numerator:[1],denominator:[1],initial:[],sample_time:.2});expect(b.blocks.find(v=>v.id==='hold')).toEqual(a.blocks.find(v=>v.id==='hold'));expect(a.edges.filter(e=>e.from==='hold').map(e=>e.to)).toEqual(['plant','input_record']);
 });
 it('refuses different, unheld or nonfinite actual input and exports its executed conditions',()=>{
  const p:ConversionPlan={...plan,input:{kind:'sampled_sine',frequency_hz:.5,amplitude:1,phase_rad:.4,offset:.25}};
  const a=result(()=>0);a.traces.push({id:'input_record',label:'Actual input',values:a.times.map((_,i)=>[Math.floor(i/10)])});const b=structuredClone(a);
  const r=conversionReport(p,converted,a,b);expect(r.input).toEqual([0,1,2]);expect(r.post_change_metrics).toBeUndefined();expect(conversionInputLabel(r)).toContain('Nyquist=2.5Hz');expect(conversionCsv(r).split('\r\n')[0]).toContain('input,frequency_hz,amplitude,phase_rad,offset');
  for(const edit of [(r:GraphResult)=>{r.traces.pop();},(r:GraphResult)=>{r.traces[1].values[10]=[3];},(r:GraphResult)=>{r.traces[1].values[1]=[.5];},(r:GraphResult)=>{r.traces[1].values[10]=[NaN];}]){const bad=structuredClone(b);edit(bad);expect(()=>conversionReport(p,converted,a,bad)).toThrow();}
 });
 it('rejects invalid held input and off-clock or out-of-range change conditions',()=>{
  const input={kind:'held_step',before:1,after:0,change_sample:1};
  expect(decodeConversionPlan(JSON.stringify({...plan,input})).input).toEqual(input);
  for(const u of [null,{}, {...input,extra:0},{...input,kind:'step'},{...input,before:null},{...input,after:'0'},{...input,change_sample:0},{...input,change_sample:3},{...input,change_sample:1.5}])expect(()=>decodeConversionPlan(JSON.stringify({...plan,input:u}))).toThrow();
 });
 it('builds the same held source for both graphs at an exact conversion instant',()=>{
  const held={...plan,input:{kind:'held_step' as const,before:2,after:-.5,change_sample:2}};
  const [a,b]=conversionModels(held,converted);
  expect(a.blocks[0].params).toEqual({kind:'step',before:[2],after:[-.5],step_time:.4});expect(b.blocks[0].params).toEqual(a.blocks[0].params);
  const [rounded]=conversionModels({...held,request:{...held.request,sample_time:.1},samples:101,input:{...held.input,change_sample:3}},converted);expect(rounded.blocks[0].params.step_time).toBe(.3);expect(rounded.blocks[0].params.step_time).not.toBe(3*.1);
 });
 it('includes the transition sample in post-change metrics and CSV input provenance',()=>{
  const p={...plan,input:{kind:'held_step' as const,before:2,after:-.5,change_sample:1}};
  const r=conversionReport(p,converted,result(()=>0),result(t=>t<.2?100:1));
  expect(r.metrics.maxAbs).toBe(100);expect(r.post_change_metrics).toEqual({maxAbs:1,rms:1,final:1,area:.2});expect(r.input).toEqual([2,-.5,-.5]);
  const rows=conversionCsv(r).split('\r\n');expect(rows[0]).toContain('input,change_sample,before,after');expect(rows[2].split(',').slice(6)).toEqual(['-0.5','1','2','-0.5']);p.input.after=99;expect((r.plan.input?.kind==='held_step'?r.plan.input.after:undefined)).toBe(-.5);
 });
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
