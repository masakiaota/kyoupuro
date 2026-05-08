# AHC063 - through 解法メモ

## 参照元

- 記事: [AHC063参加記 (最終4位)](https://zenn.dev/through/articles/d95a7007ea77ce)
- 著者: through
- サイト: Zenn
- 種別: 上位解説、提出コード
- 成績・順位: 最終4位 / 1277位、performance 3023
- コード有無: あり。記事内の最終提出 [AtCoder submission 74942809](https://atcoder.jp/contests/ahc063/submissions/74942809)
- コードを読めたか: 読めた。C++23、AC、実行時間1916ms、コードサイズ64456Byte
- 読めなかったもの: 記事中の図は本文・キャプション以上の細部は確認していない。GitHubプロフィール等は解法コードではないため対象外とした

## 解法の全体像

一致接頭長を1つ以上伸ばすまとまりを遷移としたビームサーチである。状態はターン数別の層に入り、遷移にかかった実ターン数だけ先の層へ挿入される。主遷移は、次の目標色へ向かう `GoTarget`、噛みちぎって整合復元する `CutRecover`、局所的に盤面を崩す `Wander` の3系統である。

最終提出コードも記事の説明と対応しており、`transitionGoTargetCandidates`、`transitionCutRecoverCandidates`、`transitionPrefixBrokenCutRecover`、`transitionWander` を各ターン層の上位状態に適用する構成だった。

## 主要アイデア

- 状態の良さは、おおむね「一致接頭長が長いほど良い、壁・角に残った餌が少ないほど良い」で評価する。
- ターン数は評価値に直接入れず、ターン数層ビームの深さとして扱う。
- 次の目標色へ行く経路はBFSで作る。体のマスが何ターン後に空くかをリリース時刻として持ち、尾を追う経路を許す。
- 色ずれ状態では、近場で噛みちぎる短手版と、一致接頭長の直後を狙う色ずれ近辺版の2種類の `CutRecover` を使う。
- `Wander` は数手のランダム移動で、GoTarget/CutRecoverだけでは動けない局面を崩す。
- 重複排除用ハッシュは細かくしすぎず、餌配置と頭位置中心にして探索の多様性を保つ。
- 盤面、体、色別餌位置をビットボードで管理し、BFS・衝突判定・餌判定を高速化する。

## 最終コードの構造

### 状態表現

- `SnakeState`
  - `bodyBuf`: 頭から尾への座標列。N<=8用とN>8用でテンプレートパラメータを切り替える。
  - `colorBuf`: 体色列。4bit単位の詰め込みで保持する。
  - `bodyOcc`, `foodOcc`, `foodColorOcc`: 体・餌・色別餌のビットボード。
  - `foodCount`, `wallFoodCount`, `cornerFoodCount`: 残り餌数と壁/角餌数。
  - `overlapPos`: 噛みちぎり直後など、頭と尾が同じ座標を共有する特殊状態の管理。
  - `turn`: 現在ターン。
- `BeamNode`
  - `st`: `SnakeState` の丸ごとコピー。
  - `pathTail`, `pathLen`: `PathPool` 上の復元用移動列。
  - `prefix`, `maxPrefix`: 現在/過去最大の一致接頭長。
  - `eval`, `stateKey`, `optimisticLB`: 評価値、軽い状態ハッシュ、下界枝刈り用値。

### 観測・制約・入力の扱い

- 入力の盤面は `foodOcc` と `foodColorOcc[color]` に変換し、各マスの隣接先 `gNextPos`、壁/角フラグ、マンハッタン距離を前計算する。
- 初期蛇は `(4,0),(3,0),(2,0),(1,0),(0,0)`、色はすべて1として `bodyBuf` と `colorBuf` に入れる。
- 移動時は盤外・Uターンを拒否する。餌を食べる場合は尾を残し、食べない場合は尾を消す。
- 頭が胴体に入ったときは、噛みちぎられた尾側の体色を対応座標の餌として戻す。

### 評価関数

- 記事の式は `-一致接頭長 * A + 壁際餌ペナルティ`。
- コードでは、`evaluateBeamStateFromPrefix(st, pref)` が次を返す。
  - `-activePrefixPrimaryWeight * pref + foodWallPenalty(st)`
- `foodWallPenalty` は壁餌と角餌を重めに数える。
- 既に長い完成解がある場合は `activePrefixPrimaryWeight` を非常に大きくし、残りを完成へ寄せる。
- 下界は「次の必要色までのマンハッタン距離 + 残り接頭長を埋める最低手数」に近い素朴な値で、既知完成解より短くならない状態を枝刈りする。

### 探索・構築・更新

- ビームは `layers[turn]` に状態を置くターン数層方式である。
- 各層から上位 `beamWidth` 件を選び、次の候補を挿入する。
- `GoTarget`
  - 色ずれでない場合、隣接目標チェックの後、非目標餌を踏まないBFSで最大数本の経路を作る。
  - 強い経路がない、または色ずれ中なら、非目標餌を踏むことも許す弱い経路を1本作る。
  - いずれも噛みちぎりは基本的に許さない。
- `CutRecover`
  - DFSで短手数の噛みちぎり候補を列挙する。
  - 色ずれ削減、接頭長破壊の少なさ、復元後の次色への接続、手数の短さで候補を選ぶ。
  - 噛みちぎり直前の体列を保存し、噛みちぎり後は古い体列を辿って接頭長を復元する。
- `PrefixBrokenCutRecover`
  - 一致接頭長 `p` 付近の体節を狙う専用BFS。
  - `afterBodyLen` が `p` 近辺になる噛みちぎりを探す。
- `Wander`
  - N別の長さだけランダムに合法移動する。
  - 噛みちぎりを起こす移動は避ける。餌を踏む場合も体衝突を避ける。

### 操作・クエリ・出力選択

- 探索中に完成状態が見つかれば、既知解よりターン数が短いかで更新する。
- 完成状態がない場合でも、最良の接頭長・評価値の状態を保持する。
- `PathPool` から最良ノードの移動列を復元し、各文字を1行ずつ出力する。

### 時間配分・パラメータ

- 探索時間は約1900ms。
- 初期ビーム幅は20、N別の最大ビーム幅は80または160。
- GoTarget候補数はN別に3。
- CutRecoverの列挙深さはN別に3〜5程度。
- Wander長はN別に1〜6程度。
- CutRecover/Wander/PrefixBrokenCutRecoverはビーム上位の一部状態にだけ適用する。

## 実装上重要な断片

```text
beam_search():
    layers[start_turn].push(initial)
    for turn in start_turn..cap:
        nodes = top_by_eval(layers[turn], beam_width)
        for rank, node in nodes:
            add(GoTarget(node))
            if rank < cut_limit:
                add(CutRecover(node))
            if rank < prefix_cut_limit:
                add(PrefixBrokenCutRecover(node))
            if rank < wander_limit:
                add(Wander(node))
```

```text
cut_recover(node, path_to_bite):
    save old_body
    apply moves until bite
    recovery = trace old_body from new tail/head relation
    apply recovery
    recompute prefix/eval/hash
```

```text
strict_bfs_to_target:
    block non-target foods
    allow body cell only if arrival_time >= release_time[cell]
    randomize direction order
    keep first few shortest target paths
```

## この解法の本質

この解法の本質は、1マス単位の膨大な分岐を「意味のある状態変化」に圧縮しつつ、噛みちぎりを単なる事故ではなく状態整理の操作として探索に組み込んだ点である。GoTargetだけでは蛇自身と餌密度で詰まりやすいが、CutRecoverで体の向き・長さ・色ずれを調整し、Wanderで局所配置を崩すことで、次の目標色へ進む選択肢を復活させている。

また、評価関数を過度に賢くせず、候補生成側に問題固有の知識を寄せている。AHCでは評価関数を複雑にするより、よい遷移を作って探索空間を制御する方が強い場合がある、という典型例である。

## 真似するならまず実装する部分

最小実装なら次の順で作るのがよい。

1. 蛇の移動・餌・噛みちぎりを正確にシミュレートする `State`。
2. 一致接頭長と次の目標色を計算する処理。
3. 非目標餌を避けるBFSで次の目標色へ行く `GoTarget`。
4. 噛みちぎり後、古い体列を辿って接頭長を復元する `CutRecover`。
5. ターン数層ビームと、`-prefix*A + 壁餌ペナルティ` の評価。

ビットボード、PrefixBrokenCutRecover、N別パラメータ、軽いハッシュは上位化の段階で足せばよい。

## 注意点・未理解点

- 記事中の図の細かい盤面状況は確認していないため、図に依存する直感説明は本文ベースで要約している。
- `Wander` はランダム性とN別パラメータへの依存が大きく、再実装時のスコア再現性は低い可能性がある。
- 状態ハッシュを簡略化すると別状態を同一視するため、実装によっては良くも悪くも振れる。
- `overlapPos` のような頭尾重複状態を雑に扱うと、噛みちぎり判定や体占有ビットが壊れやすい。
- CutRecoverで「どこまで復元するか」は記事では重要な調整点だが、最終コードの全パラメータ意図までは断定できない。
