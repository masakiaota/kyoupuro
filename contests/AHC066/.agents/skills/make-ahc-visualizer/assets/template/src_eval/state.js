const ROW_SORT_KEYS = new Set(["bin", "totalAvg", "executedAt"]);
const SCORE_UNITS = new Set(["raw", "k", "m", "cap100k"]);

export function createInitialState(data, savedState = null) {
  const evalSets = Array.isArray(data.evalSets) ? data.evalSets : [];
  let selectedEvalSet = evalSets[0] ?? "";
  if (
    savedState &&
    typeof savedState.selectedEvalSet === "string" &&
    evalSets.includes(savedState.selectedEvalSet)
  ) {
    selectedEvalSet = savedState.selectedEvalSet;
  }
  return {
    data,
    selectedEvalSet,
    caseSort: normalizeCaseSort(data, selectedEvalSet, savedState?.caseSort),
    rowSort: Object.prototype.hasOwnProperty.call(savedState ?? {}, "rowSort")
      ? normalizeRowSort(savedState.rowSort)
      : { key: "totalAvg", dir: "desc" },
    manualRunOrder: normalizeManualRunOrder(data, selectedEvalSet, savedState?.manualRunOrder),
    scoreUnit: SCORE_UNITS.has(savedState?.scoreUnit) ? savedState.scoreUnit : "raw",
    evalVersionSignature: "",
    lastUpdatedAt: new Date(),
    isPolling: false,
  };
}

export function setEvalSet(state, evalSet) {
  const evalSets = Array.isArray(state.data.evalSets) ? state.data.evalSets : [];
  state.selectedEvalSet = evalSets.includes(evalSet) ? evalSet : (evalSets[0] ?? "");
  state.caseSort = firstCaseSortKey(state.data, state.selectedEvalSet);
  state.rowSort = { key: "totalAvg", dir: "desc" };
  state.manualRunOrder = [];
}

export function replaceEvalData(state, data) {
  const previousEvalSet = state.selectedEvalSet;
  state.data = data;
  const evalSets = Array.isArray(data.evalSets) ? data.evalSets : [];
  if (!evalSets.includes(previousEvalSet)) {
    state.selectedEvalSet = evalSets[0] ?? "";
    state.caseSort = firstCaseSortKey(data, state.selectedEvalSet);
    state.rowSort = { key: "totalAvg", dir: "desc" };
    state.manualRunOrder = [];
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
  return Array.isArray(data.runsByEvalSet?.[evalSet]) ? data.runsByEvalSet[evalSet] : [];
}

export function getCaseNamesForEvalSet(data, evalSet) {
  return Array.isArray(data.caseNamesByEvalSet?.[evalSet]) ? data.caseNamesByEvalSet[evalSet] : [];
}

export function getCaseSortOptionsForEvalSet(data, evalSet) {
  return Array.isArray(data.caseSortOptionsByEvalSet?.[evalSet])
    ? data.caseSortOptionsByEvalSet[evalSet]
    : [];
}

export function getCaseMetaForEvalSet(data, evalSet) {
  return data.caseMetaByEvalSet?.[evalSet] ?? {};
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
