// 既存出力を公式エンジンで再生し、手動プレイで得た観察と照合する。
// 解答の生成、操作列の変更、solver の実行は行わない。
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { loadEngine } from '../../server/play-store.mjs';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
const directory = path.join(root, 'results/out/v028_two_order_lns');
const names = fs.readdirSync(directory).filter(name => /^\d{4}\.txt$/.test(name)).sort();
const { Game } = await loadEngine(root);
const consecutive = [], separated = [], retainedNestEpisodes = [];
let totalMoves = 0;

for (const name of names) {
  const game = new Game(fs.readFileSync(path.join(root, 'tools/in', name), 'utf8'));
  const lastAt = new Map(), activeNest = new Map();
  let previous = null, turn = 0;
  try {
    for (const line of fs.readFileSync(path.join(directory, name), 'utf8').trim().split('\n')) {
      const [i, j, k, d, l] = line.trim().split(/\s+/);
      const action = { i: +i, j: +j, k: +k, d, l: +l };
      const result = JSON.parse(game.apply(JSON.stringify(action)));
      ++turn; ++totalMoves;
      const cells = result.changed_cells.map(cell => `${cell.i},${cell.j}`).sort();
      const key = cells.join('/');
      const clean = result.returned.source.count + result.returned.destination.count === 0;
      const current = { turn, key, action, clean, moved: result.moved };
      const describePair = first => ({
        case: name, first: first.turn, last: turn, distance: action.l,
        actions: [first.action, action], moved: [first.moved, result.moved],
      });
      if (clean && previous?.clean && previous.key === key) {
        consecutive.push(describePair(previous));
      }
      const left = lastAt.get(cells[0]), right = lastAt.get(cells[1]);
      // 両端の直近の操作が同じなら、間の操作はどちらの端点にも触れていない。
      if (clean && left && left === right && left.clean && left.turn < turn - 1) {
        separated.push(describePair(left));
      }
      for (const cell of cells) lastAt.set(cell, current);
      previous = current;

      for (const cell of result.changed_cells) {
        if (cell.nest === null) continue;
        const position = `${cell.i},${cell.j}`;
        const before = cell.before.filter(color => color === cell.nest).length;
        const after = cell.after.filter(color => color === cell.nest).length;
        if (!activeNest.has(position) && after > 0) {
          activeNest.set(position, { case: name, start: turn, position,
            color: cell.nest, uses: [], maxOwnColorCount: after });
        }
        const episode = activeNest.get(position);
        if (!episode) continue;
        episode.maxOwnColorCount = Math.max(episode.maxOwnColorCount, after);
        if (cell.i === action.i && cell.j === action.j && before > 0 && after === before) {
          episode.uses.push({ turn, distance: action.l, moved: result.moved });
        }
        if (after === 0) {
          if (episode.uses.length >= 2) retainedNestEpisodes.push({ ...episode, end: turn });
          activeNest.delete(position);
        }
      }
    }
    if (JSON.parse(game.snapshot()).E !== 0) throw new Error(`${name}: 未完了の保存出力`);
  } finally {
    game.free();
  }
}

console.log(JSON.stringify({
  source: 'results/out/v028_two_order_lns', cases: names.length, totalMoves,
  consecutive, separated, retainedNestEpisodes,
}, null, 2));
