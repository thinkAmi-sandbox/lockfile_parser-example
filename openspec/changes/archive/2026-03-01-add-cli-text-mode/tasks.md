## 1. CLI の出力モードを追加する

- [x] 1.1 `src/cli.rs` の引数定義に `--format` を追加し、既定値を `json`、許可値を `json` / `text` のみに制限する
- [x] 1.2 `src/cli.rs` の実行フローを整理し、ファイル読込後に 1 回だけ `parse` を呼んで、`json` モードと `text` モードの出力処理へ分岐させる

## 2. text モードの出力を実装する

- [x] 2.1 `top_level_dependency_views()` を利用して、トップレベル依存を gem 名の文字列昇順で `name [<version>]` / `name []` 形式に整形して `stdout` へ出力する
- [x] 2.2 warning と parse error を `stderr` に分離し、`warning` / `parse error` 接頭辞、診断コード、行番号、セクションを含む text 出力と、warning は終了コード `0`、parse error は終了コード `1` の挙動を実装する

## 3. CLI テストを更新する

- [x] 3.1 既存の `json` モードの挙動が維持されることを検証し、`--format json` 明示時と `--format` 省略時の両方を `tests/cli.rs` に追加する
- [x] 3.2 `tests/cli.rs` に `--format text` の正常系、未解決表示、空出力、warning の `stderr` 出力、parse error の `stderr` 出力と終了コード `1`、衝突回帰ケース、不正な `--format` 値を追加して検証する
