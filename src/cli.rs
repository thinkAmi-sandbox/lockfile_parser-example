use std::collections::HashMap;
use std::fs;
use std::io::{self, Write};
use std::process::ExitCode;

use clap::{Arg, Command};
use lockfile_parser::{
    parse, ParseError, ParseErrorCode, ParsedGemfileLock, Section, WarningDiagnostic,
    WarningDiagnosticCode,
};
use serde::Serialize;

pub fn run() -> ExitCode {
    match try_run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(CliFailure::TextParse(error)) => {
            eprintln!("{}", format_parse_error_message(&error));
            ExitCode::from(1)
        }
        Err(CliFailure::Io(error)) => {
            eprintln!("io_error: {error}");
            ExitCode::from(1)
        }
        Err(CliFailure::Internal) => {
            eprintln!("internal_error");
            ExitCode::from(1)
        }
    }
}

fn try_run() -> Result<(), CliFailure> {
    let matches = build_command().get_matches();
    let format = parse_output_format(&matches);
    let source = matches
        .get_one::<String>("source")
        .expect("required by clap");
    let input = fs::read_to_string(source).map_err(CliFailure::Io)?;
    let parse_result = parse(&input);

    match format {
        OutputFormat::Json => {
            let response = map_parse_result(parse_result);
            print_json(&response)
        }
        OutputFormat::Text => print_text(parse_result),
    }
}

fn build_command() -> Command {
    Command::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .arg(
            Arg::new("format")
                .long("format")
                .value_name("FORMAT")
                .help("Output format")
                .value_parser(["json", "text"])
                .default_value("json"),
        )
        .arg(
            Arg::new("source")
                .value_name("SOURCE")
                .help("Path to Gemfile.lock")
                .required(true),
        )
}

fn parse_output_format(matches: &clap::ArgMatches) -> OutputFormat {
    match matches
        .get_one::<String>("format")
        .map(String::as_str)
        .expect("default by clap")
    {
        "json" => OutputFormat::Json,
        "text" => OutputFormat::Text,
        _ => unreachable!("value_parser limits format values"),
    }
}

fn print_json(response: &ParseResultEnvelope) -> Result<(), CliFailure> {
    let payload = serde_json::to_string(response).map_err(|_| CliFailure::Internal)?;
    let mut stdout = io::stdout().lock();

    stdout
        .write_all(payload.as_bytes())
        .map_err(CliFailure::Io)?;
    stdout.write_all(b"\n").map_err(CliFailure::Io)?;
    stdout.flush().map_err(CliFailure::Io)
}

fn print_text(result: Result<ParsedGemfileLock, ParseError>) -> Result<(), CliFailure> {
    match result {
        Ok(parsed) => {
            print_text_output(&parsed)?;
            print_text_warnings(&parsed.warnings)
        }
        Err(error) => Err(CliFailure::TextParse(error)),
    }
}

fn print_text_output(parsed: &ParsedGemfileLock) -> Result<(), CliFailure> {
    let mut entries = parsed
        .top_level_dependency_views()
        .map(|dependency| {
            (
                dependency.name.to_string(),
                dependency.resolved_version.map(str::to_string),
            )
        })
        .collect::<Vec<_>>();
    let mut payload = String::new();
    let mut stdout = io::stdout().lock();

    entries.sort_unstable_by(|left, right| left.0.cmp(&right.0));

    for (name, resolved_version) in entries {
        payload.push_str(&name);
        payload.push(' ');
        payload.push('[');
        if let Some(resolved_version) = resolved_version {
            payload.push_str(&resolved_version);
        }
        payload.push(']');
        payload.push('\n');
    }

    stdout
        .write_all(payload.as_bytes())
        .map_err(CliFailure::Io)?;
    stdout.flush().map_err(CliFailure::Io)
}

fn print_text_warnings(warnings: &[WarningDiagnostic]) -> Result<(), CliFailure> {
    let mut stderr = io::stderr().lock();

    for warning in warnings {
        let message = format_warning_message(warning);
        stderr
            .write_all(message.as_bytes())
            .map_err(CliFailure::Io)?;
        stderr.write_all(b"\n").map_err(CliFailure::Io)?;
    }

    stderr.flush().map_err(CliFailure::Io)
}

