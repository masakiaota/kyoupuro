import wasmInitModule, {
  get_max_turn as wasmGetMaxTurnModule,
  vis as wasmVisModule,
} from "../public/wasm/heuristic_contest_template_vis.js";

let wasmInit = null;
let wasmGetMaxTurn = null;
let wasmVis = null;

const els = {
  rustBin: document.getElementById("rustBin"),
  refreshBinsBtn: document.getElementById("refreshBinsBtn"),
  inputDataset: document.getElementById("inputDataset"),
  inputCase: document.getElementById("inputCase"),
  refreshCasesBtn: document.getElementById("refreshCasesBtn"),
  prevCaseBtn: document.getElementById("prevCaseBtn"),
  nextCaseBtn: document.getElementById("nextCaseBtn"),
  runBinBtn: document.getElementById("runBinBtn"),
  runStatus: document.getElementById("runStatus"),
  turn: document.getElementById("turn"),
  turnValue: document.getElementById("turnValue"),
  maxTurnValue: document.getElementById("maxTurnValue"),
  prevBtn: document.getElementById("prevBtn"),
  playBtn: document.getElementById("playBtn"),
  nextBtn: document.getElementById("nextBtn"),
  speed: document.getElementById("speed"),
  loopPlay: document.getElementById("loopPlay"),
  score: document.getElementById("score"),
  error: document.getElementById("error"),
  inputArea: document.getElementById("inputArea"),
  outputArea: document.getElementById("outputArea"),
  svgHost: document.getElementById("svgHost"),
};

let ready = false;
let playing = false;
let playbackTimer = null;
let loadingPair = false;
let pairLoadToken = 0;
let runningSolver = false;
let runToken = 0;

function currentDataset() {
  return (els.inputDataset.value || "in").trim();
}

function getMaxTurn() {
  return Number(els.turn.max) || 0;
}

function getSpeed() {
  const v = Number(els.speed.value);
  return Number.isFinite(v) && v > 0 ? v : 5;
}

function updatePlayButton() {
  els.playBtn.textContent = playing ? "停止" : "再生";
  els.playBtn.classList.toggle("is-active", playing);
}

function stopPlayback() {
  playing = false;
  if (playbackTimer !== null) {
    clearInterval(playbackTimer);
    playbackTimer = null;
  }
  updatePlayButton();
}

function stepTurn(delta) {
  const maxTurn = getMaxTurn();
  if (maxTurn <= 0) return;

  const now = Number(els.turn.value);
  let next = now + delta;
  if (delta > 0) {
    if (next > maxTurn) {
      next = els.loopPlay.checked ? 0 : maxTurn;
    }
  } else if (next < 0) {
    next = els.loopPlay.checked ? maxTurn : 0;
  }
  els.turn.value = String(next);
  render();
}

function startPlayback() {
  const maxTurn = getMaxTurn();
  if (maxTurn <= 0 || playing) return;

  playing = true;
  updatePlayButton();
  playbackTimer = setInterval(() => {
    const now = Number(els.turn.value);
    if (now >= getMaxTurn()) {
      if (els.loopPlay.checked) {
        els.turn.value = "0";
        render();
      } else {
        stopPlayback();
      }
      return;
    }
    stepTurn(1);
  }, 1000 / getSpeed());
}

function restartPlaybackIfNeeded() {
  if (!playing) return;
  stopPlayback();
  startPlayback();
}

function setTurnMax(options = {}) {
  const forceToEnd = options.forceToEnd === true;
  const input = els.inputArea.value;
  const output = els.outputArea.value;

  let mx = 0;
  if (ready && output.trim().length > 0) {
    try {
      mx = Math.max(1, wasmGetMaxTurn(input, output));
    } catch {
      mx = 1;
    }
  }

  els.turn.max = String(mx);
  if (forceToEnd) {
    els.turn.value = String(mx);
  } else if (Number(els.turn.value) > mx) {
    els.turn.value = String(mx);
  }

  if (mx === 0) {
    stopPlayback();
    els.turn.value = "0";
  }

  els.turnValue.textContent = els.turn.value;
  els.maxTurnValue.textContent = String(mx);
}

