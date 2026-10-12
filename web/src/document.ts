import type { AppDocument, ModelConfig, Limits } from './types';
export const STORAGE_KEY = 'blocksim.document.v1';
const MAX_BYTES = 1_000_000;
export function parseNumber(text: string, label: string): number {
  if (!text.trim()) throw new Error(`${label}: 値を入力してください`);
  const value = Number(text);
  if (!Number.isFinite(value)) throw new Error(`${label}: 有限の数値を入力してください`);
  return value;
}
export function parseLimit(text: string, label: string): number | null {
  return text.trim() ? parseNumber(text, label) : null;
}
export function parseVector(text: string): number[] {
  const value: unknown = JSON.parse(text);
  if (!Array.isArray(value) || value.length > 4096 || value.some(v => typeof v !== 'number' || !Number.isFinite(v))) {
    throw new Error('ベクトルは有限数値の JSON 配列（最大4096要素）にしてください');
  }
  return value as number[];
}
export function validateLimits(value: unknown): Limits {
  if (!value || typeof value !== 'object') throw new Error('制限値の形式が不正です');
  const obj = value as Record<string, unknown>;
  if (Object.keys(obj).some(k => k !== 'lower' && k !== 'upper')) throw new Error('未知の制限設定があります');
  const limit = (v: unknown): number | null => {
    if (v === null || v === undefined) return null;
    if (typeof v !== 'number' || !Number.isFinite(v)) throw new Error('制限値は有限数値または null にしてください');
    return v;
  };
  const lower = limit(obj.lower), upper = limit(obj.upper);
  if (lower !== null && upper !== null && lower > upper) throw new Error('下限は上限以下にしてください');
  return { lower, upper };
}
export function decodeDocument(text: string, normalizeModel: (json: string) => string): AppDocument {
  if (new TextEncoder().encode(text).length > MAX_BYTES) throw new Error('設定ファイルは1 MB以下にしてください');
  const value: unknown = JSON.parse(text);
  if (!value || typeof value !== 'object' || Array.isArray(value)) throw new Error('設定形式が不正です');
  const obj = value as Record<string, unknown>;
  if (!('version' in obj)) {
    return { version: 1, model: JSON.parse(normalizeModel(JSON.stringify(obj))) as ModelConfig,
      saturation: { input: [-2, 0, 0.5, 2], limits: { lower: 0, upper: 1 } } };
  }
  if (obj.version !== 1) throw new Error('未対応の設定バージョンです');
  if (Object.keys(obj).some(k => !['version', 'model', 'saturation'].includes(k))) throw new Error('未知の設定があります');
  const s = obj.saturation as Record<string, unknown> | undefined;
  if (!s || typeof s !== 'object' || Array.isArray(s)) throw new Error('Saturation 設定がありません');
  if (Object.keys(s).some(k => k !== 'input' && k !== 'limits')) throw new Error('未知の Saturation 設定があります');
  const input = parseVector(JSON.stringify(s.input));
  const limits = validateLimits(s.limits);
  const model = JSON.parse(normalizeModel(JSON.stringify(obj.model))) as ModelConfig;
  return { version: 1, model, saturation: { input, limits } };
}
export function encodeDocument(doc: AppDocument): string { return JSON.stringify(doc, null, 2); }
