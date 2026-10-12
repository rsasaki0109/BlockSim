// Reproducible README screencasts from the real app. Only capture layout,
// captions, pointer and line-reveal styling are changed; all models/results
// come from app handlers and the shipped WASM. No application source edits.
import assert from 'node:assert/strict';
import { chromium, expect } from '@playwright/test';
import { spawn } from 'node:child_process';
import { mkdir, mkdtemp, readFile, rm, stat, writeFile } from 'node:fs/promises';
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
    .graph-toolbar{padding:9px;margin-bottom:9px;min-height:43px}.graph-toolbar label,.graph-toolbar button{display:none}.graph-toolbar #graph-run,.graph-toolbar #graph-undo,.graph-toolbar #graph-redo,.graph-toolbar #graph-group,.graph-toolbar #graph-ungroup{display:block}.graph-toolbar .primary{margin-top:0!important;font-size:12px;padding:8px 16px}.graph-toolbar button{padding:8px 11px;font-size:11px}
    .graph-workspace{grid-template-columns:minmax(0,1fr);gap:16px}.graph-center{min-width:0}.graph-workspace aside{display:none}.graph-topline{height:29px}.graph-center>.hint,.graph-view-tools,#graph-search,#graph-diagnostics{display:none}
    .graph-viewport{height:280px;margin:8px 0 12px;overflow:hidden}#graph-board{transform-origin:0 0}.graph-node strong{font-size:17px}.graph-node small{font-size:14px}.graph-node{padding:12px}
    #graph-status{font-size:12px;padding-bottom:4px}.graph-results{padding:12px 18px;margin:0}.graph-results h2{font-size:14px;margin:0 0 8px}.graph-results .hint{display:none}#graph-series label{font-size:11px;margin:0}#graph-plot svg{height:190px}#graph-series{gap:6px 16px}#graph-csv{display:none}#graph-values{margin:0;padding-top:6px;font-size:11px;line-height:1.5}
    .graph-results>label{font-size:10px;margin:3px 0}#graph-cursor{margin-top:2px}#graph-breadcrumb button{padding:5px 10px}
    body.capture-inspector .graph-workspace{grid-template-columns:minmax(0,1fr) 218px}body.capture-inspector .graph-workspace aside{display:block}body.capture-inspector .graph-workspace aside>.panel:not(:first-child){display:none}
    body.capture-inspector .graph-workspace aside>.panel{padding:13px}body.capture-inspector .graph-workspace aside h2{font-size:14px;margin-bottom:9px}body.capture-inspector #block-properties label:nth-child(2),body.capture-inspector #block-properties label:nth-child(3),body.capture-inspector #block-properties label:nth-child(4){display:none}body.capture-inspector #block-properties input,body.capture-inspector #block-properties textarea{padding:7px;font-size:12px}
    body.capture-pid #pid-main .workspace{grid-template-columns:1fr}body.capture-pid #pid-main aside,body.capture-pid #status,body.capture-pid .flow,body.capture-pid article>.panel:last-child{display:none}
    body.capture-pid .panel{padding:14px 20px;margin-bottom:10px}body.capture-pid .panel h2{font-size:15px;margin:0 0 8px}body.capture-pid .hint{display:none}body.capture-pid .charts h3{font-size:12px;margin:8px 0 0}body.capture-pid .charts h3:nth-of-type(3),body.capture-pid #integral-chart{display:none}body.capture-pid .charts svg{height:185px}body.capture-pid th,body.capture-pid td{padding:10px 12px}
    body.capture-workspace .graph-toolbar #graph-select-all,body.capture-workspace .graph-toolbar #graph-copy,body.capture-workspace .graph-toolbar #graph-paste{display:block}body.capture-workspace .graph-view-tools{display:flex}body.capture-workspace #block-properties label:nth-child(4){display:block!important}body.capture-workspace .graph-viewport{height:230px}body.capture-workspace #graph-plot svg{height:165px}
    body.capture-search #graph-search{display:block;padding:8px 12px;margin:4px 0 8px}body.capture-search #graph-search summary{font-size:12px}body.capture-search #graph-search-form{margin-top:8px;gap:8px}body.capture-search #graph-search input,body.capture-search #graph-search select{padding:6px;font-size:12px;margin-top:4px}body.capture-search #graph-search-count{margin:6px 0}body.capture-search #graph-search-results{max-height:94px;margin-bottom:0}body.capture-search #graph-search-results button{padding:5px 8px;gap:2px}body.capture-search #graph-search>.hint{display:none}body.capture-search .graph-viewport{height:145px}body.capture-search #graph-plot svg{height:115px}
    body.capture-diagnostics #graph-diagnostics{display:block;padding:8px 12px;margin:4px 0 8px}body.capture-diagnostics #graph-check-run{display:block}body.capture-diagnostics #graph-diagnostics summary{font-size:13px}body.capture-diagnostics #graph-diagnostics-state{font-size:12px;margin:7px 0}body.capture-diagnostics #graph-diagnostics-list{max-height:120px;margin:0}body.capture-diagnostics #graph-diagnostics-list li{padding:8px 12px}body.capture-diagnostics #graph-diagnostics-list p{font-size:11px;margin:5px 0}body.capture-diagnostics #graph-diagnostics-list button{padding:5px 10px;font-size:11px}body.capture-diagnostics .diagnostic-pages{display:none}body.capture-diagnostics .graph-viewport{height:145px}body.capture-diagnostics #graph-plot svg{height:115px}
    .graph-reference-tools,#graph-reference-state,#graph-reference-values,#graph-comparison{display:none}body.capture-comparison .graph-reference-tools{display:flex;gap:6px;margin-bottom:8px}body.capture-comparison .graph-reference-tools button{padding:6px 8px;font-size:10px}body.capture-comparison .graph-reference-tools label{font-size:10px}body.capture-comparison #graph-reference-export{display:none}body.capture-comparison #graph-reference-state{display:block;font-size:10px;margin:6px 0}body.capture-comparison #graph-reference-values{display:block;font-size:10px;margin:4px 0;padding:0}body.capture-comparison #graph-comparison{display:block}body.capture-comparison #graph-comparison summary{margin:8px 0;font-size:12px}body.capture-comparison #graph-comparison-notices:empty{display:none}body.capture-comparison #graph-comparison table{font-size:10px}body.capture-comparison #graph-comparison th,body.capture-comparison #graph-comparison td{padding:6px}body.capture-comparison .comparison-pages{display:none}body.capture-comparison .graph-viewport{height:125px}body.capture-comparison #graph-plot svg{height:155px}
    #graph-run-files{display:none}body.capture-archive #graph-run-files{display:block;margin:4px 0}body.capture-archive #graph-run-files summary{font-size:12px;margin:5px 0}body.capture-archive #graph-run-files .buttons{display:flex;gap:6px;flex-wrap:wrap}body.capture-archive #graph-run-files button,body.capture-archive #graph-run-files .file-button{font-size:10px;padding:6px 8px}body.capture-archive #graph-reference-import-state{font-size:10px;margin:6px 0}body.capture-archive .graph-viewport{height:115px}body.capture-archive #graph-plot svg{height:100px}
    #graph-sweep-host,#graph-frequency-host{display:none}
    body.capture-adaptive .graph-workspace{grid-template-columns:minmax(0,1fr) 250px}body.capture-adaptive .graph-workspace aside{display:block}body.capture-adaptive .graph-workspace aside>.panel{display:none}body.capture-adaptive .graph-workspace aside>.panel:nth-child(3){display:block;padding:12px}body.capture-adaptive aside h2{font-size:14px}body.capture-adaptive aside label{font-size:11px;margin:8px 0}body.capture-adaptive aside input,body.capture-adaptive aside select{padding:6px;font-size:12px}body.capture-adaptive aside .hint{display:none}body.capture-adaptive #graph-statistics{display:block;color:#63e3bc;font-size:13px}body.capture-adaptive .graph-viewport{height:220px}
    body.capture-sweep #graph-sweep-host{display:block}body.capture-sweep .graph-results{display:none}body.capture-sweep .graph-viewport{height:150px}body.capture-sweep #graph-sweep{margin-top:5px;padding:10px 14px}body.capture-sweep #graph-sweep .hint{display:none}body.capture-sweep #graph-sweep-fields{grid-template-columns:repeat(4,minmax(0,1fr));gap:6px;padding:8px}body.capture-sweep #graph-sweep-fields legend{display:none}body.capture-sweep #graph-sweep label{font-size:10px;margin:0}body.capture-sweep #graph-sweep input,body.capture-sweep #graph-sweep select,body.capture-sweep #graph-sweep textarea{padding:5px;font-size:11px;margin-top:3px}body.capture-sweep #graph-sweep textarea{height:30px;min-height:30px}body.capture-sweep #graph-sweep button,body.capture-sweep #graph-sweep .file-button{font-size:10px;padding:5px 8px}body.capture-sweep #graph-sweep table{font-size:10px}body.capture-sweep #graph-sweep th,body.capture-sweep #graph-sweep td{padding:5px}body.capture-sweep #graph-sweep-state{font-size:12px;color:#63e3bc;margin:7px 0}
    body.capture-frequency #graph-frequency-host{display:block}body.capture-frequency .graph-results{display:none}body.capture-frequency .graph-viewport{height:100px}body.capture-frequency #graph-frequency{margin-top:5px;padding:10px 14px}body.capture-frequency #graph-frequency .hint{display:none}body.capture-frequency #graph-frequency-fields{grid-template-columns:repeat(5,minmax(0,1fr));gap:7px;padding:8px}body.capture-frequency #graph-frequency-fields legend,body.capture-frequency #graph-frequency-fields label:nth-of-type(2),body.capture-frequency #graph-frequency-fields label:nth-of-type(4){display:none}body.capture-frequency #graph-frequency label{font-size:10px;margin:0}body.capture-frequency #graph-frequency input,body.capture-frequency #graph-frequency select{padding:5px;font-size:11px;margin-top:3px}body.capture-frequency #graph-frequency button{font-size:10px;padding:6px 8px}body.capture-frequency #graph-frequency .buttons{display:flex;justify-content:flex-end}body.capture-frequency #graph-frequency .buttons>*:not(#graph-frequency-csv){display:none}body.capture-frequency .frequency-charts h3{font-size:12px;margin:5px 0}body.capture-frequency .frequency-charts svg{height:130px}body.capture-frequency #graph-frequency-state{font-size:12px;color:#63e3bc;margin:7px 0}body.capture-frequency #graph-frequency-values{font-size:12px;margin-top:5px}body.capture-frequency #graph-frequency-table,body.capture-frequency .frequency-open-loop{display:none}
    .capture-reveal{animation:trace-draw 3s ease-in-out both}@keyframes trace-draw{from{stroke-dashoffset:var(--trace-length)}to{stroke-dashoffset:0}}
      body.capture-margins .graph-toolbar,body.capture-margins .graph-topline,body.capture-margins #graph-status{display:none}body.capture-margins .graph-viewport{height:140px}body.capture-margins #graph-frequency-fields,body.capture-margins #graph-frequency-form,body.capture-margins #graph-frequency-caption,body.capture-margins label:has(#graph-frequency-cursor),body.capture-margins #graph-frequency-values{display:none}body.capture-margins #graph-frequency .buttons>*:not(#graph-frequency-margins-csv){display:none}body.capture-margins #graph-frequency .buttons #graph-frequency-margins-csv{display:block}body.capture-margins .frequency-charts svg{height:180px}body.capture-margins #graph-frequency-margins{display:grid;grid-template-columns:1fr 1fr;gap:2px 16px}body.capture-margins #graph-frequency-margins h3,body.capture-margins #graph-frequency-margins-summary{grid-column:1/-1;font-size:12px;margin:6px 0}body.capture-margins #graph-frequency-gain-status,body.capture-margins #graph-frequency-phase-status{display:none}body.capture-margins #graph-frequency-margins table{min-width:0;width:100%;font-size:12px}body.capture-margins #graph-frequency-margins th,body.capture-margins #graph-frequency-margins td{padding:8px 5px}
