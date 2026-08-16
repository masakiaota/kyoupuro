# Heuristic Contest Agent Notes

## AtCoder Heuristic Contest 生成AI利用ルール（最優先）
本セクションは AGENTS.md 内の他のどの記述にも優先する。ルール全文を確認する必要が生じた場合は、下記 URL ではなく相対パスの `./ahc-llm-rules-en.txt` を参照する。

```
I am currently participating in an AtCoder Heuristic Contest, and I will use this generative AI to assist in developing my solution.

When using this generative AI, the "AtCoder Heuristic Contest Generative AI Usage Rules - Version 20250616" apply.

https://info.atcoder.jp/entry/ahc-llm-rules-en

Most importantly, after running the solution program, you must not modify or improve the solution, its approach, or its code based on the execution results unless the user gives a new explicit instruction to do so.

You may run the solution program and report its execution results, logs, scores, or other observations. After reporting them, you must stop and wait for a new instruction from the user before making any improvement based on those results.

Here, "solution program" refers to any program created or being created for the purpose of solving this contest problem, regardless of whether it was created by the user or by generative AI, and regardless of whether it is still in progress or already complete.
```

### 運用上の帰結
- solution program（`src/bin/*.rs` の solver をはじめ、コンテスト解決を目的とする一切のプログラム）を実行した後は、その実行結果を根拠とするコード改変、アプローチ変更、パラメータ調整、再実行を行わない。ユーザーの新しい明示指示があるまで停止する。
- 実行結果・ログ・スコア・観察の報告と、`notes/journal.md`・`notes/backlog.md`・`results/` 以下のログ CSV への記録は「報告」の一部であり、行ってよい。
- 事前登録した採否基準に対する機械的な照合（達成/未達の事実記述）は報告に含めてよいが、その照合に続く採否判断、次実験の着手、コードの修正はユーザーの明示指示を待つ。
- `scripts/eval.py`・`scripts/run.sh`・`cargo run` などで solver を起動した時点で本ルールが発動する。機構確認目的の実行も対象である。
- solver 以外の補助（`adhoc/` の bench / probe / check、visualizer、`tools` 側の gen / vis / score など、解答そのものを生み出す目的でないプログラム）はこのルールの対象外である。

## 前提
- 運用仕様の正本はこの AGENTS.md である。README.md は人間向けの概要とコマンド例であり、仕様が食い違ったら AGENTS.md を優先し、README.md を直す。
- このディレクトリが project root である。親や兄弟ディレクトリには依存しない。
- 言語は Rust のみである。
- AtCoder のジャッジ環境を前提にし、現在の依存環境以外は用いない。

