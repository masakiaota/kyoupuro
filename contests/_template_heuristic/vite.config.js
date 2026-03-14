import fs from "node:fs";
import path from "node:path";
import { spawn } from "node:child_process";
import { fileURLToPath } from "node:url";
import { defineConfig } from "vite";

const ROOT_DIR = fileURLToPath(new URL(".", import.meta.url));
const SRC_BIN_DIR = path.join(ROOT_DIR, "src", "bin");
const MANIFEST_PATH = path.join(ROOT_DIR, "Cargo.toml");

function sendJson(res, status, payload) {
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

    const timer = setTimeout(() => {
      child.kill("SIGKILL");
      reject(new Error("実行がタイムアウトした (120秒)"));
    }, 120_000);

    child.stdout.on("data", (d) => {
      stdout += d.toString();
    });
    child.stderr.on("data", (d) => {
      stderr += d.toString();
    });

    child.on("error", (e) => {
      clearTimeout(timer);
      reject(e);
    });

    child.on("close", (code) => {
      clearTimeout(timer);
      if (code === 0) {
        resolve({ stdout, stderr });
      } else {
        const errMsg = stderr.trim() || `exit code ${code}`;
        reject(new Error(errMsg));
      }
    });

    child.stdin.write(inputText ?? "");
    child.stdin.end();
  });
}

function rustBinApiPlugin() {
  return {
    name: "rust-bin-api",
    configureServer(server) {
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

