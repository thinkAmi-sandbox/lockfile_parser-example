# parse-result JSON Schema

`schema/parse-result.schema.json` は、CLI が返す parse-result JSON の契約だけを定義します。

- 対象: `status = "ok"` と `status = "parse_error"` の JSON 応答
- 対象外: `--help`、`--version`、`usage_error`、`io_error`、`internal_error` の text 出力
- `error.section.kind = "eof"` は、ファイル末尾で確定した parse error だけを表します
- `warning.section.kind` では `"eof"` を使用しません
- `error.section.kind = "eof"` のとき、`error.raw_line` は空文字列です

順序に関する注意:

- `top_level_dependencies` の配列順は保証しません
- `warnings` の配列順は保証しません
- `locked_specs` の key 順は保証しません

text 出力は人間向けの補助であり、安定契約ではありません。
