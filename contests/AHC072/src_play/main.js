const $ = (id) => document.getElementById(id);
const colors = ["#ef8c8c", "#80b4f0", "#9bcf81", "#f5d66e", "#bc9bdd", "#79d3cf", "#f2b574", "#dba7c5", "#b5bd6b", "#6a91a8", "#ce8964", "#a3a3a3"];
const directions = { U: [-1, 0, "↑"], D: [1, 0, "↓"], L: [0, -1, "←"], R: [0, 1, "→"] };
const escape = (v) => String(v ?? "").replace(/[&<>"']/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" })[c]);
const number = (v) => Number(v).toLocaleString("ja-JP");
const chip = (c) => `<span class="color-chip" style="--slime:${colors[c]}">${c}</span>`;
const stackHtml = (stack) => `<div class="stack">${stack.length ? stack.map(chip).join("") : '<span class="hint">空</span>'}</div>`;
const targetOf = (a) => ({ i: a.i + directions[a.d][0] * a.l, j: a.j + directions[a.d][1] * a.l });
const actionText = (a) => a ? `(${a.i}, ${a.j}) → (${targetOf(a).i}, ${targetOf(a).j}) · 下に${a.k}匹残す` : "初期盤面";
const actionKey = (a) => a ? `${a.i},${a.j},${a.k},${a.d},${a.l}` : "";
const ns = "http://www.w3.org/2000/svg";
let sessionId = new URL(location.href).searchParams.get("session");
let snapshot = null, selected = null, allActions = [], actions = [], choice = null, prediction = null, lastResult = null;
let stateSequence = 0, selectionSequence = 0, previewSequence = 0, historySequence = 0;
let busy = false, polling = false, externalUpdate = false, draftFailed = false, stateLoading = 0;
let replayTip = null, replayMax = 0, replayTimer = null, sliderTimer = null, historyOffset = 0, branches = [];
const previewCache = new Map();

