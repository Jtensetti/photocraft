// Checked, bounded engine assets. Originals are never sent to the server.
export async function loadEngine(base,progress=()=>{},build=''){
 const asset=name=>{const url=new URL(name,base);if(build)url.searchParams.set('build',build);return url;};
 const response=await fetch(asset('engine.json'),{cache:'no-cache'});
 if(response.status===404)return asset('creative_studio_bg.wasm');
 if(!response.ok)throw new Error('Redigeringsmotorn kunde inte hämtas');
 const manifest=await response.json();
 if(!Number.isSafeInteger(manifest.bytes)||manifest.bytes<8||manifest.bytes>8*20*1024*1024||!Array.isArray(manifest.chunks)||manifest.chunks.length<1||manifest.chunks.length>8)throw new Error('Ogiltig motorversion');
 const output=new Uint8Array(manifest.bytes);let offset=0;
 for(const chunk of manifest.chunks){
   if(!/^engine-\d+\.wasmpart$/.test(chunk.name)||!Number.isSafeInteger(chunk.bytes)||chunk.bytes<1||chunk.bytes>20*1024*1024||offset+chunk.bytes>output.length)throw new Error('Ogiltig motordel');
   const url=asset(chunk.name);url.searchParams.set('hash',chunk.sha256);const response=await fetch(url);if(!response.ok)throw new Error('Redigeringsmotorn kunde inte hämtas');const data=await response.arrayBuffer();if(data.byteLength!==chunk.bytes)throw new Error('Motorn är ofullständig');
   const digest=Array.from(new Uint8Array(await crypto.subtle.digest('SHA-256',data)),n=>n.toString(16).padStart(2,'0')).join('');if(digest!==chunk.sha256)throw new Error('Motorns kontrollsumma stämmer inte');
   output.set(new Uint8Array(data),offset);offset+=data.byteLength;progress(offset/output.length);
 }
 if(offset!==output.length)throw new Error('Motorn är ofullständig');return output;
}
