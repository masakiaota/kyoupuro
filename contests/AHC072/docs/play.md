# 人間とAIによるプレイ

`./scripts/dev_vis.sh` で起動し、表示されたURLの `/play.html` を開く。既定の接続先は `http://127.0.0.1:5173` である。ゲームの状態更新、合法手の判定、採点には公式ツールの `State` を使う。

## 画面で操作する

「新しいプレイ」で入力ケースを選ぶ。入力テキストや入力ファイルも使用できる。操作列を一緒に読み込むと、その最終局面から続けられる。

盤面の塔をクリックすると、跳べるマスに枠が付く。複数匹の塔では、塔の近くの拡大表示から跳ばす範囲を選べる。選んだ段から上が跳び、下の `k` 匹が残る。1匹の塔には範囲を選ぶ操作は不要である。

着地点にマウスを重ねると、出発点と着地点の積み順、帰巣する匹数を予告する。その着地点をクリックすると一手進む。別の実行ボタンやEnterによる確定は不要である。実行後も結果を右側に残すため、変化を見返せる。

選択中の塔を再クリックするか、盤面を右クリックすると選択を解除できる。着地点以外の空マスをクリックしても解除する。「一手戻す」と「やり直す」はマウスで操作でき、やり直し先が複数ある場合だけ分岐を選ぶ。

積み順は選択時も結果表示も縦に並び、下側が塔の底になる。盤面ではスライムと巣の色を数字でも表示する。複数匹の塔の右上には「最上段の色 / 高さ」を示す。盤面の位置と寸法は、通知や予告の内容によって変わらない。

| キー | 操作 |
|---|---|
| 矢印キー | 選択した塔の着地点を選ぶ。同じ方向を繰り返すと距離が変わる |
| Enter | プレビュー中の一手を確定する |
| Esc | 塔の選択を解除する |
| ⌘ / Ctrl Z | 一手戻す |
| ⌘ / Ctrl Shift Z | やり直す。複数の分岐があれば選択欄で選ぶ |

履歴や手数のスライダーを操作すると、過去の局面を閲覧できる。「表示する手順」で分岐を切り替えると、選んだ手順を初期盤面から順番に表示する。途中の履歴行を選んでも終端は保持されるため、その前後へ進める。この閲覧と再生では保存された現在位置は変わらない。「ここからプレイ」を押して別の操作をすると分岐ができ、元の手順も残る。

気づきは「この局面のメモ」に保存する。「次の一手にメモ」を開くと、操作に添える文章も書ける。どちらの下書きも記録と局面ごとにブラウザへ保持し、記録の切り替えや再読み込みで復元する。下書きはプロジェクトへの保存とは区別して表示する。過去の局面を閲覧しているときは、同じタブで別の記録から戻ると閲覧位置も復元する。

「操作列を書き出す」は閲覧中の局面までを公式出力形式で保存し、「記録を保存」は全分岐と保存済みメモを含むJSONを保存する。ブラウザの下書きは書き出しに含まれない。

## AI用コマンド

ブラウザと同じサーバーに、`scripts/play.mjs` からアクセスする。ブラウザが開いていなくても、サーバーが起動していれば操作できる。すべての座標と色は0始まりである。

```sh
./scripts/dev_vis.sh
```

別の端末から、入力ケースを選んで開始する。

```sh
node scripts/play.mjs info
node scripts/play.mjs create --case 0000.txt --label '積み順を確認する'
```

作成結果の `session_id` を以後の要求に指定する。画面の「IDをコピー」で得たIDを使うと、人間のプレイを引き継げる。次の `p_...` と座標は説明用の値であり、実際の状態に合わせて指定する。

```sh
node scripts/play.mjs state --session p_...
node scripts/play.mjs legal --session p_... --i 5 --j 7
node scripts/play.mjs legal --session p_... --i 5 --j 7 --k 2
node scripts/play.mjs preview --session p_... --revision 12 \
  --action '{"i":5,"j":7,"k":2,"d":"R","l":3}'
node scripts/play.mjs step --session p_... --revision 12 \
  --action '{"i":5,"j":7,"k":2,"d":"R","l":3}' \
  --note '出発点でも帰巣することを確認する'
```

