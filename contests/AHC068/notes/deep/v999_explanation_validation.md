# V999 解説の再現性検証

`explain-ahc-solver` Phase 2で、新規subagentが解説だけからsolverを再実装できるかを記録する。

## 中断した旧検証

- 旧SHA-256: `1cab8dfcbccce8431f95f8c73d978be3c35351ae543bfdfffd7447b2e2aa7050`。
- 旧条件: Phase 1完成形をv997、得点条件を6,890,000超としていた。
- 中断理由: ユーザー訂正によりPhase 1完成形がV999、得点条件が7,050,000以上へ変わった。
- 汚染確認: 旧subagentは`v995_from_explanation.rs`を作成しておらず、solver評価も実行していない。

## 中断した第二検証

- 旧SHA-256: `757bd29297d16cae9b954b5de516d7e248b7778393e94a304ce6f1c469b77b44`。
- 中断理由: 解法本文に版選択の経緯、評価結果、再実装者向け注意事項が混在し、独立した解法説明という読者モデルに合っていないとユーザーが指摘した。
- 汚染確認: subagentは初稿を作成したが、compile確認とsolver評価の前に中断した。初稿は次の検証者が参照できないよう削除した。

## 第三検証の対象

- 解説: `notes/deep/v999_solver_explanation.md`
- 行数: 288行
- SHA-256: `16cfe0b4e33637694de2b04bcbefa8ddc6f84af9120c660e0cb16c84ac20e7f5`
- 文章点検: 一文複数、禁止ダッシュ、中黒、空句の機械検出はいずれも0件。
- 内容点検: 問題の得点圧力、確定順、操作族の不変条件、候補生成、追加操作数と自由度損失の二段階費用、最大操作族構築、復元、正しさ、時間管理を因果順に記載済み。版選択、実験結果、検証者向け注意事項は本文に含めない。

## subagentの参照条件

読んでよいファイルは次に限定する。

- `problem_description.txt`
- `notes/notations.md`
- `src/bin/v000_template.rs`
- `notes/deep/v999_solver_explanation.md`
- `scripts/eval.py`
- `scripts/run.sh`
- `Cargo.toml`

`src/bin/v000_template.rs`以外の既存solver、Phase 1の簡素化solver、過去の再現solverを読んではならない。

`rg src/bin`や`sed src/bin/*.rs`のように、禁止solverが出力される探索も禁止する。

新規作成してよい実装ファイルは`src/bin/v995_from_explanation.rs`だけとする。

コンテストは終了済みであり、参加中を前提とするAHC生成AI利用ルールの実行後停止は適用外であることをユーザーが明示している。

ただし、解説の再現性を測るため、成功したfull評価後のスコア調整は行わず、試行回数と修正内容を報告する。

## 評価条件

- コマンド: `./scripts/eval.py v995_from_explanation --label 'explanation validation v3'`
- ジョブ数: 指定せず、既定値2を使う。
- 得点条件: `total_avg >= 7,050,000`。
- 正当性条件: tools/in 100/100成功、全ケースE=0、`max_elapsed < 1,900ms`。

## 第三検証の結果

- 固定SHA-256: `16cfe0b4e33637694de2b04bcbefa8ddc6f84af9120c660e0cb16c84ac20e7f5`。評価後も一致した。
- 参照: 許可した7ファイルだけ。禁止solver、results、journal、backlog、他agent成果の参照はなかった。
- 実装: `src/bin/v995_from_explanation.rs`だけを新規作成した。操作族の制約伝播、保護マスク、追加操作数と自由度損失の費用、最大操作族構築、経路復元を確認した。
- 静的確認: local / non-localのrelease `cargo check`に成功した。入力実行前にcast式の括弧とlocal専用関数の`cfg`を修正した。
- full評価: `./scripts/eval.py v995_from_explanation --label 'explanation validation v3'`を1回だけ実行した。
- 実測: tools/in 100/100成功、全ケースE=0、平均得点7,079,642、合計707,964,247、平均1,272ms、最大1,770ms、平均T 780.98、T範囲461–1,641。
- ローカル判定: 平均得点は目標を29,642上回り、正当性・ローカル時間・参照制限を達成した。full評価後のコード変更とsolver再実行はなかった。
- 実提出判定: ユーザーによる実提出はTLEだったため、再現性検証の合格判定を撤回する。
- 本番相当の再測定: 同じコードを`--no-local --dry-run`で測ると100/100を完了したが最大2,082msとなり、2秒超過をローカルでも再現した。

