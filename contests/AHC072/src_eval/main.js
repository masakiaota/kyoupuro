import { renderApp } from "./render.js";
import { createInitialState, replaceEvalData, setEvalSet } from "./state.js";

const POLL_INTERVAL_MS = 2000;
const STORAGE_PREFIX = "ahc-visualizer";
const STORAGE_VERSION = "v1";

const els = {
  evalSet: document.getElementById("evalSet"),
  caseSort: document.getElementById("caseSort"),
  scoreUnit: document.getElementById("scoreUnit"),
  tableEmpty: document.getElementById("tableEmpty"),
  resultTable: document.getElementById("resultTable"),
};

let evalStorageKey = "";

function makeStorageKey(projectKey) {
  const key = typeof projectKey === "string" && projectKey ? projectKey : "unknown";
  return `${STORAGE_PREFIX}:${key}:eval:${STORAGE_VERSION}`;
}

function configureEvalStorage(projectKey) {
  evalStorageKey = makeStorageKey(projectKey);
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
    // Storage errors should not break the eval viewer itself.
  }
}

function loadSavedEvalState(data) {
  configureEvalStorage(data.projectKey);
  return readSessionJson(evalStorageKey);
}

function saveEvalState(state) {
  if (!evalStorageKey) {
    return;
  }
  writeSessionJson(evalStorageKey, {
    selectedEvalSet: state.selectedEvalSet,
    caseSort: state.caseSort,
    rowSort: state.rowSort,
    manualRunOrder: state.manualRunOrder,
    scoreUnit: state.scoreUnit,
  });
}

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
      configureEvalStorage(data.projectKey);
      replaceEvalData(state, data);
      state.evalVersionSignature = signature;
      state.lastUpdatedAt = new Date();
      renderApp(state, els, () => saveEvalState(state));
      saveEvalState(state);
    }
  } catch (error) {
    console.warn("eval viewer auto refresh failed", error);
  } finally {
    state.isPolling = false;
  }
}

async function main() {
  try {
    const [data, version] = await Promise.all([
      loadEvalViewData(),
      loadEvalViewVersion(),
    ]);
    const state = createInitialState(data, loadSavedEvalState(data));
    state.evalVersionSignature = typeof version.signature === "string" ? version.signature : "";

    const renderAndSave = () => {
      renderApp(state, els, () => saveEvalState(state));
      saveEvalState(state);
    };

    els.evalSet.addEventListener("change", () => {
      setEvalSet(state, els.evalSet.value);
      renderAndSave();
    });

    els.caseSort.addEventListener("change", () => {
      state.caseSort = els.caseSort.value;
      renderAndSave();
    });

    els.scoreUnit.addEventListener("click", (event) => {
      const button = event.target.closest("[data-score-unit]");
      if (!button) {
        return;
      }
      state.scoreUnit = button.dataset.scoreUnit;
      renderAndSave();
    });

    renderAndSave();
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
