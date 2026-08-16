# Heuristic Contest Template

このディレクトリは、AtCoder Heuristic Contest 用の作業テンプレートである。  
解法実装、実験、採点、visualizer をこのディレクトリの中だけで進める前提で作ってある。

運用仕様の正本は [AGENTS.md](AGENTS.md) である。この README は人間向けの概要とコマンド例をまとめる。

## ディレクトリ構成

主要なものだけ示す。各ファイルの詳細な役割と運用ルールは AGENTS.md の「各ディレクトリ・ファイルの役割」を参照。

```text
_template_heuristic/
├── README.md                    # 人間向け概要 (このファイル)
├── AGENTS.md                    # 運用仕様の正本
├── problem_description.txt      # 問題文、制約、スコア
├── Cargo.toml                   # AtCoder と同じ依存の固定 (workspace ルート)
├── .agents/skills/              # AI 用スキル (問題転記、v000 構築、visualizer ほか)
├── .claude/commands/            # スキル起動用コマンド
├── src/bin/
│   ├── v000_template.rs         # 問題固有の共通土台
│   ├── v001_*.rs 以降           # 試行錯誤する solver (提出はこのファイルを直接使う)
│   └── crate_check.rs           # 依存一覧の検査器 (固定)
├── adhoc/                       # ローカル専用の補助置き場 (依存は自由)
│   ├── src/bin/                 # bench / probe / check / DAG生成器などの補助 bin (自動認識)
│   └── scripts/                 # 単発の分析・検証スクリプト
├── scripts/
│   ├── run.sh                   # 単発の手動実行
│   ├── eval.py                  # 並列評価パイプライン
│   ├── measure_solver_cpu.py    # solver のCPU時間計測ラッパー
│   ├── gen_tools.sh             # 追加入力生成の wrapper
│   └── unpack_tools.sh          # 公式配布 zip の展開
├── notes/
│   ├── notations.md             # 記号の正本
│   ├── important_properties.md  # 任意の有効入力で成り立つ性質の正本
│   ├── input_distribution.md    # 入力生成規則に依存する数値的知見の正本
│   ├── journal.md               # 実験本文、現在状態、DAG系譜の正本
│   ├── backlog.md               # 実験アイデアと確定知見の台帳
│   ├── experiment_dag.md        # journalから自動生成する実験系譜
│   └── deep/                    # journal に収まらない深掘りメモ
├── results/                     # 評価ログ (score_summary.csv ほか) と出力
├── samples/                     # サンプル入出力
└── tools/                       # 公式 generator / tester / scorer の展開先
```

## 最初にやること

実験を始める前に、この順で土台を整える。

1. 公式配布物を `tools/` と `samples/` に置く (`./scripts/unpack_tools.sh ./tools.zip`)
2. `.agents/skills/write-problem-description/SKILL.md` に従い、`problem_description.txt` を埋め、`notes/notations.md` に記号の正本を固める
3. `scripts/eval.py` を contest の scoring tool の呼び出し方に合わせて編集する
4. 必要なら `.agents/skills/make-ahc-visualizer/SKILL.md` に従って visualizer を作る
5. `.agents/skills/make-v000-template/SKILL.md` に従い、`src/bin/v000_template.rs` に入出力・`State`・操作適用の共通土台を作る。全実験の速度がこの設計で決まるため、十分高速なデータ構造をここで固める
6. 任意の有効入力で成り立つ性質を `notes/important_properties.md`、入力生成規則に依存する数値的知見を `notes/input_distribution.md` に整理する (以降も随時更新する)

ここまで終えたら「実験の流れ」に入る。

## 短時間コンテストでの進め方

AI に全任せでもよいので初期解を素早く作り、まず動かして可視化し、問題固有の構造を観察する。
問題固有の構造を理解しないままアイデア出しに執着したり、データ構造の設計に長時間をかけたりしない。
ある程度方針が立ったら、データ構造とアイデアを短い周期で改善する。
人間は、エージェントに小さな試作と計測を任せ、観察をもとに次に確かめる仮説を選び、判断する。

## 問題文の記号をそのまま使う

`src/bin/v000_template.rs` には `#![allow(non_snake_case)]` を入れてある。Rust の snake case はコンパイルエラーではなく lint なので、問題文の `N`、`M` などは Rust の変数や field でもそのまま使える。

`src/bin` の各 solver は別の crate としてコンパイルされる。`v000_template.rs` を複製して `v001_*.rs` を作るときはこの属性を残し、空の solver を直接作るときは、1 行目の `// <file_name>.rs` に続けて追加する。問題文にない実装用の名前は通常どおり snake_case を使う。

## 実験の流れ

1. 着手前に `notes/backlog.md` と `notes/journal.md` の索引を照合し、現在状態、`base`、`imports`を含めて`notes/journal.md`に事前登録する
2. 共通土台は `src/bin/v000_template.rs` に、試行錯誤する solver は `src/bin/v001_*.rs` 以降に書く
3. `./scripts/run.sh`または`./scripts/eval.py`で機構やスコアを確認する
4. 実行結果と事前登録した採否基準との照合を報告し、ユーザーの次の明示指示を待つ
5. 指示を受けたら、当時の判定と現在状態を`notes/journal.md`へ記録し、`notes/backlog.md`の状態を同期する
6. `generate_experiment_dag`で`notes/experiment_dag.md`を再生成し、整合性検査を通す
7. 提出時は対象の `src/bin/<bin_name>.rs` を直接コピーして使う

input file を指定した `run.sh` と `eval.py` は、公式 tester を介してsolverとインタラクティブに通信する。
評価ログの `elapsed` には、solverプロセスの user CPU 時間と system CPU 時間の合計を整数 ms で記録する。

script の詳細な挙動（build オプション、warmup、timeout、ログの追記条件）と実験運用の規律（事前登録の書式、判定ルール）は AGENTS.md を参照。

## 実験記録の見方

`notes/journal.md`では、評価直後の「当時の判定」と、現時点での有用性を表す「現在状態」を分けている。

現在状態は`現行採用`、`後続への統合`、`知見のみ有効`、`条件付き再検討`、`未決着`の5種類である。

実験間のDAGは、実装上の主な出発点を表す`base`と、別実験の中心機構を取り込んだことを表す`imports`だけで構成する。

`notes/experiment_dag.md`はjournalから生成する閲覧用文書であり、手作業では編集しない。

## よく使うコマンド

```bash
./scripts/run.sh <bin_name>
./scripts/run.sh <bin_name> ./tools/in/0000.txt
./scripts/run.sh --no-local <bin_name> ./tools/in/0000.txt
./scripts/eval.py <bin_name>
./scripts/eval.py -v --label baseline <bin_name>
./scripts/eval.py --dry-run <bin_name>
./scripts/eval.py --help
./scripts/unpack_tools.sh ./tools.zip
cargo run --release --manifest-path adhoc/Cargo.toml --bin generate_experiment_dag -- --write
cargo run --release --manifest-path adhoc/Cargo.toml --bin generate_experiment_dag -- --check
```

## Visualizer の使い方

- まず `problem_description.txt` と `tools/src/` を揃える
- `.agents/skills/make-ahc-visualizer/SKILL.md` を読み、同梱テンプレートを project root に展開する
- skill の指示に従い、問題固有部分だけを編集して起動確認する
