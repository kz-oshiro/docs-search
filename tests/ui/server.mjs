import http from 'node:http';
import fs from 'node:fs/promises';
import path from 'node:path';

if (!process.env.DOCS_SEARCH_UI_SITE) throw new Error('DOCS_SEARCH_UI_SITE is required.');
const root = await fs.realpath(process.env.DOCS_SEARCH_UI_SITE);
await fs.access(path.join(root, 'pkg', 'docs_search_ui_bg.wasm'));
const types = { '.html': 'text/html; charset=utf-8', '.js': 'text/javascript; charset=utf-8', '.css': 'text/css; charset=utf-8', '.wasm': 'application/wasm' };
http.createServer(async (request, response) => {
  if (!['GET', 'HEAD'].includes(request.method)) { response.writeHead(405).end(); return; }
  try {
    const pathname = decodeURIComponent(new URL(request.url, 'http://127.0.0.1').pathname);
    if (pathname === '/favicon.ico') { response.writeHead(204).end(); return; }
    if (pathname === '/__health') { response.writeHead(200, { 'Content-Type': 'text/plain' }).end('ready'); return; }
    const relative = pathname === '/' ? 'index.html' : pathname.slice(1);
    const target = await fs.realpath(path.resolve(root, relative));
    if (!target.startsWith(root + path.sep)) { response.writeHead(403).end(); return; }
    const data = await fs.readFile(target);
    response.writeHead(200, { 'Content-Type': types[path.extname(target)] ?? 'application/octet-stream', 'Cache-Control': 'no-store' });
    response.end(request.method === 'HEAD' ? undefined : data);
  } catch { response.writeHead(404).end('not found'); }
}).listen(43861, '127.0.0.1');
