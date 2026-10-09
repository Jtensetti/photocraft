import {existsSync} from 'node:fs';
for(const name of ['creative_studio.js','creative_studio_bg.wasm']){
  if(!existsSync('public/pkg/'+name))throw new Error('WASM output is missing. Run npm run build:wasm before npm run build.');
}
