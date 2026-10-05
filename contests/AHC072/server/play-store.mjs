import fs from "node:fs";
import path from "node:path";
import crypto from "node:crypto";
import { pathToFileURL } from "node:url";

export class PlayError extends Error {
  constructor(code, message, status = 400, details = {}) {
    super(message);
    Object.assign(this, { code, status, details });
  }
}

export function engineCall(fn) {
  try { return fn(); }
  catch (error) {
    if (error instanceof Error) throw error;
    let detail;
    try { detail = JSON.parse(String(error)); } catch { throw new Error(String(error)); }
    throw new PlayError(detail.code, detail.message);
  }
}

export async function loadEngine(root) {
  const entry = path.join(root, "src_vis/wasm/ahc072_vis.js");
  const bytes = fs.readFileSync(path.join(root, "src_vis/wasm/ahc072_vis_bg.wasm"));
  const engineHash = hash(bytes);
  // Viteが同じプロセス内で再起動しても、新しいWASMと以前の接続コードを混在させない。
  const module = await import(`${pathToFileURL(entry).href}?play_engine=${engineHash}`);
  if (!module.PlayGame) throw new Error("プレイ用WASMが古い。./scripts/build_wasm.sh を実行する");
  module.initSync({ module: bytes });
  return { Game: module.PlayGame, engineHash };
}

const hash = (value) => crypto.createHash("sha256").update(value).digest("hex");
const actionLine = (a) => `${a.i} ${a.j} ${a.k} ${a.d} ${a.l}`;
const canonical = (value) => Array.isArray(value) ? value.map(canonical)
  : value && typeof value === "object" ? Object.fromEntries(Object.keys(value).sort().map((k) => [k, canonical(value[k])])) : value;
const fail = (code, message, status, details) => { throw new PlayError(code, message, status, details); };

function textField(value, name, max, defaultValue = "") {
  if (value === undefined) return defaultValue;
  if (typeof value !== "string" || value.length > max) fail("invalid_request", `${name} は${max}文字以内の文字列で指定する`);
  return value;
}

function durableWrite(file, text, flags = "a") {
  const fd = fs.openSync(file, flags, 0o600);
  try { fs.writeFileSync(fd, text, "utf8"); fs.fsyncSync(fd); }
  finally { fs.closeSync(fd); }
}

export class PlayStore {
  constructor({ root, directory = path.join(root, "results/play"), Game, engineHash }) {
    Object.assign(this, { root, directory, Game, engineHash });
    this.sessions = new Map();
    fs.mkdirSync(directory, { recursive: true });
  }

  // 同じ保存先を複数サーバーが書かないようにする。クラッシュ後は終了済みPIDだけ回収する。
  acquireLock() {
    const file = path.join(this.directory, ".server-lock.json");
    if (fs.existsSync(file)) {
      const owner = JSON.parse(fs.readFileSync(file, "utf8"));
      if (!Number.isSafeInteger(owner.pid) || owner.pid < 1) throw new Error(`保存先のロックが不正: ${file}`);
      let alive = true;
      try { process.kill(owner.pid, 0); }
      catch (error) { if (error.code === "ESRCH") alive = false; else throw error; }
      if (alive) throw new Error(`プレイの保存先は別のサーバー（PID ${owner.pid}）が使用中: ${this.directory}`);
      fs.unlinkSync(file);
      process.stderr.write("[play] 終了済みサーバーの保存先ロックを回収した\n");
    }
    this.lockToken = crypto.randomUUID();
    durableWrite(file, JSON.stringify({ pid: process.pid, token: this.lockToken }), "wx");
    this.lockFile = file;
  }

  close() {
    for (const session of this.sessions.values()) session.game.free();
    this.sessions.clear();
    if (this.lockFile && fs.existsSync(this.lockFile)) {
      const owner = JSON.parse(fs.readFileSync(this.lockFile, "utf8"));
      if (owner.token === this.lockToken) fs.unlinkSync(this.lockFile);
    }
    this.lockFile = null;
  }

  cases() {
    const directory = path.join(this.root, "tools/in");
    return fs.readdirSync(directory, { withFileTypes: true }).filter((e) => e.isFile() && e.name.endsWith(".txt"))
      .map((e) => e.name).sort();
  }

  newGame(input) { return engineCall(() => new this.Game(input)); }

