// Static export so the same UI ships inside Tauri; all data access is client-side via lib/rpc.
/** @type {import('next').NextConfig} */
export default { output: 'export', images: { unoptimized: true }, trailingSlash: true };
