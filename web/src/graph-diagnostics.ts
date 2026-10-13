import { graphContext, type GraphModel } from './graph-model';
import type { GraphSearchHit } from './graph-search';

export const ISSUE_HINTS = {
  structure: ['モデル構造', '設定や階層・接続の形式を確認してください。'],
  empty_graph: ['ブロックがありません', '信号源と Scope を追加して接続してください。'],
  unconnected_input: ['入力が未接続', '表示した入力ポートに出力を接続してください。'],
  duplicate_input: ['入力が重複', '同じ入力への接続を1本にしてください。'],
  signal_width: ['信号幅が不一致', '接続元と入力先の信号幅を合わせてください。'],
  mux_width: ['Mux の幅が不一致', '出力幅を入力幅の合計に合わせてください。'],
  selector_index: ['Selector の範囲外', '抽出インデックスを接続元の成分数の範囲内にしてください。'],
  algebraic_loop: ['直接伝達の循環', 'Integrator・FirstOrder・Delay で循環を切ってください。'],
  missing_scope: ['Scope がありません', '記録する信号に Scope を接続してください。'],
  work_limit: ['計算量の上限', 'ブロック数・信号幅・記録点数を減らしてください。'],
  recording_limit: ['記録量の上限', 'Scope の幅や終了時刻を減らしてください。'],
  runtime_error: ['実行中のエラー', '計算値や設定を確認し、モデル変更後に再実行してください。'],
} as const;
export interface GraphDiagnostic {
  code: keyof typeof ISSUE_HINTS;
  message: string;
  block_id: string|null;
  port: number|null;
  source_id: string|null;
}
export interface GraphValidation { runnable: boolean; diagnostics: GraphDiagnostic[] }
export function diagnosticTarget(root: GraphModel, address: string|null): GraphSearchHit|null {
  if (!address) return null;
  const parts=address.split('/');let model=root;
  for(let i=0;i<parts.length;i++){
    const block=model.blocks.find(b=>b.id===parts[i]);if(!block)return null;
    if(block.params.kind==='state_space'&&i<parts.length-1&&(parts.length!==i+2||!/^(input|output|vector|(?:input|state|rate|output|held)-\d+)$/.test(parts[i+1])))return null;
    if(i===parts.length-1||block.params.kind==='state_space')return {path:parts.slice(0,i),id:block.id,label:block.label,kind:block.params.kind,address:parts.slice(0,i+1).join('/')};
    if(block.params.kind!=='subsystem')return null;model=block.params.model as GraphModel;
  }
  return null;
}
export function diagnosticLinks(root: GraphModel, issue: GraphDiagnostic) {
  const target = diagnosticTarget(root, issue.block_id), source = diagnosticTarget(root, issue.source_id);
  let parentInput: { target: GraphSearchHit; port: number }|null = null;
  if (['unconnected_input','signal_width','duplicate_input'].includes(issue.code) && target?.kind === 'input' && target.path.length) {
    const parent = diagnosticTarget(root, target.path.join('/'));
    const block = graphContext(root, target.path).blocks.find(b => b.id === target.id)!;
    if (parent) parentInput = { target: parent, port: Number(block.params.port) };
  }
  if(['unconnected_input','signal_width','duplicate_input'].includes(issue.code)&&target?.kind==='state_space')parentInput={target,port:0};
  return { target, source, parentInput };
}
