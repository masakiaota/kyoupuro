import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import http from "node:http";
import { execFile } from "node:child_process";
import { promisify } from "node:util";
import { fileURLToPath } from "node:url";
import { PlayStore, loadEngine } from "../server/play-store.mjs";
import { playMiddleware } from "../server/play-api.mjs";

const root = fileURLToPath(new URL("..", import.meta.url));
const engine = await loadEngine(root);
const { vis } = await import(`../src_vis/wasm/ahc072_vis.js?play_engine=${engine.engineHash}`);
const exec = promisify(execFile);

// 固定の手作業の操作列。解法プログラムを実行せずに両端の帰巣を再現する。
function fixture() {
  const grid = Array.from({ length: 12 }, () => Array(12).fill("."));
  for (const [i, j, c] of [[0, 0, "A"], [0, 3, "B"], [11, 10, "C"], [11, 11, "D"],
    [1, 0, "b"], [1, 1, "a"], [2, 0, "a"], [10, 10, "c"], [10, 11, "d"], [3, 0, "#"]]) grid[i][j] = c;
  return `12 4\n${grid.map((row) => row.join("")).join("\n")}\n`;
}
const setupOutput = "1 1 0 L 1\n2 0 0 U 1\n1 0 0 U 1\n";
const homeAction = { i: 0, j: 0, k: 2, d: "R", l: 3 };

function makeStore(t) {
  const directory = fs.mkdtempSync(path.join(os.tmpdir(), "ahc072-play-test-"));
  const store = new PlayStore({ root, directory, ...engine });
  t.after(() => { store.close(); fs.rmSync(directory, { recursive: true, force: true }); });
  return store;
}

test("preview, legality, official score, and durable branching", (t) => {
  let store = makeStore(t);
  const created = store.create({ input: fixture(), output: setupOutput, label: "両端の帰巣" });
  const id = created.session_id, file = path.join(store.directory, id, "events.jsonl");
  assert.equal(created.T, 3);
  assert.equal(created.E, 5);
  assert.deepEqual(created.towers.find((cell) => cell.i === 0 && cell.j === 0).colors, [0, 0, 1]);
  const initialLog = fs.readFileSync(file, "utf8");
  const preview = store.preview(id, { expected_revision: 0, action: homeAction });
  assert.deepEqual(preview.returned, { source: { count: 2, colors: [0, 0] }, destination: { count: 1, colors: [1] } });
  assert.deepEqual([preview.T, preview.E, preview.S], [4, 2, 200004]);
  assert.equal(store.state(id).T, 3);
  assert.equal(fs.readFileSync(file, "utf8"), initialLog);
  const legal = store.legal(id, { i: 0, j: 0, k: 2 });
  assert(legal.actions.some((a) => JSON.stringify(a) === JSON.stringify(preview.action)));
  assert.throws(() => store.preview(id, { expected_revision: 0, action: { ...homeAction, d: "D" } }), /壁/);
  assert.throws(() => store.mutate(id, "step", { expected_revision: 0, request_id: "bad", action: { ...homeAction, l: 0 } }), /1以上/);
  assert.equal(fs.readFileSync(file, "utf8"), initialLog);
  const body = { expected_revision: 0, request_id: "home", action: homeAction, note: "両端が帰巣", actor: "human" };
  const stepped = store.mutate(id, "step", body);
  assert.equal(stepped.S, preview.S);
  assert.equal(store.state(id).received[0], 2);
  assert.deepEqual(store.mutate(id, "step", body), stepped);
  assert.throws(() => store.mutate(id, "step", { ...body, note: "変更" }), /別の内容/);
  assert.throws(() => store.mutate(id, "checkout", { expected_revision: 0, request_id: "stale", node_id: "n0" }), /更新されている/);
  const output = store.export(id, { format: "output" });
  assert.equal(output, setupOutput + "0 0 2 R 3\n");
  const official = vis(fixture(), output, 4);
  try { assert.equal(Number(official.score), stepped.S); assert.equal(official.err, ""); }
  finally { official.free(); }
  store.mutate(id, "checkout", { expected_revision: 1, request_id: "rewind", node_id: "n0" });
  const branch = store.mutate(id, "step", { expected_revision: 2, request_id: "branch", action: { i: 10, j: 10, k: 0, d: "D", l: 1 } });
  assert.equal(branch.T, 1);
  assert.equal(branch.E, 4);
  assert.equal(store.history(id).total, 6);
  assert.equal(store.state(id, { node_id: "n4" }).E, 2);
  assert.equal(store.version(id).node_id, "n5");
  assert.deepEqual(store.state(id, { node_id: "n0" }).children.map((n) => n.node_id), ["n1", "n5"]);
  const originalPath = store.history(id, { node_id: "n4" });
  assert.deepEqual(originalPath.nodes.map((n) => n.node_id), ["n0", "n1", "n2", "n3", "n4"]);
  assert.deepEqual(originalPath.branches.map((n) => [n.node_id, n.T, n.fork_T]), [["n4", 4, 0], ["n5", 1, 0]]);
  assert.deepEqual(store.history(id, { node_id: "n4", offset: 2, limit: 2 }).nodes.map((n) => n.node_id), ["n2", "n3"]);
  // 元の手順の途中を読む操作で、枝の終端も保存された現在位置も変わらない。
  assert.equal(store.state(id, { node_id: "n4", turn: 2 }).T, 2);
  assert.equal(store.history(id, { node_id: "n4" }).total, 5);
  assert.equal(store.version(id).node_id, "n5");
  store.mutate(id, "note", { expected_revision: 3, request_id: "note", node_id: "n4", text: "元の枝を保存" });
  const beforeRestart = store.state(id);
  const directory = store.directory;
  store.close();
  store = new PlayStore({ root, directory, ...engine });
  t.after(() => store.close());
  assert.deepEqual(store.state(id), beforeRestart);
  assert.equal(store.state(id, { node_id: "n4" }).notes[1].text, "元の枝を保存");
  assert.deepEqual(store.mutate(id, "step", body), stepped);
  assert.equal(store.export(id).events.length, 5);
  assert.equal(store.export(id, { format: "output", node_id: "n4" }), output);
});