## 評価運用ルール
- `scripts/run.sh` は単発の手動実行専用であり、solver を既定で `--release --features local` 付きで build する。input file を指定した場合は tools の公式 tester を介してインタラクティブ実行し、solver の出力ログを tools の score で検証する。input file を省略した場合だけ stdin と stdout を solver へ直接つなぐ。ローカルでの実行確認や評価は原則 `local` feature 付きで行い、`--no-local` は本番相当の挙動や `local` feature なしでの compile 確認に限って使う。
- `scripts/eval.py` は評価パイプライン本体である。solver は既定で `--release --features local`、tools の tester と score は通常の `--release` で build する。公式 tester がグループ情報を1件ずつ渡し、各ターンの出力を受け取ってから次の入力へ進む。先頭入力で `tester -> score` を1回ウォームアップしてから、本番の `tester -> score` をケース単位で実行する。ウォームアップ結果は保存・集計しない。既定は `-j 2` であり、ユーザーの明示的な指定がない限りジョブ数は変更しない。ローカルでの評価や time sensitive なチューニングは原則 `local` feature 付きで行い、`--no-local` は本番相当の挙動や `local` feature なしでの compile 確認に限って使う。
- `eval.py` の `elapsed` は、`scripts/measure_solver_cpu.py` が取得したsolverプロセスの user CPU 時間と system CPU 時間の合計である。tester、score、入力準備、solver の入力待ち時間は含めない。インタラクティブ処理の停止を検出する実時間timeoutは安全装置であり、`elapsed` には使わない。
- solver 内で時間制限や打ち切り判定を実装する場合は、`v000_template.rs` の `JUDGE_TIME_LIMIT_SEC`、`LOCAL_TIME_RATIO`、`PROGRAM_TIME_LIMIT_SEC` を使う。`local` feature 時だけ `LOCAL_TIME_RATIO` を掛けた時間で探索を打ち切り、timer は `main` 開始直後を基準に作る。フェーズ切替や終了前処理などの時間系パラメータは、秒数を直書きせず `PROGRAM_TIME_LIMIT_SEC` に対する割合で指定する。
- `results/out/<bin_name>/` は最新評価の scratch/workspace である。`eval.py` 実行時に同名 basename の出力が並ぶ前提なので、重複 basename は拒否する。
- `results/score_summary.csv` は評価要約ログである。列順は `bin,total_avg,total_sum,total_min,total_max,avg_elapsed,max_elapsed,eval_set,total_cases,label,executed_at` で、経過時間は整数 ms で記録する。全ケース成功時のみ追記する。
- `results/score_detail.csv` は `tools/in` 専用の wide-format 比較表である。列順は `bin,total_avg,max_elapsed,<case_name_1>,...,label,executed_at` で、全ケース成功時のみ追記する。
- `results/eval_records.jsonl` は 1 行 1 case の正本ログである。失敗ケースも含めて追記する。
- `--dry-run` は `results/score_summary.csv`、`results/score_detail.csv`、`results/eval_records.jsonl` を更新しない。
- verbose は進捗表示だけに使い、追加ログを恒久保存しない。

## 各ディレクトリ・ファイルの役割
- `problem_description.txt`
  - 問題文、制約、スコア、初動メモの保存先である。
- `.agents/skills/write-problem-description/SKILL.md`
  - problem_description 作成時に AI が従う手順である。貼り付けテキストやスクリーンショットから公式の節順を保って転記する。
- `src/bin/*.rs`
  - top-level は `v000_template.rs`、提出候補 solver、`crate_check.rs` (依存一覧の検査器、固定) だけを置く。各ファイルは単体で完結し、1 行目に `// <file_name>.rs` を置く。
  - `v000_template.rs` と提出候補 solver は、1 行目のファイル名コメントに続けて `#![allow(non_snake_case)]` を置く。各 bin は独立した crate なので、この属性は `v000_template.rs` から `v001_*.rs` へ自動では引き継がれない。v000 を複製するときは残し、直接作る solver にも追加する。
- `adhoc/`
  - ローカル専用の補助置き場である。bench / probe / check などの Rust 補助 bin は `adhoc/src/bin/*.rs` に、単発の分析・検証・PoC 用スクリプトは `adhoc/scripts/` に置く。
  - `adhoc/src/bin/*.rs` は cargo に自動認識されるため、`[[bin]]` の登録は不要である。`run.sh` と `eval.py` は solver と同じように bin 名で実行できる。
  - AtCoder のジャッジ環境では動かさないため、`adhoc/Cargo.toml` の依存はルートの固定一覧に縛られず自由に足してよい。
- `adhoc/src/bin/generate_experiment_dag.rs`
  - journal と backlog の整合性を検査し、`notes/experiment_dag.md` を生成する標準ライブラリだけの補助 bin である。
- `Cargo.toml`
  - AtCoder のジャッジ環境と同じ依存クレートを固定する場所であり、workspace のルートである。`adhoc/` の依存はここに足さない。
- `scripts/run.sh`
  - `src/bin/<name>.rs` をビルドし、stdin か 1 つの input file で手動実行する。既定で `local` feature を有効にし、`--no-local` で無効化する。
- `scripts/eval.py`
  - solver、公式 tester、score を build し、先頭入力のウォームアップ後にケース単位でインタラクティブ評価する。
  - 既定入力は `tools/in`、出力は `results/out/<bin_name>` である。
  - `--label` で実験ラベルを付け、`--dry-run` で蓄積ファイルを更新せずに確認できる。`--no-local` で solver の `local` feature を無効化する。
  - `-h` / `--help` で使い方を確認できる。
