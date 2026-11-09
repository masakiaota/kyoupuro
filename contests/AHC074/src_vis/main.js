import init, {
  get_max_turn as wasmGetMaxTurnModule,
  get_order_ends as wasmGetOrderEndsModule,
  vis as wasmVisModule,
} from "./wasm/ahc074_vis.js";

let wasmGetMaxTurn = null;
let wasmGetOrderEnds = null;
let wasmVis = null;

const CUSTOM_CASE_VALUE = "";
const MAX_RENDER_FPS = 60;
const HOLD_DELAY_MS = 500;
const HOLD_INTERVAL_MS = 55;
const BIN_POLL_INTERVAL_MS = 10_000;
const STORAGE_PREFIX = "ahc-visualizer";
const STORAGE_VERSION = "v1";
const STEP_UNITS = new Set(["move", "order"]);

const els = {
  refreshBtn: document.getElementById("refreshBtn"),
  solverBin: document.getElementById("solverBin"),
  binPrevBtn: document.getElementById("binPrevBtn"),
  binNextBtn: document.getElementById("binNextBtn"),
  caseName: document.getElementById("caseName"),
  casePrevBtn: document.getElementById("casePrevBtn"),
  caseNextBtn: document.getElementById("caseNextBtn"),
  runBinBtn: document.getElementById("runBinBtn"),
  runStatus: document.getElementById("runStatus"),
  turn: document.getElementById("turn"),
  turnLabel: document.getElementById("turnLabel"),
  turnValue: document.getElementById("turnValue"),
  maxTurnValue: document.getElementById("maxTurnValue"),
  prevBtn: document.getElementById("prevBtn"),
  playBtn: document.getElementById("playBtn"),
  nextBtn: document.getElementById("nextBtn"),
  speed: document.getElementById("speed"),
  stepUnit: document.getElementById("stepUnit"),
  score: document.getElementById("score"),
  elapsedTime: document.getElementById("elapsedTime"),
  error: document.getElementById("error"),
  inputArea: document.getElementById("inputArea"),
  outputArea: document.getElementById("outputArea"),
  svgHost: document.getElementById("svgHost"),
  copyInputBtn: document.getElementById("copyInputBtn"),
  copyOutputBtn: document.getElementById("copyOutputBtn"),
};

const state = {
  ready: false,
  running: false,
  playing: false,
  rafId: null,
  lastFrameTs: null,
  lastRenderTs: 0,
  playbackTurnFloat: 0,
  bins: [],
  runnableBins: new Set(),
  cases: [],
  projectKey: "",
  caseStorageKey: "",
  currentBin: "",
  currentCase: null,
  pendingTurn: null,
  preserveEmptyBinSelection: false,
  loadSeq: 0,
  suppressInputDirty: false,
  suppressOutputDirty: false,
  binPollInFlight: false,
  currentElapsedMs: null,
  // スライダーの値は stepUnit の単位での位置である。"move" では turn (手数) そのもの、
  // "order" では処理済みの注文数 p で、その画面は frameTurns[p] 手目の直後を表す。
  stepUnit: "move",
  // frameTurns[0] = 0、frameTurns[p] = 注文 p-1 を処理し終えた turn。
  // 出力が途中で不正なら、最後の完了注文より後の手を末尾に 1 つ足して最後まで見られるようにする。
  frameTurns: [0],
};

function positionToTurn(position) {
  if (state.stepUnit === "order") {
    return state.frameTurns[Math.min(position, state.frameTurns.length - 1)] ?? 0;
  }
  return position;
}

function turnToPosition(turn) {
  if (state.stepUnit !== "order") {
    return turn;
  }
  // turn 手目を含む注文を表示する。注文の境目の turn では、そこで終わった注文になる。
  const index = state.frameTurns.findIndex((frameTurn) => frameTurn >= turn);
  return index < 0 ? state.frameTurns.length - 1 : index;
}

// 重ね描きする手の始まり。1 手ずつなら直前の 1 手、注文単位ならその注文の全手である。
function moveStartFor(position) {
  if (state.stepUnit === "order") {
    return position > 0 ? positionToTurn(position - 1) : 0;
  }
  return Math.max(0, position - 1);
}

