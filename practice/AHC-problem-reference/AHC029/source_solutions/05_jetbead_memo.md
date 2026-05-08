# AHC029 - jetbead_memo 解法メモ

## 参照元

- 記事: [AHC029(RECRUIT)](https://jetbead.github.io/AtCoderHeuristicContestMemo/ContestMemo/ahc029.html)
- 著者: jetbead
- サイト: AtCoder Heuristic Contest Memo
- 種別: 詳細メモ、解法整理、リンク集
- 成績・順位: 著者自身の順位解説ではなく、複数参照元の整理
- コード有無: 本文自体に最終コードなし。リンク先に chokudai 氏のGitHub、Moegi 氏のAtCoder提出リンクがある。追加でAtCoder順位表から chokudai 最終提出 [#48867069](https://atcoder.jp/contests/ahc029/submissions/48867069) と Moegi 最終提出 [#48866974](https://atcoder.jp/contests/ahc029/submissions/48866974) を確認できた
- コードを読めたか: 読めた。chokudai 氏GitHubの `Solver/Program.cs` / `Tester/dialy.md` に加え、Browserで chokudai #48867069 と Moegi #48866974 を読んだ。Moegi の記事内リンク #48863724 はCEの日記文だが、最終AC提出 #48866974 は実行可能コードだった
- 読めなかったもの: tsukammo 氏の HackMD は Browser でも 403 Forbidden で読めなかった。多数の Twitter リンク、YouTube、ChatGPT share はコード・提出・GitHub・gistではないため精読対象外とした

## 解法の全体像

この参照元は単一の最終解法ではなく、AHC029の見方と複数解法の整理である。主張の中心は、ルールベースでも一定の成績を狙えるが、上位では評価関数と短い先読み、モンテカルロ的な探索が重要だったというものだ。

問題を「お金 -> カード -> プロジェクト進捗 -> 報酬 -> お金」の循環として見て、各途中状態をどのように金額換算するかが評価関数設計の中心になる。リンク先の chokudai 氏コードは、この考えを状態価値・行動価値・モンテカルロプレイアウトとして具体化した大規模なC#実装である。

## 主要アイデア

- できるだけ増資回数 `L` を増やし、かつ十分な所持金で終える。
- 2秒で1000ターンなので、1ターンに使える時間は約0.002秒しかない。
- ルールベースでは、安いカードを買う、優先カードを使う、効率の良いカードやプロジェクトを選ぶなどをif文で書く。
- 探索ベースでは、状態または行動の評価関数を作り、1手貪欲、数ターン貪欲プレイアウト、モンテカルロなどで選ぶ。
- 買うカード、使うカード、対象プロジェクトの組合せは多いため、対象プロジェクトだけ貪欲に絞るなどの削減が有効。
- 評価単位を所持金に揃え、カードやプロジェクトも「何円相当か」で考える。
- 金欠状態は、良い補充カードを買えないため明確に悪い状態として扱う。
- 相対スコアでは、絶対スコアの合計だけでなく、`log2` やケースごとの勝敗で見る必要がある。

## 最終コードの構造

本文自身のコードはないため、リンク先の chokudai 氏 `Solver/Program.cs` と、Browserで読めたAtCoder最終提出 #48867069 を中心にまとめる。Moegi 氏の最終AC提出 #48866974 は、ルールベース寄りの別実装として補助的に扱う。

### 状態表現

- `Field`
  - `N, M, K, T`
  - ローカル用の未来プロジェクト `pps` と未来カード `pcs`
  - カード種類の推定 `XGuess = [21, 11, 11, 6, 4]`
  - 乱数とタイマー
- `State`
  - `ps`: プロジェクト配列。
  - `damage`: 各プロジェクトに与えた累積労働量。`Project.HP` は固定し、残務は `HP - damage` で見る。
  - `cs`: 手札。
  - `money`, `Turn`, `L`
  - `PreUse`: 直前に使用して補充待ちの手札位置。
  - `UpdateProjects`: 完了・キャンセルで差し替えが必要なプロジェクト一覧。
  - `UsedProject`: ローカル未来プロジェクトを消費した数。
- `PreProject`, `Project`, `Card` は生成前スケールと増資後スケールを分けるための軽量クラス。
- AtCoder最終提出 #48867069 でも `Field`, `PreProject`, `Project`, `Card`, `State`, `Solver` の構造が確認できる。
- Moegi 最終提出 #48866974 は `score_calculator`, `target_project`, `cancel_target`, `choice_use_card`, `choice_new_card` を中心にした軽量な貪欲実装である。

### 観測・制約・入力の扱い

- ジャッジ入力では、初期手札、初期プロジェクトを読み、各ターンで補充候補を読み取る。
- ローカルでは全未来プロジェクト・全未来カードを先に読み、同じ `State` 更新関数で再生する。
- カード補充候補の1番目を除き、種類を `F.AddX` で観測分布に加える。
- `GetCardListWithPCS` で未来カードを現在の `L` に応じて `<< L` してスケールする。
- `Simulate` は労働、キャンセル、増資を状態に適用し、`Update` で新プロジェクトを補充する。

### 評価関数

chokudai 氏コードでは、状態価値を大きく以下に分ける。

- `GetMoneyAndLevelValue(money, L, remaining_turns)`
  - 所持金に約1.05倍の価値を置く。
  - 平均増資間隔 `AverageLevel` と残りターンから、現在レベルで得られる将来収益を見積もる。
  - 増資カード出現率の推定から補正 `fix` を入れる。
- `GetCardValue(card)`
  - 通常労働は `work` に近い価値。
  - 全力労働は `work * (0.1 + 0.9*M - 0.045*M*(M-1))` のように、Mが大きいほど過剰・効率低下を割り引く。
  - キャンセル系と増資カードの手札価値は基本0で、使用時や状態側で評価する。
- `GetProjectValue(V, HP, L)`
  - 完了済みなら報酬を高く評価。
  - 未完了なら `(V - HP)` に `1 - 0.25*HP/V` のような必要度を掛け、さらに `-300*2^L` の基礎ペナルティを置く。
- `GetAshibumiValue`
  - 金・通常労働・全力労働を資産として見て、一定水準に届かず足踏みしそうな状態を罰する。

### 探索・構築・更新

- `choose(State S, candidates)` で、補充候補 `buy` と使用カード・対象 `use` の組を列挙する。
- 同じ種類・同じ労働力の手札カードは重複候補から除く。
- 特殊カードは同種最安だけを候補にする。
- 補充候補が増資カードなら、増資を使わない候補を制限するなど、候補数を削る。
- まず `Eval(NS2)` で1手後の貪欲スコアを作り、上位候補を並べる。
- 時間残量に応じて、候補数 `Target`、試行数 `CheckNum`、先読み深さ `CheckTurn` を変える。
- 未来カードは `MakeCL` で推定分布から生成し、未来プロジェクトは必要になったら生成する。
- 各候補を同じ未来生成条件で進め、内部では簡易貪欲で `CheckTurn` 手進める。
- プレイアウト結果は `log(max(10, Eval(now)))` を足し、期待値というより対数評価の和で比較する。
- 一定試行後に上位2候補だけ残し、後続試行を集中する。

### 操作・クエリ・出力選択

- chokudai 氏コードは、思考上は `buy` と `use` を同時に選ぶ。
- 実際の出力では、初手以外は前ターンの補充候補から選んだ `buy` を先に出力し、次ターン使用する `use` を出すように、問題文順をずらして管理する。
- 初手だけは補充候補がないため、使用カードだけ選ぶ。
- 最終ターン後は無料カード `0` を出す。

### 時間配分・パラメータ

- 1800msを超えたら1手評価の貪欲に落とす。
- 1600ms以降は候補2、試行2、深さ5程度まで縮める。
- 時間に余裕があると候補50、試行300、深さ13まで増やす設定がある。
- `AverageLevel` は `220 / K / sqrt(M)` を基礎に、現在ターンとレベルから補正する。
- `XGuess` 初期値は `[21, 11, 11, 6, 4]`。

## 実装上重要な断片

```text
choose(state, cards_to_buy):
    first_scores = []
    for buy in candidates:
        if impossible_or_dominated(buy): continue
        state1 = apply_buy_if_needed(state, buy)
        for use in unique_hand_cards(state1):
            for target in targets(use):
                state2 = simulate_one_action(state1, use, target)
                first_scores.append((Eval(state2), buy, use, target, state2))
    pool = top_by_score(first_scores, Target)
    return monte_carlo_compare(pool)
```

```text
monte_carlo_compare(pool):
    for sample in 0 .. CheckNum*3:
        future_cards = MakeCL(observed_type_counts, CheckTurn)
        future_projects = shared_generated_projects()
        for candidate in active_pool:
            s = candidate.state.copy()
            for depth in CheckTurn:
                s.update_projects(future_projects)
                action = greedy_by_delta_eval(s, future_cards[depth])
                s.apply(action)
            score[candidate] += log(max(10, Eval(s)))
        after CheckNum samples:
            keep top 2
```

## この解法の本質

Jetbeadメモの本質は、AHC029を「価値の単位を揃えた状態評価問題」として整理している点である。chokudai氏コードを読むと、その整理が、状態価値、候補枝刈り、時間に応じたモンテカルロ比較、対数評価によるばらつき抑制として実装されていることが分かる。

## 真似するならまず実装する部分

最初に実装するなら、chokudai氏コード全体ではなく、`Eval = 所持金価値 + カード価値 + プロジェクト価値` の1手貪欲から始めるのがよい。次に、補充カードと使用カードをまとめて候補化し、上位数候補だけ同じ未来列で5ターン程度プレイアウトする。

## 注意点・未理解点

- Jetbead本文はリンク集も兼ねており、全リンクを精読対象にすると範囲が膨大になる。今回は本文の解法整理と、コード・提出・GitHubに該当するリンク先を優先した。
- tsukammo氏のHackMDは403で読めなかった。
- Moegi氏の AtCoder 提出リンクは、提出本文が日記でコンパイルエラーになっており、実行可能コードではなかった。ただし、疑似コード・方針メモとしては読めた。
- chokudai氏GitHubは最終版に近い実装と長い日記を読めたが、どのバージョンが最終提出そのものかはリポジトリだけでは断定できない。