`--revision` には状態取得時の `revision` を指定する。`preview` は局面を変更しないため、直後の `step` に同じ番号を使える。`step` の成功後は返された新しい番号を使う。

JSONで要求を組み立てる場合は、`--json` を使う。`--json -` は標準入力から読み込む。

```sh
node scripts/play.mjs step --json '{"session_id":"p_...","expected_revision":12,"action":{"i":5,"j":7,"k":2,"d":"R","l":3},"note":"積み順の確認"}'
node scripts/play.mjs history --session p_... --offset 0 --limit 100
node scripts/play.mjs state --session p_... --node n20 --turn 10
node scripts/play.mjs checkout --session p_... --revision 13 --node n10
node scripts/play.mjs note --session p_... --revision 14 --node n10 --text 'ここから別の距離を試す'
node scripts/play.mjs export --session p_... --format json > play-record.json
node scripts/play.mjs export --session p_... --node n20 --format output > output.txt
```

既存の解答から開始する場合は、`create --input 入力ファイル --output 解答ファイル` を使う。サーバーを別のポートで起動した場合は、各コマンドに `--url http://127.0.0.1:ポート番号` を付けるか、`AHC_PLAY_URL` を設定する。

成功時は終了コード0、失敗時は1を返す。通常の応答とエラーは標準出力にJSONで返す。`export --format output` だけは公式形式のテキストを返す。

## HTTP API

すべてのAPIは `/api/play/` 以下にある。POSTの本文はJSONオブジェクトとし、`Content-Type: application/json` を付ける。

| メソッドとパス | 指定する内容 | 結果 |
|---|---|---|
| `GET info` | なし | 入力ケース、保存済みプレイ、読込エラー |
| `GET sessions` | なし | 保存済みプレイの一覧 |
| `POST sessions` | `case_name` または `input`。任意で `output`, `label`, `actor` | 作成したプレイの状態 |
| `GET sessions/:id/state` | 任意で `node_id`, `turn`, `svg=1` | 現在または過去の状態 |
| `GET sessions/:id/version` | なし | 現在の `revision` と `node_id` |
| `GET sessions/:id/legal` | `i`, `j`。任意で `k`, `expected_revision` | 合法な `actions` と現在位置 |
| `POST sessions/:id/preview` | `expected_revision`, `action` | 一手の結果予測 |
| `POST sessions/:id/step` | 共通の更新用項目と `action`。任意で `note` | 確定した一手の結果 |
| `POST sessions/:id/checkout` | 共通の更新用項目と `node_id`。任意で `turn` | 移動後の現在位置と成績 |
| `POST sessions/:id/note` | 共通の更新用項目と `text`。任意で `node_id` | 保存したメモと更新番号 |
| `GET sessions/:id/history` | 任意で `offset`, `limit`, `node_id` | 既定は新しい順の全分岐。`node_id` を指定するとその局面までの手順を古い順に返す。既定100件、最大500件 |
| `GET sessions/:id/export` | 任意で `format=json\|output`, `node_id` | 記録全体または公式形式の操作列 |

**更新用の共通項目**は `expected_revision` と `request_id` である。操作者は `actor` で指定でき、省略時は `ai`、画面からの要求では `human` となる。`request_id` は要求ごとに一意の文字列にする。コマンドから使う場合は自動生成される。

**局面のID**を表す `node_id` は、そのプレイ内で一意である。最初の盤面は `n0` であり、分岐後も既存の局面を上書きしない。`T` はその局面までの手数、`revision` はプレイ全体の更新番号である。巻き戻しとメモの追加でも `revision` は増える。

`state` の `node_id` は閲覧対象、`live_node_id` は保存された現在位置を示す。`node_id` を省略すると現在位置を読む。`turn` を併用すると、その局面に至る分岐の指定手数目を読む。過去の局面から操作するには、まず `checkout` で現在位置を移す。

`history` の `branches` は保存済みの各手順の終端を返す。各要素の `node_id` と `T` は終端、`fork_T` は直近の分岐元の手数、`contains_live` は保存された現在位置がその手順に含まれるかを示す。

### 状態の形式

状態には次の項目が含まれる。`svg` は `svg=1` を指定した画面用の要求だけに含まれる。

