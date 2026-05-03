function createCell(tagName, text, className = "") {
  const element = document.createElement(tagName);
  if (className) {
    element.className = className;
  }
  element.textContent = text;
  return element;
}

function sortMark(rowSort, columnKey) {
  if (rowSort?.key !== columnKey) {
    return "<";
  }
  return rowSort.dir === "asc" ? "∧" : "∨";
}

function createSortableHeader(label, columnKey, rowSort, onSort, className) {
  const th = createCell("th", label, className);
  const button = document.createElement("button");
  button.type = "button";
  button.className = "sort-btn";
  button.textContent = sortMark(rowSort, columnKey);
  button.title = `${label} でソート`;
  button.addEventListener("click", () => onSort(columnKey));
  th.appendChild(button);
  return th;
}

function formatCaseHeader(caseName) {
  return typeof caseName === "string" ? caseName.replace(/\.txt$/i, "") : "";
}

function formatMs(value) {
  return Number.isFinite(Number(value)) ? `${Number(value)}ms` : "";
}

function formatScore(value, unit = "raw") {
  if (value == null) {
    return "";
  }
  const number = Number(value);
  if (!Number.isFinite(number)) {
    return "";
  }
  if (unit === "k") {
    return `${Math.round(number / 1_000).toLocaleString("en-US")}k`;
  }
  if (unit === "m") {
    return `${Math.round(number / 1_000_000).toLocaleString("en-US")}M`;
  }
  if (unit === "cap100k") {
    return number > 100_000 ? "∞" : number.toLocaleString("en-US");
  }
  return number.toLocaleString("en-US");
}

function sameOrder(left, right) {
  return left.length === right.length && left.every((value, index) => value === right[index]);
}

function startRowDrag(event, tbody, row, handle, onManualOrder) {
  if (event.pointerType === "mouse" && event.button !== 0) {
    return;
  }

  const rows = Array.from(tbody.querySelectorAll("tr"));
  const ids = rows.map((item) => item.dataset.runId);
  const draggedRunId = row.dataset.runId;
  const from = ids.indexOf(draggedRunId);
  if (from < 0) {
    return;
  }

  event.preventDefault();
  handle.setPointerCapture(event.pointerId);

  const rectById = new Map();
  for (const item of rows) {
    rectById.set(item.dataset.runId, item.getBoundingClientRect());
  }
  const draggedRect = rectById.get(draggedRunId);
  const rowHeight = draggedRect?.height || row.getBoundingClientRect().height || 1;
  const remainingIds = ids.filter((id) => id !== draggedRunId);

  let insertIndex = from;
  let latestClientY = event.clientY;
  let frameId = 0;

  function computeInsertIndex(clientY) {
    for (let index = 0; index < remainingIds.length; index += 1) {
      const rect = rectById.get(remainingIds[index]);
      if (rect && clientY < rect.top + rect.height / 2) {
        return index;
      }
    }
    return remainingIds.length;
  }

  function orderedIdsForInsert() {
    const nextIds = [...remainingIds];
    nextIds.splice(insertIndex, 0, draggedRunId);
    return nextIds;
  }

  function applyTransforms() {
    frameId = 0;
    insertIndex = computeInsertIndex(latestClientY);
    const nextIds = orderedIdsForInsert();
    const nextIndexById = new Map(nextIds.map((id, index) => [id, index]));

    for (const item of rows) {
      const id = item.dataset.runId;
      if (id === draggedRunId) {
        const deltaY = latestClientY - event.clientY;
        item.style.transform = `translateY(${deltaY}px)`;
        continue;
      }
      const originalIndex = ids.indexOf(id);
      const nextIndex = nextIndexById.get(id);
      const deltaY = (nextIndex - originalIndex) * rowHeight;
      item.style.transform = deltaY === 0 ? "" : `translateY(${deltaY}px)`;
    }
  }

  function scheduleTransform(clientY) {
    latestClientY = clientY;
    if (!frameId) {
      frameId = requestAnimationFrame(applyTransforms);
    }
  }

  function cleanup() {
    if (frameId) {
      cancelAnimationFrame(frameId);
      frameId = 0;
    }
    document.body.classList.remove("is-row-dragging");
    for (const item of rows) {
      item.classList.remove("is-dragging", "is-drag-animating");
      item.style.transform = "";
    }
    handle.removeEventListener("pointermove", onPointerMove);
    handle.removeEventListener("pointerup", onPointerUp);
    handle.removeEventListener("pointercancel", onPointerCancel);
    document.removeEventListener("keydown", onKeyDown);
    if (handle.hasPointerCapture(event.pointerId)) {
      handle.releasePointerCapture(event.pointerId);
    }
  }

  function finish(commit) {
    insertIndex = computeInsertIndex(latestClientY);
    const nextIds = orderedIdsForInsert();
    cleanup();
    if (commit && !sameOrder(ids, nextIds)) {
      onManualOrder(nextIds);
    }
  }

  function onPointerMove(moveEvent) {
    moveEvent.preventDefault();
    scheduleTransform(moveEvent.clientY);
  }

  function onPointerUp(upEvent) {
    upEvent.preventDefault();
    finish(true);
  }

  function onPointerCancel(cancelEvent) {
    cancelEvent.preventDefault();
    finish(false);
  }

  function onKeyDown(keyEvent) {
    if (keyEvent.key === "Escape") {
      keyEvent.preventDefault();
      finish(false);
    }
  }

  document.body.classList.add("is-row-dragging");
  row.classList.add("is-dragging");
  for (const item of rows) {
    if (item !== row) {
      item.classList.add("is-drag-animating");
    }
  }

  handle.addEventListener("pointermove", onPointerMove);
  handle.addEventListener("pointerup", onPointerUp);
  handle.addEventListener("pointercancel", onPointerCancel);
  document.addEventListener("keydown", onKeyDown);
  scheduleTransform(event.clientY);
}

