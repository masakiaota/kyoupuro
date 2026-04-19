let wasmInit = null;
let wasmGetMaxTurn = null;
let wasmVis = null;

const KNOWN_DATASETS = ["in", "inB", "in_generated"];
const WASM_JS_PATH = "/wasm/heuristic_contest_template_vis.js";
const WASM_BG_PATH = "/wasm/heuristic_contest_template_vis_bg.wasm";

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
let playbackRafId = null;
let playbackLastTimestamp = null;
let playbackTurnCarry = 0;
let loadingPair = false;
let pairLoadToken = 0;
let runningSolver = false;
let runToken = 0;

function currentDataset() {
  return (els.inputDataset.value || "in").trim();
}

function getSelectValues(selectEl) {
  return Array.from(selectEl.options)
    .map((opt) => opt.value)
    .filter((value) => value.length > 0);
}

function getMaxTurn() {
  return Number(els.turn.max) || 0;
}

function getSpeed() {
  const v = Number(els.speed.value);
  return Number.isFinite(v) && v > 0 ? v : 100;
}

function getPlaybackFps() {
  return getSpeed() > 60 ? 30 : 60;
}

function updatePlayButton() {
  els.playBtn.textContent = playing ? "停止" : "再生";
  els.playBtn.classList.toggle("is-active", playing);
}

