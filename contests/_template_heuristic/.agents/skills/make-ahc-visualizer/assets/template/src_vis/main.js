import init, {
  get_max_turn as wasmGetMaxTurnModule,
  vis as wasmVisModule,
} from "./wasm/heuristic_contest_template_vis.js";

let wasmGetMaxTurn = null;
let wasmVis = null;

const CUSTOM_CASE_VALUE = "";
const MAX_RENDER_FPS = 60;
const HOLD_DELAY_MS = 500;
const HOLD_INTERVAL_MS = 55;
const BIN_POLL_INTERVAL_MS = 10_000;

const els = {
  refreshBtn: document.getElementById("refreshBtn"),
  rustBin: document.getElementById("rustBin"),
  binPrevBtn: document.getElementById("binPrevBtn"),
  binNextBtn: document.getElementById("binNextBtn"),
  caseName: document.getElementById("caseName"),
  casePrevBtn: document.getElementById("casePrevBtn"),
  caseNextBtn: document.getElementById("caseNextBtn"),
  runBinBtn: document.getElementById("runBinBtn"),
  runStatus: document.getElementById("runStatus"),
  turn: document.getElementById("turn"),
  turnValue: document.getElementById("turnValue"),
  maxTurnValue: document.getElementById("maxTurnValue"),
  prevBtn: document.getElementById("prevBtn"),
  playBtn: document.getElementById("playBtn"),
  nextBtn: document.getElementById("nextBtn"),
  speed: document.getElementById("speed"),
  score: document.getElementById("score"),
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
  currentBin: "",
  currentCase: null,
  loadSeq: 0,
  suppressInputDirty: false,
  suppressOutputDirty: false,
  binPollInFlight: false,
};

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

function setRunStatus(text, isError = false) {
  els.runStatus.textContent = text;
  els.runStatus.style.color = isError ? "#b91c1c" : "#5b6472";
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
}

function syncTurnDisplay() {
  els.turnValue.textContent = String(Number(els.turn.value) || 0);
  els.maxTurnValue.textContent = String(getMaxTurn());
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
  let maxTurn = 0;
  if (els.outputArea.value.trim()) {
    try {
      maxTurn = Math.max(1, wasmGetMaxTurn(els.inputArea.value, els.outputArea.value));
    } catch {
      maxTurn = 1;
    }
  }
  els.turn.max = String(maxTurn);
  if (preferLastTurn) {
    els.turn.value = String(maxTurn);
  } else {
    const current = Number(els.turn.value) || 0;
    els.turn.value = String(Math.min(current, maxTurn));
  }
  if (maxTurn === 0) {
    stopPlayback();
  }
  syncTurnDisplay();
}

function render() {
  syncTurnDisplay();
  if (!state.ready) {
    return;
  }

  const input = els.inputArea.value;
  const output = els.outputArea.value;
  const turn = Number(els.turn.value) || 0;

  if (!input.trim()) {
    els.score.textContent = "-";
    els.error.textContent = "";
    els.svgHost.innerHTML = "";
    return;
  }

  let ret = null;
  try {
    ret = wasmVis(input, output, turn);
    els.svgHost.innerHTML = ret.svg;
    if (output.trim()) {
      els.score.textContent = formatScore(ret.score);
      els.error.textContent = ret.err || "";
    } else {
      els.score.textContent = "-";
      els.error.textContent = "";
    }
  } catch (error) {
    els.svgHost.innerHTML = "";
    els.score.textContent = "0";
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
  els.rustBin.textContent = "";

  const placeholder = document.createElement("option");
  placeholder.value = "";
  placeholder.textContent = "(bin未選択)";
  els.rustBin.appendChild(placeholder);

  for (const bin of state.bins) {
    const option = document.createElement("option");
    option.value = bin;
    option.textContent = bin;
    els.rustBin.appendChild(option);
  }

  if (previous && state.bins.includes(previous)) {
    state.currentBin = previous;
  } else if (preserveSelection) {
    state.currentBin = previous;
  } else {
    state.currentBin = state.bins[0] ?? "";
  }
  els.rustBin.value = state.currentBin;
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
  state.bins = Array.isArray(data.bins) ? data.bins : [];
  state.runnableBins = new Set(Array.isArray(data.runnableBins) ? data.runnableBins : []);
  state.cases = Array.isArray(data.cases) ? data.cases : [];
  populateBinOptions();
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
    const knownBins = new Set(state.bins);
    const addedBins = nextBins.filter((bin) => !knownBins.has(bin));
    if (addedBins.length > 0) {
      state.bins = [...state.bins, ...addedBins];
      populateBinOptions({ preserveSelection: true });
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
    setTurnMax(false);
    render();
    updateRunButton();
    setRunStatus(`case 読込失敗: ${String(error)}`, true);
  }
}

async function runSelectedRustBin() {
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
    const res = await fetch("/api/run-rust-bin", {
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
  els.rustBin.value = state.currentBin;
  updateBinButtons();
  updateRunButton();
  if (state.currentCase !== CUSTOM_CASE_VALUE) {
    void loadSelectedCase(true);
  }
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

els.rustBin.addEventListener("change", () => {
  state.currentBin = els.rustBin.value;
  updateBinButtons();
  updateRunButton();
  if (state.currentCase !== CUSTOM_CASE_VALUE) {
    void loadSelectedCase(true);
  }
});

els.binPrevBtn.addEventListener("click", () => moveBin(-1));
els.binNextBtn.addEventListener("click", () => moveBin(1));

els.caseName.addEventListener("change", () => {
  state.currentCase = els.caseName.value;
  updateCaseButtons();
  if (state.currentCase !== CUSTOM_CASE_VALUE) {
    void loadSelectedCase(true);
  }
});

els.casePrevBtn.addEventListener("click", () => moveCase(-1));
els.caseNextBtn.addEventListener("click", () => moveCase(1));

els.runBinBtn.addEventListener("click", () => {
  void runSelectedRustBin();
});

setupHoldButton(els.prevBtn, -1);
setupHoldButton(els.nextBtn, 1);

els.playBtn.addEventListener("click", togglePlayback);

els.turn.addEventListener("input", () => {
  stopPlayback();
  state.playbackTurnFloat = Number(els.turn.value) || 0;
  render();
});

els.inputArea.addEventListener("input", () => {
  if (state.suppressInputDirty) {
    return;
  }
  stopPlayback();
  setCustomInputCase();
  setTurnMax(false);
  updateRunButton();
  render();
});

els.outputArea.addEventListener("input", () => {
  if (state.suppressOutputDirty) {
    return;
  }
  stopPlayback();
  setTurnMax(false);
  render();
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
  wasmVis = wasmVisModule;
  await init();
  state.ready = true;
  updatePlayButton();
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
