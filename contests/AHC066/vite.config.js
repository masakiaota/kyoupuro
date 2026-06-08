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
const MANIFEST_PATH = path.join(ROOT_DIR, "Cargo.toml");
const EVAL_RECORDS_PATH = path.join(ROOT_DIR, "results", "eval_records.jsonl");
const TOOLS_DIR = path.join(ROOT_DIR, "tools");
const TOOLS_INPUT_DIR = path.join(ROOT_DIR, "tools", "in");
const DEFAULT_EVAL_SET = "tools/in";
const RESULTS_OUT_DIR = path.join(ROOT_DIR, "results", "out");
const SOLVER_BIN_DIR = path.join(ROOT_DIR, "target", "release");
const CASE_SORT_OPTIONS = [
  { key: "case_name_asc", label: "case_name ∧" },
  { key: "case_name_desc", label: "case_name ∨" },
  { key: "N_asc", label: "N ∧" },
  { key: "N_desc", label: "N ∨" },
  { key: "M_asc", label: "M ∧" },
  { key: "M_desc", label: "M ∨" },
  { key: "T_asc", label: "T ∧" },
  { key: "T_desc", label: "T ∨" },
  { key: "r_asc", label: "r ∧" },
  { key: "r_desc", label: "r ∨" },
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

function listRustBins() {
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
        ent.name.endsWith(".rs") &&
        /^v\d{3}.*\.rs$/.test(ent.name) &&
        ent.name !== "v000_template.rs",
    )
    .map((ent) => ent.name.slice(0, -3))
    .sort((left, right) => left.localeCompare(right, "ja"));
}

function normalizeInputDir(filePath) {
  const resolved = path.resolve(filePath);
  const relative = path.relative(ROOT_DIR, resolved);
  if (relative && !relative.startsWith("..") && !path.isAbsolute(relative)) {
    return relative.split(path.sep).join("/");
  }
  if (relative === "") {
    return ".";
  }
  return resolved;
}

function resolveInputDir(inputDir) {
  if (typeof inputDir !== "string" || inputDir.trim() === "") {
    return null;
  }
  return path.isAbsolute(inputDir) ? path.resolve(inputDir) : path.resolve(ROOT_DIR, inputDir);
}

function normalizeEvalSet(evalSet) {
  const raw = typeof evalSet === "string" && evalSet.trim() ? evalSet.trim() : DEFAULT_EVAL_SET;
  const inputDir = resolveInputDir(raw);
  if (!inputDir || !fs.existsSync(inputDir) || !fs.statSync(inputDir).isDirectory()) {
    throw new Error(`eval set not found: ${raw}`);
  }
  return {
    evalSet: normalizeInputDir(inputDir),
    inputDir,
  };
}

function outputSubdirPartsForEvalSet(evalSet) {
  if (path.isAbsolute(evalSet)) {
    const digest = crypto.createHash("sha256").update(evalSet).digest("hex").slice(0, 16);
    return ["_external", digest];
  }
  const parts = evalSet.split("/").filter(Boolean);
  if (parts.length === 0 || parts.some((part) => part === "." || part === "..")) {
    throw new Error(`invalid eval set: ${evalSet}`);
  }
  return parts;
}

function outputDirForEvalSet(binName, evalSet) {
  return path.join(RESULTS_OUT_DIR, binName, ...outputSubdirPartsForEvalSet(evalSet));
}

function listCaseFiles(inputDir, limit = Infinity) {
  if (!inputDir || !fs.existsSync(inputDir)) {
    return [];
  }
  const result = [];
  const stack = [inputDir];
  while (stack.length > 0 && result.length < limit) {
    const dir = stack.pop();
    for (const ent of fs.readdirSync(dir, { withFileTypes: true })) {
      const filePath = path.join(dir, ent.name);
      if (ent.isDirectory()) {
        stack.push(filePath);
      } else if (ent.isFile()) {
        result.push(filePath);
        if (result.length >= limit) {
          break;
        }
      }
    }
  }
  return result;
}

function isProblemInputDir(inputDir) {
  for (const filePath of listCaseFiles(inputDir, 12)) {
    if (parseCaseMeta(filePath)) {
      return true;
    }
  }
  return false;
}

