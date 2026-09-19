use rust_skgen::{
    cli::{Cli, Command},
    domain::{ContentFormat, DiscoveryScope, SiteBoundary, TraversalMode},
};

#[test]
fn create_requires_a_source_url_and_skill_name() {
    assert!(Cli::try_parse_from(["rust-skgen", "create"]).is_err());
    assert!(Cli::try_parse_from(["rust-skgen", "create", "https://docs.example.test"]).is_err());
}

#[test]
fn create_uses_the_specified_defaults() {
    let cli = Cli::try_parse_from([
        "rust-skgen",
        "create",
        "https://docs.example.test/start",
        "example-skill",
    ])
    .unwrap();

    let Command::Create(arguments) = cli.command else {
        panic!("expected create command");
    };
    assert_eq!(
        arguments.source_url.as_url().as_str(),
        "https://docs.example.test/start"
    );
    assert_eq!(arguments.skill_name.as_str(), "example-skill");
    assert_eq!(arguments.scope, DiscoveryScope::SameSite);
    assert_eq!(arguments.site_boundary, SiteBoundary::ExactHost);
    assert!(arguments.allowed_subdomains.is_empty());
    assert_eq!(arguments.traversal, TraversalMode::All);
    assert_eq!(arguments.max_pages, None);
    assert_eq!(arguments.content_format, ContentFormat::GuideWithReferences);
    assert_eq!(arguments.user_agent, None);
}

#[test]
fn create_accepts_each_configurable_option() {
    let cli = Cli::try_parse_from([
        "rust-skgen",
        "create",
        "https://docs.example.test/start",
        "example-skill",
        "--scope",
        "same-site",
        "--site-boundary",
        "base-domain",
        "--allow-subdomain",
        "api.docs.example.test",
        "--allow-subdomain",
        "reference.docs.example.test",
        "--traversal",
        "limited",
        "--max-pages",
        "25",
        "--format",
        "organized-content",
        "--user-agent",
        "rust-skgen-test",
    ])
    .unwrap();

    let Command::Create(arguments) = cli.command else {
        panic!("expected create command");
    };
    assert_eq!(arguments.scope, DiscoveryScope::SameSite);
    assert_eq!(arguments.site_boundary, SiteBoundary::BaseDomain);
    assert_eq!(arguments.allowed_subdomains.len(), 2);
    assert_eq!(arguments.traversal, TraversalMode::Limited);
    assert_eq!(arguments.max_pages, Some(25));
    assert_eq!(arguments.content_format, ContentFormat::OrganizedContent);
    assert_eq!(arguments.user_agent.as_deref(), Some("rust-skgen-test"));
}

#[test]
fn create_rejects_invalid_configuration_combinations() {
    for arguments in [
        vec![
            "create",
            "https://docs.example.test/start",
            "example-skill",
            "--scope",
            "path-prefix",
            "--site-boundary",
            "exact-host",
        ],
        vec![
            "create",
            "https://docs.example.test/start",
            "example-skill",
            "--allow-subdomain",
            "api.docs.example.test",
        ],
        vec![
            "create",
            "https://docs.example.test/start",
            "example-skill",
            "--traversal",
            "all",
            "--max-pages",
            "5",
        ],
        vec![
            "create",
            "https://docs.example.test/start",
            "example-skill",
            "--traversal",
            "limited",
            "--max-pages",
            "0",
        ],
        vec!["create", "ftp://docs.example.test/start", "example-skill"],
        vec!["create", "https://docs.example.test/start", "Invalid-Skill"],
    ] {
        let mut command = vec!["rust-skgen"];
        command.extend(arguments);
        assert!(Cli::try_parse_from(command).is_err());
    }
}

#[test]
fn update_accepts_an_unmodified_bulk_selection_and_a_single_configured_selection() {
    let bulk =
        Cli::try_parse_from(["rust-skgen", "update", "first-skill", "second-skill"]).unwrap();
    let Command::Update(bulk_arguments) = bulk.command else {
        panic!("expected update command");
    };
    assert_eq!(bulk_arguments.skill_names.len(), 2);
    assert!(!bulk_arguments.has_changes());

    let single = Cli::try_parse_from([
        "rust-skgen",
        "update",
        "existing-skill",
        "--name",
        "renamed-skill",
        "--source-url",
        "https://docs.example.test/revised",
        "--scope",
        "same-site",
        "--site-boundary",
        "base-domain",
        "--allow-subdomain",
        "api.docs.example.test",
        "--traversal",
        "limited",
        "--max-pages",
        "10",
        "--format",
        "organized-content",
        "--user-agent",
        "rust-skgen-test",
    ])
    .unwrap();
    let Command::Update(arguments) = single.command else {
        panic!("expected update command");
    };
    assert_eq!(arguments.skill_names.len(), 1);
    assert!(arguments.has_changes());
    assert_eq!(arguments.name.unwrap().as_str(), "renamed-skill");
    assert_eq!(
        arguments.source_url.unwrap().as_url().as_str(),
        "https://docs.example.test/revised"
    );
    assert_eq!(arguments.scope, Some(DiscoveryScope::SameSite));
    assert_eq!(arguments.site_boundary, Some(SiteBoundary::BaseDomain));
    assert_eq!(arguments.allowed_subdomains.len(), 1);
    assert_eq!(arguments.traversal, Some(TraversalMode::Limited));
    assert_eq!(arguments.max_pages, Some(10));
    assert_eq!(
        arguments.content_format,
        Some(ContentFormat::OrganizedContent)
    );
    assert_eq!(arguments.user_agent.as_deref(), Some("rust-skgen-test"));
}

#[test]
fn update_rejects_changes_without_exactly_one_selected_skill() {
    for arguments in [
        vec!["update", "--format", "organized-content"],
        vec![
            "update",
            "first-skill",
            "second-skill",
            "--format",
            "organized-content",
        ],
        vec![
            "update",
            "existing-skill",
            "--scope",
            "path-prefix",
            "--allow-subdomain",
            "api.docs.example.test",
        ],
    ] {
        let mut command = vec!["rust-skgen"];
        command.extend(arguments);
        assert!(Cli::try_parse_from(command).is_err());
    }
}
