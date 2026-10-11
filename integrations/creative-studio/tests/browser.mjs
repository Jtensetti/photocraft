import {chromium} from 'playwright';
import {createServer} from 'vite';
import assert from 'node:assert/strict';
import {mkdirSync} from 'node:fs';
import {execFileSync} from 'node:child_process';

mkdirSync('test-results',{recursive:true});
execFileSync('ffmpeg',['-hide_banner','-loglevel','error','-f','lavfi','-i','color=c=0x405060:s=320x180:r=25:d=2','-f','lavfi','-i','sine=frequency=440:sample_rate=48000:duration=2','-c:v','libvpx-vp9','-pix_fmt','yuv420p','-c:a','libopus','-movflags','+faststart','-shortest','-y','test-results/input.mp4']);
const server=await createServer({server:{host:'127.0.0.1',port:5174,strictPort:true}});await server.listen();
const browser=await chromium.launch({headless:true,args:['--no-sandbox','--autoplay-policy=no-user-gesture-required']});
const page=await browser.newPage({viewport:{width:1440,height:950},acceptDownloads:true});
const errors=[];page.on('pageerror',e=>errors.push(e.message));
try {
  await page.goto('http://127.0.0.1:5174');
  await page.waitForFunction(()=>document.querySelector('#status').textContent.startsWith('Redo'));
  await page.locator('#media-input').setInputFiles('test-results/input.mp4');
  await page.waitForFunction(()=>document.querySelector('#status').textContent.startsWith('Importerad'));
  let st=await page.evaluate(()=>window.creativeStudio.inspect());
  assert.equal(st.total_frames,50);assert.equal(st.project.fps.num,25);assert.equal(st.project.fps.den,1);
  await page.locator('#play').click();await page.waitForFunction(()=>creativeStudio.inspect().project.workspace.playhead>=8);
  await page.locator('#play').click();
  const clip=st.active_clip.id;
  await page.evaluate(async clip=>{
    await creativeStudio.execute('seek',{frame:10});
    await creativeStudio.execute('selection.set',{clip_id:clip,start:10,end:20});
    await creativeStudio.execute('develop.set',{values:{exposure:1,temperature:10}});
    await creativeStudio.execute('view.set',{mode:'photo',scope:'frame'});
    await creativeStudio.execute('layer.stroke',{stroke:{color:[1,0,0,1],size:0.2,erase:false,points:[[.5,.5,1]]}});
    await creativeStudio.execute('view.set',{mode:'film'});
  },clip);
  st=await page.evaluate(()=>creativeStudio.inspect());
  assert.equal(st.project.layers[0].scope.start,10);assert.equal(st.project.layers[0].scope.end,11);
  assert.equal(st.project.adjustments.length,1);assert.equal(st.project.workspace.playhead,10);
  const pixels=await page.evaluate(async()=>{
    const values=[];for(const f of [9,10,11,19,20]){const px=await creativeStudio.framePixels(f,320,180);const i=(90*320+160)*4;values.push([...px.slice(i,i+4)]);}return values;
  });
  assert.deepEqual(pixels[0],pixels[4]);assert.notDeepEqual(pixels[0],pixels[2]);assert.notDeepEqual(pixels[1],pixels[2]);assert.deepEqual(pixels[2],pixels[3]);
  await page.locator('#save').click();await page.waitForFunction(()=>document.querySelector('#save-status').textContent.startsWith('Sparat'));
  await page.reload();await page.waitForFunction(()=>document.querySelector('#status').textContent.startsWith('Redo'));
  await page.locator('#open').click();await page.locator('.saved-project').click();await page.waitForFunction(()=>document.querySelector('#status').textContent==='Projektet är öppnat');
  assert.equal((await page.evaluate(()=>creativeStudio.inspect())).project.layers.length,1);
  await page.locator('[data-mode=light]').click();await page.waitForFunction(()=>creativeStudio.inspect().project.workspace.mode==='light');
  await page.locator('#timeline-toggle').click();await page.waitForFunction(()=>document.querySelector('#timeline').hidden);assert.equal(await page.locator('#timeline').isVisible(),false);
  await page.locator('[data-mode=photo]').click();await page.waitForFunction(()=>creativeStudio.inspect().project.workspace.mode==='photo');assert.equal((await page.evaluate(()=>creativeStudio.inspect())).project.workspace.playhead,10);
  await page.screenshot({path:'test-results/workspace.png'});
  const pngDownload=page.waitForEvent('download');await page.locator('#export').click();await page.locator('#export-image').click();await (await pngDownload).saveAs('test-results/frame.png');
  const videoDownload=page.waitForEvent('download',{timeout:30000});await page.locator('#export-video').click();await (await videoDownload).saveAs('test-results/processed.webm');
  await page.waitForFunction(()=>!document.querySelector('#export-video').disabled);
  const info=JSON.parse(execFileSync('ffprobe',['-v','error','-show_streams','-show_format','-of','json','test-results/processed.webm'],{encoding:'utf8'}));
  assert.ok(info.streams.some(s=>s.codec_type==='video'));assert.ok(info.streams.some(s=>s.codec_type==='audio'));assert.ok(Number(info.format.duration)>1.7&&Number(info.format.duration)<2.5);
  // Decode the actual export. The red layer appears only around its scoped frame;
  // exposure is present in the interval and absent before/after it.
  const sample=t=>{const bytes=execFileSync('ffmpeg',['-v','error','-ss',String(t),'-i','test-results/processed.webm','-frames:v','1','-vf','format=rgb24,crop=1:1:160:90','-pix_fmt','rgb24','-f','rawvideo','-']);return [...bytes];};
  const before=sample(.2),after=sample(1.5),inside=sample(.6),painted=sample(.4);
  assert.ok(Math.abs(before[0]-after[0])<12);assert.ok(inside[0]>before[0]+15);assert.ok(painted[0]>painted[1]+60);
  const counted=JSON.parse(execFileSync('ffprobe',['-v','error','-count_frames','-select_streams','v:0','-show_entries','stream=nb_read_frames,r_frame_rate','-of','json','test-results/processed.webm'],{encoding:'utf8'}));
  assert.equal(Number(counted.streams[0].nb_read_frames),50);assert.equal(counted.streams[0].r_frame_rate,'25/1');
  const pcm=execFileSync('ffmpeg',['-v','error','-i','test-results/processed.webm','-vn','-ac','1','-ar','48000','-f','f32le','-'],{maxBuffer:2000000});
  let power=0;for(let i=0;i<pcm.length;i+=4){power+=pcm.readFloatLE(i)**2;}const rms=Math.sqrt(power/(pcm.length/4));assert.ok(rms>.05,'Exported audio must contain the original tone');
  assert.ok(Math.abs(pcm.length/4/48000-2)<.025,'Audio must match the two-second timeline, allowing Opus end padding');
  // A split retains the source offset, and muting the second half must produce
  // silence at that exact timeline position without shortening the video.
  const original=await page.evaluate(()=>creativeStudio.save());
  await page.evaluate(async()=>{await creativeStudio.execute('seek',{frame:25});await creativeStudio.execute('clip.split');const s=creativeStudio.inspect();await creativeStudio.execute('clip.mute',{id:s.project.clips[1].id});});
  const mutedDownload=page.waitForEvent('download',{timeout:30000});await page.locator('#export-video').click();await (await mutedDownload).saveAs('test-results/muted.webm');
  await page.waitForFunction(()=>!document.querySelector('#export-video').disabled);
  const mutedPcm=execFileSync('ffmpeg',['-v','error','-i','test-results/muted.webm','-vn','-ac','1','-ar','48000','-f','f32le','-'],{maxBuffer:2000000});
  const windowRms=(start,end)=>{let sum=0;for(let n=start*48000;n<end*48000;n++)sum+=mutedPcm.readFloatLE(n*4)**2;return Math.sqrt(sum/((end-start)*48000));};
  assert.ok(windowRms(.2,.8)>.05);assert.ok(windowRms(1.2,1.8)<.001,'Muted second clip must be silent');
  await page.evaluate(json=>creativeStudio.open(json),original);
  // Portable backup round-trip after removing the original from browser storage.
  await page.locator('#export-close').click();const backupDownload=page.waitForEvent('download');await page.locator('#backup').click();await (await backupDownload).saveAs('test-results/project.cstudio-backup');
  await page.evaluate(async()=>{const {dbRequest}=await import('/web/storage.js');await dbRequest('media','readwrite',s=>s.clear());});
  await page.locator('#project-input').setInputFiles('test-results/project.cstudio-backup');await page.waitForFunction(()=>document.querySelector('#status').textContent==='Projektet är öppnat');
  assert.equal((await page.evaluate(()=>creativeStudio.inspect())).project.layers.length,1);
  assert.deepEqual(errors,[]);
  console.log(JSON.stringify({result:'PASS',frames:50,pixels,export_samples:{before,painted,inside,after},duration:info.format.duration,streams:info.streams.map(s=>s.codec_type),export_frame_count:counted.streams[0].nb_read_frames,audio_rms:rms,browser_errors:errors},null,2));
} catch(error) {console.log('START/STATE',JSON.stringify({status:await page.locator('#status').textContent(),errors}));await page.screenshot({path:'test-results/browser-error.png'});throw error;} finally {await browser.close();await server.close();}