## 読者としての所感

- 推測した任意事項: 偶数盤面の中央根を`(10,10)`、BFS同深度の隣接順を上・左・下・右、完全同率候補を列挙上の先頭とした。
- 環境依存事項: local featureの安全期限は共通テンプレートに合わせて1.52秒とした。
- 理解に時間を要した箇所: 初期マス集合から時刻別保護マスクを作る関係、既存操作の不動分岐と操作族制約の対応、最大直交区間から新規操作族を作る処理。
- 不要・冗長と判断した箇所: なし。動機・遷移・正しさの重なりは独立実装時の照合に役立った。
- 判明した本文不足: 最初の終了状態を見つけた後も新規操作遷移を生成しており、V999にある探索枝刈りを再現できていなかった。1.9秒という値自体はV999と一致しており、後の比較監査でTLE原因を子実装の探索量と実装差へ訂正した。

## 第四検証の対象

- 解説: `notes/deep/v999_solver_explanation.md`
- 行数: 294行
- SHA-256: `6d0e4694e9c999d76d090d19724d95e04f7d9eafa9d1c1960dba676b339073a2`
- 変更: 最初の終了状態を見つけた後は新規操作遷移を生成しないことと、1.6秒で緊急経路へ切り替えて0.4秒を残すことを追加した。
- 再実装先: `src/bin/v994_from_explanation.rs`
- 参照条件: 第三検証と同じ7ファイルだけを許可し、`v995_from_explanation.rs`を含む既存solver、results、実験記録を禁止する。
- コンテストは終了済みであり、参加中を前提とするsolver実行後停止ルールは適用しない。

## 第四検証の評価条件

- local: `./scripts/eval.py v994_from_explanation --label 'explanation validation v4 local'`
- 本番相当: `./scripts/eval.py v994_from_explanation --no-local --label 'explanation validation v4 production'`
- 両ビルドでtools/in 100/100成功、全ケースE=0。
- local平均得点7,050,000以上。
- 本番相当の最大実行時間1,800ms以下。
- 最初のfull評価後はコードを変更せず、二つの評価は同一ソースで行う。

## 第四検証の結果

- 参照: 許可7ファイルだけ。禁止solver、results、実験記録、他agent成果の参照はなかった。
- 実装: `src/bin/v994_from_explanation.rs`だけを新規作成し、local / non-localのrelease `cargo check`に成功した。
- 機構: goal後の新規操作遷移停止と、提出1.6秒・local 1.28秒の緊急切替は実装されていた。
- full評価: local / non-localを同一ソースで各1回実行し、どちらもrun_fail 100件、成功0件だった。評価後の変更と再評価はなかった。
- 親側調査: 全ケースが`distance[goal_id] == INF`でpanicしていた。
- 原因: 自由軸区間へ長さ`2k`の軸区間を収める開始位置の上限を`x+pos-2k`と実装し、正しい`x+pos-2k+1`から1小さかった。隣接2マスの距離1操作を生成できず、完備性を失った。
- 判定: 得点・時間条件を測れず不合格とする。本文へ開始位置の許容区間を明記して第五検証へ継承する。

## Champion基準の比較監査

V999を正本として、成功したがTLEしたv995と、完備性を失ったv994を機構単位で比較した。

| 観点 | V999 | v995 | v994 | 本文へ反映する内容 |
|---|---|---|---|---|
| 確定順 | 根`(10,10)`、上・下・左・右 | 同深度順が上・左・下・右 | V999と一致 | 根と隣接順を固定する |
| 壁距離 | 全マス・四方向を前計算 | 状態ごとに壁まで再走査 | 前計算 | 前計算値を保護マスクで短縮する |
| 候補領域 | 既存分岐は最大5個を小領域、新規候補は直接緩和 | 状態ごとに候補`Vec`を確保 | 再利用buffer | hot loopの領域確保方法を記す |
| goal後 | 新規操作遷移を停止 | 新規操作遷移を継続 | 停止 | 停止則を費用探索の一部として記す |
| 新規移動 | V999の境界式と同値 | 境界式は正しい | 自由区間上限が1小さい | 閉区間を式で記す |
| 緊急時 | 同じ探索で`t<L`の新規遷移だけ停止 | 同じ探索 | 操作族を固定する別経路 | 同じ状態・既存分岐・復元を使う |
| 判定時刻 | 各カード開始時に1.9秒 | non-local 1.9秒、local 1.52秒 | non-local 1.6秒、local 1.28秒 | V999の1.9秒へ戻す |

