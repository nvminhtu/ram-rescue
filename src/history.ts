import { call, type HistoryData, type MemSnap, type HistoryPoint } from './api';
import { memoryIcon } from './memory';
const esc=(s: string)=>s.replace(/[&<>"]/g,c=>({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;'})[c]!);
const gb=(mb: number)=>`${(mb/1024).toFixed(1)} GB`;
const time=(t: number)=>new Date(t*1000).toLocaleTimeString('en-US',{hour:'numeric',minute:'2-digit'});
export async function showHistory(host: HTMLElement, snap: MemSnap, pro: boolean, back: () => void, unlock: () => void) {
  host.innerHTML='<div class="memtab history-view"><button class="all-processes" data-back>‹ Back</button><p class="faint">Reading history…</p></div>';
  host.querySelector('[data-back]')!.addEventListener('click',back);
  const view=host.firstElementChild!;
  let data: HistoryData;
  try {data=await call<HistoryData>('history');}catch(e){if(host.contains(view)){view.insertAdjacentHTML('beforeend',`<p role="alert">${esc(String(e))}</p>`);}return;}
  if(!host.contains(view))return;
  const locked=!pro||!data.pro;
  const end=Date.now()/1000,start=end-86400,total=snap.total/2**20;
  const points=data.points.filter(p=>p.t>=start&&p.t<=end).sort((a,b)=>a.t-b.t);
  const counts=new Map<string,{n:number,sum:number}>();
  for(const p of points)for(const [name,mb] of p.apps){const v=counts.get(name)||{n:0,sum:0};v.n++;v.sum+=mb;counts.set(name,v);}
  const names=[...counts].sort((a,b)=>b[1].n-a[1].n||b[1].sum-a[1].sum).slice(0,5).map(a=>a[0]);
  const colors=['#A7DCC4','#F2C58B','#C0B8E3','#F4AAA3','#A4CCE9'];
  const x=(t:number)=>46+(t-start)/86400*734, y=(mb:number)=>192-Math.min(total,Math.max(0,mb))/Math.max(1,total)*174;
  const path=(ps: {x:number,y:number}[])=>ps.map((p,i)=>`${i?'L':'M'}${p.x.toFixed(2)},${p.y.toFixed(2)}`).join(' ');
  const appMb=(p:HistoryPoint,n:string)=>p.apps.find(a=>a[0]===n)?.[1]||0;
  let areas=names.map((n,i)=>{const bottom=points.map(p=>({x:x(p.t),y:y(names.slice(0,i).reduce((v,n)=>v+appMb(p,n),0))}));const top=points.map(p=>({x:x(p.t),y:y(names.slice(0,i+1).reduce((v,n)=>v+appMb(p,n),0))}));return `<path d="${path(top)} ${bottom.reverse().map(p=>`L${p.x},${p.y}`).join(' ')} Z" fill="${colors[i]}" opacity=".7"/>`;}).join('');
  if(locked && points.length<5)areas='<path d="M46 192 L46 140 L160 120 L260 150 L360 80 L460 100 L560 60 L680 95 L780 70 L780 192Z" fill="var(--mint)" opacity=".35"/>';
  let ticks='';const tick=new Date(start*1000);tick.setMinutes(0,0,0);tick.setHours(tick.getHours()+1);for(let t=tick.getTime()/1000;t<end;t+=3600)if(new Date(t*1000).getHours()%3===0)ticks+=`<text x="${x(t)}" y="216" text-anchor="middle">${new Date(t*1000).toLocaleTimeString('en-US',{hour:'numeric'})}</text>`;
  const peak=points.reduce<HistoryPoint|undefined>((a,p)=>!a||p.usedMb>a.usedMb?p:a,undefined);
  const heaviest=[...counts].sort((a,b)=>b[1].sum-a[1].sum)[0];
  view.innerHTML=`<button class="all-processes" data-back>‹ Back</button><section class="soft-card history-card"><h3>Memory · last 24 hours</h3><div class="history-stage"><div class="history-content ${locked?'history-blur':''}" ${locked?'inert aria-hidden="true"':''}><svg class="history-chart" viewBox="0 0 800 224" preserveAspectRatio="none" role="img" aria-label="Memory in GB over the last 24 hours" tabindex="${locked?-1:0}">${[0,.5,1].map(v=>`<text x="3" y="${y(total*v)+4}">${(total*v/1024).toFixed(0)} GB</text><line x1="46" x2="780" y1="${y(total*v)}" y2="${y(total*v)}" class="chart-grid"/>`).join('')}${areas}<line x1="46" x2="780" y1="${y(total*.8)}" y2="${y(total*.8)}" class="chart-grid" stroke-dasharray="5 5"/><path d="${path(points.map(p=>({x:x(p.t),y:y(p.usedMb)})))}" fill="none" stroke="var(--text)" stroke-width="1.5"/>${ticks}<line data-cursor x1="46" x2="46" y1="18" y2="192" stroke="var(--secondary)" visibility="hidden"/></svg><div class="history-legend">${names.map((n,i)=>{const g=snap.groups.find(g=>g.name===n);return `<span class="soft-pill">${g?memoryIcon(g):`<i style="background:${colors[i]}"></i>`}${esc(n)}</span>`;}).join('')}</div><div class="history-tooltip" hidden></div></div>${locked?'<div class="history-gate soft-card"><b>See what slowed your Mac down — RAM Rescue Pro</b><button class="bigquit" data-unlock>Unlock Pro</button></div>':''}</div>${!locked&&points.length<5?'<p class="faint">History fills in as RAM Rescue runs — check back in a few minutes.</p>':''}${!locked&&peak?`<div class="history-stats"><span class="soft-pill">Peak ${gb(peak.usedMb)} at ${time(peak.t)}</span><span class="soft-pill">Heaviest: ${esc(heaviest?.[0]||'—')} (avg ${gb((heaviest?.[1].sum||0)/points.length)})</span><span class="soft-pill">Now ${gb(snap.used/2**20)}</span></div>`:''}</section>`;
  view.querySelector('[data-back]')!.addEventListener('click',back);
  view.querySelector('[data-unlock]')?.addEventListener('click',unlock);
  if(locked||!points.length)return;
  const svg=view.querySelector<SVGSVGElement>('svg')!, tooltip=view.querySelector<HTMLElement>('.history-tooltip')!, cursor=view.querySelector('[data-cursor]')!;
  let index=points.length-1;
  function inspect(i:number){index=Math.max(0,Math.min(points.length-1,i));const p=points[index];cursor.setAttribute('x1',String(x(p.t)));cursor.setAttribute('x2',String(x(p.t)));cursor.setAttribute('visibility','visible');tooltip.hidden=false;tooltip.innerHTML=`<b>${time(p.t)} · ${gb(p.usedMb)} used</b>${[...p.apps].sort((a,b)=>b[1]-a[1]).slice(0,3).map(a=>`<span>${esc(a[0])} · ${gb(a[1])}</span>`).join('')}`;}
  const point=(e:MouseEvent)=>{const r=svg.getBoundingClientRect(),t=start+((e.clientX-r.left)/r.width*800-46)/734*86400;inspect(points.reduce((best,p,i)=>Math.abs(p.t-t)<Math.abs(points[best].t-t)?i:best,0));};
  svg.addEventListener('mousemove',point);svg.addEventListener('click',point);svg.addEventListener('keydown',e=>{if(e.key==='ArrowLeft'||e.key==='ArrowRight'){e.preventDefault();inspect(index+(e.key==='ArrowLeft'?-1:1));}});
}
