import {createFile} from 'mp4box';
import {getMedia} from './storage.js';

const cache = new Map();
function dispose(value) {
  if(value instanceof HTMLVideoElement){value.pause();URL.revokeObjectURL(value.src);value.removeAttribute('src');value.load();}
  else if(value.close)value.close();
}
function remember(id,value){
  cache.set(id,value);
  // Retain at most four decoded sources. Imported originals remain in IndexedDB.
  for(const [key,old]of cache){if(cache.size<=4)break;if(key!==id&&!(old instanceof HTMLVideoElement&&!old.paused)){cache.delete(key);dispose(old);}}
  return value;
}
export async function mp4Info(file) {
  const parser = createFile(false); let info; let error;
  parser.onReady = v=>{info=v;}; parser.onError = e=>{error=e;};
  let offset = 0;
  while(offset < file.size && !info && !error) {
    const b = await file.slice(offset, Math.min(offset+1024*1024,file.size)).arrayBuffer();
    b.fileStart=offset; const next = parser.appendBuffer(b);
    offset = Number.isSafeInteger(next) && next>offset ? next : offset+b.byteLength;
  }
  if(error) throw new Error('MP4-filen kunde inte läsas: '+error);
  const video=info?.videoTracks?.[0];
  if(!video) return null;
  const samples=parser.getTrackById(video.id)?.samples || [];
  const durations=new Set(samples.map(s=>s.duration));
  return {fps:video.nb_samples/(video.duration/video.timescale),variable:durations.size>1,info,parser};
}
export async function metadata(file) {
  if(file.type.startsWith('image/')) {
    const bitmap=await createImageBitmap(file,{imageOrientation:'from-image'});
    const result={kind:'image',width:bitmap.width,height:bitmap.height,duration:1,fps:null}; bitmap.close(); return result;
  }
  const url=URL.createObjectURL(file); const video=document.createElement('video'); video.preload='metadata'; video.src=url;
  try {
    await event(video,'loadedmetadata');
    if(!Number.isFinite(video.duration) || video.duration<=0) throw new Error('Videon saknar en giltig varaktighet');
    const mp4=file.type.includes('mp4') || /\.mp4$/i.test(file.name) ? await mp4Info(file) : null;
    return {kind:'video',width:video.videoWidth,height:video.videoHeight,duration:video.duration,fps:mp4?.fps??null,variable:mp4?.variable??false};
  } finally { video.removeAttribute('src');video.load();URL.revokeObjectURL(url); }
}
export function event(target,type,timeout=15000) {
  return new Promise((resolve,reject)=>{
    const finish = error=>{clearTimeout(timer);target.removeEventListener(type,ok);target.removeEventListener('error',bad);error?reject(error):resolve();};
    const ok=()=>finish();const bad=()=>finish(new Error('Mediet kan inte avkodas av den här webbläsaren'));
    const timer=setTimeout(()=>finish(new Error('Mediet tog för lång tid att avkoda')),timeout);
    target.addEventListener(type,ok,{once:true});target.addEventListener('error',bad,{once:true});
  });
}
export async function source(asset) {
  if(cache.has(asset.id)){const value=cache.get(asset.id);cache.delete(asset.id);cache.set(asset.id,value);return value;}
  if(asset.kind==='blank') {const c=document.createElement('canvas');c.width=asset.width;c.height=asset.height;return remember(asset.id,c);}
  const blob=await getMedia(asset.id);
  if(!blob) throw new Error(`Originalet ”${asset.name}” saknas. Klicka på mediet för att återlänka filen.`);
  let value;
  if(asset.kind==='image') value=await createImageBitmap(blob,{imageOrientation:'from-image'});
  else {
    value=document.createElement('video');value.preload='auto';value.playsInline=true;value.src=URL.createObjectURL(blob);value.dataset.assetId=asset.id;
    try{await event(value,'loadeddata');}catch(e){dispose(value);throw e;}
  }
  return remember(asset.id,value);
}
export async function seek(video, seconds) {
  const t=Math.max(0,Math.min(seconds,video.duration-0.001));
  if(Math.abs(video.currentTime-t)<0.0001 && video.readyState>=2) return;
  const promise=event(video,'seeked');video.currentTime=t;await promise;
}
export function pauseAll() {for(const v of cache.values()) if(v instanceof HTMLVideoElement) v.pause();}
export function clearSources() {for(const value of cache.values())dispose(value);cache.clear();}
