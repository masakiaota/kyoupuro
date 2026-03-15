let wasmInit = null;
let wasmGen = null;

const els = {
  seed: document.getElementById("seed"),
  genBtn: document.getElementById("genBtn"),
  sampleBtn: document.getElementById("sampleBtn"),
  caseFile: document.getElementById("caseFile"),
  refreshCasesBtn: document.getElementById("refreshCasesBtn"),
  rustBin: document.getElementById("rustBin"),
  refreshBinsBtn: document.getElementById("refreshBinsBtn"),
  runBinBtn: document.getElementById("runBinBtn"),
  runStatus: document.getElementById("runStatus"),
  viewMode: document.getElementById("viewMode"),
  fitBtn: document.getElementById("fitBtn"),
  resetViewBtn: document.getElementById("resetViewBtn"),
  zoomValue: document.getElementById("zoomValue"),
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
  mainCanvas: document.getElementById("mainCanvas"),
};

const LAYOUT = {
  pad: 20,
  board: 760,
  side: 360,
  graph: 220,
};
const GRAPH = {
  left: 40,
  top: 20,
  right: 16,
  bottom: 24,
};
const C0_MODE1 = [241, 245, 249];
const C1_MODE1 = [185, 28, 28];
const C0_MODE2 = [224, 242, 254];
const C1_MODE2 = [15, 118, 110];
const MODE2_PALETTE = buildPalette(C0_MODE2, C1_MODE2, 256);
const MODE3_COLORS = {
  0: [248, 250, 252],
  1: [209, 250, 229],
  2: [253, 230, 138],
  3: [239, 68, 68],
};

let ready = false;
let playing = false;
let playbackTimer = null;
let runningBin = false;
let loadingCase = false;
let zoom = 1;
let panX = 0;
let panY = 0;
let dragging = false;
let dragLastX = 0;
let dragLastY = 0;
let autoFitPending = true;
let selectionSyncRunning = false;
let selectionSyncRerun = false;
const caseInputCache = new Map();

let inputState = null;
let analysisState = null;
let inputError = "";

const ctx = els.mainCanvas.getContext("2d");

function getWorldSize() {
  return {
    width: LAYOUT.pad * 3 + LAYOUT.board + LAYOUT.side,
    height: LAYOUT.pad * 4 + LAYOUT.board + LAYOUT.graph,
  };
}

function buildPalette(c0, c1, n) {
  const out = new Uint8Array(n * 3);
  for (let i = 0; i < n; i += 1) {
    const t = n <= 1 ? 0 : i / (n - 1);
    out[i * 3 + 0] = Math.round(c0[0] + (c1[0] - c0[0]) * t);
    out[i * 3 + 1] = Math.round(c0[1] + (c1[1] - c0[1]) * t);
    out[i * 3 + 2] = Math.round(c0[2] + (c1[2] - c0[2]) * t);
  }
  return out;
}

function clamp(v, lo, hi) {
  return Math.max(lo, Math.min(hi, v));
}

function getSpeed() {
  const v = Number(els.speed.value);
  return Number.isFinite(v) && v > 0 ? v : 1000;
}

function getViewMode() {
  const mode = Number(els.viewMode.value);
  if (mode === 2 || mode === 3) {
    return mode;
  }
  return 1;
}

function getShownTurn() {
  if (!analysisState || analysisState.outputLen <= 0) return -1;
  return clamp(Number(els.turn.value) | 0, 0, analysisState.outputLen - 1);
}

function getShownLen() {
  const t = getShownTurn();
  return t >= 0 ? t + 1 : 0;
}

function setRunStatus(text, isError = false) {
  els.runStatus.textContent = text;
  els.runStatus.style.color = isError ? "#b91c1c" : "#4b5563";
}

function setButtonsDisabled(disabled) {
  els.refreshCasesBtn.disabled = disabled;
  els.caseFile.disabled = disabled;
  els.refreshBinsBtn.disabled = disabled;
  els.rustBin.disabled = disabled;
  els.runBinBtn.disabled = disabled;
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
  if (!analysisState || analysisState.outputLen <= 0) return;
  const maxTurn = analysisState.outputLen - 1;
  const now = getShownTurn();
  let next = now + delta;
  if (delta > 0 && next > maxTurn) {
    next = els.loopPlay.checked ? 0 : maxTurn;
  }
  if (delta < 0 && next < 0) {
    next = els.loopPlay.checked ? maxTurn : 0;
  }
  els.turn.value = String(next);
  render(false);
}

