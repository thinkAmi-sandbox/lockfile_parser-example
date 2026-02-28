## Context

現在の `ParsedGemfileLock` は、トップレベル依存を `top_level_dependencies`、解決済み spec を `locked_specs` として分離して保持しています。この正規化された構造は依存グラフの参照には適していますが、トップレベル依存に対応する解決済みバージョンを知りたい利用者は、毎回 gem 名で 2 つのコレクションを突き合わせる必要があります。

この突き合わせはライブラリ利用者だけでなく、今後追加予定の CLI でも同じ形で必要になります。一方で、`TopLevelDependency` に解決済みバージョンを直接保持させると、`locked_specs` と同じ情報を二重管理することになり、同期漏れや意味の重複を招きます。

## Goals / Non-Goals

**Goals:**
- `ParsedGemfileLock` からトップレベル依存を解決済みバージョン付きで参照できる公開 helper を追加する。
- 既存の正規化されたデータモデルを維持したまま、利用側の join ロジックをライブラリへ集約する。
- CLI や他の利用者が同じ導出ロジックを再利用できるようにする。

**Non-Goals:**
- `TopLevelDependency` や `LockedSpec` の保存構造を変更しない。
- helper に順序保証を持たせない。
- CLI、JSON 出力、表示形式の変更はこの change に含めない。

## Decisions

- `ParsedGemfileLock` に一覧取得用の公開 helper を追加し、トップレベル依存を解決済みバージョン付きで列挙できるようにする。主要ユースケースは一括参照であり、単発 lookup 用 API は現時点では追加しない。
  - 代替案として名前指定 helper も考えられるが、一覧 helper があればライブラリ利用者と CLI の両方を満たせるため、公開面を最小にできる。
- helper は読み取り専用の借用ビューを返す。ビューには `name`、`raw_requirement`、`resolved_version` を含め、`resolved_version` は `locked_specs` から同名 key を参照して導出する。
  - 代替案として所有する値を返す方法もあるが、今回の用途は派生ビューであり、文字列 clone や余分なメモリ確保を避けたい。
- `resolved_version` は `TopLevelDependency` に追加せず、helper 実行時に毎回導出する。
  - 代替案として `TopLevelDependency` へフィールドを追加すると参照は簡単になるが、`locked_specs` と情報が重複し、更新時の整合性を壊しやすい。
- helper の列挙順は `HashMap` の反復順に従い、順序を保証しない。
  - 代替案として名前順に安定化する方法もあるが、並び替えは表示層で対応できるため、library helper の責務を増やさない。

## Risks / Trade-offs

- [公開 API に lifetime が出る] → 借用ビューは所有型より扱いがやや難しいが、派生データであることを型で表現でき、不要なコピーも避けられる。
- [利用者が順序を前提にする] → helper は順序非保証であることを仕様とテストで明示し、必要なソートは呼び出し側で行う。
- [helper と既存フィールドの意味が混同される] → `resolved_version` は保存値ではなく `locked_specs` からの導出値であることを設計と spec に明記する。
