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
  zero_dist_asc: {
    key: "zero_dist_asc",
    label: "dist(0,exit) ∧",
    compare: (left, right, leftMeta, rightMeta) =>
      compareNumberThenName(leftMeta?.zeroDist, rightMeta?.zeroDist, left, right, 1),
  },
  zero_dist_desc: {
    key: "zero_dist_desc",
    label: "dist(0,exit) ∨",
    compare: (left, right, leftMeta, rightMeta) =>
      compareNumberThenName(leftMeta?.zeroDist, rightMeta?.zeroDist, left, right, -1),
  },
  exit_value_asc: {
    key: "exit_value_asc",
    label: "exit_value ∧",
    compare: (left, right, leftMeta, rightMeta) =>
      compareNumberThenName(leftMeta?.exitValue, rightMeta?.exitValue, left, right, 1),
  },
  exit_value_desc: {
    key: "exit_value_desc",
    label: "exit_value ∨",
    compare: (left, right, leftMeta, rightMeta) =>
      compareNumberThenName(leftMeta?.exitValue, rightMeta?.exitValue, left, right, -1),
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
