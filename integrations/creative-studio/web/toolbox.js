import {icon} from './icons.js';

// Every entry is a canvas gesture or an existing command, never a decorative tool.
export const toolGroups = [
  ['Flytta','V',[['move','Flytta','move']]],
  ['Markera','M',[['select','Rektangulär markering','square-dashed'],['native:selectRect','Objektmarkering · rektangel','square-dashed'],['native:selectEllipse','Elliptisk markering','circle-dashed'],['native:lasso','Lasso','lasso-select'],['native:wand','Trollstav','wand']]],
  ['Beskär och maskera','R',[['native:crop','Beskärning','crop'],['mask','Rektangulär lagermask','square-dot']]],
  ['Retuschera','J',[['native:spotHeal','Punktlagning','bandage'],['native:heal','Läkningspensel','bandage'],['native:remove','Fotografisk lagning','bandage'],['native:redEye','Röda ögon','eye']]],
  ['Penslar','B',[['brush','Pensel','brush'],['native:pencil','Penna','pencil'],['native:mixer','Blandarpensel','palette'],['native:replaceColor','Färgersättning','pipette']]],
  ['Klona','S',[['native:clone','Klonstämpel','stamp']]],
  ['Suddgummi','E',[['erase','Suddgummi','eraser'],['native:backgroundErase','Bakgrundssudd','eraser-background'],['native:magicErase','Magiskt sudd','eraser-magic']]],
  ['Fyll och övertona','G',[['native:gradient','Övertoning','blend'],['native:bucket','Färgpyts','paint-bucket']]],
  ['Skärpa och smeta','U',[['native:blur','Oskärpa','droplet'],['native:sharpen','Skärpa','triangle'],['native:smudge','Smeta','hand']]],
  ['Ljus och mättnad','O',[['native:dodge','Skugga · ljusa upp','sun'],['native:burn','Efterbelys · mörka ned','flame'],['native:sponge','Svamp','lollipop']]],
  ['Former och banor','P',[['native:pen','Fri vektorform','pen-tool'],['native:rectangle','Rektangel','rectangle-horizontal'],['native:ellipse','Ellips','circle'],['native:line','Linje','slash']]],
  ['Text','T',[['text','Text','type']]],
  ['Lokala justeringar','K',[['native:maskBrush','Lokal justeringspensel','brush'],['native:maskLinear','Linjär mask','blend'],['native:maskRadial','Radiell mask','circle-dashed']]],
  ['Färg','I',[['eyedropper','Pipett','pipette']]],
  ['Navigera','H',[['hand','Handverktyg','hand'],['zoom','Zoom','zoom-in']]],
];
export const graphicTools = {
  vector:[['graphic:vector:select','Välj vektorobjekt','mouse-pointer-2'],['graphic:vector:shape.rectangle','Rektangel','rectangle-horizontal'],['graphic:vector:shape.ellipse','Ellips','circle'],['graphic:vector:shape.polygon','Polygon','pentagon'],['graphic:vector:shape.star','Stjärna','sparkles'],['graphic:vector:shape.line','Linje','slash'],['graphic:vector:path.create','Bezierbana','pen-tool'],['graphic:vector:text.create','Vektortext','type']],
  design:[['graphic:design:select','Välj layoutobjekt','mouse-pointer-2'],['graphic:design:frame.rectangle','Rektangulär ram','square'],['graphic:design:frame.ellipse','Elliptisk ram','circle'],['graphic:design:frame.text','Textram','type'],['graphic:design:line.create','Linje','slash']],
};

