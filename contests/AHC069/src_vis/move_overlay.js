const DEFAULT_EMPTY_CELL = 0xffff;

export function decodeMoveSourceGrid(encoded, N, emptyCell = DEFAULT_EMPTY_CELL) {
  if (!Number.isInteger(N) || N <= 0) {
    throw new RangeError("N must be a positive integer");
  }
  if (encoded.length === 0) {
    return null;
  }

  const cellCount = N * N;
  const grid = new Uint16Array(cellCount);
  grid.fill(emptyCell);
  let offset = 0;
  while (offset < encoded.length) {
    if (offset + 2 > encoded.length) {
      throw new RangeError("move source record header is truncated");
    }
    const groupId = encoded[offset];
    const count = encoded[offset + 1];
    offset += 2;
    if (count === 0 || offset + count > encoded.length) {
      throw new RangeError(`move source record for group ${groupId} has an invalid cell count`);
    }
    for (let i = 0; i < count; i += 1) {
      const cellIndex = encoded[offset + i];
      if (cellIndex >= cellCount) {
        throw new RangeError(`move source cell ${cellIndex} is outside the ${N} x ${N} board`);
      }
      if (grid[cellIndex] !== emptyCell) {
        throw new Error(`move source cell ${cellIndex} is assigned more than once`);
      }
      grid[cellIndex] = groupId;
    }
    offset += count;
  }
  return grid;
}

export function buildMoveArrows(sourceLabels, destinationLabels, cellSize) {
  if (!Number.isFinite(cellSize) || cellSize <= 0) {
    throw new RangeError("cellSize must be positive");
  }
  const destinationByGroup = new Map(
    destinationLabels.map((label) => [label.groupId, label]),
  );
  const arrows = [];
  for (const source of sourceLabels) {
    const destination = destinationByGroup.get(source.groupId);
    if (destination === undefined) {
      throw new Error(`move destination for group ${source.groupId} is missing`);
    }

    const sourceX = (source.col + 0.5) * cellSize;
    const sourceY = (source.row + 0.5) * cellSize;
    const destinationX = (destination.col + 0.5) * cellSize;
    const destinationY = (destination.row + 0.5) * cellSize;
    const dx = destinationX - sourceX;
    const dy = destinationY - sourceY;
    const distance = Math.hypot(dx, dy);
    if (distance <= Number.EPSILON) {
      continue;
    }

    const ux = dx / distance;
    const uy = dy / distance;
    const endpointPadding = Math.min(0.6 * cellSize, distance / 4);
    arrows.push({
      groupId: source.groupId,
      startX: sourceX + ux * endpointPadding,
      startY: sourceY + uy * endpointPadding,
      endX: destinationX - ux * endpointPadding,
      endY: destinationY - uy * endpointPadding,
    });
  }
  return arrows;
}