test("all legal moves can be previewed without mutation; numeric validation is strict", (t) => {
  const store = makeStore(t), session = store.create({ input: fixture() }), id = session.session_id;
  for (const cell of session.towers) {
    for (const action of store.legal(id, cell).actions) {
      const result = store.preview(id, { expected_revision: 0, action });
      assert.equal(result.T, 1);
      assert.equal(result.changed_cells.length, 2);
    }
  }
  assert.deepEqual(store.state(id), session);
  for (const action of [{ ...homeAction, i: -1 }, { ...homeAction, i: 1.5 }, { ...homeAction, l: 0 }, { ...homeAction, d: "up" }, { ...homeAction, extra: 1 }]) {
    assert.throws(() => store.preview(id, { expected_revision: 0, action }));
  }
  assert.throws(() => store.legal(id, { i: NaN, j: 0 }));
  assert.throws(() => store.legal(id, { i: 4294967296, j: 0 }));
  assert.throws(() => store.state(id, { turn: 1 }));
  assert.throws(() => store.create({ input: fixture(), output: "0 0 0 U 1\n" }));
  assert.equal(store.list().sessions.length, 1);
});

test("incomplete journal is reported and preserved", (t) => {
  const store = makeStore(t), session = store.create({ input: fixture() });
  const file = path.join(store.directory, session.session_id, "events.jsonl");
  store.close();
  fs.appendFileSync(file, '{"type":');
  const corrupt = fs.readFileSync(file, "utf8");
  assert.throws(() => store.get(session.session_id), /末尾が不完全/);
  assert.equal(store.list().errors.length, 1);
  assert.equal(fs.readFileSync(file, "utf8"), corrupt);
});

