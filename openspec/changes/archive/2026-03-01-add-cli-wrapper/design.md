## Context

現状の crate は `src/lib.rs` から公開される Gemfile.lock パーサーのみを提供しており、外部ツールが直接呼び出せる CLI エントリポイントはありません。今回の MVP では `library-first` を維持し、既存の `parse` と公開データモデルを再利用しながら、CLI 側で引数解釈、DTO 変換、JSON 出力を担う薄いラッパーを追加します。

制約は以下です。

- 入力は `lockfile_parser <SOURCE>` の単一パスのみを受け付ける
- `stdin`、`--format text`、複数入力は MVP の対象外とする
- `--help` と `--version` は人間向け text 出力を許可する
- parse-result JSON は `ok` と `parse_error` のみを対象とし、それ以外の CLI エラーは text 出力で扱う
- text 出力は非安定契約とし、JSON Schema は parse-result JSON のみに適用する

## Goals / Non-Goals

**Goals:**

- 既存パーサーをファイルパス入力で呼び出せる CLI を追加する
- パース成功時と `parse_error` 時に、固定 envelope の単一行 JSON を標準出力へ返す
- CLI 向け DTO と `schema/` 配下の JSON Schema を定義し、出力を検証可能にする
- `src/main.rs` を薄いエントリポイントに保ち、CLI 固有の処理を `src/cli.rs` に閉じ込める

**Non-Goals:**

- `stdin` 入力や `--format text` による人間向け本文表示の実装
- 既存ライブラリ API や `gemfile-lock-parsing` capability の要件変更
- text 出力の安定契約化
- `--help` / `--version` や `clap` 由来の usage エラー表示を細かく制御すること

## Decisions

### 1. CLI は `src/main.rs` と `src/cli.rs` に分割する

`src/main.rs` は `cli::run()` を呼んで終了コードを返すだけの極薄いエントリポイントにします。`src/cli.rs` に `clap` の Command builder、CLI 向け DTO、JSON 変換、出力処理を集約し、ライブラリ本体へ CLI 都合の型や依存を持ち込まない構成にします。

他案として `src/bin/lockfile_parser.rs` や `[[bin]]` による明示定義もありますが、MVP は単一バイナリで十分なため、最も単純な `src/main.rs` を採用します。

### 2. parse-result JSON は固定 envelope に限定する

JSON 出力は以下のトップレベル shape に固定します。

- `status`
- `data`
- `warnings`
- `error`

`status` は `ok` または `parse_error` のみです。`ok` では `data` が object、`warnings` は 0 件以上、`error` は `null` です。`parse_error` では `data` は `null`、`warnings` は空配列、`error` は object です。これにより、利用者はキー有無ではなく `status` だけで分岐できます。

CLI 全エラーを JSON に含める案もありましたが、`usage_error` や `io_error` を同一 envelope に入れると `null` 許可が増え、契約の見通しが悪くなるため採用しません。

### 3. parse-result JSON の DTO は CLI 専用型として定義する

ライブラリの公開型をそのまま serialize せず、CLI 側で以下へ写像します。

- `ParsedResult`
- `TopLevelDependencyView`
- `LockedSpec`
- `Warning`
- `ParseError`
- `SectionRef`

`ParseError` と `Warning` はどちらも `code`, `line`, `section`, `raw_line` を持つ対称な shape にし、差分は `Warning.raw_line` のみ `null` を許可します。`SectionRef` 自体の shape は `kind` と `name` で共有しますが、許可される `kind` は文脈で異なります。`Warning.section.kind` は `gem`, `gem_specs`, `dependencies`, `platforms`, `ruby_version`, `bundled_with`, `other` を使い、`eof` は使いません。`ParseError.section.kind` はこれらに加えて `eof` を使えますが、`eof` は既存パーサーが EOF 位置の fatal error を返し、かつ `raw_line` が空文字列のときにだけ使います。未知セクション名として実在する `EOF` は、warning / parse error のどちらでも `other` かつ `name = "EOF"` のまま保持します。

ライブラリ型へ `Serialize` を追加する案は、CLI の wire format をライブラリ API に漏らすため採用しません。

### 4. parse-result JSON は単一行のコンパクト JSON とする

標準出力には pretty print ではなく単一行のコンパクト JSON を出力します。AI エージェントや他ツールが直接読み取りやすく、必要であれば呼び出し側で整形できます。

pretty print を標準にする案は、人間には読みやすい一方で、MVP の主用途に対する利点が小さいため見送ります。

### 5. CLI エラーと補助コマンドは text 出力に寄せる

`--help` と `--version` は text 出力を許可し、`clap` の標準挙動に委譲します。`--version` は `lockfile_parser 0.1.0` の 1 行、`--help` は `clap` 標準文面を使います。`usage_error` は `clap` の通常エラー出力を流用し、複数行でも許容します。`io_error` は 1 行 text、`internal_error` は `internal_error` の最小表現に留めます。

この経路は JSON Schema の対象外であり、非安定契約として扱います。`usage_error` の終了コードを厳密に 1 へ固定する案もありましたが、MVP では `clap` への完全委譲を優先し、細かな制御は行いません。

### 6. JSON Schema は `schema/` 配下に置き、テストで検証する

parse-result JSON の契約は `schema/` 配下に JSON Schema Draft 2020-12 で保存し、生成された JSON がこのスキーマに適合することをテストで検証します。`jsonschema` crate は `dev-dependencies` に追加し、schema 自体のメタスキーマ検証と、サンプルまたは生成結果の検証に使います。Schema は shape だけでなく `eof` の意味づけも検証し、warning では `eof` を拒否し、`ParseError.section.kind = "eof"` のときは `raw_line = ""`、それ以外の parse error では `raw_line` を非空に制約します。

スキーマを文書専用にして検証しない案もありますが、AI 支援で実装する際に出力制約として機械的に確認できる方が有益なため採用しません。

## Risks / Trade-offs

- [Risk] `clap` へ usage エラーを委譲すると出力形式や終了コードが将来変わりうる → Mitigation: text 出力は非安定契約と明記し、MVP では parse-result JSON のみを安定契約として扱う
- [Risk] `top_level_dependencies` や `warnings` の順序に依存した利用やテストが書かれる → Mitigation: 順序非保証を文書化し、テストでは配列順に依存しない比較を行う
- [Risk] CLI 側 DTO がライブラリの内部表現から乖離する → Mitigation: DTO は写像専用に限定し、ライブラリの公開型は変更せず、変換ロジックを `src/cli.rs` に集約する
- [Risk] JSON Schema と実装がずれる → Mitigation: `jsonschema` による検証テストを追加し、出力契約を継続的に検証する

## Migration Plan

この変更は追加のみで、既存ライブラリ利用者への移行作業はありません。MVP では CLI バイナリ、JSON Schema、関連テストを追加し、必要な依存関係を `Cargo.toml` に追記します。問題があれば、CLI エントリポイントと追加依存を巻き戻すことで既存ライブラリのみの状態へ戻せます。

## Open Questions

- 将来追加する `--format text` の具体的な表示内容と、そのときの subcommand / option 設計
- parse-result JSON のサンプルをどこまで schema 検証テストへ含めるか
