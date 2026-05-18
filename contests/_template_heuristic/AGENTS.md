# Heuristic Contest Agent Notes

## 前提
- このディレクトリが project root である。親や兄弟ディレクトリには依存しない。
- 言語は Rust のみである。
- AtCoder のジャッジ環境を前提にし、現在の依存環境以外は用いない。
- `src/bin/*.rs` の top-level には `v000_template.rs` と提出候補 solver だけを置く。各ファイルは単体で完結し、1 行目に `// <file_name>.rs` を置く(AtCoder上でも識別できるように)。
- `src/bin/v000_template.rs` は問題固有の共通土台の正本である。ここには `State`、問題のルール再現、基本操作、制約判定、整合性チェック、reference 実装など、複数の solver から再利用したい確定実装を置く。
- 探索戦略、評価関数、パラメータ、ログ、暫定 hack など、version 固有で変動しやすい実装は通常 `v001_*.rs` 以降に置く。
- `v000_template.rs` は原則として頻繁に書き換えない。ただし、問題理解の更新、バグ修正、共通化のための整理はここに反映してよい。
- probe / bench / check / 一時検証用の Rust bin は `src/bin/adhoc/*.rs` に置く。
- shell などの単発分析・補助入口は `scripts/adhoc/` に置く。
- `src/bin/adhoc/*.rs` を追加するときは `Cargo.toml` に対応する `[[bin]]` を明示し、`cargo run --bin <name>` の bin 名を維持する。
- `crate_check.rs` は adhoc 補助 bin として扱い、運用上は `src/bin/adhoc/` に属するものとみなす。
- わからないことに関しては(特に問題の考察に関して)、それっぽい解説をするのではなく「わからない」と認める。
- 問題文や要点は `problem_description.txt` に記録する。
- problem_description作成は `.agents/skills/write-problem-description/SKILL.md` に従う。
- `notes/notations.md` は、問題で使う記号、コード上の代表名、型、制約の正本である。新しい重要記号、代表名、型、制約を導入したら、コード変更と同時に `notes/notations.md` も原則更新する。軽微なローカル変数だけは例外とする。
- notation は会話・実装・検証で迷わないことを優先し、まず `notes/notations.md` に合わせる。
- 公式記号名は保持する。公式が `N`, `M` なら `N`, `M` と書き、Rust 風の `n`, `m` へ直さない。添字は原則 0-based の `h[i,j]` 形式にしてよい。
- 問題文にない実装用の状態量は `state[g]`, `X[p,g]` のようにコードとの対応が見える名前にする。
- TeX は条件付き確率、総和、総積、比例関係などの構造だけに使い、コードフェンス内に入れない。
- `notes/important_properties.md` は、問題から導かれる重要な性質、不変量、探索や構築で効く性質の正本である。新しい重要な性質や有力な仮説が見えたら、コード変更とあわせて `notes/important_properties.md` に整理する。
- 公式配布物は `tools/` と `samples/` に配置する。
- visualizer実装は `.agents/skills/make-ahc-visualizer/SKILL.md` に従う。必要な UI / WASM テンプレートは skill の同梱物から project root に展開する。

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
- `scripts/run.sh` は単発の手動実行専用である。
- `scripts/eval.py` は評価パイプライン本体である。solver と tools の score をそれぞれ 1 回だけ build し、先頭入力で `run -> score` を 1 回ウォームアップしてから、本番の `run -> score` をケース単位で実行する。ウォームアップ結果は保存・集計しない。既定は `cpu//2 - 1` 並列で、最小値は 1 である。厳密に見たいときは `-j 1` を使う。
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
  - top-level は `v000_template.rs` と提出候補 solver を置く場所である。
- `src/bin/adhoc/*.rs`
  - bench / probe / check などの補助 bin を置く場所である。
  - `crate_check.rs` も運用上はこの扱いである。
- `scripts/adhoc/`
  - 単発の分析・検証・PoC 用スクリプトを置く場所である。
- `Cargo.toml`
  - `src/bin/adhoc/*.rs` を `cargo run --bin <name>` で実行できるように `[[bin]]` を明示する場所である。
- `scripts/run.sh`
  - `src/bin/<name>.rs` をビルドし、stdin か 1 つの input file で手動実行する。
- `scripts/eval.py`
  - solver と score を build し、先頭入力のウォームアップ後にケース単位で並列評価する。
  - 既定入力は `tools/in`、出力は `results/out/<bin_name>`、`-j 1` で直列評価できる。
  - `--label` で実験ラベルを付け、`--dry-run` で蓄積ファイルを更新せずに確認できる。
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
  - 公式 generator / tester / scorer の配置先である。
- `samples/`
  - サンプル input / output の配置先である。
- `.agents/skills/make-ahc-visualizer/SKILL.md`
  - visualizer 実装時に AI が従う手順である。UI / WASM / Vite のテンプレートはこの skill の同梱物から展開する。

## AI が実装時に意識すること
- fallback を勝手に実装しない。失敗時の別経路が必要に見える場合は、目的・発火条件・影響・通常経路で直せない理由を明示してから、ユーザーに許可を取る。
- `v000_template.rs` には複数 solver で共有したい確定実装を寄せ、version 固有の探索ロジックや一時的な hack は `v001_*.rs` 以降に分ける。
- Rust の補助検証コードは `src/bin/adhoc/*.rs`、shell などの補助入口は `scripts/adhoc/` に分ける。
- adhoc Rust bin を増やしたら `Cargo.toml` の `[[bin]]` も同時に更新する。
- 記号や代表名を導入するときは、solver 間で別名を乱立させず `notes/notations.md` を正本として揃える。
- `notes/important_properties.md` で使う記号も `notes/notations.md` に合わせる。
- 重要な配列や状態量を説明するときは、コード上の変数名と対応する notation を併記する。例えば `params[g][p]` を議論上 `X[p,g]` と書くなら、その対応を `notes/notations.md` に残す。
- `notes/notations.md` や `notes/important_properties.md` はユーザーの明示的な更新指示があった場合のみ更新する。
- `tools/` の中身は contest ごとに異なる。wrapper script の引数や期待する bin 名は固定だと思い込まない。
