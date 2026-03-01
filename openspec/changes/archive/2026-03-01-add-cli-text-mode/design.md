## Context

現行の CLI は `src/cli.rs` で単一の `SOURCE` 引数を受け取り、`parse` の結果を常に parse-result JSON として `stdout` へ返す。`ParsedGemfileLock::top_level_dependency_views()` により、トップレベル依存と対応する解決済みバージョンをまとめて参照する情報はすでに取得できるため、今回の変更は主に CLI の出力モード追加と出力経路の整理で完結できる。

制約は以下のとおり。

- 既存の `json` モードの出力契約は壊さない
- `--format` の既定値は `json` とし、許可値は `json` と `text` のみとする
- `text` モードの一覧は `stdout`、warning / parse error は `stderr` に分離する
- `text` モードの warning 文面と parse error 文面は text 出力とするが、細かな wording は安定契約にしない
- 既存の JSON Schema は parse-result JSON 専用のまま維持する

## Goals / Non-Goals

**Goals:**

- `--format text` を追加し、人間向けのトップレベル依存一覧を返せるようにする
- 既存の `json` モードの入出力、終了コード、JSON Schema 契約を維持する
- 既存のパーサーと `top_level_dependency_views()` を再利用し、ライブラリのデータモデル変更なしで実現する
- `text` モードの `stdout` / `stderr` / 終了コードの責務を明確化し、CLI テストで検証しやすくする

**Non-Goals:**

- parse-result JSON の shape や JSON Schema を変更すること
- `gemfile-lock-parsing` capability やライブラリ公開 API を変更すること
- `DEPENDENCIES` の元の記述順を保持するためにパーサーの内部表現を変更すること
- `text` モードの stderr wording を安定契約化すること
- `stdin` 入力、複数ファイル入力、追加のサブコマンドを導入すること

## Decisions

### 1. CLI に明示的な出力モードを追加し、既定値は `json` のまま維持する

`build_command()` に `--format` オプションを追加し、`json` と `text` のみを受け付ける。`--format` 省略時は `json` を選び、既存の `lockfile_parser <SOURCE>` の挙動をそのまま維持する。

デフォルトを `text` に変える案もあるが、既存の JSON 利用者にとって破壊的であり、現在の capability とテスト資産を壊すため採用しない。

### 2. パースは 1 回だけ行い、その後で出力モードごとに分岐する

`try_run()` は引数解釈とファイル読込の後に `parse(&input)` を 1 回だけ呼び、以降は `json` モード用の出力関数と `text` モード用の出力関数に分岐する。これにより、パーサー実行や診断生成を重複させず、モード差分を CLI 層に閉じ込められる。

`json` モードでは既存の `map_parse_result` と JSON 出力を継続利用する。`text` モードでは `Result<ParsedGemfileLock, ParseError>` を直接扱い、成功時は一覧を出力し、warning は `stderr` に追記し、fatal な parse error は `stderr` へ text で出して `ExitCode::from(1)` を返す。

parse 結果を一度 JSON DTO に変換してから text へ再変換する案もあるが、`text` モード固有の終了コードや `stderr` 分離を表現しにくく、責務が不自然になるため採用しない。

### 3. `text` モードの一覧は `top_level_dependency_views()` をソートして構築する

`text` モードの成功出力は、`top_level_dependency_views()` から得た項目を gem 名の文字列昇順でソートし、各行を以下の形式で `stdout` に出す。

- `resolved_version` がある場合: `name [version]`
- `resolved_version` がない場合: `name []`

この方法なら、既存のライブラリ helper を再利用でき、トップレベル依存の一覧化に新しいデータ構造を持ち込まずに済む。加えて、パーサーは空文字の version を受理しないため、`name []` は未解決専用の表現として使える。対象が 0 件なら `stdout` は空とし、出力がある場合は末尾改行を付ける。

`DEPENDENCIES` の記述順をそのまま保持する案もあるが、現在は `HashMap` ベースで順序を保持しておらず、順序保証のためにパーサーの内部表現や公開型を広げるのは今回のスコープに対して過剰なため採用しない。

### 4. `text` モードでは `stdout` と `stderr` を厳密に分離する

`stdout` には一覧のみを書き、warning と parse error は一切混在させない。warning と parse error は `stderr` へ text で出し、少なくとも以下を含める。

- 接頭辞: `warning` または `parse error`
- 診断コード
- 行番号
- セクション (`kind` 相当の英字)

入力由来の `Section::Other.name` は text モードの診断へ出力しない。未知セクション名は lockfile の生文字列であり、端末やログへそのまま流すと ANSI / 制御文字の注入経路になるため、text 診断では `section=other` の kind 情報だけを残す。

warning は成功扱いのため終了コード `0` を維持し、parse error は終了コード `1` を返す。これにより、`stdout` を一覧取得用に安全にパイプできる。

warning や parse error を `stdout` に混ぜる案、または `stderr` 文面を厳密に固定する案もあるが、前者はパイプ利用を壊し、後者は変更余地を不必要に狭めるため採用しない。

### 5. `json` モードの契約と JSON Schema は変更しない

`json` モードは既存の parse-result JSON 契約をそのまま維持し、warning は JSON 配列で返し、`parse_error` も従来どおり JSON envelope と既存の終了コードで扱う。`schema/parse-result.schema.json` は `json` モード専用のままとし、`text` モードの text 出力は schema の対象外とする。

`text` モードまで schema 化する案もあるが、改行区切りの人間向け表示に対して過剰であり、既存 schema の役割を曖昧にするため採用しない。

## Risks / Trade-offs

- [Risk] `json` モードと `text` モードで parse error の扱いが異なるため、利用者が終了コード差分を見落とす可能性がある → Mitigation: spec と CLI テストでモード別の挙動を明示し、`--format` ごとに検証する
- [Risk] `stderr` wording を安定契約にしないため、文面全体を固定比較するテストが壊れやすい → Mitigation: テストでは接頭辞、コード、行番号、セクションなど必要な断片のみを検証する
- [Risk] 文字列昇順の採用により `DEPENDENCIES` の記述順とは異なる表示になる → Mitigation: text モードの並び順を仕様として明記し、元順序保持はスコープ外とする
- [Risk] 成功かつ表示対象 0 件のとき `stdout` が空になるため、見た目だけでは結果が分かりにくい → Mitigation: 空出力を明示的な正常系として仕様化し、CLI テストで保証する

## Migration Plan

この変更は既存 CLI への追加であり、`json` モードの利用者に移行作業は発生しない。実装では `src/cli.rs` の引数解釈と出力分岐を拡張し、`tests/cli.rs` に `json` / `text` 両モードの挙動を追加検証する。問題があれば `--format text` 関連の分岐を取り除くことで、既存の JSON 専用 CLI に戻せる。

## Open Questions

- `text` モードの `stderr` 文面は安定契約にしない前提だが、実装時にどこまで共通 helper 化するかはコードの見通しを見て決めてよい
