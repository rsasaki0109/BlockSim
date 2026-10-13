import {it,expect} from 'vitest';
import {encodeInputGenerator,decodeInputGenerator,INPUT_GENERATOR_FILE_LIMIT} from '../src/sampled-input-generator-file';
import {generateSampleTable,type InputGeneration} from '../src/sampled-input-generator';
const request:InputGeneration={shape:'step',width:2,rows:5,base:[1,-1],delta:[2,-2],step:2};
it('round-trips all waveform conditions and reproduces tables while preserving scalar broadcast and component arrays',()=>{
 for(const q of [{...request,shape:'constant',base:.25},{...request},{...request,shape:'ramp'},{...request,shape:'sine',rows:8,cycles:2}] as InputGeneration[]){const text=encodeInputGenerator(q),loaded=decodeInputGenerator(text);expect(generateSampleTable(loaded)).toEqual(generateSampleTable(q));expect(loaded.base).toEqual(q.base);expect(JSON.parse(text).format).toBe('blocksim.input-generator');expect(JSON.parse(text).version).toBe(1);}
});
it('saves only fields used by the selected waveform and never source clocks or sample tables',()=>{
 const constant=JSON.parse(encodeInputGenerator({...request,shape:'constant',delta:'unused',step:NaN,cycles:Infinity}));expect(constant.generation).toEqual({shape:'constant',width:2,rows:5,base:[1,-1]});expect(Object.keys(constant)).toEqual(['format','version','generation']);
 const ramp=JSON.parse(encodeInputGenerator({...request,shape:'ramp'})).generation;expect(ramp.step).toBeUndefined();const sine=JSON.parse(encodeInputGenerator({...request,shape:'sine',cycles:1})).generation;expect(sine.step).toBeUndefined();expect(sine.cycles).toBe(1);
});
it('rejects other formats, versions, missing or extra fields and malformed generation parameters',()=>{
 const valid=JSON.parse(encodeInputGenerator(request));
 for(const bad of [null,[],{}, {...valid,format:'blocksim.input-preview'},{...valid,version:2},{...valid,version:'1'},{...valid,extra:0},{...valid,generation:[]},{...valid,generation:{...valid.generation,values:[[0,0]]}},{...valid,generation:{...valid.generation,cycles:1}},{...valid,generation:{...valid.generation,base:[1]}},{...valid,generation:{...valid.generation,rows:2002}},{...valid,generation:{...valid.generation,step:5}},{...valid,generation:{...valid.generation,shape:'noise'}}])expect(()=>decodeInputGenerator(JSON.stringify(bad))).toThrow();
 const missing=structuredClone(valid);delete missing.generation.delta;expect(()=>decodeInputGenerator(JSON.stringify(missing))).toThrow();expect(()=>decodeInputGenerator('{')).toThrow();
});
it('rejects conditions producing overflow or oversized tables even when the condition file is small',()=>{
 const overflow={format:'blocksim.input-generator',version:1,generation:{...request,base:[1e308,0],delta:[1e308,0]}};expect(()=>decodeInputGenerator(JSON.stringify(overflow))).toThrow();expect(()=>encodeInputGenerator(overflow.generation)).toThrow();
 const large={format:'blocksim.input-generator',version:1,generation:{shape:'constant',width:64,rows:2001,base:1.2345678901234567e308}};expect(()=>decodeInputGenerator(JSON.stringify(large))).toThrow('1 MB');
});
it('checks UTF-8 bytes before parsing, permits the exact limit and accepts a leading BOM',()=>{
 const text=encodeInputGenerator(request),padding=INPUT_GENERATOR_FILE_LIMIT-new TextEncoder().encode(text).length;expect(decodeInputGenerator(text+' '.repeat(padding))).toEqual(request);expect(()=>decodeInputGenerator(text+' '.repeat(padding+1))).toThrow('100 KB');expect(()=>decodeInputGenerator('あ'.repeat(34000))).toThrow('100 KB');expect(decodeInputGenerator('\uFEFF'+text)).toEqual(request);
});
it('returns independent settings and leaves caller objects untouched during serialization',()=>{
 const before=structuredClone(request),text=encodeInputGenerator(request),a=decodeInputGenerator(text),b=decodeInputGenerator(text);(a.base as number[])[0]=99;expect(b).toEqual(before);expect(request).toEqual(before);
});
