import test from "node:test";
import assert from "node:assert/strict";

import { computeRegionLabels } from "./region_labels.js";

const EMPTY_CELL = 0xffff;

function gridFromRows(rows) {
  const N = rows.length;
  assert.ok(rows.every((row) => row.length === N));
  return Uint16Array.from(rows.flat());
}

function activeMap(entries) {
  return new Map(entries.map(([id, t]) => [id, { id, t }]));
}

test("a square region uses its deepest central cell", () => {
  const grid = gridFromRows([
    [EMPTY_CELL, EMPTY_CELL, EMPTY_CELL, EMPTY_CELL, EMPTY_CELL],
    [EMPTY_CELL, 1, 1, 1, EMPTY_CELL],
    [EMPTY_CELL, 1, 1, 1, EMPTY_CELL],
    [EMPTY_CELL, 1, 1, 1, EMPTY_CELL],
    [EMPTY_CELL, EMPTY_CELL, EMPTY_CELL, EMPTY_CELL, EMPTY_CELL],
  ]);

  assert.deepEqual(computeRegionLabels(grid, 5, activeMap([[1, 20]]), 7), [
    {
      groupId: 1,
      row: 2,
      col: 2,
      remaining: 13,
      bounds: { minRow: 1, maxRow: 3, minCol: 1, maxCol: 3 },
    },
  ]);
});

test("a thin region resolves centroid ties in row-major order", () => {
  const grid = gridFromRows([
    [EMPTY_CELL, EMPTY_CELL, EMPTY_CELL, EMPTY_CELL, EMPTY_CELL],
    [EMPTY_CELL, EMPTY_CELL, EMPTY_CELL, EMPTY_CELL, EMPTY_CELL],
    [EMPTY_CELL, 4, 4, 4, 4],
    [EMPTY_CELL, EMPTY_CELL, EMPTY_CELL, EMPTY_CELL, EMPTY_CELL],
    [EMPTY_CELL, EMPTY_CELL, EMPTY_CELL, EMPTY_CELL, EMPTY_CELL],
  ]);

  const [label] = computeRegionLabels(grid, 5, activeMap([[4, 100]]), 25);
  assert.deepEqual(label, {
    groupId: 4,
    row: 2,
    col: 2,
    remaining: 75,
    bounds: { minRow: 2, maxRow: 2, minCol: 1, maxCol: 4 },
  });
});

test("a concave region still places its label on an occupied cell", () => {
  const grid = gridFromRows([
    [7, 7, 7, 7, EMPTY_CELL],
    [7, EMPTY_CELL, EMPTY_CELL, EMPTY_CELL, EMPTY_CELL],
    [7, 7, 7, EMPTY_CELL, EMPTY_CELL],
    [EMPTY_CELL, EMPTY_CELL, EMPTY_CELL, EMPTY_CELL, EMPTY_CELL],
    [EMPTY_CELL, EMPTY_CELL, EMPTY_CELL, EMPTY_CELL, EMPTY_CELL],
  ]);

  const [label] = computeRegionLabels(grid, 5, activeMap([[7, 50]]), 10);
  assert.equal(grid[label.row * 5 + label.col], 7);
  assert.deepEqual([label.row, label.col], [0, 1]);
});

test("multiple groups are handled independently and sorted by id", () => {
  const grid = gridFromRows([
    [9, 9, EMPTY_CELL, 2, 2],
    [9, 9, EMPTY_CELL, 2, 2],
    [EMPTY_CELL, EMPTY_CELL, EMPTY_CELL, 2, 2],
    [EMPTY_CELL, EMPTY_CELL, EMPTY_CELL, EMPTY_CELL, EMPTY_CELL],
    [EMPTY_CELL, EMPTY_CELL, EMPTY_CELL, EMPTY_CELL, EMPTY_CELL],
  ]);

  const labels = computeRegionLabels(grid, 5, activeMap([[9, 80], [2, 60]]), 20);
  assert.deepEqual(
    labels.map(({ groupId, remaining }) => [groupId, remaining]),
    [
      [2, 40],
      [9, 60],
    ],
  );
  for (const label of labels) {
    assert.equal(grid[label.row * 5 + label.col], label.groupId);
  }
});

test("time changes, movement, and departure are reflected without stored frame state", () => {
  const beforeMove = gridFromRows([
    [3, 3, EMPTY_CELL, EMPTY_CELL],
    [3, 3, EMPTY_CELL, EMPTY_CELL],
    [EMPTY_CELL, EMPTY_CELL, EMPTY_CELL, EMPTY_CELL],
    [EMPTY_CELL, EMPTY_CELL, EMPTY_CELL, EMPTY_CELL],
  ]);
  const afterMove = gridFromRows([
    [EMPTY_CELL, EMPTY_CELL, EMPTY_CELL, EMPTY_CELL],
    [EMPTY_CELL, EMPTY_CELL, EMPTY_CELL, EMPTY_CELL],
    [EMPTY_CELL, EMPTY_CELL, 3, 3],
    [EMPTY_CELL, EMPTY_CELL, 3, 3],
  ]);
  const active = activeMap([[3, 100]]);

  assert.equal(computeRegionLabels(beforeMove, 4, active, 10)[0].remaining, 90);
  assert.equal(computeRegionLabels(beforeMove, 4, active, 40)[0].remaining, 60);
  assert.deepEqual(
    [
      computeRegionLabels(beforeMove, 4, active, 40)[0].row,
      computeRegionLabels(beforeMove, 4, active, 40)[0].col,
    ],
    [0, 0],
  );
  assert.deepEqual(
    [
      computeRegionLabels(afterMove, 4, active, 40)[0].row,
      computeRegionLabels(afterMove, 4, active, 40)[0].col,
    ],
    [2, 2],
  );
  assert.deepEqual(
    computeRegionLabels(new Uint16Array(16).fill(EMPTY_CELL), 4, new Map(), 100),
    [],
  );
});
