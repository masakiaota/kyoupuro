# unread implementation review instructions

## 目的

`source_solutions/*.md` のうち、実装コードを読めていない、または読めた実装情報が不十分なものを再調査する。

BrowserでAtCoder提出、提出一覧、GitHub、gist、記事内リンクを辿り、実装が取得できる場合はコードを読んで個別メモを更新する。実装が取れない場合でも、Browserで試したURLと理由を正確に残す。

単なる感想、リンク集、写経メモ、他人のコードを見たという話だけで、本文から解法の本質が分からない記事は残さない。低品質な参照元は削除し、`solution_urls.md` と `solution.md` からも参照を外す。

## Browser指定

この指示書でいう「Browser」は、必ず `[@Browser](plugin://browser-use@openai-bundled)` / `[$browser-use:browser](/Users/masaki/.codex/plugins/cache/openai-bundled/browser-use/0.1.0-alpha2/skills/browser/SKILL.md)` の in-app Browser だけを指す。

実装取得には必ずこのBrowserを使う。このBrowserはAtCoderにログイン済みの in-app Browser である。ログインしていない外部ブラウザ、通常のPlaywright、shell上のcurl、汎用web検索だけで「読めない」と判断してはいけない。

特にAtCoder提出一覧や提出詳細は、必ずこのBrowserで開いて確認する。直接提出URLが404でも、提出一覧 `https://atcoder.jp/contests/<contest>/submissions?f.User=<user>` から辿れる場合がある。

## 作業手順

1. 対象mdを読む。
   - 参照元URL
   - 現在の「コード有無」「コードを読めたか」「読めなかったもの」
   - 本文中の推測、不明点、断定できない箇所

2. 解説ブログ本文をBrowserで開く。
   - 状態表現、評価関数、探索、近傍、差分更新、出力、パラメータが具体的か確認する。
   - 本文が解法理解に十分でない場合は削除候補にする。

3. 実装リンクを探す。
   - 記事内のAtCoder提出リンク
   - AtCoder提出一覧 `https://atcoder.jp/contests/<contest>/submissions?f.User=<user>`
   - GitHub、gist、rawリンク
   - 記事内の補助リンク
   - Browserのログイン済みセッションでしか読めない提出一覧も確認する。

4. 実装を読む。
   - 主要な型、状態表現
   - 入力前処理
   - 評価関数
   - 探索、近傍、更新
   - 差分計算
   - 出力生成
   - 時間制限、パラメータ
   - ブログ本文との一致点、相違点

5. 更新または削除を判断する。
   - ブログ本文が有用で、実装も読めた: mdを実装ベースに更新する。
   - ブログ本文は有用だが、実装は読めない: Browserで試した結果と理由をmdに正確に書く。
   - ブログ本文が低品質: mdを削除し、`solution_urls.md` と `solution.md` から参照を外す。
   - ブログ本文と実装が別物: 「本文の解法」と「読めた実装」を分離して書く。混ぜない。

## 更新ルール

- 冒頭メタ情報を必ず更新する。
  - コード有無
  - コードを読めたか
  - 読めなかったもの
  - 読んだ提出ID、GitHub、gist URL
- `最終コードの構造` は推測ではなく実装ベースで書く。
- ブログ本文と実装が違う場合は、違いを明記する。
- 長いコード全文は貼らない。重要な構造や短い断片だけを要約する。
- 日本語、常体で書く。

削除する場合:

- 対象 `source_solutions/*.md` を削除する。
- `solution_urls.md` から該当URL行を削除する。
- `solution.md` の個別メモ一覧、比較表、分類、参照元、未解決事項から該当参照を削除する。
- 必要なら番号やリンクを整える。

## 品質基準

残すべき記事:

- 解法の中心アイデアが本文から分かる。
- 状態表現、評価関数、探索、出力のいずれかが具体的である。
- 実装または提出に辿れる。
- 実装に辿れなくても、人間が理解する価値がある。

削除すべき記事:

- 感想中心で解法の説明が薄い。
- 他人のコードを見たという話だけで、本質が書かれていない。
- 生成AIに投げた、写経した、スコアが出た、だけで構造が分からない。
- 実装も本文も再現に役立たない。