function currentPosition() {
  return Number(els.turn.value) || 0;
}

function currentTurn() {
  return positionToTurn(currentPosition());
}

function getMaxTurn() {
  return Number(els.turn.max) || 0;
}

function getSpeed() {
  const value = Number(els.speed.value);
  return Number.isFinite(value) && value > 0 ? value : 5;
}

function formatScore(value) {
  if (!Number.isFinite(Number(value))) {
    return "-";
  }
  return Number(value).toLocaleString("en-US");
}

function formatElapsedMs(value) {
  const elapsed = Number(value);
  if (!Number.isFinite(elapsed) || elapsed < 0) {
    return "-";
  }
  if (elapsed < 1000) {
    return `${Math.round(elapsed)} ms`;
  }
  return `${(elapsed / 1000).toFixed(3)} s`;
}

function setElapsedMs(value) {
  const elapsed = Number(value);
  state.currentElapsedMs = Number.isFinite(elapsed) && elapsed >= 0 ? elapsed : null;
  els.elapsedTime.textContent = formatElapsedMs(state.currentElapsedMs);
}

function setRunStatus(text, isError = false) {
  els.runStatus.textContent = text;
  els.runStatus.style.color = isError ? "#b91c1c" : "#5b6472";
}

function makeStorageKey(projectKey) {
  const key = typeof projectKey === "string" && projectKey ? projectKey : "unknown";
  return `${STORAGE_PREFIX}:${key}:case:${STORAGE_VERSION}`;
}

function readSessionJson(key) {
  try {
    const raw = window.sessionStorage.getItem(key);
    if (!raw) {
      return null;
    }
    const parsed = JSON.parse(raw);
    return parsed && typeof parsed === "object" ? parsed : null;
  } catch {
    return null;
  }
}

function writeSessionJson(key, value) {
  try {
    window.sessionStorage.setItem(key, JSON.stringify(value));
  } catch {
    // Storage errors should not break the visualizer itself.
  }
}

function configureCaseStorage(projectKey) {
  const nextKey = makeStorageKey(projectKey);
  if (state.caseStorageKey === nextKey) {
    return;
  }
  state.projectKey = typeof projectKey === "string" && projectKey ? projectKey : "unknown";
  state.caseStorageKey = nextKey;

  const saved = readSessionJson(nextKey);
  if (!saved) {
    return;
  }
  if (Object.prototype.hasOwnProperty.call(saved, "bin") && typeof saved.bin === "string") {
    state.currentBin = saved.bin;
    state.preserveEmptyBinSelection = saved.bin === "";
  }
  if (Object.prototype.hasOwnProperty.call(saved, "caseName") && typeof saved.caseName === "string") {
    state.currentCase = saved.caseName;
  }
  const savedSpeed = String(saved.speed ?? "");
  if (Array.from(els.speed.options).some((option) => option.value === savedSpeed)) {
    els.speed.value = savedSpeed;
  }
  if (STEP_UNITS.has(saved.stepUnit)) {
    state.stepUnit = saved.stepUnit;
  }
  const savedTurn = Number(saved.turn);
  if (Number.isFinite(savedTurn)) {
    state.pendingTurn = Math.max(0, Math.round(savedTurn));
  }
}

function saveCaseViewState() {
  if (!state.caseStorageKey) {
    return;
  }
  writeSessionJson(state.caseStorageKey, {
    bin: state.currentBin || "",
    caseName: typeof state.currentCase === "string" ? state.currentCase : CUSTOM_CASE_VALUE,
    turn: currentTurn(),
    speed: els.speed.value,
    stepUnit: state.stepUnit,
  });
}

function updatePlayButton() {
  els.playBtn.textContent = state.playing ? "■ 停止" : "▶ 再生";
  els.playBtn.classList.toggle("is-active", state.playing);
}

function stopPlayback() {
  state.playing = false;
  state.lastFrameTs = null;
  state.lastRenderTs = 0;
  if (state.rafId !== null) {
    cancelAnimationFrame(state.rafId);
    state.rafId = null;
  }
  updatePlayButton();
  saveCaseViewState();
}

function syncTurnDisplay() {
  els.turnValue.textContent = String(currentPosition());
  els.maxTurnValue.textContent = String(getMaxTurn());
}

