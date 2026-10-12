import { nodeHeight } from './graph-edit';
import type { GraphModel } from './graph-model';

export interface LayoutResult { mode: 'layered'|'grid'; changed: boolean }
const X = 48, Y = 64, COLUMN = 240;
/** Arrange one hierarchy; never reorder blocks/edges or touch numerical settings. */
export function autoLayout(graph: GraphModel): LayoutResult {
  const ids = graph.blocks.map(b => b.id).sort();
  if (!ids.length) return { mode: 'layered', changed: false };
  const blocks = new Map(graph.blocks.map(b => [b.id, b]));
  const successors = new Map(ids.map(id => [id, new Set<string>()]));
  const neighbours = new Map(ids.map(id => [id, new Set<string>()]));
  for (const e of graph.edges) {
    if (!blocks.has(e.from) || !blocks.has(e.to)) throw new Error('配置する接続のブロックがありません');
    successors.get(e.from)!.add(e.to);
    neighbours.get(e.from)!.add(e.to);neighbours.get(e.to)!.add(e.from);
  }
  const adjacent = (id: string) => [...successors.get(id)!].sort();
  // Collapse strongly connected components so feedback cannot stall ranking.
  const index = new Map<string, number>(), low = new Map<string, number>();
  const stack: string[] = [], active = new Set<string>(), components: string[][] = [];
  let serial = 0;
  function visit(id: string) {
    index.set(id, serial);low.set(id, serial++);stack.push(id);active.add(id);
    for (const next of adjacent(id)) {
      if (!index.has(next)) { visit(next);low.set(id, Math.min(low.get(id)!, low.get(next)!)); }
      else if (active.has(next)) low.set(id, Math.min(low.get(id)!, index.get(next)!));
    }
    if (low.get(id) === index.get(id)) {
      const members: string[] = [];let next: string;
      do { next = stack.pop()!;active.delete(next);members.push(next); } while (next !== id);
      components.push(members.sort());
    }
  }
  for (const id of ids) if (!index.has(id)) visit(id);
  components.sort((a, b) => a[0] < b[0] ? -1 : a[0] > b[0] ? 1 : 0);
  const owner = new Map(components.flatMap((members, i) => members.map(id => [id, i] as const)));
  const outgoing = components.map(() => new Set<number>()), incoming = components.map(() => new Set<number>());
  for (const e of graph.edges) {
    const a = owner.get(e.from)!, b = owner.get(e.to)!;
    if (a !== b) { outgoing[a].add(b);incoming[b].add(a); }
  }
  const rank = components.map(() => 0), degree = incoming.map(s => s.size);
  const ready = degree.flatMap((n, i) => n === 0 ? [i] : []);
  const columns = new Map<string, number>();
  while (ready.length) {
    ready.sort((a, b) => a - b);const component = ready.shift()!, members = components[component];
    // Start a cycle at an external input, then follow its signal flow.
    const entries = graph.edges.filter(e => owner.get(e.to) === component && owner.get(e.from) !== component).map(e => e.to).sort();
    const order: string[] = [], seen = new Set<string>();
    function walk(id: string) { if (seen.has(id)) return;seen.add(id);order.push(id);for (const next of adjacent(id)) if (owner.get(next) === component) walk(next); }
    walk(entries[0] ?? members[0]);for (const id of members) walk(id);
    order.forEach((id, i) => columns.set(id, rank[component] + i));
    for (const next of outgoing[component]) {
      // Place fan-out beside its actual source, instead of past the entire cycle.
      for (const edge of graph.edges) if (owner.get(edge.from) === component && owner.get(edge.to) === next) {
        rank[next] = Math.max(rank[next], columns.get(edge.from)! + 1);
      }
      if (--degree[next] === 0) ready.push(next);
    }
  }
  // Independent diagrams receive separate vertical bands, including copied loops.
  const positions = new Map<string, [number, number]>(), visited = new Set<string>();
  const laneGap = 56 + 8 * Math.max(0, ...graph.edges.map(e => e.from_port + e.to_port));
  let band = Y;
  for (const first of ids) {
    if (visited.has(first)) continue;
    const members: string[] = [], queue = [first];visited.add(first);
    while (queue.length) { const id = queue.shift()!;members.push(id);for (const next of [...neighbours.get(id)!].sort()) if (!visited.has(next)) { visited.add(next);queue.push(next); } }
    const rows = new Map<number, number>();let bottom = band;
    for (const id of members.sort()) {
      const column = columns.get(id)!, y = rows.get(column) ?? band;
      positions.set(id, [X + COLUMN * column, y]);
      const end = y + nodeHeight(blocks.get(id)!) + laneGap;
      rows.set(column, end);bottom = Math.max(bottom, end);
    }
    band = bottom + 80;
  }
  let mode: LayoutResult['mode'] = 'layered';
  if ([...positions.values()].some(([x, y]) => x > 10000 || y > 10000)) {
    // Long chains or many disconnected drafts still fit the model coordinate range.
    mode = 'grid';
    const height = Math.max(...graph.blocks.map(nodeHeight)) + 64;
    const maxColumns = Math.floor((10000 - X) / COLUMN) + 1, maxRows = Math.floor((10000 - Y) / height) + 1;
    const count = Math.min(maxColumns, Math.max(Math.ceil(ids.length / maxRows), Math.ceil(Math.sqrt(ids.length * height / COLUMN))));
    const order = [...ids].sort((a, b) => columns.get(a)! - columns.get(b)! || (a < b ? -1 : a > b ? 1 : 0));
    order.forEach((id, i) => positions.set(id, [X + (i % count) * COLUMN, Y + Math.floor(i / count) * height]));
  }
  if ([...positions.values()].some(([x, y]) => !Number.isFinite(x + y) || x < 0 || y < 0 || x > 10000 || y > 10000)) throw new Error('モデルが大きすぎるため自動配置できません');
  const changed = graph.blocks.some(b => b.position.some((n, i) => n !== positions.get(b.id)![i]));
  for (const b of graph.blocks) b.position = positions.get(b.id)!;
  return { mode, changed };
}
