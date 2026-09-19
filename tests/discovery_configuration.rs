use rust_skgen::domain::{
    ContentFormat, DiscoveryScope, SiteBoundary, SourceUrl, SourceUrlError, TraversalMode,
};

#[test]
fn accepts_http_and_https_source_urls() {
    for value in [
        "http://docs.example.com/guide",
        "https://docs.example.com/guide",
    ] {
        let source_url = SourceUrl::parse(value);

        assert!(source_url.is_ok(), "expected accepted source URL: {value}");
    }
}

#[test]
fn rejects_malformed_source_urls() {
    assert!(matches!(
        SourceUrl::parse("not a URL"),
        Err(SourceUrlError::InvalidUrl(_))
    ));
}

#[test]
fn rejects_source_urls_with_unsupported_schemes() {
    assert!(matches!(
        SourceUrl::parse("ftp://docs.example.com/guide"),
        Err(SourceUrlError::UnsupportedScheme { .. })
    ));
}

#[test]
fn uses_the_specified_configuration_defaults() {
    assert_eq!(DiscoveryScope::default(), DiscoveryScope::SameSite);
    assert_eq!(SiteBoundary::default(), SiteBoundary::ExactHost);
    assert_eq!(TraversalMode::default(), TraversalMode::All);
    assert_eq!(ContentFormat::default(), ContentFormat::GuideWithReferences);
}

#[test]
fn exposes_every_supported_discovery_value() {
    let scopes = [
        DiscoveryScope::SameSite,
        DiscoveryScope::PathPrefix,
        DiscoveryScope::ParentDirectory,
        DiscoveryScope::DocumentationNavigation,
    ];
    let boundaries = [
        SiteBoundary::ExactHost,
        SiteBoundary::BaseDomain,
        SiteBoundary::SameOrigin,
    ];
    let traversal_modes = [
        TraversalMode::All,
        TraversalMode::OneLevel,
        TraversalMode::Limited,
    ];
    let content_formats = [
        ContentFormat::GuideWithReferences,
        ContentFormat::OrganizedContent,
    ];

    assert_eq!(scopes.len(), 4);
    assert_eq!(boundaries.len(), 3);
    assert_eq!(traversal_modes.len(), 3);
    assert_eq!(content_formats.len(), 2);
}
