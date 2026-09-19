//! CLI boundary for command handling.

use std::io::{self, Write};

use clap::{Args, Parser, Subcommand, ValueEnum, error::ErrorKind};

use crate::domain::{
    AuthorizedHost, ContentFormat, DiscoveryConfiguration, DiscoveryConfigurationError,
    DiscoveryScope, SiteBoundary, SkillName, SourceUrl, TraversalMode,
};
use crate::service::{CreateSkillResult, UpdateSkillsResult};

/// Marks a component that handles CLI commands.
pub trait CommandHandler {}

/// The presentation-ready outcome of one CLI command.
#[derive(Debug)]
pub enum CommandOutput {
    /// One skill was created.
    Created(SkillName),
    /// One or more selected skills were updated independently.
    Updated(Vec<SkillOutput>),
    /// Updating all skills found no managed skills.
    NoManagedSkills,
}

impl CommandOutput {
    /// Converts a successful create-service result into CLI output.
    pub fn from_created(result: CreateSkillResult) -> Self {
        Self::Created(result.name().clone())
    }

    /// Converts update-service results into one CLI outcome per selected skill.
    pub fn from_updated(result: UpdateSkillsResult) -> Self {
        let outcomes = result
            .outcomes()
            .iter()
            .map(|outcome| match outcome.result() {
                Ok(result) => SkillOutput::Updated(result.name().clone()),
                Err(error) => SkillOutput::Failed {
                    name: outcome.name().clone(),
                    reason: error.to_string(),
                },
            })
            .collect();

        Self::Updated(outcomes)
    }
}

/// The presentation-ready outcome for one selected skill.
#[derive(Debug)]
pub enum SkillOutput {
    /// The skill was updated successfully.
    Updated(SkillName),
    /// The skill failed while the remaining selection continued.
    Failed {
        /// Name of the skill that failed.
        name: SkillName,
        /// English error message explaining the failure.
        reason: String,
    },
}

/// Writes a completed command outcome and returns its process exit code.
pub fn write_command_output(
    output: CommandOutput,
    standard_output: &mut dyn Write,
    standard_error: &mut dyn Write,
) -> io::Result<u8> {
    match output {
        CommandOutput::Created(name) => {
            writeln!(standard_output, "Created skill: {}", name.as_str())?;
            Ok(0)
        }
        CommandOutput::Updated(outcomes) => {
            let mut has_failures = false;
            for outcome in outcomes {
                match outcome {
                    SkillOutput::Updated(name) => {
                        writeln!(standard_output, "Updated skill: {}", name.as_str())?;
                    }
                    SkillOutput::Failed { name, reason } => {
                        writeln!(standard_error, "Failed skill: {}: {reason}", name.as_str())?;
                        has_failures = true;
                    }
                }
            }
            Ok(if has_failures { 1 } else { 0 })
        }
        CommandOutput::NoManagedSkills => {
            writeln!(standard_output, "No managed skills found.")?;
            Ok(0)
        }
    }
}

/// Writes an argument validation failure and returns the invalid-argument exit code.
pub fn write_invalid_argument(
    reason: &str,
    _standard_output: &mut dyn Write,
    standard_error: &mut dyn Write,
) -> io::Result<u8> {
    writeln!(standard_error, "Invalid argument: {reason}")?;
    Ok(2)
}

/// Parsed and validated command-line arguments.
#[derive(Debug)]
pub struct Cli {
    /// The requested command.
    pub command: Command,
}

impl Cli {
    /// Parses and validates command-line arguments.
    pub fn try_parse_from<I, T>(arguments: I) -> Result<Self, clap::Error>
    where
        I: IntoIterator<Item = T>,
        T: Into<std::ffi::OsString> + Clone,
    {
        RawCli::try_parse_from(arguments)?.try_into()
    }
}

/// Commands supported by the CLI.
#[derive(Debug)]
pub enum Command {
    /// Creates a skill from a documentation source.
    Create(CreateArguments),
    /// Updates one, several, or all managed skills.
    Update(UpdateArguments),
}

/// Arguments for the `create` command.
#[derive(Debug)]
pub struct CreateArguments {
    /// Documentation source to retrieve.
    pub source_url: SourceUrl,
    /// Name for the created skill.
    pub skill_name: SkillName,
    /// Discovery scope.
    pub scope: DiscoveryScope,
    /// Same-site boundary.
    pub site_boundary: SiteBoundary,
    /// Explicitly allowed base-domain subdomains.
    pub allowed_subdomains: Vec<AuthorizedHost>,
    /// Link traversal mode.
    pub traversal: TraversalMode,
    /// Effective maximum page count for limited traversal.
    pub max_pages: Option<usize>,
    /// Rendered content format.
    pub content_format: ContentFormat,
    /// Optional HTTP user agent.
    pub user_agent: Option<String>,
}