function syncStepUnitControls() {
  const byOrder = state.stepUnit === "order";
  for (const button of els.stepUnit.querySelectorAll("[data-step-unit]")) {
    button.setAttribute("aria-pressed", String(button.dataset.stepUnit === state.stepUnit));
  }
  els.turnLabel.textContent = byOrder ? "注文" : "Turn";
  els.turnLabel.title = byOrder ? "処理済みの注文数" : "紙の移動の回数";
  els.prevBtn.title = byOrder ? "注文を1つ戻す" : "紙の移動を1回戻す";
  els.nextBtn.title = byOrder ? "注文を1つ進める" : "紙の移動を1回進める";
}

function setTextareaValue(textarea, value, kind) {
  if (kind === "input") {
    state.suppressInputDirty = true;
  } else {
    state.suppressOutputDirty = true;
  }
  textarea.value = value;
  if (kind === "input") {
    state.suppressInputDirty = false;
  } else {
    state.suppressOutputDirty = false;
  }
}

function setCustomInputCase() {
  if (state.currentCase === CUSTOM_CASE_VALUE) {
    return;
  }
  state.currentCase = CUSTOM_CASE_VALUE;
  populateCaseOptions(true);
  updateCaseButtons();
}

function setTurnMax(preferLastTurn = false) {
  // 単位や出力が変わっても同じ turn を指し続けるよう、変更前の turn で覚えておく。
  const previousTurn = Number.isFinite(state.pendingTurn) ? state.pendingTurn : currentTurn();
  state.pendingTurn = null;
  let maxTurn = 0;
  let orderEnds = [];
  if (els.outputArea.value.trim()) {
    try {
      maxTurn = Math.max(0, wasmGetMaxTurn(els.inputArea.value, els.outputArea.value));
      orderEnds = Array.from(wasmGetOrderEnds(els.inputArea.value, els.outputArea.value));
    } catch {
      maxTurn = 0;
      orderEnds = [];
    }
  }
  state.frameTurns = [0, ...orderEnds];
  if (state.frameTurns[state.frameTurns.length - 1] < maxTurn) {
    state.frameTurns.push(maxTurn);
  }
  const maxPosition = state.stepUnit === "order" ? state.frameTurns.length - 1 : maxTurn;
  els.turn.max = String(maxPosition);
  if (preferLastTurn) {
    els.turn.value = String(maxPosition);
  } else {
    els.turn.value = String(turnToPosition(Math.max(0, Math.min(previousTurn, maxTurn))));
  }
  if (maxPosition === 0) {
    stopPlayback();
  }
  syncTurnDisplay();
  saveCaseViewState();
}

function render() {
  syncTurnDisplay();
  if (!state.ready) {
    return;
  }

  const input = els.inputArea.value;
  const output = els.outputArea.value;
  const position = currentPosition();
  const turn = positionToTurn(position);

  if (!input.trim()) {
    els.score.textContent = "-";
    setElapsedMs(null);
    els.error.textContent = "";
    els.svgHost.innerHTML = "";
    return;
  }

  let ret = null;
  try {
    ret = wasmVis(input, output, turn, moveStartFor(position));
    els.svgHost.innerHTML = ret.svg;
    if (output.trim()) {
      els.score.textContent = formatScore(ret.score);
      els.elapsedTime.textContent = formatElapsedMs(state.currentElapsedMs);
      els.error.textContent = ret.err || "";
    } else {
      els.score.textContent = "-";
      els.elapsedTime.textContent = "-";
      els.error.textContent = "";
    }
  } catch (error) {
    els.svgHost.innerHTML = "";
    els.score.textContent = "0";
    els.elapsedTime.textContent = formatElapsedMs(state.currentElapsedMs);
    els.error.textContent = String(error);
  } finally {
    ret?.free();
  }
}

function renderAtMost60Hz(ts, force = false) {
  const minInterval = 1000 / MAX_RENDER_FPS;
  if (!force && ts - state.lastRenderTs < minInterval) {
    return;
  }
  state.lastRenderTs = ts;
  render();
}

function currentCaseIndex() {
  return state.cases.indexOf(state.currentCase);
}

function currentBinIndex() {
  return state.bins.indexOf(state.currentBin);
}

