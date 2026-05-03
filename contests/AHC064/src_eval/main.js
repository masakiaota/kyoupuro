import { renderApp } from "./render.js";
import { createInitialState, replaceEvalData, setEvalSet } from "./state.js";
import { readSessionState, writeSessionState } from "../src_common/session_state.js";

const POLL_INTERVAL_MS = 2000;
const STORAGE_KEY = "ahc064:eval-viewer:v1";
const SCORE_UNITS = new Set(["raw", "k", "m", "cap100k"]);

const els = {
  evalSet: document.getElementById("evalSet"),
  caseSort: document.getElementById("caseSort"),
  scoreUnit: document.getElementById("scoreUnit"),
  tableEmpty: document.getElementById("tableEmpty"),
  resultTable: document.getElementById("resultTable"),
};

async function loadEvalViewData() {
  const res = await fetch("/api/eval-view-data");
  if (!res.ok) {
    throw new Error(`HTTP ${res.status}`);
  }
  return res.json();
}

async function loadEvalViewVersion() {
  const res = await fetch("/api/eval-view-version");
  if (!res.ok) {
    throw new Error(`HTTP ${res.status}`);
  }
  return res.json();
}

async function pollEvalViewData(state) {
  if (state.isPolling || document.hidden) {
    return;
  }
  state.isPolling = true;
  try {
    const version = await loadEvalViewVersion();
    const signature = typeof version.signature === "string" ? version.signature : "";
    if (signature && signature !== state.evalVersionSignature) {
      const data = await loadEvalViewData();
      replaceEvalData(state, data);
      state.evalVersionSignature = signature;
      state.lastUpdatedAt = new Date();
      renderApp(state, els);
      saveViewState(state);
    }
  } catch (error) {
    console.warn("eval viewer auto refresh failed", error);
  } finally {
    state.isPolling = false;
  }
}

function applySavedViewState(state) {
  const savedState = readSessionState(STORAGE_KEY);
  const evalSets = Array.isArray(state.data.evalSets) ? state.data.evalSets : [];
  if (
    typeof savedState.selectedEvalSet === "string" &&
    evalSets.includes(savedState.selectedEvalSet)
  ) {
    setEvalSet(state, savedState.selectedEvalSet);
  }

  const caseSortOptions = Array.isArray(
    state.data.caseSortOptionsByEvalSet?.[state.selectedEvalSet],
  )
    ? state.data.caseSortOptionsByEvalSet[state.selectedEvalSet]
    : [];
  const knownCaseSorts = new Set(caseSortOptions.map((option) => option.key));
  if (typeof savedState.caseSort === "string" && knownCaseSorts.has(savedState.caseSort)) {
    state.caseSort = savedState.caseSort;
  }

  if (typeof savedState.scoreUnit === "string" && SCORE_UNITS.has(savedState.scoreUnit)) {
    state.scoreUnit = savedState.scoreUnit;
  }
}

function saveViewState(state) {
  writeSessionState(STORAGE_KEY, {
    selectedEvalSet: state.selectedEvalSet,
    caseSort: state.caseSort,
    scoreUnit: state.scoreUnit,
  });
}

async function main() {
  try {
    const [data, version] = await Promise.all([
      loadEvalViewData(),
      loadEvalViewVersion(),
    ]);
    const state = createInitialState(data);
    applySavedViewState(state);
    state.evalVersionSignature = typeof version.signature === "string" ? version.signature : "";

    els.evalSet.addEventListener("change", () => {
      setEvalSet(state, els.evalSet.value);
      renderApp(state, els);
      saveViewState(state);
    });

    els.caseSort.addEventListener("change", () => {
      state.caseSort = els.caseSort.value;
      renderApp(state, els);
      saveViewState(state);
    });

    els.scoreUnit.addEventListener("click", (event) => {
      const button = event.target.closest("[data-score-unit]");
      if (!button) {
        return;
      }
      state.scoreUnit = button.dataset.scoreUnit;
      renderApp(state, els);
      saveViewState(state);
    });

    renderApp(state, els);
    saveViewState(state);
    window.addEventListener("pagehide", () => saveViewState(state));
    setInterval(() => {
      void pollEvalViewData(state);
    }, POLL_INTERVAL_MS);
  } catch (error) {
    els.tableEmpty.hidden = false;
    els.tableEmpty.textContent = String(error);
    els.resultTable.hidden = true;
  }
}

main();
