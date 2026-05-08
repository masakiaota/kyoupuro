# AHC034 - physics0523 解法メモ

## 参照元

- 記事: [AHC034 参加記](https://physics0523.hatenablog.com/entry/2024/06/16/214158)
- 著者: physics0523
- サイト: physics0523's 精進ログ
- 種別: 上位解説、公開提出コード
- 成績・順位: 本番2位、最終提出9183817629点
- コード有無: あり
- コードを読めたか: 読めた
  - [Submission #54643078](https://atcoder.jp/contests/ahc034/submissions/54643078)
- 読めなかったもの: 特になし。記事内の画像は本文で説明された範囲のみ確認した

## 解法の全体像

全マスを2巡するようなwalkを作り、そのwalkの辺を「土を運ぶコンベア」と見なして最小費用流を流す。最初は素直な2巡walkを使い、その後、2巡目を1列飛ばしや複数列飛ばしにする候補を増やし、さらにwalk末尾を削りながら最小費用流を繰り返す。時間内に複数のwalk候補を試し、復元した操作列の実コストが最小のものを出力する。

yosupo解と同じく「walkを決めたらフローで土量を決める」方向だが、コード上は訪問時刻頂点を分ける形ではなく、グリッド頂点間にwalk辺を張って土の輸送を表す実装になっている。

## 主要アイデア

- スコア800M級に必要なコストを逆算し、全マス3巡は移動固定費だけで重すぎると判断する。
- 全体を2巡するwalkなら、移動固定費を抑えつつ多くの方向の輸送辺を確保できる。
- walkの辺を有向辺として最小費用流に入れると、どの辺でどれだけ土を運ぶかが求まる。
- 2巡のうち一方を完全走査にせず、1列飛ばしや複数列飛ばしにして歩数を減らす。
- 候補walkを複数持ち、各walkの末尾を1手ずつ削りながら評価し続ける。
- 最終操作列はフロー量から各辺に入る前の積載量を逆算して復元する。

## 最終コードの構造

### 状態表現

- `h[20][20]`: 初期高さ。
- `string w`: walkを表す移動文字列。
- `vector<string> ws`: 複数のwalk候補。
- `mcf_graph<int,int> g`: フロー計算用グラフ。
- `fmem[400][400]`: グリッド有向辺ごとのフロー量を一時的に集計する配列。
- `vector<string> res`: 出力操作列。移動文字と `+d` / `-d` を同じ文字列列で持つ。

### 観測・制約・入力の扱い

- `N` と高さだけを読む。
- 正のマスを供給、負のマスを需要として扱う。
- `work(w)` で指定walkに対して最小費用流を流し、必要流量に達しない場合は空の結果を返す。
- 復元後、`+0` や `-0` はfail safeとして出力しない。

### 評価関数

`work(w)` 内の最小費用流では、次のコストを使う。

- `S -> 正のマス`: 容量 `h`、コスト1。
- `負のマス -> T`: 容量 `-h`、コスト1。
- walk上の移動辺: 容量 `1e5`、コスト1。

記事の発想では、これは「土1単位を積む、降ろす、1辺運ぶ」コストを表す。復元後の `eval(res)` では、実際の操作列を走査して

```text
move cost = 100 + current_load
load/unload cost = abs(delta)
```

を合算する。最終選択はこの復元後コストで行う。

### 探索・構築・更新

候補walkは手書きパターンで作る。

- 行方向に全体を舐めてから列方向に舐める2巡。
- 逆向きの2巡。
- 2巡目で1列飛ばし、2列飛ばし、3列飛ばし、4列飛ばしに近いパターン。
- どちら向きから始めるかを変えたパターン。

探索は次のような単純な多候補評価である。

```text
while elapsed < 1.98s:
    if ws[0] is empty:
        break
    for each walk candidate ws[i]:
        ops = work(ws[i])
        if ops is valid:
            c = eval(ops)
            keep best
        remove last move from ws[i]
```

局所探索でwalk内部を入れ替えるのではなく、用意した複数パターンの末尾削除を順に試す。

### 操作・クエリ・出力選択

`work(w)` はフロー後に、各walk辺に流れる土量を `fmem[from][to]` に集計する。その後walkを先頭から辿り、次の辺で運ぶべき量 `aim` と現在積載量 `sd` の差 `del = aim - sd` を見て、移動前に `del` だけ積むか降ろす。

```text
for each move edge e in walk:
    aim = flow on e
    del = aim - current_load
    if del != 0:
        output load/unload del
    current_load = aim
    output move
after final edge:
    if current_load != 0:
        output unload all
```

### 時間配分・パラメータ

- 全体の評価ループは `current_clock() < 1980000`、約1.98秒。
- 各walk辺容量は `1e5`。
- 最小費用流はAC Library。
- 候補walkはコードに直接書かれた定数パターンであり、乱択や焼きなましは使わない。

## 実装上重要な断片

walk候補を評価する処理は次の形である。

```text
work(w):
    add source/sink edges from initial heights
    for each move in w:
        add directed grid edge with cost 1
    run min-cost flow
    if flow is insufficient:
        return invalid
    collect flow on each grid edge
    replay w and emit load/unload before each move
```

候補削減は非常に単純である。

```text
for each candidate walk:
    evaluate current walk
    pop_back one move
```

## この解法の本質

この解法は、完全な経路最適化を狙わず、「2巡程度のwalkでも輸送グラフとしてはかなり強い」という性質を使っている。Perlin noise由来の高さは局所的にまとまりやすく、盤面全体に有向辺をほどよく張るだけで、土の大移動の遠回りが少なくなる。

さらに、2巡目を間引くことで、輸送能力を大きく落とさずに移動固定費を削っている。最小費用流により土量は自動で最適化されるため、人間が調整すべき対象はwalkの形と長さに集中できる。

## 真似するならまず実装する部分

まずは素直な2巡walkを作り、walk辺に最小費用流を流して操作列を復元する部分を実装する。次に、2巡目の列飛ばしパターンを数種類用意し、末尾を削りながら評価するだけでよい。

yosupo型の訪問時刻頂点つきMCFより実装は粗いが、固定パターンを多数試す方針としては短時間で書きやすい。

## 注意点・未理解点

- コードはグリッド頂点間の有向辺でフローを表しており、同じ有向辺が複数回出る場合の流量割当は `fmem` に集約される。提出ではACしているが、訪問時刻ごとに分けるMCFほど一般的な復元形式ではない。
- `eval` は末尾の移動だけを削るtrimを行う。空列や予想外の操作列で壊れないようにする必要がある。
- walk候補は手書き定数が多く、バグ混入しやすい。
- 記事中でも、本番提出に一度バグがありスコアが不正確だった旨が述べられているため、候補walk生成の検証が重要である。
