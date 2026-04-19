import fs from "node:fs";
import path from "node:path";
import { spawn } from "node:child_process";
import { fileURLToPath } from "node:url";
import { defineConfig } from "vite";

const ROOT_DIR = fileURLToPath(new URL(".", import.meta.url));
const SRC_BIN_DIR = path.join(ROOT_DIR, "src", "bin");
const MANIFEST_PATH = path.join(ROOT_DIR, "Cargo.toml");
const RESULTS_OUT_DIR = path.join(ROOT_DIR, "results", "out");
const INPUT_DATASET_DIRS = {
  in: path.join(ROOT_DIR, "tools", "in"),
  inB: path.join(ROOT_DIR, "tools", "inB"),
  in_generated: path.join(ROOT_DIR, "tools", "in_generated"),
};

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

function resolveInputDir(dataset) {
  if (typeof dataset !== "string" || !(dataset in INPUT_DATASET_DIRS)) {
    throw new Error(`invalid dataset: ${dataset}`);
  }
  return INPUT_DATASET_DIRS[dataset];
}

function listInputCases(dataset) {
  const inputDir = resolveInputDir(dataset);
  if (!fs.existsSync(inputDir) || !fs.statSync(inputDir).isDirectory()) {
    return { exists: false, cases: [] };
  }
  const cases = fs
    .readdirSync(inputDir, { withFileTypes: true })
    .filter((ent) => ent.isFile())
    .map((ent) => ent.name)
    .sort();
  return { exists: true, cases };
}

function isSafeName(name) {
  return (
    typeof name === "string" &&
    name.length > 0 &&
    path.basename(name) === name &&
    !name.includes("/") &&
    !name.includes("\\")
  );
}

function readVisPair(dataset, binName, caseName) {
  if (!isSafeName(binName)) {
    throw new Error("invalid bin name");
  }
  if (!isSafeName(caseName)) {
    throw new Error("invalid case name");
  }
  if (!listRustBins().includes(binName)) {
    throw new Error(`bin '${binName}' は src/bin/*.rs に存在しない`);
  }

  const inputDir = resolveInputDir(dataset);
  if (!fs.existsSync(inputDir) || !fs.statSync(inputDir).isDirectory()) {
    throw new Error(`input dataset not found: tools/${dataset}`);
  }

  const inputPath = path.join(inputDir, caseName);
  if (!fs.existsSync(inputPath) || !fs.statSync(inputPath).isFile()) {
    throw new Error(`input case not found: tools/${dataset}/${caseName}`);
  }
  const input = fs.readFileSync(inputPath, "utf8");

  let output = "";
  let hasOutput = false;
  const datasetOutputPath = path.join(RESULTS_OUT_DIR, binName, dataset, caseName);
  const legacyOutputPath = path.join(RESULTS_OUT_DIR, binName, caseName);
  let resolvedOutputPath = datasetOutputPath;

  if (fs.existsSync(datasetOutputPath) && fs.statSync(datasetOutputPath).isFile()) {
    output = fs.readFileSync(datasetOutputPath, "utf8");
    hasOutput = true;
  } else if (
    dataset === "in" &&
    fs.existsSync(legacyOutputPath) &&
    fs.statSync(legacyOutputPath).isFile()
  ) {
    output = fs.readFileSync(legacyOutputPath, "utf8");
    hasOutput = true;
    resolvedOutputPath = legacyOutputPath;
  }

  return { input, output, hasOutput, outputPath: resolvedOutputPath };
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

      server.middlewares.use("/api/in-cases", (req, res, next) => {
        if (req.method !== "GET") {
          next();
          return;
        }
        try {
          const reqUrl = new URL(req.url ?? "", "http://localhost");
          const dataset = (reqUrl.searchParams.get("dataset") ?? "in").trim();
          const result = listInputCases(dataset);
          sendJson(res, 200, result);
        } catch (e) {
          sendJson(res, 400, { error: String(e) });
        }
      });

      server.middlewares.use("/api/vis-pair", (req, res, next) => {
        if (req.method !== "GET") {
          next();
          return;
        }
        try {
          const reqUrl = new URL(req.url ?? "", "http://localhost");
          const dataset = (reqUrl.searchParams.get("dataset") ?? "in").trim();
          const binName = (reqUrl.searchParams.get("bin") ?? "").trim();
          const caseName = (reqUrl.searchParams.get("case") ?? "").trim();
          if (!binName || !caseName) {
            sendJson(res, 400, { error: "bin and case are required" });
            return;
          }
          const pair = readVisPair(dataset, binName, caseName);
          sendJson(res, 200, pair);
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
