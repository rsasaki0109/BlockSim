import { describe, expect, it } from 'vitest';
import {
  GraphHistory, newBlock, groupBlocks, deleteBlocks, graphContext, inputCount, outputCount, setClock,
  decodeGraphDocument, encodeGraphDocument, resultCsv, type GraphModel,
} from '../src/graph-model';
import sample from '../../samples/graphs/continuous-pid.json';
import vector from '../../samples/graphs/vector.json';
const normalize=(json:string)=>json;
const fresh=()=>structuredClone(sample) as GraphModel;
describe('graph editing transactions',()=>{
  it('discrete transfer defaults preserve coefficient order, companion state and recording clock through model files',()=>{
    const g=fresh(),b=newBlock('discrete_transfer_function',g);expect(b.params).toEqual({kind:'discrete_transfer_function',numerator:[.2,0],denominator:[1,-.8],initial:[0],sample_time:g.dt});g.blocks.push(b);expect(decodeGraphDocument(encodeGraphDocument(g),normalize)).toEqual(g);expect(inputCount(b)).toBe(1);expect(outputCount(b)).toBe(1);
  });
  it('undo/redo restores entire graph and a new edit invalidates redo',()=>{
    const history=new GraphHistory(fresh(),normalize);const original=structuredClone(history.graph);
    history.apply(g=>g.blocks.push(newBlock('constant',g)));expect(history.graph.blocks).toHaveLength(7);
    history.undo();expect(history.graph).toEqual(original);history.redo();expect(history.graph.blocks).toHaveLength(7);
    history.undo();history.apply(g=>g.duration=30);expect(history.canRedo).toBe(false);
  });
  it('failed validation does not mutate graph or history',()=>{
    const history=new GraphHistory(fresh(),json=>{const g=JSON.parse(json);if(g.duration<0)throw new Error('duration');return json;});
    const original=structuredClone(history.graph);expect(()=>history.apply(g=>g.duration=-1)).toThrow();
    expect(history.graph).toEqual(original);expect(history.canUndo).toBe(false);
  });
  it('groups connected blocks with two input boundaries and fan-out output boundaries',()=>{
    const g=fresh();const id=groupBlocks(g,new Set(['error','controller','plant']));
    const sub=g.blocks.find(b=>b.id===id)!;
    expect(inputCount(sub)).toBe(1);expect(outputCount(sub)).toBe(2);
    expect(g.edges.some(e=>e.from==='reference'&&e.to===id&&e.to_port===0)).toBe(true);
    expect(g.edges.some(e=>e.from===id&&e.to==='output')).toBe(true);
    const inner=graphContext(g,[id]);expect(inner.edges.some(e=>e.from==='plant'&&e.to==='error')).toBe(true);
    expect(inner.blocks.filter(b=>b.params.kind==='output')).toHaveLength(2);
  });
  it('grouping a vector pipeline preserves port widths',()=>{
    const g=structuredClone(vector) as GraphModel;const id=groupBlocks(g,new Set(['scale','limit']));
    const inner=graphContext(g,[id]);expect(inner.blocks.find(b=>b.params.kind==='input')!.width).toBe(3);
    expect(inner.blocks.find(b=>b.params.kind==='output')!.width).toBe(3);
    expect(g.edges.filter(e=>e.from===id)).toHaveLength(2); // one boundary output with fan-out
    expect(outputCount(g.blocks.find(b=>b.id===id)!)).toBe(1);
  });
  it('nested clock updates and block deletion also update edges',()=>{
    const g=fresh();const id=groupBlocks(g,new Set(['controller','plant']));setClock(g,.005,20);
    expect(graphContext(g,[id]).dt).toBe(.005);deleteBlocks(g,new Set([id]));
    expect(g.edges.every(e=>e.from!==id&&e.to!==id)).toBe(true);
  });
  it('new block IDs are unique and subsystem interfaces are executable drafts',()=>{
    const g=fresh();const a=newBlock('gain',g);g.blocks.push(a);const b=newBlock('gain',g);expect(a.id).not.toBe(b.id);
    const sub=newBlock('subsystem',g);expect(inputCount(sub)).toBe(1);expect(outputCount(sub)).toBe(1);
  });
});
describe('graph document and result export',()=>{
  it('roundtrips graph version 2 and accepts standalone samples',()=>{
    expect(decodeGraphDocument(encodeGraphDocument(fresh()),normalize)).toEqual(fresh());
    expect(decodeGraphDocument(JSON.stringify(fresh()),normalize)).toEqual(fresh());
    expect(()=>decodeGraphDocument('{"version":9,"graph":{}}',normalize)).toThrow();
    expect(()=>decodeGraphDocument(' '.repeat(1000001),normalize)).toThrow('1 MB');
  });
  it('exports every vector component and escapes labels safely',()=>{
    const csv=resultCsv({times:[0,.1],traces:[{id:'scope',label:'=A,"B"',values:[[1,2],[3,4]]}],solver:'test'});
    expect(csv).toContain("'=A,\"\"B\"\"");expect(csv).toContain('0,1,2');expect(csv).toContain('0.1,3,4');
  });
});
