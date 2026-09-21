use rust_skgen::domain::{
    AuthorizedHost, ContentFormat, DiscoveryConfiguration, DiscoveryConfigurationError,
    DiscoveryScope, SiteBoundary, TraversalMode,
};

#[test]
fn uses_the_specified_discovery_configuration_defaults() {
    let configuration = DiscoveryConfiguration::default();

    assert_eq!(configuration.scope(), DiscoveryScope::SameSite);
    assert_eq!(configuration.site_boundary(), SiteBoundary::ExactHost);
    assert_eq!(configuration.traversal_mode(), TraversalMode::All);
    assert_eq!(configuration.max_pages(), None);
    assert_eq!(
        configuration.content_format(),
        ContentFormat::GuideWithReferences
    );
    assert!(configuration.authorized_subdomains().is_empty());
}

#[test]
fn uses_one_hundred_pages_when_limited_traversal_has_no_maximum() {
    let configuration = DiscoveryConfiguration::new(
        DiscoveryScope::SameSite,
        Some(SiteBoundary::BaseDomain),
        vec![AuthorizedHost::new("api.docs.example.com")],
        TraversalMode::Limited,
        None,
        ContentFormat::OrganizedContent,
        false,
    );

    assert!(configuration.is_ok());
    let configuration = configuration.unwrap_or_else(|error| panic!("unexpected error: {error}"));
    assert_eq!(configuration.max_pages(), Some(100));
    assert_eq!(
        configuration.content_format(),
        ContentFormat::OrganizedContent
    );
}

#[test]
fn accepts_a_positive_maximum_for_limited_traversal() {
    let configuration = DiscoveryConfiguration::new(
        DiscoveryScope::SameSite,
        None,
        Vec::new(),
        TraversalMode::Limited,
        Some(25),
        ContentFormat::default(),
        false,
    );

    assert!(configuration.is_ok());
    let configuration = configuration.unwrap_or_else(|error| panic!("unexpected error: {error}"));
    assert_eq!(configuration.max_pages(), Some(25));
}

#[test]
fn rejects_zero_as_a_limited_traversal_maximum() {
    let configuration = DiscoveryConfiguration::new(
        DiscoveryScope::SameSite,
        None,
        Vec::new(),
        TraversalMode::Limited,
        Some(0),
        ContentFormat::default(),
        false,
    );

    assert_eq!(
        configuration,
        Err(DiscoveryConfigurationError::MaximumPagesMustBePositive)
    );
}

#[test]
fn rejects_a_maximum_for_non_limited_traversal() {
    let configuration = DiscoveryConfiguration::new(
        DiscoveryScope::SameSite,
        None,
        Vec::new(),
        TraversalMode::All,
        Some(25),
        ContentFormat::default(),
        false,
    );

    assert_eq!(
        configuration,
        Err(DiscoveryConfigurationError::MaximumPagesRequiresLimitedTraversal)
    );
}

#[test]
fn rejects_a_site_boundary_outside_same_site_scope() {
    let configuration = DiscoveryConfiguration::new(
        DiscoveryScope::PathPrefix,
        Some(SiteBoundary::ExactHost),
        Vec::new(),
        TraversalMode::default(),
        None,
        ContentFormat::default(),
        false,
    );

    assert_eq!(
        configuration,
        Err(DiscoveryConfigurationError::SiteBoundaryRequiresSameSiteScope)
    );
}

#[test]
fn rejects_authorized_subdomains_without_the_base_domain_boundary() {
    let configuration = DiscoveryConfiguration::new(
        DiscoveryScope::SameSite,
        None,
        vec![AuthorizedHost::new("api.docs.example.com")],
        TraversalMode::default(),
        None,
        ContentFormat::default(),
        false,
    );

    assert_eq!(
        configuration,
        Err(DiscoveryConfigurationError::AuthorizedSubdomainsRequireBaseDomain)
    );
}
