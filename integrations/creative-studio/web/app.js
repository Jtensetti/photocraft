import './style.css';
import {icon} from './icons.js';
import {createDevelopUI} from './develop-ui.js';
import {createProUI} from './pro-ui.js';
import {createWorkspace} from './layout.js';
import {createFilmstrip} from './filmstrip.js';
import {exportTimeline} from './export.js';
import {putMedia,putMediaBatch,getMedia,putProject,projects,backup,readBackup} from './storage.js';
import {metadata,source,seek,pauseAll,clearSources,event} from './media.js';

const $=id=>document.getElementById(id);
const worker=new Worker(new URL('./worker.js',import.meta.url),{type:'module'});
const pending=new Map();let serial=0,state,dirty=false,busy=false,playing=false,playToken=0,exporting=false,cancelExport=false,tool='brush',renderToken=0,rendering=false,rerender=false;
let workspace,filmstrip,developUI,proUI,recoveryTimer,recovery,canvasGesture;
let projectId=crypto.randomUUID(); const thumbs=new Map();
worker.onmessage=({data:m})=>{const p=pending.get(m.id);if(!p)return;pending.delete(m.id);clearTimeout(p.timer);m.error?p.reject(new Error(m.error)):p.resolve(m.result);};
worker.onerror=()=>{for(const p of pending.values()){clearTimeout(p.timer);p.reject(new Error('Redigeringsmotorn stannade. Ladda om och öppna senast sparade projekt.'));}pending.clear();status('Redigeringsmotorn kunde inte köras',true);};
function rpc(op,params={},transfer=[]) {return new Promise((resolve,reject)=>{const id=++serial;const timer=setTimeout(()=>{pending.delete(id);reject(new Error('Redigeringsmotorn svarar inte'));},60000);pending.set(id,{resolve,reject,timer});worker.postMessage({id,op,...params},transfer);});}
function status(text,error=false){$('status').textContent=text;$('status').classList.toggle('error',error);}
const handle=fn=>async(...args)=>{try{await fn(...args);}catch(e){status(e.message,true);}};
function bind(id,fn){$(id).onclick=handle(fn);}
async function command(cmd,params={},render=true){
  if(exporting)throw new Error('Vänta tills exporten är klar');
  state=await rpc('command',{command:cmd,params});
  if(!['seek','selection.set','view.set'].includes(cmd)){dirty=true;$('save-status').textContent='Osparade ändringar';}
  if(!['seek','selection.set','view.set'].includes(cmd)){filmstrip?.invalidate();scheduleRecovery();}
  if(cmd==='seek')filmstrip?.follow();
  update();if(render)await paint();return state;
}
function editScope(){
  const w=state.project.workspace,c=state.active_clip;if(w.scope==='project')return 'project';
  if(!c)throw new Error('Importera media först');
  if(w.scope==='range'){if(!w.selection)throw new Error('Markera ett intervall först');return structuredClone(w.selection);}
  return {clip_id:c.id,start:w.scope==='clip'?0:state.local_frame,end:w.scope==='clip'?c.frames:state.local_frame+1};
}
function fps(){return state.project.fps.num/state.project.fps.den;}
function update(){
  const p=state.project,w=p.workspace,c=state.active_clip,a=p.assets.find(a=>a.id===c?.asset_id);
  $('app').setAttribute('aria-busy','false');$('project-title').textContent=p.name;
  document.querySelectorAll('[data-mode]').forEach(b=>b.classList.toggle('active',b.dataset.mode===w.mode));
  $('left-panel').hidden=!w.left_visible;$('right-panel').hidden=!w.right_visible;$('timeline').hidden=!w.timeline_visible;
  document.documentElement.style.setProperty('--timeline-height',w.timeline_height+'px');
  workspace?.sync(w.mode);
  $('tools').style.visibility='visible';
  $('scope').value=w.scope;
  const s=w.scope==='frame'&&c?{start:state.local_frame,end:state.local_frame+1}:w.scope==='clip'&&c?{start:0,end:c.frames}:w.selection;
  const scopeClip=p.clips.find(x=>x.id===(s?.clip_id||c?.id)),scopeAsset=p.assets.find(x=>x.id===scopeClip?.asset_id);
  $('scope-description').textContent=w.scope==='project'?'Alla klipp · fotografiska justeringar':s?`${scopeAsset?.name||'Klipp'} · ${s.start}–${s.end-1} · ${s.end-s.start} bildrutor`:'Markera ett intervall med I och O.';
  $('empty').hidden=!!p.clips.length;$('canvas').hidden=!p.clips.length;$('stroke-preview').hidden=!p.clips.length;
  $('resolution').textContent=`${p.width} × ${p.height}`;
  const f=w.playhead,F=Math.round(fps());$('timecode').textContent=[Math.floor(f/fps()/3600),Math.floor(f/fps()/60)%60,Math.floor(f/fps())%60,f%F].map(n=>String(n).padStart(2,'0')).join(':');
  $('frame-info').textContent=`Bildruta ${state.local_frame}`;$('sequence-info').textContent=`${fps().toFixed(fps()%1?3:0)} fps · ${state.total_frames} bildrutor`;
  $('scrub').max=Math.max(0,state.total_frames-1);$('scrub').value=f;
  $('undo').disabled=!state.can_undo;$('redo').disabled=!state.can_redo;
  $('play').textContent=playing?'Ⅱ':'▶';$('play').setAttribute('aria-label',playing?'Pausa':'Spela upp');
  $('range-info').textContent=w.selection?`${w.selection.start}–${w.selection.end-1}`:'Inget intervall';
  $('clip-details').textContent=a?`${a.name} · ${c.frames} bildrutor`:'Inget klipp valt';
  $('trim-start').max=Math.max(0,(c?.frames||1)-1);$('trim-end').max=c?.frames||1;$('trim-end').value=c?.frames||1;
  $('volume').value=Math.round((c?.volume??1)*100);$('volume-value').textContent=$('volume').value+' %';
  $('duration').disabled=a?.kind==='video';$('duration').value=c?(c.frames/fps()).toFixed(2):1;
  $('project-details').textContent=`${p.width} × ${p.height} · ${(state.total_frames/fps()).toFixed(2)} s · ${p.clips.length} klipp`;
  for(const id of Object.keys(controls)){const n=(w.scope==='project'?p.project_look?.[id]:state.look?.[id])??0;$(id).value=n;$('value-'+id).textContent=id==='exposure'?n.toFixed(2):String(n);}
  renderAssets();renderLayers();filmstrip?.refresh();developUI?.sync();proUI?.sync();
}
const controls={exposure:['Exponering',-5,5,.05,'light-controls'],contrast:['Kontrast',-100,100,1,'light-controls'],highlights:['Högdagrar',-100,100,1,'light-controls'],shadows:['Skuggor',-100,100,1,'light-controls'],temperature:['Temperatur',-100,100,1,'color-controls'],tint:['Nyans',-100,100,1,'color-controls'],saturation:['Mättnad',-100,100,1,'color-controls']};
for(const [id,[label,min,max,step,parent]] of Object.entries(controls)){
  const row=document.createElement('div');row.className='slider-row';row.innerHTML=`<div class="slider-label"><label for="${id}">${label}</label><output id="value-${id}">0</output></div><input id="${id}" type="range" min="${min}" max="${max}" step="${step}" value="0">`;
  $(parent).append(row);
  let latest,target,timer;$(id).oninput=handle(()=>{target=editScope();$('value-'+id).textContent=$(id).value;latest=Number($(id).value);clearTimeout(timer);timer=setTimeout(handle(async()=>{await stop();await command('develop.set',{values:{[id]:latest},scope:target});status('Justeringen gäller vald omfattning');}),110);});
}
function renderAssets(){
  const root=$('assets');root.replaceChildren();
  for(const a of state.project.assets){
    const b=document.createElement('button');b.className='asset'+(state.active_clip?.asset_id===a.id?' active':'');
    const img=document.createElement('img');img.className='asset-thumb';img.alt='';if(thumbs.has(a.id))img.src=thumbs.get(a.id);
    const text=document.createElement('div');const name=document.createElement('div');name.className='asset-name';name.textContent=a.name;const meta=document.createElement('div');meta.className='asset-meta';meta.textContent=`${a.kind==='video'?'Video':'Bild'} · ${a.width}×${a.height}`;text.append(name,meta);b.append(img,text);
    b.onclick=handle(async()=>{await stop();if(a.kind!=='blank'&&!await getMedia(a.id)){await relink(a);return;}let offset=0;for(const c of state.project.clips){if(c.asset_id===a.id){await command('seek',{frame:offset});break;}offset+=c.frames;}});root.append(b);
  }
}
function renderLayers(){
  const root=$('layers');root.replaceChildren();
  if($('layer-blend').options.length<2){$('layer-blend').replaceChildren(...state.blend_modes.map(b=>new Option(b.label,b.id)));}
  const selected=state.project.layers.find(l=>l.id===state.project.workspace.selected_layer);$('layer-edit').hidden=!selected;
  $('text-update').hidden=!selected?.text;
  if(selected?.text){$('text-content').value=selected.text.content;$('text-size').value=Math.round(selected.text.size*state.project.width);}
  $('layer-blend').disabled=!selected;$('layer-blend').value=selected?.blend||'Normal';
  for(const id of ['layer-duplicate','layer-down','layer-up'])$(id).disabled=!selected;
  if(selected){$('layer-name').value=selected.name;$('layer-scale').value=Math.round(selected.scale*100);$('layer-rotation').value=selected.rotation;$('layer-start').value=selected.scope.start;$('layer-end').value=selected.scope.end;$('layer-x').value=Math.round(selected.offset[0]*100);$('layer-y').value=Math.round(selected.offset[1]*100);}
  const layers=state.project.layers.filter(l=>l.scope.clip_id===state.active_clip?.id);
  if(!layers.length){const note=document.createElement('p');note.className='hint';note.textContent='Originalet ligger under dina lager.';root.append(note);}
  for(const l of [...layers].reverse()){
    const row=document.createElement('div');row.className='layer-row'+(state.project.workspace.selected_layer===l.id?' active':'');
    const eye=document.createElement('button');eye.innerHTML=icon(l.visible?'eye':'eye-off');eye.title='Visa / dölj lager';eye.onclick=handle(()=>command('layer.set',{id:l.id,visible:!l.visible}));
    const name=document.createElement('button');name.className='layer-name';name.textContent=l.name;name.onclick=handle(()=>command('view.set',{selected_layer:l.id},false));
    const opacity=document.createElement('input');opacity.type='number';opacity.min=0;opacity.max=100;opacity.value=Math.round(l.opacity*100);opacity.className='layer-opacity';opacity.setAttribute('aria-label','Lageropacitet');opacity.onchange=handle(()=>command('layer.set',{id:l.id,opacity:Number(opacity.value)/100}));
    const del=document.createElement('button');del.textContent='×';del.title='Ta bort lager';del.onclick=handle(()=>command('layer.delete',{id:l.id}));const thumb=document.createElement('span');thumb.className='layer-thumb';thumb.innerHTML=icon(l.text?'type':'brush');row.append(eye,thumb,name,opacity,del);
    const scope=document.createElement('div');scope.className='layer-scope';scope.textContent=`Bildruta ${l.scope.start}–${l.scope.end-1}`;root.append(row,scope);
  }
  const asset=state.project.assets.find(a=>a.id===state.active_clip?.asset_id);if(asset){const base=document.createElement('div');base.className='base-layer';const img=document.createElement('img');img.alt='';if(thumbs.has(asset.id))img.src=thumbs.get(asset.id);const name=document.createElement('span');name.textContent='Original';const label=document.createElement('small');label.textContent='Källa';base.append(img,name,label);root.append(base);}
}
const raw=document.createElement('canvas'),rawCtx=raw.getContext('2d',{willReadFrequently:true}),ctx=$('canvas').getContext('2d'),overlay=$('stroke-preview').getContext('2d');
async function framePixels(frame,width,height,realtime=false){
  let offset=0,clip,local;for(const c of state.project.clips){if(frame<offset+c.frames){clip=c;local=frame-offset;break;}offset+=c.frames;}
  if(!clip)throw new Error('Ingen bildruta');
  const asset=state.project.assets.find(a=>a.id===clip.asset_id),media=await source(asset);
  if(media instanceof HTMLVideoElement&&!realtime)await seek(media,(clip.source_in+local+.5)/fps());
  raw.width=width;raw.height=height;rawCtx.clearRect(0,0,width,height);
  const ratio=Math.min(width/asset.width,height/asset.height),w=asset.width*ratio,h=asset.height*ratio;
  rawCtx.drawImage(media,(width-w)/2,(height-h)/2,w,h);
  const data=rawCtx.getImageData(0,0,width,height).data;
  const result=await rpc('render',{width,height,frame,pixels:data.buffer},[data.buffer]);
  return new Uint8ClampedArray(result);
}
async function paint(realtime=false){
  if(!state?.project.clips.length)return;
  if(rendering){rerender=true;return;}
  rendering=true;const token=++renderToken;
  try{
    const p=state.project,ratio=Math.min(1,960/p.width,640/p.height),w=Math.max(1,Math.round(p.width*ratio)),h=Math.max(1,Math.round(p.height*ratio)),frame=p.workspace.playhead;
    const pixels=await framePixels(frame,w,h,realtime);
    if(token===renderToken){$('canvas').width=w;$('canvas').height=h;ctx.putImageData(new ImageData(pixels,w,h),0,0);developUI?.histogram(pixels);proUI?.sync();positionOverlay();}
  }finally{rendering=false;if(rerender){rerender=false;await paint(playing);}}
}
function positionOverlay(){const c=$('canvas'),o=$('stroke-preview');o.width=c.width;o.height=c.height;const r=c.getBoundingClientRect(),s=$('stage').getBoundingClientRect();o.style.width=r.width+'px';o.style.height=r.height+'px';o.style.left=(r.left-s.left+$('stage').scrollLeft)+'px';o.style.top=(r.top-s.top+$('stage').scrollTop)+'px';drawSelection();}
new ResizeObserver(()=>{if(state)positionOverlay();}).observe($('stage'));

