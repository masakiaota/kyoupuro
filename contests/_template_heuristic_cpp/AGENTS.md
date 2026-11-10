# Heuristic Contest Agent Notes

## 前提
- 運用仕様の正本はこの AGENTS.md である。README.md は人間向けの概要とコマンド例であり、仕様が食い違ったら AGENTS.md を優先し、README.md を直す。
- このディレクトリが project root である。親や兄弟ディレクトリには依存しない。
- solver の言語は C++ のみである。AtCoder 公式 tools と visualizer の WASM では、配布形態に合わせて Rust を使ってよい。
- C++23 と GCC 15 系を前提にする。solver は AtCoder へ直接提出できる単一の `.cpp` とする。
- AtCoder のジャッジは solver を単一コアで実行する。solver は単一スレッドで書き、スレッド、プロセス、OpenMP による並列化を導入しない。評価器による独立ケースの並列実行と、単一コア内の SIMD は使ってよい。
- Git は main ブランチ 1 本で運用する。作業ブランチや PR は作らず、main に直接コミットして push する。
- 作業ディレクトリもこの 1 つだけを使い、worktree や別 clone を作らない。複数セッションが同じディレクトリを共有するため、作業前に `git pull` し、コミットしたらすぐ push する。
- Git ではコードと Markdown を管理し、データは管理しない。解析結果、教師データ、追加生成した入力、solver の出力、アーカイブは各端末に置く。例外は `.gitignore` が明示的に許可した評価要約だけである。データを `git add -f` で追加しない。

## 評価運用ルール
- solver のスコアと実行時間の計測、および実験の判定は、`scripts/eval.py` の標準 `LOCAL` だけで行う。標準 `LOCAL` とは、端末設定 (`config/local_profiles.json`) の時間係数と並列数をそのまま使う評価である。端末ごとに速度と並列数を調整して条件をそろえているので、`--no-local`、`-j`、`--profile`、`--time-limit-sec` を指定した測定や、別の端末の測定との差を判定に使わない。
- `scripts/run.sh` は単発の手動実行専用であり、solver を既定で `LOCAL` マクロ付きで build する。`--no-local` は `LOCAL` マクロなしの本番相当ビルドに使う。
- `scripts/eval.py` は評価パイプライン本体である。solver は既定で `LOCAL` マクロ付き、tools の score は Cargo の `--release` で build する。先頭入力で `run -> score` を 1 回ウォームアップしてから、全 solver を並列実行し、その後に採点する。ウォームアップ結果は保存と集計の対象外である。設定を読めない場合や一致する端末がない場合は 1 並列、時間補正なしとする。
- solver 内の時間制限やフェーズ切替は AtCoder 基準の秒で記述し、`v0000_template.cpp` の `TimeKeeper` で判定する。timer は `main` 開始直後に作る。`LOCAL` 時の時間換算は timer 内部で行う。
- `TimeKeeper` の経過時間は process CPU 時間 (user+system) で数える。並列評価で CPU を割り当てられていない時間や、ジャッジ応答の待ちは含まない。インタラクティブ問題では、ジャッジからの読み込みを初期入力も含めすべて `time_keeper.wait_judge(...)` で包み、待ちの実時間を診断用の `wait_ms` に記録する。
- `eval.py` の経過時間と時間切れ判定には、`TimeKeeper` が終了時に stderr へ出す `time_keeper: solver_ms=` の値を使う。時間切れの基準は問題文の実行時間制限 (`eval.py` の `PROBLEM_TIME_LIMIT_SEC`) であり、`LOCAL` では端末の時間係数を掛ける。実時間がこの基準の 3 倍を超えたときだけ、ハングとして停止する。この報告を出さない solver は実時間で判定し、評価記録の `elapsed_source` に `wall` と残す。
- `results/out/<bin_name>/` は最新評価の scratch/workspace である。`eval.py` 実行時に同名 basename の出力が並ぶ前提なので、重複 basename は拒否する。
- `results/score_summary.csv` は評価要約ログである。列順は `bin,total_avg,total_sum,total_min,total_max,avg_elapsed,max_elapsed,eval_set,total_cases,label,executed_at` で、経過時間は整数 ms で記録する。全ケース成功時のみ追記する。
- `results/score_detail.csv` は `tools/in` 専用の wide-format 比較表である。列順は `bin,total_avg,max_elapsed,<case_name_1>,...,label,executed_at` で、全ケース成功時のみ追記する。
- `results/eval_records/<input_dir>.jsonl` は 1 行 1 case の正本ログであり、評価セットごとに分けて保存する。`tools/in` の記録は `results/eval_records/tools/in.jsonl` である。失敗ケースも含めて追記する。Git で共有するのは `tools/in` の記録だけである。
- 評価セットは置き場所で 2 種類に分ける。
  - `tools/` 配下の入力ディレクトリは共有の評価セットである。公式配布の `tools/in` のほかは、ユーザーの指示があったときだけ `tools/in_<件数>` として追加する。追加するときは、同じ入力を作り直せる生成手順を `adhoc/scripts/` に残す。件数の多い評価が必要になったら、まず既存の共有セットを使い、同じ用途のセットを作り直さない。
  - `adhoc/` 配下の入力ディレクトリは単発の評価セットである。分析や検証のために自由に作ってよい。
