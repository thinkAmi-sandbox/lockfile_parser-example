## 1. 依存関係と土台の追加

- [ ] 1.1 `Cargo.toml` に CLI 実装用の `clap`、`serde`、`serde_json` と、schema 検証用の `jsonschema` を追加する
- [ ] 1.2 `src/main.rs` を追加し、`cli::run()` を呼ぶだけのエントリポイントを作成する
- [ ] 1.3 `src/cli.rs` を追加し、`clap` の Command builder と実行の骨組みを配置する

## 2. parse-result JSON の実装

- [ ] 2.1 `src/cli.rs` に CLI 専用 DTO (`ParsedResult`、`ParseError`、`Warning`、`SectionRef` など) を定義する
- [ ] 2.2 既存の `parse` 成功結果を `ok` の parse-result JSON DTO へ写像する処理を実装する
- [ ] 2.3 既存の `ParseError` を `parse_error` の parse-result JSON DTO へ写像する処理を実装する
- [ ] 2.4 parse-result JSON を単一行のコンパクト JSON として標準出力へ出す処理を実装する

## 3. CLI 入出力経路の実装

- [ ] 3.1 `lockfile_parser <SOURCE>` の単一パス入力のみを受け付ける CLI 実行フローを実装する
- [ ] 3.2 入力ファイルを UTF-8 として読み込み、読めない場合は 1 行の text エラーを返す処理を実装する
- [ ] 3.3 `--help` / `--version` と usage エラーを `clap` の text 出力へ委譲する構成にする

## 4. Schema と検証の追加

- [ ] 4.1 `schema/parse-result.schema.json` を追加し、`ok | parse_error` の parse-result JSON 契約を記述する
- [ ] 4.2 JSON Schema 自体を検証するテストを追加する
- [ ] 4.3 CLI が生成する `ok` と `parse_error` の JSON が schema に適合することを検証するテストを追加する

## 5. テストと仕上げ

- [ ] 5.1 `ok` 応答を確認する CLI テストを追加し、配列順に依存しない比較にする
- [ ] 5.2 `parse_error` 応答を確認する CLI テストを追加し、error DTO の shape を検証する
- [ ] 5.3 `schema` 対象外の text 出力方針と順序非保証を必要なドキュメントへ反映する
