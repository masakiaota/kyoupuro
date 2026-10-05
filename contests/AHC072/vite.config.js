import fs from "node:fs";
import path from "node:path";
import crypto from "node:crypto";
import { spawn, spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { defineConfig } from "vite";
import { playApiPlugin } from "./server/play-api.mjs";

const ROOT_DIR = fileURLToPath(new URL(".", import.meta.url));
const PROJECT_KEY = crypto
  .createHash("sha256")
  .update(path.resolve(ROOT_DIR))
  .digest("hex")
  .slice(0, 16);
const SRC_BIN_DIR = path.join(ROOT_DIR, "src", "bin");
const BUILD_SOLVER_PATH = path.join(ROOT_DIR, "scripts", "build_solver.sh");
const EVAL_RECORDS_PATH = path.join(ROOT_DIR, "results", "eval_records.jsonl");
const TOOLS_INPUT_DIR = path.join(ROOT_DIR, "tools", "in");
const RESULTS_OUT_DIR = path.join(ROOT_DIR, "results", "out");
const SOLVER_BIN_DIR = path.join(ROOT_DIR, "target", "release");
const CASE_SORT_OPTIONS = [
  { key: "case_name_asc", label: "case_name ∧" },
  { key: "case_name_desc", label: "case_name ∨" },
  ...[["N", "盤面サイズ N"], ["K", "色数 K"], ["M", "スライム数 M"], ["wall_count", "壁数"]]
    .flatMap(([field, label]) => [
      { key: `${field}_asc`, label: `${label} ∧` },
      { key: `${field}_desc`, label: `${label} ∨` },
    ]),
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
        /^v\d{3}.*\.cpp$/.test(ent.name) &&
        ent.name !== "v000_template.cpp",
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
  if (!binName || !caseName || !fs.existsSync(EVAL_RECORDS_PATH)) {
    return null;
  }
  const evalSet = safeRelativePath(TOOLS_INPUT_DIR);
  let latest = null;
  try {
    for (const record of readEvalRecords()) {
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
  const tokens = fs.readFileSync(filePath, "utf-8").trim().split(/\s+/);
  const N = Number(tokens[0]);
  const K = Number(tokens[1]);
  if (!Number.isInteger(N) || N < 12 || N > 20 ||
      !Number.isInteger(K) || K < 4 || K > 12 || tokens.length !== N + 2) {
    return null;
  }
  const rows = tokens.slice(2);
  if (rows.some((row) => row.length !== N || !/^[.#a-lA-L]+$/.test(row))) {
    return null;
  }
  const cells = rows.join("");
  return {
    N, K,
    M: (cells.match(/[a-l]/g) ?? []).length,
    wall_count: (cells.match(/#/g) ?? []).length,
  };
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

          buildSolver(binName);

          const solverBinPath = path.join(SOLVER_BIN_DIR, binName);
          if (!fs.existsSync(solverBinPath)) {
            throw new Error(`solver binary not found: ${solverBinPath}`);
          }
          // 今回は非対話型なので solver を直接実行し、ビルド時間を除いて計測する。
          const startedAt = Date.now();
          const result = await new Promise((resolve, reject) => {
            const child = spawn(solverBinPath, [], {
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

          const elapsedMs = Date.now() - startedAt;
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
            elapsedMs,
            savedOutputPath,
          });
        } catch (e) {
          sendJson(res, 500, { error: String(e) });
        }
      });
    },
  };
}

function buildEvalViewData() {
  const records = readEvalRecords();
  const runMap = new Map();

  for (const record of records) {
    if (!record || typeof record !== "object") {
      continue;
    }
    const runId = typeof record.run_id === "string" ? record.run_id : "";
    if (!runId) {
      continue;
    }
    if (!runMap.has(runId)) {
      runMap.set(runId, {
        id: runId,
        bin: typeof record.bin === "string" ? record.bin : "",
        label: typeof record.label === "string" ? record.label : "",
        executedAt: typeof record.executed_at === "string" ? record.executed_at : "",
        evalSet: typeof record.input_dir === "string" ? record.input_dir : "",
        caseScores: {},
        caseElapsed: {},
        hasFailure: false,
      });
    }

    const run = runMap.get(runId);
    const caseName = typeof record.case_name === "string" ? record.case_name : "";
    const status = typeof record.status === "string" ? record.status : "";
    if (status !== "ok") {
      run.hasFailure = true;
      continue;
    }
    if (!caseName || typeof record.score !== "number" || typeof record.elapsed !== "number") {
      run.hasFailure = true;
      continue;
    }
    if (
      run.bin !== (typeof record.bin === "string" ? record.bin : "") ||
      run.label !== (typeof record.label === "string" ? record.label : "") ||
      run.executedAt !== (typeof record.executed_at === "string" ? record.executed_at : "") ||
      run.evalSet !== (typeof record.input_dir === "string" ? record.input_dir : "")
    ) {
      run.hasFailure = true;
      continue;
    }
    run.caseScores[caseName] = record.score;
    run.caseElapsed[caseName] = record.elapsed;
  }

  const runsByEvalSet = {};
  const caseNamesByEvalSet = {};
  const caseSortOptionsByEvalSet = {};
  const caseMetaByEvalSet = {};

  const validRunsByEvalSet = {};
  for (const run of runMap.values()) {
    if (run.hasFailure) {
      continue;
    }
    const caseNames = Object.keys(run.caseScores).sort();
    if (caseNames.length === 0) {
      continue;
    }
    if (!validRunsByEvalSet[run.evalSet]) {
      validRunsByEvalSet[run.evalSet] = [];
    }
    validRunsByEvalSet[run.evalSet].push({ run, caseNames });
  }

  for (const [evalSet, entries] of Object.entries(validRunsByEvalSet)) {
    // 擬似相対評価: 同一 eval set 内の run 群を competitor とみなし、
    // ケースごとに最小絶対スコアを MIN とする。
    const minByCase = {};
    for (const { run, caseNames } of entries) {
      for (const caseName of caseNames) {
        const score = run.caseScores[caseName];
        if (minByCase[caseName] === undefined || score < minByCase[caseName]) {
          minByCase[caseName] = score;
        }
      }
    }
    runsByEvalSet[evalSet] = [];
    caseNamesByEvalSet[evalSet] = new Set();
    caseSortOptionsByEvalSet[evalSet] = [...CASE_SORT_OPTIONS];
    caseMetaByEvalSet[evalSet] = {};
    for (const { run, caseNames } of entries) {
      const totalAvg =
        caseNames.reduce((acc, caseName) => acc + run.caseScores[caseName], 0) / caseNames.length;
      const relativeAvg =
        caseNames.reduce((acc, caseName) => {
          const your = run.caseScores[caseName];
          const min = minByCase[caseName];
          const relative = your > 0 ? Math.round((1e9 * min) / your) : min <= 0 ? 1e9 : 0;
          return acc + relative;
        }, 0) / caseNames.length;
      const maxElapsed = Math.max(...caseNames.map((caseName) => run.caseElapsed[caseName]));
      const resultRun = {
        id: run.id,
        bin: run.bin,
        totalAvg,
        relativeAvg,
        maxElapsed,
        label: run.label,
        executedAt: run.executedAt,
        caseScores: run.caseScores,
        caseElapsed: run.caseElapsed,
      };
      runsByEvalSet[evalSet].push(resultRun);
      for (const caseName of caseNames) {
        caseNamesByEvalSet[evalSet].add(caseName);
      }
    }
  }

  const evalSets = Object.keys(runsByEvalSet).sort();
  const normalizedCaseNamesByEvalSet = {};
  for (const evalSet of evalSets) {
    normalizedCaseNamesByEvalSet[evalSet] = Array.from(caseNamesByEvalSet[evalSet]).sort();
    caseMetaByEvalSet[evalSet] = buildCaseMetaByEvalSet(
      evalSet,
      normalizedCaseNamesByEvalSet[evalSet],
    );
    runsByEvalSet[evalSet].sort((left, right) => {
      if (right.totalAvg !== left.totalAvg) {
        return right.totalAvg - left.totalAvg;
      }
      return right.executedAt.localeCompare(left.executedAt);
    });
  }

  return {
    projectKey: PROJECT_KEY,
    evalSets,
    runsByEvalSet,
    caseNamesByEvalSet: normalizedCaseNamesByEvalSet,
    caseSortOptionsByEvalSet,
    caseMetaByEvalSet,
  };
}

function buildEvalViewVersion() {
  if (!fs.existsSync(EVAL_RECORDS_PATH)) {
    return {
      exists: false,
      mtimeMs: 0,
      size: 0,
      signature: "missing:0",
    };
  }
  const stat = fs.statSync(EVAL_RECORDS_PATH);
  return {
    exists: true,
    mtimeMs: stat.mtimeMs,
    size: stat.size,
    signature: `${stat.mtimeMs}:${stat.size}`,
  };
}

function readEvalRecords() {
  if (!fs.existsSync(EVAL_RECORDS_PATH)) {
    return [];
  }
  const raw = fs.readFileSync(EVAL_RECORDS_PATH, "utf-8");
  const lines = raw.split(/\r?\n/);
  const records = [];
  for (const line of lines) {
    if (!line.trim()) {
      continue;
    }
    try {
      records.push(JSON.parse(line));
    } catch (error) {
      throw new Error(`Failed to parse eval_records.jsonl: ${String(error)}`);
    }
  }
  return records;
}

export default defineConfig({
  plugins: [solverBinApiPlugin(), playApiPlugin(ROOT_DIR)],
  server: { host: "127.0.0.1" },
  build: {
    rollupOptions: {
      input: {
        main: path.join(ROOT_DIR, "index.html"),
        eval: path.join(ROOT_DIR, "eval.html"),
        play: path.join(ROOT_DIR, "play.html"),
      },
    },
  },
});
