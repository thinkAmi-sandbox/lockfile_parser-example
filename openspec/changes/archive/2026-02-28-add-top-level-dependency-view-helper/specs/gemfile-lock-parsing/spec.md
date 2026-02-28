## ADDED Requirements

### Requirement: トップレベル依存を解決済みバージョン付きで列挙できる
`ParsedGemfileLock` は、トップレベル依存を名前、制約文字列、および対応する解決済みバージョンをまとめて参照できる helper を提供しなければならない (MUST)。解決済みバージョンは `locked_specs` の同名 key から導出し、対応する解決済み spec が存在しないトップレベル依存は未設定の解決済みバージョンとして扱わなければならない (MUST)。この helper は保存済みデータを書き換えてはならず、列挙順を保証しなくてよい。

#### Scenario: 対応する解決済み spec があるトップレベル依存を列挙する
- **WHEN** `top_level_dependencies` に `rails` が存在し、`locked_specs` に同名の `rails` spec が存在する
- **THEN** helper が返すトップレベル依存ビューは `rails` の名前、元の制約文字列、および `locked_specs["rails"]` から導出した解決済みバージョンを含む

#### Scenario: 対応する解決済み spec がないトップレベル依存を列挙する
- **WHEN** `top_level_dependencies` に `tzinfo-data` が存在し、`locked_specs` に同名の spec が存在しない
- **THEN** helper が返すトップレベル依存ビューは `tzinfo-data` を含み、解決済みバージョンを未設定として返す
