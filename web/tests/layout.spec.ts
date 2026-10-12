import { test, expect, type Page } from '@playwright/test';
import sample from '../../samples/graphs/continuous-pid.json' with { type: 'json' };
import subsystem from '../../samples/graphs/subsystem.json' with { type: 'json' };
import { GRAPH_STORAGE_KEY, type GraphModel } from '../src/graph-model';
async function open(page:Page){page.on('dialog',d=>d.accept());await page.goto('/');await expect(page.locator('#status')).toContainText('完了');await page.getByRole('tab',{name:'ブロック図エディタ'}).click();await expect(page.locator('#graph-status')).toContainText('完了');}
async function upload(page:Page,graph:unknown){await page.locator('#graph-import').setInputFiles({name:'layout.json',mimeType:'application/json',buffer:Buffer.from(JSON.stringify(graph))});await expect(page.locator('#graph-status')).toContainText('読み込みました');}
async function saved(page:Page):Promise<GraphModel>{await page.locator('#graph-save').click();return page.evaluate(key=>JSON.parse(localStorage.getItem(key)!).graph,GRAPH_STORAGE_KEY);}
async function run(page:Page){await page.locator('#graph-run').click();await expect(page.locator('#graph-status')).toContainText('完了');}
async function sameResults(page:Page,a:GraphModel,b:GraphModel){return page.evaluate(async({a,b})=>{const path='/src/wasm/blocksim.js',m=await import(/* @vite-ignore */ path);await m.default();return m.run_graph(JSON.stringify(a))===m.run_graph(JSON.stringify(b));},{a,b});}
test('auto-layout keeps every PID sample unchanged, preserves selection, is one undo step and roundtrips saved positions',async({page})=>{
 await open(page);const scrambled=structuredClone(sample);scrambled.blocks.forEach(b=>{b.position=[500,500];});await upload(page,scrambled);const before=await saved(page);
 await page.locator('[data-block="controller"]').focus();await page.keyboard.press('Enter');await page.locator('#graph-layout').click();await expect(page.locator('#graph-status')).toContainText('信号の流れ');await expect(page.getByLabel('選択 controller',{exact:true})).toBeChecked();await expect(page.locator('#graph-plot path')).toHaveCount(0);
 await run(page);await expect(page.locator('#graph-status')).toContainText('4001 点');const arranged=await saved(page);expect(await sameResults(page,before,arranged)).toBe(true);
 await page.locator('#graph-layout').click();await expect(page.locator('#graph-status')).toHaveText('すでに自動配置されています');await expect(page.locator('#graph-csv')).toBeEnabled();await expect(page.locator('#graph-plot path')).toHaveCount(2);
 await page.locator('#graph-undo').click();expect(await saved(page)).toEqual(before);await page.locator('#graph-redo').click();expect(await saved(page)).toEqual(arranged);
 await page.reload();await expect(page.locator('#status')).toContainText('完了');await page.getByRole('tab',{name:'ブロック図エディタ'}).click();await expect(page.locator('[data-block="controller"]')).toHaveCSS('left',`${arranged.blocks.find(b=>b.id==='controller')!.position[0]}px`);expect(await saved(page)).toEqual(arranged);
});
test('root layout retains nested settings and inner layout changes only the current hierarchy with identical results',async({page})=>{
 await open(page);const graph=structuredClone(subsystem);graph.blocks.forEach(b=>{b.position=[300,300];});graph.blocks[1].params.model!.blocks.forEach(b=>{b.position=[700,700];});await upload(page,graph);const before=await saved(page);
 await page.locator('#graph-layout').click();const root=await saved(page);expect(root.blocks.find(b=>b.id==='amplifier')!.params.model).toEqual(before.blocks.find(b=>b.id==='amplifier')!.params.model);
 await page.locator('[data-block="amplifier"]').focus();await page.keyboard.press('Enter');await page.getByRole('button',{name:'内部を編集',exact:true}).click();await page.locator('#graph-layout').click();const inner=await saved(page);
 expect(inner.blocks.map(b=>b.position)).toEqual(root.blocks.map(b=>b.position));expect(inner.edges).toEqual(root.edges);expect(inner.blocks[1].params.model).not.toEqual(root.blocks[1].params.model);expect(await sameResults(page,before,inner)).toBe(true);
 await page.locator('#graph-undo').click();expect(await saved(page)).toEqual(root);await page.locator('#graph-redo').click();expect(await saved(page)).toEqual(inner);await run(page);await page.getByRole('button',{name:'ルート',exact:true}).click();await expect(page.locator('#graph-values')).toContainText('wave-output');
});
test('empty and incomplete diagrams remain drafts, while a 256-block chain uses the coordinate-bounded grid and executes',async({page})=>{
 await open(page);await page.locator('#graph-empty').click();await page.locator('#graph-layout').click();await expect(page.locator('#graph-status')).toHaveText('すでに自動配置されています');
 await page.getByLabel('ブロック種類').selectOption('gain');await page.locator('#block-add').click();await page.locator('#graph-layout').click();const draft=await saved(page);expect(draft.blocks).toHaveLength(1);expect(draft.edges).toEqual([]);await page.locator('#graph-run').click();await expect(page.locator('#graph-status')).toContainText('unconnected input');
 const blocks=Array.from({length:256},(_,i)=>({id:`b-${String(i).padStart(3,'0')}`,label:`b${i}`,width:1,position:[0,0],params:i===0?{kind:'constant',value:[1]}:i===255?{kind:'scope'}:{kind:'gain',gain:1}}));
 const edges=blocks.slice(1).map((b,i)=>({from:blocks[i].id,from_port:0,to:b.id,to_port:0}));await upload(page,{version:1,dt:.01,duration:.01,blocks,edges});await page.locator('#graph-layout').click();await expect(page.locator('#graph-status')).toContainText('格子');const grid=await saved(page);
 expect(grid.blocks.every(b=>b.position.every(n=>n>=0&&n<=10000))).toBe(true);expect(new Set(grid.blocks.map(b=>b.position.join(','))).size).toBe(256);await run(page);await expect(page.locator('#graph-values')).toContainText('1.00000');
});
