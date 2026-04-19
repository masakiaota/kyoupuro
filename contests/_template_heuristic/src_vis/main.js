import init, {
  gen as wasmGenModule,
  get_max_turn as wasmGetMaxTurnModule,
  vis as wasmVisModule,
} from "./wasm/heuristic_contest_template_vis.js";

let wasmInit = null;
let wasmGen = null;
let wasmGetMaxTurn = null;
let wasmVis = null;

const els = {
  seed: document.getElementById("seed"),
  genBtn: document.getElementById("genBtn"),
  sampleBtn: document.getElementById("sampleBtn"),
  rustBin: document.getElementById("rustBin"),
  refreshBinsBtn: document.getElementById("refreshBinsBtn"),
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
let runningBin = false;

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

function setTurnMax() {
  const input = els.inputArea.value;
  const output = els.outputArea.value;

  let mx = 0;
  if (output.trim().length > 0) {
    try {
      mx = Math.max(1, wasmGetMaxTurn(input, output));
    } catch {
      mx = 1;
    }
  }
  els.turn.max = String(mx);
  if (Number(els.turn.value) > mx) {
    els.turn.value = String(mx);
  }
  if (mx === 0) {
    stopPlayback();
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
    els.error.textContent = "input/output を入力すると描画される";
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

function setRunStatus(text, isError = false) {
  els.runStatus.textContent = text;
  els.runStatus.style.color = isError ? "#b91c1c" : "#4b5563";
}

function setRunButtonsDisabled(disabled) {
  els.refreshBinsBtn.disabled = disabled;
  els.runBinBtn.disabled = disabled;
  els.rustBin.disabled = disabled;
}

async function loadRustBins() {
  try {
    const prev = els.rustBin.value;
    const res = await fetch("/api/rust-bins");
    if (!res.ok) {
      throw new Error(`HTTP ${res.status}`);
    }
    const data = await res.json();
    const bins = Array.isArray(data.bins) ? data.bins : [];

    els.rustBin.innerHTML = "";
    for (const name of bins) {
      const opt = document.createElement("option");
      opt.value = name;
      opt.textContent = name;
      els.rustBin.appendChild(opt);
    }

    if (bins.length === 0) {
      const opt = document.createElement("option");
      opt.value = "";
      opt.textContent = "(src/bin/*.rs が無い)";
      els.rustBin.appendChild(opt);
      els.rustBin.value = "";
      setRunStatus("src/bin/*.rs が見つからない");
      return;
    }

    if (prev && bins.includes(prev)) {
      els.rustBin.value = prev;
    } else {
      els.rustBin.value = bins[0];
    }
    setRunStatus(`bin一覧を取得: ${bins.length}件`);
  } catch (e) {
    els.rustBin.innerHTML = "";
    const opt = document.createElement("option");
    opt.value = "";
    opt.textContent = "(取得失敗)";
    els.rustBin.appendChild(opt);
    setRunStatus(
      `bin一覧取得に失敗: ${String(e)} (yarn dev で開いているか確認)`,
      true,
    );
  }
}

async function runSelectedRustBin() {
  if (runningBin) return;

  const binName = (els.rustBin.value || "").trim();
  if (!binName) {
    setRunStatus("実行する bin を選択する", true);
    return;
  }
  const inputText = els.inputArea.value;
  if (!inputText.trim()) {
    setRunStatus("先に Input を用意する", true);
    return;
  }

  try {
    runningBin = true;
    stopPlayback();
    setRunButtonsDisabled(true);
    setRunStatus(`${binName} を実行中...`);

    const res = await fetch("/api/run-rust-bin", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ binName, input: inputText }),
    });
    const data = await res.json();
    if (!res.ok) {
      throw new Error(data.error || `HTTP ${res.status}`);
    }

    els.outputArea.value = data.output ?? "";
    setTurnMax();
    render();

    const elapsed = typeof data.elapsedMs === "number" ? data.elapsedMs : null;
    const stderr =
      typeof data.stderr === "string" && data.stderr.trim().length > 0;
    setRunStatus(
      `${binName} 実行完了${elapsed !== null ? ` (${elapsed} ms)` : ""}${stderr ? " / stderrあり" : ""}`,
    );
  } catch (e) {
    setRunStatus(`bin実行失敗: ${String(e)}`, true);
  } finally {
    runningBin = false;
    setRunButtonsDisabled(false);
  }
}

async function loadSample() {
  try {
    const [iRes, oRes] = await Promise.all([
      fetch("/samples/input_01.txt"),
      fetch("/samples/output_01.txt"),
    ]);
    if (!iRes.ok || !oRes.ok) {
      throw new Error(
        "samples/input_01.txt と samples/output_01.txt を配置してから使う",
      );
    }
    els.inputArea.value = await iRes.text();
    els.outputArea.value = await oRes.text();
    setTurnMax();
    render();
  } catch (e) {
    els.error.textContent = `サンプル読込失敗: ${String(e)}`;
  }
}

els.genBtn.addEventListener("click", () => {
  const seed = Number(els.seed.value) | 0;
  try {
    els.inputArea.value = wasmGen(seed);
    setTurnMax();
    render();
  } catch (e) {
    els.error.textContent = `生成失敗: ${String(e)}`;
  }
});

els.sampleBtn.addEventListener("click", () => {
  void loadSample();
});

els.refreshBinsBtn.addEventListener("click", () => {
  void loadRustBins();
});

els.runBinBtn.addEventListener("click", () => {
  void runSelectedRustBin();
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

async function main() {
  wasmInit = init;
  wasmGen = wasmGenModule;
  wasmGetMaxTurn = wasmGetMaxTurnModule;
  wasmVis = wasmVisModule;
  await wasmInit();
  ready = true;
  updatePlayButton();
  setTurnMax();
  els.error.textContent =
    "problem_description.txt と tools/src/ が揃ったら visualizer を問題固有実装に置き換える";
  render();
  await loadRustBins();
}

main().catch((e) => {
  els.error.textContent = `WASM初期化失敗: ${String(e)}`;
});