- eval viewer が読み込むのは共有の評価セットの記録だけである。`adhoc/` 配下の入力と project 外の入力による評価記録は読み込まない。単発の評価セットを eval viewer に表示するための仕組みを追加しない。
- `--dry-run` は `results/score_summary.csv`、`results/score_detail.csv`、`results/eval_records/` を更新しない。
- verbose は進捗表示だけに使い、追加ログを恒久保存しない。

## 各ディレクトリ・ファイルの役割
- `problem_description.txt`
  - 問題文、制約、スコア、初動メモの保存先である。
- `.agents/skills/write-problem-description/SKILL.md`
  - problem_description 作成時に AI が従う手順である。貼り付けテキストやスクリーンショットから公式の節順を保って転記する。
- `src/bin/*.cpp`
  - top-level は `v0000_template.cpp` と提出候補 solver だけを置く。各ファイルは単体で完結し、1 行目に `// <file_name>.cpp` を置く。
- `adhoc/`
  - ローカル専用の補助置き場である。bench / probe / check などの C++ 補助コードは `adhoc/bin/*.cpp` に、単発の分析・検証・PoC 用スクリプトは `adhoc/scripts/` に置く。
  - `run.sh` と `eval.py` は補助コードも solver と同じように bin 名で実行できる。
  - Git で管理するのはコードと Markdown だけである。`.gitignore` がこれを強制する。
- `scripts/build_solver.sh`
  - `src/bin/<name>.cpp` または `adhoc/bin/<name>.cpp` を GCC 15 系でビルドし、`target/release/<name>` を作る。既定で `LOCAL` を定義し、`--no-local` で無効化する。コンパイラは `CXX` で上書きできる。
- `scripts/run.sh`
  - `src/bin/<name>.cpp` をビルドし、stdin か 1 つの input file で手動実行する。
- `scripts/eval.py`
  - 既定入力は `tools/in`、出力は `results/out/<bin_name>` である。
  - `--label` で実験ラベルを付け、`--dry-run` で蓄積ファイルを更新せずに確認できる。`-h` / `--help` で使い方を確認できる。
- `scripts/gen_tools.sh`
  - `tools` 側の `gen` バイナリを呼ぶ薄い wrapper である。
- `scripts/unpack_tools.sh`
  - 公式配布 zip を `tools/` に展開する。
- `notes/status.md`
  - 目標、評価方針、提出状況、提出候補、実験中の一覧の正本である。書く内容と書かない内容は同ファイルの冒頭に従う。
- `notes/backlog.md`
  - 検討を続ける案、問い、観察の台帳である。
- `notes/experiments/`
  - 実験ごとの記録を置く。1 実験 1 ファイルの `vNNNN.md` とし、書式は `notes/experiments/README.md` に従う。
- `notes/notations.md`
  - 問題で使う記号、コード上の代表名、型、制約の正本である。
- `notes/important_properties.md`
  - 問題から導かれる重要な性質、不変量、探索や構築で効く性質の正本である。記号の定義は書かず、`notes/notations.md` の表記を使う。
- `notes/handoffs/`
  - 引き継ぎ文書の置き場である。
- `tools/`
  - 公式 generator / tester / scorer の配置先である。中身は contest ごとに異なるので、wrapper script の引数や bin 名を固定だと思い込まない。
- `.agents/skills/make-ahc-visualizer/SKILL.md`
  - visualizer 実装時に AI が従う手順である。UI / WASM / Vite のテンプレートはこの skill の同梱物から展開する。

## 記号
- 会話、実装、検証の表記は `notes/notations.md` の「notation 方針」にそろえる。新しい重要記号が必要なときは、記号一覧を更新してよいかユーザーに確認する。軽微なローカル変数は例外とする。
- `notes/notations.md` と `notes/important_properties.md` は、ユーザーの明示的な更新指示があった場合のみ更新する。

