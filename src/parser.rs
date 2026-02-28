use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedGemfileLock {
    pub top_level_dependencies: HashMap<String, TopLevelDependency>,
    pub locked_specs: HashMap<String, LockedSpec>,
    pub platforms: Vec<String>,
    pub ruby_version: Option<String>,
    pub bundler_version: Option<String>,
    pub warnings: Vec<WarningDiagnostic>,
}

impl ParsedGemfileLock {
    fn new() -> Self {
        Self {
            top_level_dependencies: HashMap::new(),
            locked_specs: HashMap::new(),
            platforms: Vec::new(),
            ruby_version: None,
            bundler_version: None,
            warnings: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TopLevelDependency {
    pub raw_requirement: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LockedSpec {
    pub version: String,
    pub dependencies: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Section {
    Gem,
    GemSpecs,
    Dependencies,
    Platforms,
    RubyVersion,
    BundledWith,
    Other(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseErrorCode {
    MissingGemSection,
    MissingSpecsSubsection,
    MissingDependenciesSection,
    InvalidEntry,
    UnresolvedDependency,
    UnsupportedResolvedSource,
    DuplicateEntry,
    InternalStateViolation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub code: ParseErrorCode,
    pub line: usize,
    pub section: Section,
    pub raw_line: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WarningDiagnosticCode {
    IgnoredSection,
    IncompleteOptionalSection,
    DuplicateOptionalSection,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WarningDiagnostic {
    pub code: WarningDiagnosticCode,
    pub line: usize,
    pub section: Section,
    pub raw_line: Option<String>,
}

pub fn parse(input: &str) -> Result<ParsedGemfileLock, ParseError> {
    let lines = normalize_lines(input);
    let mut state = ParserState::new(lines.len());

    for (index, raw_line) in lines.iter().enumerate() {
        state.line = index + 1;
        state.parse_line(raw_line)?;
    }

    state.finish()
}

#[derive(Debug)]
struct ParserState {
    total_lines: usize,
    line: usize,
    current_section: Section,
    current_spec_name: Option<String>,
    seen_sections: HashSet<Section>,
    pending_optional: Option<PendingOptionalSection>,
    dependency_references: Vec<DependencyReference>,
    result: ParsedGemfileLock,
}

impl ParserState {
    fn new(total_lines: usize) -> Self {
        Self {
            total_lines,
            line: 0,
            current_section: Section::Other("START".to_string()),
            current_spec_name: None,
            seen_sections: HashSet::new(),
            pending_optional: None,
            dependency_references: Vec::new(),
            result: ParsedGemfileLock::new(),
        }
    }

    fn parse_line(&mut self, raw_line: &str) -> Result<(), ParseError> {
        if raw_line.contains('\t') {
            return Err(self.error(
                ParseErrorCode::InvalidEntry,
                self.line,
                self.current_section.clone(),
                raw_line.to_string(),
            ));
        }

        match classify_line(raw_line) {
            LineKind::Blank => Ok(()),
            LineKind::TopLevelHeader(header) => {
                self.handle_top_level_header(header);
                Ok(())
            }
            LineKind::SpecsHeader => self.handle_specs_header(raw_line),
            LineKind::IndentedEntry { indent, text } => {
                self.handle_indented_entry(indent, text, raw_line)
            }
        }
    }

    fn handle_top_level_header(&mut self, header: &str) {
        self.finalize_optional_section();
        self.current_spec_name = None;

        match header {
            "GEM" => {
                self.seen_sections.insert(Section::Gem);
                self.current_section = Section::Gem;
            }
            "DEPENDENCIES" => {
                self.seen_sections.insert(Section::Dependencies);
                self.current_section = Section::Dependencies;
            }
            "PLATFORMS" => self.enter_optional_section(Section::Platforms, header),
            "RUBY VERSION" => self.enter_optional_section(Section::RubyVersion, header),
            "BUNDLED WITH" => self.enter_optional_section(Section::BundledWith, header),
            _ => {
                self.result.warnings.push(WarningDiagnostic {
                    code: WarningDiagnosticCode::IgnoredSection,
                    line: self.line,
                    section: Section::Other(header.to_string()),
                    raw_line: Some(header.to_string()),
                });
                self.current_section = Section::Other(header.to_string());
            }
        }
    }

    fn enter_optional_section(&mut self, section: Section, header: &str) {
        if self.seen_sections.contains(&section) {
            self.result.warnings.push(WarningDiagnostic {
                code: WarningDiagnosticCode::DuplicateOptionalSection,
                line: self.line,
                section: section.clone(),
                raw_line: Some(header.to_string()),
            });
            self.current_section = Section::Other(header.to_string());
            return;
        }

        self.seen_sections.insert(section.clone());
        self.current_section = section.clone();
        self.pending_optional = Some(PendingOptionalSection {
            section,
            header_line: self.line,
            has_value: false,
        });
    }

    fn handle_specs_header(&mut self, raw_line: &str) -> Result<(), ParseError> {
        match self.current_section {
            Section::Gem => {
                self.seen_sections.insert(Section::GemSpecs);
                self.current_section = Section::GemSpecs;
                self.current_spec_name = None;
                Ok(())
            }
            Section::Platforms | Section::RubyVersion | Section::BundledWith => {
                self.mark_optional_as_incomplete(raw_line);
                Ok(())
            }
            Section::Other(_) => Ok(()),
            _ => Err(self.error(
                ParseErrorCode::InvalidEntry,
                self.line,
                self.current_section.clone(),
                raw_line.to_string(),
            )),
        }
    }

    fn handle_indented_entry(
        &mut self,
        indent: usize,
        text: &str,
        raw_line: &str,
    ) -> Result<(), ParseError> {
        match self.current_section {
            Section::Gem => self.handle_gem_entry(indent, text, raw_line),
            Section::GemSpecs => self.handle_gem_specs_entry(indent, text, raw_line),
            Section::Dependencies => self.handle_dependency_entry(indent, text, raw_line),
            Section::Platforms => {
                self.handle_platform_entry(indent, text, raw_line);
                Ok(())
            }
            Section::RubyVersion => {
                self.handle_single_value_optional(Section::RubyVersion, indent, text, raw_line);
                Ok(())
            }
            Section::BundledWith => {
                self.handle_single_value_optional(Section::BundledWith, indent, text, raw_line);
                Ok(())
            }
            Section::Other(_) => Ok(()),
        }
    }

    fn handle_gem_entry(
        &mut self,
        indent: usize,
        text: &str,
        raw_line: &str,
    ) -> Result<(), ParseError> {
        if indent == 2 && text.starts_with("remote:") {
            return Ok(());
        }

        Err(self.error(
            ParseErrorCode::InvalidEntry,
            self.line,
            self.current_section.clone(),
            raw_line.to_string(),
        ))
    }

    fn handle_gem_specs_entry(
        &mut self,
        indent: usize,
        text: &str,
        raw_line: &str,
    ) -> Result<(), ParseError> {
        match indent {
            4 => {
                let (name, version) = split_parenthesized_value(text).ok_or_else(|| {
                    self.error(
                        ParseErrorCode::InvalidEntry,
                        self.line,
                        self.current_section.clone(),
                        raw_line.to_string(),
                    )
                })?;

                if self.result.locked_specs.contains_key(name) {
                    return Err(self.error(
                        ParseErrorCode::DuplicateEntry,
                        self.line,
                        self.current_section.clone(),
                        raw_line.to_string(),
                    ));
                }

                self.result.locked_specs.insert(
                    name.to_string(),
                    LockedSpec {
                        version: version.to_string(),
                        dependencies: Vec::new(),
                    },
                );
                self.current_spec_name = Some(name.to_string());
                Ok(())
            }
            6 => {
                let current_spec_name = self.current_spec_name.clone().ok_or_else(|| {
                    self.error(
                        ParseErrorCode::InternalStateViolation,
                        self.line,
                        self.current_section.clone(),
                        raw_line.to_string(),
                    )
                })?;

                let dependency_name = extract_dependency_name(text).ok_or_else(|| {
                    self.error(
                        ParseErrorCode::InvalidEntry,
                        self.line,
                        self.current_section.clone(),
                        raw_line.to_string(),
                    )
                })?;

                let Some(locked_spec) = self.result.locked_specs.get_mut(&current_spec_name) else {
                    return Err(self.error(
                        ParseErrorCode::InternalStateViolation,
                        self.line,
                        self.current_section.clone(),
                        raw_line.to_string(),
                    ));
                };

                locked_spec.dependencies.push(dependency_name.to_string());
                self.dependency_references.push(DependencyReference {
                    name: dependency_name.to_string(),
                    line: self.line,
                    section: self.current_section.clone(),
                    raw_line: raw_line.to_string(),
                    origin: DependencyOrigin::NestedSpec,
                });
                Ok(())
            }
            _ => Err(self.error(
                ParseErrorCode::InvalidEntry,
                self.line,
                self.current_section.clone(),
                raw_line.to_string(),
            )),
        }
    }

    fn handle_dependency_entry(
        &mut self,
        indent: usize,
        text: &str,
        raw_line: &str,
    ) -> Result<(), ParseError> {
        if indent != 2 {
            return Err(self.error(
                ParseErrorCode::InvalidEntry,
                self.line,
                self.current_section.clone(),
                raw_line.to_string(),
            ));
        }

        if text.ends_with('!') {
            return Err(self.error(
                ParseErrorCode::UnsupportedResolvedSource,
                self.line,
                self.current_section.clone(),
                raw_line.to_string(),
            ));
        }

        let (name, requirement) = parse_top_level_dependency(text).ok_or_else(|| {
            self.error(
                ParseErrorCode::InvalidEntry,
                self.line,
                self.current_section.clone(),
                raw_line.to_string(),
            )
        })?;

        if self.result.top_level_dependencies.contains_key(name) {
            return Err(self.error(
                ParseErrorCode::DuplicateEntry,
                self.line,
                self.current_section.clone(),
                raw_line.to_string(),
            ));
        }

        self.result.top_level_dependencies.insert(
            name.to_string(),
            TopLevelDependency {
                raw_requirement: requirement.map(str::to_string),
            },
        );
        self.dependency_references.push(DependencyReference {
            name: name.to_string(),
            line: self.line,
            section: self.current_section.clone(),
            raw_line: raw_line.to_string(),
            origin: DependencyOrigin::TopLevel,
        });
        Ok(())
    }

    fn handle_platform_entry(&mut self, indent: usize, text: &str, raw_line: &str) {
        if indent == 2 && !text.is_empty() {
            self.note_optional_value();
            self.result.platforms.push(text.to_string());
        } else {
            self.mark_optional_as_incomplete(raw_line);
        }
    }

    fn handle_single_value_optional(
        &mut self,
        section: Section,
        indent: usize,
        text: &str,
        raw_line: &str,
    ) {
        let Some(expected_indent) = expected_single_value_optional_indent(&section) else {
            self.mark_optional_as_incomplete(raw_line);
            return;
        };

        if indent != expected_indent || text.is_empty() {
            self.mark_optional_as_incomplete(raw_line);
            return;
        }

        match section {
            Section::RubyVersion => {
                if self.result.ruby_version.is_some() {
                    self.mark_optional_as_incomplete(raw_line);
                } else {
                    self.note_optional_value();
                    self.result.ruby_version = Some(text.to_string());
                }
            }
            Section::BundledWith => {
                if self.result.bundler_version.is_some() {
                    self.mark_optional_as_incomplete(raw_line);
                } else {
                    self.note_optional_value();
                    self.result.bundler_version = Some(text.to_string());
                }
            }
            _ => self.mark_optional_as_incomplete(raw_line),
        }
    }

    fn note_optional_value(&mut self) {
        if let Some(pending) = &mut self.pending_optional {
            pending.has_value = true;
        }
    }

    fn mark_optional_as_incomplete(&mut self, raw_line: &str) {
        let section = self.current_section.clone();
        self.pending_optional = None;
        self.result.warnings.push(WarningDiagnostic {
            code: WarningDiagnosticCode::IncompleteOptionalSection,
            line: self.line,
            section: section.clone(),
            raw_line: Some(raw_line.to_string()),
        });
        self.current_section = Section::Other(section.name().to_string());
        self.current_spec_name = None;
    }

    fn finalize_optional_section(&mut self) {
        if let Some(pending) = self.pending_optional.take() {
            if !pending.has_value {
                self.result.warnings.push(WarningDiagnostic {
                    code: WarningDiagnosticCode::IncompleteOptionalSection,
                    line: pending.header_line,
                    section: pending.section,
                    raw_line: None,
                });
            }
        }
    }

    fn finish(mut self) -> Result<ParsedGemfileLock, ParseError> {
        self.finalize_optional_section();

        if !self.seen_sections.contains(&Section::Gem) {
            return Err(self.eof_error(ParseErrorCode::MissingGemSection));
        }

        if !self.seen_sections.contains(&Section::GemSpecs) {
            return Err(self.eof_error(ParseErrorCode::MissingSpecsSubsection));
        }

        if !self.seen_sections.contains(&Section::Dependencies) {
            return Err(self.eof_error(ParseErrorCode::MissingDependenciesSection));
        }

        for reference in &self.dependency_references {
            if !reference.requires_locked_spec() {
                continue;
            }

            if !self.result.locked_specs.contains_key(&reference.name) {
                return Err(self.error(
                    ParseErrorCode::UnresolvedDependency,
                    reference.line,
                    reference.section.clone(),
                    reference.raw_line.clone(),
                ));
            }
        }

        Ok(self.result)
    }

    fn error(
        &self,
        code: ParseErrorCode,
        line: usize,
        section: Section,
        raw_line: String,
    ) -> ParseError {
        ParseError {
            code,
            line,
            section,
            raw_line,
        }
    }

    fn eof_error(&self, code: ParseErrorCode) -> ParseError {
        self.error(
            code,
            self.total_lines + 1,
            Section::Other("EOF".to_string()),
            String::new(),
        )
    }
}

#[derive(Debug)]
struct PendingOptionalSection {
    section: Section,
    header_line: usize,
    has_value: bool,
}

#[derive(Debug)]
struct DependencyReference {
    name: String,
    line: usize,
    section: Section,
    raw_line: String,
    origin: DependencyOrigin,
}

impl DependencyReference {
    fn requires_locked_spec(&self) -> bool {
        match self.origin {
            DependencyOrigin::TopLevel => false,
            DependencyOrigin::NestedSpec => self.name != "bundler",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DependencyOrigin {
    TopLevel,
    NestedSpec,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LineKind<'a> {
    Blank,
    TopLevelHeader(&'a str),
    SpecsHeader,
    IndentedEntry { indent: usize, text: &'a str },
}

impl Section {
    fn name(&self) -> &str {
        match self {
            Section::Gem => "GEM",
            Section::GemSpecs => "specs",
            Section::Dependencies => "DEPENDENCIES",
            Section::Platforms => "PLATFORMS",
            Section::RubyVersion => "RUBY VERSION",
            Section::BundledWith => "BUNDLED WITH",
            Section::Other(name) => name,
        }
    }
}

fn normalize_lines(input: &str) -> Vec<String> {
    if input.is_empty() {
        return Vec::new();
    }

    let mut lines = input
        .split('\n')
        .map(|line| line.trim_end_matches('\r').to_string())
        .collect::<Vec<_>>();

    if input.ends_with('\n') {
        lines.pop();
    }

    lines
}

fn classify_line(raw_line: &str) -> LineKind<'_> {
    if raw_line.trim().is_empty() {
        return LineKind::Blank;
    }

    let indent = raw_line.chars().take_while(|ch| *ch == ' ').count();
    if indent == 0 {
        return LineKind::TopLevelHeader(raw_line);
    }

    let text = &raw_line[indent..];
    if indent == 2 && text == "specs:" {
        return LineKind::SpecsHeader;
    }

    LineKind::IndentedEntry { indent, text }
}

fn split_parenthesized_value(input: &str) -> Option<(&str, &str)> {
    let open = input.rfind(" (")?;
    if !input.ends_with(')') {
        return None;
    }

    let name = &input[..open];
    let value = &input[(open + 2)..(input.len() - 1)];
    if name.is_empty() || value.is_empty() {
        return None;
    }

    Some((name, value))
}

fn extract_dependency_name(input: &str) -> Option<&str> {
    if let Some((name, _)) = split_parenthesized_value(input) {
        return Some(name);
    }

    if input.contains(' ') || input.is_empty() {
        return None;
    }

    Some(input)
}

fn parse_top_level_dependency(input: &str) -> Option<(&str, Option<&str>)> {
    if let Some((name, requirement)) = split_parenthesized_value(input) {
        return Some((name, Some(requirement)));
    }

    if input.contains(' ') || input.is_empty() {
        return None;
    }

    Some((input, None))
}

fn expected_single_value_optional_indent(section: &Section) -> Option<usize> {
    match section {
        Section::RubyVersion | Section::BundledWith => Some(3),
        _ => None,
    }
}