export function createToolbox({selectTool,getState,workspace,status}) {
  const rail=document.querySelector('.tool-rail'), choices=new Map(), groupButtons=[];
  const popup=document.createElement('div');popup.className='tool-flyout';popup.hidden=true;popup.setAttribute('role','menu');document.body.append(popup);
  const split=document.getElementById('rail-split'),colors=rail.querySelector('.rail-colors');rail.replaceChildren();
  let selected='brush',mode='';
  const remembered=new Map(),initial={photo:'brush',light:'hand',film:'hand',vector:'graphic:vector:select',design:'graphic:design:select'};
  function choose(id){selectTool(id);popup.hidden=true;const engine=id.startsWith('graphic:')?id.split(':')[1]:id.startsWith('native:mask')||['native:crop','native:remove','native:redEye'].includes(id)?'light':'photo';if(!['hand','zoom','eyedropper'].includes(id))workspace.open(engine);}
  function button(id,label,name){const b=document.createElement('button');b.type='button';b.dataset.tool=id;b.innerHTML=icon(name);b.title=label;b.setAttribute('aria-label',label);b.onclick=()=>choose(b.dataset.tool);return b;}
  const photo=document.createElement('div');photo.className='tool-grid';
  for(const [label,key,items] of toolGroups){const box=document.createElement('div');box.className='tool-group';const b=button(...items[0]);b.dataset.group=label;b.title=label+' ('+key+')'+(items.length>1?' · högerklick för fler verktyg':'');box.append(b);photo.append(box);groupButtons.push({b,items});choices.set(key.toLowerCase(),items);
    if(items.length>1){const toggle=document.createElement('button');toggle.className='tool-group-more';toggle.type='button';toggle.textContent='▾';toggle.title='Visa '+label.toLowerCase();toggle.setAttribute('aria-label',toggle.title);toggle.setAttribute('aria-expanded','false');box.append(toggle);
      const show=e=>{e.preventDefault();const already=!popup.hidden&&popup.dataset.group===label;popup.replaceChildren();popup.dataset.group=label;const h=document.createElement('strong');h.textContent=label;popup.append(h);for(const item of items){const row=button(...item);row.className='tool-choice';const text=document.createElement('span');text.textContent=item[1];row.append(text);row.setAttribute('role','menuitemradio');row.setAttribute('aria-checked',item[0]===selected);popup.append(row);}popup.hidden=already;toggle.setAttribute('aria-expanded',String(!already));const r=box.getBoundingClientRect();popup.style.left=Math.min(innerWidth-265,r.right+6)+'px';popup.style.top=Math.max(4,Math.min(innerHeight-popup.offsetHeight-4,r.top))+'px';if(!already)popup.querySelector('button').focus();};toggle.onclick=show;b.oncontextmenu=show;
    }
  }
  const graphics=document.createElement('div');graphics.className='tool-grid graphic-rail';rail.append(photo,graphics,split,colors);
  const option=document.createElement('div');option.id='selection-options';option.innerHTML='<label>Markering <select id="native-selection-mode" aria-label="Markeringsläge"><option value="replace">Ny</option><option value="add">Lägg till</option><option value="subtract">Ta bort</option><option value="intersect">Överlappa</option></select></label><label>Ludd <input id="native-feather" type="number" min="0" max="1000" value="0"> px</label><label>Tolerans <input id="native-tolerance" type="number" min="0" max="255" value="32"></label>';
  document.getElementById('tools').insertBefore(option,document.getElementById('native-tools'));
  function syncTool(id){selected=id;for(const {b,items} of groupButtons){const item=items.find(x=>x[0]===id);if(item){b.dataset.tool=id;b.innerHTML=icon(item[2]);b.setAttribute('aria-label',item[1]);}b.classList.toggle('active',!!item);}for(const b of graphics.querySelectorAll('[data-tool]'))b.classList.toggle('active',b.dataset.tool===id);const selection=/select|lasso|wand/.test(id);option.hidden=!selection;for(const label of document.querySelectorAll('.brush-size,.brush-extra'))label.hidden=selection||/hand|zoom|eyedropper|crop|select|graphic:/.test(id);document.getElementById('tool-name').textContent=[...toolGroups.flatMap(g=>g[2]),...Object.values(graphicTools).flat()].find(t=>t[0]===id)?.[1]||document.getElementById('tool-name').textContent;}
  document.addEventListener('studio-tool-selected',e=>{syncTool(e.detail);if(mode)remembered.set(mode,e.detail);});
  document.addEventListener('pointerdown',e=>{if(!e.target.closest('.tool-flyout,.tool-group'))popup.hidden=true;});
  document.addEventListener('keydown',e=>{if(e.key==='Escape'){popup.hidden=true;return;}if(!popup.hidden&&['ArrowDown','ArrowUp'].includes(e.key)){e.preventDefault();const rows=[...popup.querySelectorAll('button')],i=rows.indexOf(document.activeElement);rows[(i+(e.key==='ArrowDown'?1:-1)+rows.length)%rows.length].focus();return;}if(e.ctrlKey||e.metaKey||e.altKey||e.target.closest('input,textarea,select,[contenteditable=true]')||document.querySelector('dialog[open]'))return;const items=choices.get(e.key.toLowerCase());if(!items||mode!=='photo'||(!e.shiftKey&&'vmbethz'.includes(e.key.toLowerCase())))return;e.preventDefault();const i=items.findIndex(x=>x[0]===selected);choose(items[e.shiftKey?(i+1)%items.length:0][0]);});
  return {sync(){const current=getState()?.project.workspace.mode;if(current!==mode){mode=current;document.getElementById('extended-tool').hidden=mode!=='photo';graphics.replaceChildren();photo.hidden=mode!=='photo';graphics.hidden=mode==='photo';rail.dataset.engine=mode;
      if(graphicTools[mode]){for(const item of graphicTools[mode])graphics.append(button(...item));}
      if(mode==='light'){for(const item of [['native:crop','Beskärning','crop'],['native:maskBrush','Justeringspensel','brush'],['native:maskLinear','Linjär mask','blend'],['native:maskRadial','Radiell mask','circle-dashed'],['native:remove','Laga','bandage'],['native:redEye','Röda ögon','eye'],['eyedropper','Pipett','pipette'],['hand','Navigera','hand'],['zoom','Zoom','zoom-in']])graphics.append(button(...item));}
      if(mode==='film'){for(const [label,name,action] of [['Välj klipp','mouse-pointer-2',()=>selectTool('hand')],['Dela vid spelhuvudet','scissors',()=>document.getElementById('split').click()],['Trimma klipp','crop',()=>workspace.open('film')],['Spår','layers',()=>{const b=document.getElementById('timeline-detail');if(b.getAttribute('aria-pressed')!=='true')b.click();workspace.open('film');}],['Effekter','sliders-horizontal',()=>document.getElementById('native-tools').click()],['Spela upp','play',()=>document.getElementById('play').click()]]){const b=document.createElement('button');b.type='button';b.innerHTML=icon(name);b.title=label;b.setAttribute('aria-label',label);b.onclick=action;graphics.append(b);}}
      selectTool(remembered.get(mode)||initial[mode]||'hand');
    }syncTool(selected);},choose};
}
