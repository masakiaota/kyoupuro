import { PlayError, PlayStore, loadEngine } from "./play-store.mjs";

const MAX_BODY = 8 * 1024 * 1024;
async function readBody(req) {
  const parts = []; let size = 0;
  for await (const part of req) {
    size += part.length;
    if (size > MAX_BODY) throw new PlayError("body_too_large", "要求が8MiBを超えている", 413);
    parts.push(part);
  }
  let value;
  try { value = JSON.parse(Buffer.concat(parts).toString("utf8") || "{}"); }
  catch { throw new PlayError("invalid_json", "JSONの構文が不正である"); }
  if (!value || Array.isArray(value) || typeof value !== "object") throw new PlayError("invalid_request", "JSONオブジェクトを指定する");
  return value;
}

function query(url) {
  const result = Object.fromEntries(url.searchParams);
  for (const key of ["i", "j", "k", "turn", "expected_revision", "offset", "limit"]) {
    if (key in result) {
      if (!/^-?\d+$/.test(result[key])) throw new PlayError("invalid_query", `${key} は整数で指定する`);
      result[key] = Number(result[key]);
    }
  }
  if ("svg" in result) result.svg = result.svg === "1";
  return result;
}

export function playMiddleware(store, { lock = false } = {}) {
  return async (req, res, next) => {
    const url = new URL(req.url ?? "/", "http://localhost");
    if (!url.pathname.startsWith("/api/play/")) { next(); return; }
    const send = (status, value, raw = false) => {
      res.statusCode = status;
      res.setHeader("Cache-Control", "no-store");
      res.setHeader("Content-Type", raw ? "text/plain; charset=utf-8" : "application/json; charset=utf-8");
      res.end(raw ? value : JSON.stringify(value));
    };
    try {
      // Viteは新サーバーを構成してから旧サーバーを閉じる。要求受付時なら旧側のロックは解放済みになる。
      if (lock && !store.lockFile) store.acquireLock();
      // ローカルの記録操作を、別サイトからのフォーム送信などで実行させない。
      if (req.headers.origin && req.headers.origin !== `http://${req.headers.host}` && req.headers.origin !== `https://${req.headers.host}`) {
        throw new PlayError("origin_mismatch", "同じ起動先の画面から呼び出す", 403);
      }
      const route = url.pathname.slice("/api/play/".length).split("/");
      const options = query(url);
      if (req.method === "GET" && route[0] === "info" && route.length === 1) {
        send(200, { schema_version: 1, cases: store.cases(), engine_hash: store.engineHash, ...store.list() }); return;
      }
      if (route[0] !== "sessions") throw new PlayError("not_found", "APIが見つからない", 404);
      if (route.length === 1) {
        if (req.method === "GET") { send(200, store.list()); return; }
        if (req.method === "POST") { send(201, store.create(await readBody(req))); return; }
      }
      const [, id, command] = route;
      if (route.length === 3 && req.method === "GET") {
        if (command === "export") {
          send(200, store.export(id, options), options.format === "output"); return;
        }
        if (["state", "version", "legal", "history"].includes(command)) {
          send(200, store[command](id, options)); return;
        }
      }
      if (route.length === 3 && req.method === "POST") {
        const body = await readBody(req);
        if (command === "preview") { send(200, store.preview(id, body)); return; }
        if (["step", "checkout", "note"].includes(command)) { send(200, store.mutate(id, command, body)); return; }
      }
      throw new PlayError("not_found", "APIまたはHTTPメソッドが見つからない", 404);
    } catch (error) {
      if (!(error instanceof PlayError)) process.stderr.write(`[play] ${error.stack ?? error}\n`);
      send(error.status ?? 500, { error: { code: error.code ?? "internal_error", message: error.message ?? String(error), ...(error.details ?? {}) } });
    }
  };
}

export function playApiPlugin(root) {
  let store;
  return {
    name: "ahc072-play-api",
    async configureServer(server) {
      store = new PlayStore({ root, ...await loadEngine(root) });
      const cleanup = () => store?.close();
      const onExit = () => cleanup();
      process.once("exit", onExit);
      server.httpServer?.once("close", () => { cleanup(); process.removeListener("exit", onExit); });
      server.middlewares.use(playMiddleware(store, { lock: true }));
    },
  };
}
