## Why

Gemfile.lock を構造化して読めないため、Bundler が解決した gem のバージョンや依存関係を後続処理から安定して参照できません。脆弱性照会や最新バージョン確認の起点となる情報を型付きで取り出せるようにし、Gemfile.lock を前提にしたツール連携を始められる状態を作る必要があります。

## What Changes

- `GEM` セクション（通常解決）と `DEPENDENCIES` を中心に、Gemfile.lock をパースして構造化結果を返す機能を追加する。
- トップレベル依存、解決済み spec、platform、Ruby/Bundler バージョン、warning 診断を含む結果モデルを定義する。
- fatal なパース失敗と warning を区別し、行番号・セクション・生行を含む診断情報を返せるようにする。
- `GIT` / `PATH` / `PLUGIN` は V1 の対象外とし、未対応セクションとして扱うルールを定義する。

## Capabilities

### New Capabilities
- `gemfile-lock-parsing`: Gemfile.lock からトップレベル依存、解決済み依存、関連メタ情報、診断を型付きで取得する。

### Modified Capabilities

## Impact

- 新しい Gemfile.lock 解析用の Rust API と結果モデルが追加される。
- 依存関係の追跡、OSV 照会、最新版確認などの呼び出し側ロジックの入力源が明確になる。
- OpenSpec では新規 capability `gemfile-lock-parsing` の spec、設計、タスクが後続 artifact として追加される。