- `scripts/measure_solver_cpu.py`
  - 公式 tester と solver の間に入り、stdin と stdout を変更せずにsolver子プロセスの user CPU 時間と system CPU 時間を記録する。
- `scripts/gen_tools.sh`
  - `tools` 側の `gen` バイナリを呼ぶ薄い wrapper である。
- `scripts/unpack_tools.sh`
  - 公式配布 zip を `tools/` に展開する。
- `notes/`
  - 問題固有の発見や性質を記録する場所である。
- `notes/notations.md`
  - 問題で使う記号、コード上の代表名、型、制約の正本である。
- `notes/important_properties.md`
  - 任意の有効入力と合法な状態遷移で必ず成り立つ性質、不変量、探索や構築で効く確定事項を整理する正本である。
  - 記号の定義は書かず、`notes/notations.md` の表記を使って性質そのものを書く。
- `notes/input_distribution.md`
  - 入力生成規則に依存する分布、期待値、近似、代表値、観測上の偏りを整理する正本である。
  - 任意の有効入力で必ず成り立つ性質や、solver に依存する実験結果は書かない。
- `notes/journal.md`
  - 実験本文、現在状態、DAG 系譜の正本である。1 実験 1 エントリで、事前登録、当時の判定、その後に変化し得る現在の位置づけを記録する。運用は「実験知見の記録」に従う。
- `notes/backlog.md`
  - 実験アイデアと確定知見 (観察・問い) の台帳である。決着済み実験に表示する現在状態は journal と同期する。運用は「実験知見の記録」に従う。
- `notes/experiment_dag.md`
  - journal から自動生成する実験系譜の可視化と検索用一覧である。手作業では編集せず、`generate_experiment_dag` で再生成する。
- `notes/deep/`
  - journal のエントリに収まらない考察・設計・作業ログの置き場である。journal または backlog からリンクし、リンクされない孤児ファイルを作らない。
- `results/score_summary.csv`
  - score 要約の蓄積先である。
- `results/score_detail.csv`
  - `tools/in` 専用の wide-format score 比較表である。
- `results/eval_records.jsonl`
  - 1 行 1 case の評価記録を追記する正本である。
- `results/out/<bin_name>/...`
  - `eval.py` 実行時の出力ファイルの格納場所である。
- `tools/`
  - 公式 generator / tester / scorer の配置先である。中身は contest ごとに異なるので、wrapper script の引数や bin 名を固定だと思い込まない。
- `samples/`
  - サンプル input / output の配置先である。
- `.agents/skills/make-ahc-visualizer/SKILL.md`
  - visualizer 実装時に AI が従う手順である。UI / WASM / Vite のテンプレートはこの skill の同梱物から展開する。

## 数式・記号記述ルール
- `notes/notations.md` は、問題で使う記号、コード上の代表名、型、制約の正本である。
- 新しい重要記号を導入するしたいとき、 ユーザーに `notes/notations.md` に更新をしてよいか確認する。軽微なローカル変数だけnotations.md更新の例外とする。
- notation は会話・実装・検証で迷わないことを優先し、まず `notes/notations.md` に合わせる。
- 公式記号名はコードでも保持する。公式が `N`, `M` なら、会話・メモ・Rust の変数や field でも `N`, `M` と書き、対応づけだけを目的に `n`, `m` へ直さない。Rust の `non_snake_case` はコンパイルエラーではなく lint であり、solver 冒頭の `#![allow(non_snake_case)]` で許可する。添字は原則 0-based の `h[i,j]` 形式にしてよい。
- 問題文にない実装用の名前は、通常どおり Rust の命名規約に従う。lint の許可は公式記号との対応を保つためだけに使い、意味の異なる名前を大文字化しない。
- 問題文にない実装用の状態量は `state[g]`, `X[p,g]` のようにコードとの対応が見える名前にする。
- TeX は条件付き確率、総和、総積、比例関係などの構造だけに使い、コードフェンス内に入れない。

