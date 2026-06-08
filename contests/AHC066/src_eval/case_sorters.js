const SORTERS = {
  case_name_asc: {
    key: "case_name_asc",
    label: "case_name ∧",
    compare: (left, right) => left.localeCompare(right, "ja"),
  },
  case_name_desc: {
    key: "case_name_desc",
    label: "case_name ∨",
    compare: (left, right) => right.localeCompare(left, "ja"),
  },
  N_asc: {
    key: "N_asc",
    label: "N ∧",
    compare: (left, right, leftMeta, rightMeta) =>
      compareNumberThenName(leftMeta?.N, rightMeta?.N, left, right, 1),
  },
  N_desc: {
    key: "N_desc",
    label: "N ∨",
    compare: (left, right, leftMeta, rightMeta) =>
      compareNumberThenName(leftMeta?.N, rightMeta?.N, left, right, -1),
  },
  M_asc: {
    key: "M_asc",
    label: "M ∧",
    compare: (left, right, leftMeta, rightMeta) =>
      compareNumberThenName(leftMeta?.M, rightMeta?.M, left, right, 1),
  },
  M_desc: {
    key: "M_desc",
    label: "M ∨",
    compare: (left, right, leftMeta, rightMeta) =>
      compareNumberThenName(leftMeta?.M, rightMeta?.M, left, right, -1),
  },
  T_asc: {
    key: "T_asc",
    label: "T ∧",
    compare: (left, right, leftMeta, rightMeta) =>
      compareNumberThenName(leftMeta?.T, rightMeta?.T, left, right, 1),
  },
  T_desc: {
    key: "T_desc",
    label: "T ∨",
    compare: (left, right, leftMeta, rightMeta) =>
      compareNumberThenName(leftMeta?.T, rightMeta?.T, left, right, -1),
  },
  r_asc: {
    key: "r_asc",
    label: "r ∧",
    compare: (left, right, leftMeta, rightMeta) =>
      compareNumberThenName(leftMeta?.r, rightMeta?.r, left, right, 1),
  },
  r_desc: {
    key: "r_desc",
    label: "r ∨",
    compare: (left, right, leftMeta, rightMeta) =>
      compareNumberThenName(leftMeta?.r, rightMeta?.r, left, right, -1),
  },
};

function compareNumberThenName(leftValue, rightValue, leftName, rightName, direction) {
  const leftNumber = Number.isFinite(leftValue) ? leftValue : Infinity;
  const rightNumber = Number.isFinite(rightValue) ? rightValue : Infinity;
  if (leftNumber !== rightNumber) {
    return (leftNumber - rightNumber) * direction;
  }
  return leftName.localeCompare(rightName, "ja");
}

export function mergeCaseSortOptions(apiOptions = []) {
  const merged = new Map();
  for (const option of apiOptions) {
    if (!option || typeof option.key !== "string" || typeof option.label !== "string") {
      continue;
    }
    if (SORTERS[option.key]) {
      merged.set(option.key, { key: option.key, label: option.label });
    }
  }
  if (merged.size === 0) {
    merged.set(SORTERS.case_name_asc.key, {
      key: SORTERS.case_name_asc.key,
      label: SORTERS.case_name_asc.label,
    });
  }
  return Array.from(merged.values());
}

export function sortCaseNames(caseNames, sortKey, caseMetaByName = {}) {
  const sorter = SORTERS[sortKey] ?? SORTERS.case_name_asc;
  return [...caseNames].sort((left, right) =>
    sorter.compare(left, right, caseMetaByName[left] ?? null, caseMetaByName[right] ?? null),
  );
}
