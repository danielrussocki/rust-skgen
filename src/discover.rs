//! Related documentation discovery boundary.

use crate::domain::{AuthorizedHost, SiteBoundary};
use url::Url;

/// Marks a component that discovers documentation pages.
pub trait SiteDiscoverer {}

/// Returns the URL identity used to avoid revisiting the same document.
pub fn normalize_visit_url(url: &Url) -> Url {
    let mut normalized = url.clone();
    normalized.set_query(None);
    normalized.set_fragment(None);
    normalized
}

/// Returns whether a candidate URL belongs to the configured site boundary.
pub fn is_within_site_boundary(
    source: &Url,
    candidate: &Url,
    boundary: SiteBoundary,
    authorized_subdomains: &[AuthorizedHost],
    linked_from_included_page: bool,
) -> bool {
    match boundary {
        SiteBoundary::ExactHost => has_same_host_and_port(source, candidate),
        SiteBoundary::SameOrigin => {
            source.scheme() == candidate.scheme() && has_same_host_and_port(source, candidate)
        }
        SiteBoundary::BaseDomain => {
            has_same_host(source, candidate)
                || (linked_from_included_page
                    && has_same_registrable_domain(source, candidate)
                    && is_subdomain_of_registrable_domain(candidate)
                    && is_authorized_host(candidate, authorized_subdomains))
        }
    }
}

fn has_same_host_and_port(source: &Url, candidate: &Url) -> bool {
    has_same_host(source, candidate)
        && source.port_or_known_default() == candidate.port_or_known_default()
}

fn has_same_host(source: &Url, candidate: &Url) -> bool {
    match (source.host_str(), candidate.host_str()) {
        (Some(source_host), Some(candidate_host)) => {
            source_host.eq_ignore_ascii_case(candidate_host)
        }
        _ => false,
    }
}

fn has_same_registrable_domain(source: &Url, candidate: &Url) -> bool {
    match (source.host_str(), candidate.host_str()) {
        (Some(source_host), Some(candidate_host)) => {
            psl::domain_str(source_host) == psl::domain_str(candidate_host)
        }
        _ => false,
    }
}

fn is_subdomain_of_registrable_domain(candidate: &Url) -> bool {
    let Some(candidate_host) = candidate.host_str() else {
        return false;
    };
    let Some(registrable_domain) = psl::domain_str(candidate_host) else {
        return false;
    };

    candidate_host
        .strip_suffix(registrable_domain)
        .is_some_and(|prefix| prefix.ends_with('.'))
}

fn is_authorized_host(candidate: &Url, authorized_subdomains: &[AuthorizedHost]) -> bool {
    let Some(candidate_host) = candidate.host_str() else {
        return false;
    };

    authorized_subdomains
        .iter()
        .any(|host| host.as_str().eq_ignore_ascii_case(candidate_host))
}
