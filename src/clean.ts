import { call, type CleanItem, type CleanResult } from './api';
const esc = (s: unknown) => String(s ?? '').replace(/[&<>"]/g, c => ({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;'})[c]!);
const size = (n: number) => n >= 1024 ? `${(n/1024).toFixed(1)} GB` : `${Math.round(n)} MB`;
export function createClean(el: () => HTMLElement, toast: (s: string, bad?: boolean) => void, active: () => boolean) {
  let items: CleanItem[] = [], measured = false, loading = false, busy = false, error = '', result: CleanResult | null = null;
  let ticks: Record<string, boolean> = {};
  try { const saved = JSON.parse(localStorage.getItem('hub.cleanTicks') || '{}'); if(saved && typeof saved==='object' && !Array.isArray(saved)) ticks=saved; } catch {}
  const allowed = (i: CleanItem) => i.readable && !i.blockedBy && i.mb !== null;
  const picked = () => items.filter(i => allowed(i) && ticks[i.id]);
  const sum = (rows: CleanItem[]) => rows.reduce((n,i) => n+(i.mb || 0),0);
  function render() {
    if (!active()) return;
    const selected = picked();
    el().innerHTML = `<div class="memtab clean-tab">${result ? `<section class="soft-card clean-result"><span class="soft-pill mint">Freed ${size(result.freedMb)}</span><span>Empty the Trash to get the space back</span><button data-trash>Open Trash</button>${result.skipped.map(i=>`<small>${esc(items.find(x=>x.id===i.id)?.name || i.id)}: ${esc(i.why)}</small>`).join('')}</section>` : ''}<section class="soft-card clean-header"><div><h2>${loading ? 'Measuring…' : `Up to ${size(sum(items.filter(allowed)))}`}</h2><p class="faint">Caches your apps rebuild by themselves. You stay signed in everywhere.</p></div><button data-measure aria-label="Measure caches again" ${loading || busy ? 'disabled' : ''}>↻</button><button class="bigquit" data-clean ${!selected.length || loading || busy ? 'disabled' : ''}>${busy ? 'Moving…' : `Clean ${selected.length} · ${size(sum(selected))}`}</button></section>${error ? `<p role="alert">${esc(error)}</p>` : ''}${['Developer','Browsers'].map(group=>`<section class="soft-card"><h3>${group}</h3>${loading ? `<p class="faint">Measuring…</p>${'<div class="skeleton"></div>'.repeat(3)}` : items.filter(i=>i.group===group).sort((a,b)=>(b.mb||0)-(a.mb||0)).map(i=>`<label class="clean-row ${!allowed(i)?'unavailable':''}"><input type="checkbox" data-cache="${esc(i.id)}" ${ticks[i.id]?'checked':''} ${!allowed(i)||busy?'disabled':''} aria-label="Select ${esc(i.name)}"><span class="clean-name"><b>${esc(i.name)}</b><small>${esc(i.note)}</small></span>${i.blockedBy || !i.readable ? `<span class="soft-pill amber">${i.blockedBy ? `Quit ${esc(i.blockedBy)} first` : 'Needs Full Disk Access'}</span>` : ''}<span class="clean-size">${i.mb===null?'—':size(i.mb)}</span></label>`).join('')}</section>`).join('')}</div>`;
  }
  async function measure() {
    if (loading) return; measured=true; loading=true; error=''; render();
    try { items=(await call<{items: CleanItem[]}>('clean')).items; measured=true; for(const i of items) if(!(i.id in ticks)) ticks[i.id]=i.group==='Developer'&&!['playwright','xcode-device'].includes(i.id); }
    catch(e) { error=String(e); } finally { loading=false; render(); }
  }
  function leave() { document.getElementById('cleanPop')?.remove(); }
  function confirm() {
    leave(); const rows=picked(); const pop=document.createElement('div'); pop.id='cleanPop'; pop.className='mconfirm feature-confirm'; pop.setAttribute('role','dialog'); pop.setAttribute('aria-label','Move caches to Trash');
    pop.innerHTML=`<b>Move ${rows.length} caches to the Trash?</b><div class="confirm-list">${rows.map(i=>`<p>${esc(i.name)} <span>${size(i.mb||0)}</span></p>`).join('')}</div><p class="faint">Apps rebuild them when needed. Nothing is deleted until you empty the Trash.</p><div class="mf"><button data-cancel>Cancel</button><button class="primary" data-move>Move to Trash</button></div>`;
    document.body.append(pop); pop.querySelector<HTMLButtonElement>('[data-cancel]')!.focus();
    pop.onclick=async e=>{ const t=e.target as HTMLElement; if(t.closest('[data-cancel]')) leave(); if(t.closest('[data-move]')&&!busy) { busy=true; leave(); render(); try { result=await call<CleanResult>('cleanRun',{ids:rows.map(i=>i.id)}); toast(`Moved ${size(result.freedMb)} to the Trash`); } catch(e){toast(String(e),true);} finally {busy=false; await measure();} } };
  }
  document.addEventListener('keydown',e=>{if(e.key==='Escape')leave();});
  el().addEventListener('change',e=>{const t=e.target as HTMLInputElement;if(!t.dataset.cache)return;ticks[t.dataset.cache]=t.checked;try{localStorage.setItem('hub.cleanTicks',JSON.stringify(ticks));}catch{}render();});
  el().addEventListener('click',e=>{if(!active())return;const t=e.target as HTMLElement;if(t.closest('[data-measure]'))void measure();if(t.closest('[data-clean]'))confirm();if(t.closest('[data-trash]'))void call('openTrash').catch(e=>toast(String(e),true));});
  return {open(){ if(!measured&&!loading)void measure(); else if(!el().querySelector('.clean-tab'))render(); },leave};
}
