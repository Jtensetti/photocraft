const ready = new Promise((resolve,reject) => {
  const r = indexedDB.open('creative-studio-v1',1);
  r.onupgradeneeded = () => { r.result.createObjectStore('media'); r.result.createObjectStore('projects', {keyPath:'id'}); };
  r.onsuccess = () => resolve(r.result);
  r.onerror = () => reject(new Error('Lokal lagring kan inte öppnas. Kontrollera webbläsarens lagringsinställningar.'));
});
export async function dbRequest(store, mode, action) {
  const db = await ready;
  return new Promise((resolve,reject) => {
    const tx = db.transaction(store,mode); const r = action(tx.objectStore(store)); let value;
    r.onsuccess = () => { value = r.result; };
    tx.oncomplete = () => resolve(value);
    tx.onerror = tx.onabort = () => reject(new Error(tx.error?.name === 'QuotaExceededError' ? 'Lagringsutrymmet är fullt. Frigör utrymme eller ta en säkerhetskopia.' : 'Projektet kunde inte sparas lokalt.'));
  });
}
export const putMedia = (id,blob) => dbRequest('media','readwrite',s=>s.put(blob,id));
export const getMedia = id => dbRequest('media','readonly',s=>s.get(id));
export const putProject = p => dbRequest('projects','readwrite',s=>s.put(p));
export const projects = () => dbRequest('projects','readonly',s=>s.getAll());
const MAGIC = 'CSTUDIO1';
export async function backup(json) {
  const p = JSON.parse(json).project;
  const blobs = [];
  for (const a of p.assets) {
    const blob = await getMedia(a.id);
    if (!blob && a.kind !== 'blank') throw new Error(`Originalet ${a.name} saknas. Återlänka innan säkerhetskopiering.`);
    if (blob) blobs.push({id:a.id,blob});
  }
  const header = new TextEncoder().encode(JSON.stringify({json,media:blobs.map(({id,blob})=>({id,size:blob.size,type:blob.type}))}));
  const prefix = new Uint8Array(12); prefix.set(new TextEncoder().encode(MAGIC)); new DataView(prefix.buffer).setUint32(8,header.length);
  // Blob parts keep disk-backed files disk-backed; no whole-video ArrayBuffer.
  return new Blob([prefix,header,...blobs.map(m=>m.blob)],{type:'application/octet-stream'});
}
export async function readBackup(file) {
  const prefix = await file.slice(0,12).arrayBuffer();
  if (new TextDecoder().decode(prefix.slice(0,8)) !== MAGIC) throw new Error('Ogiltig säkerhetskopia');
  const size = new DataView(prefix).getUint32(8);
  if (size > 32_000_000 || size + 12 > file.size) throw new Error('Ogiltigt projekthuvud');
  const header = JSON.parse(await file.slice(12,12+size).text());
  let offset = 12+size;
  if (!Array.isArray(header.media) || header.media.length > 500) throw new Error('Ogiltig medialista');
  const parts = [];
  for(const m of header.media) {
    if(typeof m.id!=='string' || !Number.isSafeInteger(m.size) || m.size<0 || offset+m.size>file.size) throw new Error('Ofullständig säkerhetskopia');
    parts.push({id:m.id,blob:file.slice(offset,offset+m.size,m.type)}); offset+=m.size;
  }
  if(offset !== file.size) throw new Error('Säkerhetskopian har oväntade data');
  return {json:header.json,parts};
}
