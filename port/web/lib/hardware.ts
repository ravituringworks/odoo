// Browser-side device access for the POS: Web Serial (USB/serial receipt printers, cash drawers, scales). Settings live per browser.
import { parseScale } from './escpos'

export type HwSettings = { receipt: 'browser' | 'network' | 'epos' | 'serial'; autoPrint: boolean; drawer: boolean; kitchen: boolean; scale: boolean }
export const DEFAULT_HW: HwSettings = { receipt: 'browser', autoPrint: false, drawer: false, kitchen: true, scale: false }
const KEY = 'pos_hw'
export const loadHw = (): HwSettings => { try { return { ...DEFAULT_HW, ...(JSON.parse(localStorage.getItem(KEY) ?? '{}') as Partial<HwSettings>) } } catch { return DEFAULT_HW } }
export const saveHw = (s: HwSettings) => { try { localStorage.setItem(KEY, JSON.stringify(s)) } catch { /* private mode */ } }

// minimal Web Serial typing (not part of lib.dom)
type SerialPortLike = { open(o: { baudRate: number }): Promise<void>; close(): Promise<void>; writable: WritableStream<Uint8Array> | null; readable: ReadableStream<Uint8Array> | null }
type SerialApi = { requestPort(): Promise<SerialPortLike>; getPorts(): Promise<SerialPortLike[]> }
const api = (): SerialApi | null => (typeof navigator !== 'undefined' && (navigator as unknown as { serial?: SerialApi }).serial) || null
export const serialSupported = () => api() !== null

const ports: Partial<Record<'receipt' | 'scale', SerialPortLike>> = {}
/** Ask the user to pick a device (must run from a click). Remembered for the page's lifetime. */
export async function connectSerial(kind: 'receipt' | 'scale', baudRate = 9600): Promise<void> {
  const s = api(); if (!s) throw new Error('This browser has no Web Serial support (use Chrome or Edge).')
  const p = await s.requestPort(); await p.open({ baudRate }); ports[kind] = p
}
export const serialConnected = (kind: 'receipt' | 'scale') => !!ports[kind]
export async function writeSerial(bytes: Uint8Array): Promise<void> {
  const p = ports.receipt; if (!p?.writable) throw new Error('No printer connected — connect one in Hardware settings.')
  const w = p.writable.getWriter(); try { await w.write(bytes) } finally { w.releaseLock() }
}
/** Read the scale until a stable non-zero reading arrives (or time runs out). */
export async function readWeight(timeoutMs = 8000): Promise<number> {
  const p = ports.scale; if (!p?.readable) throw new Error('No scale connected — connect one in Hardware settings.')
  const r = p.readable.getReader(); const dec = new TextDecoder(); let buf = ''; const end = Date.now() + timeoutMs
  try {
    while (Date.now() < end) {
      const { value, done } = await r.read(); if (done) break
      buf += dec.decode(value); const lines = buf.split(/[\r\n]+/); buf = lines.pop() ?? ''
      for (const l of lines.reverse()) { const w = parseScale(l); if (w && w.stable && w.kg > 0) return w.kg }
    }
  } finally { r.releaseLock() }
  throw new Error('No stable weight — put the item on the scale and try again.')
}
