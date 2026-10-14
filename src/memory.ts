// Memory tab: what eats RAM (apps grouped with their helpers), filtered High / Medium / Low / Orphans, quit one or many.
// The core re-checks every pid before it signals anything: system, other users' and Server Hub's own processes are refused.
import { call, type MemGroup, type MemSnap } from './api';

export interface MemDeps {
  toast(msg: string, bad?: boolean): void;
  query(): string;
  get(k: string): string | null;
  set(k: string, v: string): void;
  active(): boolean;
  badge(orphans: number): void;
}

type Filter = 'high' | 'medium' | 'low' | 'orphans' | 'all';
const HIGH = 500, MEDIUM = 150; // MB

const esc = (s: unknown) => String(s ?? '').replace(/[&<>"]/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' })[c]!);
const fmtMem = (mb: number) => (mb >= 1024 ? `${(mb / 1024).toFixed(1)} GB` : `${mb} MB`);
const fmtGB = (b: number) => (b / 2 ** 30).toFixed(1);
function fmtUp(s: number) {
  if (s < 60) return `${s}s`;
  if (s < 3600) return `${Math.floor(s / 60)}m`;
  if (s < 86400) return `${Math.floor(s / 3600)}h`;
  return `${Math.floor(s / 86400)}d`;
}
const termIcon = '<svg viewBox="0 0 18 18" aria-hidden="true"><rect x="1" y="2" width="16" height="14" rx="3.5" fill="#5f6368"/><path d="M5 7l2.5 2L5 11M9 11.5h4" stroke="#fff" stroke-width="1.4" fill="none" stroke-linecap="round"/></svg>';
const lockIcon = '<svg viewBox="0 0 18 18" aria-hidden="true"><rect x="1" y="2" width="16" height="14" rx="3.5" fill="#b9b9be"/><rect x="6" y="8" width="6" height="5" rx="1" fill="#fff"/><path d="M7 8V6.6a2 2 0 0 1 4 0V8" stroke="#fff" stroke-width="1.3" fill="none"/></svg>';
function appIcon(name: string) {
  let h = 0;
  for (const c of name) h = (h * 31 + c.charCodeAt(0)) % 360;
  return `<span class="micon" style="background:hsl(${h} 60% 62%)"></span>`;
}

export function createMemory(el: () => HTMLElement, d: MemDeps) {
  let data: MemSnap | null = null;
  let filter = (d.get('hub.memFilter') as Filter) || 'high';
  let hideSystem = d.get('hub.memSys') !== '0';
  let selected: string | null = null;
  const checked = new Set<string>();
  let confirm: { ids: string[]; anchor: string } | null = null;
  let menuFor: string | null = null;
  let loading = false;

  const bucket = (g: MemGroup): Filter => (g.mb >= HIGH ? 'high' : g.mb >= MEDIUM ? 'medium' : 'low');
  const base = () => (data?.groups ?? []).filter((g) => !(hideSystem && g.system));
  const counts = () => {
    const b = base();
    return { high: b.filter((g) => bucket(g) === 'high').length, medium: b.filter((g) => bucket(g) === 'medium').length, low: b.filter((g) => bucket(g) === 'low').length, orphans: b.filter((g) => g.orphan).length, all: b.length };
  };
  const list = () => {
    const words = d.query().toLowerCase().split(/\s+/).filter(Boolean);
    return base()
      .filter((g) => (filter === 'all' ? true : filter === 'orphans' ? g.orphan : bucket(g) === filter))
      .filter((g) => words.every((w) => `${g.name} ${g.kind} ${g.leftBy ?? ''} ${g.ports.join(' ')}`.toLowerCase().includes(w.replace(/^:/, ''))));
  };
  const byId = (id: string) => data?.groups.find((g) => g.id === id);

  async function load() {
    if (loading) return;
    loading = true;
    try {
      data = await call<MemSnap>('memory');
      const ids = new Set(data.groups.map((g) => g.id));
      for (const id of [...checked]) if (!ids.has(id)) checked.delete(id);
      if (selected && !ids.has(selected)) selected = null;
      if (filter === 'orphans' && !data.groups.some((g) => g.orphan && !(hideSystem && g.system))) filter = 'high';
      d.badge(data.groups.filter((g) => g.orphan).length);
      if (d.active()) render();
    } catch {
      if (d.active() && !data) el().innerHTML = '<div class="empty">Core unavailable.</div>';
    } finally {
      loading = false;
    }
  }

  function summary() {
    if (!data) return '';
    const sum = (k: string) => data!.groups.filter((g) => (k === 'Other' ? g.system : !g.system && g.kind === k)).reduce((a, g) => a + g.mb, 0);
    const parts = [['App', 'seg-app'], ['Background', 'seg-bg'], ['Dev', 'seg-dev'], ['Other', 'seg-sys']] as const;
    const usedMb = data.used / 2 ** 20, totalMb = data.total / 2 ** 20;
    const raw = parts.map(([k]) => sum(k));
    const scale = raw.reduce((a, b) => a + b, 0) > 0 ? Math.min(1, usedMb / raw.reduce((a, b) => a + b, 0)) : 0;
    const segs = parts.map(([k, cls], i) => `<i class="${cls}" style="width:${((raw[i] * scale) / totalMb) * 100}%" title="${k} ${fmtMem(Math.round(raw[i]))}"></i>`).join('');
    const p = data.pressure;
    return `<div class="msum"><b>${fmtGB(data.used)} of ${fmtGB(data.total)} GB</b><span class="mram">${segs}</span><span class="mpress"><span class="dot ${p}"></span>Pressure: ${p[0].toUpperCase() + p.slice(1)}</span></div>`;
  }

  function filters() {
    const c = counts();
    const seg = (f: Filter, label: string) => (f === 'orphans' && !c.orphans ? '' : `<button class="${filter === f ? 'sel' : ''}" data-mfilter="${f}">${label}${f === 'all' ? '' : ` <small>${c[f]}</small>`}</button>`);
    return `<div class="mfil"><div class="tabs">${seg('high', 'High')}${seg('medium', 'Medium')}${seg('low', 'Low')}${seg('orphans', 'Orphans')}${seg('all', 'All')}</div><span class="grow"></span><label class="chk"><input type="checkbox" data-msys ${hideSystem ? 'checked' : ''}> Hide system</label></div>`;
  }

  function row(g: MemGroup, max: number) {
    const sel = selected === g.id;
    const icon = g.system ? lockIcon : g.kind === 'Dev' || g.orphan ? termIcon : appIcon(g.name);
    const kind = g.system ? 'System' : g.kind === 'Dev' && g.ports.length ? `Dev :${g.ports[0]}` : g.count > 1 ? `${g.kind} · ${g.count}` : g.kind;
    const level = g.mb >= 1024 ? 'red' : g.mb >= HIGH ? 'amber' : '';
    const mem = `<span class="mcell"><span class="mnum">${fmtMem(g.mb)}</span><span class="mbar"><i class="${level}" style="width:${Math.max(3, (g.mb / max) * 100)}%"></i></span></span>`;
    const box = g.system ? '<span></span>' : `<span class="mchk"><input type="checkbox" data-mcheck="${esc(g.id)}" ${checked.has(g.id) ? 'checked' : ''} aria-label="Select ${esc(g.name)}"></span>`;
    const acts = g.system
      ? '<span class="acts"></span>'
      : `<span class="acts mact"><button class="mq" data-mquit="${esc(g.id)}">Quit</button><span class="menu-wrap"><button class="mchev" data-mmenu="${esc(g.id)}" aria-label="More">⌄</button>${menuFor === g.id ? `<div class="popover"><button class="danger" data-mforce="${esc(g.id)}">Force Quit</button></div>` : ''}</span></span>`;
    const mid = filter === 'orphans'
      ? `<span class="faint-cell">${esc(g.leftBy ?? '—')}</span><span class="port">${g.ports.length ? g.ports.map((p) => `:${p}`).join(' ') : '—'}</span>${mem}`
      : `<span class="kind">${esc(kind)}</span>${mem}<span class="cpu">${Math.round(g.cpu)}%</span>`;
    return `<div role="row" class="row mrow ${filter === 'orphans' ? 'orph-grid' : 'mem-grid'} ${sel ? 'sel' : ''} ${g.system ? 'sys' : ''}" data-mrow="${esc(g.id)}" title="${g.system ? 'Part of macOS — not quittable' : esc(g.pids.length > 1 ? `${g.pids.length} processes` : `pid ${g.pids[0]}`)}">${box}<span class="mic">${icon}</span><span class="nm">${esc(g.name)}</span>${mid}<span class="mup">${fmtUp(g.secs)}</span>${acts}</div>`;
  }

  // Heavy apps get a big card: the number you care about (RAM) large, and a Quit you can't miss.
  function card(g: MemGroup) {
    const total = data ? data.total / 2 ** 20 : 1;
    const level = g.mb >= 1024 ? 'red' : 'amber';
    const icon = g.kind === 'Dev' || g.orphan ? termIcon : appIcon(g.name);
    const sub = [g.count > 1 ? `${g.count} processes` : g.kind, `CPU ${Math.round(g.cpu)}%`, `up ${fmtUp(g.secs)}`].join(' · ');
    return `<div class="hcard ${level} ${selected === g.id ? 'sel' : ''}" data-mrow="${esc(g.id)}">
      <span class="hic">${icon}</span>
      <span class="hmain"><b>${esc(g.name)}</b><span>${esc(sub)}</span></span>
      <span class="hmem"><b>${fmtMem(g.mb)}</b><span>${Math.max(1, Math.round((g.mb / total) * 100))}% of RAM</span></span>
      <span class="hbtns"><button class="bigquit" data-mquit="${esc(g.id)}">Quit</button><span class="menu-wrap"><button class="bigchev" data-mmenu="${esc(g.id)}" aria-label="More">⌄</button>${menuFor === g.id ? `<div class="popover"><button class="danger" data-mforce="${esc(g.id)}">Force Quit</button></div>` : ''}</span></span>
    </div>`;
  }

  function render() {
    const host = el();
    if (!data) {
      host.innerHTML = '<div class="empty">Reading memory…</div>';
      return;
    }
    const all0 = list();
    const heavy = filter === 'high' || filter === 'all' ? all0.filter((g) => !g.system && g.mb >= HIGH) : [];
    const rows = all0.filter((g) => !heavy.includes(g));
    const max = Math.max(1, ...rows.map((g) => g.mb));
    const heavyMb = heavy.reduce((a, g) => a + g.mb, 0);
    const hero = heavy.length
      ? `<div class="hero-h"><b>Heavy</b><span class="faint">${heavy.length} app${heavy.length > 1 ? 's' : ''} · ${fmtMem(heavyMb)}</span><span class="grow"></span>${heavy.length > 1 ? `<button class="bigquit ghost" data-mheavyall>Free ${fmtMem(heavyMb)}</button>` : ''}</div><div class="hgrid">${heavy.map(card).join('')}</div>`
      : '';
    const quittable = rows.filter((g) => !g.system);
    const all = quittable.length > 0 && quittable.every((g) => checked.has(g.id));
    const head = filter === 'orphans'
      ? `<span><input type="checkbox" data-mall ${all ? 'checked' : ''} aria-label="Select all"></span><span></span><span>Name</span><span>Left by</span><span class="port">Port</span><span>Mem</span><span class="mup">Up</span><span></span>`
      : `<span><input type="checkbox" data-mall ${all ? 'checked' : ''} aria-label="Select all"></span><span></span><span>Name</span><span class="kind">Kind</span><span>Mem</span><span class="cpu">CPU</span><span class="mup">Up</span><span></span>`;
    const body = rows.length
      ? rows.map((g) => row(g, max)).join('')
      : heavy.length ? '' : `<div class="empty">${filter === 'orphans' ? 'No orphans. Your Mac is tidy.' : 'Nothing here.'}</div>`;
    const table = rows.length || !heavy.length ? `<div class="table-head ${filter === 'orphans' ? 'orph-grid' : 'mem-grid'}" role="row">${head}</div>${body}` : '';
    const picked = [...checked].map(byId).filter((g): g is MemGroup => !!g);
    const bar = picked.length
      ? `<div class="mbarbot"><span class="faint">${picked.length} ${filter === 'orphans' ? (picked.length > 1 ? 'orphans' : 'orphan') : 'selected'} · ${fmtMem(picked.reduce((a, g) => a + g.mb, 0))}</span><span class="grow"></span><button class="primary" data-mquitsel>${filter === 'orphans' ? 'Quit all' : `Quit ${picked.length}`}</button></div>`
      : '';
    host.innerHTML = `<div class="memtab">${summary()}${filters()}${hero}${table}${bar}</div>`;
    renderConfirm();
  }

  function renderConfirm() {
    document.getElementById('memPop')?.remove();
    if (!confirm) return;
    const gs = confirm.ids.map(byId).filter((g): g is MemGroup => !!g);
    if (!gs.length) { confirm = null; return; }
    const anchor = document.querySelector<HTMLElement>(confirm.anchor);
    const mb = gs.reduce((a, g) => a + g.mb, 0), n = gs.reduce((a, g) => a + g.count, 0);
    const title = gs.length === 1 ? `Quit ${esc(gs[0].name)}?` : `Quit ${gs.length} items?`;
    const pop = document.createElement('div');
    pop.id = 'memPop';
    pop.className = 'mconfirm';
    pop.innerHTML = `<b>${title}</b><span class="faint">Frees about ${fmtMem(mb)}${n > 1 ? ` · ${n} processes` : ''}</span><div class="mf"><button data-mcancel>Cancel</button><button class="danger-text" data-mdo="force">Force Quit</button><button class="primary" data-mdo="quit" autofocus>Quit</button></div>`;
    document.body.appendChild(pop);
    const r = anchor?.getBoundingClientRect();
    const w = pop.offsetWidth, h = pop.offsetHeight;
    const left = r ? Math.min(innerWidth - w - 8, Math.max(8, r.right - w)) : (innerWidth - w) / 2;
    const top = r ? (r.bottom + h + 8 > innerHeight ? r.top - h - 6 : r.bottom + 6) : (innerHeight - h) / 2;
    pop.style.left = `${left}px`;
    pop.style.top = `${top}px`;
    pop.querySelector<HTMLElement>('[data-mdo="quit"]')?.focus();
  }

  async function quit(ids: string[], force: boolean) {
    const gs = ids.map(byId).filter((g): g is MemGroup => !!g && !g.system);
    confirm = null;
    menuFor = null;
    renderConfirm();
    if (!gs.length) return;
    try {
      const r = await call<{ quit: number; refused: number[] }>('quit', { pids: gs.flatMap((g) => g.pids), force });
      if (r.refused.length) d.toast('Some processes could not be quit', true);
      else d.toast(gs.length === 1 ? `${force ? 'Force quit' : 'Quit'} ${gs[0].name}` : `Quit ${gs.length} items`);
    } catch (e: any) {
      d.toast(e.message, true);
    }
    for (const g of gs) checked.delete(g.id);
    setTimeout(load, 400);
    setTimeout(load, 2500);
  }

  // Capture phase so the Localhost/Projects handlers never see Memory clicks.
  document.addEventListener('click', (ev) => {
    if (!d.active()) return;
    const t = ev.target as HTMLElement;
    const hit = t.closest<HTMLElement>('[data-mfilter],[data-msys],[data-mcheck],[data-mall],[data-mquit],[data-mmenu],[data-mforce],[data-mquitsel],[data-mheavyall],[data-mcancel],[data-mdo],[data-mrow],#memPop');
    if (!hit) {
      if (confirm || menuFor) { confirm = null; menuFor = null; render(); }
      return;
    }
    ev.stopPropagation();
    const ds = hit.dataset;
    if (hit.id === 'memPop') return;
    if (ds.mfilter) { filter = ds.mfilter as Filter; d.set('hub.memFilter', filter); checked.clear(); confirm = null; if (filter === 'orphans') list().forEach((g) => !g.system && checked.add(g.id)); }
    else if (ds.msys != null) { hideSystem = (hit as HTMLInputElement).checked; d.set('hub.memSys', hideSystem ? '1' : '0'); }
    else if (ds.mcheck) { const on = (hit as HTMLInputElement).checked; if (on) checked.add(ds.mcheck); else checked.delete(ds.mcheck); }
    else if (ds.mall != null) { const on = (hit as HTMLInputElement).checked; list().filter((g) => !g.system).forEach((g) => (on ? checked.add(g.id) : checked.delete(g.id))); }
    else if (ds.mquit) { selected = ds.mquit; confirm = { ids: [ds.mquit], anchor: `[data-mquit="${CSS.escape(ds.mquit)}"]` }; menuFor = null; }
    else if (ds.mmenu) { selected = ds.mmenu; menuFor = menuFor === ds.mmenu ? null : ds.mmenu; confirm = null; }
    else if (ds.mforce) { return void quit([ds.mforce], true); }
    else if (ds.mquitsel != null) { confirm = { ids: [...checked], anchor: '[data-mquitsel]' }; }
    else if (ds.mheavyall != null) { confirm = { ids: list().filter((g) => !g.system && g.mb >= HIGH).map((g) => g.id), anchor: '[data-mheavyall]' }; }
    else if (ds.mcancel != null) { confirm = null; }
    else if (ds.mdo) { return void quit(confirm?.ids ?? [], ds.mdo === 'force'); }
    else if (ds.mrow) { selected = selected === ds.mrow ? selected : ds.mrow; menuFor = null; confirm = null; }
    render();
  }, true);

  document.addEventListener('keydown', (ev) => {
    if (!d.active() || !confirm) return;
    if (ev.key === 'Escape') { ev.stopImmediatePropagation(); confirm = null; renderConfirm(); }
    else if (ev.key === 'Enter') { ev.preventDefault(); ev.stopImmediatePropagation(); quit(confirm.ids, false); }
  }, true);

  (function loop() {
    load();
    setTimeout(loop, d.active() && !document.hidden ? 3000 : 15000);
  })();

  return {
    render,
    load,
    leave() { confirm = null; menuFor = null; renderConfirm(); },
  };
}
