// Reproducible README screencasts from the real app. Only capture layout,
// captions, pointer and line-reveal styling are changed; all models/results
// come from app handlers and the shipped WASM. No application source edits.
import assert from 'node:assert/strict';
import { chromium, expect } from '@playwright/test';
import { spawn } from 'node:child_process';
import { mkdir, mkdtemp, rm, stat, writeFile } from 'node:fs/promises';
import { existsSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
const WEB=resolve(dirname(fileURLToPath(import.meta.url)),'..');
const ROOT=resolve(WEB,'..');
const MEDIA=join(ROOT,'docs/media');
const TMP=await mkdtemp(join(tmpdir(),'blocksim-demos-'));
const URL='http://127.0.0.1:5180';
const sleep=ms=>new Promise(resolve=>setTimeout(resolve,ms));
const selected=process.argv.slice(2);
const server=spawn(process.execPath,[join(WEB,'node_modules/vite/bin/vite.js'),'--host','127.0.0.1','--port','5180','--strictPort'],{cwd:WEB,stdio:['ignore','pipe','pipe']});
let serverErrors='';server.stderr.on('data',b=>serverErrors+=b.toString());
let browser;
const manifest=[];
try {
  for(let i=0;i<100;i++) {
    if(server.exitCode!==null)throw new Error(`Capture server failed: ${serverErrors}`);
    try { if((await fetch(URL)).ok)break; } catch {}
    if(i===99)throw new Error('Capture server readiness timed out');
    await sleep(100);
  }
  browser=await chromium.launch({executablePath:process.env.CHROMIUM_PATH||(existsSync('/usr/bin/chromium')?'/usr/bin/chromium':undefined),args:['--no-sandbox']});
  await mkdir(MEDIA,{recursive:true});
  const CSS=`
    header,.app-tabs,main>.intro,footer{display:none!important}main{max-width:none;padding:10px 26px!important}
    body{overflow:hidden} .capture-banner{height:94px;padding:18px 28px;border-bottom:1px solid #2a3c52;display:flex;align-items:center;justify-content:space-between;background:linear-gradient(115deg,#102528,#111a2a 72%)}
    .capture-brand{font-size:14px;color:#63e3bc;letter-spacing:2px;font-weight:700}.capture-title{font-size:28px;font-weight:750;letter-spacing:-.8px;line-height:1.25;color:#ecf5ff}.capture-sub{font-size:12px;color:#9fb7cd;margin-top:3px}.capture-chip{border:1px solid #3b6c64;border-radius:30px;padding:8px 12px;color:#63e3bc;font-size:11px;letter-spacing:2px}
    .capture-pointer{position:fixed;z-index:9999;width:14px;height:14px;border-radius:50%;background:#63e3bc;box-shadow:0 0 0 6px #63e3bc33;pointer-events:none;left:-50px;top:-50px;transition:box-shadow .16s}
    .graph-toolbar{padding:9px;margin-bottom:9px;min-height:43px}.graph-toolbar label,.graph-toolbar button{display:none}.graph-toolbar #graph-run,.graph-toolbar #graph-undo,.graph-toolbar #graph-redo,.graph-toolbar #graph-group{display:block}.graph-toolbar .primary{margin-top:0!important;font-size:12px;padding:8px 16px}.graph-toolbar button{padding:8px 11px;font-size:11px}
    .graph-workspace{grid-template-columns:minmax(0,1fr);gap:16px}.graph-center{min-width:0}.graph-workspace aside{display:none}.graph-topline{height:29px}.graph-center>.hint{display:none}
    .graph-viewport{height:280px;margin:8px 0 12px;overflow:hidden}#graph-board{transform:scale(.72);transform-origin:0 0}.graph-node strong{font-size:17px}.graph-node small{font-size:14px}.graph-node{padding:12px}
    #graph-status{font-size:12px;padding-bottom:4px}.graph-results{padding:12px 18px;margin:0}.graph-results h2{font-size:14px;margin:0 0 8px}.graph-results .hint{display:none}#graph-series label{font-size:11px;margin:0}#graph-plot svg{height:190px}#graph-series{gap:6px 16px}#graph-csv{display:none}#graph-values{margin:0;padding-top:6px;font-size:11px;line-height:1.5}
    .graph-results>label{font-size:10px;margin:3px 0}#graph-cursor{margin-top:2px}#graph-breadcrumb button{padding:5px 10px}
    body.capture-inspector .graph-workspace{grid-template-columns:minmax(0,1fr) 218px}body.capture-inspector .graph-workspace aside{display:block}body.capture-inspector .graph-workspace aside>.panel:not(:first-child){display:none}
    body.capture-inspector .graph-workspace aside>.panel{padding:13px}body.capture-inspector .graph-workspace aside h2{font-size:14px;margin-bottom:9px}body.capture-inspector #block-properties label:nth-child(2),body.capture-inspector #block-properties label:nth-child(3),body.capture-inspector #block-properties label:nth-child(4){display:none}body.capture-inspector #block-properties input,body.capture-inspector #block-properties textarea{padding:7px;font-size:12px}
    body.capture-pid #pid-main .workspace{grid-template-columns:1fr}body.capture-pid #pid-main aside,body.capture-pid #status,body.capture-pid .flow,body.capture-pid article>.panel:last-child{display:none}
    body.capture-pid .panel{padding:14px 20px;margin-bottom:10px}body.capture-pid .panel h2{font-size:15px;margin:0 0 8px}body.capture-pid .hint{display:none}body.capture-pid .charts h3{font-size:12px;margin:8px 0 0}body.capture-pid .charts h3:nth-of-type(3),body.capture-pid #integral-chart{display:none}body.capture-pid .charts svg{height:185px}body.capture-pid th,body.capture-pid td{padding:10px 12px}
    .capture-reveal{animation:trace-draw 3s ease-in-out both}@keyframes trace-draw{from{stroke-dashoffset:var(--trace-length)}to{stroke-dashoffset:0}}
  `;
  async function ready(page,editor=true) {
    await page.goto(URL);await expect(page.locator('#status')).toContainText('完了');
    if(editor){await page.getByRole('tab',{name:'ブロック図エディタ'}).click();await expect(page.locator('#graph-status')).toContainText('完了');}
    await page.addStyleTag({content:CSS});
    await page.evaluate(()=>{
      const banner=document.createElement('div');banner.className='capture-banner';banner.innerHTML='<div><div class="capture-brand">▦ BLOCKSIM / RUST × WASM</div><div class="capture-title"></div><div class="capture-sub"></div></div><div class="capture-chip"></div>';
      document.body.prepend(banner);const pointer=document.createElement('div');pointer.className='capture-pointer';document.body.append(pointer);
    });
  }
  async function title(page,text,sub,chapter){await page.evaluate(({text,sub,chapter})=>{document.querySelector('.capture-title').textContent=text;document.querySelector('.capture-sub').textContent=sub;document.querySelector('.capture-chip').textContent=chapter;},{text,sub,chapter});}
  async function pulse(page,id) {
    const box=await page.locator(`#${id}`).boundingBox();if(!box)return;
    await page.evaluate(({x,y})=>{const p=document.querySelector('.capture-pointer');p.style.left=x+'px';p.style.top=y+'px';p.style.boxShadow='0 0 0 12px #63e3bc33';},{x:box.x+box.width/2,y:box.y+box.height/2});
    await sleep(180);await page.evaluate(()=>document.querySelector('.capture-pointer').style.boxShadow='0 0 0 6px #63e3bc33');
  }
  async function click(page,id){await pulse(page,id);await page.locator(`#${id}`).evaluate(el=>el.click());await sleep(450);}
  async function sample(page,name){await page.evaluate(name=>{document.getElementById('graph-sample').value=name;document.getElementById('graph-load-sample').click();},name);await expect(page.locator('#graph-status')).toContainText('完了');await sleep(350);}
  async function run(page){await click(page,'graph-run');await expect(page.locator('#graph-status')).toContainText('完了');}
  async function reveal(page,selector='#graph-plot path'){await page.locator(selector).evaluateAll(paths=>paths.forEach(p=>{const n=p.getTotalLength();p.style.setProperty('--trace-length',n);p.style.strokeDasharray=String(n);p.classList.remove('capture-reveal');void p.getBoundingClientRect();p.classList.add('capture-reveal');}));await sleep(3100);}
  async function cursor(page,fraction){await page.evaluate(f=>{const el=document.getElementById('graph-cursor');el.value=Math.round(Number(el.max)*f);el.dispatchEvent(new Event('input',{bubbles:true}));},fraction);await sleep(180);}
  async function inspect(page,id){await page.locator(`[data-block="${id}"]`).evaluate(el=>el.dispatchEvent(new KeyboardEvent('keydown',{key:'Enter',bubbles:true})));await sleep(450);}
  async function gain(page,value){await page.locator('#block-properties [name="p.gain"]').fill(String(value));await page.locator('#block-properties').evaluate(form=>form.requestSubmit());await sleep(450);}
  async function connect(page,from,to){await page.evaluate(({from,to})=>{document.getElementById('edge-from').value=from;document.getElementById('edge-from').dispatchEvent(new Event('change'));document.getElementById('edge-to').value=to;document.getElementById('edge-to').dispatchEvent(new Event('change'));document.getElementById('graph-connect').requestSubmit();},{from,to});await expect(page.locator('#graph-status')).toContainText('接続を追加');await sleep(700);}
  async function record(name,setup,actions) {
    if(selected.length&&!selected.includes(name))return;
    const context=await browser.newContext({viewport:{width:1120,height:860},deviceScaleFactor:1});const page=await context.newPage();
    const errors=[];page.on('pageerror',error=>errors.push(String(error)));
    await setup(page);const frames=join(TMP,name);await mkdir(frames);let active=true,count=0;
    const capturing=(async()=>{while(active){const start=Date.now();await page.screenshot({path:join(frames,`frame-${String(count++).padStart(4,'0')}.png`)});await sleep(Math.max(0,100-(Date.now()-start)));}})();
    let failure;
    try {await sleep(600);await actions(page);await sleep(1100);} catch(error){failure=error;} finally {active=false;await capturing;}
    assert.deepEqual(errors,[]);if(failure)throw failure;
    await page.screenshot({path:join(TMP,`${name}-poster.png`)});await context.close();
    const output=join(MEDIA,`${name}.gif`);
    const args=['-hide_banner','-loglevel','error','-y','-threads','2','-framerate','10','-i',join(frames,'frame-%04d.png'),'-filter_complex','[0:v]scale=960:-1:flags=lanczos,split[a][b];[a]palettegen=max_colors=128:stats_mode=diff[p];[b][p]paletteuse=dither=bayer:bayer_scale=4:diff_mode=rectangle','-loop','0',output];
    await new Promise((resolve,reject)=>{const child=spawn('ffmpeg',args,{stdio:['ignore','ignore','pipe']});let stderr='';child.stderr.on('data',b=>stderr+=b);child.on('error',reject);child.on('exit',code=>code===0?resolve():reject(new Error(stderr)));});
    const size=(await stat(output)).size;assert.ok(size<4_000_000,`${name} GIF too large: ${size}`);
    manifest.push({file:`${name}.gif`,frames:count,duration_seconds:count/10,bytes:size,width:960,height:737});
    console.log(`${name}: ${count} frames, ${(size/1024).toFixed(0)} KiB`);await rm(frames,{recursive:true});
  }
  await record('hero',async p=>{await ready(p);await title(p,'MODEL. SIMULATE. UNDERSTAND.','ブロック図、混在時計、ベクトル。ひとつのワークスペースで。','FEATURE REEL');},async p=>{
    await reveal(p);await sample(p,'mixed');await title(p,'CONTINUOUS × DISCRETE','50 ms PID × 連続プラント × 100 ms 観測','HYBRID');await sleep(1200);
    await sample(p,'vector');await title(p,'VECTORS, UNDER CONTROL.','各成分を独立に制限。Scope で見える化。','VECTOR');await sleep(1400);
    await sample(p,'subsystem');await title(p,'SMALL BLOCKS. BIG IDEAS.','サブシステムでモデルを整理。','COMPOSE');await reveal(p);
  });
  await record('build',async p=>{await ready(p);await p.evaluate(()=>document.body.classList.add('capture-inspector'));await click(p,'graph-empty');await title(p,'BUILD YOUR OWN SIGNAL FLOW.','ブロックを追加 → 接続 → 実 WASM で計算','01 / BUILD');},async p=>{
    for(const kind of ['constant','gain','scope']){await p.evaluate(kind=>{document.getElementById('block-kind').value=kind;document.getElementById('block-add').click();},kind);await sleep(700);if(kind==='constant'){await p.locator('[name="p.value"]').fill('[2]');await p.locator('#block-properties').evaluate(form=>form.requestSubmit());}if(kind==='gain')await gain(p,3);}
    await connect(p,'constant-1','gain-1');await connect(p,'gain-1','scope-1');await run(p);await expect(p.locator('#graph-values')).toContainText('6.00000');await reveal(p);
  });
  await record('subsystems',async p=>{await ready(p);await title(p,'COMPOSE WITH SUBSYSTEMS.','接続を保ったまま階層化。内部モデルも編集できます。','02 / COMPOSE');},async p=>{
    for(const id of ['controller','plant']){await p.locator(`[data-block="${id}"] input`).evaluate(el=>el.click());await sleep(350);}
    await click(p,'graph-group');await run(p);await sleep(700);
    await p.locator('#graph-inspector button').last().evaluate(el=>el.click());await sleep(1300);
    await p.locator('#graph-breadcrumb button').first().evaluate(el=>el.click());await sleep(800);
    await click(p,'graph-undo');await run(p);await sleep(800);
  });
  await record('anti-windup',async p=>{await ready(p,false);await p.evaluate(()=>document.body.classList.add('capture-pid'));await title(p,'SATURATE. RECOVER.','同じモデルで比較：整定 26.85 秒 → 7.01 秒','03 / ANTI-WINDUP');},async p=>{
    await reveal(p,'#output-chart path, #control-chart path');await sleep(1000);
    await p.evaluate(()=>{document.querySelector('[name="anti_windup"]').checked=false;document.getElementById('model-form').requestSubmit();});await sleep(1300);
    await p.evaluate(()=>{document.querySelector('[name="anti_windup"]').checked=true;document.getElementById('model-form').requestSubmit();});await sleep(1600);
    assert.ok(Number(await p.locator('tr[data-aw="enabled"] [data-metric="iae"]').textContent())<1);
  });
  await record('vectors',async p=>{await ready(p);await sample(p,'vector');await title(p,'EVERY COMPONENT. EVERY LIMIT.','Gain → Saturation → Selector。ベクトル信号を追いかける。','04 / VECTORS');},async p=>{
    await reveal(p);for(const label of ['vector-output [0]','vector-output [1]','vector-output [2]']){await p.getByLabel('表示 '+label,{exact:true}).uncheck();await sleep(500);}await sleep(700);
    for(const label of ['vector-output [0]','vector-output [1]','vector-output [2]']){await p.getByLabel('表示 '+label,{exact:true}).check();await sleep(300);}for(const f of [0,.2,.4,.6,.8,1])await cursor(p,f);
  });
  await record('hybrid',async p=>{await ready(p);await sample(p,'mixed');await title(p,'TWO CLOCKS. ONE MODEL.','離散 PID 50 ms + 連続 RK4 + 観測 Delay 100 ms','05 / HYBRID');},async p=>{
    await reveal(p);for(const f of [0,.05,.125,.18,.25,.4,.6,.8,1])await cursor(p,f);await inspect(p,'controller');await sleep(700);await inspect(p,'observer');await sleep(800);
  });
  await record('history',async p=>{await ready(p);await sample(p,'subsystem');await p.evaluate(()=>document.body.classList.add('capture-inspector'));await inspect(p,'amplifier');await p.locator('#graph-inspector button').last().evaluate(el=>el.click());await inspect(p,'gain');await title(p,'EDIT WITHOUT REGRET.','ゲインを変更。Undo / Redo で結果も戻せる。','06 / HISTORY');},async p=>{
    await gain(p,5);await run(p);await sleep(1200);await click(p,'graph-undo');await run(p);await sleep(1200);await click(p,'graph-redo');await run(p);await sleep(1200);
    await p.locator('#graph-breadcrumb button').first().evaluate(el=>el.click());await click(p,'graph-save');await sleep(1000);
  });
  if(!selected.length)await writeFile(join(MEDIA,'manifest.json'),JSON.stringify({source:'Real BlockSim Chromium/WASM capture; presentation layout and path reveal only.',fps:10,clips:manifest},null,2)+'\n');
  console.log('Posters:',TMP);
} finally {
  await browser?.close();server.kill('SIGTERM');
  // Keep small final posters for inspection; frame directories are removed per clip.
}
