# masters2026-qual Rust/C++ Workspace Guide

## 目的
- AtCoder提出用のRust/C++23開発を、提出1ファイル制約に合わせて安全に回す。
- 実験コードを複数残しつつ、Rustは`src/main.rs`、C++は`cpp/main.cpp`を常に提出候補として保つ。

## ディレクトリ構成
```text
masters2026-qual/
├── problem_description.txt
├── rust-toolchain.toml
├── Cargo.toml
├── Cargo.lock
├── .cargo/
│   └── config.toml
├── src/
│   ├── main.rs
│   └── bin/
│       └── v001_template.rs
├── cpp/
│   ├── main.cpp
│   └── bin/
│       └── v001_template.cpp
├── scripts/
│   ├── cpp_common.sh
│   ├── run.sh
│   ├── run_cpp.sh
│   ├── promote.sh
│   └── promote_cpp.sh
├── notes/
│   └── ideas.md
├── results/
│   └── scores.csv
└── tools/
    └── tester/
        └── README.md
```

## 基本方針
- `problem_description.txt`
  - 問題文の原文または要約を記録する。
  - 制約・スコアリング・入出力形式を最初に整理して残す。
- `src/main.rs`
  - 提出直前の1本だけを置く。
- `src/bin/*.rs`
  - アイデアごとに1ファイルで管理する。
  - 提出制約に合わせ、各ファイルは単体で完結させる。
- `cpp/main.cpp`
  - C++提出直前の1本だけを置く。
- `cpp/bin/*.cpp`
  - C++のアイデアごとに1ファイルで管理する。
  - 提出制約に合わせ、各ファイルは単体で完結させる。
- `results/scores.csv`
  - Rust/C++の実行ログとスコアを蓄積する。

## scripts/run.sh
### 役割
- 実験用binを実行し、`results/scores.csv` にログを追記する。

### 使い方
```bash
./scripts/run.sh <bin_name> [input_file] [score]
```

### 例
```bash
./scripts/run.sh v001_template
./scripts/run.sh v001_template ./tools/tester/in/0000.txt 123456
```

### 出力ログ列
- `timestamp,bin,input,elapsed_sec,score`

## scripts/promote.sh
### 役割
- 指定した `src/bin/<bin_name>.rs` を `src/main.rs` に昇格する。
- 昇格後に `cargo build --release --quiet --offline` を実行し、提出互換ビルドを確認する。

### 使い方
```bash
./scripts/promote.sh <bin_name>
```

### 例
```bash
./scripts/promote.sh v001_template
```

## scripts/run_cpp.sh
### 役割
- C++実験用binをコンパイルして実行し、`results/scores.csv` にログを追記する。
- `g++ / clang++ / c++` の順でコンパイラを自動検出する（`CXX`指定時はそれを優先）。
- `-std=gnu++23` を優先し、未対応環境では `-std=c++23`, `-std=gnu++2b`, `-std=c++2b` にフォールバックする。

### 使い方
```bash
./scripts/run_cpp.sh <bin_name> [input_file] [score]
```

### 例
```bash
./scripts/run_cpp.sh v001_template
./scripts/run_cpp.sh v001_template ./tools/tester/in/0000.txt 123456
```

### 出力ログ列
- `timestamp,bin,input,elapsed_sec,score`
- `bin` 列は `cpp/<bin_name>` で記録される。

## scripts/promote_cpp.sh
### 役割
- 指定した `cpp/bin/<bin_name>.cpp` を `cpp/main.cpp` に昇格する。
- 昇格後にC++23相当フラグでコンパイルし、提出互換ビルドを確認する。

### 使い方
```bash
./scripts/promote_cpp.sh <bin_name>
```

### 例
```bash
./scripts/promote_cpp.sh v001_template
```

## 推奨ワークフロー
1. Rustなら`src/bin/vXXX_*.rs`、C++なら`cpp/bin/vXXX_*.cpp`を追加して実装する。
2. Rustは`./scripts/run.sh ...`、C++は`./scripts/run_cpp.sh ...`で試し、`results/scores.csv`に記録する。
3. 良い案をRustは`./scripts/promote.sh ...`、C++は`./scripts/promote_cpp.sh ...`で提出用ファイルに反映する。
4. 提出言語に応じて`src/main.rs`または`cpp/main.cpp`をAtCoderに提出する。

## 注意
- 提出はsingle fileなので、最終提出コードはRustなら`src/main.rs`、C++なら`cpp/main.cpp`単体で完結している必要がある。
- `rust-toolchain.toml` は `1.89.0` 固定で、AtCoder環境とのズレを減らす。
