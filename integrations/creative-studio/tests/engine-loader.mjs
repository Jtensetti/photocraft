import {readFileSync} from 'node:fs';
import {createHash} from 'node:crypto';
import assert from 'node:assert/strict';
import {loadEngine} from '../web/load-engine.js';
const actual=JSON.parse(readFileSync('public/pkg/engine.json','utf8')),originalFetch=globalThis.fetch;
let manifest=actual,corrupt=false,progress=[];
globalThis.fetch=async url=>{
 const name=new URL(url).pathname.split('/').at(-1);
 if(name==='engine.json')return Response.json(manifest);
 const data=readFileSync('public/pkg/'+name);if(corrupt)data[0]^=1;
 return new Response(data);
};
try{
 const bytes=await loadEngine(new URL('https://example.test/pkg/'),p=>progress.push(p),'test-build');
 assert.equal(bytes.byteLength,actual.bytes);assert.equal(progress.at(-1),1);assert.equal(progress.length,actual.chunks.length);
 const digest=createHash('sha256');for(const c of actual.chunks)digest.update(readFileSync('public/pkg/'+c.name));
 assert.equal(createHash('sha256').update(bytes).digest('hex'),digest.digest('hex'));
 manifest={...actual,bytes:8*20*1024*1024+1};await assert.rejects(()=>loadEngine(new URL('https://example.test/pkg/')),/Ogiltig motorversion/);
 manifest={...actual,chunks:Array(9).fill(actual.chunks[0])};await assert.rejects(()=>loadEngine(new URL('https://example.test/pkg/')),/Ogiltig motorversion/);
 manifest={...actual,chunks:[{...actual.chunks[0],name:'../other.wasm'}]};await assert.rejects(()=>loadEngine(new URL('https://example.test/pkg/')),/Ogiltig motordel/);
 manifest=actual;corrupt=true;await assert.rejects(()=>loadEngine(new URL('https://example.test/pkg/')),/kontrollsumma/);
 console.log(JSON.stringify({result:'PASS',engine_bytes:actual.bytes,verified_chunks:actual.chunks.length,size_guard:true,path_guard:true,checksum_guard:true}));
}finally{globalThis.fetch=originalFetch;}