`;
  async function ready(page,editor=true) {
    page.on('dialog',dialog=>dialog.accept());
    await page.goto(URL);await expect(page.locator('#status')).toContainText('完了');
    if(editor){await page.getByRole('tab',{name:'ブロック図エディタ'}).click();await expect(page.locator('#graph-status')).toContainText('完了');}
    await page.addStyleTag({content:CSS});
    if(editor)await page.evaluate(()=>{for(let i=0;i<3;i++)document.getElementById('graph-zoom-out').click();});
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
  async function connect(page,from,to){
    await pulse(page,`port-out-${from}-0`);await page.getByRole('button',{name:`出力 ${from}:0`,exact:true}).click();await sleep(400);
    await pulse(page,`port-in-${to}-0`);await page.getByRole('button',{name:`入力 ${to}:0`,exact:true}).click();
    await expect(page.locator('#graph-status')).toContainText('ポートを接続');await sleep(700);
  }
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
    await inspect(p,'subsystem-1');await click(p,'graph-ungroup');await run(p);await sleep(800);
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
  await record('workspace',async p=>{await ready(p);await sample(p,'subsystem');await p.evaluate(()=>document.body.classList.add('capture-workspace'));await title(p,'COPY. ZOOM. KEEP YOUR IDEAS.','内部設定と接続をコピー。全体表示で見渡し、変更を保存。','07 / WORKSPACE');},async p=>{
    await click(p,'graph-select-all');await click(p,'graph-copy');await click(p,'graph-paste');
    await expect(p.locator('#graph-count')).toContainText('6 blocks · 4 wires');
    await p.evaluate(()=>document.body.classList.add('capture-inspector'));
    for(const id of ['copy-wave','copy-amplifier','copy-wave-output']){
      await inspect(p,id);await p.locator('#block-properties [name="y"]').fill('300');await p.locator('#block-properties').evaluate(form=>form.requestSubmit());
    }
    await p.evaluate(()=>document.body.classList.remove('capture-inspector'));
    await click(p,'graph-fit');await run(p);await expect(p.locator('#graph-status')).toContainText('2 Scope');
    const match=await p.evaluate(async()=>{const path='/src/wasm/blocksim.js',m=await import(path);await m.default();document.getElementById('graph-save').click();const graph=JSON.parse(localStorage.getItem('blocksim.graph.v2')).graph,result=JSON.parse(m.run_graph(JSON.stringify(graph)));return JSON.stringify(result.traces[0].values)===JSON.stringify(result.traces[1].values);});
    assert.equal(match,true);await expect(p.locator('#graph-dirty')).toHaveText('変更なし');await reveal(p);await cursor(p,.25);
  });
  await record('layout',async p=>{await ready(p);await sample(p,'subsystem');await p.evaluate(()=>document.body.classList.add('capture-workspace'));await title(p,'FROM CLUTTER TO CLARITY.','接続と内部設定を保ったまま、自動配置。Undo で戻せます。','08 / ARRANGE');},async p=>{
    await click(p,'graph-select-all');await click(p,'graph-copy');await click(p,'graph-paste');await sleep(700);
    const before=await p.evaluate(()=>{document.getElementById('graph-save').click();return JSON.parse(localStorage.getItem('blocksim.graph.v2')).graph;});
    await click(p,'graph-layout');await expect(p.locator('#graph-status')).toContainText('自動配置しました');await run(p);await expect(p.locator('#graph-status')).toContainText('2 Scope');
    const equivalent=await p.evaluate(async before=>{document.getElementById('graph-save').click();const after=JSON.parse(localStorage.getItem('blocksim.graph.v2')).graph,path='/src/wasm/blocksim.js',m=await import(path);await m.default();return m.run_graph(JSON.stringify(before))===m.run_graph(JSON.stringify(after));},before);assert.equal(equivalent,true);
    await sleep(700);await click(p,'graph-undo');await sleep(700);await click(p,'graph-redo');await run(p);await click(p,'graph-save');await expect(p.locator('#graph-dirty')).toHaveText('変更なし');await reveal(p);
  });
  await record('search',async p=>{await ready(p);await sample(p,'subsystem');await p.evaluate(()=>{document.getElementById('graph-select-all').click();document.getElementById('graph-duplicate').click();document.getElementById('graph-save').click();document.getElementById('graph-run').click();document.body.classList.add('capture-search','capture-inspector');});await expect(p.locator('#graph-status')).toContainText('完了');await title(p,'FIND THE BLOCK. FOLLOW THE SIGNAL.','名前・ID・種類で検索。内部ブロックへ移動して編集。','09 / FIND');},async p=>{
    await p.locator('#graph-run').focus();await p.keyboard.press('Control+k');await p.getByLabel('ブロック検索',{exact:true}).fill('ＧＡＩＮ');await expect(p.locator('#graph-search-count')).toHaveText('2 件');await sleep(1300);
    await p.getByRole('button',{name:'表示 gain (copy-amplifier/gain)',exact:true}).click();await expect(p.locator('#graph-breadcrumb')).toContainText('copy-amplifier');await expect(p.getByLabel('ゲイン',{exact:true})).toHaveValue('3');await sleep(900);
    await gain(p,5);await run(p);await cursor(p,.25);await expect(p.locator('#graph-values')).toContainText('wave-output (wave-output) [0]=3.00000');await expect(p.locator('#graph-values')).toContainText('wave-output (copy-wave-output) [0]=5.00000');await reveal(p);
    await click(p,'graph-save');await p.getByLabel('ブロック検索',{exact:true}).fill('copy-amplifier gain');await expect(p.locator('#graph-search-count')).toHaveText('1 件');await expect(p.locator('#graph-dirty')).toHaveText('変更なし');
  });
  await record('diagnostics',async p=>{
    await ready(p);await p.evaluate(()=>document.body.classList.add('capture-inspector'));await click(p,'graph-empty');
    for(const kind of ['constant','gain','scope']){
      await p.evaluate(kind=>{document.getElementById('block-kind').value=kind;document.getElementById('block-add').click();},kind);
      if(kind==='constant'){await p.locator('[name="p.value"]').fill('[2]');await p.locator('#block-properties').evaluate(form=>form.requestSubmit());}
      if(kind==='gain')await gain(p,3);
    }
    await p.evaluate(()=>document.body.classList.remove('capture-inspector'));await connect(p,'constant-1','gain-1');await click(p,'graph-save');
    await p.evaluate(()=>{document.body.classList.add('capture-diagnostics');document.getElementById('graph-fit').click();});
    await title(p,'FIND THE PROBLEM. FIX THE FLOW.','モデル診断から入力ポートへ。配線を直して、再実行。','10 / DIAGNOSE');
  },async p=>{
    await click(p,'graph-check-run');await expect(p.locator('#graph-diagnostics-summary')).toContainText('1 件の問題');await sleep(1200);
    await p.screenshot({path:join(TMP,'diagnostics-issue.png')});
    await p.getByRole('button',{name:'表示 scope-1:0',exact:true}).click();await expect(p.locator('#port-in-scope-1-0')).toBeFocused();await sleep(700);
    await connect(p,'gain-1','scope-1');await expect(p.locator('#graph-diagnostics-summary')).toContainText('問題なし');await run(p);await expect(p.locator('#graph-values')).toContainText('6.00000');
    await title(p,'CONNECT. FIX. CONTINUE.','問題なし → 実 WASM で出力6 → モデル保存','10 / DIAGNOSE');await reveal(p);await click(p,'graph-save');await expect(p.locator('#graph-dirty')).toHaveText('変更なし');
  });
  await record('comparison',async p=>{
    await ready(p);await sample(p,'subsystem');await p.evaluate(()=>document.body.classList.add('capture-comparison','capture-inspector'));
    await inspect(p,'amplifier');await p.locator('#graph-inspector button').last().evaluate(el=>el.click());await inspect(p,'gain');
    await p.evaluate(()=>document.getElementById('graph-fit').click());
    await title(p,'KEEP A BASELINE. SEE THE CHANGE.','実行時の設定と波形を保持。ゲイン変更前後を比べる。','11 / COMPARE');
  },async p=>{
    await click(p,'graph-reference-set');await expect(p.locator('#graph-comparison-rows [data-metric="maxAbs"]')).toHaveText('0.00000');await sleep(900);
    await gain(p,5);await run(p);await cursor(p,.25);await expect(p.locator('#graph-values')).toContainText('5.00000');await expect(p.locator('#graph-reference-values')).toContainText('基準=3.00000 · Δ=2.00000');await expect(p.locator('#graph-comparison-rows [data-metric="maxAbs"]')).toHaveText('2.00000');
    await title(p,'BEFORE. AFTER. THE DIFFERENCE.','ゲイン3 → 5。t=0.5秒の基準3・現行5・差2。','11 / COMPARE');await reveal(p,'#graph-plot path[data-snapshot="current"]');
    await p.locator('#graph-reference-show').uncheck();await sleep(700);await p.locator('#graph-reference-show').check();await sleep(800);await expect(p.locator('#graph-plot path[data-snapshot="reference"]')).toHaveAttribute('stroke-dasharray','8 5');await click(p,'graph-save');await expect(p.locator('#graph-dirty')).toHaveText('変更なし');
  });
  await record('archive',async p=>{
    await ready(p);await sample(p,'subsystem');await p.evaluate(()=>document.body.classList.add('capture-comparison','capture-archive','capture-inspector'));
    await inspect(p,'amplifier');await p.locator('#graph-inspector button').last().evaluate(el=>el.click());await inspect(p,'gain');
    await p.evaluate(()=>{document.getElementById('graph-fit').click();document.getElementById('graph-run-files').open=true;});
    await title(p,'SAVE THE RUN. REUSE THE RESULT.','実行時の設定と全記録点を保存。再計算で確認して読み込む。','12 / ARCHIVE');
  },async p=>{
    const pending=p.waitForEvent('download');await click(p,'graph-run-export');const file=await pending,text=await readFile(await file.path(),'utf8'),archive=JSON.parse(text);
    assert.equal(archive.format,'blocksim.run');assert.equal(archive.result.times.length,201);assert.equal(archive.model.blocks[1].params.model.blocks[1].params.gain,3);await sleep(800);
    await gain(p,5);await run(p);await cursor(p,.25);await expect(p.locator('#graph-values')).toContainText('5.00000');await sleep(600);
    await p.locator('#graph-reference-import').setInputFiles({name:'saved-run.json',mimeType:'application/json',buffer:Buffer.from(text)});await expect(p.locator('#graph-reference-import-state')).toContainText('一致を確認し、基準を読み込みました');await expect(p.locator('#graph-reference-values')).toContainText('基準=3.00000 · Δ=2.00000');await expect(p.locator('#graph-comparison-rows [data-metric="maxAbs"]')).toHaveText('2.00000');
    await title(p,'READY TO COMPARE. AGAIN.','保存した結果3と現行5を比較。全201点の一致を確認済み。','12 / ARCHIVE');await reveal(p,'#graph-plot path[data-snapshot="current"]');await click(p,'graph-save');await expect(p.locator('#graph-dirty')).toHaveText('変更なし');
  });
  await record('adaptive',async p=>{
    await ready(p);await sample(p,'firstOrder');await p.evaluate(()=>document.body.classList.add('capture-adaptive'));
    await p.locator('#graph-solver').selectOption('rk4');await p.locator('#graph-clock').evaluate(form=>form.requestSubmit());await run(p);
    await expect(p.locator('#graph-statistics')).toContainText('微分評価 4000 回');
    await title(p,'ADAPT THE STEP. KEEP THE SAMPLES.','同じ1001記録点で、必要な内部刻みを選ぶ。','13 / RK45');
  },async p=>{
    await sleep(900);await p.locator('#graph-solver').selectOption('rk45');
    for(const [id,value]of [['graph-atol','1e-9'],['graph-rtol','1e-9'],['graph-min-step','1e-12'],['graph-max-step','0.8']])await p.locator('#'+id).fill(value);
    await sleep(700);await p.locator('#graph-clock').evaluate(form=>form.requestSubmit());await run(p);
    await expect(p.locator('#graph-statistics')).toContainText('微分評価 609 回');await expect(p.locator('#graph-status')).toContainText('1001 点');
    await title(p,'4000 → 609 RATE EVALUATIONS.','一次遅れ解析解との最大誤差 3.87e−10。微分評価84.8%減。','13 / RK45');await reveal(p);await click(p,'graph-save');
  });
  await record('sweep',async p=>{
    await ready(p);await p.evaluate(()=>{document.body.classList.add('capture-sweep');document.getElementById('graph-sweep').open=true;});
    const text=await readFile(join(ROOT,'samples/sweeps/pid-anti-windup.json'),'utf8');
    await p.locator('#graph-sweep-import').setInputFiles({name:'pid-sweep.json',mimeType:'application/json',buffer:Buffer.from(text)});await expect(p.locator('#graph-sweep-state')).toContainText('設定を読み込みました');await p.evaluate(()=>document.getElementById('graph-fit').click());
    await title(p,'SWEEP. MEASURE. CHOOSE.','anti-windup 0/1 を独立に計算。5〜40秒の復帰を評価。','14 / SWEEP');
  },async p=>{
    await click(p,'graph-sweep-run');await expect(p.locator('#graph-sweep-state')).toContainText('2 成功 / 0 エラー');
    const values=(await p.locator('#graph-sweep-rows [data-metric="iae"]').allTextContents()).map(Number);assert.ok(values[1]<values[0]*.1);
    await title(p,'MEASURE THE RECOVERY.','復帰 IAE '+values[0].toFixed(2)+' → '+values[1].toFixed(2)+'。各ケースを基準やモデルに使える。','14 / SWEEP');await sleep(1600);
    await p.locator('#graph-sweep-rows tr').first().getByRole('button',{name:'基準にする'}).click();await sleep(900);await p.locator('#graph-sweep-rows tr').last().getByRole('button',{name:'設定を適用'}).click();await expect(p.locator('#graph-status')).toContainText('4001 点');
    const pending=p.waitForEvent('download');await click(p,'graph-sweep-csv');const file=await pending;assert.equal((await readFile(await file.path(),'utf8')).split('\r\n').length,3);await click(p,'graph-save');await sleep(1000);
  });
  await record('frequency',async p=>{
    await ready(p);await sample(p,'lowpass');await p.evaluate(()=>{document.body.classList.add('capture-frequency');document.getElementById('graph-frequency').open=true;document.getElementById('graph-fit').click();});
    await title(p,'FROM STEP RESPONSE TO BODE.','同じ2次ローパスで、角周波数ごとの振幅と位相を確かめる。','15 / FREQUENCY');
  },async p=>{
    await click(p,'graph-frequency-run');await expect(p.locator('#graph-frequency-state')).toContainText('201 有効点 / 0 エラー点');await expect(p.locator('#graph-frequency-values')).toContainText('-6.02060 dB');await expect(p.locator('#graph-frequency-values')).toContainText('-90.0000°');
    await reveal(p,'#graph-frequency-magnitude path, #graph-frequency-phase path');
    for(const index of [0,50,100,150,200,100]){await p.locator('#graph-frequency-cursor').fill(String(index));await sleep(450);}
    await title(p,'ONE MODEL. TWO VIEWS.','ω=1 rad/s：−6.02 dB、−90°。全201点を CSV に保存。','15 / FREQUENCY');
    const pending=p.waitForEvent('download');await click(p,'graph-frequency-csv');const file=await pending,text=await readFile(await file.path(),'utf8');assert.equal(text.split('\r\n').length,202);await sleep(900);
  });
  await record('margins',async p=>{
    await ready(p);await p.evaluate(()=>{document.body.classList.add('capture-frequency','capture-margins');document.getElementById('graph-frequency').open=true;});
    const text=await readFile(join(ROOT,'samples/frequency/open-loop-plan.json'),'utf8');await p.locator('#graph-frequency-import').setInputFiles({name:'open-loop.json',mimeType:'application/json',buffer:Buffer.from(text)});await expect(p.locator('#graph-frequency-state')).toContainText('設定を読み込みました');await p.evaluate(()=>document.getElementById('graph-fit').click());
    await title(p,'FIND THE CROSSINGS.','開ループ L(s)=4 / s(s+1)(s+2)。交差を絞り込んで余裕を確認。','16 / MARGINS');
  },async p=>{
    await p.evaluate(()=>document.getElementById('graph-frequency-run').click());await expect(p.locator('#graph-frequency-state')).toContainText('201 有効点 / 0 エラー点');await expect(p.locator('#graph-frequency-gain-rows [data-metric="phase_margin_deg"]')).toHaveText('11.42498');await expect(p.locator('#graph-frequency-phase-rows [data-metric="gain_margin_db"]')).toHaveText('3.521825');await reveal(p,'#graph-frequency-magnitude path, #graph-frequency-phase path');await sleep(1500);
    const plan=JSON.parse(await readFile(join(ROOT,'samples/frequency/open-loop-plan.json'),'utf8'));plan.model.blocks[1].params.numerator=[8];plan.model.blocks[1].label='L(s) = 8 / s(s+1)(s+2)';await p.locator('#graph-frequency-import').setInputFiles({name:'gain-eight.json',mimeType:'application/json',buffer:Buffer.from(JSON.stringify(plan))});await expect(p.locator('#graph-frequency-state')).toContainText('設定を読み込みました');await p.evaluate(()=>document.getElementById('graph-fit').click());await title(p,'READ THE SIGN. CHECK THE RANGE.','設定を復元してゲイン4 → 8。PM −7.52°、GM −2.50 dB。','16 / MARGINS');await p.evaluate(()=>document.getElementById('graph-frequency-run').click());await expect(p.locator('#graph-frequency-gain-rows [data-metric="phase_margin_deg"]')).toHaveText('-7.517996');await expect(p.locator('#graph-frequency-phase-rows [data-metric="gain_margin_db"]')).toHaveText('-2.498775');await reveal(p,'#graph-frequency-magnitude path, #graph-frequency-phase path');await sleep(1700);
    const pending=p.waitForEvent('download');await click(p,'graph-frequency-margins-csv');const file=await pending,text=await readFile(await file.path(),'utf8');assert.equal(text.split('\r\n').length,3);assert.ok(text.includes('no stability inference'));await sleep(900);
  });
  if(selected.length){const previous=JSON.parse(await readFile(join(MEDIA,'manifest.json'),'utf8'));const updated=new Map(previous.clips.map(clip=>[clip.file,clip]));for(const clip of manifest)updated.set(clip.file,clip);manifest.splice(0,manifest.length,...updated.values());}
  await writeFile(join(MEDIA,'manifest.json'),JSON.stringify({source:'Real BlockSim Chromium/WASM capture; presentation layout and path reveal only.',fps:10,clips:manifest},null,2)+'\n');
  console.log('Posters:',TMP);
} finally {
  await browser?.close();server.kill('SIGTERM');
  // Keep small final posters for inspection; frame directories are removed per clip.
}
