import { computeRegionLabels } from "./region_labels.js";
import { buildMoveArrows, decodeMoveSourceGrid } from "./move_overlay.js";

const CUSTOM_CASE_VALUE = "";
const MAX_RENDER_FPS = 60;
const HOLD_DELAY_MS = 500;
const HOLD_INTERVAL_MS = 55;
const SOURCE_EDIT_SETTLE_MS = 160;
const STORAGE_SETTLE_MS = 120;
const BIN_POLL_INTERVAL_MS = 10_000;
const STORAGE_PREFIX = "ahc-visualizer";
const STORAGE_VERSION = "v1";

const EMPTY_CELL = 0xffff;
const BOARD_SIZE = 600;
const PANEL_RIGHT = 920;
const CANVAS_MARGIN = 5;
const CANVAS_WIDTH = PANEL_RIGHT + CANVAS_MARGIN * 2;
const CANVAS_HEIGHT = BOARD_SIZE + CANVAS_MARGIN * 2;
const GROUP_S = 0.36;
const GROUP_L = 0.66;
const ARRIVED_COLOR = "#ff0033";
const MOVED_COLOR = "#2563eb";
const FOCUS_WIDTH = 3.2;
const MOVED_WIDTH = 2.0;
const HALO_EXTRA = 2.6;
const MOVE_SOURCE_DASH = [6, 4];
const MOVE_ARROW_COLOR = "rgba(37, 99, 235, 0.70)";
const MOVE_ARROW_WIDTH = 1.5;
const MOVE_ARROW_HEAD_SIZE = 6;
const REGION_LABEL_FONT_SIZE = 11;
const REGION_LABEL_MIN_FONT_SIZE = 8;
const REGION_LABEL_HORIZONTAL_PADDING = 4;
const REGION_LABEL_HALO_WIDTH = 3;
const REGION_LABEL_FONT_FAMILY =
  'ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", monospace';

const visualizerWorker = new Worker(new URL("./visualizer_worker.js", import.meta.url), {
  type: "module",
});

let resolveVisualizerWorkerReady;
let rejectVisualizerWorkerReady;
const visualizerWorkerReady = new Promise((resolve, reject) => {
  resolveVisualizerWorkerReady = resolve;
  rejectVisualizerWorkerReady = reject;
});

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
  elapsedTime: document.getElementById("elapsedTime"),
  error: document.getElementById("error"),
  inputArea: document.getElementById("inputArea"),
  outputArea: document.getElementById("outputArea"),
  canvas: document.getElementById("visCanvas"),
  canvasTooltip: document.getElementById("canvasTooltip"),
  copyInputBtn: document.getElementById("copyInputBtn"),
  copyOutputBtn: document.getElementById("copyOutputBtn"),
};

const state = {
  ready: false,
  running: false,
  playing: false,
  sourcePreparing: false,
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
  suppressInputDirty: false,
  suppressOutputDirty: false,
  binPollInFlight: false,
  currentElapsedMs: null,
  sourceInput: "",
  sourceOutput: "",
  visualizationRevision: 0,
  prepareRequestId: 0,
  activePrepareRequest: null,
  preparePreference: null,
  workerPreparedRevision: -1,
  frameRequestId: 0,
  frameRequestInFlight: null,
  queuedFrameTurn: null,
  pendingCanvasFrame: null,
  canvasRafId: null,
  N: 0,
  grass: null,
  grassCells: 0,
  maxTimeLeft: 1,
  hasOutput: false,
  score: 0,
  currentFrame: null,
  backgroundCanvas: null,
  backgroundRevision: -1,
  boardCanvas: null,
  boardRevision: -1,
  boardTurn: -1,
  previousGrid: null,
  regionLabelTurn: -1,
  regionLabelRevision: -1,
  regionLabels: [],
  moveOverlayTurn: -1,
  moveOverlayRevision: -1,
  moveOverlay: null,
  loadSeq: 0,
  caseLoadController: null,
  sourceEditTimer: null,
  storageTimer: null,
  inputCache: new Map(),
};

const groupColorCache = new Map();

visualizerWorker.addEventListener("message", handleVisualizerWorkerMessage);
visualizerWorker.addEventListener("error", (event) => {
  const message = event.message || "visualizer worker failed";
  rejectVisualizerWorkerReady(new Error(message));
  state.ready = false;
  state.sourcePreparing = false;
  setRunStatus(`可視化 Worker エラー: ${message}`, true);
  els.error.textContent = message;
  updatePlayButton();
  updateRunButton();
});

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

function caseViewState() {
  return {
    bin: state.currentBin || "",
    caseName: typeof state.currentCase === "string" ? state.currentCase : CUSTOM_CASE_VALUE,
    turn: Number(els.turn.value) || 0,
    speed: els.speed.value,
  };
}

function persistCaseViewState() {
  if (!state.caseStorageKey) {
    return;
  }
  if (state.storageTimer !== null) {
    clearTimeout(state.storageTimer);
    state.storageTimer = null;
  }
  try {
    window.sessionStorage.setItem(state.caseStorageKey, JSON.stringify(caseViewState()));
  } catch {
    // Storage errors must not break the visualizer.
  }
}

function saveCaseViewState() {
  if (!state.caseStorageKey) {
    return;
  }
  if (state.storageTimer !== null) {
    clearTimeout(state.storageTimer);
  }
  state.storageTimer = window.setTimeout(persistCaseViewState, STORAGE_SETTLE_MS);
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
  const savedTurn = Number(saved.turn);
  if (Number.isFinite(savedTurn)) {
    state.pendingTurn = Math.max(0, Math.round(savedTurn));
  }
}

