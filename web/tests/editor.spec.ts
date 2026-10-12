import { test, expect, type Page } from '@playwright/test';
import { readFile } from 'node:fs/promises';
import vector from '../../samples/graphs/vector.json' with { type: 'json' };
import subsystem from '../../samples/graphs/subsystem.json' with { type: 'json' };
import { GRAPH_STORAGE_KEY } from '../src/graph-model';
async function open(page:Page){await page.goto('/');await expect(page.locator('#status')).toContainText('完了');await page.getByRole('tab',{name:'ブロック図エディタ'}).click();await expect(page.locator('#graph-status')).toContainText('完了');}
async function add(page:Page,kind:string){await page.getByLabel('ブロック種類').selectOption(kind);await page.getByRole('button',{name:'ブロックを追加',exact:true}).click();}
async function connect(page:Page,from:string,to:string,toPort='0',fromPort='0'){
  await page.getByRole('combobox',{name:'接続元',exact:true}).selectOption(from);await page.getByRole('combobox',{name:'接続先',exact:true}).selectOption(to);
  await page.getByRole('combobox',{name:'出力ポート',exact:true}).selectOption(fromPort);await page.getByRole('combobox',{name:'入力ポート',exact:true}).selectOption(toPort);
  await page.getByRole('button',{name:'接続する',exact:true}).click();await expect(page.locator('#graph-status')).toContainText('接続を追加');
}
async function run(page:Page){await page.getByRole('button',{name:'モデルを実行',exact:true}).click();await expect(page.locator('#graph-status')).toContainText('完了');}
async function chain(page:Page){
  await open(page);await page.getByRole('button',{name:'新しいモデル',exact:true}).click();
  await add(page,'constant');await page.getByLabel('値 (JSON 配列)',{exact:true}).fill('[2]');await page.getByRole('button',{name:'ブロック設定を適用'}).click();
  await add(page,'gain');await page.getByLabel('ゲイン',{exact:true}).fill('3');await page.getByRole('button',{name:'ブロック設定を適用'}).click();
  await add(page,'scope');await connect(page,'constant-1','gain-1');await connect(page,'gain-1','scope-1');await run(page);
}
test('builds an arbitrary diagram, edits gain, undo/redo, cursor and CSV',async({page})=>{
  await chain(page);await expect(page.locator('#graph-values')).toContainText('Scope [0]=6.00000');
  await expect(page.locator('#graph-plot path')).toHaveCount(1);await expect(page.locator('#graph-wires path')).toHaveCount(2);
  await page.locator('[data-block="gain-1"]').click();await page.getByLabel('ゲイン',{exact:true}).fill('4');await page.getByRole('button',{name:'ブロック設定を適用'}).click();
  await expect(page.locator('#graph-plot path')).toHaveCount(0);await run(page);await expect(page.locator('#graph-values')).toContainText('8.00000');
  await page.getByRole('button',{name:'Undo',exact:true}).click();await run(page);await expect(page.locator('#graph-values')).toContainText('6.00000');
  await page.getByRole('button',{name:'Redo',exact:true}).click();await run(page);await expect(page.locator('#graph-values')).toContainText('8.00000');
  await page.locator('#graph-cursor').fill('0');await expect(page.locator('#graph-values')).toContainText('t=0.0000s');
  const downloadPromise=page.waitForEvent('download');await page.getByRole('button',{name:'CSV を書き出す'}).click();const download=await downloadPromise;const text=await readFile((await download.path())!,'utf8');
  expect(text).toContain('"Scope (scope-1)[0]"');expect(text).toContain('0,8');expect(text).toContain('5,8');
});
test('groups a connected block, edits subsystem internals and restores numerical equivalence',async({page})=>{
  await chain(page);await page.locator('[data-block="gain-1"] input').check();
  // Scope remains selected after adding it; clear it so only Gain is grouped.
  await page.locator('[data-block="scope-1"] input').uncheck();
  await page.getByRole('button',{name:'サブシステム化',exact:true}).click();
  await expect(page.locator('[data-block="subsystem-1"]')).toBeVisible();await run(page);await expect(page.locator('#graph-values')).toContainText('6.00000');
  await page.getByRole('button',{name:'内部を編集',exact:true}).click();await expect(page.locator('[data-block="boundary-in-0"]')).toBeVisible();
  await page.locator('[data-block="gain-1"]').click();await page.getByLabel('ゲイン',{exact:true}).fill('5');await page.getByRole('button',{name:'ブロック設定を適用'}).click();await run(page);
  await expect(page.locator('#graph-values')).toContainText('10.0000');await page.getByRole('button',{name:'Undo',exact:true}).click();await run(page);await expect(page.locator('#graph-values')).toContainText('6.00000');
  await page.getByRole('button',{name:'ルート',exact:true}).click();await expect(page.locator('[data-block="subsystem-1"]')).toBeVisible();
});
test('sample models run real WASM, vectors toggle traces, mixed clocks and nested JSON roundtrip',async({page})=>{
  await open(page);
  for(const name of ['vector','mixed','subsystem']){
    await page.getByLabel('モデルサンプル').selectOption(name);await page.getByRole('button',{name:'サンプルを読み込む',exact:true}).click();await expect(page.locator('#graph-status')).toContainText('完了');
    if(name==='vector'){
      await expect(page.locator('#graph-plot path')).toHaveCount(4);await expect(page.locator('#graph-values')).toContainText('vector-output [0]=-1.00000');
      await page.getByLabel('表示 vector-output [0]',{exact:true}).uncheck();await expect(page.locator('#graph-plot path')).toHaveCount(3);
    }
    if(name==='mixed'){await expect(page.locator('#graph-status')).toContainText('4001 点');await expect(page.locator('#graph-plot path')).toHaveCount(3);}
  }
  const pending=page.waitForEvent('download');await page.getByRole('button',{name:'モデル JSON',exact:true}).click();const file=await(await pending).path();
  await page.getByRole('button',{name:'新しいモデル',exact:true}).click();await page.locator('#graph-import').setInputFiles(file!);await run(page);await expect(page.locator('#graph-values')).toContainText('wave-output');
  const reports=await page.evaluate(async({vector,subsystem})=>{
    const path='/src/wasm/blocksim.js';const m=await import(/* @vite-ignore */ path);await m.default();
    return [vector,subsystem].map(graph=>JSON.parse(m.run_graph(JSON.stringify(graph))));
  },{vector,subsystem});
  expect(reports[0].traces[0].values[0]).toEqual([-1,1,1]);expect(reports[1].traces[0].values[50][0]).toBeCloseTo(3,10);
});
test('saves drafts, restores positions, rejects invalid import and connection, cancels work',async({page})=>{
  await chain(page);
  const block=page.locator('[data-block="constant-1"]'),box=(await block.boundingBox())!;
  await page.mouse.move(box.x+45,box.y+40);await page.mouse.down();await page.mouse.move(box.x+90,box.y+70,{steps:5});await page.mouse.up();
  await page.getByRole('button',{name:'モデルを保存',exact:true}).click();
  const saved=await page.evaluate(key=>JSON.parse(localStorage.getItem(key)!),GRAPH_STORAGE_KEY);expect(saved.graph.blocks[0].position).toEqual([95,130]);
  await page.reload();await expect(page.locator('#status')).toContainText('完了');await page.getByRole('tab',{name:'ブロック図エディタ'}).click();await expect(page.locator('#graph-status')).toContainText('完了');
  await expect(page.locator('[data-block="constant-1"]')).toHaveCSS('left','95px');
  await page.getByRole('combobox',{name:'接続元',exact:true}).selectOption('constant-1');await page.getByRole('combobox',{name:'接続先',exact:true}).selectOption('gain-1');await page.getByRole('button',{name:'接続する',exact:true}).click();
  await expect(page.locator('#graph-status')).toContainText('already connected');await expect(page.locator('#graph-wires path')).toHaveCount(2);
  await page.locator('#graph-import').setInputFiles({name:'bad.json',mimeType:'application/json',buffer:Buffer.from('{"version":9,"graph":{}}')});await expect(page.locator('#graph-status')).toContainText('未対応');
  await expect(page.locator('[data-block="constant-1"]')).toBeVisible();
  await page.getByRole('button',{name:'接続を削除 gain-1 scope-1 0',exact:true}).click();await page.getByRole('button',{name:'モデルを実行',exact:true}).click();await expect(page.locator('#graph-status')).toContainText('unconnected input');
  await page.getByRole('button',{name:'モデルを保存',exact:true}).click();await expect(page.locator('#graph-status')).toContainText('保存');
  await page.getByRole('button',{name:'Undo',exact:true}).click();
  // Run and immediately cancel before the worker finishes initializing.
  await page.evaluate(()=>{(document.getElementById('graph-run') as HTMLButtonElement).click();(document.getElementById('graph-cancel') as HTMLButtonElement).click();});
  await expect(page.locator('#graph-status')).toHaveText('実行を中止しました');await expect(page.locator('#graph-run')).toBeEnabled();
});
test('algebraic loop and vector width mismatch are visible actionable errors',async({page})=>{
  await open(page);const g=structuredClone(vector);g.blocks[1].width=2;
  await page.locator('#graph-import').setInputFiles({name:'width.json',mimeType:'application/json',buffer:Buffer.from(JSON.stringify(g))});await page.getByRole('button',{name:'モデルを実行',exact:true}).click();await expect(page.locator('#graph-status')).toContainText('scale: signal width mismatch');
  const loop={version:1,dt:.01,duration:1,blocks:[{id:'gain',label:'gain',width:1,position:[20,20],params:{kind:'gain',gain:1}},{id:'scope',label:'scope',width:1,position:[220,20],params:{kind:'scope'}}],edges:[{from:'gain',from_port:0,to:'gain',to_port:0},{from:'gain',from_port:0,to:'scope',to_port:0}]};
  await page.locator('#graph-import').setInputFiles({name:'loop.json',mimeType:'application/json',buffer:Buffer.from(JSON.stringify(loop))});await page.getByRole('button',{name:'モデルを実行',exact:true}).click();await expect(page.locator('#graph-status')).toContainText('Algebraic loop');
});
