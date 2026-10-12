import { test, expect, type Page } from '@playwright/test';
import { GRAPH_STORAGE_KEY } from '../src/graph-model';
async function open(page:Page){await page.goto('/');await expect(page.locator('#status')).toContainText('完了');await page.getByRole('tab',{name:'ブロック図エディタ'}).click();await expect(page.locator('#graph-status')).toContainText('完了');}
async function saved(page:Page){await page.locator('#graph-save').click();return page.evaluate(key=>JSON.parse(localStorage.getItem(key)!).graph,GRAPH_STORAGE_KEY);}
async function run(page:Page){await page.locator('#graph-run').click();await expect(page.locator('#graph-status')).toContainText('完了');}
async function key(page:Page,keys:string){await page.locator('#graph-fit').focus();await page.keyboard.press(keys);}
test('duplicates a complete feedback model with independent settings and identical numerical traces, undo and restore',async({page})=>{
 await open(page);const baseline=await saved(page);
 await key(page,'Control+a');await key(page,'Control+d');await expect(page.locator('#graph-count')).toContainText('12 blocks · 12 wires');await run(page);
 const doubled=await saved(page);
 expect(await page.evaluate(async graph=>{const path='/src/wasm/blocksim.js';const wasm=await import(/* @vite-ignore */ path);await wasm.default();const r=JSON.parse(wasm.run_graph(JSON.stringify(graph)));return [JSON.stringify(r.traces[0].values)===JSON.stringify(r.traces[2].values),JSON.stringify(r.traces[1].values)===JSON.stringify(r.traces[3].values)];},doubled)).toEqual([true,true]);
 await page.locator('[data-block="copy-controller"]').focus();await page.keyboard.press('Enter');await page.getByLabel('kp',{exact:true}).fill('4');await page.getByRole('button',{name:'ブロック設定を適用'}).click();const changed=await saved(page);
 expect(changed.blocks.find((b:{id:string})=>b.id==='controller').params.config.kp).toBe(2);expect(changed.blocks.find((b:{id:string})=>b.id==='copy-controller').params.config.kp).toBe(4);
 await page.locator('#graph-undo').click();expect(await saved(page)).toEqual(doubled);await page.locator('#graph-undo').click();expect(await saved(page)).toEqual(baseline);await page.locator('#graph-redo').click();expect(await saved(page)).toEqual(doubled);
 await page.reload();await expect(page.locator('#status')).toContainText('完了');await page.getByRole('tab',{name:'ブロック図エディタ'}).click();await expect(page.locator('#graph-count')).toContainText('12 blocks');
});
test('clipboard keeps an immutable snapshot across models, excludes outside edges and respects text editing and clock mismatch',async({page})=>{
 await open(page);await page.getByLabel('選択 controller',{exact:true}).check();await key(page,'Control+c');await expect(page.locator('#graph-status')).toContainText('1 個');
 await page.getByLabel('kp',{exact:true}).fill('7');await page.getByRole('button',{name:'ブロック設定を適用'}).click();
 page.once('dialog',d=>d.accept());await page.locator('#graph-empty').click();await key(page,'Control+v');const pasted=await saved(page);
 expect(pasted.blocks[0].params.config.kp).toBe(2);expect(pasted.edges).toEqual([]);
 await page.getByLabel('表示名',{exact:true}).fill('draft');await page.keyboard.press('Control+a');await page.keyboard.press('Control+c');await expect(page.locator('#graph-count')).toContainText('1 blocks');
 await page.getByLabel('グラフ刻み幅 (s)').fill('.02');await page.getByRole('button',{name:'時間設定を適用'}).click();await key(page,'Control+v');await expect(page.locator('#graph-status')).toContainText('刻み幅');await expect(page.locator('#graph-count')).toContainText('1 blocks');
});
test('zoomed dragging uses model coordinates, fitting includes feedback paths, and view changes keep saved data clean',async({page})=>{
 await open(page);const initial=await saved(page);
 for(let i=0;i<5;i++)await page.getByRole('button',{name:'縮小',exact:true}).click();await expect(page.locator('#graph-zoom')).toHaveText('50%');
 const node=page.locator('[data-block="reference"]'),box=(await node.boundingBox())!;
 await page.mouse.move(box.x+35,box.y+25);await page.mouse.down();await page.mouse.move(box.x+65,box.y+45,{steps:4});await page.mouse.up();
 const moved=await saved(page);expect(moved.blocks[0].position[0]).toBeCloseTo(initial.blocks[0].position[0]+60,6);expect(moved.blocks[0].position[1]).toBeCloseTo(initial.blocks[0].position[1]+40,6);
 await page.locator('#graph-fit').click();
 const viewport=(await page.locator('.graph-viewport').boundingBox())!;
 for(const b of await page.locator('.graph-node, .wire').all()){const rect=(await b.boundingBox())!;expect(rect.x).toBeGreaterThanOrEqual(viewport.x);expect(rect.y).toBeGreaterThanOrEqual(viewport.y);expect(rect.x+rect.width).toBeLessThanOrEqual(viewport.x+viewport.width);expect(rect.y+rect.height).toBeLessThanOrEqual(viewport.y+viewport.height);}
 await expect(page.locator('#graph-dirty')).toHaveText('変更なし');expect(await saved(page)).toEqual(moved);
 await page.getByRole('button',{name:'100% 表示'}).click();await expect(page.locator('#graph-zoom')).toHaveText('100%');
 for(let i=0;i<10;i++)await page.getByRole('button',{name:'拡大',exact:true}).click();await expect(page.locator('#graph-zoom')).toHaveText('200%');await node.scrollIntoViewIfNeeded();const large=(await node.boundingBox())!;
 await page.mouse.move(large.x+65,large.y+45);await page.mouse.down();await page.mouse.move(large.x+85,large.y+65,{steps:4});await page.mouse.up();const enlarged=await saved(page);
 expect(enlarged.blocks[0].position[0]).toBeCloseTo(moved.blocks[0].position[0]+10,6);expect(enlarged.blocks[0].position[1]).toBeCloseTo(moved.blocks[0].position[1]+10,6);
 await page.locator('#graph-undo').click();expect(await saved(page)).toEqual(moved);await page.locator('#graph-undo').click();expect(await saved(page)).toEqual(initial);
});
test('unsaved replacements can be cancelled, invalid imports retain edits, undo and save clear dirty state, and beforeunload is guarded',async({page})=>{
 await open(page);await expect(page.locator('#graph-dirty')).toHaveText('変更なし');await page.locator('#block-add').click();await expect(page.locator('#graph-dirty')).toHaveText('未保存の変更あり');
 let dialogs=0;page.on('dialog',async d=>{dialogs++;expect(d.type()).toBe('confirm');await d.dismiss();});
 await page.locator('#graph-empty').click();await expect(page.locator('#graph-count')).toContainText('7 blocks');await expect(page.locator('#graph-status')).toContainText('中止');
 await page.locator('#graph-import').setInputFiles({name:'bad.json',mimeType:'application/json',buffer:Buffer.from('{broken')});await expect(page.locator('#graph-status')).toHaveClass('error');expect(dialogs).toBe(1);
 const blocked=()=>page.evaluate(()=>{const event=new Event('beforeunload',{cancelable:true});window.dispatchEvent(event);return event.defaultPrevented;});expect(await blocked()).toBe(true);
 await page.locator('#graph-undo').click();await expect(page.locator('#graph-dirty')).toHaveText('変更なし');expect(await blocked()).toBe(false);
 await page.locator('#graph-redo').click();await saved(page);expect(await blocked()).toBe(false);await page.locator('#graph-undo').click();expect(await blocked()).toBe(true);
 await page.locator('#graph-restore').click();await expect(page.locator('#graph-count')).toContainText('6 blocks'); // Dismiss leaves the current model intact.
 expect(dialogs).toBe(2);page.removeAllListeners('dialog');page.once('dialog',d=>d.accept());await page.locator('#graph-restore').click();await expect(page.locator('#graph-count')).toContainText('7 blocks');await expect(page.locator('#graph-dirty')).toHaveText('変更なし');
 await page.getByRole('tab',{name:'PID Lab',exact:true}).click();await page.getByRole('tab',{name:'ブロック図エディタ'}).click();await expect(page.locator('#graph-count')).toContainText('7 blocks');
 await page.locator('#block-add').click();const leaving=page.waitForEvent('dialog');page.once('dialog',d=>d.dismiss());await page.locator('header a').click({noWaitAfter:true});expect((await leaving).type()).toBe('beforeunload');await expect(page.locator('#graph-count')).toContainText('8 blocks');
});
test('fitting retains ports at zero coordinates, large feedback routes and only the scaled scroll extent',async({page})=>{
 await open(page);
 const block=(id:string,x:number,params:Record<string,unknown>)=>({id,label:id,width:1,position:[x,0],params});
 const graph={version:1,dt:.01,duration:.01,blocks:[block('sum',0,{kind:'sum',signs:Array(64).fill(1)}),block('source',300,{kind:'constant',value:[2]}),block('scope',600,{kind:'scope'})],edges:[...Array.from({length:64},(_,to_port)=>({from:'source',from_port:0,to:'sum',to_port})),{from:'sum',from_port:0,to:'scope',to_port:0}]};
 await page.getByRole('tab',{name:'PID Lab',exact:true}).click();
 await page.locator('#graph-import').setInputFiles({name:'ports.json',mimeType:'application/json',buffer:Buffer.from(JSON.stringify(graph))});await expect(page.locator('#graph-status')).toContainText('モデル JSON を読み込みました');
 await page.getByRole('tab',{name:'ブロック図エディタ'}).click();await page.locator('#graph-fit').click();
 const view=(await page.locator('.graph-viewport').boundingBox())!;
 for(const item of await page.locator('.node-port, .wire').all()){const r=(await item.boundingBox())!;expect(r.x).toBeGreaterThanOrEqual(view.x);expect(r.y).toBeGreaterThanOrEqual(view.y);expect(r.x+r.width).toBeLessThanOrEqual(view.x+view.width);expect(r.y+r.height).toBeLessThanOrEqual(view.y+view.height);}
 const layout=await page.evaluate(()=>{const view=document.querySelector<HTMLElement>('.graph-viewport')!,surface=document.getElementById('graph-surface')!,board=document.getElementById('graph-board')!;return {scrollWidth:view.scrollWidth,scrollHeight:view.scrollHeight,surfaceWidth:surface.clientWidth,surfaceHeight:surface.clientHeight,viewWidth:view.clientWidth,viewHeight:view.clientHeight,boardHeight:board.clientHeight};});
 expect(layout.scrollWidth).toBeLessThanOrEqual(Math.max(layout.viewWidth,layout.surfaceWidth)+1);expect(layout.scrollHeight).toBeLessThanOrEqual(Math.max(layout.viewHeight,layout.surfaceHeight)+1);expect(layout.boardHeight).toBeGreaterThan(1900);
 await run(page);await expect(page.locator('#graph-values')).toContainText('128.000');
});