test("completed game and 100000-operation boundary survive JSON and replay", (t) => {
  const store = makeStore(t);
  const completed = store.create({ input: fixture(), output: setupOutput + "0 0 2 R 3\n10 10 0 D 1\n10 11 0 D 1\n" });
  assert.deepEqual([completed.T, completed.E, completed.S, completed.completed], [6, 0, 6, true]);
  assert.equal(completed.towers.length, 0);
  const longOutput = "10 10 0 L 1\n10 9 0 R 1\n".repeat(50000);
  const maximum = store.create({ input: fixture(), output: longOutput });
  assert.equal(maximum.T, 100000);
  assert.equal(store.legal(maximum.session_id, { i: 10, j: 10 }).actions.length, 0);
  assert.throws(() => store.preview(maximum.session_id, { expected_revision: 0, action: { i: 10, j: 10, k: 0, d: "L", l: 1 } }), /100000/);
  assert.equal(store.state(maximum.session_id, { turn: 99999 }).T, 99999);
  assert.throws(() => store.create({ input: fixture(), output: longOutput + "10 10 0 L 1\n" }), /Too many operations/);
});

test("failed persistence never advances the board or accepts further writes", (t) => {
  const store = makeStore(t), state = store.create({ input: fixture(), output: setupOutput });
  const file = path.join(store.directory, state.session_id, "events.jsonl");
  fs.renameSync(file, `${file}.backup`);
  fs.mkdirSync(file);
  const body = { request_id: "save-failure", expected_revision: 0, action: homeAction };
  assert.throws(() => store.mutate(state.session_id, "step", body), /保存に失敗/);
  assert.deepEqual(store.state(state.session_id), state);
  fs.rmdirSync(file);
  fs.renameSync(`${file}.backup`, file);
  assert.throws(() => store.mutate(state.session_id, "step", body), /更新を停止/);
  store.close();
  assert.equal(store.get(state.session_id).revision, 0);
  assert.equal(store.mutate(state.session_id, "step", body).T, 4);
});

test("HTTP and actual CLI share state, revision checks, and idempotency", async (t) => {
  const store = makeStore(t), middleware = playMiddleware(store);
  const server = http.createServer((req, res) => middleware(req, res, () => { res.statusCode = 404; res.end(); }));
  await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
  t.after(() => new Promise((resolve) => server.close(resolve)));
  const base = `http://127.0.0.1:${server.address().port}`;
  const post = async (endpoint, value, headers = {}) => fetch(`${base}/api/play/${endpoint}`, {
    method: "POST", headers: { "Content-Type": "application/json", ...headers }, body: JSON.stringify(value),
  });
  const created = await post("sessions", { input: fixture(), output: setupOutput });
  assert.equal(created.status, 201);
  const session = await created.json(), id = session.session_id;
  const cli = async (...args) => JSON.parse((await exec(process.execPath, ["scripts/play.mjs", ...args, "--url", base], { cwd: root })).stdout);
  assert.equal((await cli("state", "--session", id)).T, 3);
  assert((await cli("legal", "--session", id, "--i", "0", "--j", "0", "--k", "2")).actions.length > 0);
  const step = await cli("step", "--session", id, "--revision", "0", "--request-id", "cli-home", "--action", JSON.stringify(homeAction));
  assert.equal(step.E, 2);
  const view = await (await fetch(`${base}/api/play/sessions/${id}/state?svg=1`)).json();
  assert.equal(view.T, 4);
  assert.match(view.svg, /data-cell=/);
  assert.equal(view.actor, "ai");
  assert.deepEqual(await cli("step", "--session", id, "--revision", "0", "--request-id", "cli-home", "--action", JSON.stringify(homeAction)), step);
  const invalid = await post(`sessions/${id}/step`, { expected_revision: 1, request_id: "invalid", action: { ...homeAction, i: 99 } });
  assert.equal(invalid.status, 400);
  const forbidden = await post(`sessions/${id}/note`, {}, { Origin: "https://example.com" });
  assert.equal(forbidden.status, 403);
  const concurrent = await Promise.all(["a", "b"].map((request_id) => post(`sessions/${id}/checkout`, { request_id, expected_revision: 1, node_id: "n0" })));
  assert.deepEqual(concurrent.map((r) => r.status).sort(), [200, 409]);
  const ended = await cli("state", "--session", id);
  assert.equal(ended.T, 0);
  assert.equal(ended.revision, 2);
  const { stdout } = await exec(process.execPath, ["scripts/play.mjs", "export", "--session", id, "--node", "n4", "--format", "output", "--url", base], { cwd: root });
  assert.equal(stdout, setupOutput + "0 0 2 R 3\n");
});
