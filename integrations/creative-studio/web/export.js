import {Muxer,ArrayBufferTarget} from 'webm-muxer';
import {getMedia} from './storage.js';
import {mp4Info} from './media.js';

// Integer frame -> microseconds, rounded once, without accumulating clock drift.
const timestamp=(frame,rate)=>Number((BigInt(frame)*1_000_000n*BigInt(rate.den)+BigInt(Math.floor(rate.num/2)))/BigInt(rate.num));
const fail=message=>{throw new Error(message);};
function decoderConfig(track,info){
  const entry=track.mdia.minf.stbl.stsd.entries[0],rate=info.audio.sample_rate,channels=info.audio.channel_count;
  if(rate!==48000||channels>2)fail('Ljudexport stöder för närvarande 48 kHz med en eller två kanaler.');
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
export async function exportTimeline({state,framePixels,cancelled,progress}){
  if(!window.VideoEncoder)fail('Videoexport kräver en webbläsare med WebCodecs, till exempel aktuell Chrome eller Edge.');
  const p=state.project,F=p.fps.num/p.fps.den;
  // Bound encoded output memory. Original media is always read from Blob slices.
  if(state.total_frames/F>120)fail('Första versionen exporterar upp till två minuter. Längre export med direktlagring är nästa steg.');
  const indexes=new Map();let hasAudio=false;
  for(const c of p.clips){
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
  const videoConfig={codec:'vp09.00.10.08',width:p.width,height:p.height,bitrate:4_000_000,framerate:F,latencyMode:'quality'};
  if(!(await VideoEncoder.isConfigSupported(videoConfig)).supported)fail('Webbläsaren kan inte exportera VP9 i den här upplösningen');
  const target=new ArrayBufferTarget(),muxer=new Muxer({target,video:{codec:'V_VP9',width:p.width,height:p.height,frameRate:F},...(hasAudio?{audio:{codec:'A_OPUS',numberOfChannels:2,sampleRate:48000}}:{}),firstTimestampBehavior:'offset'});
  let failure;const onError=e=>{failure=e;};const check=()=>{if(failure)throw failure;if(cancelled())fail('Exporten avbröts');};
  const canvas=document.createElement('canvas');canvas.width=p.width;canvas.height=p.height;const cx=canvas.getContext('2d');
  const video=new VideoEncoder({output:(chunk,meta)=>{try{muxer.addVideoChunk(chunk,meta);}catch(e){onError(e);}},error:onError});
  let audio;
  try{
    if(hasAudio){
      const config={codec:'opus',sampleRate:48000,numberOfChannels:2,bitrate:128000};
      if(!(await AudioEncoder.isConfigSupported(config)).supported)fail('Webbläsaren kan inte exportera Opus-ljud');
      audio=new AudioEncoder({output:(chunk,meta)=>{try{muxer.addAudioChunk(chunk,meta);}catch(e){onError(e);}},error:onError});audio.configure(config);
      await encodeAudio(p,indexes,audio,check);await audio.flush();check();
    }
    video.configure(videoConfig);
    for(let frame=0;frame<state.total_frames;frame++){
      check();const pixels=await framePixels(frame,p.width,p.height);cx.putImageData(new ImageData(pixels,p.width,p.height),0,0);
      const image=new VideoFrame(canvas,{timestamp:timestamp(frame,p.fps),duration:timestamp(frame+1,p.fps)-timestamp(frame,p.fps)});
      try{video.encode(image,{keyFrame:frame%Math.max(1,Math.round(F*2))===0});}finally{image.close();}
      if(video.encodeQueueSize>5)await video.flush();progress((frame+1)/state.total_frames);
    }
    await video.flush();check();muxer.finalize();return new Blob([target.buffer],{type:'video/webm'});
  }finally{if(video.state!=='closed')video.close();if(audio&&audio.state!=='closed')audio.close();}
}
async function encodeAudio(p,indexes,encoder,check){
  const rate=48000,F=p.fps.num/p.fps.den;let written=0,offset=0;
  function push(data,n){if(!n)return;const pcm=new AudioData({format:'f32-planar',sampleRate:rate,numberOfChannels:2,numberOfFrames:n,timestamp:Math.round(written*1e6/rate),data});try{encoder.encode(pcm);}finally{pcm.close();}written+=n;}
  function silence(until){while(written<until){const n=Math.min(1024,until-written);push(new Float32Array(n*2),n);}}
  for(const clip of p.clips){
    check();const clipStart=Math.round(offset/F*rate),clipEnd=Math.round((offset+clip.frames)/F*rate);silence(clipStart);
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
          for(let ch=0;ch<2;ch++){const plane=new Float32Array(pcm.numberOfFrames);pcm.copyTo(plane,{planeIndex:Math.min(ch,pcm.numberOfChannels-1),format:'f32-planar'});out.set(plane.subarray(first,end),ch*n);}
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
          }
        }
        await decoder.flush();if(error)throw error;
      }finally{if(decoder.state!=='closed')decoder.close();}
    }
    silence(clipEnd);offset+=clip.frames;
  }
}
