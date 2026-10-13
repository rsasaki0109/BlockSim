import {describe,it,expect} from 'vitest';
import {generateSampleTable,type InputGeneration} from '../src/sampled-input-generator';
const base:InputGeneration={shape:'step',width:2,rows:5,base:[1,-1],delta:[2,-2],step:2};
describe('sampled input table generation',()=>{
 it('broadcasts finite constants and generates independent vector rows up to the table limits',()=>{
  const values=generateSampleTable({shape:'constant',width:64,rows:2001,base:.25,delta:'unused'});expect(values).toHaveLength(2001);expect(values[2000]).toEqual(Array(64).fill(.25));values[0][0]=10;expect(values[1][0]).toBe(.25);
 });
 it('switches exactly at an integer sample including first and last rows',()=>{
  expect(generateSampleTable(base)).toEqual([[1,-1],[1,-1],[3,-3],[3,-3],[3,-3]]);
  expect(generateSampleTable({...base,step:0})).toEqual(Array(5).fill([3,-3]));expect(generateSampleTable({...base,step:4})[3]).toEqual([1,-1]);expect(generateSampleTable({...base,step:4})[4]).toEqual([3,-3]);
 });
 it('uses both ramp endpoints and holds the base for a single-row ramp',()=>{
  expect(generateSampleTable({...base,shape:'ramp'})).toEqual([[1,-1],[1.5,-1.5],[2,-2],[2.5,-2.5],[3,-3]]);
  expect(generateSampleTable({...base,shape:'ramp',rows:1})).toEqual([[1,-1]]);
 });
 it('samples complete sine cycles without duplicating the periodic endpoint',()=>{
  const values=generateSampleTable({...base,shape:'sine',rows:8,cycles:2});const expected=[[1,-1],[3,-3],[1,-1],[-1,1],[1,-1],[3,-3],[1,-1],[-1,1]];
  values.forEach((row,k)=>row.forEach((v,j)=>expect(v).toBeCloseTo(expected[k][j],12)));expect(values).toHaveLength(8);
 });
 it('rejects malformed bounds, vectors, switching indices and undersampled cycles',()=>{
  for(const patch of [{width:0},{width:65},{width:1.5},{rows:0},{rows:2002},{rows:1.5},{base:null},{base:[1]},{base:['1',2]},{base:NaN},{delta:Infinity},{step:-1},{step:5},{step:1.5},{step:NaN},{shape:'bogus'},{shape:'sine',cycles:0},{shape:'sine',cycles:3},{shape:'sine',cycles:1.5},{shape:'sine',rows:1,cycles:1}])expect(()=>generateSampleTable({...base,...patch} as InputGeneration)).toThrow();
 });
 it('rejects generated overflow and excessive JSON size without mutating parameters',()=>{
  const request={...base,base:[1e308,0],delta:[1e308,0]};const before=structuredClone(request);expect(()=>generateSampleTable(request)).toThrow();expect(request).toEqual(before);
  expect(()=>generateSampleTable({shape:'constant',width:64,rows:2001,base:1.2345678901234567e308})).toThrow('1 MB');
 });
});
