import { expect, it } from 'vitest';
import { connectPorts, mergeGraph, ungroupBlock, wirePath, nodeHeight } from '../src/graph-edit';
import { GraphHistory, groupBlocks, newBlock, type GraphModel, type GraphBlock } from '../src/graph-model';
import { seriesFrom } from '../src/graph-chart';
import sample from '../../samples/graphs/continuous-pid.json';
import vector from '../../samples/graphs/vector.json';
const fresh=()=>structuredClone(sample) as GraphModel;
const plain=(blocks:GraphBlock[],edges:GraphModel['edges']=[]):GraphModel=>({version:1,dt:.01,duration:1,blocks,edges});
const edge=(from:string,to:string,to_port=0,from_port=0)=>({from,to,to_port,from_port});
function named(kind:GraphBlock['params']['kind'],id:string,g:GraphModel):GraphBlock{return {...newBlock(kind,g),id,label:id};}
it('connects fan-out but rejects occupied ports and width mismatch without replacing edges',()=>{
 const g=structuredClone(vector) as GraphModel;const original=structuredClone(g.edges);
 expect(()=>connectPorts(g,edge('source','scale'))).toThrow('already connected');expect(g.edges).toEqual(original);
 const scope=named('scope','other',g);g.blocks.push(scope);
 expect(()=>connectPorts(g,edge('source','other'))).toThrow('width mismatch');expect(g.edges).toEqual(original);
 scope.width=3;connectPorts(g,edge('source','other'));expect(g.edges).toHaveLength(6);
 expect(()=>connectPorts(g,edge('source','other',0,1))).toThrow('ポート');
});
it('checks Mux combined widths and Selector indices including incomplete connections',()=>{
 const g=plain([]);g.blocks=[named('constant','a',g),named('constant','b',g),named('mux','mux',g),named('selector','select',g)];
 connectPorts(g,edge('a','mux'));connectPorts(g,edge('b','mux',1));expect(g.edges).toHaveLength(2);
 g.blocks[3].params.indices=[2];expect(()=>connectPorts(g,edge('mux','select'))).toThrow('outside');
 g.blocks[3].params.indices=[1];connectPorts(g,edge('mux','select'));expect(g.edges).toHaveLength(3);
 const other=plain(structuredClone(g.blocks));other.blocks[2].width=3;
 connectPorts(other,edge('a','mux'));expect(()=>connectPorts(other,edge('b','mux',1))).toThrow('入力幅合計');expect(other.edges).toHaveLength(1);
});
it('ungroups a feedback subsystem with fan-out and preserves state/configuration',()=>{
 const g=fresh(),id=groupBlocks(g,new Set(['error','controller','plant']));
 const added=ungroupBlock(g,id);expect(added).toHaveLength(3);expect(g.blocks.some(b=>b.params.kind==='input'||b.params.kind==='output'||b.id===id)).toBe(false);
 expect(g.edges).toHaveLength(6);
 expect(g.edges).toContainEqual(edge(`${id}-plant`,`${id}-error`,1));
 expect(g.edges).toContainEqual(edge('reference',`${id}-error`));
 expect(g.edges).toContainEqual(edge(`${id}-plant`,'output'));
 expect(g.edges).toContainEqual(edge(`${id}-controller`,'control'));
 expect(g.blocks.find(b=>b.id===`${id}-controller`)!.params).toEqual(sample.blocks[2].params);
});
it('inlines a boundary-only pass-through and leaves incomplete drafts incomplete',()=>{
 const g=plain([]),source=named('constant','source',g),scope=named('scope','scope',g),sub=named('subsystem','sub',g);
 const inner=sub.params.model as GraphModel;inner.blocks=inner.blocks.filter(b=>b.params.kind!=='gain');inner.edges=[edge('in','out')];
 g.blocks=[source,sub,scope];g.edges=[edge('source','sub'),edge('sub','scope')];
 expect(ungroupBlock(g,'sub')).toEqual([]);expect(g.edges).toEqual([edge('source','scope')]);
 const partial=plain([source,named('subsystem','draft',g),scope],[edge('draft','scope')]);
 ungroupBlock(partial,'draft');expect(partial.edges).toEqual([edge('draft-gain','scope')]);
});
it('preserves nested subsystems, avoids ID collisions, and rejects boundary-only cycles transactionally',()=>{
 const g=fresh(),id=groupBlocks(g,new Set(['controller','plant']));
 const child=(g.blocks.find(b=>b.id===id)!.params.model as GraphModel);groupBlocks(child,new Set(['controller']));
 g.blocks.push(named('constant',`${id}-plant`,g));const added=ungroupBlock(g,id);
 expect(new Set(g.blocks.map(b=>b.id)).size).toBe(g.blocks.length);expect(added).toContain(`${id}-plant-2`);expect(g.blocks.some(b=>b.params.kind==='subsystem')).toBe(true);
 const loop=plain([]),sub=named('subsystem','loop',loop),inner=sub.params.model as GraphModel;
 inner.blocks=inner.blocks.filter(b=>b.params.kind!=='gain');inner.edges=[edge('in','out')];loop.blocks=[sub];loop.edges=[edge('loop','loop')];
 const h=new GraphHistory(loop,json=>json);expect(()=>h.apply(g=>ungroupBlock(g,'loop'))).toThrow('循環');expect(h.graph).toEqual(loop);expect(h.canUndo).toBe(false);
});
it('adds models without replacing IDs, edges or clocks and supports undo',()=>{
 const g=fresh(),before=structuredClone(g),h=new GraphHistory(g,json=>json);
 h.apply(root=>{mergeGraph(root,fresh());mergeGraph(root,fresh());});
 expect(h.graph.blocks).toHaveLength(18);expect(new Set(h.graph.blocks.map(b=>b.id)).size).toBe(18);
 expect(h.graph.edges.slice(0,6)).toEqual(before.edges);expect(h.graph.dt).toBe(before.dt);expect(h.graph.duration).toBe(before.duration);
 expect(h.graph.edges).toContainEqual(edge('import-reference','import-error'));
 expect(h.graph.edges).toContainEqual(edge('import-reference-2','import-error-2'));
 h.undo();expect(h.graph).toEqual(before);
 expect(()=>h.apply(root=>mergeGraph(root,{...fresh(),dt:.005}))).toThrow('刻み幅');expect(h.graph).toEqual(before);
});
it('routes feedback outside block rectangles and grows nodes for multiple ports',()=>{
 const g=fresh(),plant=g.blocks.find(b=>b.id==='plant')!,error=g.blocks.find(b=>b.id==='error')!;
 const path=wirePath(plant,0,error,1);expect(path).toContain('L');expect(path).toContain(String(120+84+35+8));
 error.params.signs=Array(8).fill(1);expect(nodeHeight(error)).toBeGreaterThan(84);
});
it('disambiguates duplicate Scope labels by ID while preserving unique labels',()=>{
 const s=seriesFrom({times:[0,1],solver:'test',traces:[{id:'a',label:'Scope',values:[[1],[2]]},{id:'b',label:'Scope',values:[[3],[4]]},{id:'c',label:'Unique',values:[[5],[6]]}]});
 expect(s.map(x=>x.label)).toEqual(['Scope (a) [0]','Scope (b) [0]','Unique [0]']);
});
