import { test, expect, type Page } from '@playwright/test';
import sample from '../../samples/graphs/continuous-pid.json' with { type: 'json' };
import vector from '../../samples/graphs/vector.json' with { type: 'json' };
import { GRAPH_STORAGE_KEY } from '../src/graph-model';
async function open(page:Page){await page.goto('/');await expect(page.locator('#status')).toContainText('完了');await page.getByRole('tab',{name:'ブロック図エディタ'}).click();await expect(page.locator('#graph-status')).toContainText('完了');}
async function run(page:Page){await page.getByRole('button',{name:'モデルを実行',exact:true}).click();await expect(page.locator('#graph-status')).toContainText('完了');}
async function saved(page:Page){await page.getByRole('button',{name:'モデルを保存',exact:true}).click();return page.evaluate(key=>JSON.parse(localStorage.getItem(key)!).graph,GRAPH_STORAGE_KEY);}
async function upload(page:Page,graph:unknown){await page.locator('#graph-import').setInputFiles({name:'model.json',mimeType:'application/json',buffer:Buffer.from(JSON.stringify(graph))});}
test('port click and keyboard wiring, Escape, occupied inputs and selected wire deletion preserve undo',async({page})=>{
 await open(page);await page.getByRole('button',{name:'新しいモデル',exact:true}).click();
 for(const kind of ['constant','gain','scope']){
  await page.getByLabel('ブロック種類').selectOption(kind);await page.getByRole('button',{name:'ブロックを追加',exact:true}).click();
 }
 await page.getByRole('button',{name:'出力 constant-1:0',exact:true}).click();await expect(page.locator('#graph-wire-preview')).toHaveCount(1);
 await page.keyboard.press('Escape');await expect(page.locator('#graph-wire-preview')).toHaveCount(0);
 await page.getByRole('button',{name:'出力 constant-1:0',exact:true}).focus();await page.keyboard.press('Enter');
 await page.getByRole('button',{name:'入力 gain-1:0',exact:true}).focus();await page.keyboard.press('Enter');
 await expect(page.locator('#graph-status')).toHaveText('ポートを接続しました');
 await page.getByRole('button',{name:'出力 gain-1:0',exact:true}).click();await page.getByRole('button',{name:'入力 scope-1:0',exact:true}).click();
 await run(page);await expect(page.locator('#graph-values')).toContainText('1.00000');const before=await saved(page);
 await page.getByRole('button',{name:'出力 constant-1:0',exact:true}).click();await page.getByRole('button',{name:'入力 gain-1:0',exact:true}).click();
 await expect(page.locator('#graph-status')).toContainText('already connected');await expect(page.locator('#graph-wires .wire')).toHaveCount(2);
 await page.getByRole('button',{name:'配線をキャンセル',exact:true}).click();
 const line=page.getByRole('button',{name:'接続 constant-1:0 → gain-1:0',exact:true});await line.focus();await page.keyboard.press('Enter');await page.keyboard.press('Delete');
 await expect(page.locator('#graph-wires .wire')).toHaveCount(1);await page.getByRole('button',{name:'Undo',exact:true}).click();
 expect((await saved(page)).edges).toEqual(before.edges);expect((await saved(page)).blocks.map((b:{position:number[]})=>b.position)).toEqual(before.blocks.map((b:{position:number[]})=>b.position));
});
test('ungroup retains every numerical sample and configuration, and save/restore retains the unwrapped model',async({page})=>{
 await open(page);const baseline=await saved(page);
 await page.getByLabel('選択 controller',{exact:true}).check();await page.getByLabel('選択 plant',{exact:true}).check();await page.getByRole('button',{name:'サブシステム化',exact:true}).click();
 await run(page);await page.getByRole('button',{name:'グループ解除',exact:true}).click();await expect(page.locator('#graph-status')).toContainText('解除');
 await expect(page.locator('[data-block="subsystem-1-controller"]')).toBeVisible();await run(page);
 const unwrapped=await saved(page);
 const equal=await page.evaluate(async({baseline,unwrapped})=>{const path='/src/wasm/blocksim.js';const m=await import(/* @vite-ignore */ path);await m.default();const a=JSON.parse(m.run_graph(JSON.stringify(baseline))),b=JSON.parse(m.run_graph(JSON.stringify(unwrapped)));return JSON.stringify(a)===JSON.stringify(b);},{baseline,unwrapped});
 expect(equal).toBe(true);expect(unwrapped.blocks.find((b:{id:string})=>b.id==='subsystem-1-controller').params.config).toEqual(sample.blocks[2].params.config);
 await page.getByRole('button',{name:'Undo',exact:true}).click();await expect(page.locator('[data-block="subsystem-1"]')).toBeVisible();
 await page.getByRole('button',{name:'Redo',exact:true}).click();await page.reload();await expect(page.locator('#status')).toContainText('完了');await page.getByRole('tab',{name:'ブロック図エディタ'}).click();await expect(page.locator('[data-block="subsystem-1-controller"]')).toBeVisible();
});
test('additive import preserves existing model, avoids repeated ID collisions, rejects incompatible clocks and supports Undo',async({page})=>{
 await open(page);const baseline=await saved(page);await page.getByLabel('読み込み方法').selectOption('merge');
 await upload(page,vector);await expect(page.locator('#graph-status')).toContainText('モデルを追加しました');await run(page);await expect(page.locator('#graph-status')).toContainText('4001 点');
 const merged=await saved(page);expect(merged.blocks).toHaveLength(12);expect(merged.blocks.slice(0,6)).toEqual(baseline.blocks);expect(merged.edges.slice(0,6)).toEqual(baseline.edges);expect(merged.duration).toBe(40);
 await expect(page.locator('#graph-values')).toContainText('vector-output [0]=-1.00000');
 await upload(page,vector);const twice=await saved(page);expect(new Set(twice.blocks.map((b:{id:string})=>b.id)).size).toBe(18);
 await run(page);await expect(page.locator('#graph-values')).toContainText('import-vector-output-2');
 await upload(page,{...vector,dt:.005});await expect(page.locator('#graph-status')).toContainText('刻み幅');expect(await saved(page)).toEqual(twice);
 await page.getByRole('button',{name:'Undo',exact:true}).click();expect(await saved(page)).toEqual(merged);
});
test('multiport vectors can be wired and ungrouped without changing results',async({page})=>{
 await open(page);
 const block=(id:string,width:number,params:Record<string,unknown>,x:number)=>({id,label:id,width,position:[x,100],params});
 const edge=(from:string,to:string,to_port=0,from_port=0)=>({from,to,to_port,from_port});
 const inner={version:1,dt:.01,duration:1,blocks:[block('a',2,{kind:'input',port:0},30),block('b',1,{kind:'input',port:1},30),block('mux',3,{kind:'mux',inputs:2},270),block('gain',2,{kind:'gain',gain:2},270),block('out0',3,{kind:'output',port:0},500),block('out1',2,{kind:'output',port:1},500)],edges:[edge('a','mux'),edge('b','mux',1),edge('a','gain'),edge('mux','out0'),edge('gain','out1')]};
 const graph={version:1,dt:.01,duration:1,blocks:[block('source0',2,{kind:'constant',value:[2,4]},30),block('source1',1,{kind:'constant',value:[3]},30),block('sub',1,{kind:'subsystem',model:inner},270),block('scope0',3,{kind:'scope'},500),block('scope1',2,{kind:'scope'},740)],edges:[edge('source0','sub'),edge('source1','sub',1),edge('sub','scope0'),edge('sub','scope1',0,1)]};
 await upload(page,graph);await run(page);await expect(page.locator('#graph-values')).toContainText('scope1 [1]=8.00000');
 await page.getByRole('button',{name:'接続を削除 sub scope1 0',exact:true}).click();await page.getByRole('button',{name:'出力 sub:1',exact:true}).click();await page.getByRole('button',{name:'入力 scope1:0',exact:true}).click();await run(page);
 await page.locator('[data-block="sub"]').dblclick();await expect(page.locator('[data-block="gain"]')).toBeVisible();await page.getByRole('button',{name:'ルート',exact:true}).click();
 await page.locator('[data-block="sub"]').click();await page.getByRole('button',{name:'グループ解除',exact:true}).click();await run(page);await expect(page.locator('#graph-values')).toContainText('scope1 [1]=8.00000');await expect(page.locator('#graph-values')).toContainText('scope0 [2]=3.00000');
});
