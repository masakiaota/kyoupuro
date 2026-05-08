# AHC060 - yuuut_bfs 解法メモ

## 参照元

- 記事: BFSベースのシンプルな解法
- 著者: YuuuT
- サイト: AtCoder Editorial
- 種別: 上位解説
- 成績・順位: 16位。公開提出は 167926 点
- コード有無: あり。AtCoder提出 `https://atcoder.jp/contests/ahc060/submissions/72939049`
- コードを読めたか: 読めた
- 読めなかったもの: なし

## 解法の全体像

状態を `(現在頂点, 直前の移動元, 現在のアイス列)` として BFS し、まだその店に納品していない文字列を作れる最短経路を探す。経路が見つかればその経路を実行し、見つからなければランダムウォークする。

赤化は、BFSで得た経路が長いときにだけ確率的に挿入する。1回のシミュレーションで得たスコアを評価し、時間内に何度もパラメータの違う試行を行って最良の出力を採用する。

## 主要アイデア

- BFS状態に `prev` とコーンのハッシュを入れ、Uターン禁止と文字列差分を表現する。
- BFSは未納品文字列を持った状態で店に到達した時点で経路を返す。
- 経路長上限をランダムに変え、探索しすぎと短すぎのバランスを取る。
- 選んだ経路が長い場合だけ、現在地の白い木を確率 `0.05〜0.15` 程度で赤にする。
- 各試行のスコアを `sum |stock[i]|` で評価し、2秒弱の間に最良試行を残す。
- コーン文字列は rolling hash で持つ。
- BFS の訪問管理は `(v, prev, hash)` をキーにした自作ハッシュセットで高速化している。

## 最終コードの構造

### 状態表現

- `G`: 隣接リスト
- `shop_inventory`: 各店の `unordered_set<ull>`
- `tree_colors`: 各頂点の色。初期値は全て `W`
- `moves_log`: 出力する行動列
- `BFSNode`: `v`, `pv`, `len`, `hash`, `parent`
- `nodes_pool`: BFSノードのメモリプール
- `FastHashSet`: BFS中の訪問状態 `(v, pv, hash)` を開番地法で管理する
- `path_len`: 試行ごとにランダムに選ぶ基準長

コーンは `current_hash` と `current_len` で持つ。木に移動すると `hash = hash * HASH_BASE + color` として更新し、店に移動すると在庫へ挿入して空に戻す。

### 観測・制約・入力の扱い

- 入力座標は読み捨てる。
- 行動1では `next_v == last_v` を禁止する。
- 行動2は現在地が木で、かつ `tree_colors[current_v] == 'W'` のときだけ挿入する。
- 経路候補がなければ、直前頂点以外の隣接頂点からランダムに1つ選ぶ。

### 評価関数

- 1試行の評価は `score = sum_{i=0}^{K-1} shop_inventory[i].size()`。
- 時間内に複数試行し、最大スコアの `moves_log` を出力する。
- 探索中の目的は「店 `v < K` に到達したとき、現在 hash が `shop_inventory[v]` に存在しないこと」である。

### 探索・構築・更新

- `bfs(start_v, forbidden_prev, current_hash, current_len)` で経路を探す。
- BFS root は現在状態で、隣接展開時に直前頂点への移動を除外する。
- 店に到達し、現在ハッシュが未納品なら parent を辿って経路を復元する。
- 長さ制限として `cur.len >= path_len + 8` になった状態は展開しない。コードでは `path_len` が10〜12なので、実質18〜20程度の上限になる。
- `solve()` では `path_len = 10 + randrange(3)`、赤化確率を `0.05〜0.15` から選ぶ。
- BFSで得た `path.size() >= path_len` のとき、その経路を実行する前または途中で赤化判定を行う。
- 行動2の前に「近傍が全て赤なら赤化しない」という判定を入れている。

### 操作・クエリ・出力選択

- BFS経路が見つかった場合、その頂点列を順に行動1として出力する。
- 長い経路中で条件を満たすと `-1` を出力し、現在木を `R` にする。
- 店へ移動したら在庫へ現在ハッシュを挿入し、コーンを空にする。
- 10000ターン分の行動を作り、最良試行の行動列を最後に出力する。

### 時間配分・パラメータ

- 全体を 1978 ms まで反復する。
- `path_len` は 10, 11, 12 のいずれか。
- BFS展開のコーン長上限は `path_len + 8`。
- 赤化確率は `0.05 + random() * 0.1`。
- `nodes_pool` は20000程度を予約している。
- ハッシュ基数は `10007`。

## 実装上重要な断片

```text
bfs(state):
    push root(current_v, prev, cone_hash, cone_len)
    while queue not empty:
        cur = pop()
        if cur.v is shop and cur.hash is new for this shop:
            return restore_path(cur)
        if cur.len >= limit:
            continue
        for next in G[cur.v]:
            if next == cur.prev:
                continue
            next_hash = cur.hash
            if next is tree:
                next_hash = append_color(next_hash, color[next])
            if visited.insert(next, cur.v, next_hash):
                push(next)
```

```text
for next in chosen_path:
    if path_is_long and current is white tree and random() < p:
        output -1
        color[current] = R
    output next
    apply_move(next)
```

## この解法の本質

難しい赤化計画を直接最適化せず、まず「今の色配置で取れる新規納品」をBFSで貪欲に取る。取れなくなりがちな長い経路でだけ赤化を混ぜることで、文字列空間を少しずつ変え、短い新規納品を再び見つけやすくする。強い最適化ではないが、状態を正しく持ったBFSと時間いっぱいの反復で安定して高いスコアを出している。

## 真似するならまず実装する部分

この解法を真似るなら、最初にシミュレータ、在庫集合、BFS経路復元を実装するのがよい。赤化の条件は後から調整できるため、まずは「現在の色で未納品文字列を作れる店へ行く」部分を正しく作るべきである。

## 注意点・未理解点

- rolling hash の衝突対策はしていない。実用上は問題になりにくいが、厳密な同一判定ではない。
- `tree_colors` は全頂点サイズで持っており、近傍赤判定で店をどう扱うかはコード上やや粗い。
- BFSの訪問キーは `(v, prev, hash)` で、長さは直接入っていない。ハッシュが長さを十分区別する前提になっている。
- 赤化は確率的なので、パラメータ依存が大きい。
- 記事では Gemini がコードを書いたと明記されている。細部の意図は作者自身の説明だけでは追い切れない部分がある。
