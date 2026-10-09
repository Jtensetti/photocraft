let studio;
const ready = (async()=>{
  const url = new URL(/* @vite-ignore */ '../pkg/creative_studio.js',import.meta.url);
  const {default:init,Studio}=await import(/* @vite-ignore */ url.href);
  await init(); studio = new Studio();
})();
self.onmessage = async ({data: m}) => {
  try {
    await ready;
    let result;
    if (m.op === 'command') result = JSON.parse(studio.execute(m.command, JSON.stringify(m.params)));
    else if (m.op === 'inspect') result = JSON.parse(studio.inspect());
    else if (m.op === 'save') result = studio.save();
    else if (m.op === 'open') result = JSON.parse(studio.open(m.text));
    else if (m.op === 'render') {
      result = studio.render(m.width, m.height, m.frame, new Uint8Array(m.pixels));
      self.postMessage({id:m.id,result:result.buffer}, [result.buffer]); return;
    } else throw new Error('Okänd begäran');
    self.postMessage({id:m.id,result});
  } catch(error) { self.postMessage({id:m.id,error:String(error)}); }
};
