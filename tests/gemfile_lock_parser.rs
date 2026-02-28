use lockfile_parser::{
    parse, ParseErrorCode, Section, WarningDiagnosticCode,
};

const SAMPLE_LOCKFILE: &str =
    include_str!("../examples/rails_relying_party_of_backend/Gemfile.lock");

#[test]
fn サンプルlockfileを構造化結果に変換できる() {
    let parsed = parse(SAMPLE_LOCKFILE).expect("sample lockfile should parse");

    assert_eq!(
        parsed
            .top_level_dependencies
            .get("rails")
            .and_then(|dependency| dependency.raw_requirement.as_deref()),
        Some("~> 6.1.4")
    );
    assert_eq!(
        parsed
            .top_level_dependencies
            .get("omniauth")
            .and_then(|dependency| dependency.raw_requirement.as_deref()),
        None
    );
    assert_eq!(
        parsed
            .top_level_dependencies
            .get("tzinfo-data")
            .and_then(|dependency| dependency.raw_requirement.as_deref()),
        None
    );
    assert_eq!(
        parsed
            .locked_specs
            .get("rails")
            .map(|spec| spec.version.as_str()),
        Some("6.1.4")
    );
    assert!(
        parsed
            .locked_specs
            .get("rails")
            .expect("rails should be parsed")
            .dependencies
            .contains(&"activerecord".to_string())
    );
    assert!(!parsed.locked_specs.contains_key("bundler"));
    assert!(!parsed.locked_specs.contains_key("tzinfo-data"));
    assert_eq!(parsed.platforms, vec!["x86_64-darwin-19".to_string()]);
    assert_eq!(parsed.ruby_version.as_deref(), Some("ruby 3.0.1p64"));
    assert_eq!(parsed.bundler_version.as_deref(), Some("2.2.21"));
    assert!(parsed.warnings.is_empty());
}

#[test]
fn optionalメタ情報を保持し想定どおりwarningを返す() {
    let input = r#"GIT
  remote: https://example.com/private.git

GEM
  remote: https://rubygems.org/
  specs:
    alpha (1.0.0)

PLATFORMS
  ruby

PLATFORMS
  x86_64-linux

DEPENDENCIES
  alpha

RUBY VERSION

BUNDLED WITH
   2.5.0
"#;

    let parsed = parse(input).expect("warnings should not be fatal");

    assert_eq!(parsed.platforms, vec!["ruby".to_string()]);
    assert_eq!(parsed.ruby_version, None);
    assert_eq!(parsed.bundler_version.as_deref(), Some("2.5.0"));
    assert_eq!(parsed.warnings.len(), 3);
    assert_eq!(parsed.warnings[0].code, WarningDiagnosticCode::IgnoredSection);
    assert_eq!(parsed.warnings[0].line, 1);
    assert_eq!(parsed.warnings[0].section, Section::Other("GIT".to_string()));
    assert_eq!(
        parsed.warnings[1].code,
        WarningDiagnosticCode::DuplicateOptionalSection
    );
    assert_eq!(parsed.warnings[1].line, 12);
    assert_eq!(parsed.warnings[1].section, Section::Platforms);
    assert_eq!(
        parsed.warnings[2].code,
        WarningDiagnosticCode::IncompleteOptionalSection
    );
    assert_eq!(parsed.warnings[2].line, 18);
    assert_eq!(parsed.warnings[2].section, Section::RubyVersion);
    assert_eq!(parsed.warnings[2].raw_line, None);
}

#[test]
fn トップレベル依存を解決済みバージョン付きで参照できる() {
    let parsed = parse(SAMPLE_LOCKFILE).expect("sample lockfile should parse");

    let rails = parsed
        .top_level_dependency_views()
        .find(|dependency| dependency.name == "rails")
        .expect("rails should be included");
    assert_eq!(rails.raw_requirement, Some("~> 6.1.4"));
    assert_eq!(rails.resolved_version, Some("6.1.4"));

    let tzinfo_data = parsed
        .top_level_dependency_views()
        .find(|dependency| dependency.name == "tzinfo-data")
        .expect("tzinfo-data should be included");
    assert_eq!(tzinfo_data.raw_requirement, None);
    assert_eq!(tzinfo_data.resolved_version, None);
}

#[test]
fn 未知のトップレベルセクションでwarningを返す() {
    let input = r#"CUSTOM SOURCE
  cache: vendor/cache

GEM
  remote: https://rubygems.org/
  specs:
    alpha (1.0.0)

DEPENDENCIES
  alpha
"#;

    let parsed = parse(input).expect("unknown top-level sections should not be fatal");

    assert_eq!(parsed.warnings.len(), 1);
    assert_eq!(parsed.warnings[0].code, WarningDiagnosticCode::IgnoredSection);
    assert_eq!(parsed.warnings[0].line, 1);
    assert_eq!(
        parsed.warnings[0].section,
        Section::Other("CUSTOM SOURCE".to_string())
    );
    assert_eq!(
        parsed.warnings[0].raw_line.as_deref(),
        Some("CUSTOM SOURCE")
    );
}

