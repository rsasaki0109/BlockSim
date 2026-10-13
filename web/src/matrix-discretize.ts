import {graphContext,type GraphBlock,type GraphModel} from './graph-model';
export interface MatrixRequest {a:number[][];b:number[][];c:number[][];d:number[][];initial:number[];sample_time:number;method:'zoh'|'tustin';prewarp_omega?:number}
export interface MatrixResult {request:MatrixRequest;a:number[][];b:number[][];c:number[][];d:number[][];initial:number[];mapping_scale:number|null;squarings:number;terms:number;residual:number;method:string}
export function matrixRequest(block:GraphBlock,sampleTime:number,method:MatrixRequest['method'],prewarp?:number):MatrixRequest {
 if(block.params.kind!=='state_space'||block.params.mode!=='continuous')throw new Error('連続 StateSpace を選択してください');
 const initial=structuredClone(block.params.initial as number[]);
 if(method==='tustin'&&initial.some(v=>v!==0))throw new Error('Tustinは初期状態0のみ対応します。非ゼロの元設定を保持します。');
 return {a:structuredClone(block.params.a as number[][]),b:structuredClone(block.params.b as number[][]),c:structuredClone(block.params.c as number[][]),d:structuredClone(block.params.d as number[][]),initial,sample_time:sampleTime,method,...(prewarp===undefined?{}:{prewarp_omega:prewarp})};
}
export function matrixModel(model:GraphModel,address:string,r:MatrixResult,normalize:(s:string)=>string,diagnose:(s:string)=>string):GraphModel {
 const next=structuredClone(model),path=address.split('/'),id=path.pop()!,block=graphContext(next,path).blocks.find(b=>b.id===id);
 if(!block)throw new Error('変換対象のブロックがありません');
 const q=matrixRequest(block,r.request.sample_time,r.request.method,r.request.prewarp_omega);
 if(JSON.stringify(q)!==JSON.stringify(r.request))throw new Error('変換前の行列または初期状態が変わっています。再確認してください。');
 block.params={kind:'state_space',a:structuredClone(r.a),b:structuredClone(r.b),c:structuredClone(r.c),d:structuredClone(r.d),initial:structuredClone(r.initial),mode:'discrete',sample_time:q.sample_time};
 const text=normalize(JSON.stringify(next)),issues=JSON.parse(diagnose(text)).diagnostics as {code:string;message:string}[];
 const loop=issues.find(d=>d.code==='algebraic_loop');if(loop)throw new Error('変換で直接伝達の循環が発生します。Delayなどで循環を切ってください。 '+loop.message);
 return JSON.parse(text) as GraphModel;
}