function wrapIndex(index, length) {
  return ((index % length) + length) % length;
}

function updateCaseButtons() {
  els.casePrevBtn.disabled = state.cases.length === 0;
  els.caseNextBtn.disabled = state.cases.length === 0;
}

function updateBinButtons() {
  els.binPrevBtn.disabled = state.bins.length === 0;
  els.binNextBtn.disabled = state.bins.length === 0;
}

function updateRunButton() {
  const runnable = state.runnableBins.has(state.currentBin);
  els.runBinBtn.disabled = state.running || !runnable || !els.inputArea.value.trim();
}

function stepTurn(delta) {
  const maxTurn = getMaxTurn();
  if (maxTurn <= 0) {
    return;
  }

  const now = Number(els.turn.value) || 0;
  const next = Math.max(0, Math.min(maxTurn, now + delta));
  if (next === now) {
    return;
  }
  els.turn.value = String(next);
  state.playbackTurnFloat = next;
  render();
  saveCaseViewState();
}

function playbackFrame(ts) {
  if (!state.playing) {
    return;
  }
  const maxTurn = getMaxTurn();
  if (maxTurn <= 0) {
    stopPlayback();
    return;
  }

  if (state.lastFrameTs == null) {
    state.lastFrameTs = ts;
    state.rafId = requestAnimationFrame(playbackFrame);
    return;
  }

  const deltaSec = (ts - state.lastFrameTs) / 1000;
  state.lastFrameTs = ts;
  const currentTurn = Number(els.turn.value) || 0;
  state.playbackTurnFloat += deltaSec * getSpeed();

  if (state.playbackTurnFloat >= maxTurn) {
    state.playbackTurnFloat = maxTurn;
    if (currentTurn !== maxTurn) {
      els.turn.value = String(maxTurn);
      renderAtMost60Hz(ts, true);
    }
    stopPlayback();
    return;
  }

  const nextTurn = Math.floor(state.playbackTurnFloat);
  if (nextTurn !== currentTurn) {
    els.turn.value = String(nextTurn);
    renderAtMost60Hz(ts);
  }

  state.rafId = requestAnimationFrame(playbackFrame);
}

function startPlayback() {
  const maxTurn = getMaxTurn();
  if (maxTurn <= 0 || state.playing) {
    return;
  }
  if ((Number(els.turn.value) || 0) >= maxTurn) {
    els.turn.value = "0";
    render();
  }
  state.playing = true;
  state.playbackTurnFloat = Number(els.turn.value) || 0;
  state.lastFrameTs = null;
  state.lastRenderTs = 0;
  updatePlayButton();
  state.rafId = requestAnimationFrame(playbackFrame);
}

function togglePlayback() {
  if (state.playing) {
    stopPlayback();
  } else {
    startPlayback();
  }
}

function populateBinOptions({ preserveSelection = false } = {}) {
  const previous = state.currentBin;
  els.solverBin.textContent = "";

  const placeholder = document.createElement("option");
  placeholder.value = "";
  placeholder.textContent = "(bin未選択)";
  els.solverBin.appendChild(placeholder);

  for (const bin of state.bins) {
    const option = document.createElement("option");
    option.value = bin;
    option.textContent = bin;
    els.solverBin.appendChild(option);
  }

  if (previous && state.bins.includes(previous)) {
    state.currentBin = previous;
  } else if (previous === "" && preserveSelection) {
    state.currentBin = previous;
  } else {
    state.currentBin = state.bins[0] ?? "";
    state.preserveEmptyBinSelection = state.currentBin === "";
  }
  els.solverBin.value = state.currentBin;
  updateBinButtons();
}

function populateCaseOptions(includeCustom = state.currentCase === CUSTOM_CASE_VALUE) {
  const previous = state.currentCase;
  els.caseName.textContent = "";

  if (includeCustom) {
    const custom = document.createElement("option");
    custom.value = CUSTOM_CASE_VALUE;
    custom.textContent = "custom input";
    els.caseName.appendChild(custom);
  }

  for (const caseName of state.cases) {
    const option = document.createElement("option");
    option.value = caseName;
    option.textContent = caseName;
    els.caseName.appendChild(option);
  }

  if (previous === CUSTOM_CASE_VALUE && includeCustom) {
    state.currentCase = CUSTOM_CASE_VALUE;
  } else if (previous && state.cases.includes(previous)) {
    state.currentCase = previous;
  } else {
    state.currentCase = state.cases[0] ?? CUSTOM_CASE_VALUE;
  }
  els.caseName.value = state.currentCase;
  updateCaseButtons();
}

