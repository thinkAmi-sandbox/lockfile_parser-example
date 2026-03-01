## 1. 公開 API と結果モデル

- [x] 1.1 `parse(&str)` を公開入口とするパーサーモジュールを追加する
- [x] 1.2 `ParsedGemfileLock`、`TopLevelDependency`、`LockedSpec`、`Section`、`ParseError`、`WarningDiagnostic` と各 code enum を定義する
- [x] 1.3 行正規化と `LineKind` 分類を担う補助処理を追加する

## 2. コア解析フロー

- [x] 2.1 `ParserState` とトップレベル見出し遷移を実装し、`seen_sections` と `current_section` を更新できるようにする
- [x] 2.2 `Gem` / `GemSpecs` の解析を実装し、`locked_specs`、`current_spec_name`、依存先一覧を構築できるようにする
- [x] 2.3 `Dependencies`、`Platforms`、`RubyVersion`、`BundledWith` の解析を実装し、トップレベル依存と optional メタ情報を構築できるようにする

## 3. 診断と整合性検証

- [x] 3.1 タブ文字、インデント不一致、想定外書式に対する `InvalidEntry` の生成を実装する
- [x] 3.2 `GIT` / `PATH` / `PLUGIN` の `IgnoredSection` warning と、`DEPENDENCIES` 末尾 `!` の `UnsupportedResolvedSource` を実装する
- [x] 3.3 EOF 時の必須セクション検証、未解決依存検証、重複キー検証を実装し、対応する fatal error を返すようにする

## 4. テストと検証

- [x] 4.1 サンプルの `Gemfile.lock` を使って正常系の構造化結果を検証するテストを追加する
- [x] 4.2 optional メタ情報と warning（未対応セクション、不完全セクション、重複 optional セクション）を検証するテストを追加する
- [x] 4.3 fatal error（`InvalidEntry`、`UnresolvedDependency`、`UnsupportedResolvedSource`、`DuplicateEntry`、EOF 系）の位置情報を検証するテストを追加する
