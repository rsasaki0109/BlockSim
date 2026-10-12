import { inputCount, outputCount, outputWidth, interfaceBlocks, setClock, solverKey, type GraphBlock, type GraphModel, type Edge } from './graph-model';

export function inputWidth(block: GraphBlock, port: number): number {
  return block.params.kind === 'subsystem' ? interfaceBlocks(block, 'input')[port].width : block.width;
}
/** One occupied input is never replaced implicitly. Editing remains transactional. */
export function connectPorts(graph: GraphModel, edge: Edge): void {
  const source = graph.blocks.find(b => b.id === edge.from);
  const target = graph.blocks.find(b => b.id === edge.to);
  if (!source || !target) throw new Error('接続するブロックがありません');
  if (!Number.isInteger(edge.from_port) || edge.from_port < 0 || edge.from_port >= outputCount(source)
      || !Number.isInteger(edge.to_port) || edge.to_port < 0 || edge.to_port >= inputCount(target)) {
    throw new Error('有効なポートを指定してください');
  }
  if (graph.edges.some(e => e.to === edge.to && e.to_port === edge.to_port)) {
    throw new Error(`${edge.to}:${edge.to_port}: input already connected — 既存の接続を削除してください`);
  }
  const width = outputWidth(source, edge.from_port);
  if (target.params.kind === 'mux') {
    const incoming = graph.edges.filter(e => e.to === target.id);
    const total = incoming.reduce((n, e) => n + outputWidth(graph.blocks.find(b => b.id === e.from)!, e.from_port), width);
    if (total > target.width || (incoming.length + 1 === inputCount(target) && total !== target.width)) {
      throw new Error(`${target.id}: Mux の入力幅合計 ${total} と出力幅 ${target.width} が一致しません`);
    }
  } else if (target.params.kind === 'selector') {
    if ((target.params.indices as number[]).some(i => i >= width)) throw new Error(`${target.id}: Selector index outside source width`);
  } else if (width !== inputWidth(target, edge.to_port)) {
    throw new Error(`${target.id}: signal width mismatch — 接続元 ${width} / 接続先 ${inputWidth(target, edge.to_port)}`);
  }
  graph.edges.push({ ...edge });
}
function freshId(base: string, used: Set<string>): string {
  const stem = base.slice(0, 64);
  let id = stem, n = 2;
  while (used.has(id)) { const suffix = `-${n++}`; id = stem.slice(0, 64 - suffix.length) + suffix; }
  used.add(id); return id;
}
/** A detached snapshot: only edges whose two endpoints are selected are copied. */
export function copySelection(graph: GraphModel, ids: Set<string>): GraphModel {
  const blocks = graph.blocks.filter(b => ids.has(b.id));
  if (!blocks.length) throw new Error('コピーするブロックを選択してください');
  if (blocks.some(b => b.params.kind === 'input' || b.params.kind === 'output')) {
    throw new Error('境界 Input / Output はコピーできません。内部のブロックを選択してください');
  }
  return structuredClone({ ...graph, blocks, edges: graph.edges.filter(e => ids.has(e.from) && ids.has(e.to)) });
}
export function pasteSelection(graph: GraphModel, snapshot: GraphModel, offset = 40): string[] {
  if(solverKey(graph)!==solverKey(snapshot))throw new Error('コピー元と貼り付け先の計算方式・許容誤差を一致させてください');
  if (snapshot.dt !== graph.dt) throw new Error('コピー元と貼り付け先の刻み幅を一致させてください');
  if (snapshot.blocks.some(b => b.params.kind === 'input' || b.params.kind === 'output')) throw new Error('境界ポートは貼り付けできません');
  const used = new Set(graph.blocks.map(b => b.id)), mapping = new Map<string, string>();
  const copy = structuredClone(snapshot);setClock(copy, graph.dt, graph.duration);
  for (const b of copy.blocks) mapping.set(b.id, freshId(`copy-${b.id}`, used));
  // Apply one common translation, retaining layout even near the coordinate limit.
  const shiftX = Math.min(offset, 10000 - Math.max(0, ...copy.blocks.map(b => b.position[0])));
  const shiftY = Math.min(offset, 10000 - Math.max(0, ...copy.blocks.map(b => b.position[1])));
  for (const b of copy.blocks) { b.id = mapping.get(b.id)!;b.position = [b.position[0] + shiftX, b.position[1] + shiftY]; }
  graph.blocks.push(...copy.blocks);
  graph.edges.push(...copy.edges.map(e => ({ ...e, from: mapping.get(e.from)!, to: mapping.get(e.to)! })));
  return copy.blocks.map(b => b.id);
}
/** Add a graph without changing existing IDs, edges or the destination clock. */
export function mergeGraph(graph: GraphModel, incoming: GraphModel): string[] {
  if(solverKey(graph)!==solverKey(incoming))throw new Error('追加するモデルの計算方式・許容誤差を既存モデルと一致させてください');
  if (incoming.dt !== graph.dt) throw new Error('追加するモデルの刻み幅を既存モデルと一致させてください');
  const used = new Set(graph.blocks.map(b => b.id)), mapping = new Map<string, string>();
  const added = structuredClone(incoming.blocks);
  for (const b of added) mapping.set(b.id, freshId(`import-${b.id}`, used));
  const maxY = graph.blocks.length ? Math.max(...graph.blocks.map(b => b.position[1])) : -150;
  const minY = added.length ? Math.min(...added.map(b => b.position[1])) : 0;
  const shift = Math.max(0, maxY + 170 - minY);
  for (const b of added) {
    b.id = mapping.get(b.id)!;
    b.position[1] = Math.min(10000, b.position[1] + shift);
  }
  graph.blocks.push(...added);
  graph.edges.push(...incoming.edges.map(e => ({ ...e, from: mapping.get(e.from)!, to: mapping.get(e.to)! })));
  return added.map(b => b.id);
}
/** Inline a subsystem, resolving interface identities including pass-throughs. */
export function ungroupBlock(graph: GraphModel, id: string): string[] {
  const subsystem = graph.blocks.find(b => b.id === id);
  if (subsystem?.params.kind !== 'subsystem') throw new Error('解除するサブシステムを1つ選択してください');
  const inner = subsystem.params.model as GraphModel;
  const used = new Set(graph.blocks.map(b => b.id)), mapping = new Map<string, string>();
  for (const b of inner.blocks) mapping.set(b.id, freshId(`${id}-${b.id}`, used));
  const virtual = new Set(inner.blocks.filter(b => b.params.kind === 'input' || b.params.kind === 'output').map(b => mapping.get(b.id)!));
  const inputs = interfaceBlocks(subsystem, 'input'), outputs = interfaceBlocks(subsystem, 'output');
  const edges: Edge[] = [
    ...graph.edges.map(e => ({ ...e,
      from: e.from === id ? mapping.get(outputs[e.from_port].id)! : e.from,
      from_port: e.from === id ? 0 : e.from_port,
      to: e.to === id ? mapping.get(inputs[e.to_port].id)! : e.to,
      to_port: e.to === id ? 0 : e.to_port,
    })),
    ...inner.edges.map(e => ({ ...e, from: mapping.get(e.from)!, to: mapping.get(e.to)! })),
  ];
  const resolve = (source: { from: string; from_port: number }): { from: string; from_port: number } | null => {
    const seen = new Set<string>(); let result = source;
    while (virtual.has(result.from)) {
      if (seen.has(result.from)) throw new Error('境界ポートだけの循環は解除できません');
      seen.add(result.from);
      const predecessors = edges.filter(e => e.to === result.from);
      if (predecessors.length > 1) throw new Error('境界ポートに複数の入力があります。接続を修正してください');
      if (!predecessors.length) return null; // An unfinished input remains unfinished.
      result = { from: predecessors[0].from, from_port: predecessors[0].from_port };
    }
    return result;
  };
  // Reject disconnected boundary cycles too, rather than silently erasing them.
  for (const from of virtual) resolve({ from, from_port: 0 });
  const rewired = edges.filter(e => !virtual.has(e.to)).flatMap(e => {
    const source = resolve(e); return source ? [{ ...e, ...source }] : [];
  });
  const material = structuredClone(inner.blocks.filter(b => !virtual.has(mapping.get(b.id)!)));
  const minX = material.length ? Math.min(...material.map(b => b.position[0])) : 0;
  const minY = material.length ? Math.min(...material.map(b => b.position[1])) : 0;
  for (const b of material) {
    b.id = mapping.get(b.id)!;
    b.position = [Math.max(0, Math.min(10000, b.position[0] - minX + subsystem.position[0])),
      Math.max(0, Math.min(10000, b.position[1] - minY + subsystem.position[1]))];
  }
  graph.blocks = graph.blocks.filter(b => b.id !== id).concat(material);
  graph.edges = rewired;
  return material.map(b => b.id);
}
export function nodeHeight(block: GraphBlock): number { return Math.max(84, 40 + 22 * Math.max(inputCount(block), outputCount(block))); }
export function portY(port: number): number { return 35 + port * 22; }
export function wirePath(a: GraphBlock, fromPort: number, b: GraphBlock, toPort: number): string {
  const x1 = a.position[0] + 166, y1 = a.position[1] + portY(fromPort), x2 = b.position[0], y2 = b.position[1] + portY(toPort);
  if (x2 > x1 + 25) return `M${x1},${y1} C${x1+70},${y1} ${x2-70},${y2} ${x2},${y2}`;
  const lane = Math.max(a.position[1]+nodeHeight(a), b.position[1]+nodeHeight(b)) + 35 + 8 * (fromPort + toPort);
  return `M${x1},${y1} L${x1+28},${y1} L${x1+28},${lane} L${x2-28},${lane} L${x2-28},${y2} L${x2},${y2}`;
}
