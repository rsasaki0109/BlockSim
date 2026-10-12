import { expect, it } from 'vitest';
import { compareResults, differenceMetrics, differenceCsv, snapshotRun, sameTimeGrid } from '../src/graph-comparison';
import type { GraphModel, GraphResult, GraphTrace } from '../src/graph-model';
const trace=(id:string,values:number[][],label=id):GraphTrace=>({id,label,values});
const result=(traces:GraphTrace[],times=[0,1]):GraphResult=>({traces,times,solver:'test'});

it('snapshots executed settings and samples independently, while comparison leaves both inputs intact',()=>{
 const model:GraphModel={version:1,dt:1,duration:1,blocks:[],edges:[]};const data=result([trace('scope',[[1],[2]])]),before=structuredClone(data);
 const baseline=snapshotRun(model,data);model.duration=2;data.traces[0].values[0][0]=99;data.times[1]=2;
 expect(baseline.model.duration).toBe(1);expect(baseline.result).toEqual(before);const copy=structuredClone(baseline);compareResults(before,baseline.result);expect(baseline).toEqual(copy);expect(before).toEqual(copy.result);
});
it('uses signed end difference, sample RMSE and trapezoidal absolute area on nonuniform times',()=>{
 const m=differenceMetrics([0,.25,1],[0,2,4],[1,1,1])!;
 expect(m.maxAbs).toBe(3);expect(m.final).toBe(3);expect(m.rms).toBeCloseTo(Math.sqrt(11/3),14);expect(m.area).toBe(1.75);
});
it('avoids artificial square overflow and underflow for large and small finite differences',()=>{
 for(const delta of [1e200,1e-250]){const m=differenceMetrics([0,1],[delta,delta],[0,0])!;expect(m.maxAbs).toBe(delta);expect(m.rms).toBe(delta);expect(m.area).toBe(delta);}
 expect(differenceMetrics([0,1],[1e308,1e308],[-1e308,-1e308])).toBeNull();expect(differenceMetrics([0,10],[1e308,1e308],[0,0])).toBeNull();
});
it('pairs exact hierarchical Scope IDs despite ordering, duplicate labels or renames',()=>{
 const current=result([trace('a/scope',[[5],[6]],'renamed'),trace('b/scope',[[10],[20]],'same')]);
 const baseline=result([trace('b/scope',[[3],[4]],'same'),trace('a/scope',[[1],[2]],'same')]);
 const report=compareResults(current,baseline);expect(report.notices).toEqual([]);expect(report.rows.map(r=>[r.key,r.metrics!.final,r.referenceLabel])).toEqual([['a/scope:0',4,'same'],['b/scope:0',16,'same']]);
});
it('reports every added, removed and width-changed Scope without guessing corresponding signals',()=>{
 const report=compareResults(result([trace('wide',[[1,2],[3,4]]),trace('new',[[1],[2]],'same')]),result([trace('wide',[[1],[2]]),trace('old',[[1],[2]],'same')]));
 expect(report.rows).toEqual([]);expect(report.notices).toHaveLength(3);expect(report.notices.join(' ')).toContain('信号幅');expect(report.notices.join(' ')).toContain('new');expect(report.notices.join(' ')).toContain('old');
});
it('requires identical recorded times, refusing mismatched lengths or even a different floating point time',()=>{
 for(const times of [[0,.5,1],[0,1+Number.EPSILON],[0,2]]){
  expect(sameTimeGrid([0,1],times)).toBe(false);const a=result([trace('s',times.map(()=>[2]))],times),b=result([trace('s',[[1],[1]])]);const report=compareResults(a,b);
  expect(report.sameTimes).toBe(false);expect(report.rows[0].status).toBe('time_grid');expect(report.rows[0].metrics).toBeNull();expect(()=>differenceCsv(a,b,report)).toThrow('記録時刻');
 }
});
it('exports all matched vector differences and all times, with quoted labels and no unmatched columns',()=>{
 const a=result([trace('v',Array.from({length:2},(_,i)=>Array(64).fill(i-2)),'=A,"B"'),trace('new',[[1],[2]])]);const b=result([trace('v',[Array(64).fill(1),Array(64).fill(1)])]);const csv=differenceCsv(a,b,compareResults(a,b));
 const lines=csv.split('\r\n');expect(lines).toHaveLength(3);expect(lines[0]).toContain('"Δ =A,""B"" (v)[63]"');expect(lines[0]).not.toContain('new');expect(lines[1].split(',').slice(1)).toEqual(Array(64).fill('-3'));expect(lines[2].split(',').slice(1)).toEqual(Array(64).fill('-2'));
});
it('self comparison is zero, and an unrepresentable difference disables export instead of producing Infinity',()=>{
 const a=result([trace('s',[[2,-5],[4,1]])]);expect(compareResults(a,a).rows.map(r=>r.metrics)).toEqual(Array(2).fill({maxAbs:0,rms:0,final:0,area:0}));
 const current=result([trace('s',[[1e308],[1e308]])]),baseline=result([trace('s',[[-1e308],[-1e308]])]),report=compareResults(current,baseline);
 expect(report.rows[0].status).toBe('overflow');expect(()=>differenceCsv(current,baseline,report)).toThrow();
});
