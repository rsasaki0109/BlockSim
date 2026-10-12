import { expect, it } from 'vitest';
import { autoLayout } from '../src/graph-layout';
import { nodeHeight, pasteSelection, copySelection } from '../src/graph-edit';
import { groupBlocks, graphContext, newBlock, type GraphModel } from '../src/graph-model';
import sample from '../../samples/graphs/continuous-pid.json';
const fresh=()=>structuredClone(sample) as GraphModel;
const withoutPositions=(g:GraphModel)=>({...g,blocks:g.blocks.map(({position:_,...b})=>b)});
function noOverlap(g:GraphModel){for(let i=0;i<g.blocks.length;i++)for(let j=i+1;j<g.blocks.length;j++){const a=g.blocks[i],b=g.blocks[j];expect(a.position[0]+166<=b.position[0]||b.position[0]+166<=a.position[0]||a.position[1]+nodeHeight(a)<=b.position[1]||b.position[1]+nodeHeight(b)<=a.position[1]).toBe(true);}}
it('lays out feedback flow without changing semantics and is stable across repetition and input ordering',()=>{
 const g=fresh(),before=withoutPositions(g);expect(autoLayout(g).mode).toBe('layered');expect(withoutPositions(g)).toEqual(before);
 const x=(id:string)=>g.blocks.find(b=>b.id===id)!.position[0];expect(x('reference')).toBeLessThan(x('error'));expect(x('error')).toBeLessThan(x('controller'));expect(x('controller')).toBeLessThan(x('plant'));expect(x('plant')).toBeLessThan(x('output'));
 expect(x('control')).toBe(x('controller')+240);expect(x('control')).toBe(x('plant'));
 noOverlap(g);const after=structuredClone(g);expect(autoLayout(g).changed).toBe(false);expect(g).toEqual(after);
 const shuffled=fresh();shuffled.blocks.reverse();shuffled.edges.reverse();autoLayout(shuffled);expect(Object.fromEntries(shuffled.blocks.map(b=>[b.id,b.position]))).toEqual(Object.fromEntries(g.blocks.map(b=>[b.id,b.position])));
});
it('separates disconnected duplicated closed loops into bands',()=>{
 const g=fresh();pasteSelection(g,copySelection(g,new Set(g.blocks.map(b=>b.id))));autoLayout(g);noOverlap(g);
 const original=g.blocks.filter(b=>!b.id.startsWith('copy-')),copied=g.blocks.filter(b=>b.id.startsWith('copy-'));
 expect(Math.max(...original.map(b=>b.position[1]+nodeHeight(b)))).toBeLessThan(Math.min(...copied.map(b=>b.position[1])));expect(g.edges).toHaveLength(12);
});
it('treats subsystem internals as opaque and can separately arrange a nested interface draft',()=>{
 const g=fresh(),id=groupBlocks(g,new Set(['controller','plant'])),inner=graphContext(g,[id]),before=structuredClone(inner);
 inner.blocks.forEach(b=>{b.position=[500,500];});const nested=structuredClone(inner);autoLayout(g);expect(graphContext(g,[id])).toEqual(nested);
 const parent=structuredClone(g);autoLayout(inner);expect(withoutPositions(inner)).toEqual(withoutPositions(before));noOverlap(inner);
 expect(g.blocks.filter(b=>b.id!==id)).toEqual(parent.blocks.filter(b=>b.id!==id));expect(g.edges).toEqual(parent.edges);
});
it('uses a bounded grid for a 256-block chain with tall nodes and retains every edge/configuration',()=>{
 const g:GraphModel={version:1,dt:.01,duration:1,blocks:[],edges:[]};
 for(let i=0;i<256;i++){const b=newBlock('sum',g);b.params.signs=Array(64).fill(1);g.blocks.push(b);if(i)g.edges.push({from:g.blocks[i-1].id,from_port:0,to:b.id,to_port:0});}
 const before=withoutPositions(g);expect(autoLayout(g).mode).toBe('grid');expect(withoutPositions(g)).toEqual(before);noOverlap(g);expect(g.blocks.every(b=>b.position.every(n=>n>=0&&n<=10000))).toBe(true);expect(autoLayout(g).changed).toBe(false);
});
it('handles empty, isolated and self-loop drafts and stacks varying port counts without overlap',()=>{
 const g:GraphModel={version:1,dt:.01,duration:1,blocks:[],edges:[]};expect(autoLayout(g).changed).toBe(false);
 for(const kind of ['constant','sum','sum','scope'] as const)g.blocks.push(newBlock(kind,g));
 g.blocks[1].params.signs=Array(64).fill(1);g.edges=[{from:g.blocks[0].id,from_port:0,to:g.blocks[1].id,to_port:63},{from:g.blocks[0].id,from_port:0,to:g.blocks[2].id,to_port:0},{from:g.blocks[3].id,from_port:0,to:g.blocks[3].id,to_port:0}];
 autoLayout(g);noOverlap(g);expect(g.blocks[2].position[1]-g.blocks[1].position[1]).toBeGreaterThan(nodeHeight(g.blocks[1]));
 const invalid={...structuredClone(g),edges:[{from:'absent',from_port:0,to:'scope-1',to_port:0}]},before=structuredClone(invalid);expect(()=>autoLayout(invalid)).toThrow('ブロック');expect(invalid).toEqual(before);
});