fn map_parse_result(result: Result<ParsedGemfileLock, ParseError>) -> ParseResultEnvelope {
    match result {
        Ok(parsed) => {
            let warnings = parsed.warnings.iter().map(map_warning).collect::<Vec<_>>();
            let data = map_parsed_result(parsed);

            ParseResultEnvelope {
                status: Status::Ok,
                data: Some(data),
                warnings,
                error: None,
            }
        }
        Err(error) => ParseResultEnvelope {
            status: Status::ParseError,
            data: None,
            warnings: Vec::new(),
            error: Some(map_parse_error(error)),
        },
    }
}

fn map_parsed_result(parsed: ParsedGemfileLock) -> ParsedResultDto {
    let top_level_dependencies = parsed
        .top_level_dependency_views()
        .map(|dependency| TopLevelDependencyViewDto {
            name: dependency.name.to_string(),
            raw_requirement: dependency.raw_requirement.map(str::to_string),
            resolved_version: dependency.resolved_version.map(str::to_string),
        })
        .collect::<Vec<_>>();

    let ParsedGemfileLock {
        locked_specs,
        platforms,
        ruby_version,
        bundler_version,
        ..
    } = parsed;

    let locked_specs = locked_specs
        .into_iter()
        .map(|(name, spec)| {
            (
                name,
                LockedSpecDto {
                    version: spec.version,
                    dependencies: spec.dependencies,
                },
            )
        })
        .collect::<HashMap<_, _>>();

    ParsedResultDto {
        top_level_dependencies,
        locked_specs,
        platforms,
        ruby_version,
        bundler_version,
    }
}

fn map_warning(warning: &WarningDiagnostic) -> WarningDto {
    WarningDto {
        code: map_warning_code(warning.code),
        line: warning.line,
        section: map_warning_section(&warning.section),
        raw_line: warning.raw_line.clone(),
    }
}

fn format_warning_message(warning: &WarningDiagnostic) -> String {
    let section = map_warning_section(&warning.section);

    format!(
        "warning: code={} line={} {}",
        warning_code_text(warning.code),
        warning.line,
        format_section_text(&section),
    )
}

fn map_parse_error(error: ParseError) -> ParseErrorDto {
    let section = map_parse_error_section(&error);

    ParseErrorDto {
        code: map_parse_error_code(error.code),
        line: error.line,
        section,
        raw_line: error.raw_line,
    }
}

fn format_parse_error_message(error: &ParseError) -> String {
    let section = map_parse_error_section(error);

    format!(
        "parse error: code={} line={} {}",
        parse_error_code_text(error.code),
        error.line,
        format_section_text(&section),
    )
}

fn map_warning_code(code: WarningDiagnosticCode) -> WarningCode {
    match code {
        WarningDiagnosticCode::IgnoredSection => WarningCode::IgnoredSection,
        WarningDiagnosticCode::IncompleteOptionalSection => WarningCode::IncompleteOptionalSection,
        WarningDiagnosticCode::DuplicateOptionalSection => WarningCode::DuplicateOptionalSection,
    }
}

fn warning_code_text(code: WarningDiagnosticCode) -> &'static str {
    match code {
        WarningDiagnosticCode::IgnoredSection => "ignored_section",
        WarningDiagnosticCode::IncompleteOptionalSection => "incomplete_optional_section",
        WarningDiagnosticCode::DuplicateOptionalSection => "duplicate_optional_section",
    }
}

fn map_parse_error_code(code: ParseErrorCode) -> ParseErrorCodeValue {
    match code {
        ParseErrorCode::MissingGemSection => ParseErrorCodeValue::MissingGemSection,
        ParseErrorCode::MissingSpecsSubsection => ParseErrorCodeValue::MissingSpecsSubsection,
        ParseErrorCode::MissingDependenciesSection => {
            ParseErrorCodeValue::MissingDependenciesSection
        }
        ParseErrorCode::InvalidEntry => ParseErrorCodeValue::InvalidEntry,
        ParseErrorCode::UnresolvedDependency => ParseErrorCodeValue::UnresolvedDependency,
        ParseErrorCode::UnsupportedResolvedSource => ParseErrorCodeValue::UnsupportedResolvedSource,
        ParseErrorCode::DuplicateEntry => ParseErrorCodeValue::DuplicateEntry,
        ParseErrorCode::InternalStateViolation => ParseErrorCodeValue::InternalStateViolation,
    }
}

