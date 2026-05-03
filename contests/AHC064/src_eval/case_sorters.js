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
  initial_correct_position_desc: {
    key: "initial_correct_position_desc",
    label: "初期正位置 ∨",
    compare: (left, right, leftMeta, rightMeta) =>
      compareNumberThenName(
        Number(leftMeta?.initialCorrectPosition),
        Number(rightMeta?.initialCorrectPosition),
        left,
        right,
        -1,
      ),
  },
  initial_correct_position_asc: {
    key: "initial_correct_position_asc",
    label: "初期正位置 ∧",
    compare: (left, right, leftMeta, rightMeta) =>
      compareNumberThenName(
        Number(leftMeta?.initialCorrectPosition),
        Number(rightMeta?.initialCorrectPosition),
        left,
        right,
        1,
      ),
  },
  initial_correct_line_desc: {
    key: "initial_correct_line_desc",
    label: "初期正線 ∨",
    compare: (left, right, leftMeta, rightMeta) =>
      compareNumberThenName(
        Number(leftMeta?.initialCorrectLine),
        Number(rightMeta?.initialCorrectLine),
        left,
        right,
        -1,
      ),
  },
  initial_correct_line_asc: {
    key: "initial_correct_line_asc",
    label: "初期正線 ∧",
    compare: (left, right, leftMeta, rightMeta) =>
      compareNumberThenName(
        Number(leftMeta?.initialCorrectLine),
        Number(rightMeta?.initialCorrectLine),
        left,
        right,
        1,
      ),
  },
  inversions_desc: {
    key: "inversions_desc",
    label: "転倒数 ∨",
    compare: (left, right, leftMeta, rightMeta) =>
      compareNumberThenName(
        Number(leftMeta?.inversions),
        Number(rightMeta?.inversions),
        left,
        right,
        -1,
      ),
  },
  inversions_asc: {
    key: "inversions_asc",
    label: "転倒数 ∧",
    compare: (left, right, leftMeta, rightMeta) =>
      compareNumberThenName(
        Number(leftMeta?.inversions),
        Number(rightMeta?.inversions),
        left,
        right,
        1,
      ),
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