| 項目 | 内容 |
|---|---|
| `schema_version` | 形式の版。現在は1 |
| `session_id`, `label`, `case_name` | プレイの識別情報 |
| `node_id`, `live_node_id`, `revision`, `is_live` | 閲覧対象と現在位置 |
| `N`, `K`, `M`, `T`, `E`, `S`, `completed` | 問題の状態量と完了判定 |
| `walls` | 壁の座標 `[i, j]` の配列 |
| `nests` | `{i, j, color}` の配列 |
| `towers` | 空でない塔の `{i, j, colors}` の配列。`colors` は下から上 |
| `received`, `initial_counts` | 色番号を添字とする帰巣数と初期匹数 |
| `parent_id`, `children` | 前の局面と、そこから進める保存済みの分岐 |
| `last_action`, `actor`, `notes` | この局面を作った操作と操作者、局面のメモ |

### 操作結果の形式

`preview` と `step` は、正規化した `action`、着地点 `destination`、跳んだ匹数 `moved`、操作後の `T`, `E`, `S`, `completed` を返す。`delta` には `T`, `E`, `S` の増減が入る。

`changed_cells` は出発点、着地点の順に2要素を返す。各要素には座標、巣の色、操作前の `before` と帰巣後の `after` が入る。巣がない場合、`nest` は `null` である。

`returned.source` と `returned.destination` には、それぞれの地点で帰巣した匹数 `count` と色配列 `colors` が入る。帰巣がなければ `count=0`, `colors=[]` となる。

`preview` の `node_id` と `revision` は予測元の値である。`step` では、保存後の新しい値を返す。

### エラーと再送

不正な操作は適用せず、手数も増やさない。エラー本文は次の形式で返す。

```json
{
  "error": {
    "code": "height_limit_exceeded",
    "message": "帰巣前の着地直後の高さが8を超える"
  }
}
```

主なエラーは `invalid_action`, `invalid_source`, `jump_too_far`, `outside_board`, `wall_on_path`, `height_limit_exceeded`, `operation_limit` である。入力や要求の不正にはHTTP 400、存在しない記録や局面には404を返す。

`expected_revision` が現在値と違う場合はHTTP 409と `revision_conflict` を返す。状態を再取得してから次の操作を決める。人間の画面は約1.5秒ごとにAIによる更新を確認する。塔の選択中、メモの下書きがあるとき、過去の局面を閲覧中は画面を入れ替えず、「最新の局面を表示」から更新する。

同じ `request_id` で同じ更新要求を再送すると、最初の応答を返す。同じ手は二度適用しない。要求内容を変えた再送は `request_id_conflict` となる。応答が途切れた場合、コマンドは `request_id` をエラーと一緒に返すので、`--request-id` に指定して同じ要求を再送できる。再送時の結果は最初の更新番号であるため、以後の操作前に必要に応じて状態を取得する。

## 保存先と再開

プレイ記録は `results/play/<session_id>/events.jsonl` に保存される。初期盤面と読み込んだ操作列、以後の操作、現在位置の変更、メモを追記し、保存成功後にAPIの成功を返す。初期記録には入力のハッシュと使用したWASMのハッシュも残す。

同じプロジェクトで `dev_vis.sh` を再起動すると、操作列を公式処理で再生して現在位置を復元する。ブラウザでは記録一覧から再開できる。初期盤面を記録内に保持するため、元の入力ファイルが変更されても以前の記録は同じ初期盤面から再生される。

この保存先は `eval.py` の出力先から独立している。`events.jsonl` が不完全な場合は読込エラーを表示し、元の記録を保持する。JSON書き出しには全イベントと初期入力、選択した局面までの操作列が含まれる。

保存に失敗した場合はHTTP 503と `storage_unavailable` を返し、その記録への更新を停止する。保存先の問題を解消してサーバーを再起動すると、保存済みの記録から再開する。

## 実装の確認

以下はゲーム基盤の固定ケースを使った確認であり、solverの実行を含まない。

```sh
cargo test --manifest-path wasm/Cargo.toml
./scripts/build_wasm.sh
corepack yarn test:play
corepack yarn build
```

ルールの境界条件に加え、プレビューの非破壊性、公式スコアとの一致、分岐とメモの復元、更新の競合、要求の再送、CLIとHTTPの接続を確認する。