function render() {
  els.turnValue.textContent = els.turn.value;

  if (!ready) return;
  const input = els.inputArea.value;
  const output = els.outputArea.value;
  const turn = Number(els.turn.value);

  if (!input.trim() || !output.trim()) {
    els.score.textContent = "-";
    els.error.textContent = "input/output が揃うと描画される";
    els.svgHost.innerHTML = "";
    return;
  }

  try {
    const ret = wasmVis(input, output, turn);
    els.score.textContent = String(ret.score);
    els.error.textContent = ret.err || "";
    els.svgHost.innerHTML = ret.svg;
    ret.free();
  } catch (e) {
    els.score.textContent = "0";
    els.error.textContent = String(e);
    els.svgHost.innerHTML = "";
  }
}

function setStatus(text, isError = false) {
  els.runStatus.textContent = text;
  els.runStatus.style.color = isError ? "#b91c1c" : "#4b5563";
}

function updateRunButton() {
  const hasBin = (els.rustBin.value || "").trim().length > 0;
  const hasInput = els.inputArea.value.trim().length > 0;
  els.runBinBtn.disabled = loadingPair || runningSolver || !hasBin || !hasInput;
}

function syncControlDisabledState() {
  const disabled = loadingPair || runningSolver;
  els.refreshBinsBtn.disabled = disabled;
  els.refreshCasesBtn.disabled = disabled;
  els.rustBin.disabled = disabled;
  els.inputDataset.disabled = disabled;
  els.inputCase.disabled = disabled;
  updateCaseNavButtons();
  updateRunButton();
}

function getInputCaseValues() {
  return Array.from(els.inputCase.options)
    .map((opt) => opt.value)
    .filter((v) => v.length > 0);
}

function updateCaseNavButtons() {
  const cases = getInputCaseValues();
  const idx = cases.indexOf(els.inputCase.value);
  const invalid =
    cases.length === 0 ||
    idx < 0 ||
    loadingPair ||
    runningSolver ||
    els.inputCase.disabled;
  els.prevCaseBtn.disabled = invalid || idx === 0;
  els.nextCaseBtn.disabled = invalid || idx === cases.length - 1;
}

function setSelectOptions(selectEl, values, emptyLabel) {
  const prev = selectEl.value;
  selectEl.innerHTML = "";

  for (const value of values) {
    const opt = document.createElement("option");
    opt.value = value;
    opt.textContent = value;
    selectEl.appendChild(opt);
  }

  if (values.length === 0) {
    const opt = document.createElement("option");
    opt.value = "";
    opt.textContent = emptyLabel;
    selectEl.appendChild(opt);
    selectEl.value = "";
    return false;
  }

  if (prev && values.includes(prev)) {
    selectEl.value = prev;
  } else {
    selectEl.value = values[0];
  }
  return true;
}

async function loadRustBins() {
  try {
    const res = await fetch("/api/rust-bins");
    if (!res.ok) {
      throw new Error(`HTTP ${res.status}`);
    }
    const data = await res.json();
    const bins = Array.isArray(data.bins) ? data.bins : [];
    const ok = setSelectOptions(els.rustBin, bins, "(src/bin/*.rs が無い)");
    if (!ok) {
      setStatus("src/bin/*.rs が見つからない", true);
      updateRunButton();
      return false;
    }
    setStatus(`bin一覧を取得: ${bins.length}件`);
    updateRunButton();
    return true;
  } catch (e) {
    setSelectOptions(els.rustBin, [], "(取得失敗)");
    setStatus(`bin一覧取得に失敗: ${String(e)}`, true);
    updateRunButton();
    return false;
  }
}

async function loadInputCases() {
  const dataset = currentDataset();
  try {
    const params = new URLSearchParams({ dataset });
    const res = await fetch(`/api/in-cases?${params.toString()}`);
    if (!res.ok) {
      throw new Error(`HTTP ${res.status}`);
    }
    const data = await res.json();
    const cases = Array.isArray(data.cases) ? data.cases : [];
    const ok = setSelectOptions(
      els.inputCase,
      cases,
      `(tools/${dataset} が空)`,
    );
    if (!ok) {
      setStatus(`tools/${dataset} にケースが無い`, true);
      updateCaseNavButtons();
      updateRunButton();
      return false;
    }
    setStatus(`${dataset} 一覧を取得: ${cases.length}件`);
    updateCaseNavButtons();
    updateRunButton();
    return true;
  } catch (e) {
    setSelectOptions(els.inputCase, [], "(取得失敗)");
    setStatus(`${dataset} 一覧取得に失敗: ${String(e)}`, true);
    updateCaseNavButtons();
    updateRunButton();
    return false;
  }
}

