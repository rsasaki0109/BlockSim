import { expect, it } from 'vitest';
import { checkArchiveSize, decodeRunArchive, encodeRunArchive, verifyArchivedResult, MAX_RUN_ARCHIVE_BYTES } from '../src/graph-run-archive';
import type { RunSnapshot } from '../src/graph-comparison';
import type { GraphModel } from '../src/graph-model';
import sample from '../../samples/graphs/subsystem.json';
const validate=(json:string)=>json;
function snapshot():RunSnapshot{
 const model=structuredClone(sample) as GraphModel;model.dt=.5;model.duration=.5;const inner=model.blocks[1].params.model as GraphModel;inner.dt=.5;inner.duration=.5;
 return {model,result:{times:[0,.5],traces:[{id:'wave-output',label:'wave-output',values:[[0],[3]]}],solver:'fixed-step hybrid RK4 / synchronous sample-and-hold'}};
}
const document=()=>JSON.parse(encodeRunArchive(snapshot()));
it('roundtrips every executed model field, nested setting, time and Scope value without mutating inputs',()=>{
 const data=snapshot(),before=structuredClone(data),text=encodeRunArchive(data);expect(decodeRunArchive(text,validate)).toEqual(data);expect(data).toEqual(before);
 const restored=decodeRunArchive(text,validate);(restored.model.blocks[1].params.model as GraphModel).blocks[1].params.gain=9;restored.result.traces[0].values[1][0]=99;expect(data).toEqual(before);
});
it('rejects unsupported headers, unknown fields and model validation failures before accepting data',()=>{
 for(const edit of [(d:Record<string,unknown>)=>d.version=2,(d:Record<string,unknown>)=>d.format='other',(d:Record<string,unknown>)=>d.extra=1,(d:Record<string,unknown>)=>delete d.model]){const d=document();edit(d);expect(()=>decodeRunArchive(JSON.stringify(d),validate)).toThrow();}
 for(const input of ['null','[]','{}','{broken'])expect(()=>decodeRunArchive(input,validate)).toThrow();
 expect(()=>decodeRunArchive(encodeRunArchive(snapshot()),()=>{throw new Error('invalid model');})).toThrow('invalid model');
 const d=document();d.result.extra=1;expect(()=>decodeRunArchive(JSON.stringify(d),validate)).toThrow('保存形式');d.result.traces[0].extra=1;delete d.result.extra;expect(()=>decodeRunArchive(JSON.stringify(d),validate)).toThrow('保存形式');
});
it('enforces the file/model/recording limits without allocating recorded time series',()=>{
 expect(()=>checkArchiveSize(MAX_RUN_ARCHIVE_BYTES)).not.toThrow();expect(()=>checkArchiveSize(MAX_RUN_ARCHIVE_BYTES+1)).toThrow('64 MiB');
 const d=document();d.model.extra='x'.repeat(1_000_000);expect(()=>decodeRunArchive(JSON.stringify(d),validate)).toThrow('1 MB');
 const large=document();large.model.duration=1000;large.model.dt=.01;large.model.blocks[2].width=64;expect(()=>decodeRunArchive(JSON.stringify(large),validate)).toThrow('記録数');
 large.model.duration=1001;expect(()=>decodeRunArchive(JSON.stringify(large),validate)).toThrow('記録数');
});
it('requires every exact base-clock time, including the full sample count and finite timestamps',()=>{
 for(const times of [[0],[0,.5,1],[0,.49],[0,0],[.5,0],[0,null],['0',.5]]){const d=document();d.result.times=times;expect(()=>decodeRunArchive(JSON.stringify(d),validate)).toThrow('記録時刻');}
});
it('requires finite numeric cells and rectangular arrays matching Scope widths and sample counts',()=>{
 for(const values of [[[0]],[[0],[3],[4]],[[0,1],[3,4]],[[0],[null]],[[0],['3']],[[0],[]]]){const d=document();d.result.traces[0].values=values;expect(()=>decodeRunArchive(JSON.stringify(d),validate)).toThrow('信号幅');}
 const d=document();d.result.solver=null;expect(()=>decodeRunArchive(JSON.stringify(d),validate)).toThrow('ソルバ');
});
it('matches ordered hierarchical Scope IDs, labels and widths rather than guessing from labels',()=>{
 for(const field of ['id','label']){const d=document();d.result.traces[0][field]='other';expect(()=>decodeRunArchive(JSON.stringify(d),validate)).toThrow('Scope');}
 const data=snapshot(),inner=data.model.blocks[1].params.model as GraphModel;inner.blocks.push({id:'wave-output',label:'same',width:1,position:[10,10],params:{kind:'scope'}});inner.edges.push({from:'gain',from_port:0,to:'wave-output',to_port:0});
 data.result.traces.unshift({id:'amplifier/wave-output',label:'same',values:[[0],[3]]});expect(decodeRunArchive(encodeRunArchive(data),validate)).toEqual(data);
 const d=JSON.parse(encodeRunArchive(data));d.result.traces.reverse();expect(()=>decodeRunArchive(JSON.stringify(d),validate)).toThrow('順序');d.result.traces.pop();expect(()=>decodeRunArchive(JSON.stringify(d),validate)).toThrow('Scope 数');
});
it('compares every recomputed sample and metadata, rejecting a single changed final value or solver',()=>{
 const stored=snapshot().result;expect(()=>verifyArchivedResult(stored,structuredClone(stored))).not.toThrow();
 for(const edit of [(r:typeof stored)=>r.traces[0].values[1][0]+=1,(r:typeof stored)=>r.times[1]+=.001,(r:typeof stored)=>r.solver='different',(r:typeof stored)=>r.traces[0].label='different',(r:typeof stored)=>r.traces[0].values.pop()]){const computed=structuredClone(stored);edit(computed);expect(()=>verifyArchivedResult(stored,computed)).toThrow('再計算と一致');}
});
it('refuses nonfinite results during export instead of serializing them as null',()=>{
 for(const invalid of [NaN,Infinity,-Infinity]){const data=snapshot();data.result.traces[0].values[1][0]=invalid;expect(()=>encodeRunArchive(data)).toThrow('数値');}
});