function stopPlayback() {
  playing = false;
  if (playbackRafId !== null) {
    cancelAnimationFrame(playbackRafId);
    playbackRafId = null;
  }
  playbackLastTimestamp = null;
  playbackTurnCarry = 0;
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
  playbackLastTimestamp = null;
  playbackTurnCarry = 0;
  updatePlayButton();

  const tick = (timestamp) => {
    if (!playing) return;

    const maxTurn = getMaxTurn();
    if (maxTurn <= 0) {
      stopPlayback();
      return;
    }

    if (playbackLastTimestamp === null) {
      playbackLastTimestamp = timestamp;
      playbackRafId = requestAnimationFrame(tick);
      return;
    }

    const elapsed = timestamp - playbackLastTimestamp;
    const minFrameMs = 1000 / getPlaybackFps();
    if (elapsed < minFrameMs) {
      playbackRafId = requestAnimationFrame(tick);
      return;
    }

    playbackLastTimestamp = timestamp;
    playbackTurnCarry += (getSpeed() * elapsed) / 1000;

    const steps = Math.floor(playbackTurnCarry);
    playbackTurnCarry -= steps;

    if (steps > 0) {
      const now = Number(els.turn.value);
      let next = now;
      let shouldStop = false;

      if (els.loopPlay.checked) {
        const cycle = maxTurn + 1;
        next = (now + steps) % cycle;
      } else {
        next = Math.min(maxTurn, now + steps);
        shouldStop = next >= maxTurn;
      }

      if (next !== now) {
        els.turn.value = String(next);
        render();
      }

      if (shouldStop) {
        stopPlayback();
        return;
      }
    }

    playbackRafId = requestAnimationFrame(tick);
  };

  playbackRafId = requestAnimationFrame(tick);
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

function updateDatasetControl() {
  const datasets = getSelectValues(els.inputDataset);
  els.inputDataset.disabled =
    loadingPair || runningSolver || datasets.length <= 1;
}

function updateCaseNavButtons() {
  const cases = getSelectValues(els.inputCase);
  const idx = cases.indexOf(els.inputCase.value);
  const invalid =
    cases.length === 0 || idx < 0 || loadingPair || runningSolver;
  els.prevCaseBtn.disabled = invalid || idx === 0;
  els.nextCaseBtn.disabled = invalid || idx === cases.length - 1;
}

function syncControlDisabledState() {
  const disabled = loadingPair || runningSolver;
  els.refreshBinsBtn.disabled = disabled;
  els.refreshCasesBtn.disabled = disabled;
  els.rustBin.disabled = disabled || getSelectValues(els.rustBin).length === 0;
  els.inputCase.disabled =
    disabled || getSelectValues(els.inputCase).length === 0;
  updateDatasetControl();
  updateCaseNavButtons();
  updateRunButton();
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

async function fetchJson(url) {
  const res = await fetch(url);
  let data = null;
  try {
    data = await res.json();
  } catch {
    data = null;
  }
  if (!res.ok) {
    throw new Error(data?.error || `HTTP ${res.status}`);
  }
  return data;
}

async function loadRustBins() {
  try {
    const data = await fetchJson("/api/rust-bins");
    const bins = Array.isArray(data.bins) ? data.bins : [];
    const ok = setSelectOptions(els.rustBin, bins, "(src/bin/*.rs が無い)");
    if (!ok) {
      setStatus("src/bin/*.rs が見つからない", true);
      syncControlDisabledState();
      return false;
    }
    setStatus(`bin一覧を取得: ${bins.length}件`);
    syncControlDisabledState();
    return true;
  } catch (e) {
    setSelectOptions(els.rustBin, [], "(取得失敗)");
    setStatus(`bin一覧取得に失敗: ${String(e)}`, true);
    syncControlDisabledState();
    return false;
  }
}

async function loadDatasetOptions() {
  try {
    const results = await Promise.all(
      KNOWN_DATASETS.map(async (dataset) => {
        const data = await fetchJson(
          `/api/in-cases?dataset=${encodeURIComponent(dataset)}`,
        );
        return {
          dataset,
          exists: data.exists === true,
        };
      }),
    );
    const datasets = results
      .filter((item) => item.exists)
      .map((item) => item.dataset);
    const ok = setSelectOptions(els.inputDataset, datasets, "(dataset 無し)");
    if (!ok) {
      setStatus("利用可能な input dataset が見つからない", true);
      syncControlDisabledState();
      return false;
    }
    syncControlDisabledState();
    return true;
  } catch (e) {
    setSelectOptions(els.inputDataset, [], "(取得失敗)");
    setStatus(`dataset 一覧取得に失敗: ${String(e)}`, true);
    syncControlDisabledState();
    return false;
  }
}

async function loadInputCases() {
  const dataset = currentDataset();
  try {
    const data = await fetchJson(
      `/api/in-cases?dataset=${encodeURIComponent(dataset)}`,
    );
    const cases = Array.isArray(data.cases) ? data.cases : [];
    const ok = setSelectOptions(
      els.inputCase,
      cases,
      data.exists === true
        ? `(tools/${dataset} が空)`
        : `(tools/${dataset} が無い)`,
    );
    if (!ok) {
      const message =
        data.exists === true
          ? `tools/${dataset} にケースが無い`
          : `tools/${dataset} が見つからない`;
      setStatus(message, true);
      syncControlDisabledState();
      return false;
    }
    setStatus(`${dataset} 一覧を取得: ${cases.length}件`);
    syncControlDisabledState();
    return true;
  } catch (e) {
    setSelectOptions(els.inputCase, [], "(取得失敗)");
    setStatus(`${dataset} 一覧取得に失敗: ${String(e)}`, true);
    syncControlDisabledState();
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
    syncControlDisabledState();
    return;
  }

  const myToken = ++pairLoadToken;
  loadingPair = true;
  syncControlDisabledState();

  try {
    const data = await fetchJson(
      `/api/vis-pair?dataset=${encodeURIComponent(dataset)}&bin=${encodeURIComponent(binName)}&case=${encodeURIComponent(caseName)}`,
    );

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
    } else {
      setStatus(
        `${binName} / ${dataset} / ${caseName} の output が無い（results/out/${binName}/${dataset}/${caseName}）`,
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
  const cases = getSelectValues(els.inputCase);
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
  const datasetsOk = await loadDatasetOptions();
  if (!datasetsOk) {
    els.inputArea.value = "";
    els.outputArea.value = "";
    setTurnMax({ forceToEnd: true });
    render();
    return;
  }
  const casesOk = await loadInputCases();
  if (casesOk) {
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

els.runBinBtn.addEventListener("click", async () => {
  await runSelectedBin();
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

async function loadWasmModuleFromPublic() {
  const res = await fetch(WASM_JS_PATH, { cache: "no-store" });
  if (!res.ok) {
    throw new Error(`WASM wrapper 読込失敗: HTTP ${res.status}`);
  }

  const source = await res.text();
  const blob = new Blob([source], { type: "text/javascript" });
  const blobUrl = URL.createObjectURL(blob);
  try {
    return await import(/* @vite-ignore */ blobUrl);
  } finally {
    URL.revokeObjectURL(blobUrl);
  }
}

async function main() {
  const wasmModule = await loadWasmModuleFromPublic();
  wasmInit = wasmModule.default;
  wasmGetMaxTurn = wasmModule.get_max_turn;
  wasmVis = wasmModule.vis;
  await wasmInit(WASM_BG_PATH);

  ready = true;
  updatePlayButton();
  setTurnMax({ forceToEnd: true });
  render();

  const binsOk = await loadRustBins();
  const datasetsOk = await loadDatasetOptions();
  if (binsOk && datasetsOk) {
    const casesOk = await loadInputCases();
    if (casesOk) {
      await loadSelectedPair({ setTurnToEnd: true });
    } else {
      els.inputArea.value = "";
      els.outputArea.value = "";
      setTurnMax({ forceToEnd: true });
      render();
    }
  }
  syncControlDisabledState();
}

main().catch((e) => {
  els.error.textContent = `WASM初期化失敗: ${String(e)}`;
});