async function loadVisualizerData() {
  const res = await fetch("/api/visualizer-data");
  if (!res.ok) {
    throw new Error(`HTTP ${res.status}`);
  }
  return res.json();
}

async function refreshVisualizerData({ reloadCase = false } = {}) {
  const data = await loadVisualizerData();
  configureCaseStorage(data.projectKey);
  state.bins = Array.isArray(data.bins) ? data.bins : [];
  state.runnableBins = new Set(Array.isArray(data.runnableBins) ? data.runnableBins : []);
  state.cases = Array.isArray(data.cases) ? data.cases : [];
  populateBinOptions({ preserveSelection: state.preserveEmptyBinSelection });
  populateCaseOptions(state.currentCase === CUSTOM_CASE_VALUE);
  updateRunButton();
  setRunStatus("一覧を更新した");
  if (reloadCase && state.currentCase !== CUSTOM_CASE_VALUE) {
    await loadSelectedCase(true);
  }
}

async function pollBinsQuietly() {
  if (state.binPollInFlight || document.hidden) {
    return;
  }
  state.binPollInFlight = true;
  try {
    const data = await loadVisualizerData();
    const nextBins = Array.isArray(data.bins) ? data.bins : [];
    const binsChanged = nextBins.length !== state.bins.length
      || nextBins.some((bin, index) => bin !== state.bins[index]);
    if (binsChanged) {
      const previousBin = state.currentBin;
      // 改名・削除も反映し、一覧と表示中の出力が別の実験を指す状態を避ける。
      state.bins = nextBins;
      populateBinOptions({ preserveSelection: true });
      if (state.currentBin !== previousBin && state.currentCase !== CUSTOM_CASE_VALUE) {
        await loadSelectedCase(true);
      }
    }
    if (Array.isArray(data.runnableBins)) {
      state.runnableBins = new Set(data.runnableBins);
      updateRunButton();
    }
  } catch {
    // Silent polling must never disturb the current visualizer state.
  } finally {
    state.binPollInFlight = false;
  }
}

async function loadSelectedCase(preferLastTurn = true) {
  if (!state.currentCase) {
    updateCaseButtons();
    updateRunButton();
    setElapsedMs(null);
    setTurnMax(false);
    setRunStatus("custom input を表示中");
    return;
  }

  stopPlayback();
  const seq = ++state.loadSeq;
  setRunStatus(`${state.currentCase} を読込中...`);

  try {
    const params = new URLSearchParams({
      caseName: state.currentCase,
      binName: state.currentBin,
    });
    const res = await fetch(`/api/visualizer-case?${params.toString()}`);
    if (!res.ok) {
      const data = await res.json().catch(() => ({}));
      throw new Error(data.error || `HTTP ${res.status}`);
    }
    const data = await res.json();
    if (seq !== state.loadSeq) {
      return;
    }

    setTextareaValue(els.inputArea, data.input ?? "", "input");
    setTextareaValue(els.outputArea, data.output ?? "", "output");
    setElapsedMs(data.elapsedMs);
    setTurnMax(preferLastTurn);
    render();
    updateRunButton();

    if (data.outputExists) {
      setRunStatus(`${state.currentCase} を読込: ${state.currentBin || "(bin未選択)"} の既存 output を反映`);
    } else if (state.currentBin) {
      setRunStatus(`${state.currentCase} を読込: 既存 output は未作成`);
    } else {
      setRunStatus(`${state.currentCase} を読込: bin 未選択のため input のみ表示`);
    }
  } catch (error) {
    if (seq !== state.loadSeq) {
      return;
    }
    setTextareaValue(els.inputArea, "", "input");
    setTextareaValue(els.outputArea, "", "output");
    setElapsedMs(null);
    setTurnMax(false);
    render();
    updateRunButton();
    setRunStatus(`case 読込失敗: ${String(error)}`, true);
  }
}

