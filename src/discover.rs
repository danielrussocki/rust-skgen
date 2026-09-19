//! Related documentation discovery boundary.

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
