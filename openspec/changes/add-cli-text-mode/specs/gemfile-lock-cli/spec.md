## MODIFIED Requirements

### Requirement: ファイルパス入力で Gemfile.lock を解析できる
CLI は `lockfile_parser [--format <FORMAT>] <SOURCE>` 形式で、単一のファイルパス入力から Gemfile.lock を解析できなければならない (MUST)。`FORMAT` は省略時に `json` を選択し、指定時は `json` または `text` のみを受け付けなければならない (MUST)。`SOURCE` はファイルパスとして扱い、既存のライブラリパーサーを利用して結果を生成しなければならない (MUST)。

#### Scenario: `--format` を省略して既定の json モードで解析する
- **WHEN** 利用者が `lockfile_parser path/to/Gemfile.lock` を実行し、`SOURCE` が UTF-8 で読める Gemfile.lock ファイルである
- **THEN** CLI はそのファイル内容を既存のパーサーで解析し、json モードの成功または `parse_error` の出力契約に従って結果を返す

#### Scenario: `--format text` を指定して解析する
- **WHEN** 利用者が `lockfile_parser --format text path/to/Gemfile.lock` を実行し、`SOURCE` が UTF-8 で読める Gemfile.lock ファイルである
- **THEN** CLI はそのファイル内容を既存のパーサーで解析し、text モードの成功または parse error の出力契約に従って結果を返す

#### Scenario: 許可されない `--format` 値を拒否する
- **WHEN** 利用者が `lockfile_parser --format yaml path/to/Gemfile.lock` のように `json` と `text` 以外の `FORMAT` を指定する
- **THEN** CLI はパース実行前に引数解釈を失敗させ、parse-result JSON ではない text 出力で扱う

### Requirement: パース成功時は parse-result JSON を返す
CLI は `--format json` を明示した場合、または `--format` を省略して既定の `json` モードで実行した場合、Gemfile.lock のパースに成功したときに標準出力へ単一行のコンパクトな parse-result JSON を 1 個返さなければならない (MUST)。この JSON のトップレベルは `status`、`data`、`warnings`、`error` の固定 shape を持ち、`status` は `ok`、`data` は object、`warnings` は 0 件以上の配列、`error` は `null` でなければならない (MUST)。`data` は `top_level_dependencies`、`locked_specs`、`platforms`、`ruby_version`、`bundler_version` を含まなければならない (MUST)。`top_level_dependencies` の配列順、`warnings` の配列順、`locked_specs` の key 順は保証しなくてよい。

#### Scenario: 既定の json モードで成功結果を JSON で返す
- **WHEN** `--format` を省略した実行で、入力された Gemfile.lock のパースが成功する
- **THEN** CLI は `status = "ok"` の parse-result JSON を標準出力へ 1 行で返し、`data` に CLI 向け DTO へ写像した解析結果を含め、`error` は `null` とする

#### Scenario: 明示的な json モードで成功結果に warning を含める
- **WHEN** 利用者が `--format json` を指定し、パースは成功するが、未対応セクションや不完全な optional セクションにより warning が発生する
- **THEN** CLI は `warnings` に 1 件以上の warning 診断を含めて返し、各 warning は `code`、`line`、`section`、`raw_line` を持ち、`warning.section.kind` は `eof` であってはならない

#### Scenario: 未知セクション名 EOF は json モードの warning でも EOF として扱わない
- **WHEN** json モードのパースは成功するが、未知のトップレベルセクション名として実在する `EOF` により warning が発生する
- **THEN** CLI はその warning の `section.kind` を `other` とし、`section.name` に `"EOF"` を保持する

### Requirement: パース失敗時は `parse_error` の parse-result JSON を返す
CLI は `--format json` を明示した場合、または `--format` を省略して既定の `json` モードで実行した場合、Gemfile.lock の読込後に既存パーサーが fatal error を返したとき、標準出力へ単一行のコンパクトな parse-result JSON を 1 個返さなければならない (MUST)。この JSON の `status` は `parse_error`、`data` は `null`、`warnings` は空配列、`error` は object でなければならない (MUST)。`error` は `code`、`line`、`section`、`raw_line` を持ち、`code` はライブラリの parse error code を `snake_case` に変換した値でなければならない (MUST)。`section` は `kind` と `name` を持ち、`kind` は `gem`、`gem_specs`、`dependencies`、`platforms`、`ruby_version`、`bundled_with`、`other`、`eof` のいずれかでなければならない (MUST)。`error.section.kind = "eof"` はファイル末尾で確定した parse error に限定され、`error.raw_line` は空文字列でなければならない (MUST)。`error.section.kind != "eof"` の場合、`error.raw_line` は非空文字列でなければならない (MUST)。