function listVisualizerEvalSets() {
  const evalSets = new Set();
  if (fs.existsSync(TOOLS_DIR)) {
    for (const ent of fs.readdirSync(TOOLS_DIR, { withFileTypes: true })) {
      if (!ent.isDirectory() || ent.name === "src" || ent.name === "target") {
        continue;
      }
      const inputDir = path.join(TOOLS_DIR, ent.name);
      if (isProblemInputDir(inputDir)) {
        evalSets.add(normalizeInputDir(inputDir));
      }
    }
  }
  try {
    for (const record of readEvalRecords()) {
      if (!record || typeof record !== "object" || typeof record.input_dir !== "string") {
        continue;
      }
      const inputDir = resolveInputDir(record.input_dir);
      if (inputDir && fs.existsSync(inputDir) && fs.statSync(inputDir).isDirectory()) {
        evalSets.add(normalizeInputDir(inputDir));
      }
    }
  } catch {
    // Broken eval records should not prevent the case visualizer from opening.
  }
  return Array.from(evalSets).sort((left, right) => {
    if (left === DEFAULT_EVAL_SET) {
      return -1;
    }
    if (right === DEFAULT_EVAL_SET) {
      return 1;
    }
    return left.localeCompare(right, "ja");
  });
}

function listVisualizerCases(evalSet) {
  let inputDir = null;
  try {
    inputDir = normalizeEvalSet(evalSet).inputDir;
  } catch {
    return [];
  }
  return listCaseFiles(inputDir)
    .filter((filePath) => path.extname(filePath).toLowerCase() === ".txt")
    .map((filePath) => path.basename(filePath))
    .sort((left, right) => left.localeCompare(right, "ja"));
}

function safeJoinCase(inputDir, caseName) {
  if (typeof caseName !== "string" || caseName.trim() === "") {
    throw new Error("caseName is required");
  }
  if (path.basename(caseName) !== caseName) {
    throw new Error("invalid caseName");
  }
  const directPath = path.join(inputDir, caseName);
  if (fs.existsSync(directPath) && fs.statSync(directPath).isFile()) {
    return directPath;
  }
  const matches = listCaseFiles(inputDir).filter((filePath) => path.basename(filePath) === caseName);
  if (matches.length !== 1) {
    throw new Error(`case not found or ambiguous: ${caseName}`);
  }
  return matches[0];
}

function safeRelativePath(filePath) {
  return path.relative(ROOT_DIR, filePath).split(path.sep).join("/");
}

