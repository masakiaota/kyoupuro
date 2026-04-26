import {
  getCaseMetaForEvalSet,
  getCaseNamesForEvalSet,
  getCaseSortOptionsForEvalSet,
  getRunsForEvalSet,
  orderedRuns,
} from "./state.js";
import { mergeCaseSortOptions, sortCaseNames } from "./case_sorters.js";
import { renderResultTable } from "./table.js";

export function renderApp(state, els, onStateChange = () => {}) {
  renderEvalSetSelect(state, els);
  renderCaseSortSelect(state, els);
  renderScoreUnitSelect(state, els);

  const runs = orderedRuns(state, getRunsForEvalSet(state.data, state.selectedEvalSet));
  const caseNames = getCaseNamesForEvalSet(state.data, state.selectedEvalSet);
  const caseMetaByName = getCaseMetaForEvalSet(state.data, state.selectedEvalSet);
  const sortedCaseNames = sortCaseNames(caseNames, state.caseSort, caseMetaByName);

  if (runs.length === 0 || sortedCaseNames.length === 0) {
    els.resultTable.hidden = true;
    els.tableEmpty.hidden = false;
    if (runs.length === 0) {
      els.tableEmpty.textContent = "表示できる run が無い。";
    } else {
      els.tableEmpty.textContent = "表示できる case が無い。";
    }
    return;
  }

  els.tableEmpty.hidden = true;
  els.resultTable.hidden = false;
  renderResultTable(els.resultTable, {
    runs,
    caseNames: sortedCaseNames,
    scoreUnit: state.scoreUnit,
    rowSort: state.rowSort,
    onSort(columnKey) {
      if (state.rowSort?.key === columnKey) {
        state.rowSort = {
          key: columnKey,
          dir: state.rowSort.dir === "desc" ? "asc" : "desc",
        };
      } else {
        state.rowSort = { key: columnKey, dir: "desc" };
      }
      renderApp(state, els, onStateChange);
      onStateChange();
    },
    onManualOrder(runIds) {
      state.rowSort = null;
      state.manualRunOrder = runIds;
      renderApp(state, els, onStateChange);
      onStateChange();
    },
  });
}

function renderEvalSetSelect(state, els) {
  const evalSets = Array.isArray(state.data.evalSets) ? state.data.evalSets : [];
  els.evalSet.textContent = "";
  for (const evalSet of evalSets) {
    const option = document.createElement("option");
    option.value = evalSet;
    option.textContent = evalSet;
    els.evalSet.appendChild(option);
  }
  if (evalSets.includes(state.selectedEvalSet)) {
    els.evalSet.value = state.selectedEvalSet;
  }
}

function renderScoreUnitSelect(state, els) {
  if (!["raw", "k", "m", "cap100k"].includes(state.scoreUnit)) {
    state.scoreUnit = "raw";
  }
  for (const button of els.scoreUnit.querySelectorAll("[data-score-unit]")) {
    button.setAttribute("aria-pressed", String(button.dataset.scoreUnit === state.scoreUnit));
  }
}

function renderCaseSortSelect(state, els) {
  const options = mergeCaseSortOptions(
    getCaseSortOptionsForEvalSet(state.data, state.selectedEvalSet),
  );
  const knownKeys = new Set(options.map((option) => option.key));
  if (!knownKeys.has(state.caseSort)) {
    state.caseSort = options[0]?.key ?? "case_name_asc";
  }

  els.caseSort.textContent = "";
  for (const option of options) {
    const element = document.createElement("option");
    element.value = option.key;
    element.textContent = option.label;
    els.caseSort.appendChild(element);
  }
  els.caseSort.value = state.caseSort;
}