function startPlayback() {
  if (!analysisState || analysisState.outputLen <= 0 || playing) return;
  playing = true;
  updatePlayButton();
  playbackTimer = setInterval(() => {
    const now = getShownTurn();
    if (now >= analysisState.outputLen - 1) {
      if (els.loopPlay.checked) {
        els.turn.value = "0";
        render(false);
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

function updateZoomLabel() {
  els.zoomValue.textContent = `${Math.round(zoom * 100)}%`;
}

function resizeCanvasToHost() {
  const w = Math.max(1, els.svgHost.clientWidth);
  const h = Math.max(1, els.svgHost.clientHeight);
  const dpr = window.devicePixelRatio || 1;
  const rw = Math.round(w * dpr);
  const rh = Math.round(h * dpr);
  if (els.mainCanvas.width !== rw || els.mainCanvas.height !== rh) {
    els.mainCanvas.width = rw;
    els.mainCanvas.height = rh;
  }
}

function resetView() {
  zoom = 1;
  panX = 0;
  panY = 0;
  updateZoomLabel();
  render(false);
}

function fitView() {
  const { width, height } = getWorldSize();
  const hostW = els.svgHost.clientWidth;
  const hostH = els.svgHost.clientHeight;
  if (hostW <= 0 || hostH <= 0) return;
  zoom = clamp(Math.min(hostW / width, hostH / height), 0.1, 8.0);
  panX = (hostW - width * zoom) / 2;
  panY = (hostH - height * zoom) / 2;
  updateZoomLabel();
  render(false);
}

function parseInputText(text) {
  const raw = text.trim();
  if (!raw) {
    throw new Error("input is empty");
  }
  const tokens = raw.split(/\s+/);
  if (tokens.length < 1) {
    throw new Error("failed to read N");
  }
  const N = Number(tokens[0]);
  if (!Number.isInteger(N) || N <= 0) {
    throw new Error("invalid N");
  }
  const n2 = N * N;
  if (tokens.length < 1 + n2) {
    throw new Error(`input is short: need ${n2} values, got ${tokens.length - 1}`);
  }
  const A = new Int32Array(n2);
  for (let idx = 0; idx < n2; idx += 1) {
    const v = Number(tokens[idx + 1]);
    if (!Number.isInteger(v)) {
      throw new Error(`A parse error at idx=${idx}`);
    }
    A[idx] = v;
  }
  return { N, n2, A };
}

function parseOutputLenient(input, text) {
  const raw = text.trim();
  if (!raw) {
    return {
      i: new Int16Array(0),
      j: new Int16Array(0),
      parseErrors: [],
      parseErrorTurn: -1,
    };
  }
  const tokens = raw.split(/\s+/);
  const maxLen = input.n2;
  const ii = new Int16Array(maxLen);
  const jj = new Int16Array(maxLen);
  const parseErrors = [];
  let parseErrorTurn = -1;
  let ptr = 0;

  for (let p = 0; p < tokens.length; ) {
    const ti = tokens[p++];
    if (p >= tokens.length) {
      parseErrors.push(`turn ${ptr}: j is missing`);
      parseErrorTurn = ptr;
      break;
    }
    const tj = tokens[p++];
    if (ptr >= maxLen) {
      parseErrors.push(`turn ${ptr}: too many outputs (limit is ${maxLen})`);
      parseErrorTurn = ptr;
      break;
    }
    const i = Number(ti);
    const j = Number(tj);
    if (!Number.isInteger(i)) {
      parseErrors.push(`turn ${ptr}: i parse error ('${ti}')`);
      parseErrorTurn = ptr;
      break;
    }
    if (!Number.isInteger(j)) {
      parseErrors.push(`turn ${ptr}: j parse error ('${tj}')`);
      parseErrorTurn = ptr;
      break;
    }
    if (i < 0 || i >= input.N || j < 0 || j >= input.N) {
      parseErrors.push(
        `turn ${ptr}: out of range (${i}, ${j}) (expected 0..${input.N - 1})`,
      );
      parseErrorTurn = ptr;
      break;
    }
    ii[ptr] = i;
    jj[ptr] = j;
    ptr += 1;
  }

  return {
    i: ii.subarray(0, ptr),
    j: jj.subarray(0, ptr),
    parseErrors,
    parseErrorTurn,
  };
}

function buildMode1Rgba(input) {
  const data = new Uint8ClampedArray(input.n2 * 4);
  const denom = Math.max(1, input.n2 - 1);
  for (let idx = 0; idx < input.n2; idx += 1) {
    const t = (input.A[idx] - 1) / denom;
    const r = Math.round(C0_MODE1[0] + (C1_MODE1[0] - C0_MODE1[0]) * t);
    const g = Math.round(C0_MODE1[1] + (C1_MODE1[1] - C0_MODE1[1]) * t);
    const b = Math.round(C0_MODE1[2] + (C1_MODE1[2] - C0_MODE1[2]) * t);
    const o = idx * 4;
    data[o + 0] = r;
    data[o + 1] = g;
    data[o + 2] = b;
    data[o + 3] = 255;
  }
  return data;
}

function buildGraphPaths(analysis) {
  const { width: worldW } = getWorldSize();
  const graphW = worldW - LAYOUT.pad * 2;
  const innerW = graphW - GRAPH.left - GRAPH.right;
  const innerH = LAYOUT.graph - GRAPH.top - GRAPH.bottom;
  const gainPath = new Path2D();
  const prefixPath = new Path2D();

  if (analysis.outputLen <= 0) {
    return { gainPath, prefixPath, innerW, innerH };
  }

  const xDen = Math.max(1, analysis.outputLen - 1);
  for (let t = 0; t < analysis.outputLen; t += 1) {
    const x = (t / xDen) * innerW;
    const yg = innerH - (analysis.gains[t] / analysis.maxGain) * innerH;
    const yp = innerH - (analysis.prefix[t] / analysis.maxPrefix) * innerH;
    if (t === 0) {
      gainPath.moveTo(x, yg);
      prefixPath.moveTo(x, yp);
    } else {
      gainPath.lineTo(x, yg);
      prefixPath.lineTo(x, yp);
    }
  }

  return { gainPath, prefixPath, innerW, innerH };
}

function buildAnalysis(input, parsedOutput) {
  const N = input.N;
  const n2 = input.n2;
  const L = parsedOutput.i.length;
  const gains = new Float64Array(L);
  const prefix = new Float64Array(L);
  const firstVisit = new Int32Array(n2);
  const usedTurn = new Int32Array(n2);
  firstVisit.fill(-1);
  usedTurn.fill(-1);

  let running = 0;
  let validPrefixLen = 0;
  let firstErrorTurn = -1;
  let firstErrorMessage = "";

  for (let t = 0; t < L; t += 1) {
    if (firstErrorTurn >= 0) {
      prefix[t] = running;
      continue;
    }
    const i = parsedOutput.i[t];
    const j = parsedOutput.j[t];
    const idx = i * N + j;
    if (usedTurn[idx] !== -1) {
      firstErrorTurn = t;
      firstErrorMessage = `Duplicate move at turn ${t}: (${i}, ${j}) first=${usedTurn[idx]}`;
      prefix[t] = running;
      continue;
    }
    if (t > 0) {
      const pi = parsedOutput.i[t - 1];
      const pj = parsedOutput.j[t - 1];
      if (Math.max(Math.abs(i - pi), Math.abs(j - pj)) !== 1) {
        firstErrorTurn = t;
        firstErrorMessage = `Invalid move at turn ${t}: (${pi}, ${pj}) -> (${i}, ${j})`;
        prefix[t] = running;
        continue;
      }
    }

    usedTurn[idx] = t;
    if (firstVisit[idx] < 0) {
      firstVisit[idx] = t;
    }
    const gain = t * input.A[idx];
    gains[t] = gain;
    running += gain;
    prefix[t] = running;
    validPrefixLen = t + 1;
  }

  const errParts = [];
  if (parsedOutput.parseErrors.length > 0) {
    errParts.push(parsedOutput.parseErrors.join("\n"));
  }
  if (firstErrorMessage) {
    errParts.push(firstErrorMessage);
  }
  if (L < n2) {
    errParts.push(`Not finished: ${L} / ${n2}`);
  }
  const officialError = errParts.join("\n");
  const officialScore =
    officialError.length > 0 ? 0 : Math.floor((running + n2 / 2) / n2);

  let maxGain = 1;
  for (let t = 0; t < L; t += 1) {
    if (gains[t] > maxGain) maxGain = gains[t];
  }
  const maxPrefix = L > 0 ? Math.max(1, prefix[L - 1]) : 1;

  const rasterCanvas = document.createElement("canvas");
  rasterCanvas.width = N;
  rasterCanvas.height = N;
  const rasterCtx = rasterCanvas.getContext("2d");
  const rasterImage = rasterCtx.createImageData(N, N);

  const analysis = {
    N,
    n2,
    A: input.A,
    outputI: parsedOutput.i,
    outputJ: parsedOutput.j,
    outputLen: L,
    gains,
    prefix,
    firstVisit,
    validPrefixLen,
    firstErrorTurn,
    firstErrorMessage,
    parseErrorTurn: parsedOutput.parseErrorTurn,
    officialScore,
    officialError,
    maxGain,
    maxPrefix,
    mode1Rgba: buildMode1Rgba(input),
    visitCounts: new Uint16Array(n2),
    visitCountsComputed: 0,
    rasterCanvas,
    rasterCtx,
    rasterImage,
    cachedMode: -1,
    cachedShownLen: -1,
  };
  analysis.graph = buildGraphPaths(analysis);
  return analysis;
}

function recomputeStateFromText() {
  const inputText = els.inputArea.value;
  const outputText = els.outputArea.value;
  inputError = "";
  inputState = null;
  analysisState = null;

  if (!inputText.trim()) {
    els.score.textContent = "-";
    els.error.textContent = "input を入れると描画する";
    setTurnRange(0);
    return;
  }

  try {
    inputState = parseInputText(inputText);
  } catch (e) {
    inputError = String(e);
    els.score.textContent = "0";
    els.error.textContent = inputError;
    setTurnRange(0);
    return;
  }

  const parsed = parseOutputLenient(inputState, outputText);
  analysisState = buildAnalysis(inputState, parsed);
  const maxTurn = analysisState.outputLen > 0 ? analysisState.outputLen - 1 : 0;
  setTurnRange(maxTurn);

  if (!outputText.trim()) {
    els.score.textContent = "-";
    els.error.textContent = "output が空";
  } else {
    els.score.textContent = String(analysisState.officialScore);
    els.error.textContent = analysisState.officialError;
  }
}

function setTurnRange(maxTurn) {
  els.turn.max = String(maxTurn);
  let now = Number(els.turn.value);
  if (!Number.isFinite(now) || now < 0) now = 0;
  if (now > maxTurn) now = maxTurn;
  els.turn.value = String(now);
  els.turnValue.textContent = String(now);
  els.maxTurnValue.textContent = String(maxTurn);
}

function ensureVisitCounts(analysis, shownLen) {
  if (shownLen === analysis.visitCountsComputed) return;
  if (shownLen > analysis.visitCountsComputed) {
    for (let t = analysis.visitCountsComputed; t < shownLen; t += 1) {
      const idx = analysis.outputI[t] * analysis.N + analysis.outputJ[t];
      analysis.visitCounts[idx] += 1;
    }
    analysis.visitCountsComputed = shownLen;
    return;
  }
  analysis.visitCounts.fill(0);
  for (let t = 0; t < shownLen; t += 1) {
    const idx = analysis.outputI[t] * analysis.N + analysis.outputJ[t];
    analysis.visitCounts[idx] += 1;
  }
  analysis.visitCountsComputed = shownLen;
}

function writePixel(data, idx, r, g, b) {
  const o = idx * 4;
  data[o + 0] = r;
  data[o + 1] = g;
  data[o + 2] = b;
  data[o + 3] = 255;
}

function renderBoardRaster(analysis, mode, shownLen) {
  if (analysis.cachedMode === mode && analysis.cachedShownLen === shownLen) return;
  const data = analysis.rasterImage.data;

  if (mode === 1) {
    data.set(analysis.mode1Rgba);
  } else if (mode === 2) {
    const den = Math.max(1, shownLen - 1);
    for (let idx = 0; idx < analysis.n2; idx += 1) {
      const t = analysis.firstVisit[idx];
      if (t >= 0 && t < shownLen) {
        const bucket = Math.floor((t * 255) / den) * 3;
        writePixel(
          data,
          idx,
          MODE2_PALETTE[bucket + 0],
          MODE2_PALETTE[bucket + 1],
          MODE2_PALETTE[bucket + 2],
        );
      } else {
        writePixel(data, idx, 248, 250, 252);
      }
    }
  } else {
    ensureVisitCounts(analysis, shownLen);
    for (let idx = 0; idx < analysis.n2; idx += 1) {
      const c = analysis.visitCounts[idx];
      const key = c <= 2 ? c : 3;
      const rgb = MODE3_COLORS[key];
      writePixel(data, idx, rgb[0], rgb[1], rgb[2]);
    }
  }
  analysis.rasterCtx.putImageData(analysis.rasterImage, 0, 0);
  analysis.cachedMode = mode;
  analysis.cachedShownLen = shownLen;
}

function drawWorld(mode, shownTurn, shownLen) {
  const analysis = analysisState;
  const { width: worldW, height: worldH } = getWorldSize();
  const boardX = LAYOUT.pad;
  const boardY = LAYOUT.pad;
  const boardSize = LAYOUT.board;
  const cell = boardSize / analysis.N;

  ctx.fillStyle = "#f9fafb";
  ctx.fillRect(0, 0, worldW, worldH);

  renderBoardRaster(analysis, mode, shownLen);
  ctx.imageSmoothingEnabled = false;
  ctx.drawImage(analysis.rasterCanvas, boardX, boardY, boardSize, boardSize);
  ctx.imageSmoothingEnabled = true;
  ctx.strokeStyle = "#9ca3af";
  ctx.lineWidth = 1;
  ctx.strokeRect(boardX, boardY, boardSize, boardSize);

  if (shownLen >= 2) {
    ctx.beginPath();
    for (let t = 0; t < shownLen; t += 1) {
      const x = boardX + (analysis.outputJ[t] + 0.5) * cell;
      const y = boardY + (analysis.outputI[t] + 0.5) * cell;
      if (t === 0) ctx.moveTo(x, y);
      else ctx.lineTo(x, y);
    }
    ctx.strokeStyle = "#0f766e";
    ctx.lineWidth = mode === 3 ? 1.8 : 1.2;
    ctx.globalAlpha = 0.9;
    ctx.stroke();
    ctx.globalAlpha = 1;
  }

  if (shownLen > 0) {
    const sx = boardX + (analysis.outputJ[0] + 0.5) * cell;
    const sy = boardY + (analysis.outputI[0] + 0.5) * cell;
    ctx.beginPath();
    ctx.arc(sx, sy, 2.4, 0, Math.PI * 2);
    ctx.fillStyle = "#2563eb";
    ctx.fill();

    const cx = boardX + (analysis.outputJ[shownTurn] + 0.5) * cell;
    const cy = boardY + (analysis.outputI[shownTurn] + 0.5) * cell;
    ctx.beginPath();
    ctx.arc(cx, cy, 3.0, 0, Math.PI * 2);
    ctx.fillStyle = "#f59e0b";
    ctx.fill();
    ctx.strokeStyle = "#111827";
    ctx.lineWidth = 1;
    ctx.stroke();
  }

  if (analysis.firstErrorTurn >= 0 && analysis.firstErrorTurn < shownLen) {
    const t = analysis.firstErrorTurn;
    const i = analysis.outputI[t];
    const j = analysis.outputJ[t];
    ctx.strokeStyle = "#dc2626";
    ctx.lineWidth = 1.5;
    ctx.strokeRect(boardX + j * cell - 0.5, boardY + i * cell - 0.5, cell + 1, cell + 1);
    if (t > 0) {
      const x1 = boardX + (analysis.outputJ[t - 1] + 0.5) * cell;
      const y1 = boardY + (analysis.outputI[t - 1] + 0.5) * cell;
      const x2 = boardX + (analysis.outputJ[t] + 0.5) * cell;
      const y2 = boardY + (analysis.outputI[t] + 0.5) * cell;
      ctx.beginPath();
      ctx.moveTo(x1, y1);
      ctx.lineTo(x2, y2);
      ctx.stroke();
    }
  }

  const panelX = boardX + boardSize + LAYOUT.pad;
  const panelY = boardY;
  ctx.fillStyle = "#ffffff";
  ctx.strokeStyle = "#d1d5db";
  ctx.lineWidth = 1;
  ctx.fillRect(panelX, panelY, LAYOUT.side, boardSize);
  ctx.strokeRect(panelX, panelY, LAYOUT.side, boardSize);

  ctx.fillStyle = "#111827";
  ctx.font = "14px SFMono-Regular, Menlo, monospace";
  let ty = panelY + 24;
  const tstep = 21;
  const lines = [];
  lines.push("King's Tour Canvas Visualizer");
  lines.push(`mode: ${mode}`);
  lines.push(`shown: ${shownLen} / ${analysis.outputLen}`);
  lines.push(`k: ${shownTurn}`);
  const gain = shownTurn >= 0 ? analysis.gains[shownTurn] : 0;
  const pref = shownTurn >= 0 ? analysis.prefix[shownTurn] : 0;
  const roundNow = shownTurn >= 0 ? Math.floor((pref + analysis.n2 / 2) / analysis.n2) : 0;
  lines.push(`gain(k)=k*A: ${Math.round(gain)}`);
  lines.push(`prefix V: ${Math.round(pref)}`);
  lines.push(`round(V/N^2): ${roundNow}`);
  lines.push(`final score: ${analysis.officialScore}`);
  lines.push(`valid prefix: ${analysis.validPrefixLen}`);
  if (analysis.parseErrorTurn >= 0) lines.push(`parse error turn: ${analysis.parseErrorTurn}`);
  if (analysis.firstErrorTurn >= 0) lines.push(`constraint error turn: ${analysis.firstErrorTurn}`);
  for (const line of lines) {
    ctx.fillStyle = "#111827";
    if (line.includes("error")) ctx.fillStyle = "#b91c1c";
    ctx.fillText(line, panelX + 12, ty);
    ty += tstep;
  }

  const graphX = LAYOUT.pad;
  const graphY = LAYOUT.pad * 2 + boardSize;
  const graphW = worldW - LAYOUT.pad * 2;
  const graphH = LAYOUT.graph;
  ctx.fillStyle = "#ffffff";
  ctx.strokeStyle = "#d1d5db";
  ctx.fillRect(graphX, graphY, graphW, graphH);
  ctx.strokeRect(graphX, graphY, graphW, graphH);

  const ix = graphX + GRAPH.left;
  const iy = graphY + GRAPH.top;
  const iw = graphW - GRAPH.left - GRAPH.right;
  const ih = graphH - GRAPH.top - GRAPH.bottom;
  ctx.strokeStyle = "#9ca3af";
  ctx.lineWidth = 1;
  ctx.beginPath();
  ctx.moveTo(ix, iy + ih);
  ctx.lineTo(ix + iw, iy + ih);
  ctx.moveTo(ix, iy);
  ctx.lineTo(ix, iy + ih);
  ctx.stroke();

  if (analysis.outputLen > 0) {
    ctx.save();
    ctx.translate(ix, iy);
    ctx.lineWidth = 1.2;
    ctx.strokeStyle = "#2563eb";
    ctx.stroke(analysis.graph.gainPath);
    ctx.strokeStyle = "#f59e0b";
    ctx.stroke(analysis.graph.prefixPath);
    ctx.restore();

    const den = Math.max(1, analysis.outputLen - 1);
    const xCur = ix + (shownTurn / den) * iw;
    ctx.strokeStyle = "#dc2626";
    ctx.setLineDash([4, 4]);
    ctx.beginPath();
    ctx.moveTo(xCur, iy);
    ctx.lineTo(xCur, iy + ih);
    ctx.stroke();
    ctx.setLineDash([]);

    const yg = iy + ih - (analysis.gains[shownTurn] / analysis.maxGain) * ih;
    const yp = iy + ih - (analysis.prefix[shownTurn] / analysis.maxPrefix) * ih;
    ctx.fillStyle = "#2563eb";
    ctx.beginPath();
    ctx.arc(xCur, yg, 2.4, 0, Math.PI * 2);
    ctx.fill();
    ctx.fillStyle = "#f59e0b";
    ctx.beginPath();
    ctx.arc(xCur, yp, 2.4, 0, Math.PI * 2);
    ctx.fill();
  } else {
    ctx.fillStyle = "#6b7280";
    ctx.font = "14px SFMono-Regular, Menlo, monospace";
    ctx.fillText("output を入力すると k ごとのグラフを描画する", ix, iy + ih / 2);
  }

  ctx.fillStyle = "#111827";
  ctx.font = "13px SFMono-Regular, Menlo, monospace";
  ctx.fillText("kごとの獲得スコア(青) / 累積V(橙)", ix, graphY + 14);
}

function render(autoFit = false) {
  resizeCanvasToHost();
  const dpr = window.devicePixelRatio || 1;
  const w = els.mainCanvas.width / dpr;
  const h = els.mainCanvas.height / dpr;

  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  ctx.clearRect(0, 0, w, h);
  ctx.fillStyle = "#f4f6f8";
  ctx.fillRect(0, 0, w, h);

  els.turnValue.textContent = els.turn.value;

  if (inputError) {
    els.score.textContent = "0";
    els.error.textContent = inputError;
    updateZoomLabel();
    return;
  }
  if (!analysisState || analysisState.outputLen <= 0) {
    if (!inputState) {
      updateZoomLabel();
      return;
    }
  }

  if (!analysisState) {
    updateZoomLabel();
    return;
  }

  if (autoFit || autoFitPending) {
    const world = getWorldSize();
    zoom = clamp(Math.min(w / world.width, h / world.height), 0.1, 8.0);
    panX = (w - world.width * zoom) / 2;
    panY = (h - world.height * zoom) / 2;
    autoFitPending = false;
  }
  updateZoomLabel();

  const mode = getViewMode();
  const shownTurn = getShownTurn();
  const shownLen = getShownLen();

  ctx.save();
  ctx.translate(panX, panY);
  ctx.scale(zoom, zoom);
  drawWorld(mode, shownTurn < 0 ? 0 : shownTurn, shownLen);
  ctx.restore();
}

function refreshFromText(autoFit = true) {
  recomputeStateFromText();
  autoFitPending = autoFit;
  render(autoFit);
}

async function loadRustBins() {
  try {
    const prev = els.rustBin.value;
    const res = await fetch("/api/rust-bins");
    if (!res.ok) throw new Error(`HTTP ${res.status}`);
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
      setRunStatus("src/bin/*.rs が見つからない", true);
      return;
    }
    if (prev && bins.includes(prev)) els.rustBin.value = prev;
    else els.rustBin.value = bins[0];
    setRunStatus(`bin一覧を取得: ${bins.length}件`);
  } catch (e) {
    els.rustBin.innerHTML = "";
    const opt = document.createElement("option");
    opt.value = "";
    opt.textContent = "(取得失敗)";
    els.rustBin.appendChild(opt);
    setRunStatus(`bin一覧取得に失敗: ${String(e)}`, true);
  }
}

async function loadCaseFiles() {
  try {
    const prev = els.caseFile.value;
    const res = await fetch("/api/tool-cases");
    if (!res.ok) throw new Error(`HTTP ${res.status}`);
    const data = await res.json();
    const cases = Array.isArray(data.cases) ? data.cases : [];
    els.caseFile.innerHTML = "";
    for (const name of cases) {
      const opt = document.createElement("option");
      opt.value = name;
      opt.textContent = name;
      els.caseFile.appendChild(opt);
    }
    if (cases.length === 0) {
      const opt = document.createElement("option");
      opt.value = "";
      opt.textContent = "(tools/in/*.txt が無い)";
      els.caseFile.appendChild(opt);
      els.caseFile.value = "";
      setRunStatus("tools/in/*.txt が見つからない", true);
      return;
    }
    if (prev && cases.includes(prev)) els.caseFile.value = prev;
    else els.caseFile.value = cases[0];
    setRunStatus(`case一覧を取得: ${cases.length}件`);
  } catch (e) {
    els.caseFile.innerHTML = "";
    const opt = document.createElement("option");
    opt.value = "";
    opt.textContent = "(取得失敗)";
    els.caseFile.appendChild(opt);
    setRunStatus(`case一覧取得に失敗: ${String(e)}`, true);
  }
}

async function loadSelectedCaseInput() {
  if (loadingCase) return false;
  const caseName = (els.caseFile.value || "").trim();
  if (!caseName) {
    setRunStatus("ケースを選択する", true);
    return false;
  }
  try {
    loadingCase = true;
    if (!caseInputCache.has(caseName)) {
      const res = await fetch(`/api/tool-case-input?name=${encodeURIComponent(caseName)}`);
      const data = await res.json();
      if (!res.ok) throw new Error(data.error || `HTTP ${res.status}`);
      caseInputCache.set(caseName, data.input ?? "");
    }
    els.inputArea.value = caseInputCache.get(caseName) ?? "";
    return true;
  } catch (e) {
    setRunStatus(`case読込失敗: ${String(e)}`, true);
    return false;
  } finally {
    loadingCase = false;
  }
}

async function runSelectedRustBin(options = {}) {
  const { setStatusOnStart = true } = options;
  if (runningBin) return false;
  const binName = (els.rustBin.value || "").trim();
  if (!binName) {
    setRunStatus("実行する bin を選択する", true);
    return false;
  }
  const inputText = els.inputArea.value;
  if (!inputText.trim()) {
    setRunStatus("先に Input を用意する", true);
    return false;
  }

  try {
    runningBin = true;
    stopPlayback();
    setButtonsDisabled(true);
    if (setStatusOnStart) setRunStatus(`${binName} を実行中...`);
    const res = await fetch("/api/run-rust-bin", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ binName, input: inputText }),
    });
    const data = await res.json();
    if (!res.ok) throw new Error(data.error || `HTTP ${res.status}`);
    els.outputArea.value = data.output ?? "";
    refreshFromText(true);
    const elapsed = typeof data.elapsedMs === "number" ? data.elapsedMs : null;
    const stderr = typeof data.stderr === "string" && data.stderr.trim().length > 0;
    setRunStatus(
      `${binName} 実行完了${elapsed !== null ? ` (${elapsed} ms)` : ""}${stderr ? " / stderrあり" : ""}`,
    );
    return true;
  } catch (e) {
    setRunStatus(`bin実行失敗: ${String(e)}`, true);
    return false;
  } finally {
    runningBin = false;
    setButtonsDisabled(false);
  }
}

async function syncSelectionOnce() {
  const hasCase = (els.caseFile.value || "").trim().length > 0;
  if (!hasCase) return;
  const ok = await loadSelectedCaseInput();
  if (!ok) return;
  const hasBin = (els.rustBin.value || "").trim().length > 0;
  if (!hasBin) {
    els.outputArea.value = "";
    refreshFromText(true);
    return;
  }
  els.outputArea.value = "";
  refreshFromText(false);
  await runSelectedRustBin({ setStatusOnStart: false });
}

function requestSelectionSync() {
  if (selectionSyncRunning) {
    selectionSyncRerun = true;
    return;
  }
  void (async () => {
    selectionSyncRunning = true;
    do {
      selectionSyncRerun = false;
      await syncSelectionOnce();
    } while (selectionSyncRerun);
    selectionSyncRunning = false;
  })();
}

async function loadSample() {
  try {
    const [iRes, oRes] = await Promise.all([
      fetch("/samples/input_01.txt"),
      fetch("/samples/output_01.txt"),
    ]);
    if (!iRes.ok || !oRes.ok) {
      throw new Error("samples/input_01.txt と samples/output_01.txt を配置してから使う");
    }
    els.inputArea.value = await iRes.text();
    els.outputArea.value = await oRes.text();
    stopPlayback();
    refreshFromText(true);
  } catch (e) {
    els.error.textContent = `サンプル読込失敗: ${String(e)}`;
  }
}

els.genBtn.addEventListener("click", () => {
  const seed = Number(els.seed.value) | 0;
  try {
    els.inputArea.value = wasmGen(seed, "A");
    els.outputArea.value = "";
    stopPlayback();
    refreshFromText(true);
  } catch (e) {
    els.error.textContent = `生成失敗: ${String(e)}`;
  }
});

els.sampleBtn.addEventListener("click", () => {
  void loadSample();
});

els.refreshCasesBtn.addEventListener("click", () => {
  void (async () => {
    await loadCaseFiles();
    requestSelectionSync();
  })();
});

els.caseFile.addEventListener("change", () => {
  requestSelectionSync();
});

els.refreshBinsBtn.addEventListener("click", () => {
  void (async () => {
    await loadRustBins();
    requestSelectionSync();
  })();
});

els.rustBin.addEventListener("change", () => {
  requestSelectionSync();
});

els.runBinBtn.addEventListener("click", () => {
  void runSelectedRustBin();
});

els.viewMode.addEventListener("change", () => {
  render(false);
});

els.fitBtn.addEventListener("click", () => {
  fitView();
});

els.resetViewBtn.addEventListener("click", () => {
  resetView();
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
  if (playing) stopPlayback();
  else startPlayback();
});

els.speed.addEventListener("change", () => {
  restartPlaybackIfNeeded();
});

els.inputArea.addEventListener("input", () => {
  stopPlayback();
  refreshFromText(true);
});

els.outputArea.addEventListener("input", () => {
  stopPlayback();
  refreshFromText(true);
});

els.turn.addEventListener("input", () => {
  render(false);
});

els.loopPlay.addEventListener("change", () => {
  restartPlaybackIfNeeded();
});

els.svgHost.addEventListener(
  "wheel",
  (e) => {
    if (!analysisState) return;
    e.preventDefault();
    const rect = els.svgHost.getBoundingClientRect();
    const px = e.clientX - rect.left;
    const py = e.clientY - rect.top;
    const factor = e.deltaY < 0 ? 1.1 : 0.9;
    const newZoom = clamp(zoom * factor, 0.1, 8.0);
    if (newZoom === zoom) return;
    panX = px - ((px - panX) * newZoom) / zoom;
    panY = py - ((py - panY) * newZoom) / zoom;
    zoom = newZoom;
    autoFitPending = false;
    updateZoomLabel();
    render(false);
  },
  { passive: false },
);

els.svgHost.addEventListener("pointerdown", (e) => {
  if (e.button !== 0 || !analysisState) return;
  dragging = true;
  dragLastX = e.clientX;
  dragLastY = e.clientY;
  autoFitPending = false;
  els.svgHost.setPointerCapture(e.pointerId);
  els.mainCanvas.style.cursor = "grabbing";
});

els.svgHost.addEventListener("pointermove", (e) => {
  if (!dragging) return;
  panX += e.clientX - dragLastX;
  panY += e.clientY - dragLastY;
  dragLastX = e.clientX;
  dragLastY = e.clientY;
  render(false);
});

els.svgHost.addEventListener("pointerup", () => {
  if (!dragging) return;
  dragging = false;
  els.mainCanvas.style.cursor = "grab";
});

els.svgHost.addEventListener("pointercancel", () => {
  if (!dragging) return;
  dragging = false;
  els.mainCanvas.style.cursor = "grab";
});

document.addEventListener("keydown", (e) => {
  const activeTag = document.activeElement?.tagName ?? "";
  const inEditor = activeTag === "TEXTAREA" || activeTag === "INPUT" || activeTag === "SELECT";
  if (inEditor) return;

  if (e.code === "Space") {
    e.preventDefault();
    if (playing) stopPlayback();
    else startPlayback();
  } else if (e.code === "ArrowRight") {
    e.preventDefault();
    stopPlayback();
    stepTurn(e.shiftKey ? 10 : 1);
  } else if (e.code === "ArrowLeft") {
    e.preventDefault();
    stopPlayback();
    stepTurn(e.shiftKey ? -10 : -1);
  } else if (e.key === "0") {
    e.preventDefault();
    resetView();
  } else if (e.key.toLowerCase() === "f") {
    e.preventDefault();
    fitView();
  }
});

window.addEventListener("resize", () => {
  render(false);
});

async function main() {
  const wasmModuleUrl = new URL(
    "/wasm/heuristic_contest_template_vis.js",
    window.location.origin,
  ).toString();
  const wasmModule = await import(/* @vite-ignore */ wasmModuleUrl);
  wasmInit = wasmModule.default;
  wasmGen = wasmModule.gen;
  await wasmInit();
  ready = true;

  updatePlayButton();
  updateZoomLabel();
  setTurnRange(0);
  els.error.textContent = "Canvas高速描画モード";
  render(true);

  await Promise.all([loadCaseFiles(), loadRustBins()]);
  requestSelectionSync();
}

main().catch((e) => {
  els.error.textContent = `初期化失敗: ${String(e)}`;
});
