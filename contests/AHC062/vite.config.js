import fs from "node:fs";
import path from "node:path";
import { spawn } from "node:child_process";
import { fileURLToPath } from "node:url";
import { defineConfig } from "vite";

const ROOT_DIR = fileURLToPath(new URL(".", import.meta.url));
const SRC_BIN_DIR = path.join(ROOT_DIR, "src", "bin");
const MANIFEST_PATH = path.join(ROOT_DIR, "Cargo.toml");
const TOOLS_IN_DIR = path.join(ROOT_DIR, "tools", "in");
const guardedSockets = new WeakSet();
const guardedResponses = new WeakSet();

function isBenignSocketError(err) {
  const code = err?.code;
  return code === "EPIPE" || code === "ECONNRESET";
}

function attachResponseErrorGuards(res) {
  if (!guardedResponses.has(res)) {
    res.on("error", (err) => {
      if (!isBenignSocketError(err)) {
        console.error("[api] response error:", err);
      }
    });
    guardedResponses.add(res);
  }
  const sock = res.socket;
  if (sock && !guardedSockets.has(sock)) {
    sock.on("error", (err) => {
      if (!isBenignSocketError(err)) {
        console.error("[api] socket error:", err);
      }
    });
    guardedSockets.add(sock);
  }
}

function sendJson(res, status, payload) {
  attachResponseErrorGuards(res);
  if (res.destroyed || res.writableEnded) {
    return;
  }
  res.statusCode = status;
  res.setHeader("Content-Type", "application/json; charset=utf-8");
  res.end(JSON.stringify(payload));
}

function readBody(req) {
  return new Promise((resolve, reject) => {
    let raw = "";
    req.on("data", (chunk) => {
      raw += chunk;
      if (raw.length > 10 * 1024 * 1024) {
        reject(new Error("Request body is too large"));
      }
    });
    req.on("end", () => resolve(raw));
    req.on("error", reject);
  });
}

function listRustBins() {
  if (!fs.existsSync(SRC_BIN_DIR)) {
    return [];
  }
  return fs
    .readdirSync(SRC_BIN_DIR, { withFileTypes: true })
    .filter((ent) => ent.isFile() && ent.name.endsWith(".rs"))
    .map((ent) => ent.name.slice(0, -3))
    .sort();
}

function listCaseFiles() {
  if (!fs.existsSync(TOOLS_IN_DIR)) {
    return [];
  }
  return fs
    .readdirSync(TOOLS_IN_DIR, { withFileTypes: true })
    .filter((ent) => ent.isFile() && ent.name.endsWith(".txt"))
    .map((ent) => ent.name)
    .sort();
}

function readCaseInput(caseName) {
  if (typeof caseName !== "string" || caseName.trim().length === 0) {
    throw new Error("case name is required");
  }
  const normalized = caseName.trim();
  if (!/^[0-9A-Za-z._-]+\.txt$/.test(normalized)) {
    throw new Error("invalid case name");
  }
  const available = listCaseFiles();
  if (!available.includes(normalized)) {
    throw new Error(`case '${normalized}' is not found under tools/in`);
  }
  const p = path.join(TOOLS_IN_DIR, normalized);
  return fs.readFileSync(p, "utf-8");
}

function runRustBin(binName, inputText) {
  return new Promise((resolve, reject) => {
    const child = spawn(
      "cargo",
      [
        "run",
        "--release",
        "--quiet",
        "--manifest-path",
        MANIFEST_PATH,
        "--bin",
        binName,
      ],
      { cwd: ROOT_DIR, stdio: ["pipe", "pipe", "pipe"] },
    );

    let stdout = "";
    let stderr = "";
    let settled = false;

    const finishResolve = (value) => {
      if (settled) return;
      settled = true;
      clearTimeout(timer);
      resolve(value);
    };

    const finishReject = (err) => {
      if (settled) return;
      settled = true;
      clearTimeout(timer);
      reject(err);
    };

    const timer = setTimeout(() => {
      child.kill("SIGKILL");
      finishReject(new Error("実行がタイムアウトした (120秒)"));
    }, 120_000);

    child.stdout.on("data", (d) => {
      stdout += d.toString();
    });
    child.stderr.on("data", (d) => {
      stderr += d.toString();
    });

    child.on("error", (e) => {
      finishReject(e);
    });

    child.stdin.on("error", (e) => {
      if (isBenignSocketError(e)) {
        return;
      }
      finishReject(e);
    });

    child.on("close", (code) => {
      if (settled) {
        return;
      }
      if (code === 0) {
        finishResolve({ stdout, stderr });
      } else {
        const errMsg = stderr.trim() || `exit code ${code}`;
        finishReject(new Error(errMsg));
      }
    });

    child.stdin.end(inputText ?? "");
  });
}

function rustBinApiPlugin() {
  return {
    name: "rust-bin-api",
    configureServer(server) {
      server.httpServer?.on("clientError", (err, socket) => {
        if (isBenignSocketError(err)) {
          socket.destroy();
          return;
        }
        console.error("[vite] clientError:", err);
      });

      server.middlewares.use("/api/rust-bins", (req, res, next) => {
        if (req.method !== "GET") {
          next();
          return;
        }
        try {
          const bins = listRustBins();
          sendJson(res, 200, { bins });
        } catch (e) {
          sendJson(res, 500, { error: String(e) });
        }
      });

      server.middlewares.use("/api/tool-cases", (req, res, next) => {
        if (req.method !== "GET") {
          next();
          return;
        }
        try {
          const cases = listCaseFiles();
          sendJson(res, 200, { cases });
        } catch (e) {
          sendJson(res, 500, { error: String(e) });
        }
      });

      server.middlewares.use("/api/tool-case-input", (req, res, next) => {
        if (req.method !== "GET") {
          next();
          return;
        }
        try {
          const u = new URL(req.url || "", "http://localhost");
          const name = u.searchParams.get("name") || "";
          const input = readCaseInput(name);
          sendJson(res, 200, { name, input });
        } catch (e) {
          sendJson(res, 400, { error: String(e) });
        }
      });

      server.middlewares.use("/api/run-rust-bin", async (req, res, next) => {
        if (req.method !== "POST") {
          next();
          return;
        }
        try {
          const raw = await readBody(req);
          const body = raw ? JSON.parse(raw) : {};
          const binName =
            typeof body.binName === "string" ? body.binName.trim() : "";
          const inputText = typeof body.input === "string" ? body.input : "";

          const bins = listRustBins();
          if (!bins.includes(binName)) {
            sendJson(res, 400, {
              error: `bin '${binName}' は src/bin/*.rs に存在しない`,
            });
            return;
          }

          const startedAt = Date.now();
          const result = await runRustBin(binName, inputText);
          sendJson(res, 200, {
            output: result.stdout,
            stderr: result.stderr,
            elapsedMs: Date.now() - startedAt,
          });
        } catch (e) {
          sendJson(res, 500, { error: String(e) });
        }
      });
    },
  };
}

export default defineConfig({
  plugins: [rustBinApiPlugin()],
});
