import {it,expect} from 'vitest';
import {planGeneratedInput} from '../src/sampled-input-generation-plan';
import type {InputGeneration} from '../src/sampled-input-generator';
const q:InputGeneration={shape:'pulse',width:2,rows:8,base:[0,0],delta:[1,-1],cycles:2,duty:[25,75]};
it('plans table and waveform clocks, acquired ranges and per-period active rows without counting a terminal point',()=>{
 const p=planGeneratedInput(q,.1,true);expect(p.values).toHaveLength(8);expect(p.table_time).toBe(.8);expect(p.last_sample_time).toBeCloseTo(.7,14);expect(p.period).toEqual({rows:4,time:.4,frequency:2.5});expect(p.channels.map(c=>[c.min,c.max])).toEqual([[0,1],[-1,0]]);expect(p.channels.map(c=>[c.duty!.requested,c.duty!.effective,c.duty!.on_rows,c.duty!.off_rows])).toEqual([[25,25,1,3],[75,75,3,1]]);expect(p.channels[1].duty!.on_time).toBeCloseTo(.3,14);
});
it('reports sampled duty rather than inferring activation from visible values or nonzero amplitude',()=>{
 const p=planGeneratedInput({...q,base:2,delta:0,duty:25.1},.2,false);expect(p.repeat).toBe(false);p.channels.forEach(c=>{expect([c.min,c.max]).toEqual([2,2]);expect(c.duty).toEqual({requested:25.1,effective:50,on_rows:2,off_rows:2,on_time:.4});});
});
it('reports both extreme duties with constant sampled ranges and all 64 components',()=>{
 const p=planGeneratedInput({...q,shape:'square',width:64,base:0,delta:1,duty:Array.from({length:64},(_,j)=>j%2===0?0:100)},.1,true);expect(p.channels).toHaveLength(64);p.channels.forEach((c,j)=>{expect([c.min,c.max]).toEqual(j%2===0?[-1,-1]:[1,1]);expect(c.duty!.on_rows).toBe(j%2===0?0:4);expect(c.duty!.effective).toBe(j%2===0?0:100);});
});
it('distinguishes row acquisition from final hold, step transition, ramp arrival and fractional sine periods',()=>{
 const step=planGeneratedInput({shape:'step',width:1,rows:5,base:0,delta:1,step:2},.1,false);expect(step.step_time).toBe(.2);expect(step.period).toBeUndefined();expect(step.last_sample_time).toBe(.4);expect(step.table_time).toBe(.5);
 const ramp=planGeneratedInput({shape:'ramp',width:1,rows:1,base:3,delta:10},.1,false);expect(ramp.ramp_end_time).toBe(0);expect([ramp.channels[0].min,ramp.channels[0].max]).toEqual([3,3]);
 const sine=planGeneratedInput({shape:'sine',width:1,rows:5,base:0,delta:1,cycles:2},.1,true);expect(sine.period).toEqual({rows:2.5,time:.25,frequency:4});
 const triangle=planGeneratedInput({shape:'triangle',width:1,rows:3,base:0,delta:1,cycles:1},.1,true);expect(triangle.channels[0].min).toBe(-1);expect(triangle.channels[0].max).toBeCloseTo(1/3,14);
});
it('rejects invalid source clocks, unsupported settings and derived time/frequency overflow',()=>{
 for(const ts of [0,-1,NaN,Infinity,Number.MAX_VALUE,Number.MIN_VALUE])expect(()=>planGeneratedInput(q,ts,true)).toThrow();expect(()=>planGeneratedInput(q,.1,'true' as unknown as boolean)).toThrow();expect(()=>planGeneratedInput({...q,cycles:3},.1,true)).toThrow();
});
it('does not mutate conditions or share planned values between checks',()=>{
 const before=structuredClone(q),a=planGeneratedInput(q,.1,true),b=planGeneratedInput(q,.1,true);a.values[0][0]=99;a.channels[0].min=-99;expect(q).toEqual(before);expect(b.values[0][0]).toBe(1);expect(b.channels[0].min).toBe(0);
});