async function importFile(file){
  if(exporting||busy)throw new Error('En annan åtgärd pågår');await stop();busy=true;
  try{
    status('Läser '+file.name+'…');const m=await metadata(file);
    if(m.kind==='video'&&!m.fps)throw new Error('Första versionen kräver MP4 med läsbar bildfrekvens.');
    if(m.variable)throw new Error('Den här videon har varierande bildfrekvens. Första versionen stöder video med konstant bildfrekvens.');
    if(m.fps&&state.project.clips.length&&Math.abs(m.fps-fps())>.001)throw new Error('Videons bildfrekvens måste matcha projektet. Skapa ett nytt projekt för den här filmen.');
    if(!state.project.clips.length){
      await command('project.new',{name:file.name.replace(/\.[^.]+$/,''),width:m.width,height:m.height,fps:m.fps||25},false);
    }
    const id=crypto.randomUUID();await putMedia(id,file);
    await command('asset.add',{asset:{id,name:file.name,kind:m.kind,width:m.width,height:m.height,bytes:file.size,source_fps:m.fps},frames:Math.max(1,Math.round((m.kind==='image'?1:m.duration)*fps()))});
    await thumbnail(id);update();status('Importerad · originalet är oförändrat');
  }finally{busy=false;}
}
async function thumbnail(id){
  const a=state.project.assets.find(a=>a.id===id);const media=await source(a);const c=document.createElement('canvas');c.width=160;c.height=90;const x=c.getContext('2d');x.drawImage(media,0,0,160,90);thumbs.set(id,c.toDataURL());
}
async function relink(asset){
  const input=document.createElement('input');input.type='file';input.accept=asset.kind==='video'?'video/*':'image/*';
  input.onchange=handle(async()=>{const file=input.files[0];if(!file)return;const m=await metadata(file);if(m.kind!==asset.kind||m.width!==asset.width||m.height!==asset.height||file.size!==asset.bytes)throw new Error('Filen matchar inte originalets typ, storlek och dimensioner');await putMedia(asset.id,file);clearSources();await thumbnail(asset.id);await paint();status('Originalet har återlänkats');});input.click();
}
function scheduleRecovery(){
  clearTimeout(recoveryTimer);recoveryTimer=setTimeout(handle(async()=>{
    if(!dirty)return;if(exporting||busy){scheduleRecovery();return;}const id=projectId,json=await rpc('save');
    await putProject({id:'recovery-'+id,original_id:id,name:state.project.name,json,modified:Date.now(),recovery:true});
  }),1500);
}
async function save(){await stop();const json=await rpc('save');await putProject({id:projectId,name:state.project.name,json,modified:Date.now()});dirty=false;$('save-status').textContent='Sparat '+new Date().toLocaleTimeString('sv-SE',{hour:'2-digit',minute:'2-digit'});status('Projektet är sparat lokalt');}
async function openJson(json,id=crypto.randomUUID()){
  await stop();const result=await rpc('open',{text:json});clearSources();filmstrip?.reset();thumbs.clear();state=result;projectId=id;dirty=false;update();
  for(const a of state.project.assets){try{await thumbnail(a.id);}catch{}}
  update();await paint();status('Projektet är öppnat');$('save-status').textContent='Öppnat projekt';
}
function download(blob,name){const url=URL.createObjectURL(blob),a=document.createElement('a');a.href=url;a.download=name;a.click();setTimeout(()=>URL.revokeObjectURL(url),30000);}
function safeName(){return state.project.name.replace(/[^\p{L}\p{N}_-]+/gu,'_')||'projekt';}
async function stop(){playing=false;playToken++;pauseAll();if(state)update();}
async function play(){
  if(exporting)throw new Error('Vänta tills exporten är klar');
  if(playing){await stop();return;}if(!state.total_frames)return;
  playing=true;update();const token=++playToken;let frame=state.project.workspace.playhead;
  let currentClip,start,first;
  while(playing&&token===playToken&&frame<state.total_frames){
    let offset=0,c;for(const clip of state.project.clips){if(frame<offset+clip.frames){c=clip;break;}offset+=clip.frames;}
    const a=state.project.assets.find(a=>a.id===c.asset_id),media=await source(a);
    if(currentClip!==c.id){pauseAll();currentClip=c.id;first=frame;if(media instanceof HTMLVideoElement){await seek(media,(c.source_in+frame-offset)/fps());media.muted=c.muted;media.volume=c.volume??1;await media.play();}start=performance.now();}
    if(!playing||token!==playToken)break;
    if(state.project.workspace.playhead!==frame){state=await rpc('command',{command:'seek',params:{frame}});update();}
    await paint(true);
    await new Promise(resolve=>requestAnimationFrame(resolve));
    // The media clock is the master for video and audio. Slow rendering may
    // skip preview frames, but must never advance the timeline ahead of audio.
    frame=media instanceof HTMLVideoElement
      ? (media.ended?offset+c.frames:Math.min(offset+c.frames,offset+Math.max(0,Math.floor(media.currentTime*fps()+1e-6)-c.source_in)))
      : Math.min(offset+c.frames,first+Math.floor((performance.now()-start)/1000*fps()));
  }
  if(token===playToken)await stop();
}