  create(body) {
    if ((typeof body.input === "string") === (typeof body.case_name === "string")) {
      fail("invalid_request", "input または case_name のどちらか一方を指定する");
    }
    let input = body.input;
    if (body.case_name !== undefined) {
      if (!this.cases().includes(body.case_name)) fail("unknown_case", "指定された入力ケースがない", 404);
      input = fs.readFileSync(path.join(this.root, "tools/in", body.case_name), "utf8");
    }
    if (input.length > 10000) fail("invalid_input", "入力が長すぎる");
    const label = textField(body.label, "label", 160, body.case_name ?? "入力から開始");
    const output = textField(body.output, "output", 4000000);
    const game = this.newGame(input);
    try {
      const imported = engineCall(() => JSON.parse(game.replay(output)).actions);
      const id = `p_${crypto.randomUUID()}`;
      const event = { type: "create", schema_version: 1, session_id: id, revision: 0,
        input, input_sha256: hash(input), output: imported.map(actionLine).join("\n"),
        label, case_name: body.case_name ?? null, engine_hash: this.engineHash,
        actor: textField(body.actor, "actor", 80, "ai"), at: new Date().toISOString() };
      // 初期記録は一時ディレクトリで完成させ、完成後に一覧へ公開する。
      const temporary = fs.mkdtempSync(path.join(this.directory, ".creating-"));
      try {
        durableWrite(path.join(temporary, "events.jsonl"), `${JSON.stringify(event)}\n`, "wx");
        fs.renameSync(temporary, path.join(this.directory, id));
      } catch (error) { fs.rmSync(temporary, { recursive: true, force: true }); throw error; }
      const session = this.fromEvents([event], game, imported);
      this.sessions.set(id, session);
      return this.state(id);
    } catch (error) { game.free(); throw error; }
  }

  fromEvents(events, game, imported) {
    const first = events[0];
    if (first?.type !== "create" || first.schema_version !== 1 || hash(first.input) !== first.input_sha256) {
      throw new Error("プレイ記録の初期データまたは形式が不正である");
    }
    const nodes = new Map();
    const addNode = (node) => {
      if (nodes.has(node.node_id) || (node.parent_id !== null && !nodes.has(node.parent_id))) throw new Error("プレイ記録の分岐が不正である");
      nodes.set(node.node_id, { ...node, children: [], notes: [] });
      if (node.parent_id !== null) nodes.get(node.parent_id).children.push(node.node_id);
    };
    addNode({ node_id: "n0", parent_id: null, T: 0, action: null, at: first.at, actor: first.actor });
    imported.forEach((action, index) => addNode({ node_id: `n${index + 1}`, parent_id: `n${index}`, T: index + 1,
      action, at: first.at, actor: "import", note: "読み込んだ操作列" }));
    const session = { id: first.session_id, meta: first, nodes, events: [...events], game,
      head: `n${imported.length}`, revision: 0, requests: new Map() };
    for (const event of events.slice(1)) {
      if (event.revision !== session.revision + 1) throw new Error("プレイ記録の更新番号が連続していない");
      if (event.type === "step") {
        if (event.parent_id !== session.head) throw new Error("プレイ記録の操作元が現在位置と一致しない");
        addNode({ node_id: event.node_id, parent_id: event.parent_id, action: event.action, actor: event.actor,
          note: event.note, at: event.at, T: nodes.get(event.parent_id).T + 1, result: event.result });
        session.head = event.node_id;
      } else if (event.type === "checkout") {
        if (!nodes.has(event.node_id)) throw new Error("プレイ記録の移動先がない");
        session.head = event.node_id;
      } else if (event.type === "note") {
        if (!nodes.has(event.node_id)) throw new Error("プレイ記録のメモの局面がない");
        nodes.get(event.node_id).notes.push({ text: event.text, actor: event.actor, at: event.at });
      } else throw new Error(`未知の記録: ${event.type}`);
      session.revision = event.revision;
      if (!event.request_id || session.requests.has(event.request_id)) throw new Error("プレイ記録の要求IDが不正である");
      session.requests.set(event.request_id, event);
    }
    return session;
  }

  get(id) {
    if (!/^p_[a-f0-9-]{36}$/.test(id)) fail("unknown_session", "プレイ記録が見つからない", 404);
    if (this.sessions.has(id)) return this.sessions.get(id);
    const file = path.join(this.directory, id, "events.jsonl");
    if (!fs.existsSync(file)) fail("unknown_session", "プレイ記録が見つからない", 404);
    const raw = fs.readFileSync(file, "utf8");
    if (!raw.endsWith("\n")) throw new Error(`${id}: プレイ記録の末尾が不完全である。元の記録は変更していない`);
    const events = raw.trimEnd().split("\n").map((line) => JSON.parse(line));
    if (events[0]?.session_id !== id) throw new Error("記録のIDと保存先が一致しない");
    const game = this.newGame(events[0].input);
    try {
      const imported = engineCall(() => JSON.parse(game.replay(events[0].output)).actions);
      const session = this.fromEvents(events, game, imported);
      engineCall(() => game.replay(this.output(session, session.head)));
      this.sessions.set(id, session);
      return session;
    } catch (error) { game.free(); throw error; }
  }