function message(id, text) { $(id).textContent = text; $(id).title = text; $(id).hidden = !text; }
function saved(text = "● 保存済み", ok = true) { $("saveStatus").textContent = text; $("saveStatus").classList.toggle("saved", ok); }
function route(command, id = sessionId) { return `sessions/${encodeURIComponent(id)}/${command}`; }
async function api(command, body, raw = false) {
  let response;
  try {
    response = await fetch(`/api/play/${command}`, body === undefined ? { cache: "no-store" }
      : { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify(body) });
  } catch { throw new Error("サーバーに接続できない。dev_vis.sh の起動を確認する。"); }
  if (!response.ok) {
    const data = await response.json();
    const error = new Error(data.error?.message ?? `HTTP ${response.status}`);
    error.code = data.error?.code;
    throw error;
  }
  return raw ? response.text() : response.json();
}
async function perform(fn) {
  try { await fn(); }
  catch (error) {
    message("error", error.message);
    if (error.code === "revision_conflict") markExternalUpdate();
  }
}
function stopReplay() { clearTimeout(replayTimer); replayTimer = null; $("playReplay").textContent = "▶ 再生"; }
function clearSelection(render = true) {
  selectionSequence++; previewSequence++;
  selected = null; allActions = []; actions = []; choice = null; prediction = null; previewCache.clear();
  if (render) {
    message("notice", "塔を選び、着地点をクリックして実行。右クリックで選択解除。");
    renderSelection(); renderPrediction(); renderOverlay(); renderControls();
  }
}
function draftKey() { return snapshot ? `ahc072:play:draft:${sessionId}:${snapshot.node_id}` : null; }
function persistDraft() {
  const key = draftKey();
  if (!key) return;
  const draft = { note: $("noteText").value, action: $("actionNote").value };
  try {
    if (draft.note || draft.action) localStorage.setItem(key, JSON.stringify(draft));
    else localStorage.removeItem(key);
    draftFailed = false;
  } catch {
    draftFailed = true;
    throw new Error("メモの下書きをブラウザに保存できない。記録を切り替える前に文章をコピーする。");
  }
  renderDraftStatus();
}
function restoreDraft() {
  const raw = localStorage.getItem(draftKey());
  const draft = raw ? JSON.parse(raw) : {};
  $("noteText").value = draft.note ?? "";
  $("actionNote").value = draft.action ?? "";
  renderDraftStatus();
}
function renderDraftStatus() {
  $("noteDraftStatus").textContent = $("noteText").value ? "未保存 · 下書き保持" : "";
  $("actionDraftStatus").textContent = $("actionNote").value ? "下書きあり" : "";
  renderControls();
}
async function loadInfo() {
  const info = await api("info");
  $("caseSelect").innerHTML = info.cases.map((name) => `<option value="${escape(name)}">${escape(name)}</option>`).join("");
  $("sessions").innerHTML = '<option value="">記録を選択</option>' + info.sessions.map((s) =>
    `<option value="${escape(s.session_id)}">${escape(s.label || s.session_id)} · ${s.T}手 / 残り${s.E}</option>`).join("");
  if (sessionId) $("sessions").value = sessionId;
  if (info.errors.length) message("error", info.errors.map((e) => `${e.session_id}: ${e.message}`).join(" / "));
  return info;
}
async function openSession(id) {
  if (busy || stateLoading) return;
  persistDraft(); stopReplay(); clearTimeout(sliderTimer);
  stateSequence++; historySequence++; clearSelection(false);
  sessionId = id; snapshot = null; lastResult = null; externalUpdate = false; replayTip = null; branches = []; historyOffset = 0;
  const url = new URL(location.href); url.searchParams.set("session", id); history.replaceState(null, "", url);
  $("sessions").value = id;
  message("error", ""); message("notice", "右クリックでも選択を解除できる");
  const view = JSON.parse(sessionStorage.getItem(`ahc072:play:view:${id}`) ?? "null");
  if (view && !view.live) {
    replayTip = view.tip; replayMax = view.max;
    await loadState({ node: view.node, keepTip: true });
  } else await loadState();
}
async function loadState({ node, turn, keepTip = false, keepPlayback = false, keepSelection = false } = {}) {
  if (!sessionId) return;
  if (!keepPlayback) stopReplay();
  persistDraft();
  const sequence = ++stateSequence, id = sessionId;
  stateLoading = sequence; renderControls();
  const query = new URLSearchParams({ svg: "1" });
  if (node) query.set("node_id", node);
  if (turn !== undefined) query.set("turn", turn);
  try {
    const data = await api(route(`state?${query}`, id));
    if (sequence !== stateSequence || id !== sessionId) return;
    // 通信中に書いた文章も、切り替わる前の局面へ保存する。
    persistDraft();
    const retain = keepSelection && snapshot?.node_id === data.node_id;
    snapshot = data; externalUpdate = false;
    if (!retain) clearSelection(false);
    else { previewSequence++; previewCache.clear(); choice = null; prediction = null; }
    if (!keepTip) { replayTip = data.node_id; replayMax = data.T; }
    historyOffset = Math.floor(data.T / 100) * 100;
    restoreDraft(); renderState();
    if (retain) {
      const legal = await api(route(`legal?i=${selected.i}&j=${selected.j}&expected_revision=${data.revision}`, id));
      if (sequence !== stateSequence || id !== sessionId) return;
      allActions = legal.actions; actions = allActions.filter((action) => action.k === selected.k);
      renderOverlay();
    }
    await refreshHistory();
    // 巻き戻した位置へ戻った場合も、保存された続きまで再生範囲に含める。
    if (!keepTip && snapshot.is_live && sequence === stateSequence) {
      const continuation = branches.filter((branch) => branch.contains_live).at(-1);
      if (continuation && continuation.T > replayMax) {
        replayTip = continuation.node_id; replayMax = continuation.T;
        renderControls(); await refreshHistory();
      }
    }
    if (sequence === stateSequence && id === sessionId) sessionStorage.setItem(`ahc072:play:view:${id}`,
      JSON.stringify({ node: snapshot.node_id, live: snapshot.is_live, tip: replayTip, max: replayMax }));
  } catch (error) {
    if (sequence === stateSequence && id === sessionId) throw error;
  } finally {
    if (stateLoading === sequence) { stateLoading = 0; renderControls(); }
  }
}
function renderControls() {
  const locked = busy || Boolean(stateLoading);
  const live = snapshot?.is_live && !externalUpdate, editable = live && !locked;
  $("undo").disabled = !editable || !snapshot?.parent_id;
  $("redo").disabled = !editable || !snapshot?.children.length;
  $("clearSelection").disabled = !selected || locked;
  $("saveNote").disabled = !snapshot || locked || externalUpdate || !$("noteText").value.trim();
  $("sessions").disabled = locked; $("newSession").disabled = locked;
  $("resumeHere").disabled = locked || externalUpdate; $("returnLive").disabled = locked;
  $("historyBranch").disabled = locked || !branches.length;
  $("noteText").disabled = !snapshot;
  $("actionNote").disabled = busy || !snapshot;
  $("board").setAttribute("aria-busy", String(locked));
  $("refreshState").hidden = !externalUpdate;
  for (const id of ["firstTurn", "prevTurn"]) $(id).disabled = locked || !snapshot || snapshot.T === 0;
  for (const id of ["nextTurn", "lastTurn"]) $(id).disabled = locked || !snapshot || snapshot.T >= replayMax;
  $("playReplay").disabled = locked || !replayMax;
  $("turnSlider").disabled = busy || !replayMax;
  $("turnSlider").max = replayMax;
  if (!stateLoading) $("turnSlider").value = snapshot?.T ?? 0;
  $("turnLabel").textContent = `${snapshot?.T ?? 0} / ${replayMax}`;
  for (const id of ["exportOutput", "exportJson"]) $(id).disabled = !snapshot;
}
function renderState() {
  $("metricT").textContent = number(snapshot.T); $("metricE").textContent = number(snapshot.E); $("metricS").textContent = number(snapshot.S);
  $("boardCaption").textContent = snapshot.completed ? "全員が帰巣した。別の手順も試せる" : snapshot.is_live ? "塔を選ぶ → 着地点をクリックして実行" : `${snapshot.T}手目を閲覧中`;
  $("boardSize").textContent = `${snapshot.N} × ${snapshot.N} · 色 / 高さ`;
  $("pastActions").hidden = snapshot.is_live;
  document.querySelector(".mode-bar").classList.toggle("past", !snapshot.is_live);
  $("redoBranch").hidden = true;
  $("redoBranch").innerHTML = '<option value="">やり直す手を選択</option>' + snapshot.children.map((child) => `<option value="${child.node_id}">${escape(actionText(child.action))}</option>`).join("");
  $("received").innerHTML = snapshot.initial_counts.map((count, color) => `<span class="received-item ${snapshot.received[color] === count ? "complete" : ""}" title="色${color}の帰巣数 / 初期匹数">${chip(color)} ${snapshot.received[color]}/${count}</span>`).join("");
  $("notes").innerHTML = snapshot.notes.map((note) => `<div class="note">${escape(note.text)}<small>${escape(note.actor)} · ${escape(new Date(note.at).toLocaleString("ja-JP"))}</small></div>`).join("");
  $("noteNode").textContent = `${snapshot.T}手目へのメモ`;
  $("sessionIdentity").textContent = `ID: ${sessionId}`;
  const option = Array.from($("sessions").options).find((o) => o.value === sessionId);
  if (option && snapshot.is_live) option.textContent = `${snapshot.label || sessionId} · ${snapshot.T}手 / 残り${snapshot.E}`;
  saved(); renderBoard(); renderSelection(); renderPrediction(); renderControls();
}
function svgElement(tag, attrs, text) {
  const el = document.createElementNS(ns, tag);
  for (const [key, value] of Object.entries(attrs)) el.setAttribute(key, String(value));
  if (text !== undefined) el.textContent = text;
  return el;
}
function renderBoard() {
  if (!snapshot) return;
  $("board").innerHTML = snapshot.svg;
  const svg = $("board").querySelector("svg");
  svg.setAttribute("viewBox", "0 50 772 756"); svg.removeAttribute("width"); svg.removeAttribute("height");
  svg.setAttribute("aria-label", `${snapshot.N}行${snapshot.N}列の盤面、残り${snapshot.E}匹`);
  const cropped = svgElement("g", { "clip-path": "url(#play-board-clip)", "pointer-events": "none" });
  for (const child of Array.from(svg.children)) if (child.localName !== "defs") cropped.append(child);
  const defs = svgElement("defs", {}), clip = svgElement("clipPath", { id: "play-board-clip" });
  clip.append(svgElement("rect", { x: 0, y: 50, width: 772, height: 756 })); defs.append(clip); svg.append(defs, cropped);
  const labels = svgElement("g", { "pointer-events": "none", "font-family": "system-ui, sans-serif" }), cell = 720 / snapshot.N;
  for (const nest of snapshot.nests) {
    const x = 36 + nest.j * cell, y = 76 + nest.i * cell;
    labels.append(svgElement("rect", { x: x + 1, y: y + cell - 17, width: Math.min(34, cell - 2), height: 16, rx: 3, fill: "white", opacity: .94 }));
    labels.append(svgElement("text", { x: x + 3, y: y + cell - 4, "font-size": 12.5, fill: "#233a35" }, `巣${nest.color}`));
  }
  for (const tower of snapshot.towers) {
    const x = 36 + tower.j * cell, y = 76 + tower.i * cell, label = `${tower.colors.at(-1)}${tower.colors.length > 1 ? `/${tower.colors.length}` : ""}`;
    const width = Math.min(cell - 3, label.length * 7.5 + 4);
    labels.append(svgElement("rect", { x: x + cell - width - 1, y: y + 1, width, height: 16, rx: 3, fill: "white", opacity: .95 }));
    labels.append(svgElement("text", { x: x + cell - width / 2 - 1, y: y + 13, "text-anchor": "middle", "font-size": 12.5, "font-weight": 650, fill: "#233a35" }, label));
  }
  svg.append(labels, svgElement("g", { id: "playOverlay", "pointer-events": "none" }));
  // 一枚の透明なマスを最前面に置き、直前操作の点や塔の絵にクリックを奪われないようにする。
  const hitLayer = svgElement("g", { id: "playHitLayer" });
  const walls = new Set(snapshot.walls.map(([i, j]) => `${i},${j}`));
  for (let i = 0; i < snapshot.N; i++) for (let j = 0; j < snapshot.N; j++) {
    const tower = snapshot.towers.find((t) => t.i === i && t.j === j), nest = snapshot.nests.find((n) => n.i === i && n.j === j);
    const label = `(${i}, ${j})${walls.has(`${i},${j}`) ? " 壁" : tower ? ` 下から ${tower.colors.join("、")} · ${tower.colors.length}匹` : " 空"}${nest ? ` · 巣${nest.color}` : ""}`;
    const hit = svgElement("rect", { x: 36 + j * cell, y: 76 + i * cell, width: cell, height: cell, fill: "transparent", "pointer-events": "all", "data-play-cell": i * snapshot.N + j, "aria-label": label });
    hit.append(svgElement("title", {}, label)); hitLayer.append(hit);
  }
  svg.append(hitLayer); renderOverlay();
}
function renderOverlay() {
  const overlay = $("playOverlay"); if (!overlay || !snapshot) return;
  overlay.replaceChildren();
  const cell = 720 / snapshot.N;
  const rect = (i, j, active, source = false) => overlay.append(svgElement("rect", { x: 36 + j * cell + 2, y: 76 + i * cell + 2, width: cell - 4, height: cell - 4, rx: 4, fill: source ? "#155e63" : "#389389", "fill-opacity": active ? .18 : .07, stroke: "#157873", "stroke-width": active ? 3 : 1.5, ...(source ? { "stroke-dasharray": "5 2" } : {}) }));
  if (selected) {
    rect(selected.i, selected.j, true, true);
    for (const action of actions) { const q = targetOf(action); rect(q.i, q.j, actionKey(action) === actionKey(choice)); }
  }
  if (prediction && choice) {
    for (const [index, change] of prediction.changed_cells.entries()) {
      const x = 36 + change.j * cell, y = 76 + change.i * cell;
      overlay.append(svgElement("rect", { x: x + 4, y: y + 4, width: cell - 8, height: cell - 8, fill: "white", opacity: .72 }));
      const h = Math.min(8, (cell - 17) / Math.max(1, change.after.length));
      change.after.forEach((c, k) => overlay.append(svgElement("rect", { x: x + cell * .22, y: y + cell - 5 - h * (k + 1), width: cell * .56, height: h - .5, rx: 2, fill: colors[c], stroke: "#155e63", "stroke-width": .6 })));
      const count = index ? prediction.returned.destination.count : prediction.returned.source.count;
      if (count) overlay.append(svgElement("text", { x: x + cell / 2, y: y + 15, "text-anchor": "middle", "font-size": 13, "font-weight": 700, fill: "#155e63" }, `帰巣${count}`));
    }
  }
  const destinations = new Set(actions.map((a) => { const q = targetOf(a); return q.i * snapshot.N + q.j; }));
  const towers = new Set(snapshot.towers.map((t) => t.i * snapshot.N + t.j));
  for (const hit of $("board").querySelectorAll("[data-play-cell]")) {
    const p = Number(hit.dataset.playCell);
    hit.classList.toggle("selectable", towers.has(p) && snapshot.is_live);
    hit.classList.toggle("destination", destinations.has(p) && !externalUpdate);
  }
}
function renderSelection() {
  $("towerPicker").hidden = !selected || !snapshot?.is_live;
  if (!selected) return;
  const tower = snapshot.towers.find((t) => t.i === selected.i && t.j === selected.j);
  $("sourceTitle").textContent = `(${selected.i}, ${selected.j}) · ${tower.colors.length}匹`;
  const towerScroll = $("tower").scrollTop;
  $("tower").innerHTML = tower.colors.map((c, k) => `<button class="tower-layer ${k >= selected.k ? "moving" : "staying"} ${k === selected.k ? "boundary" : ""}" style="--slime:${colors[c]}" data-k="${k}" aria-pressed="${k === selected.k}" aria-label="下に${k}匹残し、上の${tower.colors.length - k}匹を跳ばす" ${busy ? "disabled" : ""}><span>色 ${c}</span></button>`).reverse().join("");
  $("tower").scrollTop = towerScroll;
  $("jumpSummary").textContent = `残す ${selected.k} · 跳ぶ ${tower.colors.length - selected.k}`;
  // 一匹の塔は分割を選ぶ必要がなく、盤面を余分な操作欄で覆わない。
  $("towerPicker").hidden = tower.colors.length === 1;
  positionPicker();
}
function positionPicker() {
  if (!selected || $("towerPicker").hidden) return;
  const hit = $("board").querySelector(`[data-play-cell="${selected.i * snapshot.N + selected.j}"]`);
  if (!hit) return;
  const stage = $("boardStage").getBoundingClientRect(), r = hit.getBoundingClientRect(), picker = $("towerPicker"), gap = 7;
  const x = r.x - stage.x, y = r.y - stage.y;
  const grid = { left: x - selected.j * r.width, top: y - selected.i * r.height };
  grid.right = grid.left + snapshot.N * r.width; grid.bottom = grid.top + snapshot.N * r.height;
  const overlap1d = (a, b, c, d) => Math.max(0, Math.min(b, d) - Math.max(a, c));
  function placements() {
    const box = picker.getBoundingClientRect();
    const candidates = [[x + r.width + gap, y + r.height + gap], [x - box.width - gap, y + r.height + gap],
      [x + r.width + gap, y - box.height - gap], [x - box.width - gap, y - box.height - gap],
      [grid.right + gap, y + r.height / 2 - box.height / 2], [grid.left - box.width - gap, y + r.height / 2 - box.height / 2]];
    return candidates.map(([left, top]) => {
      const l = Math.max(0, Math.min(stage.width - box.width, left)), t = Math.max(0, Math.min(stage.height - box.height, top));
      // 実際の盤面の行・列を避ける。盤面外の余白なら同じ高さでも着地点を遮らない。
      const overlap = overlap1d(l, l + box.width, grid.left, grid.right) * overlap1d(t, t + box.height, y, y + r.height)
        + overlap1d(l, l + box.width, x, x + r.width) * overlap1d(t, t + box.height, grid.top, grid.bottom);
      return { left: l, top: t, overlap, cost: overlap * 1000 + Math.abs(l - left) + Math.abs(t - top) };
    }).sort((a, b) => a.cost - b.cost);
  }
  picker.style.width = "150px"; picker.style.maxHeight = "none"; picker.style.setProperty("--layer-height", "25px");
  let placement = placements()[0];
  if (placement.overlap > .1) {
    // 高い塔と小さい画面の組み合わせでは、斜めの空間に収まる寸法にする。
    // 段の数だけが多い場合は欄内でスクロールでき、着地点は必ず空けておく。
    const height = Math.floor(Math.max(y - gap, stage.height - y - r.height - gap));
    const width = Math.floor(Math.max(x - gap, stage.width - x - r.width - gap));
    picker.style.width = `${Math.min(150, width)}px`; picker.style.maxHeight = `${height}px`;
    picker.style.setProperty("--layer-height", "20px");
    placement = placements()[0];
  }
  picker.style.left = `${placement.left}px`; picker.style.top = `${placement.top}px`;
}
function renderPrediction() {
  const result = prediction ?? (!selected ? lastResult : null);
  $("previewTitle").textContent = prediction ? "この手の予告" : result ? "実行した手の結果" : "結果の予告";
  $("previewBadge").textContent = prediction ? "クリックで実行" : result ? "保存済み" : selected ? "着地点を選択" : "未選択";
  if (!result) {
    $("previewSummary").textContent = choice ? "結果を確認中…" : selected ? `下に${selected.k}匹残す · 最大${selected.k + 1}マス` : "塔を選ぶと、跳べるマスが表示される。";
    $("preview").innerHTML = `<p class="hint">${selected ? allActions.length && !actions.length ? "この分け方では跳べない。残す匹数を変えてみよう。" : "枠のある着地点に重ねて予告を確認。クリックすると一手進む。右クリックで選択解除。" : "着地点にマウスを重ねると、積み順と帰巣する匹数を確認できる。クリックで実行する。"}</p>`;
    return;
  }
  const count = result.returned.source.count + result.returned.destination.count;
  $("previewSummary").textContent = `帰巣 ＋${count}匹 · 残り ${result.E}匹 · ${result.T}手`;
  $("preview").innerHTML = result.changed_cells.map((change, index) => {
    const count = index ? result.returned.destination.count : result.returned.source.count;
    return `<div class="change-row"><div class="change-label">${index ? "着地点" : "出発点"} (${change.i}, ${change.j})${change.nest === null ? "" : ` · 巣${change.nest}`}<span class="return-count">${count ? `帰巣 ＋${count}匹` : "上 ↑ · 下 ↓"}</span></div><div class="stack-change"><div class="stack-side"><div class="stack-label">前</div>${stackHtml(change.before)}</div><span class="arrow">→</span><div class="stack-side"><div class="stack-label">後</div>${stackHtml(change.after)}</div></div></div>`;
  }).join("");
}
async function selectTower(i, j) {
  if (busy || stateLoading || externalUpdate || !snapshot?.is_live) return;
  const tower = snapshot.towers.find((t) => t.i === i && t.j === j);
  if (!tower) { clearSelection(); return; }
  stopReplay(); clearSelection(false); lastResult = null;
  selected = { i, j, k: 0 };
  message("error", ""); message("notice", tower.colors.length > 1 ? "段を選ぶと、その段から上が跳ぶ。着地点をクリックして実行。" : "着地点に重ねて予告を確認。クリックで実行。");
  renderSelection(); renderPrediction(); renderOverlay(); renderControls();
  const sequence = ++selectionSequence, id = sessionId;
  let result;
  try { result = await api(route(`legal?i=${i}&j=${j}&expected_revision=${snapshot.revision}`, id)); }
  catch (error) { if (sequence === selectionSequence && id === sessionId) throw error; else return; }
  if (sequence !== selectionSequence || id !== sessionId) return;
  allActions = result.actions; actions = allActions.filter((a) => a.k === selected.k);
  renderOverlay(); renderPrediction();
}
function selectLayer(k) {
  if (busy || stateLoading || externalUpdate || !selected) return;
  selected.k = k; choice = null; prediction = null; previewSequence++;
  actions = allActions.filter((a) => a.k === k);
  renderSelection(); renderPrediction(); renderOverlay();
  $("board").focus({ preventScroll: true });
}
async function chooseAction(action) {
  if (busy || stateLoading || externalUpdate || !selected || actionKey(action) === actionKey(choice)) return;
  const sequence = ++previewSequence, id = sessionId, revision = snapshot.revision;
  choice = action; prediction = previewCache.get(actionKey(action)) ?? null;
  renderOverlay(); renderPrediction();
  if (prediction) return;
  let result;
  try { result = await api(route("preview", id), { expected_revision: revision, action }); }
  catch (error) {
    // クリック実行や選択変更で不要になった予告は、失敗応答も画面へ持ち込まない。
    if (sequence === previewSequence && id === sessionId && revision === snapshot?.revision) throw error;
    return;
  }
  if (sequence !== previewSequence) return;
  prediction = result; previewCache.set(actionKey(action), result); renderOverlay(); renderPrediction();
}
function clearHover() {
  if (!choice || busy) return;
  previewSequence++; choice = null; prediction = null; renderOverlay(); renderPrediction();
}
async function mutation(command, fields, after) {
  if (!snapshot || busy || stateLoading || externalUpdate) return;
  persistDraft(); busy = true; stopReplay(); selectionSequence++; previewSequence++;
  saved("保存中…", false); renderControls(); renderSelection();
  try {
    const result = await api(route(command), { expected_revision: snapshot.revision, request_id: crypto.randomUUID(), actor: "human", ...fields });
    message("error", ""); await after(result); return result;
  } catch (error) { saved("保存を確認できない", false); throw error; }
  finally { busy = false; renderControls(); renderSelection(); }
}
async function commit(action) {
  if (!action || !snapshot?.is_live) return;
  const note = $("actionNote").value;
  await mutation("step", { action, note }, async (result) => {
    $("actionNote").value = ""; persistDraft(); lastResult = result;
    await loadState();
    const count = result.returned.source.count + result.returned.destination.count;
    message("notice", count ? `${count}匹が帰巣した。${result.completed ? "全員が帰巣した。" : `残り${result.E}匹。`}` : "一手進めた。次の塔を選択。間違えたら「一手戻す」で戻せる。");
  });
}
async function checkout(node, keepTip = true) {
  await mutation("checkout", { node_id: node }, async () => {
    lastResult = null; await loadState({ keepTip }); message("notice", "元の手順も保存されている。ここから別の手を試せる。");
  });
}
function redo() {
  if (busy || stateLoading || externalUpdate || !snapshot?.is_live) return;
  if (snapshot.children.length === 1) return checkout(snapshot.children[0].node_id);
  if (snapshot.children.length > 1) { $("redoBranch").hidden = !$("redoBranch").hidden; }
}
async function refreshHistory() {
  if (!sessionId || !replayTip) return;
  const sequence = ++historySequence, tip = replayTip, id = sessionId;
  const history = await api(route(`history?node_id=${encodeURIComponent(tip)}&offset=${historyOffset}&limit=100`, id));
  if (sequence !== historySequence || tip !== replayTip || id !== sessionId) return;
  branches = history.branches;
  const options = branches.map((branch, index) => ({ ...branch, label: `手順 ${index + 1} · ${branch.T}手${branches.length > 1 ? `（${branch.fork_T}手目で分岐）` : ""}` }));
  if (!options.some((branch) => branch.node_id === replayTip)) options.push({ node_id: replayTip, label: `${replayMax}手目までの手順` });
  $("historyBranch").innerHTML = options.map((branch) => `<option value="${branch.node_id}">${branch.label}</option>`).join("");
  $("historyBranch").value = replayTip;
  $("branchHint").textContent = "行を選んで前後を確認。「ここからプレイ」で別の手を試せる。";
  $("history").innerHTML = history.nodes.map((node) => `<button class="history-row ${node.node_id === snapshot.node_id ? "current" : ""}" data-turn="${node.T}" data-node="${node.node_id}" title="${node.node_id}"><span class="history-turn">${node.T}手目</span><span class="history-body">${escape(actionText(node.action))}${node.children.length > 1 ? ` · ここから${node.children.length}通りに分岐` : ""}<span class="history-note">${escape([node.note, ...node.notes.map((n) => n.text)].filter(Boolean).join(" / "))}</span></span><span class="history-actor">${node.actor === "human" ? "人間" : escape(node.actor)}</span></button>`).join("");
  $("earlierHistory").disabled = history.offset === 0;
  $("laterHistory").disabled = history.offset + history.nodes.length >= history.total;
  $("historyRange").textContent = `${history.offset}〜${history.offset + history.nodes.length - 1}手目 / 全${history.total - 1}手`;
  renderControls();
}
async function showTurn(turn, keepPlayback = false) {
  if (!snapshot || !replayTip || busy) return;
  lastResult = null;
  await loadState({ node: replayTip, turn: Math.max(0, Math.min(replayMax, turn)), keepTip: true, keepPlayback });
}
function scheduleReplay() {
  replayTimer = setTimeout(() => perform(async () => {
    if (!replayTimer || !snapshot) return;
    if (snapshot.T >= replayMax) { stopReplay(); return; }
    await showTurn(snapshot.T + 1, true); if (replayTimer) scheduleReplay();
  }), 600);
}
async function toggleReplay() {
  if (!snapshot || !replayMax) return;
  if (replayTimer) { stopReplay(); return; }
  if (snapshot.T >= replayMax) await showTurn(0);
  $("playReplay").textContent = "Ⅱ 停止"; scheduleReplay();
}
async function download(format) {
  if (!snapshot) return;
  const text = await api(route(`export?format=${format}&node_id=${snapshot.node_id}`), undefined, true);
  const url = URL.createObjectURL(new Blob([text], { type: format === "output" ? "text/plain" : "application/json" }));
  const link = document.createElement("a"); link.href = url; link.download = `${sessionId}-${snapshot.node_id}.${format === "output" ? "txt" : "json"}`;
  link.click(); setTimeout(() => URL.revokeObjectURL(url), 1000);
}
function openCreate() {
  message("createError", "");
  for (const id of ["inputText", "outputText", "inputFile", "outputFile", "sessionLabel"]) $(id).value = "";
  $("createDialog").showModal();
}
$("newSession").addEventListener("click", openCreate); $("emptyStart").addEventListener("click", openCreate);
$("closeDialog").addEventListener("click", () => $("createDialog").close());
$("createForm").addEventListener("submit", (event) => {
  event.preventDefault(); perform(async () => {
    if (busy) return;
    persistDraft(); busy = true; $("createSubmit").disabled = true; renderControls();
    try {
      const input = $("inputText").value.trim(), label = $("sessionLabel").value.trim();
      const created = await api("sessions", { ...(input ? { input } : { case_name: $("caseSelect").value }), output: $("outputText").value, actor: "human", ...(label ? { label } : {}) });
      $("createDialog").close(); await loadInfo(); busy = false; await openSession(created.session_id);
    } catch (error) { message("createError", error.message); }
    finally { busy = false; $("createSubmit").disabled = false; renderControls(); }
  });
});
for (const [file, textarea] of [["inputFile", "inputText"], ["outputFile", "outputText"]]) $(file).addEventListener("change", () => perform(async () => { if ($(file).files[0]) $(textarea).value = await $(file).files[0].text(); }));
$("sessions").addEventListener("change", () => { if ($("sessions").value) perform(() => openSession($("sessions").value)); });
$("refreshSessions").addEventListener("click", () => perform(loadInfo));
$("copySession").addEventListener("click", () => perform(async () => { if (sessionId) { await navigator.clipboard.writeText(sessionId); message("notice", "IDをコピーした。AIに渡すと同じ局面を引き継げる。"); } }));
function cellFromEvent(event) {
  const hit = event.target.closest("[data-play-cell]");
  if (!hit || !snapshot) return null;
  const p = Number(hit.dataset.playCell); return { i: Math.floor(p / snapshot.N), j: p % snapshot.N };
}
$("board").addEventListener("click", (event) => perform(async () => {
  if (!snapshot || busy || stateLoading || externalUpdate) return;
  if (!snapshot.is_live) { message("notice", "閲覧中の局面から試すには「ここからプレイ」を選択。"); return; }
  const cell = cellFromEvent(event); if (!cell) { clearSelection(); return; }
  if (selected?.i === cell.i && selected?.j === cell.j) { clearSelection(); return; }
  const action = actions.find((a) => { const q = targetOf(a); return q.i === cell.i && q.j === cell.j; });
  if (action) await commit(action); else await selectTower(cell.i, cell.j);
}));
$("board").addEventListener("pointermove", (event) => {
  if (!selected || busy || stateLoading || event.pointerType === "touch") return;
  const cell = cellFromEvent(event);
  const action = cell && actions.find((a) => { const q = targetOf(a); return q.i === cell.i && q.j === cell.j; });
  if (action) perform(() => chooseAction(action)); else clearHover();
});
$("board").addEventListener("pointerleave", clearHover);
$("boardStage").addEventListener("contextmenu", (event) => { event.preventDefault(); if (!busy && !stateLoading) clearSelection(); });
$("tower").addEventListener("click", (event) => { const button = event.target.closest("[data-k]"); if (button) selectLayer(Number(button.dataset.k)); });
$("clearSelection").addEventListener("click", () => clearSelection());
$("undo").addEventListener("click", () => { if (snapshot?.parent_id) perform(() => checkout(snapshot.parent_id)); });
$("redo").addEventListener("click", () => perform(redo));
$("redoBranch").addEventListener("change", () => { if ($("redoBranch").value) perform(() => checkout($("redoBranch").value, false)); });
$("resumeHere").addEventListener("click", () => perform(() => checkout(snapshot.node_id)));
$("returnLive").addEventListener("click", () => perform(async () => { lastResult = null; await loadState(); }));
$("history").addEventListener("click", (event) => { const button = event.target.closest("[data-turn]"); if (button) perform(() => showTurn(Number(button.dataset.turn))); });
$("historyBranch").addEventListener("change", () => perform(async () => { lastResult = null; await loadState({ node: $("historyBranch").value }); }));
$("earlierHistory").addEventListener("click", () => perform(async () => { historyOffset = Math.max(0, historyOffset - 100); await refreshHistory(); }));
$("laterHistory").addEventListener("click", () => perform(async () => { historyOffset += 100; await refreshHistory(); }));
for (const id of ["noteText", "actionNote"]) $(id).addEventListener("input", () => perform(persistDraft));
$("saveNote").addEventListener("click", () => perform(async () => {
  if (!snapshot) return;
  const text = $("noteText").value, node = snapshot.node_id;
  if (!text.trim()) return;
  await mutation("note", { node_id: node, text }, async () => {
    if ($("noteText").value === text) $("noteText").value = "";
    persistDraft(); await loadState({ node, keepTip: true, keepSelection: true }); message("notice", "この局面にメモを保存した。");
  });
}));
$("exportOutput").addEventListener("click", () => perform(() => download("output")));
$("exportJson").addEventListener("click", () => perform(() => download("json")));
$("firstTurn").addEventListener("click", () => perform(() => showTurn(0)));
$("prevTurn").addEventListener("click", () => perform(() => showTurn(snapshot?.T - 1)));
$("nextTurn").addEventListener("click", () => perform(() => showTurn(snapshot?.T + 1)));
$("lastTurn").addEventListener("click", () => perform(() => showTurn(replayMax)));
$("playReplay").addEventListener("click", () => perform(toggleReplay));
$("turnSlider").addEventListener("input", () => { stopReplay(); clearTimeout(sliderTimer); sliderTimer = setTimeout(() => perform(() => showTurn(Number($("turnSlider").value))), 70); });
document.addEventListener("keydown", (event) => {
  if ($("createDialog").open || event.target.closest("textarea,input,select") || busy || stateLoading || !snapshot) return;
  if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "z") {
    event.preventDefault(); if (!snapshot.is_live || externalUpdate) return;
    if (event.shiftKey) perform(redo); else if (snapshot.parent_id) perform(() => checkout(snapshot.parent_id));
  } else if (event.key === "Escape") clearSelection();
  else if (event.key === "Enter" && choice && !event.target.closest("button,a")) { event.preventDefault(); perform(() => commit(choice)); }
  else if (selected && event.key.startsWith("Arrow")) {
    const d = { ArrowUp: "U", ArrowDown: "D", ArrowLeft: "L", ArrowRight: "R" }[event.key], candidates = actions.filter((a) => a.d === d);
    if (candidates.length) { event.preventDefault(); const index = candidates.findIndex((a) => actionKey(a) === actionKey(choice)); perform(() => chooseAction(candidates[(index + 1) % candidates.length])); }
  }
});
function markExternalUpdate() {
  externalUpdate = true; stopReplay();
  message("notice", "別の操作元から更新がある。選択と下書きを確認して、最新の局面へ移れる。"); renderControls();
}
$("refreshState").addEventListener("click", () => perform(async () => { lastResult = null; await loadState(); message("error", ""); message("notice", "最新の局面を表示した。"); }));
new ResizeObserver(positionPicker).observe($("boardStage"));
window.addEventListener("beforeunload", (event) => { if (draftFailed) { event.preventDefault(); event.returnValue = ""; } });
setInterval(() => perform(async () => {
  if (!sessionId || !snapshot || busy || stateLoading || polling || document.hidden || replayTimer || externalUpdate) return;
  polling = true;
  try {
    const id = sessionId, sequence = stateSequence;
    const version = await api(route("version", id));
    if (id !== sessionId || busy || !snapshot || sequence !== stateSequence || version.revision === snapshot.revision) return;
    // 選択中・執筆中は画面を入れ替えず、明示的な更新操作まで手元の文脈を保つ。
    if (selected || $("noteText").value || $("actionNote").value || !snapshot.is_live) markExternalUpdate();
    else { lastResult = null; await loadState(); message("notice", "別の操作元から更新された局面を表示している。"); }
  } finally { polling = false; }
}), 1500);
perform(async () => { renderControls(); await loadInfo(); if (sessionId) await openSession(sessionId); });
