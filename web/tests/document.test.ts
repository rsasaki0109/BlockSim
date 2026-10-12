import { describe, expect, it } from 'vitest';
import { decodeDocument, encodeDocument, parseLimit, parseNumber, parseVector, validateLimits } from '../src/document';
import sample from '../../samples/pid-recovery.json';
import type { AppDocument } from '../src/types';
// WASM's model validation is covered through the actual browser boundary tests.
const normalize = (json: string) => json;
describe('document transport and frontend validation', () => {
  it('roundtrips all PID fields, AW flag and vector block', () => {
    const doc: AppDocument = { version: 1, model: { ...sample, mode: 'discrete' },
      saturation: { input: [-5, 0, 9], limits: { lower: null, upper: 2 } } };
    expect(decodeDocument(encodeDocument(doc), normalize)).toEqual(doc);
  });
  it('imports the standalone sample with a default vector block', () => {
    expect(decodeDocument(JSON.stringify(sample), normalize).model).toEqual(sample);
  });
  it('rejects unsupported documents without accepting corrupt vector data', () => {
    for (const value of [null, [], { version: 2 }, { version: 1, model: sample, saturation: null },
      { version: 1, model: sample, saturation: { input: [1, '2'], limits: {} } }]) {
      expect(() => decodeDocument(JSON.stringify(value), normalize)).toThrow();
    }
    expect(() => decodeDocument(' '.repeat(1_000_001), normalize)).toThrow('1 MB');
  });
  it('distinguishes absent limits from zero and rejects nonfinite or reversed bounds', () => {
    expect(parseLimit('', 'limit')).toBeNull();
    expect(parseLimit('  ', 'limit')).toBeNull();
    expect(parseLimit('0', 'limit')).toBe(0);
    expect(parseNumber('-1.2e3', 'value')).toBe(-1200);
    for (const v of ['', 'NaN', 'Infinity', '1e999', '1,2']) expect(() => parseNumber(v, 'value')).toThrow();
    expect(validateLimits({})).toEqual({ lower: null, upper: null });
    expect(() => validateLimits({ lower: 3, upper: 2 })).toThrow();
    expect(() => validateLimits({ lower: Infinity })).toThrow();
    expect(() => validateLimits({ upper: '2' })).toThrow();
  });
  it('accepts empty vectors but rejects oversized and nonnumeric vectors', () => {
    expect(parseVector('[]')).toEqual([]);
    expect(parseVector('[1,-2,0]')).toEqual([1,-2,0]);
    for (const v of ['1', '[null]', '["1"]', '[1e999]', JSON.stringify(Array(4097).fill(0))]) {
      expect(() => parseVector(v)).toThrow();
    }
  });
});