## 実験の進め方
1. **提案**　`notes/backlog.md` の観察や問い、または `notes/important_properties.md` の性質から、根拠を 1 行で示す。案を考える段階では実験ノート本文を読まない。既存ログや `adhoc/` の分析で答えが出る問いは、solver を書く前に解く。
2. **照合**　事前登録の前に、`notes/status.md` の「実験中」と関連する未着手案を確認する。過去実験は親 ID、対象の関数名、手法名で絞り込み、該当するノートだけを読んで、派生元、今回の案との差分、判定、再開条件を確認する。
3. **事前登録**　実装前に `notes/experiments/vNNNN.md` へ実験を登録する。同じコミットで `notes/status.md` の「実験中」に追加し、対応する未着手案を backlog から消す。番号の予約を push してから実装する。
4. **実装と評価**　1 実験の中心アイデアは 1 つとし、親からの差分を最小にする。部品の組み合わせ自体を 1 つの実験として検証してもよい。新機構の発動を確かめてから評価する。発動していないなら、効果がなかったのではなくバグであり、原因を調べる。登録した判定基準は変えない。ノイズ測定のために同じ実装を再評価しない。
5. **判定**　判定は「基準通過」「基準未達」「中断」のいずれかとし、この実験の比較基準に対して判定基準を満たしたかどうかだけを表す。提出候補にするかどうかは実験ノートに書かず、`notes/status.md` に書く。結果と判定は同じ実験ノートに、ケース別スコアは `results/` に記録する。判定または打ち切りで「実験中」の行を消す。新しい仮説には別番号を使う。

- 実験ノート、backlog、status は AI が更新する義務を負う。更新にユーザーの許可は不要であり、書き忘れが運用違反である。
- 会話に出た実験アイデアは、ユーザー発・AI 発を問わず backlog へ追記する。
- ユーザーから聞いた AtCoder の提出結果も含め、現在地が変わったら `notes/status.md` を更新する。
- 同じ会話で複数の実験を進めてよい。改善の見込みが薄れたら方針を見直すか、ユーザーに相談する。

### 採番
- 実験 ID は 4 桁の version 番号 `vNNNN` とし、ノート名、Front Matter の `id`、solver の version 番号を一致させる。たとえば `v0003_reconnect.cpp` の実験ノートは `notes/experiments/v0003.md` である。
- 最新の main で実験ノートの最大番号に 1 を足し、1 件ずつ予約する。push が競合したら同期し、番号の空きを再確認する。中断や基準未達の番号も再利用しない。
- ユーザーが番号帯を指定した場合は、その帯の中で同じ手順により採番する。

## AI が実装時に意識すること
- 問題の考察をする際に、わからないことは素直に「わからない」と認める。
- 保険のための `fallback` は実装しない。失敗を隠す分岐は通常経路の問題を見えにくくするため、別経路が必要な場合は、目的・発火条件・影響・通常経路で直せない理由を明示してからユーザーに許可を取る。
- 実装から読み取れない意図は、コード上のコメントとして残す。関数名、ロジック名、変数名だけでは伝わらない内容、たとえば「なぜこの処理が必要か」「この数字が何を意味するか」「なぜこの順番で処理するか」を短く書く。AHC のコードは提出物であると同時に探索過程の記録でもあるため、意図を残しておくと次の改善につなげやすい。
- 実装時には `v0000_template.cpp` の `TraceStats`、`LOCAL_ONLY`、`LOCAL_TIME` を活用し、意図した経路を通っているか、fallback に落ちていないか、主要処理の回数・時間が妥当かを確認する。
- `v0000_template.cpp` は問題固有の共通土台の正本である。ここには `State`、問題のルール再現、基本操作、高速な reference 実装など、複数の solver から再利用したい確定実装を置く。
- version 固有の探索戦略、評価関数、パラメータ、ログ、暫定 hack は `v0001_*.cpp` 以降に分ける。
- `v0000_template.cpp` 以外では、不要になったコードを残さない。使われなくなった分岐、旧実装、暫定の互換コード、デッドコードは削除し、アルゴリズムを改変したときは現在の方針に合わせて関連処理も更新する。
- `v0000_template.cpp` は原則として頻繁に書き換えない。ただし、バグ修正、共通化の整理は反映してよい。その場合は、ユーザーに更新内容を説明し、更新の許可を取る。
- C++ の補助検証コードは `adhoc/bin/*.cpp`、shell などの補助入口は `adhoc/scripts/` に置く。ルートの `src/bin/` には solver 以外を増やさない。
