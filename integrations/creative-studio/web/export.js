import {Muxer,ArrayBufferTarget,StreamTarget} from 'webm-muxer';
import {Muxer as Mp4Muxer,ArrayBufferTarget as Mp4ArrayBufferTarget,StreamTarget as Mp4StreamTarget} from 'mp4-muxer';
import {getMedia} from './storage.js';
import {durationOffset,durationBytes} from './webm-duration.js';
import {mp4Info} from './media.js';

// Integer frame -> microseconds, rounded once, without accumulating clock drift.
const timestamp=(frame,rate)=>Number((BigInt(frame)*1_000_000n*BigInt(rate.den)+BigInt(Math.floor(rate.num/2)))/BigInt(rate.num));
const fail=message=>{throw new Error(message);};
export function decoderConfig(track,info){
  const entry=track.mdia.minf.stbl.stsd.entries[0],rate=info.audio.sample_rate,channels=info.audio.channel_count;
  if(rate<8000||rate>192000||channels<1||channels>6)fail('Ljudets frekvens eller kanalantal stöds inte.');
  if(info.codec.toLowerCase()==='opus'){
    const ops=entry.dOps;if(!ops||ops.ChannelMappingFamily!==0)fail('Den här Opus-kanalmappningen stöds inte');
    const description=new Uint8Array(19);description.set(new TextEncoder().encode('OpusHead'));description[8]=1;description[9]=channels;new DataView(description.buffer).setUint32(12,48000,true);
    // Container edit lists apply pre-skip below; do not apply it twice in the decoder.
    return {codec:'opus',sampleRate:rate,numberOfChannels:channels,description};
  }
  if(info.codec.startsWith('mp4a.')){
    const description=entry.esds?.esd?.findDescriptor(5)?.data;
    if(!description)fail('AAC-filen saknar avkodningsinställningar');
    return {codec:info.codec,sampleRate:rate,numberOfChannels:channels,description};
  }
  fail('Ljudkodningen stöds inte för export: '+info.codec);
}
export async function exportTimeline({state,framePixels,cancelled,progress,sink,mixAudio,nativeVideo,format='webm'}){
  if(!['webm','mp4'].includes(format))fail('Okänt exportformat');
  const mp4=format==='mp4',p=state.project,F=p.fps.num/p.fps.den;
  if(!window.VideoEncoder&&!(mp4&&nativeVideo))fail('WebM-export kräver en webbläsare med WebCodecs, till exempel aktuell Chrome eller Edge.');
  // Bound encoded output memory. Original media is always read from Blob slices.
  if(!sink&&state.total_frames/F>120)fail('Välj ”Spara video direkt till fil” för filmer längre än två minuter.');
  const indexes=new Map();let hasAudio=false;
  for(const c of (mixAudio?[]:p.clips)){
    if(c.muted)continue;const a=p.assets.find(a=>a.id===c.asset_id);if(a.kind!=='video')continue;
    if(!indexes.has(a.id)){
      const blob=await getMedia(a.id);if(!blob)fail('Originalet saknas: '+a.name);
      if(!/\.mp4$/i.test(a.name)&&!blob.type.includes('mp4'))fail('Ljudexport kräver MP4 i första versionen. Stäng av klippets ljud för att exportera enbart bild.');
      const index=await mp4Info(blob);if(!index)fail('MP4-filen kunde inte indexeras');
      const audio=index.info.audioTracks[0];
      if(audio){if(!window.AudioDecoder||!window.AudioEncoder)fail('Webbläsaren saknar ljudexport');const track=index.parser.getTrackById(audio.id),config=decoderConfig(track,audio);const supported=await AudioDecoder.isConfigSupported(config);if(!supported.supported)fail('Webbläsaren kan inte avkoda klippets ljud');index.audio={info:audio,track,config};hasAudio=true;}
      index.blob=blob;indexes.set(a.id,index);
    }
  }
  if(mixAudio&&p.assets.some(a=>['video','audio'].includes(a.kind))||p.assets.some(a=>a.native_item?.kind?.Media?.info?.audio)||p.timeline?.audio_tracks.some(t=>t.items.some(i=>i.enabled&&p.assets.find(a=>a.native_item?.id===i.item)?.kind==='generator')))hasAudio=true;
  const videoConfig={codec:mp4?'avc1.640034':'vp09.00.10.08',width:p.width,height:p.height,bitrate:4_000_000,framerate:F,latencyMode:'quality',...(mp4?{avc:{format:'avc'}}:{})};
  const software=mp4&&nativeVideo&&(!window.VideoEncoder||!(await VideoEncoder.isConfigSupported(videoConfig)).supported);
  if(!software&&!(await VideoEncoder.isConfigSupported(videoConfig)).supported)fail('Webbläsaren kan inte exportera '+(mp4?'H.264':'VP9')+' i den här upplösningen');
  const writes=[];let durationPosition;
  if(hasAudio&&!window.AudioEncoder)fail('Webbläsaren saknar ljudexport');
  const target=mp4?(sink?new Mp4StreamTarget({chunked:true,chunkSize:1_048_576,onData:(data,position)=>writes.push({data:data.slice(),position})}):new Mp4ArrayBufferTarget()):(sink?new StreamTarget({chunked:true,chunkSize:1_048_576,onHeader:(data,position)=>{const offset=durationOffset(data);if(offset===null)throw new Error('WebM-varaktighet saknas');durationPosition=position+offset;},onData:(data,position)=>writes.push({data:data.slice(),position})}):new ArrayBufferTarget());
  async function drainWrites(){while(writes.length){const {data,position}=writes.shift();await sink.write({type:'write',position,data});}}
  let audioCodec='opus';
  if(mp4&&hasAudio&&(await AudioEncoder.isConfigSupported({codec:'mp4a.40.2',sampleRate:48000,numberOfChannels:2,bitrate:128000})).supported)audioCodec='mp4a.40.2';
  const muxer=mp4?new Mp4Muxer({target,video:{codec:'avc',width:p.width,height:p.height,frameRate:F},...(hasAudio?{audio:{codec:audioCodec==='opus'?'opus':'aac',numberOfChannels:2,sampleRate:48000}}:{}),fastStart:'fragmented',firstTimestampBehavior:'offset'}):new Muxer({target,video:{codec:'V_VP9',width:p.width,height:p.height,frameRate:F},...(hasAudio?{audio:{codec:'A_OPUS',numberOfChannels:2,sampleRate:48000}}:{}),firstTimestampBehavior:'offset'});
  let failure;const onError=e=>{failure=e;};const check=()=>{if(failure)throw failure;if(cancelled())fail('Exporten avbröts');};
  const canvas=document.createElement('canvas');canvas.width=p.width;canvas.height=p.height;const cx=canvas.getContext('2d');
  const video=software?null:new VideoEncoder({output:(chunk,meta)=>{try{muxer.addVideoChunk(chunk,meta);}catch(e){onError(e);}},error:onError});
  let audio;
  try{
    let softwareConfig;
    if(software){const description=new Uint8Array(await nativeVideo.begin());softwareConfig={decoderConfig:{codec:'avc1.'+[...description.subarray(1,4)].map(n=>n.toString(16).padStart(2,'0')).join(''),codedWidth:p.width,codedHeight:p.height,description}};}else video.configure(videoConfig);
    let nextFrame=0;
    async function renderUntil(samples){
      const end=Math.min(state.total_frames,Math.ceil(samples/48000*F));
      while(nextFrame<end){
        check();const frame=nextFrame++;const pixels=await framePixels(frame,p.width,p.height);
        if(software){const packet=new Uint8Array(await nativeVideo.frame(frame,pixels));muxer.addVideoChunkRaw(packet.subarray(1),packet[0]?'key':'delta',timestamp(frame,p.fps),timestamp(frame+1,p.fps)-timestamp(frame,p.fps),frame===0?softwareConfig:undefined);await drainWrites();}
        else{cx.putImageData(new ImageData(pixels,p.width,p.height),0,0);const image=new VideoFrame(canvas,{timestamp:timestamp(frame,p.fps),duration:timestamp(frame+1,p.fps)-timestamp(frame,p.fps)});try{video.encode(image,{keyFrame:frame%Math.max(1,Math.round(F*2))===0});}finally{image.close();}if(video.encodeQueueSize>5||frame%12===0){await video.flush();await drainWrites();}}
        progress(nextFrame/state.total_frames);
      }
    }
    if(hasAudio){
      const config={codec:audioCodec,sampleRate:48000,numberOfChannels:2,bitrate:128000};
      if(!(await AudioEncoder.isConfigSupported(config)).supported)fail('Webbläsaren kan inte exportera Opus-ljud');
      audio=new AudioEncoder({output:(chunk,meta)=>{try{muxer.addAudioChunk(chunk,meta);}catch(e){onError(e);}},error:onError});audio.configure(config);
      if(mixAudio){let written=0;const total=Math.round(state.total_frames/F*48000);while(written<total){check();const n=Math.min(4096,total-written),data=await mixAudio(written,n);const pcm=new AudioData({format:'f32-planar',sampleRate:48000,numberOfChannels:2,numberOfFrames:n,timestamp:Math.round(written*1e6/48000),data});try{audio.encode(pcm);}finally{pcm.close();}written+=n;if(audio.encodeQueueSize>12)await audio.flush();await renderUntil(written);}}else await encodeAudio(p,indexes,audio,check,renderUntil);await audio.flush();check();
    }
    await renderUntil(Math.ceil(state.total_frames/F*48000));
    if(video)await video.flush();check();muxer.finalize();await drainWrites();
    if(mp4)return sink?null:new Blob([target.buffer],{type:'video/mp4'});
    const duration=durationBytes(state.total_frames/F*1000);
    if(sink){if(durationPosition===undefined)fail('WebM-varaktighet saknas');await sink.write({type:'write',position:durationPosition,data:duration});return null;}
    const bytes=new Uint8Array(target.buffer),offset=durationOffset(bytes);if(offset===null)fail('WebM-varaktighet saknas');bytes.set(duration,offset);return new Blob([target.buffer],{type:'video/webm'});
  }finally{if(software)await nativeVideo.end();if(video&&video.state!=='closed')video.close();if(audio&&audio.state!=='closed')audio.close();}
}
async function encodeAudio(p,indexes,encoder,check,renderUntil){
  const rate=48000,F=p.fps.num/p.fps.den;let written=0,offset=0;
  function push(data,n){if(!n)return;const pcm=new AudioData({format:'f32-planar',sampleRate:rate,numberOfChannels:2,numberOfFrames:n,timestamp:Math.round(written*1e6/rate),data});try{encoder.encode(pcm);}finally{pcm.close();}written+=n;}
  function silence(until){if(until-written>rate*2)fail('Ljudspåret innehåller ett tidsmässigt glapp som inte stöds.');while(written<until){const n=Math.min(1024,until-written);push(new Float32Array(n*2),n);}}
  async function fillSilence(until){let count=0;while(written<until){check();const n=Math.min(1024,until-written);push(new Float32Array(n*2),n);if(++count%16===0){await encoder.flush();await renderUntil(written);}}}
  for(const clip of p.clips){
    check();const clipStart=Math.round(offset/F*rate),clipEnd=Math.round((offset+clip.frames)/F*rate);await fillSilence(clipStart);
    const index=indexes.get(clip.asset_id);
    if(index?.audio&&!clip.muted){
      const {track,info,config}=index.audio,edits=info.edits||[];
      if(edits.length>1||edits.some(e=>e.media_time<0||e.media_rate_integer!==1))fail('Ljudfilen har en redigeringslista som ännu inte stöds');
      const mediaOffset=(edits[0]?.media_time||0)/info.timescale,sourceIn=clip.source_in/F,sourceOut=(clip.source_in+clip.frames)/F;
      let error;
      const decoder=new AudioDecoder({error:e=>{error=e;},output:pcm=>{
        try{
          const begin=pcm.timestamp/1e6-mediaOffset;
          let first=Math.max(0,Math.ceil((sourceIn-begin)*rate-1e-5)),end=Math.min(pcm.numberOfFrames,Math.ceil((sourceOut-begin)*rate-1e-5));
          let start=clipStart+Math.round((begin+first/rate-sourceIn)*rate);
          if(start<written){first+=written-start;start=written;}
          end=Math.min(end,first+clipEnd-start);if(end<=first)return;
          silence(start);const n=end-first,out=new Float32Array(n*2);
          for(let ch=0;ch<2;ch++){const plane=new Float32Array(pcm.numberOfFrames);pcm.copyTo(plane,{planeIndex:Math.min(ch,pcm.numberOfChannels-1),format:'f32-planar'});out.set(plane.subarray(first,end).map(value=>value*(clip.volume??1)),ch*n);}
          push(out,n);
        }catch(e){error=e;}finally{pcm.close();}
      }});
      try{
        decoder.configure(config);let count=0;
        for(const sample of track.samples){
          check();const t=sample.cts/sample.timescale-mediaOffset,d=sample.duration/sample.timescale;
          if(t+d<sourceIn-.08)continue;if(t>=sourceOut)break;
          const bytes=await index.blob.slice(sample.offset,sample.offset+sample.size).arrayBuffer();
          decoder.decode(new EncodedAudioChunk({type:'key',timestamp:Math.round(sample.cts/sample.timescale*1e6),duration:Math.round(d*1e6),data:bytes}));
          if(++count%16===0){
            // flush() ends a codec segment and can add Opus padding. Drain the
            // queues without ending the segment, then flush once at the end.
            while(decoder.decodeQueueSize>8)await new Promise(r=>decoder.addEventListener('dequeue',r,{once:true}));
            while(encoder.encodeQueueSize>16)await new Promise(r=>encoder.addEventListener('dequeue',r,{once:true}));
            if(error)throw error;
            await renderUntil(written);
          }
        }
        await decoder.flush();if(error)throw error;
      }finally{if(decoder.state!=='closed')decoder.close();}
    }
    await fillSilence(clipEnd);await encoder.flush();await renderUntil(written);offset+=clip.frames;
  }
}