/// Arguments for the `update` command.
#[derive(Debug)]
pub struct UpdateArguments {
    /// Selected skills. An empty selection means all managed skills.
    pub skill_names: Vec<SkillName>,
    /// Optional replacement skill name.
    pub name: Option<SkillName>,
    /// Optional replacement source URL.
    pub source_url: Option<SourceUrl>,
    /// Optional replacement discovery scope.
    pub scope: Option<DiscoveryScope>,
    /// Optional replacement same-site boundary.
    pub site_boundary: Option<SiteBoundary>,
    /// Optional replacement allowed subdomains.
    pub allowed_subdomains: Vec<AuthorizedHost>,
    /// Optional replacement traversal mode.
    pub traversal: Option<TraversalMode>,
    /// Optional replacement maximum page count.
    pub max_pages: Option<usize>,
    /// Optional replacement content format.
    pub content_format: Option<ContentFormat>,
    /// Optional replacement HTTP user agent.
    pub user_agent: Option<String>,
}

impl UpdateArguments {
    /// Reports whether the command changes a skill configuration.
    pub fn has_changes(&self) -> bool {
        self.name.is_some()
            || self.source_url.is_some()
            || self.scope.is_some()
            || self.site_boundary.is_some()
            || !self.allowed_subdomains.is_empty()
            || self.traversal.is_some()
            || self.max_pages.is_some()
            || self.content_format.is_some()
            || self.user_agent.is_some()
    }
}

#[derive(Parser)]
#[command(name = "rust-skgen")]
struct RawCli {
    #[command(subcommand)]
    command: RawCommand,
}

#[derive(Subcommand)]
enum RawCommand {
    Create(RawCreateArguments),
    Update(RawUpdateArguments),
}

#[derive(Args)]
struct RawCreateArguments {
    #[arg(value_parser = parse_source_url)]
    source_url: SourceUrl,
    #[arg(value_parser = parse_skill_name)]
    skill_name: SkillName,
    #[arg(long, default_value = "same-site", value_parser = parse_scope)]
    scope: DiscoveryScope,
    #[arg(long, default_value = "exact-host", value_parser = parse_site_boundary)]
    site_boundary: SiteBoundary,
    #[arg(long = "allow-subdomain")]
    allowed_subdomains: Vec<String>,
    #[arg(long, default_value = "all", value_parser = parse_traversal)]
    traversal: TraversalMode,
    #[arg(long)]
    max_pages: Option<usize>,
    #[arg(long = "format", default_value = "guide-with-references", value_parser = parse_content_format)]
    content_format: ContentFormat,
    #[arg(long)]
    user_agent: Option<String>,
}

#[derive(Args)]
struct RawUpdateArguments {
    #[arg(value_parser = parse_skill_name)]
    skill_names: Vec<SkillName>,
    #[arg(long, value_parser = parse_skill_name)]
    name: Option<SkillName>,
    #[arg(long, value_parser = parse_source_url)]
    source_url: Option<SourceUrl>,
    #[arg(long, value_parser = parse_scope)]
    scope: Option<DiscoveryScope>,
    #[arg(long, value_parser = parse_site_boundary)]
    site_boundary: Option<SiteBoundary>,
    #[arg(long = "allow-subdomain")]
    allowed_subdomains: Vec<String>,
    #[arg(long, value_parser = parse_traversal)]
    traversal: Option<TraversalMode>,
    #[arg(long)]
    max_pages: Option<usize>,
    #[arg(long = "format", value_parser = parse_content_format)]
    content_format: Option<ContentFormat>,
    #[arg(long)]
    user_agent: Option<String>,
}

impl TryFrom<RawCli> for Cli {
    type Error = clap::Error;

    fn try_from(value: RawCli) -> Result<Self, Self::Error> {
        let command = match value.command {
            RawCommand::Create(arguments) => Command::Create(arguments.try_into()?),
            RawCommand::Update(arguments) => Command::Update(arguments.try_into()?),
        };

        Ok(Self { command })
    }
}

impl TryFrom<RawCreateArguments> for CreateArguments {
    type Error = clap::Error;

    fn try_from(value: RawCreateArguments) -> Result<Self, Self::Error> {
        let allowed_subdomains = authorized_hosts(value.allowed_subdomains);
        let configuration = DiscoveryConfiguration::new(
            value.scope,
            Some(value.site_boundary),
            allowed_subdomains.clone(),
            value.traversal,
            value.max_pages,
            value.content_format,
        )
        .map_err(configuration_error)?;

        Ok(Self {
            source_url: value.source_url,
            skill_name: value.skill_name,
            scope: configuration.scope(),
            site_boundary: configuration.site_boundary(),
            allowed_subdomains,
            traversal: configuration.traversal_mode(),
            max_pages: configuration.max_pages(),
            content_format: configuration.content_format(),
            user_agent: value.user_agent,
        })
    }
}

impl TryFrom<RawUpdateArguments> for UpdateArguments {
    type Error = clap::Error;

