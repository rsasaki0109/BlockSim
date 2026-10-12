import { expect, it } from 'vitest';
import { diagnosticTarget, diagnosticLinks, type GraphDiagnostic } from '../src/graph-diagnostics';
import { GraphHistory, newBlock, type GraphModel } from '../src/graph-model';
import { copySelection, pasteSelection } from '../src/graph-edit';
import subsystem from '../../samples/graphs/subsystem.json';
const fresh=()=>structuredClone(subsystem) as GraphModel;
const issue=(block_id:string|null,port:number|null=0):GraphDiagnostic=>({code:'unconnected_input',message:'check input',block_id,port,source_id:null});
it('maps exact nested IDs and flat boundary port zero to its parent interface port',()=>{
 const graph=fresh(),inner=graph.blocks[1].params.model as GraphModel;inner.blocks.push(newBlock('input',inner));pasteSelection(graph,copySelection(graph,new Set(graph.blocks.map(b=>b.id))));
 const links=diagnosticLinks(graph,issue('copy-amplifier/input-1'));
 expect(links.target?.path).toEqual(['copy-amplifier']);expect(links.parentInput?.target.address).toBe('copy-amplifier');expect(links.parentInput?.port).toBe(1);expect(links.source).toBeNull();
 const width=diagnosticLinks(graph,{...issue('amplifier/gain'),code:'signal_width',source_id:'amplifier/in'});expect(width.source?.address).toBe('amplifier/in');expect(width.parentInput).toBeNull();
});
it('does not guess targets from messages or partial/stale addresses for global diagnostics',()=>{
 const graph=fresh();expect(diagnosticTarget(graph,'amplifier/missing')).toBeNull();expect(diagnosticTarget(graph,'missing/gain')).toBeNull();
 expect(diagnosticLinks(graph,{...issue(null,null),code:'structure',message:'amplifier/gain: invalid'})).toEqual({target:null,source:null,parentInput:null});
 expect(diagnosticTarget(graph,'gain')).toBeNull();expect(diagnosticTarget(graph,'amplifier/gain')?.id).toBe('gain');
});
it('resolves current labels after rename/delete/undo without modifying the model',()=>{
 const history=new GraphHistory(fresh(),json=>json),before=structuredClone(history.graph);
 expect(diagnosticTarget(history.graph,'amplifier/gain')?.label).toBe('gain');expect(history.graph).toEqual(before);
 history.apply(g=>{(g.blocks[1].params.model as GraphModel).blocks[1].label='調整器';});expect(diagnosticTarget(history.graph,'amplifier/gain')?.label).toBe('調整器');
 history.apply(g=>{(g.blocks[1].params.model as GraphModel).blocks.splice(1,1);});expect(diagnosticTarget(history.graph,'amplifier/gain')).toBeNull();history.undo();expect(diagnosticTarget(history.graph,'amplifier/gain')?.label).toBe('調整器');
});