  list() {
    const sessions = [], errors = [];
    for (const entry of fs.readdirSync(this.directory, { withFileTypes: true })) {
      if (!entry.isDirectory() || !entry.name.startsWith("p_")) continue;
      try {
        const session = this.get(entry.name);
        const state = JSON.parse(session.game.snapshot());
        sessions.push({ session_id: session.id, label: session.meta.label, case_name: session.meta.case_name,
          revision: session.revision, node_id: session.head, T: state.T, E: state.E, S: state.S,
          completed: state.completed, updated_at: session.events.at(-1).at });
      } catch (error) { errors.push({ session_id: entry.name, message: String(error.message ?? error) }); }
    }
    sessions.sort((a, b) => b.updated_at.localeCompare(a.updated_at));
    return { sessions, errors };
  }

  node(session, id) {
    const node = session.nodes.get(id);
    if (!node) fail("unknown_node", "指定された局面がない", 404);
    return node;
  }

  path(session, nodeId) {
    const result = [];
    let node = this.node(session, nodeId);
    while (node) { result.push(node); node = node.parent_id === null ? null : this.node(session, node.parent_id); }
    return result.reverse();
  }

  output(session, nodeId) {
    const lines = this.path(session, nodeId).slice(1).map((node) => actionLine(node.action));
    return lines.length ? `${lines.join("\n")}\n` : "";
  }

  resolveNode(session, { node_id = session.head, turn } = {}) {
    const node = this.node(session, node_id);
    if (turn === undefined) return node;
    if (!Number.isSafeInteger(turn) || turn < 0 || turn > node.T) fail("invalid_query", "turn は指定した分岐の手数以内の整数で指定する");
    return this.path(session, node_id)[turn];
  }

  at(session, nodeId, fn) {
    if (nodeId === session.head) return fn(session.game);
    const game = this.newGame(session.meta.input);
    try { engineCall(() => game.replay(this.output(session, nodeId))); return fn(game); }
    finally { game.free(); }
  }

  metadata(session, node) {
    return { schema_version: 1, session_id: session.id, label: session.meta.label, case_name: session.meta.case_name,
      revision: session.revision, node_id: node.node_id, live_node_id: session.head,
      is_live: node.node_id === session.head, parent_id: node.parent_id,
      children: node.children.map((id) => { const n = this.node(session, id); return { node_id: id, action: n.action, actor: n.actor }; }),
      notes: [...(node.note ? [{ text: node.note, at: node.at, actor: node.actor }] : []), ...node.notes],
      last_action: node.action, actor: node.actor };
  }

  state(id, options = {}) {
    const session = this.get(id), node = this.resolveNode(session, options);
    return this.at(session, node.node_id, (game) => ({ ...this.metadata(session, node),
      ...JSON.parse(game.snapshot()), ...(options.svg ? { svg: game.svg() } : {}) }));
  }

  version(id) { const s = this.get(id); return { session_id: id, revision: s.revision, node_id: s.head }; }

  checkRevision(session, revision) {
    if (!Number.isSafeInteger(revision) || revision < 0) fail("revision_required", "expected_revision に状態取得時の更新番号を指定する");
    if (revision !== session.revision) fail("revision_conflict", "局面が更新されている。state を再取得して操作を選び直す", 409,
      { revision: session.revision, node_id: session.head });
  }

  legal(id, { i, j, k = -1, expected_revision }) {
    const s = this.get(id);
    if (expected_revision !== undefined) this.checkRevision(s, expected_revision);
    if (![i, j, k].every(Number.isSafeInteger) || i < 0 || j < 0 || i > 19 || j > 19 || k < -1 || k > 7) {
      fail("invalid_query", "legal には盤面内の i, j と、必要なら k を指定する");
    }
    return { ...this.version(id), ...engineCall(() => JSON.parse(s.game.legal(i, j, k))) };
  }

  preview(id, body) {
    const s = this.get(id);
    this.checkRevision(s, body.expected_revision);
    return { ...this.version(id), ...engineCall(() => JSON.parse(s.game.preview(JSON.stringify(body.action ?? null)))) };
  }