let points=[],drawing=false,drawingScope;
function point(e){const r=$('canvas').getBoundingClientRect();return [Math.max(0,Math.min(1,(e.clientX-r.left)/r.width)),Math.max(0,Math.min(1,(e.clientY-r.top)/r.height)),e.pointerType==='pen'?Math.max(.01,e.pressure):1];}
function selectTool(next){
  tool=next;document.querySelectorAll('[data-tool]').forEach(b=>b.classList.toggle('active',b.dataset.tool===tool));
  const names={move:'Flytta',select:'Markering',brush:'Pensel',erase:'Sudd',mask:'Lagermask',text:'Text',eyedropper:'Pipett',hand:'Hand',zoom:'Zoom'};$('tool-name').textContent=names[tool];
  $('canvas').style.cursor=({move:'move',hand:'grab',zoom:'zoom-in',text:'text'})[tool]||'crosshair';
  if(tool==='text'){workspace.open('photo');document.querySelector('.text-controls').open=true;}
}
function drawSelection(){const r=state?.project.workspace.pixel_selection;if(!r||drawing)return;const c=$('stroke-preview');overlay.clearRect(0,0,c.width,c.height);overlay.lineWidth=1;overlay.strokeStyle='white';overlay.setLineDash([5,4]);overlay.strokeRect(r[0]*c.width,r[1]*c.height,(r[2]-r[0])*c.width,(r[3]-r[1])*c.height);overlay.setLineDash([]);}
$('canvas').onpointerdown=handle(async e=>{
  if(exporting||playing||e.button!==0)return;
  if(tool==='zoom'){proUI.stepZoom(e.altKey);return;}
  if(tool==='eyedropper'){const [x,y]=point(e),c=$('canvas'),color=ctx.getImageData(Math.min(c.width-1,Math.floor(x*c.width)),Math.min(c.height-1,Math.floor(y*c.height)),1,1).data;$('color').value='#'+[...color].slice(0,3).map(n=>n.toString(16).padStart(2,'0')).join('');status('Färg hämtad från canvasen');return;}
  if(tool==='hand'){canvasGesture={tool,x:e.clientX,y:e.clientY,left:$('stage').scrollLeft,top:$('stage').scrollTop};$('canvas').setPointerCapture(e.pointerId);return;}
  if(tool==='move'){const layer=state.project.layers.find(l=>l.id===state.project.workspace.selected_layer);if(!layer)throw new Error('Välj ett lager att flytta.');canvasGesture={tool,id:layer.id,start:point(e),offset:[...layer.offset]};$('canvas').setPointerCapture(e.pointerId);return;}
  if(tool==='text'){if(state.project.workspace.scope==='project')throw new Error('Välj bildruta, intervall eller klipp för text.');await command('layer.text',{text:{...textSettings(),position:point(e).slice(0,2)},scope:editScope()});$('text-content').focus();$('text-content').select();return;}
  if(!workspace.has('photo')){workspace.open('photo');}
  if(tool!=='select'&&state.project.workspace.scope==='project')throw new Error('Välj bildruta, intervall eller klipp för att måla eller maskera.');
  if(tool==='mask'&&!state.project.workspace.selected_layer)throw new Error('Välj ett lager för att maskera.');
  drawingScope=tool==='select'?null:editScope();drawing=true;points=[point(e)];$('canvas').setPointerCapture(e.pointerId);drawOverlay();
});
$('canvas').onpointermove=e=>{
  if(canvasGesture?.tool==='hand'){const g=canvasGesture;$('stage').scrollLeft=g.left+g.x-e.clientX;$('stage').scrollTop=g.top+g.y-e.clientY;return;}
  if(canvasGesture?.tool==='move'){canvasGesture.end=point(e);return;}
  if(drawing&&points.length<20000){points.push(point(e));drawOverlay();}
};
function drawOverlay(){const c=$('stroke-preview');
  if(tool==='mask'||tool==='select'){overlay.clearRect(0,0,c.width,c.height);const first=points[0],last=points.at(-1);overlay.strokeStyle='white';overlay.lineWidth=1;overlay.setLineDash([5,4]);overlay.strokeRect(first[0]*c.width,first[1]*c.height,(last[0]-first[0])*c.width,(last[1]-first[1])*c.height);overlay.setLineDash([]);return;}
  overlay.clearRect(0,0,c.width,c.height);overlay.strokeStyle=$('color').value;overlay.globalAlpha=Number($('brush-opacity').value)/100;overlay.lineWidth=Number($('brush-size').value)/state.project.width*c.width;overlay.lineCap='round';overlay.lineJoin='round';overlay.beginPath();for(const [i,p]of points.entries()){i?overlay.lineTo(p[0]*c.width,p[1]*c.height):overlay.moveTo(p[0]*c.width,p[1]*c.height);}if(points.length===1)overlay.lineTo(points[0][0]*c.width+.1,points[0][1]*c.height+.1);overlay.stroke();overlay.globalAlpha=1;
}
$('canvas').onpointerup=handle(async()=>{
  if(canvasGesture){const g=canvasGesture;canvasGesture=null;if(g.tool==='move'&&g.end){await command('layer.set',{id:g.id,offset:g.offset.map((n,i)=>Math.max(-1,Math.min(1,n+g.end[i]-g.start[i])))});}return;}
  if(!drawing)return;drawing=false;
  if(tool==='mask'||tool==='select'){const a=points[0],b=points.at(-1),rect=[Math.min(a[0],b[0]),Math.min(a[1],b[1]),Math.max(a[0],b[0]),Math.max(a[1],b[1])];overlay.clearRect(0,0,$('stroke-preview').width,$('stroke-preview').height);if(rect[0]===rect[2]||rect[1]===rect[3])return;if(tool==='select')await command('view.set',{pixel_selection:rect},false);else await command('layer.set',{id:state.project.workspace.selected_layer,mask:rect});drawSelection();status(tool==='select'?'Bildområdet markerat · nya penseldrag begränsas till markeringen':'Lagermask sparad');return;}
  const color=$('color').value.match(/\w\w/g).map(h=>parseInt(h,16)/255);const stroke={points:points.slice(),color:[...color,1],size:Math.max(.0001,Math.min(.5,Number($('brush-size').value)/state.project.width)),erase:tool==='erase',hardness:Number($('brush-hardness').value)/100,opacity:Number($('brush-opacity').value)/100,flow:Number($('brush-flow').value)/100,pressure_size:$('brush-pressure').checked,selection:state.project.workspace.pixel_selection};overlay.clearRect(0,0,$('stroke-preview').width,$('stroke-preview').height);await command('layer.stroke',{stroke,scope:drawingScope});drawSelection();status('Penseldraget sparat i valt intervall');
});
$('canvas').onpointercancel=()=>{drawing=false;canvasGesture=null;points=[];overlay.clearRect(0,0,$('stroke-preview').width,$('stroke-preview').height);drawSelection();};
document.querySelectorAll('[data-tool]').forEach(b=>b.onclick=()=>selectTool(b.dataset.tool));
document.querySelectorAll('[data-mode]').forEach(b=>b.onclick=handle(async()=>{workspace.open(b.dataset.mode);await command('view.set',{mode:b.dataset.mode,left_visible:true,right_visible:true},false);}));
$('scope').onchange=handle(()=>command('view.set',{scope:$('scope').value},false));
for(const id of ['import','add-media','empty-import'])bind(id,()=>$('media-input').click());
$('media-input').onchange=handle(async()=>{for(const file of $('media-input').files)await importFile(file);$('media-input').value='';});
$('stage').ondragover=e=>{e.preventDefault();};$('stage').ondrop=handle(async e=>{e.preventDefault();for(const f of e.dataTransfer.files)await importFile(f);});
bind('blank',()=>command('asset.add',{asset:{id:crypto.randomUUID(),name:'Tom bild',kind:'blank',width:state.project.width,height:state.project.height,bytes:0,source_fps:null},frames:Math.round(fps())}));
bind('demo',async()=>{const response=await fetch(new URL('demo/great-wave.jpg',document.baseURI));if(!response.ok)throw new Error('Exempelbilden kunde inte öppnas');const blob=await response.blob();await importFile(new File([blob],'The Great Wave.jpg',{type:'image/jpeg'}));});
bind('new',async()=>{if(dirty){await save();}await stop();clearSources();filmstrip?.reset();thumbs.clear();projectId=crypto.randomUUID();await command('project.new',{},false);$('save-status').textContent='Nytt projekt';status('Skapa en bild eller importera media');});
bind('recover',async()=>{if(recovery){await openJson(recovery.json,recovery.original_id);$('recover').hidden=true;status('Projektet återställt från automatisk återställningspunkt');}});
bind('save',save);bind('undo',async()=>{await stop();await command('undo');});bind('redo',()=>command('redo'));
bind('backup',async()=>{await save();download(await backup(await rpc('save')),safeName()+'.cstudio-backup');status('Säkerhetskopian innehåller projektet och originalmedia');});
bind('open',async()=>{const all=await projects(),seen=new Set();const list=all.sort((a,b)=>b.modified-a.modified).filter(p=>{const id=p.original_id||p.id;if(seen.has(id))return false;seen.add(id);return true;});$('saved-projects').replaceChildren();for(const p of list){const b=document.createElement('button');b.type='button';b.className='saved-project';b.textContent=p.name;const small=document.createElement('small');small.textContent=(p.recovery?'Återställningspunkt · ':'')+new Date(p.modified).toLocaleString('sv-SE');b.append(small);b.onclick=handle(async()=>{if(dirty)await save();await openJson(p.json,p.original_id||p.id);$('open-dialog').close();});$('saved-projects').append(b);}if(!list.length){$('saved-projects').textContent='Inga projekt är sparade på den här enheten ännu.';}$('open-dialog').showModal();});
bind('open-file',()=>$('project-input').click());
$('project-input').onchange=handle(async()=>{const f=$('project-input').files[0];if(!f)return;if(dirty)await save();
  if(f.name.endsWith('.cstudio-backup')){const b=await readBackup(f);const previous=await rpc('save');try{await rpc('open',{text:b.json});await putMediaBatch(b.parts);await openJson(b.json);}catch(e){state=await rpc('open',{text:previous});update();throw e;}}
  else{if(f.size>32_000_000)throw new Error('Projektfilen är för stor');await openJson(await f.text());}
  $('open-dialog').close();$('project-input').value='';});
