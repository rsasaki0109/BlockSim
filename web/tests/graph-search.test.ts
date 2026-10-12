import { expect, it } from 'vitest';
import { searchGraph } from '../src/graph-search';
import { GraphHistory, newBlock, type GraphModel } from '../src/graph-model';
import { pasteSelection, copySelection } from '../src/graph-edit';
import sample from '../../samples/graphs/subsystem.json';
const fresh=()=>structuredClone(sample) as GraphModel;
it('matches Unicode-normalized case-insensitive AND tokens across label, ID, kind and ancestors with literal punctuation',()=>{
 const g=fresh();g.blocks[1].label='増幅器Ａ';const inner=g.blocks[1].params.model as GraphModel;inner.blocks[1].label='Gain [fast] <preview>';
 expect(searchGraph(g,'増幅器a ＧＡＩＮ [fast]').map(h=>h.address)).toEqual(['amplifier/gain']);
 expect(searchGraph(g,'gain absent')).toEqual([]);expect(searchGraph(g,'[').map(h=>h.id)).toEqual(['gain']);expect(searchGraph(g,'<preview>').map(h=>h.id)).toEqual(['gain']);
 const plant=newBlock('first_order',g);g.blocks.push(plant);expect(searchGraph(g,'FIRST ORDER').map(h=>h.id)).toEqual([plant.id]);
});
it('distinguishes duplicate nested IDs by address and limits kind/current-hierarchy searches correctly',()=>{
 const g=fresh();pasteSelection(g,copySelection(g,new Set(g.blocks.map(b=>b.id))));
 expect(searchGraph(g,'gain').map(h=>h.address)).toEqual(['amplifier/gain','copy-amplifier/gain']);
 const local=searchGraph(g,'gain',{path:['copy-amplifier'],recursive:false,kind:'gain'});expect(local[0].path).toEqual(['copy-amplifier']);expect(local.map(h=>h.address)).toEqual(['copy-amplifier/gain']);
 expect(searchGraph(g,'gain',{recursive:false})).toEqual([]);expect(searchGraph(g,'',{kind:'input'})).toHaveLength(2);expect(searchGraph(g,'gain',{kind:'scope'})).toEqual([]);
});
it('searches nested depth and empty models without mutating graph or caller paths and returns deterministic addresses',()=>{
 const g=fresh(),nested=newBlock('subsystem',g);(g.blocks[1].params.model as GraphModel).blocks.push(nested);
 const before=structuredClone(g),path=['amplifier'];const hits=searchGraph(g,'gain',{path});path.push('changed');expect(hits.map(h=>h.address)).toEqual(['amplifier/gain','amplifier/subsystem-1/gain']);expect(hits[0].path).toEqual(['amplifier']);expect(g).toEqual(before);
 const reversed=structuredClone(g);reversed.blocks.reverse();expect(searchGraph(reversed,'gain')).toEqual(searchGraph(g,'gain'));expect(searchGraph({...g,blocks:[],edges:[]},'gain')).toEqual([]);
});
it('reflects renamed, deleted, undone and replaced models instead of retaining stale block objects',()=>{
 const h=new GraphHistory(fresh(),json=>json);h.apply(g=>{(g.blocks[1].params.model as GraphModel).blocks[1].label='調整器';});
 expect(searchGraph(h.graph,'調整器')[0].address).toBe('amplifier/gain');h.apply(g=>{const inner=g.blocks[1].params.model as GraphModel;inner.blocks=inner.blocks.filter(b=>b.id!=='gain');inner.edges=[];});expect(searchGraph(h.graph,'調整器')).toEqual([]);
 h.undo();expect(searchGraph(h.graph,'調整器')).toHaveLength(1);h.replace({...fresh(),blocks:[],edges:[]});expect(searchGraph(h.graph,'調整器')).toEqual([]);
});