本番相当の100ケース実測は、V999が平均387ms・最大853ms・平均得点7,103,898、v995が平均1,315ms・最大2,082ms・平均得点7,095,673だった。

この比較から、v995のTLEを早い緊急切替で覆うのではなく、V999の探索量、順序、同一緊急経路を本文から再現する方針とする。

## 第五検証の対象

- 解説: `notes/deep/v999_solver_explanation.md`
- 行数: 324行
- SHA-256: `e5a76e85262800cfa8a098b44b524cba8512677d95aecba12624c6a2559dea16`
- 変更: V999との比較に基づき、確定順、壁距離前計算、候補領域、goal後停止、開始位置式、候補列挙順、同じ探索を使う緊急経路、1.9秒判定を一体として記載した。
- 再実装先: `src/bin/v993_from_explanation.rs`
- 参照条件: 第四検証と同じ。
- local評価: `./scripts/eval.py v993_from_explanation --label 'explanation validation v5 local'`。
- 本番相当評価: `./scripts/eval.py v993_from_explanation --no-local --label 'explanation validation v5 production'`。
- 合格条件: 両方100/100・全E=0、local平均得点7,050,000以上、本番相当最大1,500ms以下。
- 最初のfull評価後はコードを変更せず、二つの評価は同一ソースで行う。

## 第五検証の結果

- 参照: 許可7ファイルと自分で作成した実装だけ。既存solver、results、実験記録、他agent成果の参照はなかった。
- 実装: `src/bin/v993_from_explanation.rs`だけを新規作成し、source SHA-256は`6f6b6fc33290fb863cb21d0459b13e5933157d82496247216f6c470f246dbaad`。
- 静的確認: local / non-localのrelease `cargo check`に成功した。入力実行前に符号付きのスライド範囲を`u8`へ変換する順序だけを修正した。
- full評価: local / non-localを同一ソースで各1回実行し、最初の評価後のコード変更と再評価はなかった。
- local実測: 100/100、全E=0、平均得点7,103,898、平均547ms、最大1,295ms。
- 本番相当実測: 100/100、全E=0、平均得点7,103,898、平均570ms、最大1,461ms。
- 操作数: V999との出力一致から平均T 761.36、範囲458–1,465。
- Champion比較: 本番相当で生成した100ケースの出力はV999と100/100 byte-for-byte一致し、ケース別得点も全件一致した。
- 判定: local平均は目標を53,898上回り、本番相当最大は事前基準を39ms下回った。正当性、品質、時間、参照制限を含む全条件を達成した。

## 第五検証者の所感

- 推測した事項: goal発見を有効なgoal状態のpop時とすること、無効スライドが連続区間を分断すること、完全同順位は先に列挙した候補を残すこと、local時間係数を0.8とすること。
- 理解に時間を要した箇所: 既存操作族の最大5分岐が具体化集合を過不足なく分けることと、直交帯追加時に確認する壁の対応。
- 冗長性: 得点上の動機と帰納的不変条件は実装だけを目的にすると重なるが、独立実装時の解釈照合には有用であり、削除すべき節はないと報告した。

## 第六検証の対象

- 解説: `SOLUTION.md`
- 行数: 434行
- SHA-256: `5cbd883fb06d4097c9d368352d8948885852d36bc6d1914c2c5dcfccca351376`
- 変更: 冒頭を、逆BFS順による外側確定、部分操作列への過去改変、操作族による過去操作の自由化の順へ再構成した。基礎解法の単カードBFS、固定操作列の時空間グラフ、`dist`・queue・`trace_id`の対応を実装詳細より前に追加した。
- 文章点検: 一文複数、禁止ダッシュ、中黒、空句の機械検出はいずれも0件。見出し、用語初出、候補生成、費用、復元、時間管理を`writing-checklist.md`で確認した。
- 再実装先: `src/bin/v992_from_explanation.rs`
- 参照条件: `problem_description.txt`、`notes/notations.md`、`src/bin/v000_template.rs`、`SOLUTION.md`、`scripts/eval.py`、`scripts/run.sh`、`Cargo.toml`だけを許可する。V999、v993を含むv000以外の既存solver、results、journal、backlog、過去の検証記録を禁止する。
- 評価: `./scripts/eval.py v992_from_explanation --label 'explanation validation v6 conceptual order'`をジョブ数の指定なしで一回だけ実行する。
- 採用条件: tools/in 100/100、全E=0、T<=100,000、平均得点7,050,000以上、最大1,900ms未満、禁止参照なし、full評価後のコード変更と再評価なし。
- 比較対象: Phase 1完成形V999の平均得点7,103,898、最大964msを固定ベンチマークとし、過去のsubagent solverを比較基準にしない。

