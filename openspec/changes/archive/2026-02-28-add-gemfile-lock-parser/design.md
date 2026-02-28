## Context

この変更では、Gemfile.lock を入力として受け取り、後続の脆弱性照会や最新バージョン確認の起点にできる構造化データを返す Rust パーサーを追加する。V1 では `GEM` セクションによる通常解決を主対象とし、トップレベル依存、解決済み spec、依存関係、platform、Ruby/Bundler バージョン、warning 診断を取得できることを目標にする。

Gemfile.lock は通常 `bundle install` で自動生成されるため、入力の揺れを広く許容するよりも、典型的な出力に対して厳密に解釈し、依存グラフを正しく構築できない場合は fatal とする方針を採る。一方で、`PLATFORMS`、`RUBY VERSION`、`BUNDLED WITH` は必須情報ではないため、セクション自体の欠落はそのまま許容し、見出しはあるが本文が不足または不正な場合のみ warning として扱う。

## Goals / Non-Goals

**Goals:**
- `parse(&str)` だけで利用できる、I/O を含まない Gemfile.lock パーサーを提供する。
- `top_level_dependencies` と `locked_specs` を gem 名キーで直接参照できる型付き結果を返す。
- `locked_specs` の依存を gem 名文字列で保持し、トップレベル依存から間接依存まで辿れるようにする。
- fatal なパース失敗と warning を分離し、AI エージェントや人間が追跡しやすい位置情報付き診断を返す。
- V1 の対象外である未対応トップレベルセクションを明示的に扱い、誤って通常解決対象として混ぜない。

**Non-Goals:**
- `GIT` / `PATH` / `PLUGIN` の spec を構造化して返すこと。
- lockfile 内の制約文字列を詳細に構文解析すること。
- ネットワークアクセスを伴う脆弱性照会や最新バージョン確認をパーサー自身が行うこと。
- ファイル読み込みやパス解決を含む convenience API を V1 で提供すること。
- spec や dependency の出現順を保持すること。

## Decisions

**1. 行ベースの状態機械で解析する**

Gemfile.lock は見出し、空行、インデントで意味が決まるため、AST を組み立てる前に行単位で分類して `Section` と組み合わせて解釈する。これにより、`InvalidEntry` を「現在の `Section` で受理できないローカル文法エラー」として一貫して扱える。

採用案:
- `LineKind` で `Blank`、`TopLevelHeader(String)`、`SpecsHeader`、`IndentedEntry { indent, text }` を分類する。
- `ParserState` は `current_section`、`current_spec_name`、`line`、`seen_sections`、`pending_optional`、`dependency_references` と構築中の結果を持つ。
- 既知セクションは出現順を固定せず、順不同でも受理する。ただし各行の意味づけは、その時点の `current_section` に従って行う。
- 既知のトップレベル見出しは完全一致で判定し、完全一致しない見出しは既知セクションとして扱わない。

代替案:
- 文字列を都度直接分岐して解釈する方式は、`Section` ごとの条件が分散しやすく、`InvalidEntry` と他の error code の境界が曖昧になるため採用しない。
- 完全な中間 AST を構築する方式は、V1 の要件に対して過剰であり、データモデルと重複するため採用しない。

**2. 公開 API は `parse(&str)` のみとし、結果は所有データで返す**

V1 の公開入口は `parse(&str) -> Result<ParsedGemfileLock, ParseError>` に限定する。入力は Gemfile.lock 全文の文字列であり、I/O は呼び出し側の責務とする。返り値は `String` を所有する構造体群で構成し、ライフタイムを公開 API に持ち込まない。

採用案:
- 文字列入力に対して純粋に解析だけを行う。
- `ParsedGemfileLock` に warning 診断を含め、成功時に追加情報も返す。

代替案:
- `parse_file` や `parse_reader` のような補助入口は、用途が明確になってから追加できるため V1 では採用しない。
- 借用ベースの返り値はライフタイム制約が外部に漏れ、V1 の利用性を下げるため採用しない。

**3. 結果モデルは gem 名キーの `HashMap` を中心に構成する**

呼び出し側の主な用途は「名前で直接引くこと」であるため、順序保持よりも直接参照を優先する。`top_level_dependencies` と `locked_specs` はいずれも gem 名をキーにした `HashMap` とし、キーと値に同じ名前を重複保持しない。

採用案:
- `top_level_dependencies: HashMap<String, TopLevelDependency>`
- `locked_specs: HashMap<String, LockedSpec>`
- `TopLevelDependency` は `raw_requirement: Option<String>` のみ保持する。
- `LockedSpec` は `version: String` と `dependencies: Vec<String>` を保持する。

