# AHC074保存先の運用

- このディレクトリはkyoupuro内の保存用資料である。Git操作はkyoupuroの既存ブランチとremoteに従う。AHC074の独立リポジトリへpushしない。
- 原稿の名称と実験番号を保つ。解法を修正する場合は保存原稿との違いを明示する。
- 日本語の常体で記述する。記号はnotes/notations.mdに従う。
- notes/notations.md、notes/important_properties.md、v0000_template.cppの内容変更にはユーザーの明示的な許可を得る。
- solverは単一ファイルのC++23、GCC 15、単一コアかつ単一スレッドを前提とする。
- スコアと時間の比較には端末設定を用いた標準LOCALを使う。TimeKeeperのsolver_msを時間の正とする。
- 通常の動作確認にはeval.pyの--dry-runを使い、保存済みの評価記録を変更しない。
- .gitignoreは保存ファイルの明示的な一覧である。資産を追加するときは必要性と容量を確認して一覧も更新する。大量の生成データを強制追加しない。
- 未保存の資料はMIGRATION.mdに記した元リポジトリと保存元を参照する。元ディレクトリはこの保存先の実行に必要な依存先にはしない。