    fn try_from(value: RawUpdateArguments) -> Result<Self, Self::Error> {
        if value
            .scope
            .is_some_and(|scope| scope != DiscoveryScope::SameSite)
            && (value.site_boundary.is_some() || !value.allowed_subdomains.is_empty())
        {
            return Err(configuration_error(
                DiscoveryConfigurationError::SiteBoundaryRequiresSameSiteScope,
            ));
        }

        if !value.allowed_subdomains.is_empty()
            && value
                .site_boundary
                .is_some_and(|boundary| boundary != SiteBoundary::BaseDomain)
        {
            return Err(configuration_error(
                DiscoveryConfigurationError::AuthorizedSubdomainsRequireBaseDomain,
            ));
        }

        if value.max_pages == Some(0) {
            return Err(configuration_error(
                DiscoveryConfigurationError::MaximumPagesMustBePositive,
            ));
        }

        if value.max_pages.is_some()
            && value
                .traversal
                .is_some_and(|traversal| traversal != TraversalMode::Limited)
        {
            return Err(configuration_error(
                DiscoveryConfigurationError::MaximumPagesRequiresLimitedTraversal,
            ));
        }

        let max_pages = if value.traversal == Some(TraversalMode::Limited) {
            Some(
                value
                    .max_pages
                    .unwrap_or(DiscoveryConfiguration::DEFAULT_MAX_PAGES),
            )
        } else {
            value.max_pages
        };
        let arguments = Self {
            skill_names: value.skill_names,
            name: value.name,
            source_url: value.source_url,
            scope: value.scope,
            site_boundary: value.site_boundary,
            allowed_subdomains: authorized_hosts(value.allowed_subdomains),
            traversal: value.traversal,
            max_pages,
            content_format: value.content_format,
            user_agent: value.user_agent,
        };

        if arguments.has_changes() && arguments.skill_names.len() != 1 {
            return Err(invalid_argument(
                "configuration changes require exactly one selected skill",
            ));
        }

        Ok(arguments)
    }
}

fn parse_skill_name(value: &str) -> Result<SkillName, String> {
    SkillName::parse(value).map_err(|error| error.to_string())
}

fn parse_source_url(value: &str) -> Result<SourceUrl, String> {
    SourceUrl::parse(value).map_err(|error| error.to_string())
}

fn parse_scope(value: &str) -> Result<DiscoveryScope, String> {
    ScopeValue::from_str(value, true)
        .map(ScopeValue::into_domain)
        .map_err(|error| error.to_string())
}

fn parse_site_boundary(value: &str) -> Result<SiteBoundary, String> {
    SiteBoundaryValue::from_str(value, true)
        .map(SiteBoundaryValue::into_domain)
        .map_err(|error| error.to_string())
}

fn parse_traversal(value: &str) -> Result<TraversalMode, String> {
    TraversalValue::from_str(value, true)
        .map(TraversalValue::into_domain)
        .map_err(|error| error.to_string())
}

fn parse_content_format(value: &str) -> Result<ContentFormat, String> {
    ContentFormatValue::from_str(value, true)
        .map(ContentFormatValue::into_domain)
        .map_err(|error| error.to_string())
}

fn authorized_hosts(hosts: Vec<String>) -> Vec<AuthorizedHost> {
    hosts.into_iter().map(AuthorizedHost::new).collect()
}

fn configuration_error(error: DiscoveryConfigurationError) -> clap::Error {
    invalid_argument(error.to_string())
}

fn invalid_argument(message: impl Into<String>) -> clap::Error {
    clap::Error::raw(ErrorKind::InvalidValue, message.into())
}

#[derive(Clone, Copy, ValueEnum)]
enum ScopeValue {
    SameSite,
    PathPrefix,
    ParentDirectory,
    DocumentationNavigation,
}

impl ScopeValue {
    fn into_domain(self) -> DiscoveryScope {
        match self {
            Self::SameSite => DiscoveryScope::SameSite,
            Self::PathPrefix => DiscoveryScope::PathPrefix,
            Self::ParentDirectory => DiscoveryScope::ParentDirectory,
            Self::DocumentationNavigation => DiscoveryScope::DocumentationNavigation,
        }
    }
}

#[derive(Clone, Copy, ValueEnum)]
enum SiteBoundaryValue {
    ExactHost,
    BaseDomain,
    SameOrigin,
}

impl SiteBoundaryValue {
    fn into_domain(self) -> SiteBoundary {
        match self {
            Self::ExactHost => SiteBoundary::ExactHost,
            Self::BaseDomain => SiteBoundary::BaseDomain,
            Self::SameOrigin => SiteBoundary::SameOrigin,
        }
    }
}

#[derive(Clone, Copy, ValueEnum)]
enum TraversalValue {
    All,
    OneLevel,
    Limited,
}

impl TraversalValue {
    fn into_domain(self) -> TraversalMode {
        match self {
            Self::All => TraversalMode::All,
            Self::OneLevel => TraversalMode::OneLevel,
            Self::Limited => TraversalMode::Limited,
        }
    }
}

#[derive(Clone, Copy, ValueEnum)]
enum ContentFormatValue {
    GuideWithReferences,
    OrganizedContent,
}

impl ContentFormatValue {
    fn into_domain(self) -> ContentFormat {
        match self {
            Self::GuideWithReferences => ContentFormat::GuideWithReferences,
            Self::OrganizedContent => ContentFormat::OrganizedContent,
        }
    }
}
