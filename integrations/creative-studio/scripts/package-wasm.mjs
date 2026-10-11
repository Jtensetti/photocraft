// Static hosts can serve bounded chunks; the browser assembles only the executable, never media.
import {readFileSync,writeFileSync,readdirSync,unlinkSync} from 'node:fs';
import {createHash} from 'node:crypto';
const root='public/pkg',file=root+'/creative_studio_bg.wasm',bytes=readFileSync(file),limit=20*1024*1024;
for(const name of readdirSync(root))if(/^engine-\d+\.wasmpart$/.test(name)||name==='engine.json')unlinkSync(root+'/'+name);
const chunks=[];
for(let at=0;at<bytes.length;at+=limit){const name='engine-'+chunks.length+'.wasmpart',part=bytes.subarray(at,Math.min(bytes.length,at+limit));writeFileSync(root+'/'+name,part);chunks.push({name,bytes:part.length,sha256:createHash('sha256').update(part).digest('hex')});}
writeFileSync(root+'/engine.json',JSON.stringify({bytes:bytes.length,chunks}));
unlinkSync(file);
console.log(`Packaged ${bytes.length} WASM bytes in ${chunks.length} bounded assets`);
