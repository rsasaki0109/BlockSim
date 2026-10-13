import {it,expect} from 'vitest';
import {decodeSampleTableCsv,sampleTableCsv,validateSampleTable} from '../src/sampled-input-csv';
it('reads scalar or vector sample rows and exact optional headers with BOM and decimal syntax',()=>{
 expect(decodeSampleTableCsv('\uFEFFinput\r\n+1.25\r\n-.5\r\n',1)).toEqual([[1.25],[-.5]]);
 expect(decodeSampleTableCsv(' input_0, input_1\n0,1e-2\n-2,.5\n',2)).toEqual([[0,.01],[-2,.5]]);
 expect(decodeSampleTableCsv('3',1)).toEqual([[3]]);expect(decodeSampleTableCsv('\n0, 1\n\n',2)).toEqual([[0,1]]);
});
it('refuses ragged rows, wrong headers, empty cells, ambiguous or nonfinite numbers and invalid width',()=>{
 for(const s of ['', 'input', '1,2\n3','input_1,input_0\n1,2','time,input\n0,1','input\n1,2','input_0,input_1,input_2\n1,2,3','1,\n2,3','1,2\n\n3,4','"1",2','0x10,2','NaN,1','Infinity,1','1e999,1','1;2','1\r2'])expect(()=>decodeSampleTableCsv(s,2)).toThrow();
 for(const width of [0,65,1.5,NaN])expect(()=>decodeSampleTableCsv('1',width)).toThrow();
});
it('enforces byte/row/column bounds while accepting 2001 by 64',()=>{
 const rows=Array.from({length:2001},()=>Array(64).fill(.5));expect(decodeSampleTableCsv(sampleTableCsv(rows,64),64)).toEqual(rows);
 expect(()=>decodeSampleTableCsv(Array(2002).fill('1').join('\n'),1)).toThrow();expect(()=>decodeSampleTableCsv(' '.repeat(1_000_001),1)).toThrow(/1 MB/);expect(()=>decodeSampleTableCsv('あ'.repeat(333334),1)).toThrow(/1 MB/);
 expect(()=>sampleTableCsv(Array.from({length:2001},()=>Array(64).fill(1.234567890123456e100)),64)).toThrow(/1 MB/);
});
it('roundtrips finite numeric precision and refuses malformed draft tables before export',()=>{
 const rows=[[1.2345678901234567,1e-300],[-1e300,0]];const csv=sampleTableCsv(rows,2);expect(csv.startsWith('input_0,input_1\r\n')).toBe(true);expect(decodeSampleTableCsv(csv,2)).toEqual(rows);expect(sampleTableCsv([[.75]],1)).toBe('input\r\n0.75');
 for(const value of [[],[[1],[2,3]],[[null]],[[NaN]],[[Infinity]],['1'],Array(2002).fill([0])])expect(()=>validateSampleTable(value,1)).toThrow();
});
