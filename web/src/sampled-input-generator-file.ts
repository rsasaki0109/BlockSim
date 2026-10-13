import {generateSampleTable,isTableWave,type InputGeneration} from './sampled-input-generator';
export const INPUT_GENERATOR_FILE_LIMIT=100_000;
export interface InputGeneratorFile {format:'blocksim.input-generator';version:1|2|3;generation:InputGeneration}
function fields(shape:unknown,phased=false){return ['shape','width','rows','base',...(shape==='constant'?[]:['delta']),...(shape==='step'?['step']:shape==='sine'||isTableWave(shape)?['cycles']:[]),...(shape==='square'||shape==='pulse'?['duty']:[]),...(phased?['phase']:[])];}
function canonical(request:InputGeneration,phased=false):InputGeneration {
 generateSampleTable(request);
 return structuredClone(Object.fromEntries(fields(request.shape,phased).map(key=>[key,key==='duty'&&request.duty===undefined?50:request[key as keyof InputGeneration]]))) as unknown as InputGeneration;
}
export function encodeInputGenerator(request:InputGeneration):string {
 const generation=canonical(request,request.shape==='sine'&&request.phase!==undefined);
 const phased=generation.phase!==undefined&&(Array.isArray(generation.phase)?generation.phase.some(v=>v!==0):generation.phase!==0);
 if(!phased)delete generation.phase;
 return JSON.stringify({format:'blocksim.input-generator',version:isTableWave(generation.shape)?3:phased?2:1,generation} satisfies InputGeneratorFile,null,2);
}
export function decodeInputGenerator(text:string):InputGeneration {
 if(new TextEncoder().encode(text).length>INPUT_GENERATOR_FILE_LIMIT)throw new Error('生成条件JSONは100 KBまでです');
 const value=JSON.parse(text.replace(/^\uFEFF/,'')) as unknown;
 if(!value||typeof value!=='object'||Array.isArray(value))throw new Error('生成条件JSONの形式が不正です');
 const file=value as Record<string,unknown>,generation=file.generation;
 if(file.format!=='blocksim.input-generator'||![1,2,3].includes(file.version as number)||Object.keys(file).some(key=>!['format','version','generation'].includes(key))||!generation||typeof generation!=='object'||Array.isArray(generation))throw new Error('生成条件JSONはblocksim.input-generator version 1・2・3です');
 const request=generation as unknown as InputGeneration,phased=file.version===2,allowed=fields(request.shape,phased);
 if(phased&&request.shape!=='sine')throw new Error('生成条件JSON version 2は位相付き正弦波です');
 if((file.version===3)!==isTableWave(request.shape))throw new Error('矩形波・三角波・パルス列の生成条件はversion 3です');
 if(Object.keys(generation).some(key=>!allowed.includes(key))||allowed.some(key=>!Object.hasOwn(generation,key)))throw new Error('生成条件JSONの項目が波形の種類と一致しません');
 return canonical(request,phased);
}
