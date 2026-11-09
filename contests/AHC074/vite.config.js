import fs from "node:fs";
import path from "node:path";
import crypto from "node:crypto";
import { spawn, spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { defineConfig } from "vite";
import { evalRecordsVersion, iterateEvalRecordFile, recordPaths } from "./scripts/eval_records.js";

const ROOT_DIR = fileURLToPath(new URL(".", import.meta.url));
const PROJECT_KEY = crypto
  .createHash("sha256")
  .update(path.resolve(ROOT_DIR))
  .digest("hex")
  .slice(0, 16);
const SRC_BIN_DIR = path.join(ROOT_DIR, "src", "bin");
const ADHOC_BIN_DIR = path.join(ROOT_DIR, "adhoc", "bin");
// AtCoder の提出ソース長の上限 (512 KiB)。
const ATCODER_SOURCE_LIMIT_BYTES = 512 * 1024;
const TOOLS_MANIFEST_PATH = path.join(ROOT_DIR, "tools", "Cargo.toml");
const BUILD_SOLVER_PATH = path.join(ROOT_DIR, "scripts", "build_solver.sh");
const OFFICIAL_EVAL_RECORDS = path.join(ROOT_DIR, "results", "eval_records", "tools", "in.jsonl");
const EVAL_VIEW_DATA_DIR = path.join(ROOT_DIR, "results", "eval_view_data");
const EVAL_VIEW_EXCLUDED_ENTRIES = ["adhoc", "adhoc.json"];
const TOOLS_INPUT_DIR = path.join(ROOT_DIR, "tools", "in");
const RESULTS_OUT_DIR = path.join(ROOT_DIR, "results", "out");
const SOLVER_BIN_DIR = path.join(ROOT_DIR, "target", "release");
const TESTER_BIN_PATH = path.join(ROOT_DIR, "tools", "target", "release", "tester");
const CASE_SORT_OPTIONS = [
  { key: "case_name_asc", label: "case_name ∧" },
  { key: "case_name_desc", label: "case_name ∨" },
];

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

function listSolverBins() {
  const bins = new Set(listRunnableBins());
  if (fs.existsSync(RESULTS_OUT_DIR)) {
    for (const ent of fs.readdirSync(RESULTS_OUT_DIR, { withFileTypes: true })) {
      if (ent.isDirectory()) {
        bins.add(ent.name);
      }
    }
  }
  return Array.from(bins).sort((left, right) => left.localeCompare(right, "ja"));
}

function listRunnableBins() {
  if (!fs.existsSync(SRC_BIN_DIR)) {
    return [];
  }
  return fs
    .readdirSync(SRC_BIN_DIR, { withFileTypes: true })
    .filter(
      (ent) =>
        ent.isFile() &&
        ent.name.endsWith(".cpp") &&
        /^v\d{4}.*\.cpp$/.test(ent.name) &&
        ent.name !== "v0000_template.cpp",
    )
    .map((ent) => ent.name.slice(0, -4))
    .sort((left, right) => left.localeCompare(right, "ja"));
}

function listVisualizerCases() {
  if (!fs.existsSync(TOOLS_INPUT_DIR)) {
    return [];
  }
  return fs
    .readdirSync(TOOLS_INPUT_DIR, { withFileTypes: true })
    .filter((ent) => ent.isFile())
    .map((ent) => ent.name)
    .sort((left, right) => left.localeCompare(right, "ja"));
}

function safeJoinCase(caseName) {
  if (typeof caseName !== "string" || caseName.trim() === "") {
    throw new Error("caseName is required");
  }
  if (path.basename(caseName) !== caseName) {
    throw new Error("invalid caseName");
  }
  const casePath = path.join(TOOLS_INPUT_DIR, caseName);
  if (!fs.existsSync(casePath) || !fs.statSync(casePath).isFile()) {
    throw new Error(`case not found: ${caseName}`);
  }
  return casePath;
}

function safeRelativePath(filePath) {
  return path.relative(ROOT_DIR, filePath).split(path.sep).join("/");
}

function findLatestElapsedMs(binName, caseName) {
  if (!binName || !caseName || !fs.existsSync(OFFICIAL_EVAL_RECORDS)) {
    return null;
  }
  const evalSet = safeRelativePath(TOOLS_INPUT_DIR);
  let latest = null;
  try {
    for (const record of iterateEvalRecordFile(OFFICIAL_EVAL_RECORDS)) {
      if (
        record &&
        typeof record === "object" &&
        record.bin === binName &&
        record.case_name === caseName &&
        record.input_dir === evalSet &&
        typeof record.elapsed === "number"
      ) {
        if (
          !latest ||
          String(record.executed_at ?? "").localeCompare(String(latest.executed_at ?? "")) > 0
        ) {
          latest = record;
        }
      }
    }
  } catch {
    return null;
  }
  return latest ? latest.elapsed : null;
}

function findLatestExecutedAt(binName) {
  let latest = "";
  for (const snapshot of readEvalViewSnapshots()) {
    const executedAt = snapshot.latestExecutedAtByBin[binName];
    if (typeof executedAt === "string" && executedAt.localeCompare(latest) > 0) {
      latest = executedAt;
    }
  }
  return latest;
}

// scripts/build_solver.sh と同じ優先順位 (src/bin → adhoc/bin) でソースを解決する。
function resolveSolverSource(binName) {
  if (typeof binName !== "string" || !/^[A-Za-z0-9_.-]+$/.test(binName)) {
    throw new Error("invalid binName");
  }
  for (const dir of [SRC_BIN_DIR, ADHOC_BIN_DIR]) {
    const sourcePath = path.join(dir, `${binName}.cpp`);
    if (fs.existsSync(sourcePath) && fs.statSync(sourcePath).isFile()) {
      return sourcePath;
    }
  }
  return null;
}

function buildSolverSource(binName) {
  const sourcePath = resolveSolverSource(binName);
  if (!sourcePath) {
    return null;
  }
  const source = fs.readFileSync(sourcePath, "utf-8");
  const stat = fs.statSync(sourcePath);
  const bytes = Buffer.byteLength(source, "utf-8");
  const warnings = [];
  if (bytes > ATCODER_SOURCE_LIMIT_BYTES) {
    warnings.push(`ソース長 ${bytes} bytes が AtCoder の上限 512 KiB を超えている`);
  }
  // 単一ファイル提出なので、ローカルヘッダの include は AtCoder 上でコンパイルできない。
  const localIncludes = source.match(/^\s*#\s*include\s*"[^"]+"/gm) ?? [];
  if (localIncludes.length > 0) {
    warnings.push(`ローカル include がある: ${localIncludes.map((s) => s.trim()).join(", ")}`);
  }
  const latestExecutedAt = findLatestExecutedAt(binName);
  if (latestExecutedAt && stat.mtimeMs > Date.parse(latestExecutedAt)) {
    warnings.push(`最新評価 (${latestExecutedAt}) の後にソースが更新されている`);
  }
  return {
    binName,
    path: safeRelativePath(sourcePath),
    source,
    bytes,
    lines: source.split("\n").length - (source.endsWith("\n") ? 1 : 0),
    mtime: new Date(stat.mtimeMs).toISOString(),
    warnings,
  };
}

function buildTool(binName) {
  const result = spawnSync("cargo", [
    "build",
    "--release",
    "--quiet",
    "--manifest-path",
    TOOLS_MANIFEST_PATH,
    "--bin",
    binName,
  ], {
    cwd: ROOT_DIR,
    encoding: "utf-8",
  });
  if (result.status !== 0) {
    throw new Error(result.stderr?.trim() || `build failed: ${binName}`);
  }
}

function buildSolver(binName) {
  const result = spawnSync(BUILD_SOLVER_PATH, [binName], {
    cwd: ROOT_DIR,
    encoding: "utf-8",
  });
  if (result.status !== 0) {
    throw new Error(result.stderr?.trim() || `build failed: ${binName}`);
  }
}

function parseCaseMeta(filePath) {
  const header = fs.readFileSync(filePath, "utf-8").trim().split(/\r?\n/, 1)[0];
  const sizes = header.split(/\s+/).map(Number);
  if (sizes.length !== 4 || sizes.some((v) => !Number.isInteger(v) || v <= 0)) {
    throw new Error(`入力の N K P R が不正: ${filePath}`);
  }
  const [N, K, P, R] = sizes;
  // 公式ケースでは全サイズが固定なので、並び替えはケース名だけを使う。
  return { N, K, P, R };
}

function resolveInputDir(inputDir) {
  if (typeof inputDir !== "string" || inputDir.trim() === "") {
    return null;
  }
  return path.isAbsolute(inputDir) ? inputDir : path.join(ROOT_DIR, inputDir);
}

function buildCaseMetaByEvalSet(inputDir, caseNames) {
  const resolvedInputDir = resolveInputDir(inputDir);
  if (!resolvedInputDir || !fs.existsSync(resolvedInputDir)) {
    return {};
  }
  const meta = {};
  for (const caseName of caseNames) {
    const filePath = path.join(resolvedInputDir, caseName);
    if (!fs.existsSync(filePath) || !fs.statSync(filePath).isFile()) {
      continue;
    }
    const parsed = parseCaseMeta(filePath);
    if (parsed) {
      meta[caseName] = parsed;
    }
  }
  return meta;
}

function solverBinApiPlugin() {
  return {
    name: "solver-bin-api",
    configureServer(server) {
      server.middlewares.use("/api/visualizer-data", (req, res, next) => {
        if (req.method !== "GET") {
          next();
          return;
        }
        try {
          sendJson(res, 200, {
            projectKey: PROJECT_KEY,
            bins: listSolverBins(),
            runnableBins: listRunnableBins(),
            cases: listVisualizerCases(),
          });
        } catch (e) {
          sendJson(res, 500, { error: String(e) });
        }
      });

      server.middlewares.use("/api/visualizer-case", (req, res, next) => {
        if (req.method !== "GET") {
          next();
          return;
        }
        try {
          const requestUrl = new URL(req.url ?? "", "http://localhost");
          const caseName = requestUrl.searchParams.get("caseName") ?? "";
          const binName = (requestUrl.searchParams.get("binName") ?? "").trim();
          const casePath = safeJoinCase(caseName);
          const input = fs.readFileSync(casePath, "utf-8");

          let output = "";
          let outputExists = false;
          let elapsedMs = null;
          if (binName && path.basename(binName) === binName) {
            const outputPath = path.join(RESULTS_OUT_DIR, binName, caseName);
            if (fs.existsSync(outputPath) && fs.statSync(outputPath).isFile()) {
              output = fs.readFileSync(outputPath, "utf-8");
              outputExists = true;
              elapsedMs = findLatestElapsedMs(binName, caseName);
            }
          }
          sendJson(res, 200, { input, output, outputExists, elapsedMs });
        } catch (e) {
          sendJson(res, 400, { error: String(e) });
        }
      });

      server.middlewares.use("/api/eval-view-data", (req, res, next) => {
        if (req.method !== "GET") {
          next();
          return;
        }
        try {
          sendJson(res, 200, buildEvalViewData());
        } catch (e) {
          sendJson(res, 500, { error: String(e) });
        }
      });

      server.middlewares.use("/api/eval-view-version", (req, res, next) => {
        if (req.method !== "GET") {
          next();
          return;
        }
        try {
          sendJson(res, 200, buildEvalViewVersion());
        } catch (e) {
          sendJson(res, 500, { error: String(e) });
        }
      });

      server.middlewares.use("/api/solver-source", (req, res, next) => {
        if (req.method !== "GET") {
          next();
          return;
        }
        try {
          const requestUrl = new URL(req.url ?? "", "http://localhost");
          const binName = (requestUrl.searchParams.get("binName") ?? "").trim();
          const payload = buildSolverSource(binName);
          if (!payload) {
            sendJson(res, 404, {
              error: `ソースが見つからない: src/bin/${binName}.cpp, adhoc/bin/${binName}.cpp`,
            });
            return;
          }
          sendJson(res, 200, payload);
        } catch (e) {
          sendJson(res, 400, { error: String(e) });
        }
      });

      server.middlewares.use("/api/solver-bins", (req, res, next) => {
        if (req.method !== "GET") {
          next();
          return;
        }
        try {
          sendJson(res, 200, { bins: listSolverBins(), runnableBins: listRunnableBins() });
        } catch (e) {
          sendJson(res, 500, { error: String(e) });
        }
      });

      server.middlewares.use("/api/run-solver", async (req, res, next) => {
        if (req.method !== "POST") {
          next();
          return;
        }
        try {
          const raw = await readBody(req);
          const body = raw ? JSON.parse(raw) : {};
          const binName =
            typeof body.binName === "string" ? body.binName.trim() : "";
          const caseName =
            typeof body.caseName === "string" ? body.caseName.trim() : "";
          const inputText = typeof body.input === "string" ? body.input : "";
          const runnableBins = listRunnableBins();
          if (!runnableBins.includes(binName)) {
            sendJson(res, 400, {
              error: `bin '${binName}' は runnable solver ではない`,
            });
            return;
          }

          const startedAt = Date.now();
          buildSolver(binName);
          buildTool("tester");

          const solverBinPath = path.join(SOLVER_BIN_DIR, binName);
          if (!fs.existsSync(solverBinPath)) {
            throw new Error(`solver binary not found: ${solverBinPath}`);
          }
          if (!fs.existsSync(TESTER_BIN_PATH)) {
            throw new Error(`tester binary not found: ${TESTER_BIN_PATH}`);
          }

          const result = await new Promise((resolve, reject) => {
            const child = spawn(TESTER_BIN_PATH, [solverBinPath], {
              cwd: ROOT_DIR,
              stdio: ["pipe", "pipe", "pipe"],
            });

            let stdout = "";
            let stderr = "";
            const timer = setTimeout(() => {
              child.kill("SIGKILL");
              reject(new Error("実行がタイムアウトした (120秒)"));
            }, 120_000);

            child.stdout.on("data", (chunk) => {
              stdout += chunk.toString();
            });
            child.stderr.on("data", (chunk) => {
              stderr += chunk.toString();
            });
            child.on("error", (error) => {
              clearTimeout(timer);
              reject(error);
            });
            child.on("close", (code) => {
              clearTimeout(timer);
              if (code === 0) {
                resolve({ stdout, stderr });
              } else {
                reject(new Error(stderr.trim() || `exit code ${code}`));
              }
            });

            child.stdin.write(inputText);
            child.stdin.end();
          });

          let savedOutputPath = "";
          if (caseName) {
            const casePath = safeJoinCase(caseName);
            const outputDir = path.join(RESULTS_OUT_DIR, binName);
            fs.mkdirSync(outputDir, { recursive: true });
            const outputPath = path.join(outputDir, path.basename(casePath));
            fs.writeFileSync(outputPath, result.stdout, "utf-8");
            savedOutputPath = safeRelativePath(outputPath);
          }

          sendJson(res, 200, {
            output: result.stdout,
            stderr: result.stderr,
            elapsedMs: Date.now() - startedAt,
            savedOutputPath,
          });
        } catch (e) {
          sendJson(res, 500, { error: String(e) });
        }
      });
    },
  };
}

function* readEvalViewSnapshots() {
  for (const file of recordPaths(EVAL_VIEW_DATA_DIR, ".json", EVAL_VIEW_EXCLUDED_ENTRIES)) {
    yield JSON.parse(fs.readFileSync(file, "utf-8"));
  }
}

function buildEvalViewData() {
  const runsByEvalSet = {};
  const caseNamesByEvalSet = {};
  const caseSortOptionsByEvalSet = {};
  const caseMetaByEvalSet = {};

  for (const snapshot of readEvalViewSnapshots()) {
    for (const [evalSet, { caseNames, runs }] of Object.entries(snapshot.datasets)) {
      runsByEvalSet[evalSet] = runs.sort((left, right) =>
        left.totalAvg - right.totalAvg || right.executedAt.localeCompare(left.executedAt),
      );
      caseNamesByEvalSet[evalSet] = caseNames;
      caseSortOptionsByEvalSet[evalSet] = CASE_SORT_OPTIONS;
      caseMetaByEvalSet[evalSet] = buildCaseMetaByEvalSet(evalSet, caseNames);
    }
  }

  return {
    projectKey: PROJECT_KEY,
    evalSets: Object.keys(runsByEvalSet).sort(),
    runsByEvalSet,
    caseNamesByEvalSet,
    caseSortOptionsByEvalSet,
    caseMetaByEvalSet,
  };
}

function buildEvalViewVersion() {
  return evalRecordsVersion(EVAL_VIEW_DATA_DIR, ".json", EVAL_VIEW_EXCLUDED_ENTRIES);
}

export default defineConfig({
  plugins: [solverBinApiPlugin()],
  build: {
    rollupOptions: {
      input: {
        main: path.join(ROOT_DIR, "index.html"),
        eval: path.join(ROOT_DIR, "eval.html"),
      },
    },
  },
});
