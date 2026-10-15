// Every call goes to hub_core::api::App::call(cmd, args).
// Inside the app: the Tauri `api` command. In a browser (npm run dev + npm run bridge): the HTTP bridge on :4410.
import { invoke } from '@tauri-apps/api/core';

// Lemon Squeezy checkout (RAM Rescue Pro, $9.99, license key for 2 Macs)
export const BUY_URL = 'https://icecraft.lemonsqueezy.com/checkout/buy/54ffb226-9b3c-42bf-9860-4e0eb794f50f';

export const isTauri = '__TAURI_INTERNALS__' in window;
const BRIDGE = 'http://127.0.0.1:4410/api/';

export async function call<T = any>(cmd: string, args: Record<string, unknown> = {}): Promise<T> {
  if (isTauri) return invoke<T>('api', { cmd, args });
  const r = await fetch(BRIDGE + cmd, { method: 'POST', body: JSON.stringify(args) });
  const j = await r.json();
  if (!r.ok) throw new Error(j.error || r.statusText);
  return j as T;
}

export type Status = 'running' | 'starting' | 'stopped' | 'error' | 'blocked';
export interface Row {
  id: string; group: string; repo: string; project: string; name: string; kind: string; cwd: string; cwdFull: string; command: string; canStart: boolean;
  port: number | null; ports: number[]; url: string | null; status: Status; note: string; own: boolean; pid: number | null;
  mb: number | null; secs: number | null; source: string; sharedPort: string[]; file: string; health: Health | null;
}
export type Health = 'healthy' | 'unhealthy' | 'tcp';
export interface Port {
  port: number; pid: number; name: string; tool: string; cwd: string; args: string; mb: number; secs: number; source: string;
  health: Health; code: number | null; ms: number | null; rowId: string | null;
  /** open = other devices on the LAN can already reach it · shared = through Server Hub's relay · null = this Mac only */
  lan: 'open' | 'shared' | null; lanUrl: string | null;
  /** its terminal / editor / agent is gone (parent is launchd) */
  orphan: boolean; leftBy: string | null;
}
export interface MemGroup {
  id: string; name: string; kind: 'App' | 'Background' | 'Dev' | 'System'; pids: number[]; count: number; mb: number;
  leakMbH: number | null; cpu: number; secs: number; ports: number[]; system: boolean; orphan: boolean; leftBy: string | null;
}
export interface MemSnap { focus?: string | null; total: number; used: number; pressure: 'normal' | 'warning' | 'critical'; groups: MemGroup[] }
export interface Outside { pid: number; name: string; cwd: string; args: string; ports: number[]; mb: number; secs: number; source: string }
export interface Snapshot { root: string; roots: { path: string; exists: boolean }[]; rootExists: boolean; entries: Row[]; outside: Outside[]; ports: Port[]; unhealthy: number; running: number; lanIp: string | null }

export interface License { state: 'trial' | 'licensed' | 'expired'; daysLeft: number; key: string }
export interface GuardStatus {
  rules: Rule[]; meetingApps: AppChoice[]; meetingOn: boolean; meetingQuit: (AppChoice & {path: string})[];
  license: License; on: boolean; autoOrphans: boolean; orphanMins: number; running: boolean;
  log: { t: number; what: 'warned' | 'leaking' | 'quit' | 'idle' | 'meeting'; name: string; mb: number; mins?: number; count?: number }[];
}

export interface AppChoice { id: string; name: string }
export interface Rule extends AppChoice { mins: number; always: boolean }
export interface CleanItem { id: string; name: string; group: 'Developer' | 'Browsers'; note: string; path: string; mb: number | null; readable: boolean; blockedBy: string | null }
export interface CleanResult { ok: boolean; freedMb: number; done: string[]; skipped: {id: string; why: string}[] }
export interface HistoryPoint { t: number; usedMb: number; apps: [string, number][] }
export interface HistoryData { pro: boolean; points: HistoryPoint[] }
