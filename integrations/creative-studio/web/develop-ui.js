// UI built from the exact supported LightCraft controls exposed by the Rust Studio.
export function createDevelopUI({getState,command,editScope,status}){
  const $=id=>document.getElementById(id),fields=new Map();let ready=false,drag=null,curveScope,curvePoints=[],channel='master';
  const run=fn=>async(...args)=>{try{await fn(...args);}catch(e){status(e.message,true);}};
  function build(){
    if(ready)return;ready=true;const groups=new Map();
    const legacy=new Set(['light.exposure','light.contrast','light.highlights','light.shadows','color.saturation','wb.temp','wb.tint']);
    const titles={light:'Ljus',color:'Färg',curve:'Parametrisk kurva',mixer:'Färgmixer · HSL',bwMix:'Svartvit färgmix',grading:'Färggradering',effects:'Effekter',vignette:'Vinjettering',grain:'Korn',detail:'Detalj',calibration:'Kalibrering',optics:'Optik',geometry:'Geometri och perspektiv',profile:'Profil'};
    for(const spec of getState().develop_controls){
      if(legacy.has(spec.id))continue;
      const key=spec.section;let parent;
      if(key==='light')parent=$('light-controls');else if(key==='color')parent=$('color-controls');else{
        if(!groups.has(key)){const d=document.createElement('details');d.className='develop-section advanced-section';const summary=document.createElement('summary');summary.textContent=titles[key]||key;const div=document.createElement('div');d.append(summary,div);$('advanced-controls').append(d);groups.set(key,div);}parent=groups.get(key);
      }
      const row=document.createElement('div');row.className='slider-row';row.dataset.control=spec.id;
      const header=document.createElement('div');header.className='slider-label';const label=document.createElement('label');label.textContent=spec.label;const number=document.createElement('input');number.type='number';number.min=spec.min;number.max=spec.max;number.step=spec.step;number.setAttribute('aria-label',spec.label+' värde');header.append(label,number);
      const range=document.createElement('input');range.type='range';range.id='develop-'+spec.id;label.htmlFor=range.id;range.min=spec.min;range.max=spec.max;range.step=spec.step;range.value=spec.default;range.setAttribute('aria-label',spec.label);row.append(header,range);parent.append(row);fields.set(spec.id,{range,number,spec});
      let scope,timer,value;
      range.onpointerdown=()=>{scope=editScope();};range.onpointerup=()=>{scope=null;};
      function change(event){const target=scope||editScope();value=Number(event.target.value);range.value=value;number.value=value;clearTimeout(timer);timer=setTimeout(run(()=>command('develop.set',{scope:target,values:{[spec.id]:value}})),110);}
      range.oninput=run(change);number.onchange=run(change);range.ondblclick=run(()=>command('develop.set',{scope:editScope(),values:{[spec.id]:spec.default}}));
    }
  }
  function look(){const s=getState();return s.project.workspace.scope==='project'?s.project.project_look:s.look||{};}
  const graph=$('tone-curve'),cx=graph.getContext('2d'),hist=$('histogram'),hx=hist.getContext('2d');
  function sync(){build();const values=look();for(const [id,{range,number,spec}] of fields){if(document.activeElement===range||document.activeElement===number)continue;range.value=values[id]??spec.default;number.value=Number(range.value).toFixed(spec.decimals);}if(!drag){curvePoints=structuredClone(values['curve.'+channel]?.length>=2?values['curve.'+channel]:[{x:0,y:0},{x:1,y:1}]);drawCurve();}$('develop-bw').classList.toggle('active',values.treatment==='bw');}
  function drawCurve(){cx.clearRect(0,0,256,256);cx.fillStyle='#232323';cx.fillRect(0,0,256,256);cx.strokeStyle='#3b3b3b';cx.lineWidth=1;for(let i=1;i<4;i++){cx.beginPath();cx.moveTo(i*64,0);cx.lineTo(i*64,256);cx.moveTo(0,i*64);cx.lineTo(256,i*64);cx.stroke();}cx.strokeStyle='#646464';cx.setLineDash([3,4]);cx.beginPath();cx.moveTo(0,256);cx.lineTo(256,0);cx.stroke();cx.setLineDash([]);cx.strokeStyle=({red:'#dd8888',green:'#84c28e',blue:'#82aef0'})[channel]||'#ddd';cx.lineWidth=1.5;cx.beginPath();curvePoints.forEach((p,i)=>{if(i)cx.lineTo(p.x*256,(1-p.y)*256);else cx.moveTo(p.x*256,(1-p.y)*256);});cx.stroke();for(const p of curvePoints){cx.fillStyle='#333';cx.strokeStyle='#ddd';cx.fillRect(p.x*256-3,(1-p.y)*256-3,6,6);cx.strokeRect(p.x*256-3,(1-p.y)*256-3,6,6);}}
  function xy(event){const r=graph.getBoundingClientRect();return {x:Math.max(0,Math.min(1,(event.clientX-r.left)/r.width)),y:Math.max(0,Math.min(1,1-(event.clientY-r.top)/r.height))};}
  graph.onpointerdown=run(e=>{if(e.button!==0)return;const p=xy(e);curveScope=editScope();let index=curvePoints.findIndex(v=>Math.hypot(p.x-v.x,p.y-v.y)<.055);if(index<0){if(curvePoints.length>=32)return;curvePoints.push(p);curvePoints.sort((a,b)=>a.x-b.x);index=curvePoints.indexOf(p);}drag={index,channel};graph.setPointerCapture(e.pointerId);drawCurve();});
  graph.onpointermove=e=>{if(!drag)return;const p=xy(e),i=drag.index;if(i===0)p.x=0;else if(i===curvePoints.length-1)p.x=1;else p.x=Math.max(curvePoints[i-1].x+.005,Math.min(curvePoints[i+1].x-.005,p.x));curvePoints[i]=p;drawCurve();};
  graph.onpointerup=run(async()=>{if(!drag)return;const key='curve.'+drag.channel,points=structuredClone(curvePoints);drag=null;await command('develop.set',{scope:curveScope,values:{[key]:points}});});graph.onpointercancel=()=>{drag=null;sync();};
  graph.ondblclick=run(async e=>{const p=xy(e),i=curvePoints.findIndex(v=>Math.hypot(p.x-v.x,p.y-v.y)<.055);if(i>0&&i<curvePoints.length-1){curvePoints.splice(i,1);await command('develop.set',{scope:editScope(),values:{['curve.'+channel]:curvePoints}});}});
  $('curve-channel').onchange=()=>{channel=$('curve-channel').value;sync();};$('curve-reset').onclick=run(()=>command('develop.set',{scope:editScope(),values:{['curve.'+channel]:[{x:0,y:0},{x:1,y:1}]}}));
  $('develop-bw').onclick=run(()=>command('develop.set',{scope:editScope(),values:{treatment:look().treatment==='bw'?'color':'bw'}}));
  function histogram(pixels){const bins=[new Uint32Array(256),new Uint32Array(256),new Uint32Array(256)];for(let i=0;i<pixels.length;i+=16){if(pixels[i+3]===0)continue;for(let c=0;c<3;c++)bins[c][pixels[i+c]]++;}let max=1;for(const bin of bins)for(const count of bin)max=Math.max(max,count);hx.clearRect(0,0,hist.width,hist.height);hx.fillStyle='#202020';hx.fillRect(0,0,hist.width,hist.height);hx.globalCompositeOperation='screen';for(let c=0;c<3;c++){hx.fillStyle=['#a64c48','#609267','#566ea3'][c];hx.beginPath();hx.moveTo(0,92);for(let i=0;i<256;i++)hx.lineTo(i/255*hist.width,92-Math.sqrt(bins[c][i]/max)*84);hx.lineTo(hist.width,92);hx.closePath();hx.fill();}hx.globalCompositeOperation='source-over';}
  return {sync,histogram,count:()=>fields.size};
}