#[test]
fn optionalセクションの不正インデントで不完全warningを返す() {
    let input = r#"GEM
  remote: https://rubygems.org/
  specs:
    alpha (1.0.0)

DEPENDENCIES
  alpha

PLATFORMS
 ruby

RUBY VERSION
  ruby 3.2.2

BUNDLED WITH
    2.5.0
"#;

    let parsed = parse(input).expect("invalid optional indentation should warn");

    assert!(parsed.platforms.is_empty());
    assert_eq!(parsed.ruby_version, None);
    assert_eq!(parsed.bundler_version, None);
    assert_eq!(parsed.warnings.len(), 3);
    assert_eq!(
        parsed.warnings[0].code,
        WarningDiagnosticCode::IncompleteOptionalSection
    );
    assert_eq!(parsed.warnings[0].line, 10);
    assert_eq!(parsed.warnings[0].section, Section::Platforms);
    assert_eq!(parsed.warnings[0].raw_line.as_deref(), Some(" ruby"));
    assert_eq!(
        parsed.warnings[1].code,
        WarningDiagnosticCode::IncompleteOptionalSection
    );
    assert_eq!(parsed.warnings[1].line, 13);
    assert_eq!(parsed.warnings[1].section, Section::RubyVersion);
    assert_eq!(
        parsed.warnings[1].raw_line.as_deref(),
        Some("  ruby 3.2.2")
    );
    assert_eq!(
        parsed.warnings[2].code,
        WarningDiagnosticCode::IncompleteOptionalSection
    );
    assert_eq!(parsed.warnings[2].line, 16);
    assert_eq!(parsed.warnings[2].section, Section::BundledWith);
    assert_eq!(parsed.warnings[2].raw_line.as_deref(), Some("    2.5.0"));
}

#[test]
fn 不正なエントリで位置情報付きエラーを返す() {
    let input = "GEM\n  remote: https://rubygems.org/\n  specs:\n    alpha (1.0.0)\n   stray\n\nDEPENDENCIES\n  alpha\n";
    let error = parse(input).expect_err("invalid indentation should fail");

    assert_eq!(error.code, ParseErrorCode::InvalidEntry);
    assert_eq!(error.line, 5);
    assert_eq!(error.section, Section::GemSpecs);
    assert_eq!(error.raw_line, "   stray");
}

#[test]
fn 未対応の解決元で位置情報付きエラーを返す() {
    let input = "GEM\n  remote: https://rubygems.org/\n  specs:\n    alpha (1.0.0)\n\nDEPENDENCIES\n  alpha!\n";
    let error = parse(input).expect_err("unsupported sources should fail");

    assert_eq!(error.code, ParseErrorCode::UnsupportedResolvedSource);
    assert_eq!(error.line, 7);
    assert_eq!(error.section, Section::Dependencies);
    assert_eq!(error.raw_line, "  alpha!");
}

#[test]
fn 重複エントリで位置情報付きエラーを返す() {
    let input = "GEM\n  remote: https://rubygems.org/\n  specs:\n    alpha (1.0.0)\n    alpha (1.1.0)\n\nDEPENDENCIES\n  alpha\n";
    let error = parse(input).expect_err("duplicate specs should fail");

    assert_eq!(error.code, ParseErrorCode::DuplicateEntry);
    assert_eq!(error.line, 5);
    assert_eq!(error.section, Section::GemSpecs);
    assert_eq!(error.raw_line, "    alpha (1.1.0)");
}

#[test]
fn 未解決依存で位置情報付きエラーを返す() {
    let input = "GEM\n  remote: https://rubygems.org/\n  specs:\n    alpha (1.0.0)\n      beta (~> 1.0)\n\nDEPENDENCIES\n  alpha\n";
    let error = parse(input).expect_err("missing nested specs should fail");

    assert_eq!(error.code, ParseErrorCode::UnresolvedDependency);
    assert_eq!(error.line, 5);
    assert_eq!(error.section, Section::GemSpecs);
    assert_eq!(error.raw_line, "      beta (~> 1.0)");
}

#[test]
fn eofで必須セクション不足の位置情報付きエラーを返す() {
    let missing_gem = parse("DEPENDENCIES\n  alpha\n").expect_err("missing gem should fail");
    assert_eq!(missing_gem.code, ParseErrorCode::MissingGemSection);
    assert_eq!(missing_gem.line, 3);
    assert_eq!(missing_gem.section, Section::Other("EOF".to_string()));
    assert_eq!(missing_gem.raw_line, "");

    let missing_specs =
        parse("GEM\n  remote: https://rubygems.org/\n\nDEPENDENCIES\n").expect_err("missing specs should fail");
    assert_eq!(missing_specs.code, ParseErrorCode::MissingSpecsSubsection);
    assert_eq!(missing_specs.line, 5);
    assert_eq!(missing_specs.section, Section::Other("EOF".to_string()));
    assert_eq!(missing_specs.raw_line, "");

    let missing_dependencies =
        parse("GEM\n  remote: https://rubygems.org/\n  specs:\n    alpha (1.0.0)\n").expect_err("missing dependencies should fail");
    assert_eq!(
        missing_dependencies.code,
        ParseErrorCode::MissingDependenciesSection
    );
    assert_eq!(missing_dependencies.line, 5);
    assert_eq!(missing_dependencies.section, Section::Other("EOF".to_string()));
    assert_eq!(missing_dependencies.raw_line, "");
}