async function loadSelectedPair(options = {}) {
  const setTurnToEnd = options.setTurnToEnd === true;
  const binName = (els.rustBin.value || "").trim();
  const dataset = currentDataset();
  const caseName = (els.inputCase.value || "").trim();

  if (!binName || !caseName) {
    els.inputArea.value = "";
    els.outputArea.value = "";
    setTurnMax({ forceToEnd: true });
    render();
    updateRunButton();
    return;
  }

  const myToken = ++pairLoadToken;
  loadingPair = true;
  syncControlDisabledState();

  try {
    const params = new URLSearchParams({
      dataset,
      bin: binName,
      case: caseName,
    });
    const res = await fetch(`/api/vis-pair?${params.toString()}`);
    const data = await res.json();
    if (!res.ok) {
      throw new Error(data.error || `HTTP ${res.status}`);
    }

    if (myToken !== pairLoadToken) {
      return;
    }

    els.inputArea.value = data.input ?? "";
    els.outputArea.value = data.hasOutput ? (data.output ?? "") : "";

    setTurnMax({ forceToEnd: setTurnToEnd });
    render();
    updateRunButton();

    if (data.hasOutput) {
      setStatus(`${binName} / ${dataset} / ${caseName} を表示中`);
    } else if (dataset === "in_generated") {
      setStatus(`${binName} / ${dataset} / ${caseName} を読込済み。output は未実行`);
    } else {
      setStatus(
        `${binName} / ${dataset} / ${caseName} の output が無い（results/out/${binName}/${caseName}）`,
        true,
      );
    }
  } catch (e) {
    if (myToken !== pairLoadToken) {
      return;
    }
    els.outputArea.value = "";
    setTurnMax({ forceToEnd: true });
    render();
    updateRunButton();
    setStatus(`in/out 読込に失敗: ${String(e)}`, true);
  } finally {
    if (myToken === pairLoadToken) {
      loadingPair = false;
      syncControlDisabledState();
    }
  }
}

async function runSelectedBin() {
  if (loadingPair || runningSolver) return;

  const binName = (els.rustBin.value || "").trim();
  const dataset = currentDataset();
  const caseName = (els.inputCase.value || "").trim();
  const input = els.inputArea.value;
  if (!binName || !input.trim()) {
    setStatus("bin と input が必要", true);
    updateRunButton();
    return;
  }

  const labelCase = caseName || "(manual)";
  const myToken = ++runToken;
  runningSolver = true;
  stopPlayback();
  syncControlDisabledState();
  setStatus(`${binName} / ${dataset} / ${labelCase} を実行中...`);

  try {
    const res = await fetch("/api/run-rust-bin", {
      method: "POST",
      headers: {
        "Content-Type": "application/json; charset=utf-8",
      },
      body: JSON.stringify({
        binName,
        input,
      }),
    });
    const data = await res.json();
    if (!res.ok) {
      throw new Error(data.error || `HTTP ${res.status}`);
    }
    if (myToken !== runToken) {
      return;
    }

    els.outputArea.value = data.output ?? "";
    setTurnMax({ forceToEnd: true });
    render();
    updateRunButton();

    const elapsedMs = Number(data.elapsedMs);
    const stderr = typeof data.stderr === "string" ? data.stderr.trim() : "";
    const elapsedText = Number.isFinite(elapsedMs) ? `${elapsedMs}ms` : "?ms";
    const suffix = stderr.length > 0 ? " / stderrあり" : "";
    setStatus(
      `${binName} / ${dataset} / ${labelCase} を実行: ${elapsedText}${suffix}`,
    );
  } catch (e) {
    if (myToken !== runToken) {
      return;
    }
    setStatus(`bin 実行に失敗: ${String(e)}`, true);
  } finally {
    if (myToken === runToken) {
      runningSolver = false;
      syncControlDisabledState();
    }
  }
}

