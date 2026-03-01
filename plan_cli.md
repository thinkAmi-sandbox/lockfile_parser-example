# CLI Plan

## 目的

このプロジェクトに追加する CLI は、人間向けの操作ツールというより、AI エージェントや他ツール向けの薄いラッパーとして設計する。

## 前提方針

- `library-first` を維持する
- CLI は薄いラッパーにする
- 公開インターフェースを増やしすぎない

パース本体は既存ライブラリに残し、CLI は入力取得、エラーハンドリング、出力整形だけを担当する。

## 出力方針

- 標準出力は JSON を基本とする
- 成功時も失敗時も、`stdout` に 1 個の JSON オブジェクトを返す
- `stderr` は原則として契約しない
- 終了コードは補助信号で、主な判定は JSON の `status` で行う

JSON はライブラリの公開型をそのまま外へ出さず、CLI 専用 DTO に写像して返す。

## 文字列表現

CLI の JSON に含める enum 相当の値は、Rust の PascalCase をそのまま使わず、`snake_case` にする。

例:

- `MissingGemSection` -> `missing_gem_section`
- `GemSpecs` -> `gem_specs`

この変換はライブラリ側ではなく、CLI ラッパー側で行う。より正確には、CLI 専用 DTO へ写像する過程で wire format を確定する。

## コマンド形

現時点では機能が実質 1 つだけなので、`parse` サブコマンドは必須ではない。最小案は単一コマンドとする。

想定:

- `lockfile_parser <SOURCE>`

補助的に将来追加しうるもの:

- `--format text`

ただし AI エージェント向けを最優先するため、v1 は JSON のみでもよい。

`stdin` 読み取りは自動ではなく、`-` を明示させる。

例:

- `lockfile_parser path/to/Gemfile.lock`
- `lockfile_parser -`

## トップレベル依存の解決済みバージョン

CLI では、トップレベル gem の解決済みバージョンを明示的に扱いたい。

ただし、これは CLI だけで独自に `top_level_dependencies` と `locked_specs` を join するのではなく、ライブラリ側の共通 helper を使って扱う。

方針:

- `TopLevelDependency` に `resolved_version` を直接持たせない
- 既存の正規化データモデル (`top_level_dependencies` / `locked_specs`) は維持する
- `ParsedGemfileLock` に、トップレベル依存を `name` / `raw_requirement` / `resolved_version` 付きで見られる一覧 helper を持たせる
- helper の返り値は借用ビューにする
- helper の順序は非保証

この helper はライブラリ利用者と CLI の両方で共通利用する。

## 既に反映した内容

CLI 自体は未実装だが、前提となるライブラリ helper は実装済み。

実装済み内容:

- `ParsedGemfileLock::top_level_dependency_views()` の追加
- `TopLevelDependencyView<'a>` の追加
- crate ルートでの再 export
- 対応テストの追加

## まだ未実装の内容

以下は今後の CLI 実装で決めて実装する。

- JSON envelope の最終スキーマ
- `status` の具体的な値
- `error.code` / `section.kind` の最終定義
- CLI 専用 DTO の構造
- 引数仕様 (`SOURCE`, `-`, 必要なら `--format`)
- 実際の CLI バイナリ実装

## 次の実装順

1. JSON envelope のスキーマを固定する
2. CLI の引数仕様を確定する
3. CLI 専用 DTO を定義する
4. ライブラリ helper を使って JSON を組み立てる CLI を実装する
