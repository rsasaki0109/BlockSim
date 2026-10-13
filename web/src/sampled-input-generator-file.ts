import {generateSampleTable,type InputGeneration} from './sampled-input-generator';
export const INPUT_GENERATOR_FILE_LIMIT=100_000;
export interface InputGeneratorFile {format:'blocksim.input-generator';version:1;generation:InputGeneration}
function fields(shape:unknown){return ['shape','width','rows','base',...(shape==='constant'?[]:['delta']),...(shape==='step'?['step']:shape==='sine'?['cycles']:[])];}
function canonical(request:InputGeneration):InputGeneration {
 generateSampleTable(request);
 return structuredClone(Object.fromEntries(fields(request.shape).map(key=>[key,request[key as keyof InputGeneration]]))) as unknown as InputGeneration;
}
export function encodeInputGenerator(request:InputGeneration):string {
 return JSON.stringify({format:'blocksim.input-generator',version:1,generation:canonical(request)} satisfies InputGeneratorFile,null,2);
}
export function decodeInputGenerator(text:string):InputGeneration {
 if(new TextEncoder().encode(text).length>INPUT_GENERATOR_FILE_LIMIT)throw new Error('生成条件JSONは100 KBまでです');
 const value=JSON.parse(text.replace(/^\uFEFF/,'')) as unknown;
 if(!value||typeof value!=='object'||Array.isArray(value))throw new Error('生成条件JSONの形式が不正です');
 const file=value as Record<string,unknown>,generation=file.generation;
 if(file.format!=='blocksim.input-generator'||file.version!==1||Object.keys(file).some(key=>!['format','version','generation'].includes(key))||!generation||typeof generation!=='object'||Array.isArray(generation))throw new Error('生成条件JSONはblocksim.input-generator version 1です');
 const request=generation as unknown as InputGeneration,allowed=fields(request.shape);
 if(Object.keys(generation).some(key=>!allowed.includes(key))||allowed.some(key=>!Object.hasOwn(generation,key)))throw new Error('生成条件JSONの項目が波形の種類と一致しません');
 return canonical(request);
}
