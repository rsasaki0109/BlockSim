import { graphContext, KIND_NAMES, type GraphModel, type Kind } from './graph-model';

export interface GraphSearchHit {
  path: string[];
  id: string;
  label: string;
  kind: Kind;
  address: string;
}
export interface SearchOptions { path?: string[]; recursive?: boolean; kind?: Kind|'' }
const normalized = (text: string) => text.normalize('NFKC').toLowerCase();
/** Search fresh metadata; paths distinguish equal local IDs in different subsystems. */
export function searchGraph(root: GraphModel, query: string, options: SearchOptions = {}): GraphSearchHit[] {
  const path = [...(options.path ?? [])], recursive = options.recursive ?? true;
  const tokens = normalized(query).trim().split(/\s+/).filter(Boolean), hits: GraphSearchHit[] = [];
  const ancestorLabels: string[] = [];
  for (let i = 0; i < path.length; i++) {
    ancestorLabels.push(graphContext(root, path.slice(0, i)).blocks.find(b => b.id === path[i])!.label);
  }
  function visit(graph: GraphModel, parent: string[], labels: string[]) {
    for (const block of graph.blocks) {
      const address = [...parent, block.id].join('/');
      const fields = normalized([address, ...labels, block.label, block.params.kind, KIND_NAMES[block.params.kind]].join(' '));
      if ((!options.kind || block.params.kind === options.kind) && tokens.every(token => fields.includes(token))) {
        hits.push({ path: [...parent], id: block.id, label: block.label, kind: block.params.kind, address });
      }
      if (recursive && block.params.kind === 'subsystem') visit(block.params.model as GraphModel, [...parent, block.id], [...labels, block.label]);
    }
  }
  visit(graphContext(root, path), path, ancestorLabels);
  return hits.sort((a, b) => a.address < b.address ? -1 : a.address > b.address ? 1 : 0);
}