## 実験の進め方
- 1 回の実験は、照合 → 事前登録 → 実装 → 機構確認 → 評価 → 報告 → （ユーザー指示待ち）→ 判定と記録、の順で進める。solver 実行前の実装段階では、同一仮説の中での実装修正、バグ修正、パラメータ調整を自律的に進めてよい。solver を実行した後（機構確認・評価いずれも含む）は、その実行結果を根拠とする修正・調整・再実行を行わず、結果と事前登録した採否基準への機械的な照合を報告した上で、次のアクションについてユーザーの指示を待つ（「AtCoder Heuristic Contest 生成AI利用ルール」を参照）。仮説そのものを変えたくなったとき、採否基準を動かしたくなったときも、停止してユーザーに相談する。
- 主力 solver の採否基準には、同一評価セットで記録済みの全実験中最高の平均絶対スコアを上回ることを必ず含める。部品切り分け実験で比較元を上回っても全体最高を更新しなければ、知見は記録するが、その実験の当時の判定は棄却とする。現在状態は、知見の有効性や後続への統合状況を別途評価して付ける。
- 実験は一本筋にする。1 つの version に入れる中心アイデアは 1 つとし、ベース version との差分を最小に保つ。既存部品の組み合わせを検証したいときは、「組み合わせ」自体を中心アイデアとする 1 実験として扱う。
- 同じ実装に対して 2 回以上 eval してノイズの影響を測ることはしない。
- 実験アイデアは `notes/backlog.md` の観察・問い、`notes/important_properties.md` の性質、または `notes/input_distribution.md` の数値的知見から導く。提案時には、どの観察・性質・数値的知見から導いたかを 1 行で示す。
- 実験アイデアを考える段階では journal の本文を読まない。着手を決めてから、事前登録を書く前に backlog 全体と `grep "^## " notes/journal.md` の索引を照合し、類似実験があればその本文だけを読む。決着済みの再開条件に、現在の状況で満たされるものがないかもこのとき確認する。
- 問いのうち、solver を書かずに既存ログや `adhoc/` の分析で答えが出るものは、実験より先に潰す。

## 実験知見の記録
- `notes/journal.md` は実験本文、現在状態、DAG 系譜の正本であり、`notes/backlog.md` はアイデアと知見の台帳である。どちらも AI が更新する義務を負う。更新にユーザーの許可は不要であり、書き忘れが運用違反である。
- 当時の判定と現在状態を分けて記録する。当時の判定は評価直後の `未判定`、`採用`、`棄却`、`中断` のいずれかであり、後続実験によって位置づけが変わっても書き換えない。
- 現在状態は次の 5 種類だけを使う。
  - `現行採用`: 現在使う solver、または現在使う補助実装である。
  - `後続への統合`: 中心機構が明示的な `base` または `imports` の系譜を通じて現行版へ残り、実装内容からも継承を確認できる。
  - `知見のみ有効`: 中心機構は現行版へ残っていないが、学びが今後の判断材料として有効である。
  - `条件付き再検討`: 現在は採用しないが、具体的な再開条件を満たせば再検討する価値がある。
  - `未決着`: 実験中、未評価、または判定待ちである。
- DAG の関係は `base` と `imports` だけを使う。`base` は実装上の主な出発点を 0 個または 1 個、`imports` は別実験の中心機構を明示的に取り込んだ関係を 0 個以上記録する。単なる着想元、機構の類似、同等機構の独立検証から関係を推測してはならない。
- `series` は `foundation`、`no_move`、`current`、`auxiliary` のいずれかとする。
- 実験に着手したら、実装を書き始める前に journal へ見出しと次の 8 行を事前登録する。
  - 見出し: `## <bin名> — 未決着: 実験中`
  - `系譜: series=<系列>; base=<基盤実験|->; imports=[<統合元>, ...]`
  - `当時の判定: 未判定。`
  - `仮説:` 何が正しければスコアが上がるのか。
  - `変更:` ベース version 名と、そこからの最小差分。
  - `機構確認:` 新しい機構が発動したことを何で確認するか (`TraceStats` のキーなど)。
  - `採否基準:` どの評価セットで何がどうなったら採用か。実装後・評価後に基準を動かさない。
  - `結果: 未評価。`
  - `学び: 未確定。`
