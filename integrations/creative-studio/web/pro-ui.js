import {icon} from './icons.js';
export function createProUI({getState,command,selectTool,status,updateOverlay}){
  const $=id=>document.getElementById(id);let zoom='fit';
  for(const el of document.querySelectorAll('[data-icon]')){el.innerHTML=icon(el.dataset.icon);el.setAttribute('aria-label',el.title);}
  for(const [id,name] of [['undo','undo-2'],['redo','redo-2']]){$(id).innerHTML=icon(name);$(id).setAttribute('aria-label',$(id).title);}
  const run=fn=>async()=>{try{await fn();}catch(e){status(e.message,true);}};
  const menus={
    Arkiv:[['Nytt projekt','new','Ctrl+N'],['Öppna projekt…','open','Ctrl+O'],['Importera…','import',''],null,['Spara','save','Ctrl+S'],['Säkerhetskopia…','backup',''],['Exportera…','export','']],
    Redigera:[['Ångra','undo','Ctrl+Z'],['Gör om','redo','Ctrl+Shift+Z'],null,['Duplicera lager','layer-duplicate','Ctrl+J']],
    Bild:[['Framkallning',()=>document.querySelector('[data-mode=light]').click(),''],['Svartvitt','develop-bw',''],['Återställ justeringar','reset-look','']],
    Lager:[['Nytt lager','layer-new',''],['Textlager',()=>selectTool('text'),'T'],['Duplicera','layer-duplicate','Ctrl+J'],['Flytta upp','layer-up',''],['Flytta ned','layer-down',''],['Ta bort mask','mask-clear','']],
    Markera:[['Rektangulär markering',()=>selectTool('select'),'M'],['Avmarkera','selection-clear','Ctrl+D']],
    Sekvens:[['Dela vid spelhuvudet','split','C'],['Trimma markerat klipp','trim',''],['Flytta tidigare','clip-earlier',''],['Flytta senare','clip-later',''],['Ljud av / på','mute','']],
    Visa:[['Passa canvas','zoom-fit',''],['100 %',()=>setZoom('100'),''],['Filmrulle av / på','timeline-toggle',''],['Fokusera canvas','focus','Tab']],
    Fönster:[['Projektmedia av / på','left-show',''],['Verktyg av / på','right-toggle',''],['Spara arbetsyta…','layout-save',''],['Återställ arbetsyta','layout-reset','']],
    Hjälp:[['Verktyg och kortkommandon',()=>$('help-dialog').showModal(),'']]
  };
  for(const [label,items] of Object.entries(menus)){const d=document.createElement('details');d.className='app-menu';const summary=document.createElement('summary');summary.textContent=label;const div=document.createElement('div');div.className='app-menu-content';for(const item of items){if(!item){div.append(document.createElement('hr'));continue;}const [title,action,key]=item,b=document.createElement('button');const text=document.createElement('span');text.textContent=title;const kbd=document.createElement('kbd');kbd.textContent=key;b.append(text,kbd);b.onclick=run(async()=>{d.open=false;if(typeof action==='function')await action();else $(action).click();});div.append(b);}d.append(summary,div);d.addEventListener('toggle',()=>{if(d.open)for(const other of $('app-menus').querySelectorAll('details'))if(other!==d)other.open=false;});$('app-menus').append(d);}
  document.addEventListener('pointerdown',e=>{if(!e.target.closest('#app-menus'))for(const d of $('app-menus').querySelectorAll('details'))d.open=false;});
  function setZoom(value){zoom=value;$('canvas-zoom').value=value;syncZoom();updateOverlay();}
  function syncZoom(){const canvas=$('canvas'),stage=$('stage'),p=getState()?.project;if(!p)return;stage.classList.toggle('zoomed',zoom!=='fit');let scale=Number(zoom)/100;if(zoom==='fit'){const css=getComputedStyle(stage),width=stage.clientWidth-parseFloat(css.paddingLeft)-parseFloat(css.paddingRight),height=stage.clientHeight-parseFloat(css.paddingTop)-parseFloat(css.paddingBottom);scale=Math.max(.001,Math.min(width/p.width,height/p.height));}canvas.style.width=(p.width*scale)+'px';canvas.style.height=(p.height*scale)+'px';const frame=getState().local_frame;$('document-tab').textContent=`${p.name} @ ${zoom==='fit'?'Passa':zoom+' %'} (RGB/8) · ${frame}`;}
  $('canvas-zoom').onchange=()=>setZoom($('canvas-zoom').value);$('zoom-fit').onclick=()=>setZoom('fit');$('rail-split').onclick=()=>$('split').click();$('help-close').onclick=()=>$('help-dialog').close();$('stage').addEventListener('scroll',updateOverlay);
  new ResizeObserver(()=>{if(zoom==='fit'){syncZoom();updateOverlay();}}).observe($('stage'));
  return {sync:syncZoom,setZoom,stepZoom(out=false){const values=[25,50,100,200,400],n=zoom==='fit'?50:Number(zoom),next=out?[...values].reverse().find(v=>v<n):values.find(v=>v>n);setZoom(String(next||(out?25:400)));}};
}
