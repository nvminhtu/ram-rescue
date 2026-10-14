import { BUY_URL, call, type GuardStatus } from './api';
const esc = (s: unknown) => String(s ?? '').replace(/[&<>"]/g, c => ({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;'})[c]!);
const mem = (n: number) => n >= 1024 ? `${(n / 1024).toFixed(1)} GB` : `${n} MB`;
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
      return `<li>${l.what === 'quit' ? `Quit ${esc(l.name)}` : l.what === 'warned' ? `Warned about ${esc(l.name)}` : `${esc(l.name)} is leaking`} · ${mem(l.mb)} · ${ago}</li>`;
    }).join('');
    pop.innerHTML = `<label class="guard-row"><span>Warn before my Mac slows down<small>and when an app leaks memory</small></span><input type="checkbox" disabled ${on ? 'checked' : ''}></label><label class="guard-row"><span>Auto-quit orphans after</span><input id="autoOrphans" type="checkbox" ${status.autoOrphans ? 'checked' : ''} ${busy ? 'disabled' : ''}></label><select id="orphanMins" aria-label="Orphan timeout" ${busy ? 'disabled' : ''}>${[15,30,60].map(n=>`<option value="${n}" ${n === status!.orphanMins ? 'selected' : ''}>${n} min</option>`).join('')}</select><h4>Recent</h4>${recent ? `<ul>${recent}</ul>` : `<p class="faint">Nothing yet — Guard is watching.</p>`}<button class="link" data-license>License…</button>`;
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
    ov.innerHTML = `<section class="mdl pro-sheet" role="dialog" aria-modal="true" aria-labelledby="proTitle"><button class="pro-close icon" aria-label="Close">✕</button><img class="pro-logo" src="/logo.png" alt=""><h2 id="proTitle">RAM Rescue Pro</h2><p>Let RAM Rescue watch your memory for you.</p><ul class="pro-benefits"><li>Warns you before your Mac freezes — and names the app to quit</li><li>Catches apps that leak memory</li><li>Cleans up forgotten dev processes automatically</li></ul><b>$9.99 once · no subscription · 2 Macs</b><p class="license-state">${l?.state === 'licensed' ? `Licensed · key ••••${esc(l.key.slice(-4))}` : l?.state === 'trial' ? `Trial: ${l.daysLeft} days left. When it ends, Guard turns off and everything else stays free.` : l?.state === 'expired' ? 'Your trial has ended. Everything else stays free.' : 'License status unavailable. Please try again.'}</p>${l?.state === 'licensed' ? '<button class="link" id="deactivate">Deactivate this Mac</button>' : `${BUY_URL ? '<button class="bigquit buy">Buy Pro</button>' : ''}<form id="activateForm"><label for="licenseKey">Already bought?</label><div class="license-input"><input id="licenseKey" placeholder="License key" aria-label="License key" required><button type="submit">Activate</button></div><div class="err" role="alert" id="licenseError"></div></form><button class="link" data-url="https://app.lemonsqueezy.com/my-orders">Find my key</button>`}<footer><button class="link" data-url="https://oneclicktool.app/services/app/ram-rescue/terms.html">Terms</button><button class="link" data-url="https://oneclicktool.app/services/app/ram-rescue/privacy.html">Privacy</button></footer></section>`;
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
  pop.addEventListener('change', e => { const t = e.target as HTMLInputElement; void change(t.id === 'autoOrphans' ? {autoOrphans: t.checked} : {orphanMins: Number(t.value)}); });
  pop.addEventListener('click', e => { if ((e.target as HTMLElement).closest('[data-license]')) void show(); });
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
