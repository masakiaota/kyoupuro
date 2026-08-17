# Heuristic Contest Template (C++)

このディレクトリは、AtCoder Heuristic Contest 用の C++ 作業テンプレートである。
解法実装、実験、採点、visualizer をこのディレクトリの中だけで進める前提で作ってある。

solver は C++ のみを使う。AtCoder 公式の generator / tester / scorer と visualizer の WASM では、配布形態に合わせて Rust を使う。

運用仕様の正本は [AGENTS.md](AGENTS.md) である。この README は人間向けの概要とコマンド例をまとめる。

## ディレクトリ構成

```text
_template_heuristic_cpp/
├── README.md                    # 人間向け概要
├── AGENTS.md                    # 運用仕様の正本
├── problem_description.txt      # 問題文、制約、スコア
├── .agents/skills/              # AI 用スキル
├── .claude/commands/            # スキル起動用コマンド
├── src/bin/
│   ├── v000_template.cpp        # 問題固有の共通土台
│   └── v001_*.cpp 以降          # 試行錯誤する solver。各ファイルを直接提出する
├── adhoc/
│   ├── bin/                     # bench / probe / check などの C++ 補助コード
│   └── scripts/                 # 単発の分析・検証スクリプト
├── scripts/
│   ├── build_solver.sh          # C++ solver の共通ビルド入口
│   ├── run.sh                   # 単発の手動実行
│   ├── eval.py                  # 並列評価パイプライン
│   ├── gen_tools.sh             # 追加入力生成の wrapper
│   └── unpack_tools.sh          # 公式配布 zip の展開
├── notes/
│   ├── notations.md             # 記号の正本
│   ├── important_properties.md  # 問題から導かれる性質の正本
│   ├── journal.md               # 実験の正史
│   ├── backlog.md               # 実験アイデアと確定知見の台帳
│   └── deep/                    # journal に収まらない深掘りメモ
├── results/                     # 評価ログと出力
├── samples/                     # サンプル入出力
└── tools/                       # 公式 generator / tester / scorer の展開先
```

## 最初にやること

1. 公式配布物を `tools/` と `samples/` に置く (`./scripts/unpack_tools.sh ./tools.zip`)
2. `.agents/skills/write-problem-description/SKILL.md` に従い、`problem_description.txt` と `notes/notations.md` を整える
3. `scripts/eval.py` を contest の scoring tool の呼び出し方に合わせて編集する
4. 必要なら `.agents/skills/make-ahc-visualizer/SKILL.md` に従って visualizer を作る
5. `.agents/skills/make-v000-template/SKILL.md` に従い、`src/bin/v000_template.cpp` に入出力・`State`・操作適用の共通土台を作る
6. 見えてきた重要な性質を `notes/important_properties.md` に整理する

## C++ のビルド

既定では `g++-15` を使う。別の実行ファイルを使う場合は `CXX` を指定する。

```bash
CXX=/opt/homebrew/bin/g++-15 ./scripts/run.sh v001_solver ./tools/in/0000.txt
```

基本オプションは AtCoder の C++23 環境に合わせて `-std=gnu++23 -O2 -Wall -Wextra -march=native` などを使う。通常ビルドでは `LOCAL` マクロを定義し、`--no-local` では AtCoder 側の主要マクロを定義する。

各 solver は単独で完結する `.cpp` とし、提出時は対象ファイルをそのまま AtCoder へ貼り付ける。問題文の公式記号が `N`, `M` なら、C++ の変数やメンバーでも同じ綴りを使ってよい。問題文にない実装用の名前は通常どおり `snake_case` にする。

## 実験の流れ

1. 着手前に `notes/backlog.md` と `notes/journal.md` の索引を照合し、`notes/journal.md` に事前登録する
2. 共通土台は `src/bin/v000_template.cpp` に、試行錯誤する solver は `src/bin/v001_*.cpp` 以降に書く
3. `./scripts/run.sh` で単発確認する
4. `./scripts/eval.py` で公式スコアを確認する
5. 判定を `notes/journal.md` のエントリに確定し、`notes/backlog.md` の状態を更新する
6. 提出時は対象の `src/bin/<bin_name>.cpp` を直接使う

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
- skill の指示に従い、問題固有部分だけを編集する