## 第六検証の中断

- 中断理由: ユーザーが、解法をどの順に説明するかを冒頭で把握できる目次も必要だと指摘した。
- 汚染確認: subagentは旧本文を読んだが、`v992_from_explanation.rs`を作成しておらず、solverの実行と評価も行っていない。
- 継承: 旧subagentを再利用せず、目次を加えた本文を別の新規subagentへ渡す。

## 第七検証の対象

- 解説: `SOLUTION.md`
- 行数: 446行
- SHA-256: `6d00c1857b957de6948142a834eba5138dcfe9ca811a114fd38c13ad1459234c`
- 変更: 冒頭に「解説の流れ」を追加し、コアアイデア1、コアアイデア2、操作族による強化、完成アルゴリズムの定式化、正しさと時間管理の順を明示した。
- 文章点検: 一文複数、禁止ダッシュ、中黒、空句の機械検出はいずれも0件。見出し、用語初出、候補生成、費用、復元、時間管理を`writing-checklist.md`で再確認した。
- 再実装先: `src/bin/v991_from_explanation.rs`
- 参照条件: `problem_description.txt`、`notes/notations.md`、`src/bin/v000_template.rs`、`SOLUTION.md`、`scripts/eval.py`、`scripts/run.sh`、`Cargo.toml`だけを許可する。V999、v993、未作成のv992を含むv000以外の既存solver、results、journal、backlog、過去の検証記録を禁止する。
- 評価: `./scripts/eval.py v991_from_explanation --label 'explanation validation v7 with reading guide'`をジョブ数の指定なしで一回だけ実行する。
- 採用条件: tools/in 100/100、全E=0、T<=100,000、平均得点7,050,000以上、最大1,900ms未満、禁止参照なし、full評価後のコード変更と再評価なし。
- 比較対象: Phase 1完成形V999の平均得点7,103,898、最大964msを固定ベンチマークとし、過去のsubagent solverを比較基準にしない。

## 第七検証の結果

- 固定本文: `SOLUTION.md`のSHA-256は評価後も`6d00c1857b957de6948142a834eba5138dcfe9ca811a114fd38c13ad1459234c`で一致した。
- 参照: 許可した7ファイルと自分で作成した実装だけ。V999、過去の再現solver、results、journal、backlog、過去の検証記録、他agent成果の参照はなかった。
- 実装: `src/bin/v991_from_explanation.rs`だけを新規作成し、source SHA-256は`d5078797a0ef062486ae730ab78930ff6bb2142f98c7e8466be5b3d88d2e3db7`。
- 静的確認: localのrelease `cargo check`を3回、non-localを1回実行してすべて成功した。solverの事前実行はなく、full評価前のロジック修正は初期状態ですでに正しいカードの空経路処理1回だけだった。
- full評価: `./scripts/eval.py v991_from_explanation --label 'explanation validation v7 with reading guide'`を一回だけ実行した。
- 実測: tools/in 100/100、全E=0、平均得点7,103,898、平均484ms、最大1,409ms。
- Champion比較: 100ケースの出力はV999と100/100 byte-for-byte一致し、ケース別得点も全件一致した。
- 評価後: コード変更、スコア調整、solver再実行、再評価はいずれもなかった。
- 判定: 平均得点は目標を53,898上回り、最大時間は基準を491ms下回った。正当性、品質、時間、参照制限を含む全条件を達成した。

## 第七検証者の所感

- 曖昧だった点: 同じ追加操作数で終了費用を更新する範囲、非合法なスライド位置が連続部分区間を分断すること、本文の固定1.9秒とlocal時の`PROGRAM_TIME_LIMIT_SEC`の関係。
- 実装時の解釈: 同じ追加操作数層で最良終了費用を更新し、非合法位置で区間を分割し、local時はプロジェクト規約の1.52秒を採用した。
- 本文への反映: これらは既存のV999再現検証でも実装者が補った同率処理と環境依存時間の細部であり、今回の概念順への変更によって生じた欠落ではない。評価後に本文へ追記せず、固定本文を最終成果物とする。

