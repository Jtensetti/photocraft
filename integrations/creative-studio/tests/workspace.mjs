import {chromium} from 'playwright';
import {createServer} from 'vite';
import assert from 'node:assert/strict';
import {execFileSync} from 'node:child_process';
import {mkdirSync,writeFileSync} from 'node:fs';
mkdirSync('test-results',{recursive:true});
execFileSync('ffmpeg',['-v','error','-f','lavfi','-i','testsrc2=s=320x180:r=25:d=2','-f','lavfi','-i','sine=frequency=440:sample_rate=48000:duration=2','-c:v','libvpx-vp9','-c:a','libopus','-movflags','+faststart','-shortest','-y','test-results/workspace-input.mp4']);
const server=await createServer({server:{host:'127.0.0.1',port:5176,strictPort:true}});await server.listen();
const browser=await chromium.launch({headless:true,args:['--no-sandbox','--autoplay-policy=no-user-gesture-required']});
const page=await browser.newPage({viewport:{width:1600,height:1000},acceptDownloads:true});const errors=[];page.on('pageerror',e=>errors.push(e.message));
async function poll(predicate){const deadline=Date.now()+15000;while(Date.now()<deadline){if(await page.evaluate(predicate))return;await new Promise(resolve=>setTimeout(resolve,100));}throw new Error('Timed out waiting for persisted recovery');}
try{
  await page.goto('http://127.0.0.1:5176');await page.waitForFunction(()=>document.querySelector('#status').textContent.startsWith('Redo'));
  await page.locator('#media-input').setInputFiles('test-results/workspace-input.mp4');await page.waitForFunction(()=>document.querySelector('#status').textContent.startsWith('Importerad'));
  await page.locator('#timeline-frame').click();await page.waitForFunction(()=>document.querySelectorAll('.frame-cell img[src]').length>=5);
  const thumbs=await page.locator('.frame-cell img[src]').evaluateAll(nodes=>nodes.map(n=>n.src));assert.ok(new Set(thumbs).size>=3,'Thumbnails must represent different source-time frames');
  const cell=async n=>{await page.locator(`.frame-cell[data-frame="${n}"]`).waitFor({state:'visible',timeout:5000});const r=await page.locator(`.frame-cell[data-frame="${n}"]`).boundingBox();assert.ok(r);return {x:r.x+r.width/2,y:r.y+25};};
  let a=await cell(3);await page.mouse.click(a.x,a.y);await page.waitForFunction(()=>creativeStudio.inspect().local_frame===3);
  a=await cell(8);await page.keyboard.down('Shift');await page.mouse.click(a.x,a.y);await page.keyboard.up('Shift');
  await page.waitForFunction(()=>creativeStudio.inspect().project.workspace.selection?.end===9);
  assert.equal((await page.evaluate(()=>creativeStudio.inspect())).project.workspace.selection.start,3);
  const start=await cell(2),end=await cell(6);await page.mouse.move(start.x,start.y);await page.mouse.down();await page.mouse.move(end.x,end.y,{steps:6});await page.mouse.up();
  await page.waitForFunction(()=>creativeStudio.inspect().project.workspace.selection?.start===2);assert.equal((await page.evaluate(()=>creativeStudio.inspect())).project.workspace.selection.end,7);
  // A debounced adjustment keeps the scope captured at the start of the gesture.
  await page.locator('[data-mode=light]').click();
  await page.evaluate(async()=>{const input=document.querySelector('#exposure');input.value=1;input.dispatchEvent(new Event('input'));await creativeStudio.execute('view.set',{scope:'frame'});await creativeStudio.execute('seek',{frame:15});});
  await page.waitForFunction(()=>creativeStudio.inspect().project.adjustments.length===1);
  let s=await page.evaluate(()=>creativeStudio.inspect());assert.equal(s.project.adjustments[0].scope.start,2);assert.equal(s.project.adjustments[0].scope.end,7);
  const content=JSON.stringify([s.project.assets,s.project.clips,s.project.adjustments,s.project.layers]);
  await page.locator('[data-panel=photo] .dock-actions select').selectOption('right');await page.locator('#right-panel .dock-bar button').click();await page.locator('#right-panel .dock-tabs button').filter({hasText:'LightCraft'}).click();await page.locator('[data-panel=light] .dock-actions select').selectOption('left');
  assert.ok(await page.locator('#left-panel #light-panel').isVisible());assert.ok(await page.locator('#right-panel #photo-panel').isVisible());
  await page.locator('[data-panel=photo] .dock-actions select').selectOption('float');assert.ok(await page.locator('.floating #photo-panel').isVisible());
  await page.evaluate(()=>creativeStudio.execute('view.set',{timeline_height:280,timeline_visible:false}));await page.locator('#layout-save').click();await page.locator('#layout-name').fill('Color + paint');await page.locator('#layout-form button[value=save]').click();
  s=await page.evaluate(()=>creativeStudio.inspect());assert.equal(JSON.stringify([s.project.assets,s.project.clips,s.project.adjustments,s.project.layers]),content);
  await page.locator('#layout-reset').click();await page.locator('#layout-list').selectOption('Color + paint');await page.waitForFunction(()=>creativeStudio.inspect().project.workspace.timeline_visible===false);assert.equal((await page.evaluate(()=>creativeStudio.inspect())).project.workspace.timeline_height,280);await page.evaluate(()=>creativeStudio.execute('view.set',{timeline_visible:true}));assert.ok(await page.locator('.floating #photo-panel').isVisible());
  await page.locator('[data-panel=photo] .dock-actions select').selectOption('left');
  await page.locator('[data-mode=photo]').click();await page.locator('#scope').selectOption('frame');
  // Actual canvas pointer input, followed by an upstream-rendered mask.
  const canvas=await page.locator('#canvas').boundingBox();await page.locator('#color').fill('#ff0000');await page.locator('#brush-size').fill('110');
  await page.mouse.click(canvas.x+canvas.width*.5,canvas.y+canvas.height*.5);await page.waitForFunction(()=>creativeStudio.inspect().project.layers.length===1);
  await page.locator('[data-tool=mask]').click();await page.mouse.move(canvas.x+1,canvas.y+1);await page.mouse.down();await page.mouse.move(canvas.x+canvas.width*.45,canvas.y+canvas.height-1,{steps:8});await page.mouse.up();
  await page.waitForFunction(()=>creativeStudio.inspect().project.layers[0].mask!==null);
  await page.locator('[data-mode=film]').click();await page.locator('#timeline-detail').click();assert.ok(await page.locator('.layer-operation').isVisible());
  await page.locator('#volume').evaluate(el=>{el.value=50;el.dispatchEvent(new Event('change'));});await page.waitForFunction(()=>creativeStudio.inspect().active_clip.volume===.5);
  const selected=await page.evaluate(()=>creativeStudio.inspect().active_clip.id);
  await page.locator('#trim-start').fill('10');await page.locator('#trim-end').fill('40');await page.locator('#trim').click();await page.waitForFunction(()=>creativeStudio.inspect().total_frames===30);
  s=await page.evaluate(()=>creativeStudio.inspect());assert.equal(s.active_clip.source_in,10);assert.equal(s.project.layers[0].scope.start,5);assert.equal(s.project.layers[0].mask.length,4);
  await page.evaluate(async()=>{const s=creativeStudio.inspect();await creativeStudio.execute('selection.set',{clip_id:s.active_clip.id,start:5,end:20});});
  await page.locator('[data-mode=photo]').click();await page.locator('.text-controls summary').click();await page.locator('#text-content').fill('AI');await page.locator('#text-size').fill('36');await page.locator('#text-add').click();await page.waitForFunction(()=>creativeStudio.inspect().project.layers.length===2);
  s=await page.evaluate(()=>creativeStudio.inspect());assert.equal(s.project.layers[1].text.content,'AI');assert.equal(s.project.layers[1].scope.end,20);
  const textPixels=await page.evaluate(async()=>{await creativeStudio.execute('seek',{frame:10});const before=await creativeStudio.framePixels(10,320,180);const layer=creativeStudio.inspect().project.layers[1];await creativeStudio.execute('layer.set',{id:layer.id,visible:false});const after=await creativeStudio.framePixels(10,320,180);await creativeStudio.execute('layer.set',{id:layer.id,visible:true});return before.some((v,i)=>v!==after[i]);});assert.ok(textPixels);
  await page.locator('[data-mode=film]').click();
  const download=page.waitForEvent('download');await page.locator('#export').click();await page.locator('#export-video').click();await(await download).saveAs('test-results/workspace.webm');await page.waitForFunction(()=>!document.querySelector('#export-video').disabled);
  const pcm=execFileSync('ffmpeg',['-v','error','-i','test-results/workspace.webm','-vn','-ac','1','-ar','48000','-f','f32le','-'],{maxBuffer:1000000});let sum=0;for(let i=0;i<pcm.length;i+=4)sum+=pcm.readFloatLE(i)**2;const rms=Math.sqrt(sum/(pcm.length/4));assert.ok(rms>.05&&rms<.08,'Clip volume must reach encoded audio');
  const frames=JSON.parse(execFileSync('ffprobe',['-v','error','-count_frames','-select_streams','v:0','-show_entries','stream=nb_read_frames','-of','json','test-results/workspace.webm'],{encoding:'utf8'}));assert.equal(Number(frames.streams[0].nb_read_frames),30);
  await page.locator('#export-close').click();await page.screenshot({path:'test-results/workspace-v2.png'});
  await poll(async()=>{const {projects}=await import('/web/storage.js');const list=await projects();return list.some(p=>p.recovery&&JSON.parse(p.json).project.layers.length===2);});
  page.on('dialog',d=>d.accept());await page.reload();await page.waitForFunction(()=>document.querySelector('#status').textContent.startsWith('Redo'));await page.locator('#recover').click();await page.waitForFunction(()=>creativeStudio.inspect().project.layers.length===2);assert.equal((await page.evaluate(()=>creativeStudio.inspect())).active_clip.source_in,10);
  // A 121-second project bypasses the in-memory cap only through the seekable sink.
  const streamed=await page.evaluate(async()=>{
    const {exportTimeline}=await import('/web/export.js');await creativeStudio.execute('project.new',{width:32,height:24,fps:1});await creativeStudio.execute('asset.add',{asset:{id:'long',name:'Long blank','kind':'blank',width:32,height:24,bytes:0,source_fps:null},frames:121});
    const state=creativeStudio.inspect();let rejected=false;try{await exportTimeline({state,framePixels:creativeStudio.framePixels,cancelled:()=>false,progress:()=>{}});}catch{rejected=true;}
    const root=await navigator.storage.getDirectory(),file=await root.getFileHandle('test-long.webm',{create:true}),sink=await file.createWritable();let writes=0;const target={write:async chunk=>{writes++;await sink.write(chunk);}};
    const result=await exportTimeline({state,framePixels:creativeStudio.framePixels,cancelled:()=>false,progress:()=>{},sink:target});await sink.close();const bytes=new Uint8Array(await(await file.getFile()).arrayBuffer());await root.removeEntry('test-long.webm');return {rejected,nullResult:result===null,writes,bytes:[...bytes]};
  });
  assert.ok(streamed.rejected&&streamed.nullResult&&streamed.writes>0);writeFileSync('test-results/long.webm',Buffer.from(streamed.bytes));
  const long=JSON.parse(execFileSync('ffprobe',['-v','error','-count_frames','-show_streams','-show_format','-of','json','test-results/long.webm'],{encoding:'utf8'}));assert.equal(Number(long.streams[0].nb_read_frames),121);assert.equal(Number(long.format.duration),121);
  const cancelled=await page.evaluate(async()=>{const root=await navigator.storage.getDirectory(),file=await root.getFileHandle('test-cancel.webm',{create:true});const original=await file.createWritable();await original.write('original');await original.close();setTimeout(()=>document.querySelector('#export-cancel').click(),50);let rejected=false;try{await creativeStudio.exportVideo(file);}catch(error){rejected=error.message==='Exporten avbröts';}const contents=await(await file.getFile()).text();await root.removeEntry('test-cancel.webm');return rejected&&contents==='original';});assert.ok(cancelled,'Cancellation must abort the staging stream without overwriting the existing file');
  await page.setViewportSize({width:390,height:844});await page.evaluate(()=>creativeStudio.execute('view.set',{left_visible:false,right_visible:false}));await page.screenshot({path:'test-results/mobile-v2.png'});
  assert.ok(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth),'Mobile must not overflow horizontally');
  assert.deepEqual(errors,[]);console.log(JSON.stringify({result:'PASS',selection:'click / Shift / drag',thumbnail_frames:new Set(thumbs).size,docking:'move / float / save / restore',trimmed_frames:30,audio_rms:rms,long_export_frames:121,long_export_seconds:121,sink_writes:streamed.writes,cancellation_preserves_original:cancelled,recovery:true,browser_errors:errors},null,2));
}catch(error){console.log('DEBUG',await page.evaluate(()=>({status:document.querySelector('#status').textContent,frames:[...document.querySelectorAll('.frame-cell')].map(x=>x.dataset.frame),scroll:document.querySelector('#timeline-scroll').scrollLeft,strip:creativeStudio.filmstrip.inspect(),state:creativeStudio.inspect().project.workspace})));await page.screenshot({path:'test-results/workspace-error.png'});throw error;}finally{await browser.close();await server.close();}
