import test from "node:test";
import assert from "node:assert/strict";

import { buildMoveArrows, decodeMoveSourceGrid } from "./move_overlay.js";

const EMPTY_CELL = 0xffff;

test("move source records decode multiple groups in row-major cells", () => {
  const grid = decodeMoveSourceGrid(Uint16Array.from([1, 3, 0, 1, 4, 3, 2, 3, 6]), 3);

  assert.deepEqual(Array.from(grid), [
    1,
    1,
    EMPTY_CELL,
    3,
    1,
    EMPTY_CELL,
    3,
    EMPTY_CELL,
    EMPTY_CELL,
  ]);
});

test("move source cells remain independent from current occupancy", () => {
  const sourceGrid = decodeMoveSourceGrid(Uint16Array.from([5, 2, 0, 1]), 2);
  const currentGrid = Uint16Array.from([8, 8, EMPTY_CELL, EMPTY_CELL]);

  assert.deepEqual(Array.from(sourceGrid), [5, 5, EMPTY_CELL, EMPTY_CELL]);
  assert.deepEqual(Array.from(currentGrid), [8, 8, EMPTY_CELL, EMPTY_CELL]);
});

test("simultaneous swaps pair arrows by group id", () => {
  const sourceLabels = [
    { groupId: 1, row: 0, col: 0 },
    { groupId: 2, row: 0, col: 3 },
  ];
  const destinationLabels = [
    { groupId: 1, row: 0, col: 3 },
    { groupId: 2, row: 0, col: 0 },
  ];

  const arrows = buildMoveArrows(sourceLabels, destinationLabels, 12);

  assert.deepEqual(
    arrows.map(({ groupId }) => groupId),
    [1, 2],
  );
  assert.ok(arrows[0].startX < arrows[0].endX);
  assert.ok(arrows[1].startX > arrows[1].endX);
});

test("arrow endpoints are shortened by at most 0.6 cells", () => {
  const arrows = buildMoveArrows(
    [{ groupId: 4, row: 0, col: 0 }],
    [{ groupId: 4, row: 0, col: 3 }],
    12,
  );

  assert.equal(arrows[0].startX, 13.2);
  assert.equal(arrows[0].endX, 34.8);
  assert.equal(arrows[0].startY, 6);
  assert.equal(arrows[0].endY, 6);
});

test("a move with the same source and destination center has no arrow", () => {
  const label = { groupId: 6, row: 2, col: 1 };

  assert.deepEqual(buildMoveArrows([label], [label], 12), []);
});

test("an empty move source payload avoids allocating an overlay grid", () => {
  assert.equal(decodeMoveSourceGrid(new Uint16Array(), 50), null);
});
