import init, {
  get_frame as wasmGetFrame,
  prepare_case as wasmPrepareCase,
} from "./wasm/heuristic_contest_template_vis.js";

let sourceRevision = -1;

const initialized = init().then(
  () => self.postMessage({ type: "ready" }),
  (error) => {
    self.postMessage({ type: "fatal", error: String(error) });
    throw error;
  },
);

self.addEventListener("message", (event) => {
  void handleMessage(event.data);
});

async function handleMessage(message) {
  try {
    await initialized;
  } catch {
    return;
  }

  if (message?.type === "prepare") {
    prepare(message);
  } else if (message?.type === "frame") {
    sendFrame(message);
  }
}

function prepare(message) {
  let ret = null;
  try {
    ret = wasmPrepareCase(message.input, message.output);
    sourceRevision = message.revision;
    const grass = ret.grass;
    self.postMessage(
      {
        type: "prepared",
        requestId: message.requestId,
        revision: message.revision,
        N: ret.N,
        maxTurn: ret.max_turn,
        defaultTurn: ret.default_turn,
        score: ret.score,
        error: ret.error || "",
        grassCells: ret.grass_cells,
        maxTimeLeft: ret.max_time_left,
        grass,
        hasOutput: Boolean(message.output.trim()),
      },
      [grass.buffer],
    );
  } catch (error) {
    self.postMessage({
      type: "prepare-error",
      requestId: message.requestId,
      revision: message.revision,
      error: String(error),
    });
  } finally {
    ret?.free();
  }
}

function sendFrame(message) {
  if (message.revision !== sourceRevision) {
    self.postMessage({
      type: "frame-error",
      requestId: message.requestId,
      revision: message.revision,
      error: "visualizer worker source is out of date",
    });
    return;
  }

  let ret = null;
  try {
    ret = wasmGetFrame(message.turn);
    const grid = ret.grid;
    const moved = ret.moved;
    const moveSources = ret.move_sources;
    const departed = ret.departed;
    const actives = ret.actives;
    self.postMessage(
      {
        type: "frame",
        requestId: message.requestId,
        revision: message.revision,
        turn: ret.turn,
        money: ret.money,
        now: ret.now,
        arrivalId: ret.arrival_id,
        arrivalAccepted: ret.arrival_accepted,
        arrivalS: ret.arrival_s,
        arrivalT: ret.arrival_t,
        arrivalP: ret.arrival_p,
        arrivalV: ret.arrival_v,
        cellsUsed: ret.cells_used,
        accepted: ret.accepted,
        rejected: ret.rejected,
        totalFee: ret.total_fee,
        totalMoveCost: ret.total_move_cost,
        comment: ret.comment || "",
        grid,
        moved,
        moveSources,
        departed,
        actives,
      },
      [grid.buffer, moved.buffer, moveSources.buffer, departed.buffer, actives.buffer],
    );
  } catch (error) {
    self.postMessage({
      type: "frame-error",
      requestId: message.requestId,
      revision: message.revision,
      error: String(error),
    });
  } finally {
    ret?.free();
  }
}
