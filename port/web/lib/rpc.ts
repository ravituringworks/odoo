// Transport port: Tauri IPC when embedded, HTTP otherwise. Everything above this file is transport-agnostic.
export type Json = null | boolean | number | string | Json[] | { [k: string]: Json }
export type RpcRequest = { method: string; model?: string; args?: unknown[]; kwargs?: Record<string, unknown>; uid?: number; db?: string | null; token?: string | null }
export class RpcError extends Error { constructor(message: string, readonly kind?: string) { super(message) } }

const API = process.env.NEXT_PUBLIC_ODOO_API ?? 'http://127.0.0.1:8069'
type TauriWindow = Window & { __TAURI_INTERNALS__?: { invoke: (cmd: string, args: unknown) => Promise<unknown> } }

const LANG_MAP: Record<string, string> = { en: 'en_US', es: 'es', fr: 'fr', de: 'de', pt: 'pt_BR', it: 'it', ru: 'ru', hi: 'hi', ar: 'ar', zh: 'zh_CN', ja: 'ja' }
const currentOdooLang = (): string => { try { return LANG_MAP[localStorage.getItem('odoo-rs-lang') || (navigator.language || 'en').split('-')[0]] ?? 'en_US' } catch { return 'en_US' } }
const TOKEN_KEY = 'odoo_token'
const DB_KEY = 'odoo_db'
/** Trial database this browser works in (absent = the main database). */
export const getDb = (): string | null => { try { return localStorage.getItem(DB_KEY) } catch { return null } }
export const setDb = (d: string | null) => { try { d ? localStorage.setItem(DB_KEY, d) : localStorage.removeItem(DB_KEY) } catch { /* ignore */ } }
export const getToken = (): string | null => { try { return localStorage.getItem(TOKEN_KEY) } catch { return null } }
export const setToken = (t: string | null) => { try { t ? localStorage.setItem(TOKEN_KEY, t) : localStorage.removeItem(TOKEN_KEY) } catch {} }

export async function rpc<T = unknown>(req0: RpcRequest): Promise<T> {
  const req = { ...req0, token: req0.token !== undefined ? req0.token : getToken(), lang: currentOdooLang(), db: req0.db === undefined ? getDb() : req0.db }
  const tauri = typeof window !== 'undefined' ? (window as TauriWindow).__TAURI_INTERNALS__ : undefined
  if (tauri) {
    try { return (await tauri.invoke('rpc', { req })) as T } catch (e) { throw new RpcError(String(e)) }
  }
  const res = await fetch(`${API}/api/rpc`, { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify(req) })
  const body = (await res.json()) as { result?: T; error?: { message: string; kind?: string } }
  if (body.error?.kind === 'AccessDenied' && /session/.test(body.error.message) && typeof window !== 'undefined' && req0.token === undefined) { setToken(null); window.dispatchEvent(new Event('odoo-auth')) }
  if (body.error) throw new RpcError(body.error.message, body.error.kind)
  return body.result as T
}

export const call = <T = unknown>(model: string, method: string, args: unknown[] = [], kwargs: Record<string, unknown> = {}) =>
  rpc<T>({ method, model, args, kwargs })
