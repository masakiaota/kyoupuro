const DEFAULT_EMPTY_CELL = 0xffff;

function isBoundaryCell(grid, N, index, gid) {
  const row = Math.floor(index / N);
  const col = index % N;
  return (
    row === 0 ||
    grid[index - N] !== gid ||
    row + 1 === N ||
    grid[index + N] !== gid ||
    col === 0 ||
    grid[index - 1] !== gid ||
    col + 1 === N ||
    grid[index + 1] !== gid
  );
}

/**
 * Finds one deterministic label position inside every active region.
 *
 * The boundary-distance BFS chooses a cell deep inside the region. Ties are resolved by distance
 * to the region centroid and then by row-major order, so irregular regions never place a label
 * outside their occupied cells.
 */
export function computeRegionLabels(grid, N, activeMap, now, emptyCell = DEFAULT_EMPTY_CELL) {
  if (!Number.isInteger(N) || N <= 0 || grid.length !== N * N) {
    throw new RangeError("grid must contain exactly N * N cells");
  }

  const cellCount = grid.length;
  const distances = new Int16Array(cellCount);
  distances.fill(-1);
  const queue = new Int32Array(cellCount);
  let queueHead = 0;
  let queueTail = 0;
  const statsByGroup = new Map();

  for (let index = 0; index < cellCount; index += 1) {
    const gid = grid[index];
    if (gid === emptyCell) {
      continue;
    }
    const row = Math.floor(index / N);
    const col = index % N;
    let stats = statsByGroup.get(gid);
    if (stats === undefined) {
      stats = {
        count: 0,
        rowSum: 0,
        colSum: 0,
        minRow: row,
        maxRow: row,
        minCol: col,
        maxCol: col,
        bestIndex: -1,
        bestBoundaryDistance: -1,
        bestCentroidDistance: Number.POSITIVE_INFINITY,
      };
      statsByGroup.set(gid, stats);
    }
    stats.count += 1;
    stats.rowSum += row;
    stats.colSum += col;
    stats.minRow = Math.min(stats.minRow, row);
    stats.maxRow = Math.max(stats.maxRow, row);
    stats.minCol = Math.min(stats.minCol, col);
    stats.maxCol = Math.max(stats.maxCol, col);

    if (isBoundaryCell(grid, N, index, gid)) {
      distances[index] = 0;
      queue[queueTail] = index;
      queueTail += 1;
    }
  }

  // All regions share one queue, while propagation remains restricted to cells of the same group.
  // This keeps the work linear in the 50 x 50 board instead of running one board scan per group.
  while (queueHead < queueTail) {
    const index = queue[queueHead];
    queueHead += 1;
    const gid = grid[index];
    const row = Math.floor(index / N);
    const col = index % N;
    const nextDistance = distances[index] + 1;

    if (row > 0) {
      const next = index - N;
      if (grid[next] === gid && distances[next] === -1) {
        distances[next] = nextDistance;
        queue[queueTail] = next;
        queueTail += 1;
      }
    }
    if (row + 1 < N) {
      const next = index + N;
      if (grid[next] === gid && distances[next] === -1) {
        distances[next] = nextDistance;
        queue[queueTail] = next;
        queueTail += 1;
      }
    }
    if (col > 0) {
      const next = index - 1;
      if (grid[next] === gid && distances[next] === -1) {
        distances[next] = nextDistance;
        queue[queueTail] = next;
        queueTail += 1;
      }
    }
    if (col + 1 < N) {
      const next = index + 1;
      if (grid[next] === gid && distances[next] === -1) {
        distances[next] = nextDistance;
        queue[queueTail] = next;
        queueTail += 1;
      }
    }
  }

  for (let index = 0; index < cellCount; index += 1) {
    const gid = grid[index];
    if (gid === emptyCell) {
      continue;
    }
    const stats = statsByGroup.get(gid);
    const row = Math.floor(index / N);
    const col = index % N;
    // Comparing the centroid distance after multiplying by count avoids floating-point tie drift.
    const rowOffset = row * stats.count - stats.rowSum;
    const colOffset = col * stats.count - stats.colSum;
    const centroidDistance = rowOffset * rowOffset + colOffset * colOffset;
    const boundaryDistance = distances[index];
    const isBetter =
      boundaryDistance > stats.bestBoundaryDistance ||
      (boundaryDistance === stats.bestBoundaryDistance &&
        (centroidDistance < stats.bestCentroidDistance ||
          (centroidDistance === stats.bestCentroidDistance &&
            (stats.bestIndex === -1 || index < stats.bestIndex))));
    if (isBetter) {
      stats.bestIndex = index;
      stats.bestBoundaryDistance = boundaryDistance;
      stats.bestCentroidDistance = centroidDistance;
    }
  }

  const labels = [];
  for (const [groupId, stats] of statsByGroup) {
    const active = activeMap.get(groupId);
    if (active === undefined) {
      continue;
    }
    labels.push({
      groupId,
      row: Math.floor(stats.bestIndex / N),
      col: stats.bestIndex % N,
      remaining: active.t - now,
      bounds: {
        minRow: stats.minRow,
        maxRow: stats.maxRow,
        minCol: stats.minCol,
        maxCol: stats.maxCol,
      },
    });
  }
  labels.sort((left, right) => left.groupId - right.groupId);
  return labels;
}
