- [Usage](#usage)
  - [Requirements](#requirements)
  - [Input Generation](#input-generation)
  - [Visualization](#visualization)
  - [Porting to other languages](#porting-to-other-languages)
- [使い方](#%E4%BD%BF%E3%81%84%E6%96%B9)
  - [実行環境](#%E5%AE%9F%E8%A1%8C%E7%92%B0%E5%A2%83)
  - [入力生成](#%E5%85%A5%E5%8A%9B%E7%94%9F%E6%88%90)
  - [ビジュアライザ](#%E3%83%93%E3%82%B8%E3%83%A5%E3%82%A2%E3%83%A9%E3%82%A4%E3%82%B6)
  - [他言語への移植](#%E4%BB%96%E8%A8%80%E8%AA%9E%E3%81%B8%E3%81%AE%E7%A7%BB%E6%A4%8D)

# Usage

## Requirements
Please install a compiler for Rust language (see https://www.rust-lang.org).
If a compile error occurs, the compiler version may be old.
You can update to the latest compiler by executing the following command.
```
rustup update
```

For those who are not familiar with the Rust language environment, we have prepared a [pre-compiled binary for Windows](https://img.atcoder.jp/ahc072/BrsBAXF2_windows.zip).
The following examples assume that you will be working in the directory where this README is located.

## Input Generation
The `in` directory contains pre-generated input files for seed=0-99.
If you want more inputs, prepare `seeds.txt` which contains a list of random seeds (unsigned 64bit integers) and execute the following command.
```
cargo run -r --bin gen seeds.txt
```
This will output input files into `in` directory.
When using the precompiled binary for Windows, execute the following command.
```
./gen.exe seeds.txt
```
If you use the command prompt instead of WSL, use `gen.exe` instead of `./gen.exe`.


Run one of the following commands for the complete and up-to-date list of options.
```
cargo run -r --bin gen -- --help
./gen.exe --help
```

## Visualization
Let `in.txt` be an input file and `out.txt` be an output file.
You can visualize the output by executing the following command.
```
cargo run -r --bin vis in.txt out.txt
```
When using the precompiled binary for Windows,
```
./vis.exe in.txt out.txt
```

The above command writes a visualization result to `vis.html`.
It also outputs the score to standard output.
If you only need the score, `--no-vis` skips writing `vis.html`.
`vis.html` shows the last visualization position; `-t <TURN>` selects another position instead.

You can also use a [web visualizer](https://img.atcoder.jp/ahc072/BrsBAXF2.html?lang=en) which is more rich in features.

## Porting to other languages
If you want to port the scoring function to another language, `src/lib.rs` is the only file you need.
Input generation, validation of the input/output format, and the score computation are all contained in that single file.
`src/vis.rs` is only used by the visualizer and does not need to be ported.

# 使い方

## 実行環境
Rust言語のコンパイル環境が必要です。
https://www.rust-lang.org/ja を参考に各自インストールして下さい。
コンパイルエラーになった場合、コンパイラのバージョンが古い可能性があります。
以下のコマンド実行することで最新のコンパイラに更新が可能です。
```
rustup update
```

Rust言語の環境構築が面倒な方向けに、[Windows用のコンパイル済みバイナリ](https://img.atcoder.jp/ahc072/BrsBAXF2_windows.zip)も用意してあります。
以下の実行例では、このREADMEが置かれているディレクトリに移動して作業することを想定しています。

## 入力生成
`in` ディレクトリに予め生成された seed=0~99 に対する入力ファイルが置かれています。
より多くの入力が欲しい場合は、`seeds.txt` に欲しい入力ファイルの数だけ乱数seed値(符号なし64bit整数値)を記入し、以下のコマンドを実行します。
```
cargo run -r --bin gen seeds.txt
```
生成された入力ファイルは `in` ディレクトリに出力されます。
Windows用のコンパイル済バイナリを使用する場合は以下のようにします。
```
./gen.exe seeds.txt
```
WSLではなくコマンドプロンプトを使用する場合は `./gen.exe` ではなく `gen.exe` として下さい。

利用可能なオプションの完全かつ最新の一覧は、次のいずれかを実行して確認して下さい。
```
cargo run -r --bin gen -- --help
./gen.exe --help
```

## ビジュアライザ
入力ファイル名を`in.txt`、出力ファイル名を`out.txt`としたとき、以下のコマンドを実行します。
```
cargo run -r --bin vis in.txt out.txt
```
Windows用のコンパイル済バイナリを使用する場合は以下のようにします。
```
./vis.exe in.txt out.txt
```

出力のビジュアライズ結果は `vis.html` というファイルに書き出されます。
標準出力にはスコアを出力します。
スコアだけが必要な場合は `--no-vis` を付けると `vis.html` の書き出しを省略できます。
`vis.html` には表示軸の最後の位置が描かれます。`-t <TURN>` を付けると別の位置を選べます。

より機能が豊富な[ウェブ版のビジュアライザ](https://img.atcoder.jp/ahc072/BrsBAXF2.html?lang=ja)も利用可能です。

## 他言語への移植
スコア計算等を他言語へ移植したい場合、対象は `src/lib.rs` のみです。
入力生成・入出力形式の検証・スコア計算は、すべてこの 1 ファイルに含まれています。
`src/vis.rs` はビジュアライザ専用のため、移植は不要です。
