#!/usr/bin/env node
import fs from "node:fs";
import crypto from "node:crypto";

const help = `AHC072 プレイ用コマンド（先に ./scripts/dev_vis.sh を起動する）

node scripts/play.mjs <操作> [オプション]

info / list                         入力ケース / 保存したプレイを列挙
create --case 0000.txt               プレイを開始（--input 入力ファイル も使用可）
       [--output 解答ファイル] [--label 名前]
state --session ID                   局面を取得（--node ID / --turn T で過去を閲覧）
legal --session ID --i I --j J        合法手を取得（--k K で絞り込み）
preview --session ID --revision R --action '{"i":0,"j":0,"k":0,"d":"R","l":1}'
step    --session ID --revision R --action '{...}' [--note 理由]
checkout --session ID --revision R --node n0
history --session ID [--offset 0] [--limit 100]
note --session ID --revision R --text 気づき [--node ID]
export --session ID [--format json|output] [--node ID]

--json JSON / --json -               要求をJSON文字列 / 標準入力から読み込む
--url URL                           接続先（既定: AHC_PLAY_URL または http://127.0.0.1:5173）
--request-id ID                      step / checkout / note の再送用ID（省略時は自動生成）
--actor 名前                        操作者（既定: ai）

座標・色は0始まり。塔の配列は下から上。step等にはstateで読んだrevisionを指定する。
書き込みの自動再送は行わない。通信に失敗したときは返されたrequest_idで同じ要求を再送する。
export --format output は公式形式のテキストを返し、それ以外はJSONを返す。
`;

let requestId;
try {
  const args = process.argv.slice(2);
  if (!args.length || args.includes("--help") || args[0] === "help") {
    process.stdout.write(help); process.exit(0);
  }
  const command = args.shift(), flags = {};
  while (args.length) {
    const key = args.shift();
    if (!/^--[a-z-]+$/.test(key) || !args.length) throw new Error(`不正なオプション: ${key}`);
    if (key.slice(2) in flags) throw new Error(`オプションが重複している: ${key}`);
    flags[key.slice(2)] = args.shift();
  }
  const allowed = ["url", "json", "session", "revision", "node", "turn", "case", "input", "output", "label", "action",
    "i", "j", "k", "d", "l", "note", "text", "actor", "offset", "limit", "format", "request-id"];
  for (const key of Object.keys(flags)) if (!allowed.includes(key)) throw new Error(`未知のオプション: --${key}`);
  const body = flags.json ? JSON.parse(flags.json === "-" ? fs.readFileSync(0, "utf8") : flags.json) : {};
  if (!body || Array.isArray(body) || typeof body !== "object") throw new Error("要求はJSONオブジェクトで指定する");
  const sessionId = flags.session ?? body.session_id;
  delete body.session_id;
  const mappings = { revision: "expected_revision", node: "node_id", case: "case_name", "request-id": "request_id" };
  for (const [key, value] of Object.entries(flags)) {
    if (["url", "json", "session", "input", "output", "action", "i", "j", "k", "d", "l"].includes(key)) continue;
    body[mappings[key] ?? key] = ["revision", "turn", "offset", "limit"].includes(key) ? Number(value) : value;
  }
  if (flags.input) body.input = fs.readFileSync(flags.input, "utf8");
  if (flags.output) body.output = fs.readFileSync(flags.output, "utf8");
  if (flags.action) body.action = JSON.parse(flags.action);
  if (command === "legal") {
    for (const key of ["i", "j", "k"]) if (key in flags) body[key] = Number(flags[key]);
  } else if (["i", "j", "k", "d", "l"].some((key) => key in flags)) {
    body.action = { ...body.action };
    for (const key of ["i", "j", "k", "d", "l"]) if (key in flags) body.action[key] = key === "d" ? flags[key] : Number(flags[key]);
  }
  const read = ["info", "list", "state", "legal", "history", "export"];
  const write = ["create", "preview", "step", "checkout", "note"];
  if (![...read, ...write].includes(command)) throw new Error(`未知の操作: ${command}`);
  if (!["create", "info", "list"].includes(command) && !sessionId) throw new Error("--session または JSONのsession_id が必要である");
  const pathname = command === "info" ? "info" : command === "list" || command === "create" ? "sessions"
    : `sessions/${encodeURIComponent(sessionId)}/${command}`;
  const url = new URL(`/api/play/${pathname}`, flags.url ?? process.env.AHC_PLAY_URL ?? "http://127.0.0.1:5173");
  const options = {};
  if (read.includes(command)) {
    for (const [key, value] of Object.entries(body)) url.searchParams.set(key, String(value));
  } else {
    body.actor ??= "ai";
    if (["step", "checkout", "note"].includes(command)) {
      body.request_id ??= crypto.randomUUID();
      requestId = body.request_id;
    }
    Object.assign(options, { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify(body) });
  }
  let response;
  try { response = await fetch(url, { ...options, signal: AbortSignal.timeout(30000) }); }
  catch (error) { throw new Error(`APIに接続できないか、応答が途切れた: ${url.origin}。dev_vis.shの起動先を確認する。${error.message}`); }
  const raw = await response.text();
  if (!response.ok) {
    let detail;
    try { detail = JSON.parse(raw); } catch { detail = { error: { code: "http_error", message: raw } }; }
    process.stdout.write(`${JSON.stringify({ ...detail, ...(requestId ? { request_id: requestId } : {}) }, null, 2)}\n`);
    process.exitCode = 1;
  } else if (command === "export" && body.format === "output") process.stdout.write(raw);
  else process.stdout.write(`${JSON.stringify(JSON.parse(raw), null, 2)}\n`);
} catch (error) {
  process.stdout.write(`${JSON.stringify({ error: { code: "client_error", message: error.message }, ...(requestId ? { request_id: requestId } : {}) }, null, 2)}\n`);
  process.exitCode = 1;
}