  mutate(id, type, body) {
    const s = this.get(id);
    const requestId = textField(body.request_id, "request_id", 160);
    if (!requestId) fail("request_id_required", "再送の重複を防ぐ request_id を指定する");
    const payload = { ...body }; delete payload.request_id;
    const digest = hash(JSON.stringify(canonical({ type, ...payload })));
    const previous = s.requests.get(requestId);
    // 再送判定はrevision判定より先に行い、完了済み要求には最初と同じ結果を返す。
    if (previous) {
      if (previous.request_digest !== digest) fail("request_id_conflict", "同じ request_id が別の内容に使われている", 409);
      return previous.result;
    }
    if (s.writeFailure) fail("storage_unavailable", "この記録は保存エラーのため更新を停止している。保存先を確認してサーバーを再起動する", 503);
    this.checkRevision(s, body.expected_revision);
    const event = { type, request_id: requestId, request_digest: digest, revision: s.revision + 1,
      actor: textField(body.actor, "actor", 80, "ai"), at: new Date().toISOString() };
    let result;
    if (type === "step") {
      const effect = engineCall(() => JSON.parse(s.game.preview(JSON.stringify(body.action ?? null))));
      Object.assign(event, { node_id: `n${s.nodes.size}`, parent_id: s.head, action: effect.action,
        note: textField(body.note, "note", 4000) });
      result = { ...effect, session_id: id, revision: event.revision, node_id: event.node_id };
    } else if (type === "checkout") {
      event.node_id = this.resolveNode(s, { node_id: body.node_id, turn: body.turn }).node_id;
      const state = this.state(id, { node_id: event.node_id });
      result = { session_id: id, revision: event.revision, node_id: event.node_id,
        T: state.T, E: state.E, S: state.S, completed: state.completed };
    } else if (type === "note") {
      event.node_id = this.node(s, body.node_id ?? s.head).node_id;
      event.text = textField(body.text, "text", 4000).trim();
      if (!event.text) fail("invalid_request", "メモは空にできない");
      result = { session_id: id, revision: event.revision, node_id: event.node_id, text: event.text };
    } else fail("unknown_command", "未知の操作である", 404);
    event.result = result;
    // 保存成功を操作完了の境界にする。書き込みに失敗した要求では盤面を進めない。
    try {
      durableWrite(path.join(this.directory, id, "events.jsonl"), `${JSON.stringify(event)}\n`);
    } catch (error) {
      // 部分書き込みの成否が不明なまま追記を続けると、後続の記録も壊れる。
      s.writeFailure = true;
      throw new PlayError("storage_unavailable", `保存に失敗したため、この記録への更新を停止した: ${error.message}`, 503);
    }
    if (type === "step") {
      engineCall(() => s.game.apply(JSON.stringify(event.action)));
      const parent = this.node(s, s.head);
      parent.children.push(event.node_id);
      s.nodes.set(event.node_id, { node_id: event.node_id, parent_id: s.head, action: event.action,
        T: parent.T + 1, at: event.at, actor: event.actor, note: event.note, result, children: [], notes: [] });
      s.head = event.node_id;
    } else if (type === "checkout") {
      engineCall(() => s.game.replay(this.output(s, event.node_id)));
      s.head = event.node_id;
    } else this.node(s, event.node_id).notes.push({ text: event.text, actor: event.actor, at: event.at });
    s.revision = event.revision;
    s.events.push(event);
    s.requests.set(requestId, event);
    return result;
  }

  history(id, { offset = 0, limit = 100, node_id } = {}) {
    if (!Number.isSafeInteger(offset) || offset < 0 || !Number.isSafeInteger(limit) || limit < 1 || limit > 500) {
      fail("invalid_query", "offset は0以上、limit は1〜500の整数で指定する");
    }
    const s = this.get(id), current = new Set(this.path(s, s.head).map((n) => n.node_id));
    // 閲覧する枝の終端と閲覧位置を分離する。途中を選んでも、この順序付きの手順は切り詰めない。
    const all = node_id === undefined ? Array.from(s.nodes.values()).reverse() : this.path(s, node_id);
    const forks = new Map(), descendants = new Set(), branches = [];
    for (const node of s.nodes.values()) {
      const parent = node.parent_id === null ? null : this.node(s, node.parent_id);
      const fork = parent?.children.length > 1 ? parent.T : (forks.get(node.parent_id) ?? 0);
      forks.set(node.node_id, fork);
      if (node.node_id === s.head || descendants.has(node.parent_id)) descendants.add(node.node_id);
      if (!node.children.length) branches.push({ node_id: node.node_id, T: node.T, fork_T: fork, actor: node.actor,
        contains_live: descendants.has(node.node_id) });
    }
    return { ...this.version(id), total: all.length, offset, branches,
      nodes: all.slice(offset, offset + limit).map(({ result, ...node }) => ({ ...node,
        on_current_path: current.has(node.node_id), ...(result ? { E: result.E, S: result.S } : {}) })) };
  }

  export(id, { format = "json", node_id } = {}) {
    const s = this.get(id);
    if (format === "output") return this.output(s, node_id ?? s.head);
    if (format !== "json") fail("invalid_query", "format は json または output で指定する");
    return { schema_version: 1, session_id: id, current_node_id: s.head, exported_node_id: node_id ?? s.head,
      events: s.events, input: s.meta.input, output: this.output(s, node_id ?? s.head) };
  }
}