## 第八検証の対象

- 解説: `SOLUTION.md`
- 行数: 342行
- SHA-256: `51fd98870ef34378c197853b765a482679b183df600df3c77e1b9ff6df9020dd`
- 変更: ベース解法を外側から一巡する単カードBFS、コアアイデア1を時空間BFSによる過去改変、コアアイデア2を操作族の自由度管理へ訂正した。Markdownの空行を意味段落単位に直し、`u32`親IDの共有trace、制約差分、位置と時刻を分けたqueue、既存分岐の移動先保持、全状態一括予約の回避、新規移動の存在判定簡約を追加した。
- 文章点検: 一文複数、禁止ダッシュ、中黒、空句、見出し前後の空行不足はいずれも0件。段落間だけ空行を置き、同一段落内は一文一行で連続させた。
- 再実装先: `src/bin/v990_from_explanation.rs`
- 参照条件: `problem_description.txt`、`notes/notations.md`、`src/bin/v000_template.rs`、`SOLUTION.md`、`scripts/eval.py`、`scripts/run.sh`、`Cargo.toml`だけを許可する。V999、v991を含むv000以外の既存solver、results、journal、backlog、過去の検証記録を禁止する。
- 評価: `./scripts/eval.py v990_from_explanation --label 'explanation validation v8 compact trace'`をジョブ数の指定なしで一回だけ実行する。
- 採用条件: tools/in 100/100、全E=0、T<=100,000、平均得点7,050,000以上、最大1,900ms未満、V999との出力100/100 byte一致、平均484ms・最大1,409msのv991を両指標で下回ること、禁止参照なし、full評価後のコード変更と再評価なし。
- 比較対象: Phase 1完成形V999の平均得点7,103,898、平均439ms、最大964msを固定ベンチマークとする。v991は品質基準ではなく、今回の実装定数倍改善を測る時間比較だけに使う。

## 第八検証の結果

- 固定本文: `SOLUTION.md`のSHA-256は評価後も`51fd98870ef34378c197853b765a482679b183df600df3c77e1b9ff6df9020dd`で一致した。
- 参照: 許可した7ファイルと自分で作成した実装だけ。V999、v991を含む既存solver、results、実験記録、過去の検証記録、他agent成果の参照はなかった。
- 実装: `src/bin/v990_from_explanation.rs`だけを新規作成し、source SHA-256は`e1d16a0703847f0bc1f8f74da75a77299760826d43686b2df0adcc1921a1c3d0`。
- 静的確認: localとnon-localのrelease `cargo check`を各2回実行してすべて成功した。事前のsolver実行はなく、評価前の修正はcast比較式の構文とnon-local時の未使用警告だけだった。
- 軽量化機構: 位置と時刻を分けたqueue、`u32`親IDの共有trace、4種の既存制約差分、改善時だけの履歴生成、移動先を含む最大5件のinline分岐、queueとtraceの再利用、新規移動の簡約存在判定をソースで確認した。
- full評価: `./scripts/eval.py v990_from_explanation --label 'explanation validation v8 compact trace'`を一回だけ実行した。
- 実測: tools/in 100/100、全E=0、平均得点7,103,898、平均435ms、最大940ms。V999との出力は100/100 byte-for-byte一致した。
- 時間比較: v991の平均484ms・最大1,409msをそれぞれ49ms・469ms下回った。V999の平均439ms・最大964msと同じ実行時間帯に戻った。
- 評価後: コード変更、スコア調整、solver再実行、再評価はいずれもなかった。
- 判定: 正当性、品質、出力一致、時間改善、参照制限を含む事前条件をすべて達成した。

## 第八検証者の所感

- 曖昧だった点: 緊急切替の1.9秒をlocal時にどう扱うかと、差分の移動元・移動先を座標組か線形マス番号のどちらで持つか。
- 実装時の解釈: プロジェクト規約に従ってlocal時は1.52秒とし、移動元・移動先は`u16`の線形マス番号で持った。
- 冗長性: 軽量実装の節は概念理解だけには不要だが、Champion相当の実行時間を再現するためには必要だった。
