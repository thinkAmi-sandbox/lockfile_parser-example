## Why

このプロジェクトは現在ライブラリとしては利用できる一方で、AI エージェントや他ツールが Rust の組み込みなしに呼び出せるコマンドライン入口がありません。CLI の入出力契約が整理できたため、既存ライブラリを薄く包む MVP を追加し、パース結果を外部から安定して利用できるようにする必要があります。

## What Changes

- `lockfile_parser <SOURCE>` 形式で Gemfile.lock をファイルパスから受け取る CLI を追加する
- MVP では、パース成功時と `parse_error` 時に単一行の parse-result JSON を標準出力へ返す
- `--help` と `--version` は人間向けの text 出力を許可し、その他の CLI エラーも text 出力で扱う
- parse-result JSON の契約を `schema/` 配下の JSON Schema として定義し、生成結果を検証できるようにする

## Capabilities

### New Capabilities
- `gemfile-lock-cli`: Gemfile.lock をパス入力で受け取り、既存のパーサー結果を CLI 向けの parse-result JSON と最小限の text 出力として提供する

### Modified Capabilities
なし

## Impact

- 新しい CLI エントリポイントとして `src/main.rs` と `src/cli.rs` を追加する
- parse-result JSON の契約ファイルを `schema/` 配下に追加する
- CLI 向け DTO、JSON 出力、CLI テストを追加する
- `clap`、`serde`、`serde_json`、および schema 検証用の `jsonschema` を依存関係に追加する
- 既存の `gemfile-lock-parsing` capability はパース本体として再利用し、要件変更は行わない
