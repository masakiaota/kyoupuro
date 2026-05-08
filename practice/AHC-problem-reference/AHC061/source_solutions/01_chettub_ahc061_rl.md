# AHC061 - chettub_ahc061_rl 解法メモ

## 参照元

- 記事: https://github.com/chettub/AHC061-RL
- 著者: cuthbert / chettub
- サイト: GitHub
- 種別: 上位解法リポジトリ、強化学習実装、提出コード
- 成績・順位: 1位解法として `solution_urls.md` に記載。本人記事ではRLで優勝できそうな流れとして説明されている
- コード有無: あり。`BLOG_ja.md`、`exps/exp002/README.md`、`exps/exp003/README.md`、`features_research_v4.hpp`、`adf_beta_estimator.hpp`、`models.py`、`submit/solver_base_compact_ppconcat_*.cpp` を確認した
- コードを読めたか: 主要な提出テンプレート、特徴量、推定器、モデル構造は読めた。生成済み `main.cpp` は圧縮済みモデル重みを含む巨大な単一ファイルであり、重みそのものの意味は読めない
- 読めなかったもの: `BLOG_ja.md` からリンクされている `SOLUTION_ja.md` は 404 だった。最終提出に埋め込まれた学習済み重み、学習ログ、wandb、実際のチェックポイントは読めない。`BLOG_ja.md` 内のAHC015提出リンクはAHC061とは別問題の歴史的言及であり、AHC061解法コードとしては扱わなかった

## 解法の全体像

実行時に深い探索をしない方針で、盤面状態から次に置く100マスの方策分布をニューラルネットで直接出す。学習時はC++実装のゲーム環境でPPOを回し、提出時は学習済みモデルをC++に圧縮埋め込みして、合法手マスク内の最大logitを出力する。

AIの隠れパラメータを完全に明示推定して探索するというより、推定器から得た相手の次手分布・推定パラメータを特徴量としてNNに渡し、残りの戦略判断を方策に学習させる構成である。最終付近の特徴量は `research_v4`、モデルは `dwres_ppconcat_v1` 系、推定器は `adf_beta`、提出は int8/fp16 量子化とHuffman/base91系エンコードで 512 KiB 制限に収める構成だった。

## 主要アイデア

- 100マスを行動空間とし、合法手マスクをかけたうえで最大logitのマスを選ぶ。
- 盤面を `[C,10,10]` の多チャンネル画像として表す。価値、レベル、所有者、連結成分、到達可能、スコア、距離、相手推定パラメータを平面として入れる。
- AIプレイヤーは現在スコア降順に並べ替え、常に「最も強い敵」が近いチャンネル番号になるようにする。
- 相手ごとに `adf_beta` 推定器を持ち、観測されたAIの移動から `wa, wb, wc, wd, eps` の平均推定値を更新する。
- NN本体は全体盤面を処理するmain branchに加え、`player0 + enemy p` の組を処理するplayer branchを各敵について回し、敵との相互作用特徴をmain branchへ合流させる。
- RLが伸びなくなったらモデル拡大、蒸留、学習率・entropy係数調整、EMA、提出用モデルへの再蒸留を行う。
- 提出時はTorchランタイムを使わないcompact C++推論器に落とし込み、重みをHuffman/base91系で埋め込む。

## 最終コードの構造

### 状態表現

- `State` が `value[100]`, `owner[100]`, `level[100]`, 各プレイヤー位置、`m`, `u_max`, `t_max` を持つ。
- `EnvInstance` は `State`、現在ターン、スコア、乱数、推定器、合法手・連結成分・到達可能マスクのキャッシュを持つ。
- 提出用C++では毎ターン、`State` と `AdfBetaEstimator[M_MAX]`、現在スコアを保持し、入力された観測で更新する。
- 特徴量生成では `moves[p]`, `move_cnt[p]`, `comp[p]`, `reach[p]` を一括計算し、player0の合法手をaction maskにする。

### 観測・制約・入力の扱い

- 初期入力から初期所有者・レベルを構築する。
- 各ターン、出力後に返ってくる全プレイヤーの移動先、終了位置、所有者、レベルを読む。
- AIの推定更新は、ターン開始時の状態とAIの観測移動先から `MoveSummary` を作り、`adf_beta[p].update(summary)` を呼ぶ。
- `enumerate_legal_moves` が到達可能領土と隣接候補を列挙し、合法手マスクと相手の候補手分布生成に再利用される。
- 不正手は学習環境では例外、提出側ではNN出力を合法手マスクで絞るため基本的に避ける。

### 評価関数

- 学習環境では `phi` として公式スコアに対応するスコア比の変換値を持ち、1手の報酬は `new_phi - old_phi`。
- 方策学習はPPOで、policy/valueに加えて相手次手分布や相手パラメータ推定の補助ロスを使える。
- 実行時には明示的な評価関数を計算せず、NNのpolicy logitを評価値として使う。
- 評価・提出比較では tools 固定seedの平均スコア、TTAあり/なし、量子化による劣化有無を測る。

### 探索・構築・更新

