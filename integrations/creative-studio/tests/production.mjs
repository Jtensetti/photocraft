import {chromium} from 'playwright';
import {createServer} from 'node:http';
import {readFile} from 'node:fs/promises';
import {resolve,extname} from 'node:path';
import assert from 'node:assert/strict';

// Serve the built app from a subdirectory, catching worker/WASM path regressions.
const root=resolve('dist'),types={'.wasm':'application/wasm','.js':'text/javascript','.css':'text/css','.html':'text/html','.json':'application/json','.jpg':'image/jpeg'};
const server=createServer(async(req,res)=>{
  try{
    const url=new URL(req.url,'http://localhost'),prefix='/creative-studio/';
    if(!url.pathname.startsWith(prefix)){res.writeHead(404).end();return;}
    const file=resolve(root,decodeURIComponent(url.pathname.slice(prefix.length))||'index.html');
    if(!file.startsWith(root+'/')){res.writeHead(403).end();return;}
    const bytes=await readFile(file);res.setHeader('Content-Type',types[extname(file)]||'text/plain');res.end(bytes);
  }catch{res.writeHead(404).end();}
});
await new Promise(r=>server.listen(5180,'127.0.0.1',r));
const browser=await chromium.launch({headless:true,args:['--no-sandbox']});
try{
  const page=await browser.newPage({viewport:{width:1440,height:950}}),errors=[];
  page.on('pageerror',e=>errors.push(e.message));await page.goto('http://127.0.0.1:5180/creative-studio/');
  await page.waitForFunction(()=>document.querySelector('#status').textContent.startsWith('Redo'));
  await page.locator('#demo').click();await page.waitForFunction(()=>document.querySelector('#status').textContent.startsWith('Importerad'));
  await page.locator('[data-mode=light]').click();
  await page.evaluate(()=>creativeStudio.execute('develop.set',{values:{exposure:.3}}));
  const s=await page.evaluate(()=>creativeStudio.inspect());assert.equal(s.project.adjustments.length,1);assert.equal(s.total_frames,25);
  await page.screenshot({path:'test-results/production.png'});
  assert.deepEqual(errors,[]);console.log('PASS: production worker/WASM, render, mode switch and subdirectory hosting');
}finally{await browser.close();await new Promise(r=>server.close(r));}
