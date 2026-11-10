import {
  CASE_PAGE_SIZE,
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

  const allRuns = orderedRuns(state, getRunsForEvalSet(state.data, state.selectedEvalSet));
  // 行は上位から rowLimit 件だけ描画する。並べ替えは全 run に対して行う。
  const runs = allRuns.slice(0, state.rowLimit);
  const caseNames = getCaseNamesForEvalSet(state.data, state.selectedEvalSet);
  const caseMetaByName = getCaseMetaForEvalSet(state.data, state.selectedEvalSet);
  const allSortedCaseNames = sortCaseNames(caseNames, state.caseSort, caseMetaByName);
  // ケース列は並べ替えた後の 1 ページぶんだけ描画する。
  const pageCount = Math.max(1, Math.ceil(allSortedCaseNames.length / CASE_PAGE_SIZE));
  state.casePage = Math.min(Math.max(0, state.casePage), pageCount - 1);
  const caseBegin = state.casePage * CASE_PAGE_SIZE;
  const sortedCaseNames = allSortedCaseNames.slice(caseBegin, caseBegin + CASE_PAGE_SIZE);
  const caseIndexByName = new Map(caseNames.map((caseName, index) => [caseName, index]));
  renderPagers(state, els, {
    caseBegin,
    caseShown: sortedCaseNames.length,
    caseTotal: allSortedCaseNames.length,
    pageCount,
    rowShown: runs.length,
    rowTotal: allRuns.length,
  });

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
    caseIndexByName,
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
      // 描画していない行は、手動で並べた行の後ろに元の順で続ける。
      const moved = new Set(runIds);
      state.manualRunOrder = [
        ...runIds,
        ...allRuns.map((run) => run.id).filter((id) => !moved.has(id)),
      ];
      renderApp(state, els, onStateChange);
      onStateChange();
    },
  });
}

function renderPagers(state, els, counts) {
  const { caseBegin, caseShown, caseTotal, pageCount, rowShown, rowTotal } = counts;
  els.casePageLabel.textContent =
    caseTotal === 0 ? "0 / 0" : `${caseBegin + 1}–${caseBegin + caseShown} / ${caseTotal}`;
  els.casePrev.disabled = state.casePage <= 0;
  els.caseNext.disabled = state.casePage >= pageCount - 1;
  els.rowMore.hidden = rowShown >= rowTotal;
  els.rowCountLabel.textContent = `${rowShown} / ${rowTotal} runs`;
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
