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
};

for (const [field, label] of [["N", "盤面サイズ N"], ["K", "色数 K"], ["M", "スライム数 M"], ["wall_count", "壁数"]]) {
  for (const [suffix, direction, arrow] of [["asc", 1, "∧"], ["desc", -1, "∨"]]) {
    const key = `${field}_${suffix}`;
    SORTERS[key] = {
      key,
      label: `${label} ${arrow}`,
      compare: (left, right, leftMeta, rightMeta) =>
        compareNumberThenName(leftMeta?.[field], rightMeta?.[field], left, right, direction),
    };
  }
}

function compareNumberThenName(leftValue, rightValue, leftName, rightName, direction) {
  // 入力ファイルが見つからないケースは、昇順・降順とも末尾へ置く。
  const leftValid = Number.isFinite(leftValue);
  const rightValid = Number.isFinite(rightValue);
  if (leftValid !== rightValid) return leftValid ? -1 : 1;
  if (!leftValid) return leftName.localeCompare(rightName, "ja");
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
