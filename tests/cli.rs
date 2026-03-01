use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

use jsonschema::{Draft, JSONSchema};
use serde_json::{json, Value};

fn sample_lockfile_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("examples/rails_relying_party_of_backend/Gemfile.lock")
}

fn schema_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("schema/parse-result.schema.json")
}

fn run_cli(path: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_lockfile_parser"))
        .arg(path)
        .output()
        .expect("cli should run")
}

fn parse_stdout_json(output: &Output) -> Value {
    serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "stdout should be JSON: {error}; stdout={}",
            String::from_utf8_lossy(&output.stdout)
        )
    })
}

fn compile_schema() -> JSONSchema {
    let schema = serde_json::from_str::<Value>(
        &fs::read_to_string(schema_path()).expect("schema should be readable"),
    )
    .expect("schema should be valid JSON");

    JSONSchema::options()
        .with_draft(Draft::Draft202012)
        .compile(&schema)
        .expect("schema should compile")
}

fn assert_valid_against_schema(instance: &Value) {
    let compiled = compile_schema();

    if !compiled.is_valid(instance) {
        let errors = compiled
            .validate(instance)
            .expect_err("invalid instance should produce validation errors")
            .map(|error| error.to_string())
            .collect::<Vec<_>>();
        panic!(
            "instance should satisfy schema: {}; instance={instance}",
            errors.join(", ")
        );
    }
}

fn assert_invalid_against_schema(instance: &Value) {
    let compiled = compile_schema();

    if compiled.is_valid(instance) {
        panic!("instance should not satisfy schema: {instance}");
    }
}

fn write_temp_file(contents: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time should advance")
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "lockfile_parser_cli_{}_{}.lock",
        std::process::id(),
        unique
    ));

    fs::write(&path, contents).expect("temp file should be writable");
    path
}

#[test]
fn parse_result_schemaをコンパイルできる() {
    let _ = compile_schema();
}