async function runSelectedSolver() {
  if (state.running || !state.runnableBins.has(state.currentBin)) {
    return;
  }
  if (!els.inputArea.value.trim()) {
    setRunStatus("Input が空なので実行できない", true);
    return;
  }

  stopPlayback();
  state.running = true;
  updateRunButton();
  setRunStatus(`${state.currentBin} を tester 経由で実行中...`);

  try {
    const res = await fetch("/api/run-solver", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        binName: state.currentBin,
        caseName: state.currentCase,
        input: els.inputArea.value,
      }),
    });
    const data = await res.json();
    if (!res.ok) {
      throw new Error(data.error || `HTTP ${res.status}`);
    }

    setTextareaValue(els.outputArea, data.output ?? "", "output");
    setElapsedMs(data.elapsedMs);
    setTurnMax(true);
    render();
    const saved = data.savedOutputPath ? ` / saved=${data.savedOutputPath}` : "";
    const elapsed = typeof data.elapsedMs === "number" ? ` (${data.elapsedMs} ms)` : "";
    setRunStatus(`${state.currentBin} 実行完了${elapsed}${saved}`);
  } catch (error) {
    setRunStatus(`実行失敗: ${String(error)}`, true);
  } finally {
    state.running = false;
    updateRunButton();
  }
}

function moveCase(delta) {
  if (state.cases.length === 0) {
    return;
  }
  const idx = currentCaseIndex();
  const nextIdx = idx < 0 ? (delta > 0 ? 0 : state.cases.length - 1) : wrapIndex(idx + delta, state.cases.length);
  if (nextIdx === idx) {
    return;
  }
  state.currentCase = state.cases[nextIdx];
  populateCaseOptions(false);
  saveCaseViewState();
  void loadSelectedCase(true);
}

function moveBin(delta) {
  if (state.bins.length === 0) {
    return;
  }
  const idx = currentBinIndex();
  const nextIdx = idx < 0 ? (delta > 0 ? 0 : state.bins.length - 1) : wrapIndex(idx + delta, state.bins.length);
  if (nextIdx === idx) {
    return;
  }
  state.currentBin = state.bins[nextIdx];
  state.preserveEmptyBinSelection = false;
  els.solverBin.value = state.currentBin;
  updateBinButtons();
  updateRunButton();
  if (state.currentCase !== CUSTOM_CASE_VALUE) {
    void loadSelectedCase(true);
  }
  saveCaseViewState();
}

function setupHoldButton(button, delta) {
  let delayId = null;
  let intervalId = null;
  const clear = () => {
    if (delayId !== null) {
      clearTimeout(delayId);
      delayId = null;
    }
    if (intervalId !== null) {
      clearInterval(intervalId);
      intervalId = null;
    }
  };
  button.addEventListener("pointerdown", (event) => {
    if (button.disabled) {
      return;
    }
    event.preventDefault();
    stopPlayback();
    stepTurn(delta);
    delayId = window.setTimeout(() => {
      intervalId = window.setInterval(() => stepTurn(delta), HOLD_INTERVAL_MS);
    }, HOLD_DELAY_MS);
  });
  for (const eventName of ["pointerup", "pointerleave", "pointercancel", "blur"]) {
    button.addEventListener(eventName, clear);
  }
}

async function copyText(text, label) {
  try {
    await navigator.clipboard.writeText(text);
    setRunStatus(`${label} をコピーした`);
  } catch {
    setRunStatus(`${label} のコピーに失敗した`, true);
  }
}

els.refreshBtn.addEventListener("click", () => {
  void refreshVisualizerData({ reloadCase: true }).catch((error) =>
    setRunStatus(`更新失敗: ${String(error)}`, true),
  );
});

els.solverBin.addEventListener("change", () => {
  state.currentBin = els.solverBin.value;
  state.preserveEmptyBinSelection = state.currentBin === "";
  updateBinButtons();
  updateRunButton();
  if (state.currentCase !== CUSTOM_CASE_VALUE) {
    void loadSelectedCase(true);
  }
  saveCaseViewState();
});

els.binPrevBtn.addEventListener("click", () => moveBin(-1));
els.binNextBtn.addEventListener("click", () => moveBin(1));

