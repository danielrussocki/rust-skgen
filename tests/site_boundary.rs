use rust_skgen::discover::is_within_site_boundary;
use rust_skgen::domain::{AuthorizedHost, SiteBoundary};
use url::Url;

fn parse_url(value: &str) -> Url {
    Url::parse(value).unwrap_or_else(|error| panic!("invalid test URL: {error}"))
}

#[test]
fn exact_host_requires_the_same_host_and_port_but_allows_scheme_changes() {
    let source = parse_url("https://docs.example.com:8443/guide");
    let same_host_and_port = parse_url("http://docs.example.com:8443/api");
    let different_port = parse_url("https://docs.example.com:443/api");
    let different_host = parse_url("https://api.example.com:8443/api");

    assert!(is_within_site_boundary(
        &source,
        &same_host_and_port,
        SiteBoundary::ExactHost,
        &[],
        true
    ));
    assert!(!is_within_site_boundary(
        &source,
        &different_port,
        SiteBoundary::ExactHost,
        &[],
        true
    ));
    assert!(!is_within_site_boundary(
        &source,
        &different_host,
        SiteBoundary::ExactHost,
        &[],
        true
    ));
}

#[test]
fn same_origin_requires_the_same_scheme_host_and_port() {
    let source = parse_url("https://docs.example.com:8443/guide");
    let same_origin = parse_url("https://docs.example.com:8443/api");
    let different_scheme = parse_url("http://docs.example.com:8443/api");
    let different_port = parse_url("https://docs.example.com:443/api");

    assert!(is_within_site_boundary(
        &source,
        &same_origin,
        SiteBoundary::SameOrigin,
        &[],
        true
    ));
    assert!(!is_within_site_boundary(
        &source,
        &different_scheme,
        SiteBoundary::SameOrigin,
        &[],
        true
    ));
    assert!(!is_within_site_boundary(
        &source,
        &different_port,
        SiteBoundary::SameOrigin,
        &[],
        true
    ));
}

#[test]
fn base_domain_allows_an_authorized_and_linked_subdomain() {
    let source = parse_url("https://docs.example.co.uk/guide");
    let candidate = parse_url("https://api.example.co.uk/reference");
    let authorized_hosts = [AuthorizedHost::new("api.example.co.uk")];

    assert!(is_within_site_boundary(
        &source,
        &candidate,
        SiteBoundary::BaseDomain,
        &authorized_hosts,
        true
    ));
}

#[test]
fn base_domain_rejects_subdomains_that_are_not_authorized_or_linked() {
    let source = parse_url("https://docs.example.co.uk/guide");
    let candidate = parse_url("https://api.example.co.uk/reference");
    let authorized_hosts = [AuthorizedHost::new("api.example.co.uk")];

    assert!(!is_within_site_boundary(
        &source,
        &candidate,
        SiteBoundary::BaseDomain,
        &[],
        true
    ));
    assert!(!is_within_site_boundary(
        &source,
        &candidate,
        SiteBoundary::BaseDomain,
        &authorized_hosts,
        false
    ));
}

#[test]
fn base_domain_rejects_other_registered_domains_and_keeps_the_source_host() {
    let source = parse_url("https://docs.example.co.uk/guide");
    let same_host = parse_url("https://docs.example.co.uk/api");
    let unrelated_host = parse_url("https://api.example.com/reference");
    let authorized_hosts = [AuthorizedHost::new("api.example.com")];

    assert!(is_within_site_boundary(
        &source,
        &same_host,
        SiteBoundary::BaseDomain,
        &[],
        false
    ));
    assert!(!is_within_site_boundary(
        &source,
        &unrelated_host,
        SiteBoundary::BaseDomain,
        &authorized_hosts,
        true
    ));
}
