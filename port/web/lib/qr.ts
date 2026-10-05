import qrcode from './vendor/qrcode'

type QR = { addData(s: string): void; make(): void; getModuleCount(): number; isDark(r: number, c: number): boolean }

/** Self-contained SVG for `text` (error correction M, auto version). Used for table QR codes. */
export function qrSvg(text: string, px = 220): string {
  const q = (qrcode as unknown as (t: number, e: string) => QR)(0, 'M'); q.addData(text); q.make()
  const n = q.getModuleCount(); const quiet = 2; const size = n + quiet * 2
  let path = ''
  for (let r = 0; r < n; r++) for (let c = 0; c < n; c++) if (q.isDark(r, c)) path += `M${c + quiet} ${r + quiet}h1v1h-1z`
  return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${size} ${size}" width="${px}" height="${px}" shape-rendering="crispEdges"><rect width="${size}" height="${size}" fill="#fff"/><path d="${path}" fill="#000"/></svg>`
}