function updatePlayButton() {
  els.playBtn.textContent = state.playing ? "■ 停止" : "▶ 再生";
  els.playBtn.classList.toggle("is-active", state.playing);
  els.playBtn.disabled = !state.ready || state.sourcePreparing || getMaxTurn() <= 0;
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
  els.turnValue.textContent = String(Number(els.turn.value) || 0);
  els.maxTurnValue.textContent = String(getMaxTurn());
}

function setTextareaValue(textarea, value, kind) {
  if (textarea.value === value) {
    return;
  }
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

function clearCanvas() {
  state.pendingCanvasFrame = null;
  state.currentFrame = null;
  state.previousGrid = null;
  state.boardRevision = -1;
  state.boardTurn = -1;
  state.regionLabelTurn = -1;
  state.regionLabelRevision = -1;
  state.regionLabels = [];
  state.moveOverlayTurn = -1;
  state.moveOverlayRevision = -1;
  state.moveOverlay = null;
  hideCanvasTooltip();
  const ctx = els.canvas.getContext("2d");
  ctx.setTransform(1, 0, 0, 1, 0, 0);
  ctx.clearRect(0, 0, els.canvas.width, els.canvas.height);
}

function hslToRgb(h, s, l) {
  const c = (1 - Math.abs(2 * l - 1)) * s;
  const hp = h / 60;
  const x = c * (1 - Math.abs((hp % 2) - 1));
  let rgb;
  if (hp < 1) rgb = [c, x, 0];
  else if (hp < 2) rgb = [x, c, 0];
  else if (hp < 3) rgb = [0, c, x];
  else if (hp < 4) rgb = [0, x, c];
  else if (hp < 5) rgb = [x, 0, c];
  else rgb = [c, 0, x];
  const m = l - c / 2;
  return `#${rgb
    .map((channel) => Math.round((channel + m) * 255).toString(16).padStart(2, "0"))
    .join("")}`;
}

function groupColor(gid) {
  if (!groupColorCache.has(gid)) {
    groupColorCache.set(gid, hslToRgb((gid * 137.508) % 360, GROUP_S, GROUP_L));
  }
  return groupColorCache.get(gid);
}

function darken(hex, factor) {
  const value = Number.parseInt(hex.slice(1), 16);
  const channel = (shift) => Math.round(((value >> shift) & 0xff) * factor);
  return `#${[channel(16), channel(8), channel(0)]
    .map((part) => part.toString(16).padStart(2, "0"))
    .join("")}`;
}

function drawBaseCell(ctx, index) {
  const N = state.N;
  const d = BOARD_SIZE / N;
  const row = Math.floor(index / N);
  const col = index % N;
  const x = col * d;
  const y = row * d;
  const grass = state.grass[index] !== 0;
  ctx.fillStyle = grass ? "#ffffff" : "#4a6070";
  ctx.fillRect(x, y, d, d);
  ctx.strokeStyle = grass ? "#e8e8e8" : "#43596a";
  ctx.lineWidth = 1;
  ctx.strokeRect(x + 0.5, y + 0.5, d - 1, d - 1);
}

function drawOccupiedCell(ctx, index, gid) {
  const N = state.N;
  const d = BOARD_SIZE / N;
  const row = Math.floor(index / N);
  const col = index % N;
  const x = col * d;
  const y = row * d;
  ctx.fillStyle = groupColor(gid);
  ctx.fillRect(x, y, d, d);
  ctx.strokeStyle = "rgba(255,255,255,0.3)";
  ctx.lineWidth = 1;
  ctx.strokeRect(x + 0.5, y + 0.5, d - 1, d - 1);
}

function ensureBackgroundCanvas() {
  if (
    state.backgroundCanvas !== null &&
    state.backgroundRevision === state.visualizationRevision
  ) {
    return;
  }
  const canvas = document.createElement("canvas");
  canvas.width = BOARD_SIZE;
  canvas.height = BOARD_SIZE;
  const ctx = canvas.getContext("2d");
  ctx.fillStyle = "#ffffff";
  ctx.fillRect(0, 0, BOARD_SIZE, BOARD_SIZE);
  for (let index = 0; index < state.N * state.N; index += 1) {
    drawBaseCell(ctx, index);
  }
  state.backgroundCanvas = canvas;
  state.backgroundRevision = state.visualizationRevision;
  state.boardCanvas = null;
  state.previousGrid = null;
  state.boardRevision = -1;
  state.boardTurn = -1;
}

function updateBoardLayer(frame) {
  ensureBackgroundCanvas();
  if (state.boardCanvas === null) {
    state.boardCanvas = document.createElement("canvas");
    state.boardCanvas.width = BOARD_SIZE;
    state.boardCanvas.height = BOARD_SIZE;
  }
  const ctx = state.boardCanvas.getContext("2d");
  const incremental =
    state.boardRevision === state.visualizationRevision &&
    state.previousGrid !== null &&
    state.previousGrid.length === frame.grid.length &&
    Math.abs(state.boardTurn - frame.turn) === 1;

  if (!incremental) {
    ctx.clearRect(0, 0, BOARD_SIZE, BOARD_SIZE);
    ctx.drawImage(state.backgroundCanvas, 0, 0);
    for (let index = 0; index < frame.grid.length; index += 1) {
      const gid = frame.grid[index];
      if (gid !== EMPTY_CELL) {
        drawOccupiedCell(ctx, index, gid);
      }
    }
  } else {
    // A coalesced or manually jumped frame may differ anywhere. Only adjacent frames are safe for
    // the cheap cell-level repaint; every other transition rebuilds from the cached background.
    const d = BOARD_SIZE / state.N;
    for (let index = 0; index < frame.grid.length; index += 1) {
      if (state.previousGrid[index] === frame.grid[index]) {
        continue;
      }
      const row = Math.floor(index / state.N);
      const col = index % state.N;
      ctx.drawImage(state.backgroundCanvas, col * d, row * d, d, d, col * d, row * d, d, d);
      if (frame.grid[index] !== EMPTY_CELL) {
        drawOccupiedCell(ctx, index, frame.grid[index]);
      }
    }
  }

  state.previousGrid = frame.grid.slice();
  state.boardRevision = state.visualizationRevision;
  state.boardTurn = frame.turn;
}

function regionPaths(grid) {
  const paths = new Map();
  const N = state.N;
  const d = BOARD_SIZE / N;
  const pathFor = (gid) => {
    if (!paths.has(gid)) {
      paths.set(gid, new Path2D());
    }
    return paths.get(gid);
  };
  for (let index = 0; index < grid.length; index += 1) {
    const gid = grid[index];
    if (gid === EMPTY_CELL) {
      continue;
    }
    const row = Math.floor(index / N);
    const col = index % N;
    const x = col * d;
    const y = row * d;
    const path = pathFor(gid);
    if (row === 0 || grid[index - N] !== gid) {
      path.moveTo(x, y);
      path.lineTo(x + d, y);
    }
    if (row + 1 === N || grid[index + N] !== gid) {
      path.moveTo(x, y + d);
      path.lineTo(x + d, y + d);
    }
    if (col === 0 || grid[index - 1] !== gid) {
      path.moveTo(x, y);
      path.lineTo(x, y + d);
    }
    if (col + 1 === N || grid[index + 1] !== gid) {
      path.moveTo(x + d, y);
      path.lineTo(x + d, y + d);
    }
  }
  return paths;
}

function drawRegionOutlines(ctx, frame) {
  const paths = regionPaths(frame.grid);
  const accented = [];
  const arrived = frame.arrivalAccepted ? frame.arrivalId : -1;
  const moved = frame.movedSet;
  ctx.lineCap = "square";

  const draw = (gid) => {
    const path = paths.get(gid);
    if (!path) {
      return;
    }
    let accent = null;
    let width = 1.4;
    if (gid === arrived) {
      accent = ARRIVED_COLOR;
      width = FOCUS_WIDTH;
    } else if (moved.has(gid)) {
      accent = MOVED_COLOR;
      width = MOVED_WIDTH;
    }
    if (accent !== null) {
      ctx.strokeStyle = "#ffffff";
      ctx.lineWidth = width + HALO_EXTRA;
      ctx.stroke(path);
      ctx.strokeStyle = accent;
      ctx.lineWidth = width;
      ctx.stroke(path);
    } else {
      ctx.strokeStyle = darken(groupColor(gid), 0.5);
      ctx.lineWidth = width;
      ctx.stroke(path);
    }
  };

  for (const gid of frame.activeMap.keys()) {
    if (gid === arrived || moved.has(gid)) {
      accented.push(gid);
    } else {
      draw(gid);
    }
  }
  for (const gid of accented) {
    draw(gid);
  }
}

function currentRegionLabels(frame) {
  if (
    state.regionLabelTurn === frame.turn &&
    state.regionLabelRevision === state.visualizationRevision
  ) {
    return state.regionLabels;
  }
  state.regionLabels = computeRegionLabels(
    frame.grid,
    state.N,
    frame.activeMap,
    frame.now,
    EMPTY_CELL,
  );
  state.regionLabelTurn = frame.turn;
  state.regionLabelRevision = state.visualizationRevision;
  return state.regionLabels;
}

function currentMoveOverlay(frame) {
  if (
    state.moveOverlayTurn === frame.turn &&
    state.moveOverlayRevision === state.visualizationRevision
  ) {
    return state.moveOverlay;
  }

  let overlay = null;
  if (frame.moved.length > 0) {
    const sourceGrid = decodeMoveSourceGrid(frame.moveSources, state.N, EMPTY_CELL);
    if (sourceGrid === null) {
      throw new Error(`move sources for turn ${frame.turn} are missing`);
    }
    const sourceLabels = computeRegionLabels(
      sourceGrid,
      state.N,
      frame.activeMap,
      frame.now,
      EMPTY_CELL,
    );
    if (sourceLabels.length !== frame.moved.length) {
      throw new Error(`move source count for turn ${frame.turn} does not match moved groups`);
    }
    overlay = {
      sourcePaths: regionPaths(sourceGrid),
      arrows: buildMoveArrows(
        sourceLabels,
        currentRegionLabels(frame),
        BOARD_SIZE / state.N,
      ),
    };
  }

  state.moveOverlayTurn = frame.turn;
  state.moveOverlayRevision = state.visualizationRevision;
  state.moveOverlay = overlay;
  return overlay;
}

function drawMoveArrows(ctx, frame) {
  const overlay = currentMoveOverlay(frame);
  if (overlay === null) {
    return;
  }

  ctx.save();
  ctx.strokeStyle = MOVE_ARROW_COLOR;
  ctx.fillStyle = MOVE_ARROW_COLOR;
  ctx.lineWidth = MOVE_ARROW_WIDTH;
  ctx.lineCap = "round";
  ctx.lineJoin = "round";
  for (const arrow of overlay.arrows) {
    ctx.beginPath();
    ctx.moveTo(arrow.startX, arrow.startY);
    ctx.lineTo(arrow.endX, arrow.endY);
    ctx.stroke();

    const angle = Math.atan2(arrow.endY - arrow.startY, arrow.endX - arrow.startX);
    ctx.beginPath();
    ctx.moveTo(arrow.endX, arrow.endY);
    ctx.lineTo(
      arrow.endX - MOVE_ARROW_HEAD_SIZE * Math.cos(angle - Math.PI / 6),
      arrow.endY - MOVE_ARROW_HEAD_SIZE * Math.sin(angle - Math.PI / 6),
    );
    ctx.lineTo(
      arrow.endX - MOVE_ARROW_HEAD_SIZE * Math.cos(angle + Math.PI / 6),
      arrow.endY - MOVE_ARROW_HEAD_SIZE * Math.sin(angle + Math.PI / 6),
    );
    ctx.closePath();
    ctx.fill();
  }
  ctx.restore();
}

function drawMoveSourceOutlines(ctx, frame) {
  const overlay = currentMoveOverlay(frame);
  if (overlay === null) {
    return;
  }

  ctx.save();
  ctx.lineCap = "butt";
  ctx.setLineDash(MOVE_SOURCE_DASH);
  for (const gid of frame.moved) {
    const path = overlay.sourcePaths.get(gid);
    if (path === undefined) {
      throw new Error(`move source outline for group ${gid} is missing`);
    }
    ctx.strokeStyle = "#ffffff";
    ctx.lineWidth = MOVED_WIDTH + HALO_EXTRA;
    ctx.stroke(path);
    ctx.strokeStyle = MOVED_COLOR;
    ctx.lineWidth = MOVED_WIDTH;
    ctx.stroke(path);
  }
  ctx.restore();
}

function regionLabelFont(ctx, text, availableWidth) {
  const font = (size) => `700 ${size}px ${REGION_LABEL_FONT_FAMILY}`;
  ctx.font = font(REGION_LABEL_FONT_SIZE);
  const measuredWidth = ctx.measureText(text).width;
  if (measuredWidth <= availableWidth || measuredWidth <= 0) {
    return font(REGION_LABEL_FONT_SIZE);
  }
  const fittedSize = Math.max(
    REGION_LABEL_MIN_FONT_SIZE,
    (REGION_LABEL_FONT_SIZE * availableWidth) / measuredWidth,
  );
  return font(fittedSize);
}

function drawRemainingTimeLabels(ctx, frame) {
  const d = BOARD_SIZE / state.N;
  ctx.save();
  ctx.textAlign = "center";
  ctx.textBaseline = "middle";
  ctx.lineJoin = "round";
  ctx.strokeStyle = "rgba(255, 255, 255, 0.95)";
  ctx.fillStyle = "#111827";
  ctx.lineWidth = REGION_LABEL_HALO_WIDTH;

  for (const label of currentRegionLabels(frame)) {
    const text = String(label.remaining);
    const regionWidth = (label.bounds.maxCol - label.bounds.minCol + 1) * d;
    const availableWidth = Math.max(1, regionWidth - REGION_LABEL_HORIZONTAL_PADDING);
    ctx.font = regionLabelFont(ctx, text, availableWidth);
    const x = (label.col + 0.5) * d;
    const y = (label.row + 0.5) * d;
    ctx.strokeText(text, x, y);
    ctx.fillText(text, x, y);
  }
  ctx.restore();
}

function canvasText(ctx, x, y, size, color, value, align = "left") {
  ctx.font = `${size}px sans-serif`;
  ctx.fillStyle = color;
  ctx.textAlign = align;
  ctx.textBaseline = "middle";
  ctx.fillText(String(value), x, y);
}

function drawSwatch(ctx, x, y, fill) {
  ctx.fillStyle = fill;
  ctx.fillRect(x, y - 6, 12, 12);
  ctx.strokeStyle = "#666666";
  ctx.lineWidth = 1;
  ctx.strokeRect(x + 0.5, y - 5.5, 11, 11);
}

function idsString(ids, max = 6) {
  const values = Array.from(ids);
  const text = values.slice(0, max).join(", ");
  return values.length > max ? `${text}, +${values.length - max} more` : text;
}

function drawPanel(ctx, frame) {
  const rx = BOARD_SIZE + 20;
  const panelWidth = PANEL_RIGHT - rx - 5;
  const x0 = rx + 10;
  ctx.fillStyle = "#fafafa";
  ctx.fillRect(rx - 1, 0, panelWidth + 2, BOARD_SIZE);
  ctx.strokeStyle = "#bbbbbb";
  ctx.lineWidth = 1;
  ctx.strokeRect(rx - 0.5, 0.5, panelWidth + 1, BOARD_SIZE - 1);

  let y = 24;
  canvasText(ctx, x0, y, 20, "#000000", `frame ${frame.turn} / ${getMaxTurn()}`);
  y += 28;
  canvasText(ctx, x0, y, 16, "#000000", `score = ${state.hasOutput ? formatScore(state.score) : "-"}`);
  y += 22;
  canvasText(ctx, x0, y, 16, "#000000", `money = ${frame.money}`);
  y += 22;
  canvasText(ctx, x0, y, 13, "#666666", `now = ${frame.now}`);
  y += 30;

  if (frame.arrivalId >= 0) {
    const verdict = frame.arrivalAccepted ? "accepted" : "rejected";
    const fill = frame.arrivalAccepted ? "#1a7f37" : "#b02020";
    if (frame.arrivalAccepted) {
      drawSwatch(ctx, x0, y, ARRIVED_COLOR);
    }
    canvasText(ctx, x0 + 18, y, 14, fill, `group ${frame.arrivalId} ${verdict}`);
    y += 18;
    canvasText(
      ctx,
      x0 + 18,
      y,
      12,
      "#444444",
      `T=${frame.arrivalT} P=${frame.arrivalP} V=${frame.arrivalV}`,
    );
    y += 24;
  }
  if (frame.moved.length > 0) {
    drawSwatch(ctx, x0, y, MOVED_COLOR);
    canvasText(ctx, x0 + 18, y, 14, "#000000", `moved: ${frame.moved.length}`);
    y += 18;
    canvasText(ctx, x0 + 18, y, 12, "#444444", idsString(frame.moved));
    y += 24;
  }
  if (frame.departed.length > 0) {
    let fee = 0;
    const ids = [];
    for (let i = 0; i < frame.departed.length; i += 2) {
      ids.push(frame.departed[i]);
      fee += frame.departed[i + 1];
    }
    canvasText(ctx, x0, y, 14, "#000000", `departed: ${ids.length} (+${fee})`);
    y += 18;
    canvasText(ctx, x0 + 18, y, 12, "#444444", idsString(ids));
    y += 24;
  }

  y += 6;
  canvasText(ctx, x0, y, 14, "#000000", `active groups: ${frame.activeMap.size}`);
  y += 20;
  const usedPercent = state.grassCells > 0 ? (100 * frame.cellsUsed) / state.grassCells : 0;
  canvasText(
    ctx,
    x0,
    y,
    13,
    "#666666",
    `used ${frame.cellsUsed}/${state.grassCells} (${usedPercent.toFixed(1)}%)`,
  );
  y += 20;
  canvasText(ctx, x0, y, 13, "#666666", `${frame.accepted} accepted / ${frame.rejected} rejected`);
  y += 20;
  canvasText(ctx, x0, y, 13, "#666666", `fee +${frame.totalFee} / move -${frame.totalMoveCost}`);

  if (frame.comment.trim()) {
    y += 28;
    for (const line of frame.comment.trim().split(/\r?\n/).slice(0, 8)) {
      canvasText(ctx, x0, y, 12, "#2266aa", `# ${line}`);
      y += 16;
    }
  }

  const legendY = BOARD_SIZE - 68;
  canvasText(ctx, x0, legendY, 13, "#000000", "color: group id");
  canvasText(ctx, x0, legendY + 22, 12, "#666666", "hue = group index");
}

function normalizeFrame(message) {
  const activeMap = new Map();
  for (let i = 0; i < message.actives.length; i += 7) {
    const active = {
      id: message.actives[i],
      p: message.actives[i + 1],
      v: message.actives[i + 2],
      t: message.actives[i + 3],
      l: message.actives[i + 4],
      maxL: message.actives[i + 5],
      fee: message.actives[i + 6],
    };
    activeMap.set(active.id, active);
  }
  return {
    ...message,
    activeMap,
    movedSet: new Set(message.moved),
  };
}

function drawCanvasFrame(frame) {
  updateBoardLayer(frame);
  const dpr = Math.max(1, window.devicePixelRatio || 1);
  const ctx = els.canvas.getContext("2d");
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  ctx.clearRect(0, 0, CANVAS_WIDTH, CANVAS_HEIGHT);
  ctx.fillStyle = "#ffffff";
  ctx.fillRect(0, 0, CANVAS_WIDTH, CANVAS_HEIGHT);
  ctx.save();
  ctx.translate(CANVAS_MARGIN, CANVAS_MARGIN);
  ctx.imageSmoothingEnabled = false;
  ctx.drawImage(state.boardCanvas, 0, 0);
  drawRegionOutlines(ctx, frame);
  drawMoveSourceOutlines(ctx, frame);
  drawMoveArrows(ctx, frame);
  drawRemainingTimeLabels(ctx, frame);
  ctx.strokeStyle = "#000000";
  ctx.lineWidth = 2;
  ctx.strokeRect(1, 1, BOARD_SIZE - 2, BOARD_SIZE - 2);
  drawPanel(ctx, frame);
  ctx.restore();
  state.currentFrame = frame;
}

function scheduleCanvasFrame(frame) {
  state.pendingCanvasFrame = frame;
  if (state.canvasRafId !== null) {
    return;
  }
  state.canvasRafId = requestAnimationFrame(() => {
    state.canvasRafId = null;
    const latest = state.pendingCanvasFrame;
    state.pendingCanvasFrame = null;
    if (latest !== null && latest.revision === state.visualizationRevision) {
      drawCanvasFrame(latest);
    }
  });
}

function resizeCanvas() {
  const dpr = Math.max(1, window.devicePixelRatio || 1);
  const width = Math.round(CANVAS_WIDTH * dpr);
  const height = Math.round(CANVAS_HEIGHT * dpr);
  if (els.canvas.width === width && els.canvas.height === height) {
    return;
  }
  els.canvas.width = width;
  els.canvas.height = height;
  if (state.currentFrame !== null) {
    drawCanvasFrame(state.currentFrame);
  }
}

function hideCanvasTooltip() {
  els.canvasTooltip.hidden = true;
}

function updateCanvasTooltip(event) {
  const frame = state.currentFrame;
  if (frame === null || state.N <= 0) {
    hideCanvasTooltip();
    return;
  }
  const rect = els.canvas.getBoundingClientRect();
  const logicalX = ((event.clientX - rect.left) * CANVAS_WIDTH) / rect.width - CANVAS_MARGIN;
  const logicalY = ((event.clientY - rect.top) * CANVAS_HEIGHT) / rect.height - CANVAS_MARGIN;
  if (logicalX < 0 || logicalY < 0 || logicalX >= BOARD_SIZE || logicalY >= BOARD_SIZE) {
    hideCanvasTooltip();
    return;
  }
  const d = BOARD_SIZE / state.N;
  const col = Math.floor(logicalX / d);
  const row = Math.floor(logicalY / d);
  const gid = frame.grid[row * state.N + col];
  const active = frame.activeMap.get(gid);
  if (!active) {
    hideCanvasTooltip();
    return;
  }
  const cNow = (4 * Math.sqrt(active.p)) / active.l;
  const cMin = (4 * Math.sqrt(active.p)) / active.maxL;
  els.canvasTooltip.textContent =
    `group ${gid} (P=${active.p}, V=${active.v}, T=${active.t}, ` +
    `L=${active.l}, C(now)=${cNow.toFixed(3)}, C(min)=${cMin.toFixed(3)}, fee=${active.fee})`;
  const host = els.canvas.parentElement.getBoundingClientRect();
  els.canvasTooltip.style.left = `${event.clientX - host.left + 12}px`;
  els.canvasTooltip.style.top = `${event.clientY - host.top + 12}px`;
  els.canvasTooltip.hidden = false;
}

function setCustomInputCase() {
  if (state.currentCase === CUSTOM_CASE_VALUE) {
    return;
  }
  cancelCaseLoad();
  state.loadSeq += 1;
  state.currentCase = CUSTOM_CASE_VALUE;
  populateCaseOptions(true);
  updateCaseButtons();
}

function resetFrameRequests() {
  state.workerPreparedRevision = -1;
  state.frameRequestInFlight = null;
  state.queuedFrameTurn = null;
}

function refreshVisualizationSource(preferLastTurn = false) {
  const requestedTurn = Number(els.turn.value) || 0;
  const pendingTurn = Number.isFinite(state.pendingTurn) ? state.pendingTurn : null;
  state.pendingTurn = null;
  state.visualizationRevision += 1;
  const revision = state.visualizationRevision;
  resetFrameRequests();
  hideCanvasTooltip();

  if (!state.sourceInput.trim()) {
    state.sourcePreparing = false;
    els.turn.max = "0";
    els.turn.value = "0";
    els.turn.disabled = true;
    syncTurnDisplay();
    els.score.textContent = "-";
    els.error.textContent = "";
    clearCanvas();
    updatePlayButton();
    updateRunButton();
    return;
  }

  state.sourcePreparing = true;
  els.turn.disabled = true;
  updatePlayButton();
  updateRunButton();
  const requestId = ++state.prepareRequestId;
  state.activePrepareRequest = { requestId, revision };
  state.preparePreference = { requestedTurn, pendingTurn, preferLastTurn };
  visualizerWorker.postMessage({
    type: "prepare",
    requestId,
    revision,
    input: state.sourceInput,
    output: state.sourceOutput,
  });
}

function pumpFrameRequest() {
  if (
    !state.ready ||
    state.sourcePreparing ||
    state.workerPreparedRevision !== state.visualizationRevision ||
    state.frameRequestInFlight !== null ||
    state.queuedFrameTurn === null
  ) {
    return;
  }
  const turn = state.queuedFrameTurn;
  state.queuedFrameTurn = null;
  const requestId = ++state.frameRequestId;
  const revision = state.visualizationRevision;
  state.frameRequestInFlight = { requestId, revision, turn };
  visualizerWorker.postMessage({ type: "frame", requestId, revision, turn });
}

function requestFrame(turn) {
  if (state.sourcePreparing || state.workerPreparedRevision !== state.visualizationRevision) {
    return;
  }
  const maxTurn = getMaxTurn();
  const clamped = Math.max(0, Math.min(maxTurn, Math.round(Number(turn) || 0)));
  els.turn.value = String(clamped);
  syncTurnDisplay();
  state.queuedFrameTurn = clamped;
  pumpFrameRequest();
}

function render() {
  requestFrame(Number(els.turn.value) || 0);
}

function handlePrepared(message) {
  const active = state.activePrepareRequest;
  if (
    active === null ||
    active.requestId !== message.requestId ||
    message.revision !== state.visualizationRevision
  ) {
    return;
  }
  state.activePrepareRequest = null;
  state.sourcePreparing = false;
  state.workerPreparedRevision = message.revision;
  state.N = Math.max(1, Number(message.N) || 1);
  state.grass = message.grass;
  state.grassCells = Number(message.grassCells) || 0;
  state.maxTimeLeft = Number(message.maxTimeLeft) || 1;
  state.hasOutput = Boolean(message.hasOutput);
  state.score = Number(message.score) || 0;
  state.backgroundRevision = -1;
  state.boardRevision = -1;
  state.previousGrid = null;
  state.regionLabelTurn = -1;
  state.regionLabelRevision = -1;
  state.regionLabels = [];
  state.moveOverlayTurn = -1;
  state.moveOverlayRevision = -1;
  state.moveOverlay = null;

  const maxTurn = Math.max(0, Number(message.maxTurn) || 0);
  els.turn.max = String(maxTurn);
  els.turn.disabled = maxTurn <= 0;
  const preference = state.preparePreference ?? {};
  const requested = Number.isFinite(preference.pendingTurn)
    ? preference.pendingTurn
    : preference.preferLastTurn
      ? Number(message.defaultTurn) || 0
      : preference.requestedTurn;
  const turn = Math.max(0, Math.min(maxTurn, Math.round(Number(requested) || 0)));
  els.turn.value = String(turn);
  syncTurnDisplay();
  if (state.hasOutput) {
    els.score.textContent = formatScore(state.score);
    els.elapsedTime.textContent = formatElapsedMs(state.currentElapsedMs);
    els.error.textContent = message.error || "";
  } else {
    els.score.textContent = "-";
    els.elapsedTime.textContent = "-";
    els.error.textContent = "";
  }
  updatePlayButton();
  updateRunButton();
  requestFrame(turn);
}

function handleVisualizerWorkerMessage(event) {
  const message = event.data ?? {};
  if (message.type === "ready") {
    resolveVisualizerWorkerReady();
    return;
  }
  if (message.type === "fatal") {
    const error = new Error(message.error || "visualizer worker initialization failed");
    rejectVisualizerWorkerReady(error);
    state.ready = false;
    state.sourcePreparing = false;
    setRunStatus(`可視化 Worker エラー: ${error.message}`, true);
    els.error.textContent = error.message;
    updatePlayButton();
    updateRunButton();
    return;
  }
  if (message.type === "prepared") {
    handlePrepared(message);
    return;
  }
  if (message.type === "prepare-error") {
    const active = state.activePrepareRequest;
    if (
      active !== null &&
      active.requestId === message.requestId &&
      message.revision === state.visualizationRevision
    ) {
      state.activePrepareRequest = null;
      state.sourcePreparing = false;
      els.turn.max = "0";
      els.turn.value = "0";
      els.turn.disabled = true;
      syncTurnDisplay();
      els.score.textContent = "0";
      els.error.textContent = message.error || "visualizer preparation failed";
      clearCanvas();
      updatePlayButton();
      updateRunButton();
    }
    return;
  }
  if (message.type === "frame-error" || message.type === "frame") {
    const active = state.frameRequestInFlight;
    if (active === null || active.requestId !== message.requestId) {
      return;
    }
    state.frameRequestInFlight = null;
    const hasNewerFrame = state.queuedFrameTurn !== null;
    if (
      message.type === "frame-error" &&
      message.revision === state.visualizationRevision &&
      !hasNewerFrame
    ) {
      els.error.textContent = message.error || "frame generation failed";
    } else if (
      message.type === "frame" &&
      message.revision === state.visualizationRevision &&
      !hasNewerFrame
    ) {
      scheduleCanvasFrame(normalizeFrame(message));
    }
    pumpFrameRequest();
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
  els.runBinBtn.disabled =
    state.running || state.sourcePreparing || !runnable || !state.sourceInput.trim();
}

function stepTurn(delta) {
  const maxTurn = getMaxTurn();
  if (maxTurn <= 0 || state.sourcePreparing) {
    return;
  }
  const now = Number(els.turn.value) || 0;
  const next = Math.max(0, Math.min(maxTurn, now + delta));
  if (next === now) {
    return;
  }
  state.playbackTurnFloat = next;
  requestFrame(next);
  saveCaseViewState();
}

function playbackFrame(ts) {
  if (!state.playing) {
    return;
  }
  const maxTurn = getMaxTurn();
  if (maxTurn <= 0 || state.sourcePreparing) {
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
    syncTurnDisplay();
    renderAtMost60Hz(ts);
  }
  state.rafId = requestAnimationFrame(playbackFrame);
}

function startPlayback() {
  const maxTurn = getMaxTurn();
  if (maxTurn <= 0 || state.playing || state.sourcePreparing) {
    return;
  }
  if ((Number(els.turn.value) || 0) + 1 >= maxTurn) {
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
  } else if (previous === "" && preserveSelection) {
    state.currentBin = previous;
  } else {
    state.currentBin = state.bins[0] ?? "";
    state.preserveEmptyBinSelection = state.currentBin === "";
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
  configureCaseStorage(data.projectKey);
  state.bins = Array.isArray(data.bins) ? data.bins : [];
  state.runnableBins = new Set(Array.isArray(data.runnableBins) ? data.runnableBins : []);
  state.cases = Array.isArray(data.cases) ? data.cases : [];
  state.inputCache.clear();
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

function cancelCaseLoad() {
  if (state.caseLoadController !== null) {
    state.caseLoadController.abort();
    state.caseLoadController = null;
  }
}

function cancelSourceEditRefresh() {
  if (state.sourceEditTimer !== null) {
    clearTimeout(state.sourceEditTimer);
    state.sourceEditTimer = null;
  }
}

function scheduleSelectedCaseLoad(preferLastTurn = true) {
  cancelCaseLoad();
  const seq = ++state.loadSeq;
  // Start immediately. AbortController and loadSeq discard superseded rapid-click requests.
  void loadSelectedCase(preferLastTurn, seq);
}

async function loadSelectedCase(preferLastTurn = true, scheduledSeq = null) {
  if (!state.currentCase) {
    cancelCaseLoad();
    state.loadSeq += 1;
    state.sourcePreparing = false;
    updateCaseButtons();
    updateRunButton();
    setElapsedMs(null);
    refreshVisualizationSource(false);
    setRunStatus("custom input を表示中");
    return;
  }

  stopPlayback();
  if (scheduledSeq === null) {
    cancelCaseLoad();
  }
  cancelSourceEditRefresh();
  const seq = scheduledSeq ?? ++state.loadSeq;
  state.sourcePreparing = true;
  updatePlayButton();
  updateRunButton();
  setRunStatus(`${state.currentCase} を読込中...`);
  const controller = new AbortController();
  state.caseLoadController = controller;

  try {
    const cachedInput = state.inputCache.get(state.currentCase);
    const params = new URLSearchParams({
      caseName: state.currentCase,
      binName: state.currentBin,
      includeInput: cachedInput === undefined ? "1" : "0",
    });
    const res = await fetch(`/api/visualizer-case?${params.toString()}`, {
      signal: controller.signal,
    });
    if (!res.ok) {
      const data = await res.json().catch(() => ({}));
      throw new Error(data.error || `HTTP ${res.status}`);
    }
    const data = await res.json();
    if (seq !== state.loadSeq) {
      return;
    }
    const input = typeof data.input === "string" ? data.input : cachedInput;
    if (typeof input !== "string") {
      throw new Error("case input was not returned and is not cached");
    }
    state.inputCache.set(state.currentCase, input);
    state.sourceInput = input;
    state.sourceOutput = data.output ?? "";
    setElapsedMs(data.elapsedMs);
    // Keep Output out of its hidden textarea; materializing tens of thousands of editable lines
    // can block the main thread even though replay parsing itself runs in a Worker.
    refreshVisualizationSource(preferLastTurn);
    setTextareaValue(els.inputArea, state.sourceInput, "input");
    updateRunButton();

    if (data.outputExists) {
      setRunStatus(`${state.currentCase} を読込: ${state.currentBin || "(bin未選択)"} の既存 output を反映`);
    } else if (state.currentBin) {
      setRunStatus(`${state.currentCase} を読込: 既存 output は未作成`);
    } else {
      setRunStatus(`${state.currentCase} を読込: bin 未選択のため input のみ表示`);
    }
  } catch (error) {
    if (error?.name === "AbortError" || seq !== state.loadSeq) {
      return;
    }
    state.sourceInput = "";
    state.sourceOutput = "";
    setTextareaValue(els.inputArea, "", "input");
    setElapsedMs(null);
    state.sourcePreparing = false;
    refreshVisualizationSource(false);
    updateRunButton();
    setRunStatus(`case 読込失敗: ${String(error)}`, true);
  } finally {
    if (state.caseLoadController === controller) {
      state.caseLoadController = null;
    }
  }
}

function scheduleSourceRefresh() {
  if (state.sourceEditTimer !== null) {
    clearTimeout(state.sourceEditTimer);
  }
  state.sourcePreparing = true;
  updatePlayButton();
  updateRunButton();
  state.sourceEditTimer = window.setTimeout(() => {
    state.sourceEditTimer = null;
    refreshVisualizationSource(false);
  }, SOURCE_EDIT_SETTLE_MS);
}

async function runSelectedRustBin() {
  if (state.running || !state.runnableBins.has(state.currentBin)) {
    return;
  }
  if (!state.sourceInput.trim()) {
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
        input: state.sourceInput,
      }),
    });
    const data = await res.json();
    if (!res.ok) {
      throw new Error(data.error || `HTTP ${res.status}`);
    }
    state.sourceOutput = data.output ?? "";
    setElapsedMs(data.elapsedMs);
    refreshVisualizationSource(true);
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
  els.caseName.value = state.currentCase;
  updateCaseButtons();
  saveCaseViewState();
  scheduleSelectedCaseLoad(true);
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
  els.rustBin.value = state.currentBin;
  updateBinButtons();
  updateRunButton();
  if (state.currentCase !== CUSTOM_CASE_VALUE) {
    scheduleSelectedCaseLoad(true);
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

for (const select of [els.rustBin, els.caseName]) {
  select.addEventListener("pointerdown", () => {
    if (state.playing) {
      stopPlayback();
    }
  });
}

els.rustBin.addEventListener("change", () => {
  state.currentBin = els.rustBin.value;
  state.preserveEmptyBinSelection = state.currentBin === "";
  updateBinButtons();
  updateRunButton();
  if (state.currentCase !== CUSTOM_CASE_VALUE) {
    scheduleSelectedCaseLoad(true);
  }
  saveCaseViewState();
});

els.binPrevBtn.addEventListener("click", () => moveBin(-1));
els.binNextBtn.addEventListener("click", () => moveBin(1));

els.caseName.addEventListener("change", () => {
  state.currentCase = els.caseName.value;
  updateCaseButtons();
  if (state.currentCase !== CUSTOM_CASE_VALUE) {
    scheduleSelectedCaseLoad(true);
  } else {
    cancelCaseLoad();
    cancelSourceEditRefresh();
    state.loadSeq += 1;
    state.sourcePreparing = false;
    updatePlayButton();
    updateRunButton();
    setRunStatus("custom input を表示中");
  }
  saveCaseViewState();
});

els.casePrevBtn.addEventListener("click", () => moveCase(-1));
els.caseNextBtn.addEventListener("click", () => moveCase(1));
els.runBinBtn.addEventListener("click", () => void runSelectedRustBin());

setupHoldButton(els.prevBtn, -1);
setupHoldButton(els.nextBtn, 1);
els.playBtn.addEventListener("click", togglePlayback);

els.turn.addEventListener("input", () => {
  stopPlayback();
  state.playbackTurnFloat = Number(els.turn.value) || 0;
  render();
  saveCaseViewState();
});

els.speed.addEventListener("change", saveCaseViewState);

els.inputArea.addEventListener("input", () => {
  if (state.suppressInputDirty) {
    return;
  }
  stopPlayback();
  setCustomInputCase();
  setElapsedMs(null);
  state.sourceInput = els.inputArea.value;
  scheduleSourceRefresh();
  updateRunButton();
  saveCaseViewState();
});

els.outputArea.addEventListener("input", () => {
  if (state.suppressOutputDirty) {
    return;
  }
  stopPlayback();
  cancelCaseLoad();
  state.loadSeq += 1;
  setElapsedMs(null);
  state.sourceOutput = els.outputArea.value;
  scheduleSourceRefresh();
  saveCaseViewState();
});

window.addEventListener("pagehide", persistCaseViewState);
window.addEventListener("resize", resizeCanvas);
els.canvas.addEventListener("mousemove", updateCanvasTooltip);
els.canvas.addEventListener("mouseleave", hideCanvasTooltip);

els.copyInputBtn.addEventListener("click", () => void copyText(state.sourceInput, "Input"));
els.copyOutputBtn.addEventListener("click", () => void copyText(state.sourceOutput, "Output"));

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
  resizeCanvas();
  await visualizerWorkerReady;
  state.ready = true;
  updatePlayButton();
  await refreshVisualizerData();
  await loadSelectedCase(true);
  window.setInterval(() => void pollBinsQuietly(), BIN_POLL_INTERVAL_MS);
}

main().catch((error) => {
  state.sourcePreparing = false;
  setRunStatus(`初期化失敗: ${String(error)}`, true);
  els.error.textContent = String(error);
  updatePlayButton();
  updateRunButton();
});
