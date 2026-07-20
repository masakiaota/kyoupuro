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
│   ├── src/bin/                 # bench / probe / check などの補助 bin (自動認識)
│   └── scripts/                 # 単発の分析・検証スクリプト
├── scripts/
│   ├── run.sh                   # 単発の手動実行
│   ├── eval.py                  # 並列評価パイプライン
│   ├── gen_tools.sh             # 追加入力生成の wrapper
│   └── unpack_tools.sh          # 公式配布 zip の展開
├── notes/
│   ├── notations.md             # 記号の正本
│   ├── important_properties.md  # 問題から導かれる性質の正本
│   ├── journal.md               # 実験の正史 (事前登録と判定)
│   ├── backlog.md               # 実験アイデアと確定知見の台帳
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
6. 見えてきた重要な性質を `notes/important_properties.md` に整理する (以降も随時更新する)

ここまで終えたら「実験の流れ」に入る。

## 実験の流れ

1. 着手前に `notes/backlog.md` と `notes/journal.md` の索引を照合し、`notes/journal.md` に事前登録する
2. 共通土台は `src/bin/v000_template.rs` に、試行錯誤する solver は `src/bin/v001_*.rs` 以降に書く
3. `./scripts/run.sh` で単発確認する (機構確認を含む)
4. `./scripts/eval.py` で公式スコアを確認する
5. 判定を `notes/journal.md` のエントリに確定し、`notes/backlog.md` の状態を更新する
6. 提出時は対象の `src/bin/<bin_name>.rs` を直接コピーして使う

script の詳細な挙動 (build オプション、warmup、ログの追記条件) と実験運用の規律 (事前登録の書式、判定ルール) は AGENTS.md を参照。

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
```

## Visualizer の使い方

- まず `problem_description.txt` と `tools/src/` を揃える
- `.agents/skills/make-ahc-visualizer/SKILL.md` を読み、同梱テンプレートを project root に展開する
- skill の指示に従い、問題固有部分だけを編集して起動確認する
