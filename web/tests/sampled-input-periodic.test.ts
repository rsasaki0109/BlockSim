import {it,expect} from 'vitest';
import {generateSampleTable,type InputGeneration} from '../src/sampled-input-generator';
import {encodeInputGenerator,decodeInputGenerator} from '../src/sampled-input-generator-file';
const q:InputGeneration={shape:'pulse',width:2,rows:8,base:[0,0],delta:[1,-1],cycles:2,duty:[25,75]};
it('uses exact sample boundaries for independently gated pulse and bipolar square components',()=>{
 const one=[[1,-1],[0,-1],[0,-1],[0,0]];expect(generateSampleTable(q)).toEqual([...one,...one]);
 const square=[[1,-1],[-1,-1],[-1,-1],[-1,1]];expect(generateSampleTable({...q,shape:'square'})).toEqual([...square,...square]);
});
it('accepts zero/full duty and fractional percentages with sample-quantized active intervals',()=>{
 expect(generateSampleTable({...q,duty:[0,100]})).toEqual(Array(8).fill([0,-1]));expect(generateSampleTable({...q,shape:'square',duty:[0,100]})).toEqual(Array(8).fill([-1,-1]));
 const values=generateSampleTable({...q,width:1,rows:16,base:0,delta:1,duty:12.5});expect(values.flat()).toEqual([1,0,0,0,0,0,0,0,1,0,0,0,0,0,0,0]);
 expect(generateSampleTable({...q,width:1,base:2,delta:3,duty:25.1}).flat()).toEqual([5,5,2,2,5,5,2,2]);
});
it('generates triangle troughs, peaks and period wrap and exposes unsampled peaks for odd periods',()=>{
 const t=generateSampleTable({...q,shape:'triangle',rows:16,base:0,delta:[1,-1]});const one=[-1,-.5,0,.5,1,.5,0,-.5];expect(t.map(r=>r[0])).toEqual([...one,...one]);expect(t.map(r=>r[1])).toEqual([...one,...one].map(x=>-x===0?0:-x));
 const odd=generateSampleTable({...q,shape:'triangle',width:1,rows:3,cycles:1,base:0,delta:1});expect(odd[0][0]).toBe(-1);expect(odd[1][0]).toBeCloseTo(1/3,14);expect(odd[2][0]).toBeCloseTo(1/3,14);
});
it('rejects nonintegral periods, malformed duty, generated overflow and excessive output size',()=>{
 for(const patch of [{cycles:0},{cycles:1.5},{cycles:3},{rows:1,cycles:1},{duty:null},{duty:[25]},{duty:[0,NaN]},{duty:Infinity},{duty:-1},{duty:101}])expect(()=>generateSampleTable({...q,...patch})).toThrow();
 expect(()=>generateSampleTable({...q,shape:'square',base:1e308,delta:1e308,duty:50})).toThrow();expect(()=>generateSampleTable({...q,shape:'triangle',width:64,rows:2000,base:1.2345678901234567e308,delta:0})).toThrow('1 MB');
});
it('preserves scalar and vector duties in version 3, fills the default and regenerates independent data',()=>{
 for(const shape of ['square','triangle','pulse'] as const){const request={...q,shape},text=encodeInputGenerator(request),loaded=decodeInputGenerator(text);expect(JSON.parse(text).version).toBe(3);expect(generateSampleTable(loaded)).toEqual(generateSampleTable(request));if(shape==='triangle')expect(loaded.duty).toBeUndefined();else expect(loaded.duty).toEqual([25,75]);}
 const defaultDuty=decodeInputGenerator(encodeInputGenerator({...q,duty:undefined}));expect(defaultDuty.duty).toBe(50);expect(decodeInputGenerator(encodeInputGenerator({...q,duty:25})).duty).toBe(25);const before=structuredClone(q);const loaded=decodeInputGenerator(encodeInputGenerator(q));(loaded.duty as number[])[0]=100;expect(q).toEqual(before);
});
it('enforces waveform/version contracts and rejects irrelevant, missing or invalid fields',()=>{
 const valid=JSON.parse(encodeInputGenerator(q)),missing=structuredClone(valid);delete missing.generation.duty;
 for(const bad of [missing,{...valid,version:1},{...valid,version:2},{...valid,version:4},{...valid,generation:{...valid.generation,phase:0}},{...valid,generation:{...valid.generation,duty:101}},{...valid,generation:{...valid.generation,cycles:3}},{...valid,generation:{...valid.generation,shape:'triangle'}}])expect(()=>decodeInputGenerator(JSON.stringify(bad))).toThrow();
 const legacy=JSON.parse(encodeInputGenerator({shape:'constant',width:2,rows:8,base:0}));expect(legacy.version).toBe(1);expect(()=>decodeInputGenerator(JSON.stringify({...legacy,version:3}))).toThrow();
});
