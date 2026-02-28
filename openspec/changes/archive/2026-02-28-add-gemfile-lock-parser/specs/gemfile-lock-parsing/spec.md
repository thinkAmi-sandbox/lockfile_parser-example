## ADDED Requirements

### Requirement: Gemfile.lock を構造化して返却できる
パーサーは Gemfile.lock 全文の文字列入力を受け取り、トップレベル依存、解決済み spec、メタ情報、warning 診断を含む構造化結果を返さなければならない (MUST)。

#### Scenario: 通常の Gemfile.lock を解析する
- **WHEN** `GEM`、`DEPENDENCIES`、および任意のメタ情報セクションを含む Gemfile.lock を解析する
- **THEN** パーサーは fatal error を返さず、構造化結果を返す

#### Scenario: 既知セクションが順不同で現れる
- **WHEN** 既知セクションが典型順と異なる順序で現れる Gemfile.lock を解析する
- **THEN** パーサーはセクション順では失敗せず、その時点のセクション文脈に従って各行を解釈する

### Requirement: 既知セクション見出しを完全一致で判定する
パーサーは `GEM`、`DEPENDENCIES`、`PLATFORMS`、`RUBY VERSION`、`BUNDLED WITH` などの既知見出しを完全一致でのみ既知セクションとして扱わなければならない (MUST)。

#### Scenario: 既知見出しを完全一致で認識する
- **WHEN** トップレベル行が既知見出しと完全一致する
- **THEN** パーサーは対応する既知セクションへ遷移する

#### Scenario: 完全一致しない見出しを既知セクションにしない
- **WHEN** トップレベル行が既知見出しと完全一致しない
- **THEN** パーサーはその行を既知セクションとして扱わず、未対応のトップレベル見出しとして扱う

### Requirement: トップレベル依存を gem 名キーで取得できる
パーサーは `DEPENDENCIES` セクションの各依存を gem 名キーで保持し、制約文字列が存在する場合は未解釈の生文字列として保持しなければならない (MUST)。トップレベル依存は、同名の解決済み spec が `locked_specs` に存在しない場合でも保持できなければならない。

#### Scenario: 制約付きトップレベル依存を保持する
- **WHEN** `DEPENDENCIES` に `rails (~> 6.1.4)` のような行が含まれる
- **THEN** 結果の `top_level_dependencies` は `rails` をキーとして保持し、その値に `~> 6.1.4` を生文字列で保持する

#### Scenario: 制約なしトップレベル依存を保持する
- **WHEN** `DEPENDENCIES` に `omniauth` のような行が含まれる
- **THEN** 結果の `top_level_dependencies` は `omniauth` をキーとして保持し、その値の制約は未設定とする

#### Scenario: 解決済み spec がないトップレベル依存を保持する
- **WHEN** `DEPENDENCIES` に `tzinfo-data` のような行が含まれ、対応する解決済み spec が `locked_specs` に存在しない
- **THEN** パーサーは fatal error を返さず、`top_level_dependencies` にその依存を保持する

### Requirement: 解決済み spec と依存関係を gem 名キーで取得できる
パーサーは `GEM` セクションの `specs:` から解決済み spec を gem 名キーで保持し、各 spec のバージョンと依存先 gem 名一覧を保持しなければならない (MUST)。

#### Scenario: 解決済み spec を名前で参照できる
- **WHEN** `GEM` セクションの `specs:` に `rails (6.1.4)` が含まれる
- **THEN** 結果の `locked_specs` は `rails` をキーとして保持し、その値に `6.1.4` を保持する

#### Scenario: 間接依存を名前で辿れる
- **WHEN** `rails (6.1.4)` の配下に `activerecord (= 6.1.4)` が含まれる
- **THEN** 結果の `locked_specs["rails"]` は依存先一覧に `activerecord` を含み、呼び出し側は `locked_specs["activerecord"]` を直接参照できる

### Requirement: optional メタ情報を生テキスト寄りに保持する
パーサーは `PLATFORMS`、`RUBY VERSION`、`BUNDLED WITH` を生テキスト寄りの値として保持しなければならない (MUST)。`PLATFORMS` の本文は 2 スペース、`RUBY VERSION` と `BUNDLED WITH` の本文は 3 スペースのインデントを正しい書式として扱う。これらのセクションが存在しない場合でも fatal error にしてはならない。

#### Scenario: platform を配列で保持する
- **WHEN** `PLATFORMS` セクションに `x86_64-darwin-19` が含まれる
- **THEN** 結果の `platforms` は `x86_64-darwin-19` を要素として保持する