- ユーザーの指示を受けて判定するときは、当時の判定を `採用`、`棄却`、`中断` のいずれかへ確定し、結果と学びを更新する。結果には採否基準に対する実測と機構確認の実測を書く。同時に、見出しの現在状態と現在の位置づけを更新する。
- 機構確認をパスしていない状態で「効果がなかった」と結論しない。発動していないなら、それは効果の否定ではなくバグである。
- 学びには「何が起きたか」ではなく「次の実験の前提が何に変わったか」を書く。棄却でも、条件が変われば再試行に値するなら「再開条件:」を 1 行残す。
- 1 エントリは見出し + 8 行以内とする。収まらない考察・設計・作業ログは `notes/deep/` に置き、エントリからリンクする。
- 同一 bin の再評価や実装途中の状態でエントリを増やさない。1 実験 1 エントリを保つ。
- スコアの絶対値の羅列を journal に書かない。それは `results/` 以下の CSV の仕事である。journal には比較対象・差分・判定だけを書く。
- 会話に出た実験アイデアは、ユーザー発・AI 発を問わず backlog の未着手へ追記する。着手で実験中へ、判定で決着済みへ移動し、journal の実験名、現在状態、一言の位置づけを添える。backlog に過去の採否やスコアを記録済みなら、歴史的事実として残す。
- 実験せずに不要になったアイデアは、削除せず決着済みへ「取り下げ: 理由」として移動する。別アイデアに包含された場合は包含先の ID を書く。
- journal の学びが特定実験を超えて一般に成り立つと確認できたら、backlog の観察へ 1 行に要約して昇格させる。
- journal または backlog を更新したら、`cargo run --release --manifest-path adhoc/Cargo.toml --bin generate_experiment_dag -- --write` で `notes/experiment_dag.md` を再生成し、続けて同じコマンドの末尾を `--check` に替えて検査を通す。生成物を手作業で編集してはならない。
- 現行 solver を変更したら、`後続への統合`の全実験について、現行版へ至る DAG 上の系譜と、中心機構が実装に残っている事実を再監査する。DAG 上で到達できるだけでは統合の根拠にならない。

## AI が実装時に意識すること
- 問題の考察をする際に、わからないことは素直に「わからない」と認める。
- 保険のための `fallback` は実装しない。失敗を隠す分岐は通常経路の問題を見えにくくするため、別経路が必要な場合は、目的・発火条件・影響・通常経路で直せない理由を明示してからユーザーに許可を取る。
- 実装から読み取れない意図は、コード上のコメントとして残す。関数名、ロジック名、変数名だけでは伝わらない内容、たとえば「なぜこの処理が必要か」「この数字が何を意味するか」「なぜこの順番で処理するか」を短く書く。AHC のコードは提出物であると同時に探索過程の記録でもあるため、意図を残しておくと次の改善につなげやすい。
- 実装時には `v000_template.rs` の `TraceStats`、`local!`、`local_time!` の`local` feature を活用し、意図した経路を通っているか、fallback に落ちていないか、主要処理の回数・時間が妥当かを確認する。
- `v000_template.rs` は問題固有の共通土台の正本である。ここには `State`、問題のルール再現、基本操作、高速な reference 実装など、複数の solver から再利用したい確定実装を置く。
- version 固有の探索戦略、評価関数、パラメータ、ログ、暫定 hack は `v001_*.rs` 以降に分ける。
- `v000_template.rs` 以外では、不要になったコードを残さない。使われなくなった分岐、旧実装、暫定の互換コード、デッドコードは削除し、アルゴリズムを改変したときは現在の方針に合わせて関連処理も更新する。
- `v000_template.rs` は原則として頻繁に書き換えない。ただし、バグ修正、共通化の整理は反映してよい。その場合は、ユーザーに更新内容を説明し、更新の許可を取る。
- Rust の補助検証コードは `adhoc/src/bin/*.rs`、shell などの補助入口は `adhoc/scripts/` に置く。ルートの `src/bin/` には solver 以外を増やさない。
- `notes/important_properties.md` と `notes/input_distribution.md` では `notes/notations.md` の表記を使い、記号定義を重複させない。
- `notes/notations.md`、`notes/important_properties.md`、`notes/input_distribution.md` はユーザーの明示的な更新指示があった場合のみ更新する。
