import {loadEngine} from './load-engine.js';
let studio;
globalThis.__studioBlobs=new Map();
const ready = (async()=>{
  const url = new URL(/* @vite-ignore */ '../pkg/creative_studio.js',import.meta.url);
  const build=new URL(import.meta.url).pathname;url.searchParams.set('build',build);
  const {default:init,Studio}=await import(/* @vite-ignore */ url.href);
  const binary=await loadEngine(new URL('.',url),progress=>self.postMessage({loading:Math.round(progress*100)}),build);
  await init({module_or_path:binary}); studio = new Studio();
})();
self.onmessage = async ({data: m}) => {
  try {
    await ready;
    globalThis.__studioPanic=null;
    let result;
    if(m.op === 'attach-media'){studio.forget_media(m.asset);globalThis.__studioBlobs.set(m.asset,m.blob);result=true;}
    else if(m.op === 'forget-media'){studio.forget_media(m.asset);globalThis.__studioBlobs.delete(m.asset);result=true;}
    else if(m.op === 'reset-media'){for(const id of globalThis.__studioBlobs.keys())studio.forget_media(id);globalThis.__studioBlobs.clear();result=true;}
    else if(m.op === 'probe-media'){studio.forget_media(m.asset);globalThis.__studioBlobs.set(m.asset,m.blob);result=JSON.parse(studio.probe_media(m.asset,m.name,BigInt(m.bytes)));}
    else if(m.op === 'decode-frame'){result=studio.decode_media_frame(m.asset,m.seconds,m.scale);self.postMessage({id:m.id,result:result.buffer},[result.buffer]);return;}
    else if(m.op === 'decode-audio'){result=studio.decode_media_audio(m.asset,BigInt(m.start),m.frames,m.rate);self.postMessage({id:m.id,result:result.buffer},[result.buffer]);return;}
    else if(m.op === 'encode-video-begin'){result=studio.encode_video_begin();self.postMessage({id:m.id,result:result.buffer},[result.buffer]);return;}
    else if(m.op === 'encode-video-frame'){result=studio.encode_video_frame(m.frame,new Uint8Array(m.pixels));self.postMessage({id:m.id,result:result.buffer},[result.buffer]);return;}
    else if(m.op === 'encode-video-end'){studio.encode_video_end();result=true;}
    else if (m.op === 'command') result = JSON.parse(studio.execute(m.command, JSON.stringify(m.params)));
    else if (m.op === 'inspect') result = JSON.parse(studio.inspect());
    else if (m.op === 'save') result = studio.save();
    else if (m.op === 'open') result = JSON.parse(studio.open(m.text));
    else if(m.op === 'catalog') result=JSON.parse(studio.native_catalog());
    else if(m.op === 'native-graphics') result=JSON.parse(studio.execute_graphics(m.engine,m.command,JSON.stringify({params:m.params,scope:m.scope})));
    else if(m.op === 'graphics-view') result=JSON.parse(studio.graphics_view(m.engine));
    else if(m.op === 'native-film') result=JSON.parse(studio.execute_film(m.command,JSON.stringify(m.params)));
    else if(m.op === 'audio-plan') result=JSON.parse(studio.audio_plan(BigInt(m.start),m.frames));
    else if(m.op === 'mix-audio'){result=studio.mix_audio(BigInt(m.start),m.frames,JSON.stringify(m.inputs),new Float32Array(m.samples));self.postMessage({id:m.id,result:result.buffer},[result.buffer]);return;}
    else if(m.op === 'source-frame'){result=m.clip?studio.asset_clip_frame(m.asset,m.clip,m.width,m.height,m.seconds):studio.asset_frame(m.asset,m.width,m.height,m.seconds);self.postMessage({id:m.id,result:result.buffer},[result.buffer]);return;}
    else if(m.op === 'frame-plan') result=JSON.parse(m.clip?studio.clip_plan(m.clip,m.frame):studio.frame_plan(m.frame));
    else if(m.op === 'native') result=JSON.parse(studio.execute_pixels(m.engine,m.command,JSON.stringify({params:m.params,scope:m.scope}),m.width,m.height,m.frame,new Uint8Array(m.pixels)));
    else if(m.op === 'canvas-coordinates') result=JSON.parse(studio.canvas_coordinates(m.kind,JSON.stringify(m.points),m.docWidth,m.docHeight));
    else if(m.op === 'native-view') result=JSON.parse(studio.native_view(m.width,m.height,m.frame,new Uint8Array(m.pixels)));
    else if(m.op === 'render-clip-scene'){result=studio.render_clip_sequence(m.clip,m.width,m.height,m.frame,JSON.stringify(m.inputs),new Uint8Array(m.pixels));self.postMessage({id:m.id,result:result.buffer},[result.buffer]);return;}
    else if(m.op === 'render-scene'){
      result=studio.render_sequence(m.width,m.height,m.frame,JSON.stringify(m.inputs),new Uint8Array(m.pixels));
      self.postMessage({id:m.id,result:result.buffer},[result.buffer]);return;
    }
    else if (m.op === 'render-target') {
      result=studio.render_target(m.clip,m.width,m.height,m.frame,new Uint8Array(m.pixels));
      self.postMessage({id:m.id,result:result.buffer},[result.buffer]);return;
    }
    else if (m.op === 'render') {
      result = studio.render(m.width, m.height, m.frame, new Uint8Array(m.pixels));
      self.postMessage({id:m.id,result:result.buffer}, [result.buffer]); return;
    } else throw new Error('Okänd begäran');
    self.postMessage({id:m.id,result});
  } catch(error) { if(globalThis.__studioPanic)console.error(error.stack);self.postMessage({id:m.id,error:globalThis.__studioPanic||String(error)}); }
};