function findLatestElapsedMs(binName, evalSet, caseName) {
  if (!binName || !evalSet || !caseName || !fs.existsSync(EVAL_RECORDS_PATH)) {
    return null;
  }
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

function buildBinary(manifestPath, binName) {
  const result = spawnSync("cargo", [
    "build",
    "--release",
    "--quiet",
    "--manifest-path",
    manifestPath,
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

function parseCaseMeta(filePath) {
  const tokens = fs.readFileSync(filePath, "utf-8").trim().split(/\s+/);
  let at = 0;
  const N = Number(tokens[at++]);
  const M = Number(tokens[at++]);
  const T = Number(tokens[at++]);
  if (![N, M, T].every(Number.isFinite)) {
    return null;
  }

  const wallV = [];
  for (let i = 0; i < N; i += 1) {
    wallV.push(tokens[at++] ?? "");
  }
  const wallH = [];
  for (let i = 0; i < N - 1; i += 1) {
    wallH.push(tokens[at++] ?? "");
  }

  const points = [[0, 0]];
  for (let k = 0; k < M; k += 1) {
    const b = Number(tokens[at++]);
    const c = Number(tokens[at++]);
    const d = Number(tokens[at++]);
    const e = Number(tokens[at++]);
    if (![b, c, d, e].every(Number.isFinite)) {
      return { N, M, T };
    }
    points.push([b, c], [d, e]);
  }

  const X = computeOrderedDistance(N, wallV, wallH, points);
  const lo = 2 * X + 4 * M;
  const hi = 2 * N * N * M;
  let r = null;
  if (X >= 0 && lo > 0 && hi > 0 && lo !== hi && T > 0) {
    r = (Math.log(T) - Math.log(hi)) / (Math.log(lo) - Math.log(hi));
    if (Number.isFinite(r)) {
      r = Math.max(0, Math.min(1, r));
    } else {
      r = null;
    }
  }
  return { N, M, T, X, r };
}

function computeOrderedDistance(N, wallV, wallH, points) {
  let total = 0;
  for (let idx = 0; idx + 1 < points.length; idx += 1) {
    const d = shortestDistance(N, wallV, wallH, points[idx], points[idx + 1]);
    if (d < 0) {
      return -1;
    }
    total += d;
  }
  return total;
}

function shortestDistance(N, wallV, wallH, from, to) {
  const start = from[0] * N + from[1];
  const goal = to[0] * N + to[1];
  if (start === goal) {
    return 0;
  }

  const total = N * N;
  const dist = new Int16Array(total);
  dist.fill(-1);
  const queue = new Int16Array(total);
  let head = 0;
  let tail = 0;
  queue[tail++] = start;
  dist[start] = 0;

  while (head < tail) {
    const cell = queue[head++];
    const i = Math.floor(cell / N);
    const j = cell % N;
    const nextDist = dist[cell] + 1;

    const push = (ni, nj) => {
      const next = ni * N + nj;
      if (dist[next] >= 0) {
        return true;
      }
      if (next === goal) {
        return false;
      }
      dist[next] = nextDist;
      queue[tail++] = next;
      return true;
    };

    if (i > 0 && wallH[i - 1]?.[j] !== "1" && !push(i - 1, j)) {
      return nextDist;
    }
    if (j + 1 < N && wallV[i]?.[j] !== "1" && !push(i, j + 1)) {
      return nextDist;
    }
    if (i + 1 < N && wallH[i]?.[j] !== "1" && !push(i + 1, j)) {
      return nextDist;
    }
    if (j > 0 && wallV[i]?.[j - 1] !== "1" && !push(i, j - 1)) {
      return nextDist;
    }
  }

  return -1;
}

function buildCaseMetaByEvalSet(inputDir, caseNames) {
  const resolvedInputDir = resolveInputDir(inputDir);
  if (!resolvedInputDir || !fs.existsSync(resolvedInputDir)) {
    return {};
  }
  const meta = {};
  for (const caseName of caseNames) {
    let filePath = "";
    try {
      filePath = safeJoinCase(resolvedInputDir, caseName);
    } catch {
      continue;
    }
    const parsed = parseCaseMeta(filePath);
    if (parsed) {
      meta[caseName] = parsed;
    }
  }
  return meta;
}

function rustBinApiPlugin() {
  return {
    name: "rust-bin-api",
    configureServer(server) {
      server.middlewares.use("/api/visualizer-data", (req, res, next) => {
        if (req.method !== "GET") {
          next();
          return;
        }
        try {
          const evalSets = listVisualizerEvalSets();
          const casesByEvalSet = {};
          for (const evalSet of evalSets) {
            casesByEvalSet[evalSet] = listVisualizerCases(evalSet);
          }
          sendJson(res, 200, {
            projectKey: PROJECT_KEY,
            bins: listRustBins(),
            runnableBins: listRunnableBins(),
            evalSets,
            casesByEvalSet,
            cases: casesByEvalSet[DEFAULT_EVAL_SET] ?? casesByEvalSet[evalSets[0]] ?? [],
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
          const requestedEvalSet = requestUrl.searchParams.get("evalSet") ?? DEFAULT_EVAL_SET;
          const { evalSet, inputDir } = normalizeEvalSet(requestedEvalSet);
          const casePath = safeJoinCase(inputDir, caseName);
          const input = fs.readFileSync(casePath, "utf-8");

          let output = "";
          let outputExists = false;
          let elapsedMs = null;
          if (binName && path.basename(binName) === binName) {
            const outputPath = path.join(outputDirForEvalSet(binName, evalSet), caseName);
            if (fs.existsSync(outputPath) && fs.statSync(outputPath).isFile()) {
              output = fs.readFileSync(outputPath, "utf-8");
              outputExists = true;
              elapsedMs = findLatestElapsedMs(binName, evalSet, caseName);
            }
          }
          sendJson(res, 200, { evalSet, input, output, outputExists, elapsedMs });
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

      server.middlewares.use("/api/eval-case-meta", (req, res, next) => {
        if (req.method !== "GET") {
          next();
          return;
        }
        try {
          const requestUrl = new URL(req.url ?? "", "http://localhost");
          const requestedEvalSet = requestUrl.searchParams.get("evalSet") ?? DEFAULT_EVAL_SET;
          const { evalSet } = normalizeEvalSet(requestedEvalSet);
          const caseNames = listVisualizerCases(evalSet);
          const caseMeta = buildCaseMetaByEvalSet(evalSet, caseNames);
          sendJson(res, 200, { evalSet, caseMeta });
        } catch (e) {
          sendJson(res, 500, { error: String(e) });
        }
      });

      server.middlewares.use("/api/rust-bins", (req, res, next) => {
        if (req.method !== "GET") {
          next();
          return;
        }
        try {
          sendJson(res, 200, { bins: listRustBins(), runnableBins: listRunnableBins() });
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
          const caseName =
            typeof body.caseName === "string" ? body.caseName.trim() : "";
          const requestedEvalSet =
            typeof body.evalSet === "string" && body.evalSet.trim()
              ? body.evalSet.trim()
              : DEFAULT_EVAL_SET;
          const inputText = typeof body.input === "string" ? body.input : "";
          const { evalSet, inputDir } = normalizeEvalSet(requestedEvalSet);
          const runnableBins = listRunnableBins();
          if (!runnableBins.includes(binName)) {
            sendJson(res, 400, {
              error: `bin '${binName}' は runnable solver ではない`,
            });
            return;
          }

          const startedAt = Date.now();
          buildBinary(MANIFEST_PATH, binName);

          const solverBinPath = path.join(SOLVER_BIN_DIR, binName);
          if (!fs.existsSync(solverBinPath)) {
            throw new Error(`solver binary not found: ${solverBinPath}`);
          }

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

          let savedOutputPath = "";
          if (caseName) {
            const casePath = safeJoinCase(inputDir, caseName);
            const outputDir = outputDirForEvalSet(binName, evalSet);
            fs.mkdirSync(outputDir, { recursive: true });
            const outputPath = path.join(outputDir, path.basename(casePath));
            fs.writeFileSync(outputPath, result.stdout, "utf-8");
            savedOutputPath = safeRelativePath(outputPath);
          }

          sendJson(res, 200, {
            evalSet,
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

  for (const run of runMap.values()) {
    if (run.hasFailure) {
      continue;
    }
    const caseNames = Object.keys(run.caseScores).sort();
    if (caseNames.length === 0) {
      continue;
    }
    const totalSum = caseNames.reduce((acc, caseName) => acc + run.caseScores[caseName], 0);
    const totalAvg = Math.round((totalSum / caseNames.length) * 100) / 100;
    const maxElapsed = Math.max(...caseNames.map((caseName) => run.caseElapsed[caseName]));
    const resultRun = {
      id: run.id,
      bin: run.bin,
      totalAvg,
      maxElapsed,
      label: run.label,
      executedAt: run.executedAt,
      caseScores: run.caseScores,
      caseElapsed: run.caseElapsed,
    };
    if (!runsByEvalSet[run.evalSet]) {
      runsByEvalSet[run.evalSet] = [];
      caseNamesByEvalSet[run.evalSet] = new Set();
      caseSortOptionsByEvalSet[run.evalSet] = [...CASE_SORT_OPTIONS];
      caseMetaByEvalSet[run.evalSet] = {};
    }
    runsByEvalSet[run.evalSet].push(resultRun);
    for (const caseName of caseNames) {
      caseNamesByEvalSet[run.evalSet].add(caseName);
    }
  }

  const evalSets = Object.keys(runsByEvalSet).sort();
  const normalizedCaseNamesByEvalSet = {};
  for (const evalSet of evalSets) {
    normalizedCaseNamesByEvalSet[evalSet] = Array.from(caseNamesByEvalSet[evalSet]).sort();
    attachLocalRelativeScores(
      runsByEvalSet[evalSet],
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

function attachLocalRelativeScores(runs, caseNames) {
  const bestByCase = {};
  for (const caseName of caseNames) {
    let best = Infinity;
    for (const run of runs) {
      const score = Number(run.caseScores?.[caseName]);
      if (Number.isFinite(score) && score > 0 && score < best) {
        best = score;
      }
    }
    if (Number.isFinite(best)) {
      bestByCase[caseName] = best;
    }
  }

  for (const run of runs) {
    let sum = 0;
    const caseRelativeScores = {};
    for (const caseName of caseNames) {
      const score = Number(run.caseScores?.[caseName]);
      const best = bestByCase[caseName];
      if (Number.isFinite(score) && score > 0 && Number.isFinite(best)) {
        const relative = Math.round((1_000_000_000 * best) / score);
        caseRelativeScores[caseName] = relative;
        sum += relative;
      }
    }
    run.localRelSum = sum;
    run.caseRelativeScores = caseRelativeScores;
  }
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
  plugins: [rustBinApiPlugin()],
  build: {
    rollupOptions: {
      input: {
        main: path.join(ROOT_DIR, "index.html"),
        eval: path.join(ROOT_DIR, "eval.html"),
      },
    },
  },
});
