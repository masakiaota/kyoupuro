# Heuristic Contest Agent Notes

## 前提
- このディレクトリが project root である。親や兄弟ディレクトリには依存しない。
- 言語は Rust のみである。
- AtCoder のジャッジ環境を前提にし、現在の依存環境以外は用いない。

## 生成AI利用ルール (AtCoder Heuristic Contest 生成AI利用ルール の解釈ボーダー)
- AI は 1 つの会話で、新しく生成する解候補を 1 つまでにする。実務上は `src/bin` に増える新規ファイルを 1 本までとみなす。
  - なお、1つの会話とは、ユーザーの指示に対してAIが出力するまでである。次にユーザーの指示が入力されるときは新しい会話とみなす。そのため 1 threadのなかで複数のversionを作成することは問題ない。
- AI は現在取り組んでいる 1 候補に対しては、実行確認、デバッグ、改善をしてよい。
- AI は、人間が指定した既存 solver に対するハイパーパラメータ探索スクリプトを作成してよい。
- AI が直接複数候補を生成し、テストケースで自動比較・自動選別しながら改善することは禁止する。
- 複数候補の比較は、人間が明示的に指定した既存候補に対してのみ行ってよい。

### OK例
- `この1案を改善して`
- `この bin を実行してバグを直して`
- `既存の v003 と v007 を比較して`

### NG例
- `3案新しく作って比較して`
- `10個生成して一番スコアの良いものを選んで`
- `改善案を複数作って自動でベンチを回し、良いものだけ残して`

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

## AI が実装時に意識すること
- わからないこと、特に問題の考察は、それっぽい解説で埋めず「わからない」と認める。
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
