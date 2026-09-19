//! Related documentation discovery boundary.

use crate::domain::{AuthorizedHost, SiteBoundary};
use scraper::{Html, Selector};
use url::Url;

/// Marks a component that discovers documentation pages.
pub trait SiteDiscoverer {}

/// Returns link targets found within HTML navigation elements.
pub fn navigation_links(html: &str) -> Vec<String> {
    let document = Html::parse_document(html);
    let Ok(selector) = Selector::parse("nav a[href]") else {
        return Vec::new();
    };

    document
        .select(&selector)
        .filter_map(|element| element.value().attr("href").map(str::to_owned))
        .collect()
}

/// Returns the URL identity used to avoid revisiting the same document.
pub fn normalize_visit_url(url: &Url) -> Url {
    let mut normalized = url.clone();
    normalized.set_query(None);
    normalized.set_fragment(None);
    normalized
}

/// Returns whether a candidate URL is the source path or a descendant of it.
pub fn is_within_path_prefix(source: &Url, candidate: &Url) -> bool {
    path_is_within_directory(candidate.path(), source.path())
}

/// Returns whether a candidate URL belongs to the directory containing the source URL.
pub fn is_within_parent_directory(source: &Url, candidate: &Url) -> bool {
    path_is_within_directory(candidate.path(), parent_directory(source.path()))
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

fn path_is_within_directory(candidate_path: &str, directory_path: &str) -> bool {
    if directory_path == "/" {
        return candidate_path.starts_with('/');
    }

    let directory_path = directory_path.trim_end_matches('/');
    candidate_path == directory_path
        || candidate_path
            .strip_prefix(directory_path)
            .is_some_and(|suffix| suffix.starts_with('/'))
}

fn parent_directory(path: &str) -> &str {
    let path = path.trim_end_matches('/');
    let Some(index) = path.rfind('/') else {
        return "/";
    };

    &path[..=index]
}