async function moveInputCase(delta) {
  if (loadingPair || runningSolver) return;
  const cases = getInputCaseValues();
  if (cases.length === 0) return;
  const idx = cases.indexOf(els.inputCase.value);
  if (idx < 0) return;
  const next = Math.max(0, Math.min(cases.length - 1, idx + delta));
  if (next === idx) {
    updateCaseNavButtons();
    return;
  }
  els.inputCase.value = cases[next];
  updateCaseNavButtons();
  stopPlayback();
  await loadSelectedPair({ setTurnToEnd: true });
}

els.refreshBinsBtn.addEventListener("click", async () => {
  if (loadingPair || runningSolver) return;
  stopPlayback();
  const ok = await loadRustBins();
  if (ok) {
    await loadSelectedPair({ setTurnToEnd: true });
  }
});

els.refreshCasesBtn.addEventListener("click", async () => {
  if (loadingPair || runningSolver) return;
  stopPlayback();
  const ok = await loadInputCases();
  if (ok) {
    await loadSelectedPair({ setTurnToEnd: true });
  } else {
    els.inputArea.value = "";
    els.outputArea.value = "";
    setTurnMax({ forceToEnd: true });
    render();
  }
});

els.rustBin.addEventListener("change", async () => {
  if (loadingPair || runningSolver) return;
  stopPlayback();
  await loadSelectedPair({ setTurnToEnd: true });
});

els.inputDataset.addEventListener("change", async () => {
  if (loadingPair || runningSolver) return;
  stopPlayback();
  const ok = await loadInputCases();
  if (ok) {
    await loadSelectedPair({ setTurnToEnd: true });
  } else {
    els.inputArea.value = "";
    els.outputArea.value = "";
    setTurnMax({ forceToEnd: true });
    render();
  }
});

els.inputCase.addEventListener("change", async () => {
  if (loadingPair || runningSolver) return;
  updateCaseNavButtons();
  stopPlayback();
  await loadSelectedPair({ setTurnToEnd: true });
});

els.prevCaseBtn.addEventListener("click", async () => {
  await moveInputCase(-1);
});

els.nextCaseBtn.addEventListener("click", async () => {
  await moveInputCase(1);
});

els.prevBtn.addEventListener("click", () => {
  stopPlayback();
  stepTurn(-1);
});

els.nextBtn.addEventListener("click", () => {
  stopPlayback();
  stepTurn(1);
});

els.playBtn.addEventListener("click", () => {
  if (playing) {
    stopPlayback();
  } else {
    startPlayback();
  }
});

els.speed.addEventListener("change", () => {
  restartPlaybackIfNeeded();
});

els.runBinBtn.addEventListener("click", async () => {
  await runSelectedBin();
});

els.inputArea.addEventListener("input", () => {
  stopPlayback();
  setTurnMax();
  render();
  updateRunButton();
});

els.outputArea.addEventListener("input", () => {
  stopPlayback();
  setTurnMax();
  render();
  updateRunButton();
});

els.turn.addEventListener("input", () => {
  render();
});

els.loopPlay.addEventListener("change", () => {
  restartPlaybackIfNeeded();
});

document.addEventListener("keydown", (e) => {
  const activeTag = document.activeElement?.tagName ?? "";
  const inEditor =
    activeTag === "TEXTAREA" ||
    activeTag === "INPUT" ||
    activeTag === "SELECT";

  if (inEditor) return;

  if (e.code === "Space") {
    e.preventDefault();
    if (playing) {
      stopPlayback();
    } else {
      startPlayback();
    }
  } else if (e.code === "ArrowRight") {
    e.preventDefault();
    stopPlayback();
    stepTurn(1);
  } else if (e.code === "ArrowLeft") {
    e.preventDefault();
    stopPlayback();
    stepTurn(-1);
  }
});

async function main() {
  wasmInit = wasmInitModule;
  wasmGetMaxTurn = wasmGetMaxTurnModule;
  wasmVis = wasmVisModule;
  await wasmInit();

  ready = true;
  updatePlayButton();

  const [binOk, caseOk] = await Promise.all([loadRustBins(), loadInputCases()]);
  if (binOk && caseOk) {
    await loadSelectedPair({ setTurnToEnd: true });
  } else {
    setTurnMax({ forceToEnd: true });
    render();
  }
  syncControlDisabledState();
}

main().catch((e) => {
  els.error.textContent = `WASM初期化失敗: ${String(e)}`;
  setStatus(`WASM初期化失敗: ${String(e)}`, true);
});