代替案:
- 順序保持用の `Vec` は、V1 では利用側の要求がなく、設計を複雑にするため採用しない。
- 間接依存の `raw_requirement` 保持は、ユースケースが未確定で型が重くなるため採用しない。
- `BTreeMap` はデバッグ時の安定出力には有利だが、V1 では重視しないため採用しない。

**4. V1 の対応範囲は `GEM` セクション（通常解決）に限定する**

`GIT` / `PATH` / `PLUGIN` を含む未対応のトップレベルセクションは V1 の解析対象外とし、未対応セクションとして `IgnoredSection` warning を返す。`DEPENDENCIES` のエントリ末尾に `!` がある場合は、その依存が通常解決対象ではない明示的なシグナルとして `UnsupportedResolvedSource` の fatal とする。

採用案:
- `remote` は保持しない。
- `Other(String)` セクション内の行はヘッダ以外を基本的に無視する。
- `UnsupportedResolvedSource` は `DEPENDENCIES` の `!` でのみ確定的に発火させる。

代替案:
- `ignored_resolved_names` と追加状態を使って未対応 source 由来 spec を厳密追跡する設計は、V1 では内部状態が増えすぎるため採用しない。
- `!` を剥がして通常 gem として扱う方式は、対象外依存を通常解決に混ぜてしまうため採用しない。

**5. fatal と warning を分離し、fatal は依存グラフを構築できない場合に限定する**

V1 では、依存グラフを構築できない状況を fatal とし、メタ情報の欠損や未対応セクションは warning に留める。これにより、呼び出し側は成功結果を信頼して後続処理に使える。

採用案:
- `ParseErrorCode`: `MissingGemSection`, `MissingSpecsSubsection`, `MissingDependenciesSection`, `InvalidEntry`, `UnresolvedDependency`, `UnsupportedResolvedSource`, `DuplicateEntry`, `InternalStateViolation`
- `WarningDiagnosticCode`: `IgnoredSection`, `IncompleteOptionalSection`, `DuplicateOptionalSection`
- `DuplicateEntry` は `top_level_dependencies` または `locked_specs` のキー重複で使用する。
- `DEPENDENCIES` のエントリは、対応する `locked_specs` が存在しない場合でも保持する。
- `UnresolvedDependency` は `GEM` セクション `specs:` 配下の依存解決に限定し、`bundler` は解決済み spec がなくても許容する。
- optional セクション (`PLATFORMS`、`RUBY VERSION`、`BUNDLED WITH`) は、見出しが重複した場合に `DuplicateOptionalSection` warning を返し、最初に得た値を採用する。
- optional セクションの見出しは存在するが本文が不足または不正な場合は `IncompleteOptionalSection` warning を返し、fatal にはしない。`PLATFORMS` は 2 スペース、`RUBY VERSION` と `BUNDLED WITH` は 3 スペースの本文インデントのみを正しい書式として扱う。

代替案:
- `InvalidEntry` に重複を含めると、ローカル文法エラーとモデル衝突の意味が混ざるため採用しない。
- `kind + code` の二段構成は現時点でユースケースが固まっておらず、冗長になるため採用しない。

**6. 位置情報は AI デバッグ重視で保持する**

fatal と warning の両方に 1-based の行番号と `Section` を持たせる。fatal は `raw_line` を必須、warning は `raw_line` を任意とする。EOF で確定する fatal は `line = 総行数 + 1`、`raw_line = ""`、`section = Other("EOF")` とする。

採用案:
- `ParseError { code, line, section, raw_line }`
- `WarningDiagnostic { code, line, section, raw_line: Option<String> }`
- 行は正規化し、タブ文字が含まれる場合は `InvalidEntry` にする。

代替案:
- `line` や `section` を省略すると、AI と人間の双方で原因追跡が遅くなるため採用しない。
- EOF 系 error に特別な専用 enum を追加するより、`Other("EOF")` で統一する方が V1 では簡潔なため採用する。

## Risks / Trade-offs

- [未対応 source 判定が限定的] → `UnsupportedResolvedSource` は `DEPENDENCIES` の `!` に限定し、それ以外の未対応トップレベルセクションは `IgnoredSection` として扱う制限を仕様に明記する。
- [Gem 名キーの衝突] → 同名 spec や同名トップレベル依存は `DuplicateEntry` で即時停止し、曖昧な上書きを避ける。
- [厳密なインデント判定による失敗] → 自動生成物を前提とする V1 では厳密さを優先し、許容範囲拡大は将来の要件として切り出す。
- [optional メタ情報の解釈が最小限] → `platforms`、`ruby_version`、`bundler_version` は生テキスト寄りに保持し、用途が明確になってから詳細解釈を追加する。
- [公開 API が最小限] → `parse(&str)` のみに絞ることで利用シーンによっては前処理が必要になるが、I/O と構文解析の責務分離を優先する。
