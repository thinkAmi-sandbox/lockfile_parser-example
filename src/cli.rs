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
    let source = matches
        .get_one::<String>("source")
        .expect("required by clap");
    let input = fs::read_to_string(source).map_err(CliFailure::Io)?;
    let response = map_parse_result(parse(&input));
    print_json(&response)
}

fn build_command() -> Command {
    Command::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .arg(
            Arg::new("source")
                .value_name("SOURCE")
                .help("Path to Gemfile.lock")
                .required(true),
        )
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

fn map_parse_error(error: ParseError) -> ParseErrorDto {
    let section = map_parse_error_section(&error);

    ParseErrorDto {
        code: map_parse_error_code(error.code),
        line: error.line,
        section,
        raw_line: error.raw_line,
    }
}

fn map_warning_code(code: WarningDiagnosticCode) -> WarningCode {
    match code {
        WarningDiagnosticCode::IgnoredSection => WarningCode::IgnoredSection,
        WarningDiagnosticCode::IncompleteOptionalSection => WarningCode::IncompleteOptionalSection,
        WarningDiagnosticCode::DuplicateOptionalSection => WarningCode::DuplicateOptionalSection,
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

enum CliFailure {
    Io(io::Error),
    Internal,
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
