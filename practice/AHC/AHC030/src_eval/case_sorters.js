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
  n_asc: {
    key: "n_asc",
    label: "N ∧",
    compare: (left, right, leftMeta, rightMeta) =>
      compareNumberThenName(leftMeta?.n, rightMeta?.n, left, right, 1),
  },
  n_desc: {
    key: "n_desc",
    label: "N ∨",
    compare: (left, right, leftMeta, rightMeta) =>
      compareNumberThenName(leftMeta?.n, rightMeta?.n, left, right, -1),
  },
  m_asc: {
    key: "m_asc",
    label: "M ∧",
    compare: (left, right, leftMeta, rightMeta) =>
      compareNumberThenName(leftMeta?.m, rightMeta?.m, left, right, 1),
  },
  m_desc: {
    key: "m_desc",
    label: "M ∨",
    compare: (left, right, leftMeta, rightMeta) =>
      compareNumberThenName(leftMeta?.m, rightMeta?.m, left, right, -1),
  },
  eps_asc: {
    key: "eps_asc",
    label: "eps ∧",
    compare: (left, right, leftMeta, rightMeta) =>
      compareNumberThenName(leftMeta?.eps, rightMeta?.eps, left, right, 1),
  },
  eps_desc: {
    key: "eps_desc",
    label: "eps ∨",
    compare: (left, right, leftMeta, rightMeta) =>
      compareNumberThenName(leftMeta?.eps, rightMeta?.eps, left, right, -1),
  },
  oil_cells_asc: {
    key: "oil_cells_asc",
    label: "oil cells ∧",
    compare: (left, right, leftMeta, rightMeta) =>
      compareNumberThenName(leftMeta?.oilCells, rightMeta?.oilCells, left, right, 1),
  },
  oil_cells_desc: {
    key: "oil_cells_desc",
    label: "oil cells ∨",
    compare: (left, right, leftMeta, rightMeta) =>
      compareNumberThenName(leftMeta?.oilCells, rightMeta?.oilCells, left, right, -1),
  },
  total_area_asc: {
    key: "total_area_asc",
    label: "total area ∧",
    compare: (left, right, leftMeta, rightMeta) =>
      compareNumberThenName(leftMeta?.totalArea, rightMeta?.totalArea, left, right, 1),
  },
  total_area_desc: {
    key: "total_area_desc",
    label: "total area ∨",
    compare: (left, right, leftMeta, rightMeta) =>
      compareNumberThenName(leftMeta?.totalArea, rightMeta?.totalArea, left, right, -1),
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
