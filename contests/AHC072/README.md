# AHC072 ワークスペース

このディレクトリは、終了したAHC072のソースと実験記録を保存し、復習に使うワークスペースである。
解法実装、実験、採点、visualizer をこのディレクトリの中だけで進める前提で作ってある。

solver は C++ のみを使う。AtCoder 公式の generator / tester / scorer と visualizer の WASM では、配布形態に合わせて Rust を使う。

運用仕様の正本は [AGENTS.md](AGENTS.md) である。この README は人間向けの概要とコマンド例をまとめる。

## 移行について

2026年10月5日に元ディレクトリから必要なファイルをコピーした。Git管理情報、ビルド成果物、依存ライブラリ、大量の実行出力は省略した。保存範囲と再利用時の注意は [MIGRATION.md](MIGRATION.md) を参照する。

上位解法の参照先は [AHC072 統合解法資料](../../practice/AHC-problem-reference/AHC072/solution.md) である。`notes/solutions/` は移行時点の資料を保持する。

## コンテスト終了後の解法資料

[AHC072 解法資料](notes/solutions/README.md)には、上位20人の比較と8人の個別解説を保存している。将来のコンペで参照するときは、順位や参加者名に加え、問題の特徴から手法を探せる。各解説の冒頭に適用条件、必要な状態表現、持ち出せる設計と制限がある。

今回の取り組みは [REFRECTION.md](REFRECTION.md)、条件付き支持が完成解の改善につながらなかった理由は [考察](notes/solutions/conditional_support_review.md)に記録している。

## ディレクトリ構成

```text
AHC072/
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
│   ├── solutions/               # 将来のコンペで参照する上位解法と適用条件
│   ├── experiments/             # 1 実験 1 ファイルの実験記録
│   │   ├── README.md            # 実験ノートの書式と運用規則
│   │   └── vXXX.md              # 派生元、事前登録、判定、考察
│   └── backlog.md               # 実験アイデアと確定知見の台帳
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

持ち込みコードの提出時の挙動を保持する移植では、原版の計時と探索処理を非LOCAL側に保持し、Mac向けの時間補正と計測をLOCAL側に置く。両ビルドで変更の有効範囲を確認する。非LOCALだけを変えた版のLOCALスコア差は、その変更の性能効果を示さない。確認手順は [AGENTS.mdの移植規則](AGENTS.md#持ち込みコードの挙動を保持する移植) に従う。

## 実験の流れ

1. 着手前に `notes/backlog.md` と `notes/experiments/` のファイル名および Front Matter を照合し、`notes/experiments/vXXX.md` に事前登録する
2. 共通土台は `src/bin/v000_template.cpp` に、試行錯誤する solver は `src/bin/v001_*.cpp` 以降に書く
3. `./scripts/run.sh` で単発確認する
4. `./scripts/eval.py` で公式スコアを確認する
5. 判定、結果、考察を同じ `notes/experiments/vXXX.md` に追記し、`notes/backlog.md` の状態を更新する
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

プロジェクト直下で起動する。初回は依存関係の導入と WASM のビルドを自動で行う。

```bash
./scripts/dev_vis.sh
```

表示された URL の `/` でケースを可視化し、`/eval.html` で評価結果を比較できる。
`/play.html` では人間がプレイできる。AIも同じサーバーに `node scripts/play.mjs` から接続できる。
盤面・塔の積み順・帰巣数・各操作時点のスコアには、`tools/src` の公式処理を使用する。
スコアは小さいほどよい。ケースを選択し、保存済みの出力または貼り付けた出力をコマ送り・再生できる。
実行ボタンは選択した C++ solver をビルドして実行する。

評価一覧は `results/eval_records.jsonl` を読み、盤面サイズ `N`、色数 `K`、スライム数 `M`、壁数でケースを並べ替えられる。
これらの情報は入力ファイルから取得するため、評価ログの形式は変わらない。

WASM の実装や公式ツールを変更した後は、次のコマンドで再ビルドする。

```bash
./scripts/build_wasm.sh
corepack yarn build
```

プレイ用APIもWASMを読み込むため、WASMの変更後は `dev_vis.sh` を再起動する。起動時にソースが生成物より新しければ、WASMを自動で再ビルドする。

## validation1の固定参照

`tools/validation1` の相対評価用参照は [固定参照の説明](notes/references/README.md) にまとめる。別環境でもGitで共有したCSVと操作列を使い、`python3 adhoc/scripts/run_v802_reference.py --install` で評価画面へ復元できる。

## 人間とAIによるプレイ

起動は可視化と共通の `./scripts/dev_vis.sh` を使う。`Play` タブの「新しいプレイ」で入力ケースを選ぶ。塔を選択し、必要なら跳ばす部分を指定すると、着地点に重ねて結果を予告できる。着地点のクリックで実行する。巻き戻し、分岐、再生、メモも利用できる。

記録は `results/play/` に自動保存され、再起動後も続きから再開できる。画面の「IDをコピー」で取得したプレイIDをAIに渡せば、同じ局面を引き継げる。

```sh
node scripts/play.mjs info
node scripts/play.mjs create --case 0000.txt --label '積み順を試す'
node scripts/play.mjs --help
```

操作方法、JSONの形式、HTTP APIの仕様は [プレイの使い方](docs/play.md) にまとめてある。
