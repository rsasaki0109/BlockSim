import {test,expect,type Page} from '@playwright/test';
import {readFile} from 'node:fs/promises';
import sample from '../../samples/frequency/lowpass.json' with {type:'json'};
import heldPlan from '../../samples/discrete/held-comparison-plan.json' with {type:'json'};
import unitPlan from '../../samples/discrete/comparison-plan.json' with {type:'json'};
import type {ConversionReport} from '../src/discretize-comparison';
import {GRAPH_STORAGE_KEY} from '../src/graph-model';
async function open(page:Page,model:unknown=sample){page.on('dialog',d=>d.accept());await page.goto('/');await expect(page.locator('#status')).toContainText('完了');await page.getByRole('tab',{name:'ブロック図エディタ'}).click();await expect(page.locator('#graph-status')).toContainText('完了');await page.locator('#graph-import').setInputFiles({name:'model.json',mimeType:'application/json',buffer:Buffer.from(JSON.stringify(model))});await expect(page.locator('#graph-status')).toContainText('読み込みました');await page.locator('[data-block="plant"]').focus();await page.keyboard.press('Enter');}
async function download(page:Page,id:string){const e=page.waitForEvent('download');await page.locator('#'+id).click();return readFile((await(await e).path())!,'utf8');}
async function run(page:Page):Promise<ConversionReport>{await page.locator('#conversion-run').click();await expect(page.locator('#conversion-state')).toContainText('比較完了');return JSON.parse(await download(page,'conversion-report'));}
async function load(page:Page,value:unknown){await page.locator('#conversion-import').setInputFiles({name:'plan.json',mimeType:'application/json',buffer:Buffer.from(JSON.stringify(value))});}
test('actual held-input drop matches independent pulse equations and post-change differences for both methods',async({page})=>{
 await open(page);await load(page,heldPlan);await expect(page.locator('#conversion-state')).toContainText('読み込みました');const zoh=await run(page);
 const g=(t:number)=>t<0?0:1-(1+t)*Math.exp(-t);
 for(let i=0;i<101;i++){const t=i*.2,exact=g(t)-g(t-2);expect(Math.abs(zoh.continuous[i]-exact)).toBeLessThan(2e-10);expect(Math.abs(zoh.discrete[i]-exact)).toBeLessThan(1e-12);}expect(zoh.post_change_metrics!.maxAbs).toBeLessThan(2e-10);expect(zoh.input!.slice(8,12)).toEqual([1,1,0,0]);
 await page.locator('#graph-discretize-method').selectOption('tustin');const tustin=await run(page);let y1=0,y2=0,u1=0,u2=0;
 for(let k=0;k<101;k++){const u=k<10?1:0,y=18/11*y1-81/121*y2+(u+2*u1+u2)/121;expect(Math.abs(tustin.discrete[k]-y)).toBeLessThan(3e-13);y2=y1;y1=y;u2=u1;u1=u;}
 expect(tustin.post_change_metrics!.maxAbs).toBeGreaterThan(.02);await expect(page.locator('#conversion-metrics')).toContainText('変更以後');await expect(page.locator('#conversion-caption')).toContainText('変更サンプル10 (t=2s)');
 const csv=(await download(page,'conversion-csv')).split('\r\n');expect(csv).toHaveLength(102);expect(csv[11].split(',').slice(6)).toEqual(['0','10','1','0']);
});
test('held-input plans roundtrip, bad or blank inputs retain good results, and old unit-step plans reset the input',async({page})=>{
 await open(page);await load(page,heldPlan);const first=await run(page),planText=await download(page,'conversion-plan');expect(JSON.parse(planText)).toEqual(heldPlan);
 await page.locator('#conversion-after').fill('-.5');await page.locator('#conversion-change').fill('11');expect(JSON.parse(await download(page,'conversion-report'))).toEqual(first);await expect(page.locator('#conversion-caption')).toContainText('入力1→0');
 await load(page,JSON.parse(planText));await expect(page.locator('#conversion-state')).toContainText('読み込みました');expect(await run(page)).toEqual(first);
 await page.locator('#conversion-before').fill('');await page.locator('#conversion-run').click();await expect(page.locator('#conversion-state')).toHaveClass(/error/);expect(JSON.parse(await download(page,'conversion-report'))).toEqual(first);
 await load(page,{...heldPlan,input:{...heldPlan.input,change_sample:1.5}});await expect(page.locator('#conversion-state')).toHaveClass(/error/);expect(JSON.parse(await download(page,'conversion-report'))).toEqual(first);
 await load(page,unitPlan);await expect(page.locator('#conversion-state')).toContainText('読み込みました');await expect(page.locator('#conversion-input')).toHaveValue('unit');await expect(page.locator('#conversion-input-fields')).toBeHidden();const old=await run(page);expect(old.input).toBeUndefined();expect(old.post_change_metrics).toBeUndefined();expect((await download(page,'conversion-csv')).split('\r\n')[0]).toBe('method,sample_time_s,time_s,continuous,discrete,delta');
});
test('negative held input and direct feedthrough change on the exact sample, including final sample, at 320px',async({page})=>{
 await page.setViewportSize({width:320,height:844});const m=structuredClone(sample);m.blocks[1].params={kind:'transfer_function',numerator:[1,2],denominator:[1,1],initial:[0]} as typeof m.blocks[1]['params'];await open(page,m);await page.locator('#graph-save').click();const before=await page.evaluate(k=>localStorage.getItem(k),GRAPH_STORAGE_KEY);
 const p={...heldPlan,request:{numerator:[1,2],denominator:[1,1],sample_time:.2},input:{kind:'held_step',before:2,after:-.5,change_sample:10}};await load(page,p);const r=await run(page);const g=(t:number)=>t<0?0:2-Math.exp(-t);for(let k=0;k<101;k++)expect(Math.abs(r.discrete[k]-(2*g(k*.2)-2.5*g(k*.2-2)))).toBeLessThan(1e-12);expect(Math.abs(r.continuous[10]-r.continuous[9])).toBeGreaterThan(2);expect(await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth)).toBe(false);
 await load(page,{...p,request:{...p.request,sample_time:.1},input:{...p.input,change_sample:3}});const rounded=await run(page);expect(rounded.change_time).toBe(.3);expect(rounded.input![3]).toBe(-.5);for(let k=0;k<101;k++){const exact=2*g(k*.1)+(k>=3?-2.5*g((k-3)*.1):0);expect(Math.abs(rounded.continuous[k]-exact)).toBeLessThan(2e-10);expect(Math.abs(rounded.discrete[k]-exact)).toBeLessThan(1e-12);}
 await load(page,{...p,input:{kind:'held_step',before:0,after:1,change_sample:100}});const last=await run(page);expect(last.discrete.slice(0,100).every(v=>v===0)).toBe(true);expect(last.discrete[100]).toBe(1);expect(last.continuous[100]).toBe(1);expect(last.post_change_metrics!.area).toBe(0);await page.locator('#graph-save').click();expect(await page.evaluate(k=>localStorage.getItem(k),GRAPH_STORAGE_KEY)).toEqual(before);
});
