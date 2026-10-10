// Presentation only. Panel placement is never a project operation or undo entry.
const KEY='creative-studio-workspaces-v2';
const names={media:'Projektmedia',light:'LightCraft',photo:'PhotoCraft',film:'FilmCraft',vector:'VectorCraft',design:'DesignCraft'};
const defaults=()=>({panels:{media:{dock:'left',open:true},photo:{dock:'right',open:true},light:{dock:'right',open:false},film:{dock:'right',open:false},vector:{dock:'right',open:false},design:{dock:'right',open:false}},tabs:{left:false,right:true},active:{left:'media',right:'photo'},widths:{left:218,right:310}});
export function createWorkspace({status,getPresentation,applyPresentation}){
  const left=document.getElementById('left-panel'),right=document.getElementById('right-panel');
  const saved=()=>{try{return JSON.parse(localStorage.getItem(KEY))||{layouts:{}};}catch{return {layouts:{}};}};
  let view=defaults(),lastMode='photo';
  const nodes={media:document.getElementById('media-panel'),photo:document.getElementById('photo-panel'),light:document.getElementById('light-panel'),film:document.getElementById('film-panel'),vector:document.getElementById('vector-panel'),design:document.getElementById('design-panel')};
  const cards={};
  for(const [id,node] of Object.entries(nodes)){
    const card=document.createElement('section');card.className='dock-card';card.dataset.panel=id;
    const head=document.createElement('header');head.className='dock-heading';head.draggable=true;
    const title=document.createElement('strong');title.textContent=names[id];
    const actions=document.createElement('div');actions.className='dock-actions';
    const dock=document.createElement('select');dock.setAttribute('aria-label',`Placering för ${names[id]}`);
    for(const [value,label] of [['left','Vänster'],['right','Höger'],['float','Frikoppla']]){const o=new Option(label,value);dock.add(o);}
    dock.onchange=()=>move(id,dock.value);
    const min=document.createElement('button');min.textContent='−';min.title='Minimera';min.setAttribute('aria-label',`Minimera ${names[id]}`);min.onclick=()=>{view.panels[id].min=!view.panels[id].min;render();};
    const close=document.createElement('button');close.textContent='×';close.title='Dölj';close.setAttribute('aria-label',`Dölj ${names[id]}`);close.onclick=()=>{view.panels[id].open=false;render();};
    actions.append(dock,min,close);head.append(title,actions);head.ondragstart=e=>{e.dataTransfer.setData('text/studio-panel',id);e.dataTransfer.effectAllowed='move';};
    card.append(head,node);nodes[id].hidden=false;cards[id]=card;
    head.onpointerdown=e=>{if(view.panels[id].dock!=='float'||e.target.closest('button,select'))return;const r=card.getBoundingClientRect(),origin={x:e.clientX,y:e.clientY,left:r.left,top:r.top};head.setPointerCapture(e.pointerId);head.onpointermove=ev=>{const pos=view.panels[id];pos.x=Math.max(0,Math.min(innerWidth-240,origin.left+ev.clientX-origin.x));pos.y=Math.max(100,Math.min(innerHeight-100,origin.top+ev.clientY-origin.y));card.style.left=pos.x+'px';card.style.top=pos.y+'px';};head.onpointerup=()=>{head.onpointermove=null;persist();};};
  }
  function persist(){const data=saved();data.current=view;try{localStorage.setItem(KEY,JSON.stringify(data));}catch{status('Layouten kunde inte sparas på enheten',true);}}
  function move(id,dock){view.panels[id].dock=dock;view.panels[id].open=true;view.active[dock]=id;render();}
  function valid(v){return v?.panels&&['media','photo','light','film'].every(id=>v.panels[id]&&['left','right','float'].includes(v.panels[id].dock));}
  const previous=saved().current;if(valid(previous)){view={...defaults(),...previous,panels:{...defaults().panels,...previous.panels}};if(!previous.panels.vector&&!previous.panels.design)view.tabs={...view.tabs,right:true};}
  for(const [dock,root] of [['left',left],['right',right]]){
    root.replaceChildren();const header=document.createElement('div');header.className='dock-bar';
    const add=document.createElement('select');add.setAttribute('aria-label',`Öppna verktyg på ${dock==='left'?'vänster':'höger'} sida`);add.add(new Option('+ Verktyg',''));
    for(const [id,name] of Object.entries(names))add.add(new Option(name,id));add.onchange=()=>{if(add.value)move(add.value,dock);add.value='';};
    const stack=document.createElement('button');stack.textContent='≡';stack.title='Växla flikar / staplade paneler';stack.setAttribute('aria-label',`Växla panelvisning ${dock}`);stack.onclick=()=>{view.tabs[dock]=!view.tabs[dock];render();};
    header.append(add,stack);const tabs=document.createElement('nav');tabs.className='dock-tabs';tabs.setAttribute('aria-label','Verktygspaneler');
    const content=document.createElement('div');content.className='dock-content';root.append(header,tabs,content);
    root.ondragover=e=>{if(e.dataTransfer.types.includes('text/studio-panel')){e.preventDefault();root.classList.add('drop-target');}};
    root.ondragleave=()=>root.classList.remove('drop-target');root.ondrop=e=>{e.preventDefault();root.classList.remove('drop-target');const id=e.dataTransfer.getData('text/studio-panel');if(names[id])move(id,dock);};
    const grip=document.createElement('div');grip.className='dock-resize';grip.setAttribute('role','separator');grip.setAttribute('aria-label','Ändra panelbredd');root.append(grip);
    grip.onpointerdown=e=>{const start=e.clientX,width=root.clientWidth;grip.setPointerCapture(e.pointerId);grip.onpointermove=ev=>{view.widths[dock]=Math.max(190,Math.min(480,width+(ev.clientX-start)*(dock==='left'?1:-1)));root.style.width=view.widths[dock]+'px';};grip.onpointerup=()=>{grip.onpointermove=null;persist();};};
  }
  const floats=document.createElement('div');floats.id='floating-panels';document.getElementById('app').append(floats);
  function render(){
    for(const [dock,root] of [['left',left],['right',right]]){
      root.style.width=Math.max(190,Math.min(480,view.widths[dock]||260))+'px';const tabs=root.querySelector('.dock-tabs'),content=root.querySelector('.dock-content');tabs.replaceChildren();
      for(const id of Object.keys(names))if(view.panels[id].dock===dock)content.append(cards[id]);
      const ids=Object.keys(names).filter(id=>view.panels[id].open&&view.panels[id].dock===dock);
      if(!ids.includes(view.active[dock]))view.active[dock]=ids[0];
      tabs.hidden=!view.tabs[dock];
      for(const id of ids){const b=document.createElement('button');b.textContent=names[id];b.className=view.active[dock]===id?'active':'';b.onclick=()=>{view.active[dock]=id;render();};tabs.append(b);content.append(cards[id]);cards[id].hidden=view.tabs[dock]&&view.active[dock]!==id;}
      root.dataset.empty=String(!ids.length);
    }
    for(const id of Object.keys(names)){
      const p=view.panels[id],c=cards[id];c.querySelector('select').value=p.dock;nodes[id].hidden=!!p.min;
      if(p.dock==='float')floats.append(c);
      if(!p.open){c.hidden=true;continue;}
      c.classList.toggle('floating',p.dock==='float');
      if(p.dock==='float'){floats.append(c);c.hidden=false;c.style.left=Math.max(0,Math.min(innerWidth-Math.min(300,innerWidth*.85),p.x??innerWidth-620))+'px';c.style.top=Math.max(100,Math.min(innerHeight-120,p.y??155))+'px';}
      else{c.style.left='';c.style.top='';}
    }
    persist();
  }
  render();
  window.addEventListener('resize',()=>render());
  const select=document.getElementById('layout-list');
  function refresh(){select.replaceChildren(new Option('Mina arbetsytor',''));for(const name of Object.keys(saved().layouts||{}))select.add(new Option(name,name));}
  refresh();select.onchange=()=>{const l=saved().layouts?.[select.value];if(valid(l)){view={...defaults(),...structuredClone(l),panels:{...defaults().panels,...structuredClone(l.panels)}};render();if(view.presentation)applyPresentation(view.presentation).catch(error=>status(error.message,true));status('Arbetsytan är återställd');}};
  return {
    sync(mode){if(mode!==lastMode){this.open(mode);lastMode=mode;}const presentation=getPresentation();if(JSON.stringify(view.presentation)!==JSON.stringify(presentation)){view.presentation=presentation;persist();}},
    open(id){view.panels[id].open=true;view.panels[id].min=false;if(view.panels[id].dock!=='float')view.active[view.panels[id].dock]=id;render();},
    has(id){return !!view.panels[id].open;},
    move,
    save(name){view.presentation=getPresentation();const data=saved();data.layouts??={};data.layouts[name]=structuredClone(view);localStorage.setItem(KEY,JSON.stringify(data));refresh();status(`Arbetsytan ”${name}” är sparad`);},
    reset(){view=defaults();render();},
    inspect:()=>structuredClone(view),
  };
}
