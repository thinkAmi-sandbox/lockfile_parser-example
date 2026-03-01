## Why

現行の CLI は parse-result JSON のみを返すため、ターミナル上でトップレベル依存の解決結果を人間が素早く確認したい用途では扱いづらい。既存の JSON 契約を維持したまま、人間向けの一覧表示を追加して利用場面を広げる必要がある。

## What Changes

- `--format` オプションを追加し、既定値を `json`、入力可能値を `json` と `text` に限定する。
- `json` モードは現行挙動を完全に維持し、既存の parse-result JSON 出力契約を変更しない。
- `text` モードでは、トップレベル依存のみを gem 名の文字列昇順で `stdout` に 1 行ずつ出力し、解決済みは `name [<version>]`、未解決は `name []` で表示する。
- `text` モードでは、warning と parse error を `stderr` の text 出力に分離し、warning は終了コード `0`、parse error は終了コード `1` とする。
- `stdout` と `stderr` の責務分離、空出力、末尾改行、warning / parse error の最低限の表示情報を CLI 仕様として明確化する。

## Capabilities

### New Capabilities
なし

### Modified Capabilities
- `gemfile-lock-cli`: `--format text` の追加、text モードの一覧出力、`stdout` / `stderr` の責務分離、終了コードの要件を追加する。

## Impact

- CLI の引数解釈と出力経路を担う `src/cli.rs` が主な変更対象となる。
- CLI の期待動作を検証する `tests/cli.rs` のテスト観点が増える。
- 既存 capability の要件変更として `openspec/specs/gemfile-lock-cli/spec.md` に差分 spec が必要になる。
