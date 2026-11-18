// Serveur statique minimal, sans dépendance.
// Usage : node scripts/serve.mjs [port]
import { createServer } from "node:http";
import { readFile, stat } from "node:fs/promises";
import { extname, join, normalize, resolve } from "node:path";

const root = resolve(process.cwd());
const port = Number(process.argv[2] ?? 4173);

const types = new Map(Object.entries({
  ".html": "text/html; charset=utf-8",
  ".js": "text/javascript; charset=utf-8",
  ".mjs": "text/javascript; charset=utf-8",
  ".css": "text/css; charset=utf-8",
  ".json": "application/json; charset=utf-8",
  ".wasm": "application/wasm",
  ".svg": "image/svg+xml",
  ".ico": "image/x-icon",
  ".png": "image/png",
}));

createServer(async (request, response) => {
  try {
    const url = new URL(request.url ?? "/", `http://${request.headers.host}`);
    let path = decodeURIComponent(url.pathname);
    if (path === "/") path = "/index.html";

    // Pas de réécriture d'URL : le routage est dans le fragment.
    const target = normalize(join(root, path));
    if (!target.startsWith(root)) {
      response.writeHead(403).end("Interdit");
      return;
    }

    let info;
    try {
      info = await stat(target);
    } catch {
      response.writeHead(404, { "content-type": "text/plain; charset=utf-8" }).end("Introuvable");
      return;
    }
    if (!info.isFile()) {
      response.writeHead(404).end("Introuvable");
      return;
    }

    const body = await readFile(target);
    response.writeHead(200, {
      "content-type": types.get(extname(target).toLowerCase()) ?? "application/octet-stream",
      "cache-control": "no-cache",
      "cross-origin-opener-policy": "same-origin",
      "cross-origin-embedder-policy": "require-corp",
    }).end(body);
  } catch (error) {
    response.writeHead(500).end(String(error));
  }
}).listen(port, () => {
  console.log(`Archifier servi sur http://127.0.0.1:${port}/`);
});
