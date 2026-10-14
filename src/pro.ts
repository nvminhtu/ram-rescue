import { memoryIcon } from './memory';
import { BUY_URL, call, type GuardStatus, type MemSnap, type AppChoice, type Rule, type MemGroup } from './api';
const esc = (s: unknown) => String(s ?? '').replace(/[&<>"]/g, c => ({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;'})[c]!);
const mem = (n: number) => n >= 1024 ? `${(n / 1024).toFixed(1)} GB` : `${n} MB`;

export async function pickApps(saved: AppChoice[], single: boolean, save: (apps: AppChoice[]) => Promise<void>, toast: (s: string, bad?: boolean) => void) {
  try {
    const snap = await call<MemSnap>('memory');
    const running = snap.groups.filter(g=>!g.system && g.id.startsWith('b:')).sort((a,b)=>b.mb-a.mb).slice(0,12);
    const choices = [...running, ...saved.filter(a=>!running.some(g=>g.id===a.id))];
    const selected = new Set(single ? [] : saved.map(a=>a.id));
    const ov=document.createElement('div'); ov.className='ov show app-picker';
    const draw=()=>{ov.innerHTML=`<section class="mdl" role="dialog" aria-modal="true" aria-label="Choose apps"><h3>${single?'Add an idle rule':'Meeting apps'}</h3><p class="faint">${single?'Choose an app.':'Save your work before freeing RAM.'}</p><div class="picker-list">${choices.map(a=>`<label class="picker-row"><input type="${single?'radio':'checkbox'}" name="app" value="${esc(a.id)}" ${selected.has(a.id)?'checked':''}>${'mb' in a ? memoryIcon(a as MemGroup,draw) : '<span class="micon">'+esc(a.name[0])+'</span>'}<b>${esc(a.name)}</b><span class="faint">${'mb' in a?mem(Number(a.mb)):'Not running'}</span></label>`).join('') || '<p class="faint">No apps available.</p>'}</div><div class="mf"><button data-dismiss>Cancel</button><button class="primary" data-save>Save</button></div></section>`;};
    const focus=document.activeElement as HTMLElement;
    const close=()=>{ov.remove();document.removeEventListener('keydown',key,true);focus?.focus();};
    const key=(e: KeyboardEvent)=>{if(e.key==='Escape'){e.stopImmediatePropagation();close();} if(e.key==='Tab'){const inputs=[...ov.querySelectorAll<HTMLElement>('button:not(:disabled),input')];const first=inputs[0],last=inputs[inputs.length-1];if(e.shiftKey&&document.activeElement===first){e.preventDefault();last?.focus();}else if(!e.shiftKey&&document.activeElement===last){e.preventDefault();first?.focus();}}};
    document.addEventListener('keydown',key,true); document.body.append(ov); draw(); ov.querySelector<HTMLElement>('input,button')?.focus();
    ov.onchange=e=>{const t=e.target as HTMLInputElement;if(single)selected.clear();if(t.checked)selected.add(t.value);else selected.delete(t.value);};
    ov.onclick=async e=>{const t=e.target as HTMLElement;if(t===ov||t.closest('[data-dismiss]'))close();if(t.closest('[data-save]')){const b=t.closest('button')!;b.disabled=true;try{await save(choices.filter(a=>selected.has(a.id)).map(a=>({id:a.id,name:a.name})));close();}catch(e){toast(String(e),true);b.disabled=false;}}};
  } catch(e){toast(String(e),true);}
}

export function createPro(toast: (s: string, bad?: boolean) => void) {
  let status: GuardStatus | null = null;
  let busy = false;
  let returnFocus: HTMLElement | null = null;
  const host = document.getElementById('guard')!;
  const pop = document.createElement('div');
  pop.className = 'guard-pop'; pop.hidden = true; document.body.append(pop);
  const ov = document.createElement('div'); ov.className = 'ov'; ov.id = 'proOv'; document.body.append(ov);
  const open = (target: string) => void call('open', {target}).catch(e => toast(e.message, true));
  function render() {
    const on = status?.license.state !== 'expired' && !!status?.on;
    host.innerHTML = `<button class="guard-label" aria-expanded="${!pop.hidden}">Guard <span class="pro-badge">PRO</span></button><button class="switch ${on ? 'on' : ''}" role="switch" aria-label="Guard" aria-checked="${on}" ${busy || !status ? 'disabled' : ''}></button>${status?.license.state === 'trial' ? `<small>Trial · ${status.license.daysLeft} days left</small>` : ''}`;
    if (pop.hidden || !status) return;
    const recent = [...status.log].sort((a,b) => b.t-a.t).slice(0,5).map(l => {
      const age = Math.max(0, Math.floor((Date.now() - (l.t < 1e12 ? l.t * 1000 : l.t)) / 60000));
      const ago = age < 1 ? 'just now' : age < 60 ? `${age}m ago` : `${Math.floor(age/60)}h ago`;
      return `<li>${l.what === 'idle' ? `Quit ${esc(l.name)} (not used ${l.mins ?? status?.rules?.find(r=>r.name===l.name)?.mins ?? 30} min)` : l.what === 'meeting' ? `Meeting mode · ${l.count != null ? `${l.count} apps` : esc(l.name)}` : l.what === 'quit' ? `Quit ${esc(l.name)}` : l.what === 'warned' ? `Warned about ${esc(l.name)}` : `${esc(l.name)} is leaking`} · ${mem(l.mb)} · ${ago}</li>`;
    }).join('');
    pop.innerHTML = `<label class="guard-row"><span>Warn before my Mac slows down<small>and when an app leaks memory</small></span><input type="checkbox" disabled ${on ? 'checked' : ''}></label><label class="guard-row"><span>Auto-quit orphans after</span><input id="autoOrphans" type="checkbox" ${status.autoOrphans ? 'checked' : ''} ${busy ? 'disabled' : ''}></label><select id="orphanMins" aria-label="Orphan timeout" ${busy ? 'disabled' : ''}>${[15,30,60].map(n=>`<option value="${n}" ${n === status!.orphanMins ? 'selected' : ''}>${n} min</option>`).join('')}</select><section class="idle-section"><h4>Quit when not used ${status.license.state==='expired'?'<span class="pro-badge">PRO</span>':''}</h4>${status.license.state==='expired'?'<fieldset disabled class="idle-preview"><select aria-label="Idle timeout"><option>30 min</option></select><select aria-label="Idle condition"><option>when memory is tight</option></select></fieldset><button class="link" data-license>Unlock idle rules</button>':`${(status.rules||[]).map((r,i)=>`<div class="idle-rule">${memoryIcon(r,()=>{if(!busy)render();})}<b>${esc(r.name)}</b><button data-remove-rule="${i}" aria-label="Remove ${esc(r.name)}" ${busy?'disabled':''}>✕</button><select data-rule="${i}" data-field="mins" aria-label="Idle time for ${esc(r.name)}" ${busy?'disabled':''}>${[15,30,60,120].map(n=>`<option value="${n}" ${n===r.mins?'selected':''}>${n} min</option>`).join('')}</select><select data-rule="${i}" data-field="always" aria-label="Condition for ${esc(r.name)}" ${busy?'disabled':''}><option value="false" ${!r.always?'selected':''}>when memory is tight</option><option value="true" ${r.always?'selected':''}>always</option></select></div>`).join('')}<button class="link" data-add-rule ${busy?'disabled':''}>+ Add app</button>`}</section><h4>Recent</h4>${recent ? `<ul>${recent}</ul>` : `<p class="faint">Nothing yet — Guard is watching.</p>`}<button class="link" data-license>License…</button>`;
  }
  async function refresh() { try { status = await call<GuardStatus>('guard'); render(); } catch(e) { if (!pop.hidden) toast(String(e), true); } }
  async function change(args: Record<string, unknown>) {
    if (status?.license.state === 'expired') return show();
    busy = true; render();
    try { status = await call<GuardStatus>('guardSet', args); await refresh(); } catch(e) { toast(String(e), true); }
    finally { busy = false; render(); }
  }
  function close() { ov.classList.remove('show'); returnFocus?.focus(); }
  async function show() {
    pop.hidden = true; returnFocus = document.activeElement as HTMLElement;
    if (!status) await refresh();
    const l = status?.license;
    ov.innerHTML = `<section class="mdl pro-sheet" role="dialog" aria-modal="true" aria-labelledby="proTitle"><button class="pro-close icon" aria-label="Close">✕</button><img class="pro-logo" src="/logo.png" alt=""><h2 id="proTitle">RAM Rescue Pro</h2><p>Let RAM Rescue watch your memory for you.</p><ul class="pro-benefits"><li>Warns you before your Mac freezes — and names the app to quit</li><li>Catches apps that leak memory</li><li>24-hour memory history — see what slowed you down</li><li>Meeting mode: free RAM in one click, bring apps back after</li><li>Quits apps you are not using, and forgotten dev processes</li></ul><b>$9.99 once · no subscription · 2 Macs</b><p class="license-state">${l?.state === 'licensed' ? `Licensed · key ••••${esc(l.key.slice(-4))}` : l?.state === 'trial' ? `Trial: ${l.daysLeft} days left. Memory and Clean stay free.` : l?.state === 'expired' ? 'Your trial has ended. Memory and Clean stay free.' : 'License status unavailable. Please try again.'}</p>${l?.state === 'licensed' ? '<button class="link" id="deactivate">Deactivate this Mac</button>' : `${BUY_URL ? '<button class="bigquit buy">Buy Pro</button>' : ''}<form id="activateForm"><label for="licenseKey">Already bought?</label><div class="license-input"><input id="licenseKey" placeholder="License key" aria-label="License key" required><button type="submit">Activate</button></div><div class="err" role="alert" id="licenseError"></div></form><button class="link" data-url="https://app.lemonsqueezy.com/my-orders">Find my key</button>`}<footer><button class="link" data-url="https://oneclicktool.app/services/app/ram-rescue/terms.html">Terms</button><button class="link" data-url="https://oneclicktool.app/services/app/ram-rescue/privacy.html">Privacy</button></footer></section>`;
    ov.querySelector('img')!.addEventListener('error', e => (e.target as HTMLElement).hidden = true);
    ov.classList.add('show'); ov.querySelector<HTMLElement>('.pro-close')!.focus(); render();
    ov.querySelector('form')?.addEventListener('submit', async e => {
      e.preventDefault(); const btn = ov.querySelector<HTMLButtonElement>('[type=submit]')!; btn.disabled = true;
      try {
        const result = await call<{error?: string; ok?: boolean}>('licenseActivate', {key: ov.querySelector<HTMLInputElement>('input')!.value.trim()});
        if (result.error || result.ok === false) throw new Error(result.error || 'Activation failed');
        await refresh(); toast('RAM Rescue Pro is active'); close();
      } catch(e) { ov.querySelector('#licenseError')!.textContent = e instanceof Error ? e.message : String(e); }
      finally { btn.disabled = false; }
    });
  }
  host.addEventListener('click', e => { if ((e.target as HTMLElement).closest('.switch')) void change({on: !status?.on}); else { pop.hidden = !pop.hidden; render(); } });
  pop.addEventListener('change', e => { const t = e.target as HTMLInputElement; if(t.dataset.rule!=null){const rules: Rule[]=(status?.rules||[]).map(r=>({...r})); const r=rules[Number(t.dataset.rule)]; if(t.dataset.field==='mins')r.mins=Number(t.value);else r.always=t.value==='true';void change({rules});return;} void change(t.id === 'autoOrphans' ? {autoOrphans: t.checked} : {orphanMins: Number(t.value)}); });
  pop.addEventListener('click', e => { const t=e.target as HTMLElement; const remove=t.closest<HTMLElement>('[data-remove-rule]'); if(remove)void change({rules:(status?.rules||[]).filter((_,i)=>i!==Number(remove.dataset.removeRule))}); if(t.closest('[data-add-rule]'))void pickApps([],true,async apps=>{await change({rules:[...(status?.rules||[]),...apps.filter(a=>!status?.rules?.some(r=>r.id===a.id)).map(a=>({...a,mins:30,always:false}))]});},toast); if ((e.target as HTMLElement).closest('[data-license]') || (status?.license.state==='expired' && t.closest('.idle-section'))) void show(); });
  // composedPath: the clicked button may already be replaced by render() when this runs
  document.addEventListener('click', e => { const path = e.composedPath(); if (!path.includes(host) && !path.includes(pop)) { pop.hidden = true; } });
  ov.addEventListener('click', async e => {
    const t = e.target as HTMLElement;
    if (t === ov || t.closest('.pro-close')) close();
    const link = t.closest<HTMLElement>('[data-url]'); if(link) open(link.dataset.url!);
    if(t.closest('.buy') && BUY_URL) open(BUY_URL);
    if(t.closest('#deactivate')) {
      const b = t.closest<HTMLButtonElement>('button')!;
      if(b.dataset.armed !== 'yes') { b.dataset.armed = 'yes'; b.textContent = 'Deactivate this Mac? Click to confirm'; return; }
      b.disabled = true;
      try { const r = await call<{error?: string}>('licenseDeactivate'); if(r.error) throw new Error(r.error); await refresh(); await show(); }
      catch(e) { toast(String(e), true); b.disabled = false; }
    }
  });
  document.addEventListener('keydown', e => {
    if (!ov.classList.contains('show')) { if (e.key === 'Escape' && !pop.hidden) { pop.hidden = true; e.stopImmediatePropagation(); } return; }
    if(e.key === 'Escape') { e.stopImmediatePropagation(); close(); }
    if(e.key === 'Tab') {
      const items = [...ov.querySelectorAll<HTMLElement>('button:not(:disabled), input')].filter(x=>x.offsetParent !== null);
      const first=items[0], last=items[items.length-1];
      if(e.shiftKey && document.activeElement === first) { e.preventDefault(); last.focus(); }
      else if(!e.shiftKey && document.activeElement === last) { e.preventDefault(); first.focus(); }
    }
  }, true);
  void refresh(); setInterval(() => { if(!document.hidden && !busy) void refresh(); }, 10000);
  return {show};
}
