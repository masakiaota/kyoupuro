const ROW_SORT_KEYS = new Set(["bin", "totalAvg", "executedAt"]);
const SCORE_UNITS = new Set(["raw", "k", "m", "cap100k"]);
// 一度に描画するケース列と行の数。評価セットが大きくても表のセル数を抑える。
export const CASE_PAGE_SIZE = 100;
export const ROW_PAGE_SIZE = 200;

export function createInitialState(data, savedState = null) {
  // data は選択中の評価セット 1 つぶんだけを持つ。
  const selectedEvalSet = typeof data.evalSet === "string" ? data.evalSet : "";
  return {
    data,
    selectedEvalSet,
    caseSort: normalizeCaseSort(data, selectedEvalSet, savedState?.caseSort),
    rowSort: Object.prototype.hasOwnProperty.call(savedState ?? {}, "rowSort")
      ? normalizeRowSort(savedState.rowSort)
      : { key: "totalAvg", dir: "desc" },
    manualRunOrder: normalizeManualRunOrder(data, selectedEvalSet, savedState?.manualRunOrder),
    scoreUnit: SCORE_UNITS.has(savedState?.scoreUnit) ? savedState.scoreUnit : "raw",
    casePage: 0,
    rowLimit: ROW_PAGE_SIZE,
    evalVersionSignature: "",
    lastUpdatedAt: new Date(),
    isPolling: false,
  };
}

function resetViewForEvalSet(state) {
  state.caseSort = firstCaseSortKey(state.data, state.selectedEvalSet);
  state.rowSort = { key: "totalAvg", dir: "desc" };
  state.manualRunOrder = [];
  state.casePage = 0;
  state.rowLimit = ROW_PAGE_SIZE;
}

export function replaceEvalData(state, data) {
  const previousEvalSet = state.selectedEvalSet;
  state.data = data;
  state.selectedEvalSet = typeof data.evalSet === "string" ? data.evalSet : "";
  if (state.selectedEvalSet !== previousEvalSet) {
    resetViewForEvalSet(state);
    return;
  }

  state.caseSort = normalizeCaseSort(data, state.selectedEvalSet, state.caseSort);
  state.rowSort = normalizeRowSort(state.rowSort);
  state.scoreUnit = SCORE_UNITS.has(state.scoreUnit) ? state.scoreUnit : "raw";
  const runs = getRunsForEvalSet(data, state.selectedEvalSet);
  const validRunIds = new Set(runs.map((run) => run.id));
  state.manualRunOrder = state.manualRunOrder.filter((id) => validRunIds.has(id));
}

export function getRunsForEvalSet(data, evalSet) {
  return data.evalSet === evalSet && Array.isArray(data.runs) ? data.runs : [];
}

export function getCaseNamesForEvalSet(data, evalSet) {
  return data.evalSet === evalSet && Array.isArray(data.caseNames) ? data.caseNames : [];
}

export function getCaseSortOptionsForEvalSet(data, evalSet) {
  return data.evalSet === evalSet && Array.isArray(data.caseSortOptions)
    ? data.caseSortOptions
    : [];
}

export function getCaseMetaForEvalSet(data, evalSet) {
  return data.evalSet === evalSet ? (data.caseMeta ?? {}) : {};
}

export function firstCaseSortKey(data, evalSet) {
  const first = getCaseSortOptionsForEvalSet(data, evalSet)[0];
  return typeof first?.key === "string" ? first.key : "case_name_asc";
}

function normalizeCaseSort(data, evalSet, caseSort) {
  const key = typeof caseSort === "string" ? caseSort : "";
  const options = getCaseSortOptionsForEvalSet(data, evalSet);
  if (options.some((option) => option?.key === key)) {
    return key;
  }
  return firstCaseSortKey(data, evalSet);
}

function normalizeRowSort(rowSort) {
  if (rowSort === null) {
    return null;
  }
  if (!rowSort || typeof rowSort !== "object") {
    return { key: "totalAvg", dir: "desc" };
  }
  const key = ROW_SORT_KEYS.has(rowSort.key) ? rowSort.key : "totalAvg";
  const dir = rowSort.dir === "asc" ? "asc" : "desc";
  return { key, dir };
}

function normalizeManualRunOrder(data, evalSet, manualRunOrder) {
  if (!Array.isArray(manualRunOrder)) {
    return [];
  }
  const validRunIds = new Set(getRunsForEvalSet(data, evalSet).map((run) => run.id));
  const seen = new Set();
  const result = [];
  for (const id of manualRunOrder) {
    if (typeof id !== "string" || !validRunIds.has(id) || seen.has(id)) {
      continue;
    }
    seen.add(id);
    result.push(id);
  }
  return result;
}

export function orderedRuns(state, runs) {
  if (state.rowSort) {
    return sortRuns(runs, state.rowSort);
  }
  const runById = new Map(runs.map((run) => [run.id, run]));
  const ordered = [];
  for (const id of state.manualRunOrder) {
    if (runById.has(id)) {
      ordered.push(runById.get(id));
      runById.delete(id);
    }
  }
  ordered.push(...Array.from(runById.values()));
  return ordered;
}

export function sortRuns(runs, sort) {
  const key = sort?.key;
  const dir = sort?.dir === "asc" ? 1 : -1;
  return [...runs].sort((left, right) => {
    let result = 0;
    if (key === "bin") {
      result = left.bin.localeCompare(right.bin, "ja");
    } else if (key === "executedAt") {
      result = left.executedAt.localeCompare(right.executedAt, "ja");
    } else {
      result = Number(left.totalAvg) - Number(right.totalAvg);
    }
    if (result !== 0) {
      return result * dir;
    }
    return left.id.localeCompare(right.id, "ja");
  });
}
