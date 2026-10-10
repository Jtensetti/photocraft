import {chromium} from 'playwright';
import {createServer} from 'vite';
import assert from 'node:assert/strict';
import {mkdirSync} from 'node:fs';
mkdirSync('test-results',{recursive:true});
const server=await createServer({server:{host:'127.0.0.1',port:5179,strictPort:true}});await server.listen();
const browser=await chromium.launch({headless:true,args:['--no-sandbox']}),page=await browser.newPage({viewport:{width:1600,height:1000}}),errors=[];page.on('pageerror',e=>errors.push(e.message));
const state=()=>page.evaluate(()=>creativeStudio.inspect()),pixels=f=>page.evaluate(async f=>[...await creativeStudio.framePixels(f,160,100)],f);
const cmd=(id,params)=>page.evaluate(({id,params})=>creativeStudio.execute(id,params),{id,params});
async function drag(tool,a,b){await page.locator(`[data-tool="${tool}"]`).click();const r=await page.locator('#canvas').boundingBox();await page.mouse.move(r.x+r.width*a[0],r.y+r.height*a[1]);await page.mouse.down();await page.mouse.move(r.x+r.width*b[0],r.y+r.height*b[1],{steps:6});await page.mouse.up();}
try {
  await page.goto('http://127.0.0.1:5179');await page.waitForFunction(()=>document.querySelector('#status').textContent.startsWith('Redo'));await page.locator('#demo').click();await page.waitForFunction(()=>document.querySelector('#status').textContent.startsWith('Importerad'));
  const original=await pixels(0),canvas=await page.locator('#canvas').elementHandle();await cmd('selection.set',{clip_id:(await state()).active_clip.id,start:3,end:10});await cmd('view.set',{playhead:5,scope:'range'});
  for(const mode of ['photo','light','film','vector','design']){await page.locator(`[data-mode=${mode}]`).click();const s=await state();assert.equal(s.project.workspace.playhead,5);assert.deepEqual(s.project.workspace.selection,{clip_id:s.active_clip.id,start:3,end:10});assert.ok(await page.locator(`[data-panel=${mode}]`).isVisible());assert.equal(await canvas.evaluate(el=>el===document.querySelector('#canvas')),true);await page.screenshot({path:`test-results/workspace-${mode}.png`});}
  await page.locator('[data-mode=vector]').click();await drag('graphic:vector:shape.rectangle',[.12,.15],[.36,.5]);await page.waitForFunction(()=>creativeStudio.inspect().project.graphic_layers.some(l=>l.engine==='vector'&&l.calls.some(c=>c.command==='shape.rectangle')));
  const vector=await pixels(5);assert.notDeepEqual(vector,original);assert.deepEqual(await pixels(10),original);
  await page.locator('#vector-panel .graphic-object').last().click();await page.locator('#vector-panel [name=x]').fill('300');await page.locator('#vector-panel [data-action=transform]').click();await page.waitForFunction(()=>creativeStudio.inspect().project.graphic_layers.some(l=>l.calls.some(c=>c.command==='object.transform')));const moved=await pixels(5);assert.notDeepEqual(moved,vector);
  await page.locator('[data-mode=design]').click();await drag('graphic:design:frame.rectangle',[.6,.2],[.82,.6]);await page.waitForFunction(()=>creativeStudio.inspect().project.graphic_layers.some(l=>l.engine==='design'));
  await page.locator('#design-panel .graphic-object').last().click();await page.locator('#design-panel [data-fill]').selectOption('[Black]');await page.waitForFunction(()=>creativeStudio.inspect().project.graphic_layers.some(l=>l.engine==='design'&&l.calls.some(c=>c.command==='object.fill')));
  const combined=await pixels(5);assert.notDeepEqual(combined,moved);assert.deepEqual(await pixels(10),original);
  const saved=await page.evaluate(()=>creativeStudio.save());await page.evaluate(saved=>creativeStudio.open(saved),saved);assert.deepEqual(await pixels(5),combined);
  for(const mode of ['photo','light','film','vector','design']){await page.locator(`[data-mode=${mode}]`).click();assert.deepEqual(await pixels(5),combined);}
  await page.locator('#undo').click();assert.deepEqual(await pixels(5),moved);await page.locator('#redo').click();assert.deepEqual(await pixels(5),combined);
  await page.locator('#timeline-detail').click();assert.ok(await page.locator('.layer-operation').filter({hasText:'VectorCraft'}).count());assert.ok(await page.locator('.layer-operation').filter({hasText:'DesignCraft'}).count());await page.screenshot({path:'test-results/five-crafts-shared-canvas.png'});
  assert.deepEqual(errors,[]);console.log(JSON.stringify({result:'PASS',distinct_workspaces:5,permanent_canvas:true,scope:true,vector_transform:true,layout_fill:true,shared_save_and_history:true,timeline_graphics:true}));
} catch(e){console.log('STATUS',await page.locator('#status').textContent());await page.screenshot({path:'test-results/graphics-error.png'});throw e;}finally{await browser.close();await server.close();}