#[test]
fn 成功時のjsonがschemaに適合し必要な内容を含む() {
    let output = run_cli(&sample_lockfile_path());

    assert!(
        output.status.success(),
        "success path should exit 0: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.stderr.is_empty(),
        "success path should not write stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let payload = parse_stdout_json(&output);
    assert_valid_against_schema(&payload);

    assert_eq!(payload["status"], "ok");
    assert!(payload["error"].is_null());
    assert!(payload["warnings"].is_array());

    let data = payload["data"]
        .as_object()
        .expect("data should be an object");
    assert!(data.contains_key("locked_specs"));
    assert!(data.contains_key("top_level_dependencies"));

    let dependencies = data["top_level_dependencies"]
        .as_array()
        .expect("top_level_dependencies should be an array");
    let rails = dependencies
        .iter()
        .find(|dependency| dependency["name"] == "rails")
        .expect("rails should be present");
    let tzinfo_data = dependencies
        .iter()
        .find(|dependency| dependency["name"] == "tzinfo-data")
        .expect("tzinfo-data should be present");

    assert_eq!(rails["raw_requirement"], "~> 6.1.4");
    assert_eq!(rails["resolved_version"], "6.1.4");
    assert!(tzinfo_data["raw_requirement"].is_null());
    assert!(tzinfo_data["resolved_version"].is_null());

    let locked_specs = data["locked_specs"]
        .as_object()
        .expect("locked_specs should be an object");
    let rails_spec = locked_specs
        .get("rails")
        .expect("rails spec should be present")
        .as_object()
        .expect("rails spec should be an object");
    let dependencies = rails_spec["dependencies"]
        .as_array()
        .expect("spec dependencies should be an array");

    assert_eq!(rails_spec["version"], "6.1.4");
    assert!(
        dependencies
            .iter()
            .any(|dependency| dependency == "activerecord"),
        "rails dependencies should include activerecord"
    );
}

#[test]
fn parse_error時のjsonがschemaに適合しerror_shapeを満たす() {
    let invalid_lockfile = write_temp_file("GEM\n  remote: https://rubygems.org/\n");
    let output = run_cli(&invalid_lockfile);
    let _ = fs::remove_file(&invalid_lockfile);

    assert!(
        output.status.success(),
        "parse_error should still exit 0: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.stderr.is_empty(),
        "parse_error should not write stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let payload = parse_stdout_json(&output);
    assert_valid_against_schema(&payload);

    assert_eq!(payload["status"], "parse_error");
    assert!(payload["data"].is_null());
    assert_eq!(
        payload["warnings"]
            .as_array()
            .expect("warnings should be an array")
            .len(),
        0
    );

    let error = payload["error"]
        .as_object()
        .expect("error should be an object");
    let section = error["section"]
        .as_object()
        .expect("section should be an object");

    assert_eq!(error["code"], "missing_specs_subsection");
    assert_eq!(section["kind"], "eof");
    assert!(section["name"].is_null());
    assert_eq!(error["raw_line"], "");
}

#[test]
fn 未知セクションeofはwarningでもotherとして扱う() {
    let lockfile = write_temp_file(
        "GEM\n  specs:\n    alpha (1.0.0)\n\nDEPENDENCIES\n  alpha\n\nEOF\n  ignored\n",
    );
    let output = run_cli(&lockfile);
    let _ = fs::remove_file(&lockfile);

    assert!(
        output.status.success(),
        "success path should exit 0: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.stderr.is_empty(),
        "success path should not write stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let payload = parse_stdout_json(&output);
    assert_valid_against_schema(&payload);

    assert_eq!(payload["status"], "ok");
    let warnings = payload["warnings"]
        .as_array()
        .expect("warnings should be an array");
    let warning = warnings
        .iter()
        .find(|warning| warning["code"] == "ignored_section")
        .expect("ignored_section warning should be present");
    let section = warning["section"]
        .as_object()
        .expect("section should be an object");

    assert_eq!(section["kind"], "other");
    assert_eq!(section["name"], "EOF");
}

#[test]
fn 未知セクションeof内の構文エラーはeof扱いしない() {
    let lockfile = write_temp_file("EOF\n\tbad\n");
    let output = run_cli(&lockfile);
    let _ = fs::remove_file(&lockfile);

    assert!(
        output.status.success(),
        "parse_error should still exit 0: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.stderr.is_empty(),
        "parse_error should not write stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let payload = parse_stdout_json(&output);
    assert_valid_against_schema(&payload);

    assert_eq!(payload["status"], "parse_error");
    let error = payload["error"]
        .as_object()
        .expect("error should be an object");
    let section = error["section"]
        .as_object()
        .expect("section should be an object");

    assert_eq!(error["code"], "invalid_entry");
    assert_eq!(section["kind"], "other");
    assert_eq!(section["name"], "EOF");
    assert_eq!(error["raw_line"], "\tbad");
}

#[test]
fn schemaはeofの意味づけに反するpayloadを拒否する() {
    let eof_with_non_empty_raw_line = json!({
        "status": "parse_error",
        "data": null,
        "warnings": [],
        "error": {
            "code": "invalid_entry",
            "line": 2,
            "section": {
                "kind": "eof",
                "name": null
            },
            "raw_line": "  bad"
        }
    });
    let warning_with_eof_section = json!({
        "status": "ok",
        "data": {
            "top_level_dependencies": [],
            "locked_specs": {},
            "platforms": [],
            "ruby_version": null,
            "bundler_version": null
        },
        "warnings": [
            {
                "code": "ignored_section",
                "line": 1,
                "section": {
                    "kind": "eof",
                    "name": null
                },
                "raw_line": "EOF"
            }
        ],
        "error": null
    });
    let other_eof_with_empty_raw_line = json!({
        "status": "parse_error",
        "data": null,
        "warnings": [],
        "error": {
            "code": "invalid_entry",
            "line": 2,
            "section": {
                "kind": "other",
                "name": "EOF"
            },
            "raw_line": ""
        }
    });

    assert_invalid_against_schema(&eof_with_non_empty_raw_line);
    assert_invalid_against_schema(&warning_with_eof_section);
    assert_invalid_against_schema(&other_eof_with_empty_raw_line);
}
