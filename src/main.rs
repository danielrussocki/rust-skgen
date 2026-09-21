//! Executable bootstrap for the rust-skgen command-line interface.

use std::{
    io::{self, BufRead},
    time::Duration,
};

use clap::error::ErrorKind;
use rust_skgen::{
    cli::{Cli, Command, CommandOutput, write_command_output},
    domain::DiscoveryConfiguration,
    fetch::{FetchConfiguration, HttpDocumentFetcher},
    policy::RobotsTxtPolicy,
    service::{
        CreateSkillRequest, RebuildConfirmation, UpdateSkillChanges, create_skill, update_skills,
        update_skills_with_changes_and_confirmation,
    },
    storage::{SkillLocator, TransactionalSkillCreator},
};

fn main() {
    let cli = match Cli::try_parse_from(std::env::args_os()) {
        Ok(cli) => cli,
        Err(error) => {
            let exit_code = if error.kind() == ErrorKind::DisplayHelp {
                let _ = error.print();
                0
            } else {
                eprintln!("Invalid argument: {}", invalid_argument_reason(&error));
                2
            };
            std::process::exit(exit_code);
        }
    };

    let exit_code = match run(cli) {
        Ok(exit_code) => exit_code,
        Err(error) => {
            eprintln!("Failed to initialize application: {error}");
            1
        }
    };
    std::process::exit(i32::from(exit_code));
}

fn run(cli: Cli) -> io::Result<u8> {
    let working_directory = std::env::current_dir()?;
    let skills_root = working_directory.join(".agents").join("skills");
    let mut standard_output = io::stdout().lock();
    let mut standard_error = io::stderr().lock();

    match cli.command {
        Command::Create(arguments) => {
            let user_agent = arguments
                .user_agent
                .clone()
                .unwrap_or_else(|| "rust-skgen/0.1".to_owned());
            let fetcher = HttpDocumentFetcher::new(FetchConfiguration::new(
                user_agent.clone(),
                Duration::from_secs(30),
                2,
            ))
            .map_err(io::Error::other)?;
            let discovery = DiscoveryConfiguration::new(
                arguments.scope,
                Some(arguments.site_boundary),
                arguments.allowed_subdomains,
                arguments.traversal,
                arguments.max_pages,
                arguments.content_format,
            )
            .map_err(io::Error::other)?;
            let request = CreateSkillRequest::new(
                arguments.skill_name.clone(),
                arguments.source_url,
                discovery,
            );
            let publisher = TransactionalSkillCreator::new(skills_root);
            let policy = RobotsTxtPolicy::new(fetcher.clone(), user_agent);
            let output = match create_skill(request, &fetcher, &policy, &publisher) {
                Ok(result) => CommandOutput::from_created(result),
                Err(error) => CommandOutput::CreateFailed {
                    name: arguments.skill_name,
                    reason: error.to_string(),
                },
            };
            write_command_output(output, &mut standard_output, &mut standard_error)
        }
        Command::Update(arguments) => {
            let user_agent = arguments
                .user_agent
                .clone()
                .unwrap_or_else(|| "rust-skgen/0.1".to_owned());
            let fetcher = HttpDocumentFetcher::new(FetchConfiguration::new(
                user_agent.clone(),
                Duration::from_secs(30),
                2,
            ))
            .map_err(io::Error::other)?;
            let policy = RobotsTxtPolicy::new(fetcher.clone(), user_agent);
            let publisher = TransactionalSkillCreator::new(&skills_root);
            let result = if arguments.has_changes() {
                let changes = update_changes(&arguments);
                let confirmation = InteractiveRebuildConfirmation;
                update_skills_with_changes_and_confirmation(
                    &arguments.skill_names,
                    changes,
                    &confirmation,
                    &fetcher,
                    &policy,
                    &publisher,
                )
                .map_err(io::Error::other)?
            } else {
                update_skills(
                    (!arguments.skill_names.is_empty()).then_some(arguments.skill_names.as_slice()),
                    &SkillLocator::new(&skills_root),
                    &fetcher,
                    &policy,
                    &publisher,
                )
                .map_err(io::Error::other)?
            };
            let output = if result.outcomes().is_empty() {
                CommandOutput::NoManagedSkills
            } else {
                CommandOutput::from_updated(result)
            };
            write_command_output(output, &mut standard_output, &mut standard_error)
        }
    }
}

fn invalid_argument_reason(error: &clap::Error) -> String {
    let message = error.to_string();
    message
        .split_once("error: ")
        .map_or(message.as_str(), |(_, reason)| reason)
        .lines()
        .next()
        .unwrap_or(message.trim())
        .trim()
        .to_owned()
}

struct InteractiveRebuildConfirmation;

impl RebuildConfirmation for InteractiveRebuildConfirmation {
    fn confirm_rebuild(&self) -> Option<bool> {
        let mut response = String::new();
        eprint!("Rebuild metadata and update this skill? [y/N] ");
        io::stdin()
            .lock()
            .read_line(&mut response)
            .ok()
            .filter(|read| *read > 0)
            .map(|_| response.trim().eq_ignore_ascii_case("y"))
    }
}

fn update_changes(arguments: &rust_skgen::cli::UpdateArguments) -> UpdateSkillChanges {
    let mut changes = UpdateSkillChanges::default();
    if let Some(name) = arguments.name.clone() {
        changes = changes.with_name(name);
    }
    if let Some(source_url) = arguments.source_url.clone() {
        changes = changes.with_source_url(source_url);
    }
    if let Some(scope) = arguments.scope {
        changes = changes.with_scope(scope);
    }
    if let Some(site_boundary) = arguments.site_boundary {
        changes = changes.with_site_boundary(site_boundary);
    }
    if !arguments.allowed_subdomains.is_empty() {
        changes = changes.with_authorized_subdomains(arguments.allowed_subdomains.clone());
    }
    if let Some(traversal) = arguments.traversal {
        changes = changes.with_traversal_mode(traversal);
    }
    if let Some(max_pages) = arguments.max_pages {
        changes = changes.with_max_pages(max_pages);
    }
    if let Some(content_format) = arguments.content_format {
        changes = changes.with_content_format(content_format);
    }
    changes
}