bind('play',play);bind('prev',async()=>{await stop();await command('seek',{frame:Math.max(0,state.project.workspace.playhead-1)});});bind('next',async()=>{await stop();await command('seek',{frame:state.project.workspace.playhead+1});});
$('scrub').oninput=handle(async()=>{await stop();await command('seek',{frame:Number($('scrub').value)});});
bind('timeline-toggle',()=>command('view.set',{timeline_visible:!state.project.workspace.timeline_visible},false));
bind('left-show',()=>command('view.set',{left_visible:!state.project.workspace.left_visible},false));
bind('right-toggle',()=>command('view.set',{right_visible:!state.project.workspace.right_visible},false));
bind('focus',()=>{const w=state.project.workspace;return command('view.set',{left_visible:!w.left_visible&&!w.right_visible,right_visible:!w.left_visible&&!w.right_visible,timeline_visible:!w.left_visible&&!w.right_visible},false);});
bind('layout-save',()=>$('layout-dialog').showModal());
$('layout-dialog').addEventListener('close',()=>{if($('layout-dialog').returnValue==='save')workspace.save($('layout-name').value.trim()||'Min arbetsyta');});
bind('project-title',()=>{$('rename-name').value=state.project.name;$('rename-dialog').showModal();});
$('rename-dialog').addEventListener('close',handle(async()=>{if($('rename-dialog').returnValue==='save')await command('project.rename',{name:$('rename-name').value},false);}));
bind('layout-reset',async()=>{workspace.reset();await command('view.set',{left_visible:true,right_visible:true,timeline_visible:true,timeline_height:250},false);});
let resizeStart;$('resize-handle').onpointerdown=e=>{resizeStart={y:e.clientY,h:state.project.workspace.timeline_height};e.target.setPointerCapture(e.pointerId);};
$('resize-handle').onpointermove=e=>{if(resizeStart)document.documentElement.style.setProperty('--timeline-height',Math.max(120,Math.min(480,resizeStart.h+resizeStart.y-e.clientY))+'px');};
$('resize-handle').onpointerup=handle(async e=>{if(!resizeStart)return;const h=Math.max(120,Math.min(480,resizeStart.h+resizeStart.y-e.clientY));resizeStart=null;await command('view.set',{timeline_height:h},false);});
let inPoint;
bind('set-in',()=>{if(!state.active_clip)return;inPoint={clip_id:state.active_clip.id,start:state.local_frame};status('Intervallets start är satt till bildruta '+state.local_frame);});
bind('set-out',async()=>{if(!inPoint||inPoint.clip_id!==state.active_clip?.id)throw new Error('Sätt intervallets start med I i samma klipp först');await command('selection.set',{clip_id:inPoint.clip_id,start:Math.min(inPoint.start,state.local_frame),end:Math.max(inPoint.start,state.local_frame)+1},false);status('Intervallet är markerat');});
bind('layer-new',()=>command('layer.new'));bind('split',async()=>{await stop();await command('clip.split');});bind('mute',()=>command('clip.mute',{id:state.active_clip?.id}));
$('duration').onchange=handle(()=>command('clip.duration',{id:state.active_clip?.id,frames:Math.round(Number($('duration').value)*fps())}));
bind('trim',async()=>{const params={id:state.active_clip?.id,start:Number($('trim-start').value),end:Number($('trim-end').value)};await stop();await command('clip.trim',params);status('Klippet är trimmat. Redigeringarna följer källbildrutorna.');});
for(const [id,delta] of [['clip-earlier',-1],['clip-later',1]])bind(id,async()=>{await stop();const index=state.project.clips.findIndex(c=>c.id===state.active_clip?.id);await command('clip.move',{id:state.active_clip?.id,index:Math.max(0,Math.min(state.project.clips.length-1,index+delta))});});
$('volume').onchange=handle(()=>command('clip.volume',{id:state.active_clip?.id,volume:Number($('volume').value)/100}));
function textSettings(){const color=$('color').value.match(/\w\w/g).map(h=>parseInt(h,16)/255);return {content:$('text-content').value,size:Number($('text-size').value)/state.project.width,color:[...color,1],position:[.1,.65]};}
bind('text-add',()=>command('layer.text',{text:textSettings(),scope:editScope()}));
bind('text-update',()=>command('layer.set',{id:state.project.workspace.selected_layer,text:textSettings()}));
bind('layer-scope-apply',()=>command('layer.scope',{id:state.project.workspace.selected_layer,start:Number($('layer-start').value),end:Number($('layer-end').value)}));
bind('mask-clear',()=>command('layer.set',{id:state.project.workspace.selected_layer,mask:null}));
for(const id of ['layer-x','layer-y'])$(id).onchange=handle(()=>command('layer.set',{id:state.project.workspace.selected_layer,offset:[Number($('layer-x').value)/100,Number($('layer-y').value)/100]}));
$('layer-blend').onchange=handle(()=>command('layer.set',{id:state.project.workspace.selected_layer,blend:$('layer-blend').value}));
$('layer-name').onchange=handle(()=>command('layer.set',{id:state.project.workspace.selected_layer,name:$('layer-name').value}));
for(const id of ['layer-scale','layer-rotation'])$(id).onchange=handle(()=>command('layer.set',{id:state.project.workspace.selected_layer,scale:Number($('layer-scale').value)/100,rotation:Number($('layer-rotation').value)}));
bind('layer-duplicate',()=>command('layer.duplicate',{id:state.project.workspace.selected_layer}));
for(const [id,delta] of [['layer-up',1],['layer-down',-1]])bind(id,()=>{const layers=state.project.layers,index=layers.findIndex(l=>l.id===state.project.workspace.selected_layer);return command('layer.move',{id:state.project.workspace.selected_layer,index:Math.max(0,Math.min(layers.length-1,index+delta))});});
bind('selection-clear',async()=>{await command('view.set',{pixel_selection:null},false);overlay.clearRect(0,0,$('stroke-preview').width,$('stroke-preview').height);});
bind('reset-look',()=>command('develop.reset',{scope:editScope()}));
bind('export',async()=>{await stop();$('export-description').textContent=`${state.project.width} × ${state.project.height} · ${fps().toFixed(3)} fps`;$('export-dialog').showModal();});bind('export-close',()=>$('export-dialog').close());
bind('export-image',async()=>{const p=state.project;const pixels=await framePixels(p.workspace.playhead,p.width,p.height);const c=document.createElement('canvas');c.width=p.width;c.height=p.height;c.getContext('2d').putImageData(new ImageData(pixels,p.width,p.height),0,0);download(await new Promise(r=>c.toBlob(r,'image/png')),safeName()+'.png');status('PNG exporterad');});
bind('export-cancel',()=>{cancelExport=true;});
bind('export-video',()=>exportVideo());
bind('export-file',async()=>{if(!window.showSaveFilePicker)throw new Error('Direktlagring kräver Chrome eller Edge på desktop.');const file=await showSaveFilePicker({suggestedName:safeName()+'.webm',types:[{description:'WebM-video',accept:{'video/webm':['.webm']}}]});await exportVideo(file);});
async function exportVideo(file){
  if(exporting||!state.total_frames)throw new Error('Importera media först');
  await stop();exporting=true;cancelExport=false;
  $('export-progress').hidden=false;$('export-cancel').hidden=false;$('export-image').disabled=true;$('export-video').disabled=true;$('export-close').disabled=true;
  let sink;
  $('export-file').disabled=true;
  try{
    if(file)sink=await file.createWritable();
    const blob=await exportTimeline({state,framePixels,sink,cancelled:()=>cancelExport,progress:value=>{$('export-progress').value=value;status(`Exporterar ${Math.round(value*100)} %…`);}});
    if(sink)await sink.close();else download(blob,safeName()+'.webm');status('Video med samtliga bildrutor och redigeringar exporterad');
  }catch(error){if(sink)await sink.abort().catch(()=>{});throw error;}finally{
    $('export-file').disabled=false;
    exporting=false;$('export-progress').hidden=true;$('export-cancel').hidden=true;$('export-image').disabled=false;$('export-video').disabled=false;$('export-close').disabled=false;
    await paint();
  }
}
window.addEventListener('beforeunload',e=>{if(dirty||exporting){e.preventDefault();e.returnValue='';}});
window.addEventListener('keydown',handle(async e=>{
  if(['INPUT','TEXTAREA','SELECT'].includes(document.activeElement.tagName)||document.querySelector('dialog[open]'))return;
  if(e.ctrlKey||e.metaKey){if(e.key.toLowerCase()==='n'){e.preventDefault();$('new').click();}else if(e.key.toLowerCase()==='o'){e.preventDefault();$('open').click();}else if(e.key.toLowerCase()==='j'){e.preventDefault();$('layer-duplicate').click();}else if(e.key.toLowerCase()==='d'){e.preventDefault();$('selection-clear').click();}else if(e.key.toLowerCase()==='s'){e.preventDefault();await save();}else if(e.key.toLowerCase()==='z'){e.preventDefault();await command(e.shiftKey?'redo':'undo');}return;}
  if(e.altKey&&e.key.toLowerCase()==='i'){e.preventDefault();selectTool('eyedropper');return;}
  const hotkeys={v:'move',m:'select',b:'brush',e:'erase',t:'text',h:'hand',z:'zoom'};if(hotkeys[e.key.toLowerCase()]){e.preventDefault();selectTool(hotkeys[e.key.toLowerCase()]);return;}if(e.key==='Tab'){e.preventDefault();$('focus').click();return;}if(e.key.toLowerCase()==='c'){e.preventDefault();$('split').click();return;}
  if(e.code==='Space'){e.preventDefault();await play();}else if(e.key==='ArrowRight')$('next').click();else if(e.key==='ArrowLeft')$('prev').click();else if(e.key.toLowerCase()==='i')$('set-in').click();else if(e.key.toLowerCase()==='o')$('set-out').click();
}));
workspace=createWorkspace({status,getPresentation:()=>{const {timeline_visible,left_visible,right_visible,timeline_height}=state.project.workspace;return {timeline_visible,left_visible,right_visible,timeline_height};},applyPresentation:params=>command('view.set',params,false)});
filmstrip=createFilmstrip({getState:()=>state,command,stop,rpc,status,isPlaying:()=>playing||exporting});
developUI=createDevelopUI({getState:()=>state,command,editScope,status});
proUI=createProUI({getState:()=>state,command,selectTool,status,updateOverlay:positionOverlay});
window.creativeStudio={selectTool,developUI,proUI,workspace,filmstrip,execute:command,inspect:()=>structuredClone(state),save:()=>rpc('save'),open:openJson,importFile,framePixels,exportVideo};
handle(async()=>{state=await rpc('inspect');const legacy=localStorage.getItem('creative-studio-layout'),l=workspace.inspect().presentation||(legacy?JSON.parse(legacy):null);if(l){state=await rpc('command',{command:'view.set',params:{timeline_visible:l.timeline_visible,left_visible:l.left_visible,right_visible:l.right_visible,timeline_height:l.timeline_height}});}if(matchMedia('(max-width:780px)').matches)state=await rpc('command',{command:'view.set',params:{left_visible:false,right_visible:false}});update();const list=await projects();recovery=list.filter(p=>p.recovery&&!list.some(saved=>saved.id===p.original_id&&saved.modified>=p.modified)).sort((a,b)=>b.modified-a.modified)[0];$('recover').hidden=!recovery;status('Redo · importera media eller skapa en bild');})();
