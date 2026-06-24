# subagent による解説検証

このファイルは、Phase 2 で subagent を起動するときに使う。
役割は、検証プロンプト、参照を許可するファイル、参照を禁止するファイル、checksum の記録方法を固定することである。

検証では、subagent が元 solver を読まずに、解説だけから目標スコアを超える solver を作れるかを確認する。
検証対象は、`writing-checklist.md` で点検した後の解説本文である。
未点検の本文で検証し、その後で本文を整える順序にしてはいけない。

検証後に本文の意味を変えた場合、その検証結果は失効する。
意味を変える編集をしたら、別の新規 subagent で再検証する。

## 検証条件の汚染を避ける

必ず新規 subagent を使う。
以前に対象 solver や失敗実装を読んだ subagent を再利用しない。

検証用の subagent には、元 solver、関連する過去版、再現済み solver を読ませない。
`rg src/bin` や `sed src/bin/*.rs` のように、禁止 solver が出力されうる探索も禁止する。

参照を許可するファイルを列挙する。
通常は次に絞る。

- `problem_description.txt`
- `notes/notations.md`
- `src/bin/v000_template.rs`
- 解説 Markdown
- `scripts/eval.py`
- `scripts/run.sh`
- `Cargo.toml`

## 検証プロンプト

次の形を基準にする。

```text
作業ディレクトリは <project root> である。

目的: <解説ファイル> の再現性を、新規実装で検証する。
解説だけを読んで Rust solver を 1 本実装し、評価して total_avg が <threshold> を超えるか確認する。
この <解説ファイル> は、文章点検後の検証対象版である。

厳守事項:
- src/bin 以下の既存 solver を参照してはいけない。
- 読んでよい src/bin の既存ファイルは src/bin/v000_template.rs だけである。
- <対象 solver>、過去の再現 solver、その他の既存 solver を読まない。
- rg src/bin や sed src/bin/*.rs のように、v000 以外が出力されうる探索をしない。
- 実装ファイルとして新規に作ってよいのは <new solver file> のみである。
- 他の人の変更を戻さない。

読んでよい主なファイル:
- problem_description.txt
- notes/notations.md
- src/bin/v000_template.rs
- <解説ファイル>
- scripts/eval.py, scripts/run.sh, Cargo.toml

やること:
1. <解説ファイル> と問題文を読む。
2. <new solver file> を実装する。
3. <eval command> を実行する。ジョブ数は指定しない。
4. final で、読んだファイル、変更したファイル、実行した評価コマンド、total_avg、成功可否、実装中に解説で曖昧だった点、試行錯誤の回数と内容を報告する。

評価が失敗した場合も、v000 以外の既存 solver を読まずに、解説と自分の実装だけから原因を調べること。
```

## 検証対象を記録する

subagent を起動する前に、検証対象のファイル名と内容を記録する。
可能なら checksum も残す。

```sh
shasum -a 256 <解説ファイル>
```

subagent の final には、読んだ解説ファイル名を必ず報告させる。
親 agent は、検証後の最終本文がこの検証対象と同一内容か確認する。
同一でない場合は差分を確認し、意味を変えない編集だけか分類する。
分類できない差分があれば、検証は失効する。

## 失敗時の扱い

閾値未満なら、まず検証用の subagent の報告から曖昧点を拾う。

ユーザーが許可した場合だけ、同じ subagent に元 solver を読ませて、解説との差分を分析させる。
この分析は解説改善のためにだけ使う。
再検証には、必ず別の新規 subagent を使う。

## 成功時の扱い

閾値を超えても、次を確認する。

- full 評価を何回実行したか。
- 評価後にパラメータを調整したか。
- 構文修正とスコア改善を区別できているか。
- 解説で曖昧だった判断は何か。

初回に近い実装で閾値を超えた場合、解説は再現性を持つ可能性が高い。
何度も評価して最後に閾値を超えた場合、本文へ運用原則を戻す。
本文へ運用原則を戻したら、それは意味のある編集なので再検証する。