export function renderResultTable(table, payload) {
  const { runs, caseNames, scoreUnit = "raw", rowSort, onSort, onManualOrder } = payload;
  table.textContent = "";

  const thead = document.createElement("thead");
  const headRow = document.createElement("tr");
  headRow.appendChild(createCell("th", "", "sticky sticky-handle"));
  headRow.appendChild(createSortableHeader("bin", "bin", rowSort, onSort, "sticky sticky-bin"));
  headRow.appendChild(
    createSortableHeader("total_avg", "totalAvg", rowSort, onSort, "sticky sticky-avg align-right"),
  );
  headRow.appendChild(createCell("th", "max_elapsed", "sticky sticky-elapsed align-right"));
  for (const caseName of caseNames) {
    headRow.appendChild(createCell("th", formatCaseHeader(caseName), "align-right"));
  }
  headRow.appendChild(createCell("th", "label"));
  headRow.appendChild(createSortableHeader("executed_at", "executedAt", rowSort, onSort, ""));
  thead.appendChild(headRow);

  const tbody = document.createElement("tbody");

  for (const run of runs) {
    const row = document.createElement("tr");
    row.dataset.runId = run.id;

    const handleCell = createCell("td", "", "sticky sticky-handle");
    const handle = document.createElement("button");
    handle.type = "button";
    handle.className = "drag-handle";
    handle.textContent = "≡";
    handle.title = "行をドラッグして並び替え";
    handleCell.appendChild(handle);
    row.appendChild(handleCell);

    row.appendChild(createCell("td", run.bin, "sticky sticky-bin mono"));
    row.appendChild(
      createCell("td", formatScore(run.totalAvg), "sticky sticky-avg align-right mono"),
    );
    row.appendChild(createCell("td", formatMs(run.maxElapsed), "sticky sticky-elapsed align-right mono"));
    for (const caseName of caseNames) {
      const score = run.caseScores?.[caseName];
      row.appendChild(createCell("td", formatScore(score, scoreUnit), "align-right mono"));
    }
    row.appendChild(createCell("td", run.label || "", "muted"));
    row.appendChild(createCell("td", run.executedAt, "mono muted"));

    handle.addEventListener("pointerdown", (event) => {
      startRowDrag(event, tbody, row, handle, onManualOrder);
    });

    tbody.appendChild(row);
  }

  table.appendChild(thead);
  table.appendChild(tbody);
}
