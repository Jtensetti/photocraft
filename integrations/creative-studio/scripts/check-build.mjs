import {existsSync} from 'node:fs';
if(!existsSync('public/pkg/creative_studio.js')||!['engine.json','creative_studio_bg.wasm'].some(n=>existsSync('public/pkg/'+n)))throw new Error('WASM output is missing. Run npm run build:wasm before npm run build.');
