import {it,expect} from 'vitest';
import {generateSampleTable,type InputGeneration} from '../src/sampled-input-generator';
import {encodeInputGenerator,decodeInputGenerator} from '../src/sampled-input-generator-file';
const q:InputGeneration={shape:'sine',width:2,rows:8,base:[.25,-.5],delta:[1,2],cycles:1};
it('generates independently phased channels matching sine and cosine at every row',()=>{
 const values=generateSampleTable({...q,phase:[0,Math.PI/2]});values.forEach((row,k)=>{expect(row[0]).toBeCloseTo(.25+Math.sin(k*Math.PI/4),14);expect(row[1]).toBeCloseTo(-.5+2*Math.cos(k*Math.PI/4),14);});expect(values).toHaveLength(8);
});
it('keeps absent, scalar-zero and all-zero vector phases exactly equal to the previous formula',()=>{
 const expected=Array.from({length:8},(_,k)=>{const f=Math.sin(2*Math.PI*(k/8));return [.25+f,-.5+2*f];});for(const phase of [undefined,0,[0,0]])expect(generateSampleTable({...q,phase})).toEqual(expected);
});
it('broadcasts scalar phase, supports negative phase and accepts both full-cycle boundaries',()=>{
 const phased=generateSampleTable({...q,base:0,delta:1,phase:-Math.PI/2});phased.forEach((row,k)=>{expect(row[0]).toBeCloseTo(-Math.cos(k*Math.PI/4),14);expect(row[0]).toBe(row[1]);});
 const values=generateSampleTable({...q,phase:[-2*Math.PI,2*Math.PI]}),zero=generateSampleTable(q);values.forEach((row,k)=>row.forEach((v,j)=>expect(v).toBeCloseTo(zero[k][j],14)));
 expect(generateSampleTable({...q,shape:'constant',phase:'unused'})).toEqual(Array(8).fill([.25,-.5]));
});
it('rejects malformed or excessively large phases without changing parameters',()=>{
 for(const phase of [null,true,'0',[0],[0,NaN],Infinity,1e308,2*Math.PI+1e-12,-2*Math.PI-1e-12]){const request={...q,phase},before=structuredClone(request);expect(()=>generateSampleTable(request)).toThrow();expect(request).toEqual(before);}
});
it('round-trips nonzero scalar and component phases as version 2 while preserving version 1 zero-phase exports',()=>{
 for(const phase of [Math.PI/2,[0,-Math.PI/2]]){const request={...q,phase},text=encodeInputGenerator(request);expect(JSON.parse(text).version).toBe(2);expect(decodeInputGenerator(text)).toEqual(request);expect(generateSampleTable(decodeInputGenerator(text))).toEqual(generateSampleTable(request));}
 const expected=JSON.stringify({format:'blocksim.input-generator',version:1,generation:q},null,2);for(const phase of [undefined,0,[0,0]])expect(encodeInputGenerator({...q,phase})).toBe(expected);expect(generateSampleTable(decodeInputGenerator(expected))).toEqual(generateSampleTable(q));
});
it('enforces version-specific phase fields and rejects unknown versions and invalid phase payloads',()=>{
 const good=JSON.parse(encodeInputGenerator({...q,phase:[0,Math.PI/2]}));const missing=structuredClone(good);delete missing.generation.phase;
 for(const bad of [missing,{...good,version:1},{...good,version:3},{...good,generation:{...good.generation,shape:'ramp'}},{...good,generation:{...good.generation,phase:[0]}},{...good,generation:{...good.generation,phase:7}}])expect(()=>decodeInputGenerator(JSON.stringify(bad))).toThrow();
});
it('keeps decoded phase vectors independent from both the caller and subsequent imports',()=>{
 const request={...q,phase:[0,Math.PI/2]},before=structuredClone(request),text=encodeInputGenerator(request),a=decodeInputGenerator(text),b=decodeInputGenerator(text);(a.phase as number[])[1]=0;expect(request).toEqual(before);expect(b).toEqual(before);
});