- 学習時は大量自己対戦rolloutでPPO更新を行う。READMEの最終系コマンドでは `research_v4`, `dwres_ppconcat_v1`, hidden 112, blocks 20 などが使われている。
- 相手推定の `adf_beta` は、`delta=(log(wb/wa), log(wc/wa), log(wd/wa))` を3次元ガウス、`eps` をBeta分布で近似する。
- 観測手が貪欲候補になり得る場合、カテゴリ間の不等式を半空間制約としてガウスをモーメントマッチ更新する。
- `eps` はランダム選択と貪欲選択の混合尤度を線形尤度としてBeta分布に反映する。
- NNはmain branchで盤面全体を処理し、player branchで `global + player0 block + enemy block + enemy id` を処理する。7敵分をconcatし、1x1 fuseでmainへ足す。
- 提出用推論器はgroup norm、SiLU、depthwise/pointwise conv、player branch、policy headをC++配列演算で実装する。

### 操作・クエリ・出力選択

- 毎ターンの流れは「特徴量生成 -> NN推論 -> 合法手内最大logit -> 座標出力 -> 観測入力 -> 相手推定器更新 -> 状態差し替え」である。
- `player0` の `next` 特徴は合法手一様、AIの `next` 特徴は `adf_beta` の平均パラメータでAI-like分布を計算する。
- TTAでは盤面の回転・反転で複数推論し、確率をsumまたはprodで結合する設計がある。ただし読んだcompactテンプレートでは単体推論の基本構造を中心に確認した。

### 時間配分・パラメータ

- 実行時探索は行わず、100ターンそれぞれ1回の特徴量生成とNN推論に収める。
- 学習例では `updates=1000000`, `lr=3.0e-5`, `ent-coef=5e-3`, `batch-size=512` またはmulti-GPUで大きいbatch、EMA decay複数が使われている。
- `research_v4` のチャンネル数は149。global 19ch、player block 8人×16ch、player0位置2ch。
- compact提出では hidden 112、player hidden 56、Huffman/base91またはbase122、int8/fp16系の量子化設定が使われる。
- 記事ではRunpodやvast.aiの多数GPUを使った長時間学習が説明されており、再現には大きな計算資源が必要である。

## 実装上重要な断片

```text
turn_loop:
    enumerate_legal_moves_for_all_players()
    build_research_v4_features(state, scores, reach, comp, adf_beta)
    logits = compact_model_forward(board)
    action = argmax(logits[cell] for cell if legal[cell])
    print(action)
    read_observation()
    for each AI p:
        summary = summarize_ai_observation(previous_state, p, observed_tx[p])
        adf_beta[p].update(summary)
    replace_state_with_observed_board()
```

```text
adf_beta_update(summary):
    if observed move can be greedy:
        convert "chosen category beats other categories" into halfspace constraints
        truncate Gaussian posterior of log weight ratios
    update eps Beta distribution from random-vs-greedy mixture likelihood
    mean_param = exp(mu + variance/2), eps_mean
```

```text
ppconcat_forward(board):
    x_main = main_front(stem(board))
    common = global_features + player0_block
    for enemy p in 1..7:
        y[p] = player_branch(common, enemy_block[p], enemy_id[p])
        y[p] = 0 if p is inactive
    x = x_main + fuse(main=x_main, players=concat(y))
    logits = policy_head(main_back(x))
```

## この解法の本質

探索で各ターンの最善を近似するのではなく、膨大な自己対戦で「相手推定込みの局面判断」を方策ネットワークに償却している点が本質である。AHC061は盤面が小さく、ゲームルールが完全にシミュレート可能で、相手方策も低次元の隠れパラメータで記述されるため、RLで大量に経験を作りやすい。さらに提出時の2秒制限と512 KiB制限に対して、探索を捨てて圧縮済みNN推論に寄せた判断が効いている。

もう一つの重要点は、完全なend-to-endではなく、相手推定器・次手分布・距離・連結成分など、人間が設計した特徴をNNへ渡していることだ。学習に任せる範囲と構造化して渡す範囲の切り分けが強い。

## 真似するならまず実装する部分

完全再現は現実的ではない。まずはC++の正確なシミュレータ、合法手列挙、相手行動サマリ、簡易な `adf_beta` 風パラメータ推定器を作るのが先である。そのうえで、NNではなく手書き評価やモンテカルロに推定結果を入れる方が理解用には近道である。

RLルートを試すなら、提出圧縮より前に、ローカル環境で `State -> feature -> policy/value -> rollout -> PPO update` が回る最小構成を作るべきである。最初から `research_v4` やppconcatを再現するより、`submit_v1` 相当の46ch特徴量と小さいResNetで挙動を確認する方が壊れにくい。

## 注意点・未理解点

- `SOLUTION_ja.md` はリンクだけ存在し、実体は404だった。
- 最終提出 `main.cpp` は単一ファイル化・圧縮済みで、重みの数値や学習済み方策の意味は読めない。
- `BLOG_ja.md` 内のAHC015提出リンクはAHC061解法ではないため、AHC061の精読対象からは外した。
- PPO更新、GAE、rolloutバッファなどの学習コードは大枠のみ確認した。細かいハイパーパラメータ探索の履歴は全て追っていない。
- 記事からは最終提出に使った厳密なチェックポイント名、TTA設定、EMA選択は断定できない。
- 再現には大量GPU、実験管理、提出サイズ圧縮、量子化誤差検証が必要で、通常のAHC実装よりかなり重い。
