## Why

`ParsedGemfileLock` からトップレベル依存の解決済みバージョンを参照するには、呼び出し側が毎回 `top_level_dependencies` と `locked_specs` を突き合わせる必要があります。ライブラリ利用者と今後追加する CLI の両方で同じ導出ロジックが必要になるため、利用側の重複実装が増える前にライブラリ側へ集約します。

## What Changes

- `ParsedGemfileLock` からトップレベル依存を解決済みバージョン付きで参照できる公開 helper を追加する。
- helper が返す読み取り専用の借用ビューを追加し、制約文字列と解決済みバージョンをまとめて扱えるようにする。
- トップレベル依存に対応する `locked_specs` が存在する場合と存在しない場合の両方を検証するテストを追加する。
- `TopLevelDependency` や `locked_specs` の既存データモデルは維持し、CLI や JSON 出力はこの change の対象外とする。

## Capabilities

### New Capabilities
- なし

### Modified Capabilities
- `gemfile-lock-parsing`: トップレベル依存を解決済みバージョン付きで参照できる要件を追加する。

## Impact

- 公開 API として `ParsedGemfileLock` に新しい参照 helper が追加される。
- パース結果の利用方法に関する仕様更新が必要になる。
- 影響範囲は主に [src/lib.rs](/Users/thinkami/project/sandbox/rust/lockfile_parser/src/lib.rs)、[src/parser.rs](/Users/thinkami/project/sandbox/rust/lockfile_parser/src/parser.rs)、および関連テスト。