#### Scenario: json モードで文法エラーを `parse_error` JSON で返す
- **WHEN** json モードで入力ファイルの内容が Gemfile.lock として不正で、既存パーサーが `InvalidEntry` などの fatal error を返す
- **THEN** CLI は `status = "parse_error"` の parse-result JSON を標準出力へ 1 行で返し、`error.code` に対応する `snake_case` 値、`line`、`section`、`raw_line` を含める

#### Scenario: json モードで EOF で確定するエラーを返す
- **WHEN** json モードで必須セクション不足などにより、既存パーサーが EOF 位置の fatal error を返す
- **THEN** CLI の `error.section.kind` は `eof` となり、`error.raw_line` は空文字列のまま返る

#### Scenario: json モードで未知セクション名 EOF 内の fatal error は EOF として扱わない
- **WHEN** json モードで現在のセクション名が文字列として実在する `EOF` であり、既存パーサーがその文脈で fatal error を返す
- **THEN** CLI の `error.section.kind` は `other` とし、`error.section.name` に `"EOF"` を保持し、`error.raw_line` は非空文字列のまま返る

## ADDED Requirements

### Requirement: text モードではトップレベル依存一覧を返す
CLI は `--format text` を指定して Gemfile.lock のパースに成功した場合、トップレベル依存のみを標準出力へ改行区切りの text として返さなければならない (MUST)。一覧は gem 名の文字列昇順で並べなければならず (MUST)、各行は `name [<resolved-version>]` または `name []` の形式でなければならない (MUST)。対応する解決済み spec が存在しないトップレベル依存は、一覧から除外せず `[]` の空括弧で表示しなければならない (MUST)。対応する解決済み spec が存在する場合は、角括弧の中身にその解決済みバージョン文字列をそのまま表示しなければならない (MUST)。標準出力には一覧以外の text を混在させてはならない (MUST NOT)。表示対象が 0 件の場合、標準出力は空でなければならない (MUST)。

#### Scenario: 解決済みと未解決のトップレベル依存を昇順で返す
- **WHEN** 利用者が `--format text` を指定し、トップレベル依存として `rails` と `tzinfo-data` を含む Gemfile.lock のパースが成功し、`rails` だけが解決済み spec を持つ
- **THEN** CLI は標準出力に `rails [<resolved-version>]` と `tzinfo-data []` を gem 名の文字列昇順で 1 行ずつ返す

#### Scenario: 解決済みバージョンが `unresolved` でも未解決表示と衝突しない
- **WHEN** 利用者が `--format text` を指定し、`alpha` の解決済み spec のバージョン文字列が `unresolved` で、`beta` は対応する解決済み spec を持たない
- **THEN** CLI は標準出力に `alpha [unresolved]` と `beta []` を別の行として返し、両者を同じ表現にしてはならない

#### Scenario: 表示対象がない場合は空出力で返す
- **WHEN** 利用者が `--format text` を指定し、パースは成功するが表示対象となるトップレベル依存が 0 件である
- **THEN** CLI の標準出力は空でなければならない

### Requirement: text モードでは診断を stderr に分離する
CLI は `--format text` を指定した場合、warning と parse error を標準出力ではなく標準エラー出力へ text で返さなければならない (MUST)。warning の接頭辞は `warning`、parse error の接頭辞は `parse error` を含まなければならず (MUST)、どちらも少なくとも診断コード、1-based 行番号、および現在セクションを `kind` 相当の英字で含まなければならない (MUST)。入力由来の `section.name` は text 診断に含めてはならない (MUST NOT)。warning の細かな wording は安定契約として扱わなくてよい。warning が発生してもパース成功である限り終了コードは `0` のままでなければならず (MUST)、parse error が発生した場合は終了コード `1` を返さなければならない (MUST)。

#### Scenario: text モードで warning は stderr に出しつつ成功終了する
- **WHEN** 利用者が `--format text` を指定し、パースは成功するが warning が 1 件以上発生する
- **THEN** CLI は一覧のみを標準出力へ返し、warning を `warning` 接頭辞付きの text で標準エラー出力へ返し、終了コード `0` を返す

#### Scenario: text モードで parse error は stderr に出して失敗終了する
- **WHEN** 利用者が `--format text` を指定し、既存パーサーが fatal error を返す
- **THEN** CLI は parse error を `parse error` 接頭辞付きの text で標準エラー出力へ返し、終了コード `1` を返す

#### Scenario: text モードで未知セクション名を診断へ埋め込まない
- **WHEN** 利用者が `--format text` を指定し、入力由来の未知セクション名により `section.kind = "other"` の warning または parse error が発生する
- **THEN** CLI は `section=other` を含めつつ、入力由来の `section.name` を text 診断へ含めてはならない
