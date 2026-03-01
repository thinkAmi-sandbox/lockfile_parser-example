# gemfile-lock-cli Specification

## Purpose
TBD - created by archiving change add-cli-wrapper. Update Purpose after archive.
## Requirements
### Requirement: ファイルパス入力で Gemfile.lock を解析できる
CLI は `lockfile_parser <SOURCE>` 形式で、単一のファイルパス入力から Gemfile.lock を解析できなければならない (MUST)。`SOURCE` はファイルパスとして扱い、既存のライブラリパーサーを利用して結果を生成しなければならない (MUST)。

#### Scenario: 単一のファイルパス入力を解析する
- **WHEN** 利用者が `lockfile_parser path/to/Gemfile.lock` を実行し、`SOURCE` が UTF-8 で読める Gemfile.lock ファイルである
- **THEN** CLI はそのファイル内容を既存のパーサーで解析し、後続の成功または `parse_error` の出力契約に従って結果を返す

### Requirement: パース成功時は parse-result JSON を返す
CLI は Gemfile.lock のパースに成功した場合、標準出力へ単一行のコンパクトな parse-result JSON を 1 個返さなければならない (MUST)。この JSON のトップレベルは `status`、`data`、`warnings`、`error` の固定 shape を持ち、`status` は `ok`、`data` は object、`warnings` は 0 件以上の配列、`error` は `null` でなければならない (MUST)。`data` は `top_level_dependencies`、`locked_specs`、`platforms`、`ruby_version`、`bundler_version` を含まなければならない (MUST)。`top_level_dependencies` の配列順、`warnings` の配列順、`locked_specs` の key 順は保証しなくてよい。

#### Scenario: 成功結果を JSON で返す
- **WHEN** 入力された Gemfile.lock のパースが成功する
- **THEN** CLI は `status = "ok"` の parse-result JSON を標準出力へ 1 行で返し、`data` に CLI 向け DTO へ写像した解析結果を含め、`error` は `null` とする

#### Scenario: 成功結果に warning を含める
- **WHEN** パースは成功するが、未対応セクションや不完全な optional セクションにより warning が発生する
- **THEN** CLI は `warnings` に 1 件以上の warning 診断を含めて返し、各 warning は `code`、`line`、`section`、`raw_line` を持ち、`warning.section.kind` は `eof` であってはならない

#### Scenario: 未知セクション名 EOF は warning でも EOF として扱わない
- **WHEN** パースは成功するが、未知のトップレベルセクション名として実在する `EOF` により warning が発生する
- **THEN** CLI はその warning の `section.kind` を `other` とし、`section.name` に `"EOF"` を保持する

### Requirement: パース失敗時は `parse_error` の parse-result JSON を返す
CLI は Gemfile.lock の読込後に既存パーサーが fatal error を返した場合、標準出力へ単一行のコンパクトな parse-result JSON を 1 個返さなければならない (MUST)。この JSON の `status` は `parse_error`、`data` は `null`、`warnings` は空配列、`error` は object でなければならない (MUST)。`error` は `code`、`line`、`section`、`raw_line` を持ち、`code` はライブラリの parse error code を `snake_case` に変換した値でなければならない (MUST)。`section` は `kind` と `name` を持ち、`kind` は `gem`、`gem_specs`、`dependencies`、`platforms`、`ruby_version`、`bundled_with`、`other`、`eof` のいずれかでなければならない (MUST)。`error.section.kind = "eof"` はファイル末尾で確定した parse error に限定され、`error.raw_line` は空文字列でなければならない (MUST)。`error.section.kind != "eof"` の場合、`error.raw_line` は非空文字列でなければならない (MUST)。

#### Scenario: 文法エラーを `parse_error` JSON で返す
- **WHEN** 入力ファイルの内容が Gemfile.lock として不正で、既存パーサーが `InvalidEntry` などの fatal error を返す
- **THEN** CLI は `status = "parse_error"` の parse-result JSON を標準出力へ 1 行で返し、`error.code` に対応する `snake_case` 値、`line`、`section`、`raw_line` を含める

#### Scenario: EOF で確定するエラーを返す
- **WHEN** 必須セクション不足などにより、既存パーサーが EOF 位置の fatal error を返す
- **THEN** CLI の `error.section.kind` は `eof` となり、`error.raw_line` は空文字列のまま返る

#### Scenario: 未知セクション名 EOF 内の fatal error は EOF として扱わない
- **WHEN** 現在のセクション名が文字列として実在する `EOF` であり、既存パーサーがその文脈で fatal error を返す
- **THEN** CLI の `error.section.kind` は `other` とし、`error.section.name` に `"EOF"` を保持し、`error.raw_line` は非空文字列のまま返る

### Requirement: パース以外の出力経路は text とする
CLI は parse-result JSON の対象外である経路について、text 出力を返さなければならない (MUST)。`--help` と `--version` は人間向けの text 出力とし、`--version` は `lockfile_parser <version>` の 1 行を返さなければならない (MUST)。ファイルが読めない場合や、パース実行前の CLI エラーも text 出力で扱わなければならない (MUST)。これらの text 出力は安定契約として扱わなくてよい。

#### Scenario: バージョンを text で返す
- **WHEN** 利用者が `lockfile_parser --version` を実行する
- **THEN** CLI は `lockfile_parser <version>` 形式の 1 行 text を返す

#### Scenario: パース前の失敗を text で返す
- **WHEN** 入力ファイルが存在しない、または CLI の引数解釈がパース実行前に失敗する
- **THEN** CLI は parse-result JSON ではなく text を返す

### Requirement: parse-result JSON の契約を JSON Schema として提供する
リポジトリは parse-result JSON の契約を検証できる JSON Schema を `schema/` 配下に提供しなければならない (MUST)。この Schema は `ok` と `parse_error` の parse-result JSON のみを対象とし、text 出力経路を対象に含めてはならない (MUST)。この Schema は `eof` の意味づけも検証し、warning で `section.kind = "eof"` を受理してはならず (MUST NOT)、`parse_error` では `error.section.kind = "eof"` かつ `error.raw_line` が非空の payload を受理してはならない (MUST NOT)。

#### Scenario: JSON 出力を schema で検証できる
- **WHEN** 利用者またはテストが `schema/` 配下の JSON Schema で CLI の JSON 出力を検証する
- **THEN** `ok` と `parse_error` の parse-result JSON は検証対象となり、`--help`、`--version`、およびその他の text 出力は検証対象外となる

#### Scenario: EOF の意味づけに反する payload を schema が拒否する
- **WHEN** 利用者またはテストが、warning で `section.kind = "eof"` を持つ payload や、`parse_error` で `error.section.kind = "eof"` かつ `error.raw_line` が非空の payload を schema で検証する
- **THEN** Schema 検証は失敗しなければならない

