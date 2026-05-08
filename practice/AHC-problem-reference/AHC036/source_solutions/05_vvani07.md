# AHC036 - vvani07 解法メモ

## 参照元

- 記事: [AHC036 参加記 (101位)](https://vvani07.hatenadiary.org/entry/2024/10/07/212149)
- 著者: VVani07
- サイト: 今日も陽気にアレグロ技研
- 種別: 詳細参加記、GitHub PR、最終コード
- 成績・順位: 101位。記事では最終付近で 64000 から 58000 程度へ改善
- コード有無: あり。Browserで記事内GitHub PR `https://github.com/vvani06/atcoder_dlang/pull/1` と master 上の [`home/heuristic/ahc036/src/a.d`](https://github.com/vvani06/atcoder_dlang/blob/master/home/heuristic/ahc036/src/a.d) を確認した
- コードを読めたか: 読めた。D言語実装で、GitHub上の `a.d` に `Simulator`, `provisionRoute`, `provisionSignal`, `simulate`, `repeatSimulate` がある
- 読めなかったもの: BrowserでAtCoder提出一覧 `https://atcoder.jp/contests/ahc036/submissions?f.User=vvani07` と `?f.User=VVani07` を開いたが該当提出はなかった。PR 内の全コミット差分は量が多いため、最終コードと本文・コミットログ中心に読んだ。ローカル実行まではしていない

## 解法の全体像

複数種類のグラフと頂点コストで目的地列をつなぐ基礎ルートを作り、そのルート上でよく現れる連続頂点集合を A の信号部品として抽出する。A の部品は BitSet で評価し、重複を許しすぎないようにしながら、高スコアな集合から採用する。

旅行時は、現在 B に入っている `visitable` を見て、次に必要な頂点がなければ A からコピーする区間を選ぶ。長さ LB 固定だけでなく、B 内の今後使わなそうな連続空き領域に部分コピーする。複数のグラフ構築案と、信号部品の乱択並べ替えを時間内に試し、最良出力を選ぶ。

## 主要アイデア

- 生グラフ、重み付き生グラフ、中心から作る木、2中心から作る木など複数の経路生成器を試す。
- 目的地周辺の頂点コストを下げる、または目的地周辺に重みを付けることで、よく使う頂点を通るルートを作る。
- ルートを長さごとの集合として BitSet 化し、頻出・長い集合を A の候補にする。
- A 候補は重複許容量 `LA - uniqueRoute.length` を見ながら採用し、重複が多すぎる集合は捨てる。
- 長い信号列と短い信号列を交互に近い順で並べ、A 区間の隣接性を上げる。
- シミュレーションでは、今後の route をどれだけ満たせるかでコピー区間を選ぶ。
- B 内の不要になった領域を見つけ、そこへ部分コピーできるなら部分置き換えする。
- 最後は `connectedSignals` を乱択シャッフルし、スコアが悪化しない範囲で最良解を探す。

## 最終コードの構造

### 状態表現

- `Simulator` が1つの構築案を表す。名前、グラフ、頂点コスト、全点間コスト、目的地列から作る `route`、信号配列 `signals` を持つ。
- `signalsArray` は A に入れる候補集合の列である。
- `connectedSignals` は、長い列と短い列を隣接しやすい形で連結した部品列で、乱択並べ替えの単位にもなる。
- `visitable` は現在の B の内容で、初期はすべて -1。
- `accNodeCount[n][i]` は A の prefix 中に頂点 n が何回出るかを表し、任意区間が頂点を含むかを O(1) で判定する。
- `startIndiciesPerSignal[n]` は、頂点 n を含み得る A の開始位置リストである。

### 観測・制約・入力の扱い

- `provisionRoute` は各目的地へ向かう最短路をつなぎ、移動ルートを先に固定する。
- グラフ候補は、元グラフ、目的地周辺の重み付き元グラフ、中心からの木、2中心からの木を使う。
- 中心は、目的地への距離和が小さい頂点、または2点組を総当たりで選ぶ。
- MST と呼んでいるが、実装上は中心からの BFS 的な木構築に近い。

### 評価関数

- `provisionSignal` では、ルート上の連続集合を BitSet 化し、長さに強く依存するスコアを加算する。長いまとまりほど高評価である。
- 採用時は、既採用頂点との重複数が許容量を超える候補を捨てる。また、新規頂点が少なすぎる候補も捨てる。
- `simulate` のスコアは信号操作回数で、`Ans` の比較はこの値の小ささを基準にする。
- コピー区間選択では、現在位置 ti から先の route を何個連続で満たせるか `satisfied` を最大化し、同率なら小さいコピー長を選ぶ。

### 探索・構築・更新

- `provisionRoute`:
  - グラフと頂点コストで全点間最短路を求める。
  - 現在地点から次目的地までの next ポインタを辿り、ルート列を作る。
- `provisionSignal`:
  - route の連続部分集合を BitSet として集計する。
  - 単独頂点も候補に足す。
  - スコア順に候補を見て、重複制約を満たすものを採用する。
- `sortSignals`:
  - 候補を短い順に管理し、長い効率的な列の両側に隣接しやすい短い列を置く。
  - 連結済み部品を `connectedSignals` に保存する。
- `simulate`:
  - route を前から見て、次頂点が現在 B にないときだけ信号操作を行う。
  - A の候補開始位置とコピー長を全探索し、先の route を長く満たすものを選ぶ。
  - B 内で今後 LB 手程度使わなさそうな連続領域があれば、そこに部分コピーする。
- `repeatSimulate`:
  - `connectedSignals` を乱択シャッフルして A を作り直し、最良を更新する。

### 操作・クエリ・出力選択

- 出力1行目は `signals`、すなわち A である。
- route の各頂点 t について、B に t がなければ信号操作を出す。
- 部分コピー可能なら `s sigSize sigLeft partialIndex`、無理なら `s sigSize sigLeft 0` として B を更新する。
- その後に `m t` を出す。route は隣接頂点列として作っているため、各移動は道路に沿う。

### 時間配分・パラメータ

- 最終コードは 2650ms まで `repeatSimulate(1)` を繰り返す。
- 目的地周辺コストでは、距離層ごとに重みを変える2種類の重み付けを試している。
- 記事では、部分置き換えと A 並べ替えで 64000 から 60000、乱択山登りで 58000 程度へ改善したとある。

## 実装上重要な断片

```text
choose_signal_copy(ti):
    for size in 1..LB:
        for start in windows containing route[ti]:
            satisfied = count future route nodes covered by A[start:start+size]
            keep best satisfied, then shorter size
```

```text
partial_replace():
    find B ranges whose vertices are not used in near future
    if range length >= selected signal size:
        copy selected A segment into that B range
    else:
        copy full segment to B[0..]
```

## この解法の本質

この解法の本質は、ルートを先に固定し、そのルート上で繰り返し現れる頂点集合を A に埋めることである。強い A を完全に探索するのではなく、実際に通る予定の route から信号候補を抽出するため、実装が比較的素直で、改善の効果も測りやすい。

また、部分コピーを「B の近未来で使わない領域へ差し込む」という現実的な条件で導入している。長さ LB 固定コピーだけでは捨てる情報が多いが、全状態探索をせずに部分更新の利益を拾っている点が実装上の特徴である。

## 真似するならまず実装する部分

まず、通常グラフの全点間最短路から目的地列をつなぐ route を作り、その route をそのまま A に詰めて、必要になったら LB 固定でコピーして移動するシミュレータを作る。

次に、route の連続集合を BitSet で数え、頻出集合を優先して A 候補にする。最後に、部分コピーと `connectedSignals` の乱択並べ替えを入れる。

## 注意点・未理解点

- PR の全コミット差分は読まず、最終コードと記事・主要コミットログを中心に読んだため、途中方針の細かい失敗理由は網羅していない。
- route を先に固定するため、A によって移動経路そのものを大きく変える上位解より自由度は低い。
- `signalsArray` の BitSet スコアは長さを非常に強く評価しており、ケースによっては短いが重要な候補が落ちる可能性がある。
- 部分コピーの判定は近未来 route に基づく貪欲であり、後方まで含めた最適性はない。
