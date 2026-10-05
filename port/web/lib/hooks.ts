'use client'
import { useCallback, useEffect, useState } from 'react'
import { call, rpc } from './rpc'
import type { Fields } from './model'

export type Menu = { id: string; name: string; sequence: number; model: string | null; view_mode: string | null; domain?: unknown[] | null; context?: Record<string, unknown> | null; children: Menu[] }

type Async<T> = { data?: T; error?: string; loading: boolean }
export function useAsync<T>(fn: () => Promise<T>, deps: unknown[]): Async<T> & { reload: () => void } {
  const [s, set] = useState<Async<T>>({ loading: true })
  const [tick, setTick] = useState(0)
  useEffect(() => {
    let live = true
    set((p) => ({ ...p, loading: true, error: undefined }))
    fn().then((data) => live && set({ data, loading: false })).catch((e) => live && set({ error: String(e.message ?? e), loading: false }))
    return () => { live = false }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [...deps, tick])
  return { ...s, reload: useCallback(() => setTick((t) => t + 1), []) }
}

const cache = new Map<string, Promise<unknown>>()
const memo = <T,>(key: string, f: () => Promise<T>) => (cache.has(key) ? (cache.get(key) as Promise<T>) : (cache.set(key, f()), cache.get(key) as Promise<T>))
export const useFields = (model: string) => useAsync(() => memo(`f:${model}:${typeof localStorage !== 'undefined' ? localStorage.getItem('odoo-rs-lang') + (localStorage.getItem('odoo_db') ?? '') : ''}`, () => call<Fields>(model, 'fields_get')), [model])
export const useMenus = () => useAsync(() => memo(`menus:${typeof localStorage !== 'undefined' ? localStorage.getItem('odoo-rs-lang') + (localStorage.getItem('odoo_db') ?? '') : ''}`, () => rpc<Menu[]>({ method: 'menus' })), [])