#### Scenario: Ruby と Bundler のバージョンを保持する
- **WHEN** `RUBY VERSION` に `ruby 3.0.1p64`、`BUNDLED WITH` に `2.2.21` が含まれる
- **THEN** 結果は `ruby_version` に `ruby 3.0.1p64`、`bundler_version` に `2.2.21` を保持する

#### Scenario: optional セクションがない
- **WHEN** `PLATFORMS`、`RUBY VERSION`、`BUNDLED WITH` のいずれかが存在しない Gemfile.lock を解析する
- **THEN** パーサーは fatal error を返さず、対応する値を空または未設定のまま返す

#### Scenario: optional セクションが重複する
- **WHEN** `PLATFORMS`、`RUBY VERSION`、または `BUNDLED WITH` の見出しが複数回現れる
- **THEN** パーサーは `DuplicateOptionalSection` warning を返し、最初に取得した値を保持し続ける

#### Scenario: optional セクションが不完全である
- **WHEN** `PLATFORMS`、`RUBY VERSION`、または `BUNDLED WITH` の見出しはあるが本文が不足または不正である
- **THEN** パーサーは `IncompleteOptionalSection` warning を返し、fatal error にはしない

#### Scenario: optional セクション本文のインデントが不正である
- **WHEN** `PLATFORMS` の本文が 2 スペース以外、または `RUBY VERSION` / `BUNDLED WITH` の本文が 3 スペース以外で記述されている
- **THEN** パーサーは `IncompleteOptionalSection` warning を返し、対応する値を設定しない

### Requirement: V1 の対象外セクションを明示的に扱う
パーサーは `GIT`、`PATH`、`PLUGIN` を含む未対応のトップレベルセクションを通常解決対象に含めてはならず、依存グラフに影響しない場合は warning として扱わなければならない (MUST)。`DEPENDENCIES` の依存末尾が `!` の場合は V1 対象外の解決元として fatal error にしなければならない。

#### Scenario: 未対応セクションを warning として無視する
- **WHEN** `GIT` や未知のトップレベル見出しなど、未対応セクションが含まれるが、トップレベル依存と解決済み spec の構築には不要である
- **THEN** パーサーは fatal error を返さず、`IgnoredSection` warning を返す

#### Scenario: DEPENDENCIES の対象外依存を拒否する
- **WHEN** `DEPENDENCIES` に `my_private_gem!` が含まれる
- **THEN** パーサーは `UnsupportedResolvedSource` の fatal error を返す

### Requirement: fatal error と warning に位置情報を含める
パーサーは fatal error と warning の両方に、1-based の行番号と現在のセクションを含めなければならない (MUST)。fatal error は AI デバッグに利用できる行テキストを含めなければならず、warning の行テキストは対応する行が存在する場合に含め、対応する行が存在しない場合は省略できる。EOF で確定する fatal error は EOF を示す位置情報を返さなければならない。

#### Scenario: ローカル文法エラーに位置情報を付与する
- **WHEN** 現在のセクションで受理できないインデントまたは書式の行を検出する
- **THEN** パーサーは `InvalidEntry` の fatal error を返し、行番号、該当セクション、失敗した生行を含める

#### Scenario: EOF で確定する fatal error に EOF 位置を付与する
- **WHEN** 必須セクション欠落など、ファイル末尾まで読んでから fatal error が確定する
- **THEN** パーサーは `line = 総行数 + 1`、`section = Other("EOF")`、空文字列の `raw_line` を返す

#### Scenario: 対応する行がない warning では行テキストを省略できる
- **WHEN** optional セクションの見出しはあるが本文がなく、warning が見出し行ベースで確定する
- **THEN** パーサーは warning に行番号とセクションを含め、`raw_line` は未設定にできる

### Requirement: 依存グラフを構築できない場合は fatal にする
パーサーは依存グラフの構築に必要な情報が欠けた場合、warning で継続してはならず、対応する fatal error を返さなければならない (MUST)。

#### Scenario: 必須セクションが欠けている
- **WHEN** `GEM`、`GEM` 配下の `specs:`、または `DEPENDENCIES` のいずれかが存在しない
- **THEN** パーサーはそれぞれ対応する欠落系の fatal error を返す

#### Scenario: 参照先の spec が存在しない
- **WHEN** `bundler` を除く spec 配下の依存が、`locked_specs` のキーとして解決できない
- **THEN** パーサーは `UnresolvedDependency` の fatal error を返す

#### Scenario: 名前キーが重複する
- **WHEN** `top_level_dependencies` または `locked_specs` に同じ gem 名が複数回現れる
- **THEN** パーサーは `DuplicateEntry` の fatal error を返し、曖昧な上書きを行わない
