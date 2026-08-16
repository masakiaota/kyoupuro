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
  r_asc: {
    key: "r_asc",
    label: "R ∧",
    compare: (left, right, leftMeta, rightMeta) =>
      compareNumberThenName(leftMeta?.R, rightMeta?.R, left, right, 1),
  },
  r_desc: {
    key: "r_desc",
    label: "R ∨",
    compare: (left, right, leftMeta, rightMeta) =>
      compareNumberThenName(leftMeta?.R, rightMeta?.R, left, right, -1),
  },
  pond_count_asc: {
    key: "pond_count_asc",
    label: "ponds ∧",
    compare: (left, right, leftMeta, rightMeta) =>
      compareNumberThenName(leftMeta?.pondCount, rightMeta?.pondCount, left, right, 1),
  },
  pond_count_desc: {
    key: "pond_count_desc",
    label: "ponds ∨",
    compare: (left, right, leftMeta, rightMeta) =>
      compareNumberThenName(leftMeta?.pondCount, rightMeta?.pondCount, left, right, -1),
  },
  peak_people_asc: {
    key: "peak_people_asc",
    label: "peak P ∧",
    compare: (left, right, leftMeta, rightMeta) =>
      compareNumberThenName(leftMeta?.peakPeople, rightMeta?.peakPeople, left, right, 1),
  },
  peak_people_desc: {
    key: "peak_people_desc",
    label: "peak P ∨",
    compare: (left, right, leftMeta, rightMeta) =>
      compareNumberThenName(leftMeta?.peakPeople, rightMeta?.peakPeople, left, right, -1),
  },
};

function compareNumberThenName(leftValue, rightValue, leftName, rightName, direction) {
  const leftFinite = Number.isFinite(leftValue);
  const rightFinite = Number.isFinite(rightValue);
  if (leftFinite !== rightFinite) {
    return leftFinite ? -1 : 1;
  }
  if (!leftFinite) {
    return leftName.localeCompare(rightName, "ja");
  }
  const leftNumber = leftValue;
  const rightNumber = rightValue;
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
