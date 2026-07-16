# Heuristic Contest Agent Notes

## 前提
- 運用仕様の正本はこの AGENTS.md である。README.md は人間向けの概要とコマンド例であり、仕様が食い違ったら AGENTS.md を優先し、README.md を直す。
- このディレクトリが project root である。親や兄弟ディレクトリには依存しない。
- 言語は Rust のみである。
- AtCoder のジャッジ環境を前提にし、現在の依存環境以外は用いない。

## 評価運用ルール
- `scripts/run.sh` は単発の手動実行専用であり、solver を既定で `--release --features local` 付きで build する。ローカルでの実行確認や評価は原則 `local` feature 付きで行い、`--no-local` は本番相当の挙動や `local` feature なしでの compile 確認に限って使う。
- `scripts/eval.py` は評価パイプライン本体である。solver は既定で `--release --features local`、tools の score は通常の `--release` で build する。先頭入力で `run -> score` を 1 回ウォームアップしてから、本番の `run -> score` をケース単位で実行する。ウォームアップ結果は保存・集計しない。既定は `-j 2` であり、ユーザーの明示的な指定がない限りジョブ数は変更しない。ローカルでの評価や time sensitive なチューニングは原則 `local` feature 付きで行い、`--no-local` は本番相当の挙動や `local` feature なしでの compile 確認に限って使う。
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
  - top-level は `v000_template.rs` と提出候補 solver だけを置く。各ファイルは単体で完結し、1 行目に `// <file_name>.rs` を置く。
- `src/bin/adhoc/*.rs`
  - bench / probe / check などの補助 bin を置く場所である。
  - `crate_check.rs` も運用上はこの扱いである。
- `scripts/adhoc/`
  - 単発の分析・検証・PoC 用スクリプトを置く場所である。
- `Cargo.toml`
  - `src/bin/adhoc/*.rs` を `cargo run --bin <name>` で実行できるように `[[bin]]` を明示する場所である。
- `scripts/run.sh`
  - `src/bin/<name>.rs` をビルドし、stdin か 1 つの input file で手動実行する。既定で `local` feature を有効にし、`--no-local` で無効化する。
- `scripts/eval.py`
  - solver と score を build し、先頭入力のウォームアップ後にケース単位で並列評価する。
  - 既定入力は `tools/in`、出力は `results/out/<bin_name>` である。
  - `--label` で実験ラベルを付け、`--dry-run` で蓄積ファイルを更新せずに確認できる。`--no-local` で solver の `local` feature を無効化する。
  - `-h` / `--help` で使い方を確認できる。
- `scripts/gen_tools.sh`
  - `tools` 側の `gen` バイナリを呼ぶ薄い wrapper である。
- `scripts/unpack_tools.sh`
  - 公式配布 zip を `tools/` に展開する。
- `notes/`
  - 問題固有の発見や性質を記録する場所である。
- `notes/notations.md`
  - 問題で使う記号、コード上の代表名、型、制約の正本である。
- `notes/important_properties.md`
  - 問題から導かれる重要な性質、不変量、探索や構築で効く性質を整理する正本である。
  - 記号の定義は書かず、`notes/notations.md` の表記を使って性質そのものを書く。
- `notes/journal.md`
  - 実験の正史である。1 実験 1 エントリで、事前登録と判定を追記する。運用は「実験知見の記録」に従う。
- `notes/backlog.md`
  - 実験アイデアと確定知見 (観察・問い) の台帳である。運用は「実験知見の記録」に従う。
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
- 公式記号名は保持する。公式が `N`, `M` なら `N`, `M` と書き、Rust 風の `n`, `m` へ直さない。添字は原則 0-based の `h[i,j]` 形式にしてよい。
- 問題文にない実装用の状態量は `state[g]`, `X[p,g]` のようにコードとの対応が見える名前にする。
- TeX は条件付き確率、総和、総積、比例関係などの構造だけに使い、コードフェンス内に入れない。

## 実験の進め方
- 1 回の実験は、照合 → 事前登録 → 実装 → 機構確認 → 評価 → 判定と記録、の順で進める。同一仮説の中での実装修正、バグ修正、パラメータ調整、再評価は自律的に進めてよい。仮説そのものを変えたくなったとき、機構確認がどうしてもパスできないとき、採否基準を動かしたくなったときは、停止してユーザーに相談する。
- 実験は一本筋にする。1 つの version に入れる中心アイデアは 1 つとし、ベース version との差分を最小に保つ。既存部品の組み合わせを検証したいときは、「組み合わせ」自体を中心アイデアとする 1 実験として扱う。
- 同じ実装に対して 2 回以上 eval してノイズの影響を測ることはしない。
- 実験アイデアは `notes/backlog.md` の観察・問い、または `notes/important_properties.md` の性質から導く。提案時には、どの観察・性質から導いたかを 1 行で示す。
- 実験アイデアを考える段階では journal の本文を読まない。着手を決めてから、事前登録を書く前に backlog 全体と `grep "^## " notes/journal.md` の索引を照合し、類似実験があればその本文だけを読む。決着済みの再開条件に、現在の状況で満たされるものがないかもこのとき確認する。
- 問いのうち、solver を書かずに既存ログや `scripts/adhoc/` の分析で答えが出るものは、実験より先に潰す。

