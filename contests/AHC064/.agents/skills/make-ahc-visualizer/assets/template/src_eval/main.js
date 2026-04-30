import { renderApp } from "./render.js";
import { createInitialState, replaceEvalData, setEvalSet } from "./state.js";

const POLL_INTERVAL_MS = 2000;

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
    const state = createInitialState(data);
    state.evalVersionSignature = typeof version.signature === "string" ? version.signature : "";

    els.evalSet.addEventListener("change", () => {
      setEvalSet(state, els.evalSet.value);
      renderApp(state, els);
    });

    els.caseSort.addEventListener("change", () => {
      state.caseSort = els.caseSort.value;
      renderApp(state, els);
    });

    els.scoreUnit.addEventListener("click", (event) => {
      const button = event.target.closest("[data-score-unit]");
      if (!button) {
        return;
      }
      state.scoreUnit = button.dataset.scoreUnit;
      renderApp(state, els);
    });

    renderApp(state, els);
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
