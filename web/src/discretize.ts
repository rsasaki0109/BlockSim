import {graphContext,type GraphBlock,type GraphModel} from './graph-model';
export interface TustinRequest {numerator:number[];denominator:number[];sample_time:number;prewarp_omega?:number}
export interface ConversionResult {request:TustinRequest;numerator:number[];denominator:number[];method:string}
export interface TustinResult extends ConversionResult {mapping_scale:number;leading_condition_ratio:number}
export interface ZohResult extends ConversionResult {a:number[][];b:number[];c:number[];d:number;squarings:number;terms:number;exponential_identity_residual:number;cayley_hamilton_residual:number;reconstruction_residual:number}
export function tustinRequest(block:GraphBlock,sampleTime:number,prewarp?:number):TustinRequest {
 if(block.params.kind!=='transfer_function')throw new Error('連続 TransferFunction を選択してください');
 if((block.params.initial as number[]).some(v=>v!==0))throw new Error('非ゼロ初期状態の変換には未対応です。元の設定を保持します。');
 return {numerator:structuredClone(block.params.numerator as number[]),denominator:structuredClone(block.params.denominator as number[]),sample_time:sampleTime,...(prewarp===undefined?{}:{prewarp_omega:prewarp})};
}
export function tustinModel(model:GraphModel,address:string,r:ConversionResult,normalize:(json:string)=>string,diagnose:(json:string)=>string):GraphModel {
 const next=structuredClone(model),path=address.split('/'),id=path.pop()!,block=graphContext(next,path).blocks.find(b=>b.id===id);
 if(!block)throw new Error('変換対象のブロックがありません');
 const q=tustinRequest(block,r.request.sample_time,r.request.prewarp_omega);
 if(JSON.stringify(q)!==JSON.stringify(r.request))throw new Error('変換前の係数が変わっています。再確認してください。');
 block.params={kind:'discrete_transfer_function',numerator:structuredClone(r.numerator),denominator:structuredClone(r.denominator),initial:Array(block.width*(r.denominator.length-1)).fill(0),sample_time:q.sample_time};
 const text=normalize(JSON.stringify(next)),diagnostics=JSON.parse(diagnose(text)).diagnostics as Array<{code:string;message:string}>;
 const loop=diagnostics.find(d=>d.code==='algebraic_loop');if(loop)throw new Error('変換で直接伝達の循環が発生します。Delayなどで循環を切ってください。 '+loop.message);
 return JSON.parse(text) as GraphModel;
}
