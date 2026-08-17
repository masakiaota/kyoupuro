# subagent による解説検証

このファイルは、Phase 2 で subagent を起動するときに使う。
役割は、検証プロンプト、参照を許可するファイル、参照を禁止するファイル、checksum の記録方法を固定することである。

検証では、subagent が元 solver を読まずに、解説だけから目標スコアを超える solver を作れるかを確認する。
検証対象は、`writing-checklist.md` で点検した後の解説本文である。
未点検の本文で検証し、その後で本文を整える順序にしてはいけない。

Phase 1 の完成 solver は、親 agent が保持する固定された正本かつベンチマークである。
subagent solver は解説を測る使い捨てプローブであり、改善対象や次の検証のベースではない。

検証後に本文の意味を変えた場合、その検証結果は失効する。
意味を変える編集をしたら、別の新規 subagent で再検証する。

## 検証条件の汚染を避ける

必ず新規 subagent を使う。
以前に対象 solver や失敗実装を読んだ subagent を再利用しない。

検証用の subagent には、元 solver、関連する過去版、再現済み solver を読ませない。
`rg src/bin` や `sed src/bin/*.cpp` のように、禁止 solver が出力されうる探索も禁止する。

参照を許可するファイルを列挙する。
通常は次に絞る。

- `problem_description.txt`
- `notes/notations.md`
- `src/bin/v000_template.cpp`
- 解説 Markdown
- `scripts/eval.py`
- `scripts/run.sh`
- `scripts/build_solver.sh`

## 検証プロンプト

次の形を基準にする。

```text
作業ディレクトリは <project root> である。

目的: <解説ファイル> の再現性を、新規実装で検証する。
解説だけを読んで C++ solver を 1 本実装し、評価して total_avg が <threshold> を超えるか確認する。
この <解説ファイル> は、文章点検後の検証対象版である。

厳守事項:
- src/bin 以下の既存 solver を参照してはいけない。
- 読んでよい src/bin の既存ファイルは src/bin/v000_template.cpp だけである。
- <対象 solver>、過去の再現 solver、その他の既存 solver を読まない。
- rg src/bin や sed src/bin/*.cpp のように、v000 以外が出力されうる探索をしない。
- 実装ファイルとして新規に作ってよいのは <new solver file> のみである。
- 他の人の変更を戻さない。

読んでよい主なファイル:
- problem_description.txt
- notes/notations.md
- src/bin/v000_template.cpp
- <解説ファイル>
- scripts/eval.py, scripts/run.sh, scripts/build_solver.sh

やること:
1. <解説ファイル> と問題文を読む。
2. <new solver file> を実装する。
3. <eval command> を1回実行する。ジョブ数は指定しない。
4. final で、読んだファイル、変更したファイル、実行した評価コマンド、total_avg、成功可否、実装中に解説で曖昧だった点、試行錯誤の回数と内容を報告する。

full 評価を開始した後は、成功・失敗にかかわらずコードを変更せず、再評価せず、結果を報告して終了すること。
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

閾値や時間条件を満たさなければ、subagent の報告を受け取ってその検証を終了する。

親 agent が Phase 1 の完成 solver、解説、subagent solver を比較し、解説不足と subagent 単独の実装ミスを分ける。
解説不足の場合だけ本文を修正する。
subagent 単独の実装ミスを理由に、本文へ注意事項や実装細部を追加しない。

失敗した subagent solver は修正、改善、再評価せず、次の検証にも流用しない。
本文を修正した場合は、別の新規 subagent が新しい solver を最初から実装する。

## 成功時の扱い

閾値を超えても、次を確認する。

- full 評価を何回実行したか。
- 評価後にパラメータを調整したか。
- 構文修正とスコア改善を区別できているか。
- 解説で曖昧だった判断は何か。

初回に近い実装で閾値を超えた場合、解説は再現性を持つ可能性が高い。
何度も評価して最後に閾値を超えた場合、本文へ運用原則を戻す。
本文へ運用原則を戻したら、それは意味のある編集なので再検証する。

成功結果も Phase 1 の完成 solver と比較する。
成功した subagent solver を新しいベンチマークや次の実装のベースにしない。