## 実験知見の記録
- `notes/journal.md` は実験の正史、`notes/backlog.md` はアイデアと知見の台帳である。どちらも AI が更新する義務を負う。更新にユーザーの許可は不要であり、書き忘れが運用違反である。
- 実験に着手したら、実装を書き始める前に journal へ事前登録する。書式は見出し `## <bin名> — 実験中` と、以下の 4 行である。
  - 仮説: 何が正しければスコアが上がるのか。
  - 変更: ベース version 名と、そこからの最小差分。
  - 機構確認: 新しい機構が発動したことを何で確認するか (`TraceStats` のキーなど)。
  - 採否基準: どの評価セットで何がどうなったら採用か。実装後・評価後に基準を動かさない。
- 判定したら、見出しを `## <bin名> — <採用|棄却|中断>: <一言>` に確定し、結果と学びの 2 行を追記する。結果には採否基準に対する実測と機構確認の実測を書く。
- 機構確認をパスしていない状態で「効果がなかった」と結論しない。発動していないなら、それは効果の否定ではなくバグである。
- 学びには「何が起きたか」ではなく「次の実験の前提が何に変わったか」を書く。棄却でも、条件が変われば再試行に値するなら「再開条件:」を 1 行残す。
- 1 エントリは見出し + 6 行以内とする。収まらない考察・設計・作業ログは `notes/deep/` に置き、エントリからリンクする。
- 同一 bin の再評価や実装途中の状態でエントリを増やさない。1 実験 1 エントリを保つ。
- スコアの絶対値の羅列を journal に書かない。それは `results/` 以下の CSV の仕事である。journal には比較対象・差分・判定だけを書く。
- 会話に出た実験アイデアは、ユーザー発・AI 発を問わず backlog の未着手へ追記する。着手で実験中へ、判定で決着済みへ移動し、journal のエントリ名と一言 (棄却なら再開条件) を添える。
- 実験せずに不要になったアイデアは、削除せず決着済みへ「取り下げ: 理由」として移動する。別アイデアに包含された場合は包含先の ID を書く。
- journal の学びが特定実験を超えて一般に成り立つと確認できたら、backlog の観察へ 1 行に要約して昇格させる。

## AI が実装時に意識すること
- 問題の考察をする際に、わからないことは素直に「わからない」と認める。
- 保険のための `fallback` は実装しない。失敗を隠す分岐は通常経路の問題を見えにくくするため、別経路が必要な場合は、目的・発火条件・影響・通常経路で直せない理由を明示してからユーザーに許可を取る。
- 実装から読み取れない意図は、コード上のコメントとして残す。関数名、ロジック名、変数名だけでは伝わらない内容、たとえば「なぜこの処理が必要か」「この数字が何を意味するか」「なぜこの順番で処理するか」を短く書く。AHC のコードは提出物であると同時に探索過程の記録でもあるため、意図を残しておくと次の改善につなげやすい。
- 実装時には `v000_template.rs` の `TraceStats`、`local!`、`local_time!` の`local` feature を活用し、意図した経路を通っているか、fallback に落ちていないか、主要処理の回数・時間が妥当かを確認する。
- `v000_template.rs` は問題固有の共通土台の正本である。ここには `State`、問題のルール再現、基本操作、高速な reference 実装など、複数の solver から再利用したい確定実装を置く。
- version 固有の探索戦略、評価関数、パラメータ、ログ、暫定 hack は `v001_*.rs` 以降に分ける。
- `v000_template.rs` 以外では、不要になったコードを残さない。使われなくなった分岐、旧実装、暫定の互換コード、デッドコードは削除し、アルゴリズムを改変したときは現在の方針に合わせて関連処理も更新する。
- `v000_template.rs` は原則として頻繁に書き換えない。ただし、バグ修正、共通化の整理は反映してよい。その場合は、ユーザーに更新内容を説明し、更新の許可を取る。
- Rust の補助検証コードは `src/bin/adhoc/*.rs`、shell などの補助入口は `scripts/adhoc/` に分ける。adhoc Rust bin を増やしたら `Cargo.toml` の `[[bin]]` も同時に更新する。
- `notes/important_properties.md` では `notes/notations.md` の表記を使い、記号定義を重複させない。
- `notes/notations.md` や `notes/important_properties.md` はユーザーの明示的な更新指示があった場合のみ更新する。
