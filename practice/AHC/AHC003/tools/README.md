# Usage

## Requirements
Please install a compiler for Rust language (see https://www.rust-lang.org).

## Input Generation
Prepare `seeds.txt` which contains a list of random seeds (unsigned 64bit integers) and execute the following command.
```
cargo run --release --bin gen seeds.txt
```

This will output input files into `in` directory.

## Testing
Let `in.txt` be an input file and `cmd` be a command to run your program.
You can test your program by executing the following command.
```
cargo run --release --bin tester in.txt cmd
```
You can specify arguments to your program by 
```
cargo run --release --bin tester in.txt cmd arg1 arg2 ...
```

The above command outputs the output of your program to standard output and outputs the score to standard error.
You can visualize your output by running the following command and pasting the contents of `in.txt` and `out.txt` to the visualizer (https://img.atcoder.jp/ahc003/e7eb814463364c249c93216eee64275.html).
```
cargo run --release --bin tester in.txt cmd > out.txt
```

## Visualize
Let `in.txt` be an input file and `out.txt` be an output file.
You can visualize the output by executing the following command.
```
cargo run --release --bin vis in.txt out.txt
```
The above command writes a visualization result to `out.svg`.
It also outputs the score to standard output.
You can open the svg file using image viewers, web browsers, or via `vis.html` file.
A point at coordinates (x, y) indicates that in the x-th query, the length of the shortest path divided by the length of your output path is y.
The web visualizer (https://img.atcoder.jp/ahc003/e7eb814463364c249c93216eee64275.html) is more rich in features and you can check the edge lengths, shortest paths, and output paths.

# 使い方

## 実行環境
Rust言語のコンパイル環境が必要です。
https://www.rust-lang.org/ja を参考に各自インストールして下さい。

## 入力生成
`seeds.txt` に欲しい入力ファイルの数だけ乱数seed値(符号なし64bit整数値)を記入し、以下のコマンドを実行します。
```
cargo run --release --bin gen seeds.txt
```

生成された入力ファイルは `in` ディレクトリに出力されます。

## テスト実行
入力ファイル名を`in.txt`、あなたのプログラムの実行コマンドを`cmd`としたとき、以下のコマンドを実行します。
```
cargo run --release --bin tester in.txt cmd
```
引数が必要な場合には
```
cargo run --release --bin tester in.txt cmd arg1 arg2 ...
```
のようにします。

あなたのプログラムの出力はそのまま標準出力に書き出されます。
以下のコマンドを実行して出力を `out.txt` に書き出し、`in.txt` と `out.txt` の中身をビジュアライザ (https://img.atcoder.jp/ahc003/e7eb814463364c249c93216eee64275.html) の該当欄に貼り付けることで結果をビジュアライズ出来ます。
```
cargo run --release --bin tester in.txt cmd > out.txt
```

## ビジュアライザ
入力ファイル名を`in.txt`、出力ファイル名を`out.txt`としたとき、以下のコマンドを実行します。
```
cargo run --release --bin vis in.txt out.txt
```
出力のビジュアライズ結果は `out.svg` というファイルに書き出されます。
標準出力にはスコアを出力します。
svgファイルは画像ビューアソフト、webブラウザなどで表示できます。
`vis.html` ファイルを開くことでも表示できます。
座標 (x,y) にある点は、x 番目のクエリにおいて、最短路長 / 出力のパス長が y であったことを示しています。
Web版のビジュアライザ (https://img.atcoder.jp/ahc003/e7eb814463364c249c93216eee64275.html) はより機能が豊富で、辺の長さや、各最短路・出力パスを確認できます。