fn parse_error_code_text(code: ParseErrorCode) -> &'static str {
    match code {
        ParseErrorCode::MissingGemSection => "missing_gem_section",
        ParseErrorCode::MissingSpecsSubsection => "missing_specs_subsection",
        ParseErrorCode::MissingDependenciesSection => "missing_dependencies_section",
        ParseErrorCode::InvalidEntry => "invalid_entry",
        ParseErrorCode::UnresolvedDependency => "unresolved_dependency",
        ParseErrorCode::UnsupportedResolvedSource => "unsupported_resolved_source",
        ParseErrorCode::DuplicateEntry => "duplicate_entry",
        ParseErrorCode::InternalStateViolation => "internal_state_violation",
    }
}

fn map_warning_section(section: &Section) -> SectionRefDto {
    map_standard_section(section)
}

fn map_parse_error_section(error: &ParseError) -> SectionRefDto {
    if matches!(&error.section, Section::Other(name) if name == "EOF") && error.raw_line.is_empty()
    {
        return SectionRefDto {
            kind: SectionKind::Eof,
            name: None,
        };
    }

    map_standard_section(&error.section)
}

fn map_standard_section(section: &Section) -> SectionRefDto {
    match section {
        Section::Gem => SectionRefDto {
            kind: SectionKind::Gem,
            name: None,
        },
        Section::GemSpecs => SectionRefDto {
            kind: SectionKind::GemSpecs,
            name: None,
        },
        Section::Dependencies => SectionRefDto {
            kind: SectionKind::Dependencies,
            name: None,
        },
        Section::Platforms => SectionRefDto {
            kind: SectionKind::Platforms,
            name: None,
        },
        Section::RubyVersion => SectionRefDto {
            kind: SectionKind::RubyVersion,
            name: None,
        },
        Section::BundledWith => SectionRefDto {
            kind: SectionKind::BundledWith,
            name: None,
        },
        Section::Other(name) => SectionRefDto {
            kind: SectionKind::Other,
            name: Some(name.clone()),
        },
    }
}

fn format_section_text(section: &SectionRefDto) -> String {
    format!("section={}", section_kind_text(&section.kind))
}

fn section_kind_text(kind: &SectionKind) -> &'static str {
    match kind {
        SectionKind::Gem => "gem",
        SectionKind::GemSpecs => "gem_specs",
        SectionKind::Dependencies => "dependencies",
        SectionKind::Platforms => "platforms",
        SectionKind::RubyVersion => "ruby_version",
        SectionKind::BundledWith => "bundled_with",
        SectionKind::Other => "other",
        SectionKind::Eof => "eof",
    }
}

enum CliFailure {
    TextParse(ParseError),
    Io(io::Error),
    Internal,
}

enum OutputFormat {
    Json,
    Text,
}

#[derive(Debug, Serialize)]
struct ParseResultEnvelope {
    status: Status,
    data: Option<ParsedResultDto>,
    warnings: Vec<WarningDto>,
    error: Option<ParseErrorDto>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
enum Status {
    Ok,
    ParseError,
}

#[derive(Debug, Serialize)]
struct ParsedResultDto {
    top_level_dependencies: Vec<TopLevelDependencyViewDto>,
    locked_specs: HashMap<String, LockedSpecDto>,
    platforms: Vec<String>,
    ruby_version: Option<String>,
    bundler_version: Option<String>,
}

#[derive(Debug, Serialize)]
struct TopLevelDependencyViewDto {
    name: String,
    raw_requirement: Option<String>,
    resolved_version: Option<String>,
}

#[derive(Debug, Serialize)]
struct LockedSpecDto {
    version: String,
    dependencies: Vec<String>,
}

#[derive(Debug, Serialize)]
struct WarningDto {
    code: WarningCode,
    line: usize,
    section: SectionRefDto,
    raw_line: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
enum WarningCode {
    IgnoredSection,
    IncompleteOptionalSection,
    DuplicateOptionalSection,
}

#[derive(Debug, Serialize)]
struct ParseErrorDto {
    code: ParseErrorCodeValue,
    line: usize,
    section: SectionRefDto,
    raw_line: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
enum ParseErrorCodeValue {
    MissingGemSection,
    MissingSpecsSubsection,
    MissingDependenciesSection,
    InvalidEntry,
    UnresolvedDependency,
    UnsupportedResolvedSource,
    DuplicateEntry,
    InternalStateViolation,
}

#[derive(Debug, Serialize)]
struct SectionRefDto {
    kind: SectionKind,
    name: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
enum SectionKind {
    Gem,
    GemSpecs,
    Dependencies,
    Platforms,
    RubyVersion,
    BundledWith,
    Other,
    Eof,
}