els.caseName.addEventListener("change", () => {
  state.currentCase = els.caseName.value;
  updateCaseButtons();
  if (state.currentCase !== CUSTOM_CASE_VALUE) {
    void loadSelectedCase(true);
  }
  saveCaseViewState();
});

els.casePrevBtn.addEventListener("click", () => moveCase(-1));
els.caseNextBtn.addEventListener("click", () => moveCase(1));

els.runBinBtn.addEventListener("click", () => {
  void runSelectedSolver();
});

setupHoldButton(els.prevBtn, -1);
setupHoldButton(els.nextBtn, 1);

els.playBtn.addEventListener("click", togglePlayback);

els.turn.addEventListener("input", () => {
  stopPlayback();
  state.playbackTurnFloat = Number(els.turn.value) || 0;
  render();
  saveCaseViewState();
});

els.speed.addEventListener("change", () => {
  saveCaseViewState();
});

els.stepUnit.addEventListener("click", (event) => {
  const button = event.target.closest("[data-step-unit]");
  if (!button || button.dataset.stepUnit === state.stepUnit) {
    return;
  }
  stopPlayback();
  state.pendingTurn = currentTurn();
  state.stepUnit = button.dataset.stepUnit;
  syncStepUnitControls();
  setTurnMax(false);
  state.playbackTurnFloat = currentPosition();
  render();
  saveCaseViewState();
});

// 公式 SVG の注文グラフから、1 手ずつならその注文の開始時点へ、注文単位ならその注文の結果へ移動する。
els.svgHost.addEventListener("click", (event) => {
  const order = event.target.closest("[data-turn]");
  if (!order) return;
  let position = Number(order.dataset.turn);
  if (state.stepUnit === "order") {
    // 0 手の注文が続くと開始 turn から注文を特定できないので、公式の title "order r: ..." から読む。
    const match = /^order (\d+):/.exec(order.querySelector("title")?.textContent ?? "");
    if (!match) return;
    position = Number(match[1]) + 1;
  }
  if (!Number.isInteger(position) || position < 0 || position > getMaxTurn()) return;
  stopPlayback();
  els.turn.value = String(position);
  state.playbackTurnFloat = position;
  render();
  saveCaseViewState();
});

els.inputArea.addEventListener("input", () => {
  if (state.suppressInputDirty) {
    return;
  }
  stopPlayback();
  setCustomInputCase();
  setElapsedMs(null);
  setTurnMax(false);
  updateRunButton();
  render();
  saveCaseViewState();
});

els.outputArea.addEventListener("input", () => {
  if (state.suppressOutputDirty) {
    return;
  }
  stopPlayback();
  setElapsedMs(null);
  setTurnMax(false);
  render();
  saveCaseViewState();
});

window.addEventListener("pagehide", () => {
  saveCaseViewState();
});

els.copyInputBtn.addEventListener("click", () => {
  void copyText(els.inputArea.value, "Input");
});

els.copyOutputBtn.addEventListener("click", () => {
  void copyText(els.outputArea.value, "Output");
});

document.addEventListener("keydown", (event) => {
  const activeTag = document.activeElement?.tagName ?? "";
  if (["TEXTAREA", "INPUT", "SELECT"].includes(activeTag)) {
    return;
  }
  if (event.code === "Space") {
    event.preventDefault();
    togglePlayback();
  } else if (event.code === "ArrowLeft") {
    event.preventDefault();
    stopPlayback();
    stepTurn(-1);
  } else if (event.code === "ArrowRight") {
    event.preventDefault();
    stopPlayback();
    stepTurn(1);
  }
});

async function main() {
  wasmGetMaxTurn = wasmGetMaxTurnModule;
  wasmGetOrderEnds = wasmGetOrderEndsModule;
  wasmVis = wasmVisModule;
  await init();
  state.ready = true;
  updatePlayButton();
  syncStepUnitControls();
  await refreshVisualizerData();
  await loadSelectedCase(true);
  window.setInterval(() => {
    void pollBinsQuietly();
  }, BIN_POLL_INTERVAL_MS);
}

main().catch((error) => {
  setRunStatus(`初期化失敗: ${String(error)}`, true);
  els.error.textContent = String(error);
});
