import fs from "node:fs";
import path from "node:path";
import crypto from "node:crypto";
import { spawn, spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { defineConfig } from "vite";

const ROOT_DIR = fileURLToPath(new URL(".", import.meta.url));
const PROJECT_KEY = crypto
  .createHash("sha256")
  .update(path.resolve(ROOT_DIR))
  .digest("hex")
  .slice(0, 16);
const SRC_BIN_DIR = path.join(ROOT_DIR, "src", "bin");
const TOOLS_MANIFEST_PATH = path.join(ROOT_DIR, "tools", "Cargo.toml");
const BUILD_SOLVER_PATH = path.join(ROOT_DIR, "scripts", "build_solver.sh");
const EVAL_RECORDS_DIR = path.join(ROOT_DIR, "results", "eval_records");
// 単発の評価セットは eval viewer に載せない。件数が増えると表示が重くなるためである。
// adhoc/ 配下と project 外の入力による記録を読み込み対象から外す。
const EVAL_RECORDS_EXCLUDED_ENTRIES = ["adhoc", "adhoc.jsonl", "_external"];
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
        /^v\d{3}.*\.cpp$/.test(ent.name) &&
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
  if (!binName || !caseName) {
    return null;
  }
  try {
    const parsed = loadEvalSet(safeRelativePath(TOOLS_INPUT_DIR));
    const latest = parsed?.latestElapsed.get(`${binName}\0${caseName}`);
    return latest ? latest.elapsed : null;
  } catch {
    return null;
  }
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
  // Problem-specific extension point:
  // read filePath and return metadata used by src_eval/case_sorters.js.
  // Return null when no metadata-based case sort is needed.
  void filePath;
  return null;
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
          const requested = new URL(req.url ?? "", "http://localhost").searchParams.get("evalSet");
          sendJson(res, 200, buildEvalViewData(requested ?? ""));
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

// eval viewer は選択中の評価セットだけを読み込む。評価セットは
// results/eval_records/<input_dir>.jsonl の 1 ファイルに対応する (scripts/eval.py)。
function listEvalRecordFiles(directory = EVAL_RECORDS_DIR, isTopLevel = true) {
  if (!fs.existsSync(directory)) {
    return [];
  }
  const files = [];
  for (const entry of fs.readdirSync(directory, { withFileTypes: true })) {
    if (isTopLevel && EVAL_RECORDS_EXCLUDED_ENTRIES.includes(entry.name)) {
      continue;
    }
    const entryPath = path.join(directory, entry.name);
    if (entry.isDirectory()) {
      files.push(...listEvalRecordFiles(entryPath, false));
    } else if (entry.isFile() && entry.name.endsWith(".jsonl")) {
      files.push(entryPath);
    }
  }
  return files.sort();
}

function evalSetOfRecordFile(file) {
  const relative = path.relative(EVAL_RECORDS_DIR, file).split(path.sep).join("/");
  return relative.slice(0, -".jsonl".length);
}

function listEvalSets() {
  return listEvalRecordFiles().map(evalSetOfRecordFile);
}

function recordFileOfEvalSet(evalSet) {
  return listEvalRecordFiles().find((file) => evalSetOfRecordFile(file) === evalSet) ?? null;
}

// 記録ファイルごとの集計状態。記録は追記専用なので、前回読んだ位置より後ろだけを読み足す。
// 数百 MB の記録でも、全体を読むのは dev server の起動後に初めて選ばれたときの 1 回だけになる。
const evalSetCache = new Map();
const READ_CHUNK_BYTES = 1 << 22;

function loadEvalSet(evalSet) {
  const file = recordFileOfEvalSet(evalSet);
  if (!file) {
    return null;
  }
  const stat = fs.statSync(file);
  let entry = evalSetCache.get(file);
  // 縮んだファイルは書き換えられたものとして、先頭から読み直す。
  if (!entry || stat.size < entry.offset) {
    entry = { offset: 0, runMap: new Map(), latestElapsed: new Map(), view: null };
    evalSetCache.set(file, entry);
  }
  if (stat.size > entry.offset || !entry.view) {
    readAppendedRecords(file, evalSet, entry, stat.size);
    entry.view = buildEvalSetView(evalSet, entry);
  }
  return entry.view;
}

// entry.offset から end までの完結した行を読み、集計状態へ反映する。
// 書き込み途中の最終行は次回に回すので、評価の実行中に読んでも壊れない。
function readAppendedRecords(file, evalSet, entry, end) {
  const fd = fs.openSync(file, "r");
  try {
    const chunk = Buffer.allocUnsafe(READ_CHUNK_BYTES);
    let pending = Buffer.alloc(0);
    let position = entry.offset;
    while (position < end) {
      const bytesRead = fs.readSync(fd, chunk, 0, Math.min(READ_CHUNK_BYTES, end - position), position);
      if (bytesRead <= 0) {
        break;
      }
      position += bytesRead;
      const data = pending.length
        ? Buffer.concat([pending, chunk.subarray(0, bytesRead)])
        : chunk.subarray(0, bytesRead);
      let lineStart = 0;
      for (;;) {
        const newline = data.indexOf(0x0a, lineStart);
        if (newline < 0) {
          break;
        }
        addEvalRecordLine(data.toString("utf-8", lineStart, newline), file, evalSet, entry);
        lineStart = newline + 1;
      }
      pending = Buffer.from(data.subarray(lineStart));
    }
    entry.offset = position - pending.length;
  } finally {
    fs.closeSync(fd);
  }
}

function addEvalRecordLine(line, file, evalSet, entry) {
  if (!line.trim()) {
    return;
  }
  let record;
  try {
    record = JSON.parse(line);
  } catch (error) {
    throw new Error(`Failed to parse ${file}: ${String(error)}`);
  }
  if (!record || typeof record !== "object") {
    return;
  }
  const bin = typeof record.bin === "string" ? record.bin : "";
  const label = typeof record.label === "string" ? record.label : "";
  const executedAt = typeof record.executed_at === "string" ? record.executed_at : "";
  const caseName = typeof record.case_name === "string" ? record.case_name : "";

  if (bin && caseName && typeof record.elapsed === "number") {
    const key = `${bin}\0${caseName}`;
    const previous = entry.latestElapsed.get(key);
    if (!previous || executedAt.localeCompare(previous.executedAt) > 0) {
      entry.latestElapsed.set(key, { executedAt, elapsed: record.elapsed });
    }
  }

  const runId = typeof record.run_id === "string" ? record.run_id : "";
  if (!runId) {
    return;
  }
  let run = entry.runMap.get(runId);
  if (!run) {
    run = { id: runId, bin, label, executedAt, caseScores: new Map(), maxElapsed: 0, hasFailure: false };
    entry.runMap.set(runId, run);
  }
  if (
    record.status !== "ok" ||
    !caseName ||
    typeof record.score !== "number" ||
    typeof record.elapsed !== "number" ||
    record.input_dir !== evalSet ||
    run.bin !== bin ||
    run.label !== label ||
    run.executedAt !== executedAt
  ) {
    run.hasFailure = true;
    return;
  }
  run.caseScores.set(caseName, record.score);
  run.maxElapsed = Math.max(run.maxElapsed, record.elapsed);
}

function buildEvalSetView(evalSet, entry) {
  const validRuns = Array.from(entry.runMap.values()).filter(
    (run) => !run.hasFailure && run.caseScores.size > 0,
  );
  const caseNameSet = new Set();
  for (const run of validRuns) {
    for (const caseName of run.caseScores.keys()) {
      caseNameSet.add(caseName);
    }
  }
  const caseNames = Array.from(caseNameSet).sort();

  const runs = validRuns.map((run) => {
    let sum = 0;
    for (const score of run.caseScores.values()) {
      sum += score;
    }
    return {
      id: run.id,
      bin: run.bin,
      // 平均と最大経過時間は、表示中の列に関係なく全ケースで集計する。
      totalAvg: Math.round(sum / run.caseScores.size),
      maxElapsed: run.maxElapsed,
      label: run.label,
      executedAt: run.executedAt,
      // caseNames と同じ並びの配列で送り、ケース名の繰り返しを省く。未実行のケースは null。
      scores: caseNames.map((caseName) => run.caseScores.get(caseName) ?? null),
    };
  });
  runs.sort((left, right) => {
    if (right.totalAvg !== left.totalAvg) {
      return right.totalAvg - left.totalAvg;
    }
    return right.executedAt.localeCompare(left.executedAt);
  });

  // ケース名の集合が変わらなければ、入力ファイルを読む metadata は前回のものを使い回す。
  const previous = entry.view;
  const sameCases =
    previous &&
    previous.caseNames.length === caseNames.length &&
    previous.caseNames.every((caseName, index) => caseName === caseNames[index]);
  return {
    caseNames,
    caseSortOptions: [...CASE_SORT_OPTIONS],
    caseMeta: sameCases ? previous.caseMeta : buildCaseMetaByEvalSet(evalSet, caseNames),
    runs,
    latestElapsed: entry.latestElapsed,
  };
}

function buildEvalViewData(requestedEvalSet) {
  const evalSets = listEvalSets();
  const evalSet = evalSets.includes(requestedEvalSet) ? requestedEvalSet : (evalSets[0] ?? "");
  const parsed = evalSet ? loadEvalSet(evalSet) : null;
  return {
    projectKey: PROJECT_KEY,
    evalSets,
    evalSet,
    caseNames: parsed?.caseNames ?? [],
    caseSortOptions: parsed?.caseSortOptions ?? [...CASE_SORT_OPTIONS],
    caseMeta: parsed?.caseMeta ?? {},
    runs: parsed?.runs ?? [],
  };
}

// 記録を読まずに答える軽い問い合わせ。画面はこれを定期的に呼び、変化があったときだけデータを取り直す。
function buildEvalViewVersion() {
  const files = listEvalRecordFiles();
  const entries = files.map((file) => {
    const stat = fs.statSync(file);
    return [evalSetOfRecordFile(file), stat.mtimeMs, stat.size];
  });
  return {
    projectKey: PROJECT_KEY,
    evalSets: entries.map((entry) => entry[0]),
    exists: files.length > 0,
    mtimeMs: entries.reduce((max, entry) => Math.max(max, entry[1]), 0),
    size: entries.reduce((sum, entry) => sum + entry[2], 0),
    // ファイルの増減や同サイズの置換も更新として検知する。
    signature: files.length
      ? crypto.createHash("sha256").update(JSON.stringify(entries)).digest("hex")
      : "missing:0",
  };
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
