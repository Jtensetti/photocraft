import {getMedia} from './storage.js';
import {event,seek} from './media.js';

export function createFilmstrip({getState,command,stop,rpc,status,isPlaying}){
  const $=id=>document.getElementById(id),root=$('tracks'),scroll=$('timeline-scroll'),ruler=$('ruler');
  let scale=1,origin=0,length=0,signature='',visibleSignature='',revision=0,detail=false,anchor=null,gesture=null,token=0,generating=false,follow=false;
  const images=new Map(),decoders=new Map();let queue=[];
  const run=fn=>async(...args)=>{try{await fn(...args);}catch(e){status(e.message,true);}};
  function segments(){const s=getState();let at=0;return s.project.clips.map(clip=>{const start=at;at+=clip.frames;return {clip,start,asset:s.project.assets.find(a=>a.id===clip.asset_id)};});}
  function position(e,segment){const x=e.clientX-root.getBoundingClientRect().left;return Math.max(0,Math.min(segment.clip.frames-1,Math.floor(origin+x/scale)-segment.start));}
  async function select(segment,local,shift=false){
    await stop();
    if(shift&&anchor&&anchor.id===segment.clip.id){await command('selection.set',{clip_id:segment.clip.id,start:Math.min(anchor.local,local),end:Math.max(anchor.local,local)+1},false);}
    else{anchor={id:segment.clip.id,local};await command('view.set',{scope:'frame'},false);}
    await command('seek',{frame:segment.start+local});
  }
  function refresh(){
    const s=getState();if(!s)return;
    const fit=Math.max(1,scroll.clientWidth-34)/Math.max(1,s.total_frames),zoom=Number($('timeline-zoom').value)/100;
    scale=fit*Math.pow(Math.max(1,76/fit),zoom);
    // Browsers cap element widths. Rebase a very long, frame-zoomed timeline
    // around the playhead rather than creating a hundred-million-pixel div.
    const maxFrames=Math.floor(4_000_000/scale);origin=s.total_frames>maxFrames?Math.max(0,Math.min(s.total_frames-maxFrames,s.project.workspace.playhead-Math.floor(maxFrames/2))):0;
    length=Math.min(s.total_frames,maxFrames);
    const sig=JSON.stringify([s.project.clips,s.project.layers,s.project.adjustments,s.project.project_look,revision,zoom,scroll.clientWidth,origin,detail]);
    if(sig!==signature){signature=sig;visibleSignature='';token++;queue=[];root.replaceChildren();ruler.replaceChildren();$('operation-tracks').replaceChildren();
      const width=Math.max(scroll.clientWidth-34,length*scale);root.style.width=width+'px';ruler.style.width=width+'px';$('operation-tracks').style.width=width+'px';
      root.classList.toggle('frame-view',scale>=65);
      for(const seg of segments()){
        const left=Math.max(origin,seg.start),end=Math.min(origin+length,seg.start+seg.clip.frames);if(left>=end)continue;
        const c=document.createElement('div');c.className='clip';c.dataset.clipId=seg.clip.id;c.setAttribute('role','group');c.setAttribute('aria-label',seg.asset.name);c.style.left=((left-origin)*scale)+'px';c.style.width=((end-left)*scale)+'px';
        const title=document.createElement('div');title.className='clip-title';title.draggable=true;title.title='Dra rubriken för att ordna om klippet';title.ondragstart=e=>{e.dataTransfer.setData('text/studio-clip',seg.clip.id);e.dataTransfer.effectAllowed='move';};title.textContent=`${seg.asset.name} · ${(seg.clip.frames/(s.project.fps.num/s.project.fps.den)).toFixed(2)} s`;
        const cells=document.createElement('div');cells.className='frame-cells';c.append(title,cells);root.append(c);
        c.onpointerdown=e=>{if(e.target.closest('.clip-title'))return;if(e.button!==0)return;e.preventDefault();const local=position(e,seg);gesture={seg,local,current:local,shift:e.shiftKey,x:e.clientX,y:e.clientY,moved:false,element:c};c.setPointerCapture(e.pointerId);};
        c.onpointermove=e=>{if(!gesture||gesture.element!==c)return;gesture.current=position(e,seg);if(Math.abs(e.clientX-gesture.x)>4){gesture.moved=true;let overlay=c.querySelector('.drag-range');if(!overlay){overlay=document.createElement('div');overlay.className='range-overlay drag-range';c.append(overlay);}overlay.style.left=((Math.min(gesture.local,gesture.current)+seg.start-left)*scale)+'px';overlay.style.width=((Math.abs(gesture.local-gesture.current)+1)*scale)+'px';}};
        c.onpointerup=run(async()=>{const g=gesture;gesture=null;if(!g)return;g.element.querySelector('.drag-range')?.remove();if(g.moved){await stop();anchor={id:seg.clip.id,local:g.local};await command('selection.set',{clip_id:seg.clip.id,start:Math.min(g.local,g.current),end:Math.max(g.local,g.current)+1},false);await command('seek',{frame:seg.start+g.current});}else await select(seg,g.current,g.shift);refresh();});
        c.ondragover=e=>{if(e.dataTransfer.types.includes('text/studio-clip')){e.preventDefault();c.classList.add('drop-target');}};c.ondragleave=()=>c.classList.remove('drop-target');
        c.ondrop=run(async e=>{e.preventDefault();c.classList.remove('drop-target');const id=e.dataTransfer.getData('text/studio-clip'),clips=getState().project.clips,from=clips.findIndex(x=>x.id===id),to=clips.findIndex(x=>x.id===seg.clip.id);if(from<0||to<0)return;const after=e.clientX>c.getBoundingClientRect().left+c.clientWidth/2;let index=to+(after?1:0);if(from<index)index--;await stop();await command('clip.move',{id,index:Math.max(0,Math.min(clips.length-1,index))});});
        c.onpointercancel=()=>{gesture=null;c.querySelector('.drag-range')?.remove();};
        c.ondblclick=run(()=>command('view.set',{scope:'clip'},false));
        c.onkeydown=run(async e=>{if(e.key==='Enter'||e.key===' '){e.preventDefault();await select(seg,Math.max(0,left-seg.start),e.shiftKey);}});c.tabIndex=0;
      }
      const F=s.project.fps.num/s.project.fps.den,step=Math.max(1,Math.ceil(90/scale));
      // Only visible ruler labels are produced below, like the frame thumbnails.
      ruler.dataset.step=step;ruler.dataset.fps=F;
      if(detail)renderOperations(s);
      renderVisible();
    }
    for(const el of root.querySelectorAll('.playhead,.range-overlay:not(.drag-range),.edit-marker'))el.remove();
    for(const seg of segments()){
      const c=[...root.children].find(el=>el.dataset.clipId===seg.clip.id);if(!c)continue;
      const left=Math.max(origin,seg.start);c.classList.toggle('active',s.active_clip?.id===seg.clip.id);
      if(s.active_clip?.id===seg.clip.id){const p=document.createElement('div');p.className='playhead';p.style.left=((s.project.workspace.playhead-left)*scale)+'px';c.append(p);}
      const range=s.project.workspace.selection;
      if(range?.clip_id===seg.clip.id&&s.project.workspace.scope==='range'){const p=document.createElement('div');p.className='range-overlay';p.style.left=((seg.start+range.start-left)*scale)+'px';p.style.width=((range.end-range.start)*scale)+'px';c.append(p);}
      for(const edit of s.project.adjustments.filter(a=>a.scope.clip_id===seg.clip.id)){const p=document.createElement('div');p.className='edit-marker';p.title='LightCraft-justering';p.style.left=((seg.start+edit.scope.start-left)*scale)+'px';p.style.width=((edit.scope.end-edit.scope.start)*scale)+'px';c.append(p);}
    }
    if(follow){const x=(s.project.workspace.playhead-origin)*scale;if(x<scroll.scrollLeft||x>scroll.scrollLeft+scroll.clientWidth-100)scroll.scrollLeft=Math.max(0,x-scroll.clientWidth/3);follow=false;}
    renderVisible();
  }
  function renderOperations(s){
    const tracks=$('operation-tracks');
    for(const seg of segments()){
      const audio=document.createElement('div');audio.className='operation audio-operation';audio.style.left=((seg.start-origin)*scale)+'px';audio.style.width=(seg.clip.frames*scale)+'px';audio.textContent=seg.asset.kind==='video'?`${seg.clip.muted?'Ljud av':'Originalljud'} · ${Math.round(seg.clip.volume*100)} %`:'Stillbild · inget ljud';audio.style.top='0px';tracks.append(audio);
    }
    let row=1;
    for(const layer of s.project.layers){const seg=segments().find(x=>x.clip.id===layer.scope.clip_id);if(!seg)continue;const y=row++;const item=document.createElement('div');item.className='operation layer-operation';item.dataset.layerId=layer.id;item.style.left=((seg.start+layer.scope.start-origin)*scale)+'px';item.style.width=((layer.scope.end-layer.scope.start)*scale)+'px';item.style.top=(y*30)+'px';item.textContent=layer.name;item.title=`${layer.name} · bildruta ${layer.scope.start}–${layer.scope.end-1}`;
      item.onclick=run(()=>command('view.set',{selected_layer:layer.id},false));
      for(const edge of ['start','end']){const grip=document.createElement('span');grip.className='layer-grip '+edge;grip.title='Dra för att ändra lagrets tidsintervall';item.append(grip);grip.onpointerdown=e=>{e.stopPropagation();const x=e.clientX,old=layer.scope[edge];grip.setPointerCapture(e.pointerId);let next=old;grip.onpointermove=ev=>{const local=Math.round(old+(ev.clientX-x)/scale);next=edge==='start'?Math.max(0,Math.min(layer.scope.end-1,local)):Math.max(layer.scope.start+1,Math.min(seg.clip.frames,local));item.title=`${edge==='start'?next:layer.scope.start}–${edge==='end'?next-1:layer.scope.end-1}`;};grip.onpointerup=run(async ev=>{ev.stopPropagation();grip.onpointermove=null;await command('layer.scope',{id:layer.id,start:edge==='start'?next:layer.scope.start,end:edge==='end'?next:layer.scope.end});});};}
      tracks.append(item);
    }
    tracks.style.height=(row*30)+'px';
  }
  function renderVisible(){
    const s=getState();if(!s)return;const first=Math.max(origin,Math.floor(scroll.scrollLeft/scale)+origin),last=Math.min(origin+length,Math.ceil((scroll.scrollLeft+scroll.clientWidth)/scale)+origin);
    const step=scale>=65?1:Math.max(1,Math.ceil(76/scale));
    const visible=`${token}:${first}:${last}:${step}`;if(visible===visibleSignature){drain();return;}visibleSignature=visible;queue=[];
    for(const seg of segments()){
      const c=[...root.children].find(el=>el.dataset.clipId===seg.clip.id);if(!c)continue;const cells=c.querySelector('.frame-cells');cells.replaceChildren();const left=Math.max(origin,seg.start);
      const begin=Math.max(0,Math.floor((first-seg.start)/step)*step),end=Math.min(seg.clip.frames,last-seg.start+step);
      for(let local=begin;local<end;local+=step){if(local+seg.start<origin)continue;
        const cell=document.createElement('div');cell.className='frame-cell';cell.dataset.frame=local;cell.style.left=((seg.start+local-left)*scale)+'px';cell.style.width=(Math.min(step,seg.clip.frames-local)*scale)+'px';const img=document.createElement('img');img.alt='';const label=document.createElement('span');label.textContent=scale>=65?String(local):`${(local/(s.project.fps.num/s.project.fps.den)).toFixed(1)} s`;cell.append(img,label);cells.append(cell);
        const key=`${revision}:${seg.clip.id}:${seg.clip.source_in}:${local}`;
        if(images.has(key)){img.src=images.get(key);}else if(!isPlaying()){queue.push({key,img,seg,local,global:seg.start+local,token});}
      }
    }
    ruler.replaceChildren();const rulerStep=Number(ruler.dataset.step)||1,F=Number(ruler.dataset.fps)||25;
    for(let frame=Math.floor(first/rulerStep)*rulerStep;frame<last+rulerStep;frame+=rulerStep){const b=document.createElement('button');b.className='ruler-tick';b.style.left=((frame-origin)*scale)+'px';b.textContent=scale>=65?String(frame):`${(frame/F).toFixed(1)}s`;b.onclick=run(()=>command('seek',{frame:Math.min(s.total_frames-1,Math.max(0,frame))}));ruler.append(b);}
    drain();
  }
  async function decoder(asset){
    if(decoders.has(asset.id))return decoders.get(asset.id);
    let src;if(asset.kind==='blank'){src=document.createElement('canvas');src.width=asset.width;src.height=asset.height;}
    else{const blob=await getMedia(asset.id);if(!blob)throw new Error('Saknat original');if(asset.kind==='image')src=await createImageBitmap(blob);else{src=document.createElement('video');src.muted=true;src.preload='auto';src.src=URL.createObjectURL(blob);await event(src,'loadeddata');}}
    decoders.set(asset.id,src);while(decoders.size>2){const [id,value]=decoders.entries().next().value;dispose(value);decoders.delete(id);}return src;
  }
  function dispose(src){if(src instanceof HTMLVideoElement){const url=src.src;src.removeAttribute('src');src.load();URL.revokeObjectURL(url);}else src.close?.();}
  async function drain(){
    if(generating)return;generating=true;
    try{while(queue.length&&!isPlaying()){
      const job=queue.shift();if(job.token!==token||!job.img.isConnected)continue;
      try{const src=await decoder(job.seg.asset),s=getState(),F=s.project.fps.num/s.project.fps.den;
        if(src instanceof HTMLVideoElement)await seek(src,(job.seg.clip.source_in+job.local+.5)/F);
        const canvas=document.createElement('canvas');canvas.width=112;canvas.height=63;const cx=canvas.getContext('2d',{willReadFrequently:true});const ratio=Math.min(112/job.seg.asset.width,63/job.seg.asset.height),w=job.seg.asset.width*ratio,h=job.seg.asset.height*ratio;cx.drawImage(src,(112-w)/2,(63-h)/2,w,h);
        const pixels=cx.getImageData(0,0,112,63).data;const result=await rpc('render',{width:112,height:63,frame:job.global,pixels:pixels.buffer},[pixels.buffer]);
        if(job.token!==token)continue;cx.putImageData(new ImageData(new Uint8ClampedArray(result),112,63),0,0);const url=canvas.toDataURL('image/jpeg',.75);images.set(job.key,url);while(images.size>128)images.delete(images.keys().next().value);if(job.img.isConnected)job.img.src=url;
      }catch{/* Missing/unsupported media stays visible as an empty thumbnail; the canvas reports relink errors. */}
    }}finally{generating=false;}
  }
  scroll.addEventListener('scroll',()=>{requestAnimationFrame(renderVisible);});
  $('timeline-zoom').oninput=()=>{signature='';follow=true;refresh();};
  $('timeline-fit').onclick=()=>{$('timeline-zoom').value=0;signature='';scroll.scrollLeft=0;refresh();};
  $('timeline-frame').onclick=()=>{$('timeline-zoom').value=100;signature='';follow=true;refresh();};
  $('timeline-detail').onclick=()=>{detail=!detail;$('timeline-detail').setAttribute('aria-pressed',String(detail));signature='';refresh();};
  new ResizeObserver(()=>refresh()).observe(scroll);
  return {refresh,invalidate(){revision++;images.clear();signature='';},reset(){token++;queue=[];images.clear();for(const v of decoders.values())dispose(v);decoders.clear();anchor=null;signature='';},follow(){follow=true;},inspect:()=>({scale,origin,thumbnails:images.size,decoders:decoders.size})};
}
