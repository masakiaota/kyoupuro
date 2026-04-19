# Heuristic Contest Template

このディレクトリは、AtCoder Heuristic Contest 用の作業テンプレートである。  
解法実装、実験、採点、visualizer をこのディレクトリの中だけで進める前提で作ってある。

## ディレクトリ構成
主要なものだけ示す。

```text
_template_heuristic/
├── README.md
├── AGENTS.md
├── problem_description.txt
├── Cargo.toml
├── .agents/
│   └── skills/
│       └── make-visualizer/
│           └── SKILL.md
├── src/
│   └── bin/
│       ├── v000_template.rs
│       └── crate_check.rs
├── scripts/
├── notes/
│   ├── important_properties.md
│   └── notations.md
├── results/
│   ├── score_summary.csv
│   └── out/
├── samples/
├── tools/
├── src_vis/
├── wasm/
└── public/
    └── wasm/
```

## 役割
### ルート
- `problem_description.txt`
  - 問題文、入出力、スコア、制約、初動メモを書く。

### 解法・実験
- `src/bin/v000_template.rs`
  - 問題理解の過程で確定した共通土台を置く。
  - 例: `State`、問題のルール再現、基本遷移、制約判定、整合性チェック、reference 実装。
- `src/bin/v001_*.rs` 以降
  - 探索戦略、評価関数、パラメータ、枝刈りなど、試行錯誤する solver を置く。提出時はこの中のファイルを直接使う。
- `results/score_summary.csv`
  - score の要約ログを追記する。評価イベント単位で集約結果を残す。
- `results/out/<bin_name>/`
  - `run.sh` や `eval.sh` 実行時の出力を保存する。`bin` ごとにフォルダ分けされ、評価 dataset ごとに `results/out/<bin_name>/<eval_set>/` を使う。
- `notes/`
  - 問題固有のアイデア、重要な性質、観察結果を書く。
- `notes/notations.md`
  - 問題で使う記号と、コード上の代表名・型・制約をまとめる正本である。
- `notes/important_properties.md`
  - 問題から導かれる重要な性質、不変量、探索や構築で効く性質を整理する正本である。

### contest assets
- `tools/`
  - 公式 generator / tester / scorer を展開する場所である。
- `samples/`
  - サンプル input / output を置く場所である。

### visualizer
- `.agents/skills/make-visualizer/SKILL.md`
  - visualizer 実装時に AI が従う手順である。
- `src_vis/main.js`
  - Vite 側の UI ロジックとローカル API 連携を書く。
- `wasm/src/impl_vis.rs`
  - 問題固有の visualizer 実装本体である。

## 基本的な使い方
### 最初にやること
1. `problem_description.txt` を埋める
2. 公式配布物を `tools/` と `samples/` に置く
3. 並列評価できるように `scripts/eval.sh` が contest の scoring tool 呼び出し方に対応するように編集。
4. 必要な記号を `notes/notations.md` に早めに書き出し、命名と型の正本を固める。
5. 見えてきた重要な性質や不変量を `notes/important_properties.md` に整理する。
6. 必要なら visualizer もつくる (適宜改善)。
7. `src/bin/v000_template.rs` に共通土台を整え、実験用 solver は `v001_*.rs` 以降として追加する

### 実験の流れ
1. 共通土台は `src/bin/v000_template.rs` に書き、試行錯誤する solver は `src/bin/v001_*.rs` 以降に書く
2. `./scripts/run.sh <bin_name> [input_file]` で単発確認する
   - `input_file` が `tools/in`, `tools/inB`, `tools/in_generated` 配下なら、対応する `results/out/<bin_name>/<eval_set>/` に出力を保存する。
3. scorer があるなら `./scripts/eval.sh [-v] [-j jobs|--serial] <bin_name> [input_dir]` で公式スコアを確認する
   - `input_dir` 省略時は、存在する既定 dataset (`tools/in`, `tools/inB`) を順に評価する。
   - 既定では各ケースについて `run -> score` を `cpu//2` 並列で実行する。
   - 発熱や計測ぶれを避けたいときは `-j 1` か `--serial` で直列実行する。
   - 出力は `results/out/<bin_name>/<eval_set>/` に保存し、要約は dataset ごとに `results/score_summary.csv` に追記する。
4. 提出時は対象の `src/bin/<bin_name>.rs` を直接コピーして使う

## shell script の役割
- `./scripts/run.sh <bin_name> [input_file]`
  - stdin または 1 つの input_file に対して手動実行する。
- `./scripts/eval.sh [-v] [-j jobs|--serial] <bin_name> [input_dir]`
  - solver と公式 `score` を 1 回だけ build し、ケース単位で `run -> score` を実行する。
  - `input_dir` 省略時は、存在する既定 dataset (`tools/in`, `tools/inB`) を順に評価する。
  - 既定ジョブ数は `cpu//2` で、`-j 1` か `--serial` で直列評価に切り替えられる。
  - 出力は `results/out/<bin_name>/<eval_set>/` に保存し、要約は `results/score_summary.csv` に追記する。
- `./scripts/gen_tools.sh <args...>`
  - 公式 `tools` の `gen` バイナリをラップする。追加入力生成用である。
- `./scripts/unpack_tools.sh [tools_zip_path]`
  - `tools.zip` などの公式配布 zip を `tools/` に展開する。
- `./scripts/build_wasm.sh`
  - `wasm-pack build --target web --out-dir ../public/wasm` を実行し、browser 用 WASM を更新する。
- `./scripts/dev_vis.sh`
  - 必要なら `yarn install` を行い、Vite の開発サーバーを起動する。

## よく使うコマンド
```bash
./scripts/run.sh <bin_name>
./scripts/run.sh <bin_name> ./tools/in/0000.txt
./scripts/eval.sh <bin_name>
./scripts/eval.sh -j 1 <bin_name>
./scripts/eval.sh -v <bin_name>
./scripts/unpack_tools.sh ./tools.zip
./scripts/build_wasm.sh
./scripts/dev_vis.sh
cargo run --bin crate_check
```

## Visualizer の使い方
- まず `problem_description.txt` と `tools/src/` を揃える
- `wasm/src/impl_vis.rs` に問題固有の描画ロジックを入れる
- `./scripts/build_wasm.sh` で `public/wasm/` を更新する
- `./scripts/dev_vis.sh` でローカル server を立ち上げる
- `src_vis/main.js` には Rust bin 実行 UI と SVG 表示 UI が入っている
