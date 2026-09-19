//! Related documentation discovery boundary.

use std::collections::BTreeSet;

use crate::{
    domain::{
        AuthorizedHost, DiscoveryConfiguration, DiscoveryScope, DocumentationPage, SiteBoundary,
        TraversalMode,
    },
    extract::{DocumentExtractionError, extract_document, extract_links},
    fetch::{DocumentFetcher, FetchError},
    policy::{CrawlPolicy, RobotsError},
};
use scraper::{Html, Selector};
use url::Url;

/// Marks a component that discovers documentation pages.
pub trait SiteDiscoverer {}

/// Error returned while discovering documentation pages.
#[derive(Debug)]
pub enum DiscoveryError {
    /// The crawl policy could not be evaluated.
    Policy(RobotsError),
    /// A source document could not be retrieved.
    Fetch(FetchError),
    /// The crawl policy prohibited a source document.
    Forbidden(Url),
    /// A source document responded with a redirect.
    Redirect(Url),
    /// A source document did not return a successful HTTP status.
    UnexpectedStatus { url: Url, status: u16 },
    /// A source document did not contain extractable documentation.
    Extraction(DocumentExtractionError),
}

impl std::fmt::Display for DiscoveryError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Policy(error) => write!(formatter, "failed to evaluate crawl policy: {error}"),
            Self::Fetch(error) => write!(formatter, "failed to retrieve documentation: {error}"),
            Self::Forbidden(url) => write!(formatter, "crawl policy forbids URL: {url}"),
            Self::Redirect(url) => write!(formatter, "URL redirected: {url}"),
            Self::UnexpectedStatus { url, status } => {
                write!(
                    formatter,
                    "URL returned unexpected HTTP status {status}: {url}"
                )
            }
            Self::Extraction(error) => {
                write!(formatter, "failed to extract documentation: {error}")
            }
        }
    }
}

impl std::error::Error for DiscoveryError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Policy(error) => Some(error),
            Self::Fetch(error) => Some(error),
            Self::Extraction(error) => Some(error),
            Self::Forbidden(_) | Self::Redirect(_) | Self::UnexpectedStatus { .. } => None,
        }
    }
}

/// Discovers in-scope documentation pages using deterministic URL order.
pub fn discover_all<F: DocumentFetcher, P: CrawlPolicy>(
    source_url: Url,
    configuration: &DiscoveryConfiguration,
    fetcher: &F,
    policy: &P,
) -> Result<Vec<DocumentationPage>, DiscoveryError> {
    let source_canonical = normalize_visit_url(&source_url);
    let source_document = fetch_document(source_url.clone(), fetcher, policy)?;
    let source_page = extract_document(source_url.clone(), source_document.body())
        .map_err(DiscoveryError::Extraction)?;
    let navigation_urls = navigation_urls(&source_url, source_document.body());
    let mut visited = BTreeSet::from([source_canonical]);
    let mut queue = source_links(&source_url, source_document.body(), configuration.scope());
    let mut pages = vec![source_page];

    while let Some(candidate) = queue.pop_first() {
        if maximum_reached(configuration, pages.len()) {
            break;
        }

        let candidate = normalize_visit_url(&candidate);
        if !visited.insert(candidate.clone())
            || !is_in_scope(&source_url, &candidate, configuration, &navigation_urls)
        {
            continue;
        }

        let document = fetch_document(candidate.clone(), fetcher, policy)?;
        let page = extract_document(candidate.clone(), document.body())
            .map_err(DiscoveryError::Extraction)?;
        pages.push(page);
        if configuration.traversal_mode() != TraversalMode::OneLevel
            && !maximum_reached(configuration, pages.len())
        {
            queue.extend(
                extract_links(&candidate, document.body())
                    .into_iter()
                    .map(|url| normalize_visit_url(&url)),
            );
        }
    }

    pages.sort_by_key(|page| normalize_visit_url(page.source_url()).to_string());
    Ok(pages)
}

fn maximum_reached(configuration: &DiscoveryConfiguration, page_count: usize) -> bool {
    configuration.traversal_mode() == TraversalMode::Limited
        && configuration
            .max_pages()
            .is_some_and(|maximum| page_count >= maximum)
}

fn fetch_document<F: DocumentFetcher, P: CrawlPolicy>(
    url: Url,
    fetcher: &F,
    policy: &P,
) -> Result<crate::fetch::FetchedDocument, DiscoveryError> {
    if !policy.allows(&url).map_err(DiscoveryError::Policy)? {
        return Err(DiscoveryError::Forbidden(url));
    }

    let document = fetcher.fetch(url.clone()).map_err(DiscoveryError::Fetch)?;
    if (300..400).contains(&document.status()) {
        return Err(DiscoveryError::Redirect(url));
    }
    if !(200..300).contains(&document.status()) {
        return Err(DiscoveryError::UnexpectedStatus {
            url,
            status: document.status(),
        });
    }

    Ok(document)
}

fn source_links(source_url: &Url, html: &str, scope: DiscoveryScope) -> BTreeSet<Url> {
    match scope {
        DiscoveryScope::DocumentationNavigation => navigation_urls(source_url, html),
        DiscoveryScope::SameSite | DiscoveryScope::PathPrefix | DiscoveryScope::ParentDirectory => {
            extract_links(source_url, html)
                .into_iter()
                .map(|url| normalize_visit_url(&url))
                .collect()
        }
    }
}

fn navigation_urls(source_url: &Url, html: &str) -> BTreeSet<Url> {
    navigation_links(html)
        .into_iter()
        .filter_map(|href| source_url.join(&href).ok())
        .filter(|url| matches!(url.scheme(), "http" | "https"))
        .map(|url| normalize_visit_url(&url))
        .collect()
}

fn is_in_scope(
    source_url: &Url,
    candidate: &Url,
    configuration: &DiscoveryConfiguration,
    navigation_urls: &BTreeSet<Url>,
) -> bool {
    match configuration.scope() {
        DiscoveryScope::SameSite => is_within_site_boundary(
            source_url,
            candidate,
            configuration.site_boundary(),
            configuration.authorized_subdomains(),
            true,
        ),
        DiscoveryScope::PathPrefix => is_within_path_prefix(source_url, candidate),
        DiscoveryScope::ParentDirectory => is_within_parent_directory(source_url, candidate),
        DiscoveryScope::DocumentationNavigation => navigation_urls.contains(candidate),
    }
}

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

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, collections::BTreeMap};

    use super::{DiscoveryError, discover_all};
    use crate::{
        domain::{ContentFormat, DiscoveryConfiguration, DiscoveryScope, TraversalMode},
        extract::DocumentExtractionError,
        fetch::{DocumentFetcher, FetchError, FetchedDocument},
        policy::{CrawlPolicy, RobotsError},
    };
    use url::Url;

    struct GraphFetcher {
        documents: BTreeMap<Url, String>,
        fetched_urls: RefCell<Vec<Url>>,
    }

    impl GraphFetcher {
        fn new(documents: BTreeMap<Url, String>) -> Self {
            Self {
                documents,
                fetched_urls: RefCell::new(Vec::new()),
            }
        }
    }

    impl DocumentFetcher for GraphFetcher {
        fn fetch(&self, url: Url) -> Result<FetchedDocument, FetchError> {
            self.fetched_urls.borrow_mut().push(url.clone());
            Ok(FetchedDocument::new(
                url.clone(),
                200,
                self.documents[&url].clone(),
            ))
        }
    }

    struct AllowAllPolicy;

    impl CrawlPolicy for AllowAllPolicy {
        fn allows(&self, _url: &Url) -> Result<bool, RobotsError> {
            Ok(true)
        }
    }

    struct DenyUrlPolicy {
        denied_url: Url,
    }

    impl CrawlPolicy for DenyUrlPolicy {
        fn allows(&self, url: &Url) -> Result<bool, RobotsError> {
            Ok(url != &self.denied_url)
        }
    }

    struct RedirectingFetcher {
        source_url: Url,
        redirect_url: Url,
        fetched_urls: RefCell<Vec<Url>>,
    }

    impl DocumentFetcher for RedirectingFetcher {
        fn fetch(&self, url: Url) -> Result<FetchedDocument, FetchError> {
            self.fetched_urls.borrow_mut().push(url.clone());
            let (status, body) = if url == self.source_url {
                (
                    200,
                    document("Start") + "<a href=\"/redirect\">Redirect</a>",
                )
            } else if url == self.redirect_url {
                (302, String::new())
            } else {
                panic!("unexpected URL requested by test fetcher: {url}");
            };

            Ok(FetchedDocument::new(url, status, body))
        }
    }

    fn url(path: &str) -> Url {
        Url::parse(&format!("https://docs.example.com{path}")).expect("test URL must be valid")
    }

    fn document(body: &str) -> String {
        format!("<main><p>{body}</p></main>")
    }

    fn configuration(
        traversal_mode: TraversalMode,
        max_pages: Option<usize>,
    ) -> DiscoveryConfiguration {
        DiscoveryConfiguration::new(
            DiscoveryScope::SameSite,
            None,
            Vec::new(),
            traversal_mode,
            max_pages,
            ContentFormat::GuideWithReferences,
        )
        .expect("test discovery configuration must be valid")
    }

    #[test]
    fn traverses_all_in_scope_pages_in_canonical_order_without_duplicates() {
        let source_url = url("/start");
        let mut documents = BTreeMap::new();
        documents.insert(
            source_url.clone(),
            format!(
                "{}<a href=\"/z\">Z</a><a href=\"/a?first\">A</a><a href=\"/a#section\">A duplicate</a><a href=\"https://other.example.com/outside\">Outside</a>",
                document("Start")
            ),
        );
        documents.insert(url("/a"), format!("{}<a href=\"/b\">B</a>", document("A")));
        documents.insert(
            url("/b"),
            format!("{}<a href=\"/z#related\">Z duplicate</a>", document("B")),
        );
        documents.insert(url("/z"), document("Z"));
        let fetcher = GraphFetcher::new(documents);

        let pages = discover_all(
            source_url.clone(),
            &DiscoveryConfiguration::default(),
            &fetcher,
            &AllowAllPolicy,
        )
        .expect("the local documentation graph should be discovered");

        assert_eq!(
            pages
                .iter()
                .map(|page| page.source_url().clone())
                .collect::<Vec<_>>(),
            vec![url("/a"), url("/b"), source_url, url("/z")]
        );
        assert_eq!(
            fetcher.fetched_urls.into_inner(),
            vec![url("/start"), url("/a"), url("/b"), url("/z")]
        );
    }

    #[test]
    fn one_level_traversal_does_not_enqueue_descendants() {
        let source_url = url("/start");
        let mut documents = BTreeMap::new();
        documents.insert(
            source_url.clone(),
            format!(
                "{}<a href=\"/a\">A</a><a href=\"/b\">B</a>",
                document("Start")
            ),
        );
        documents.insert(
            url("/a"),
            format!("{}<a href=\"/descendant\">Descendant</a>", document("A")),
        );
        documents.insert(url("/b"), document("B"));
        let fetcher = GraphFetcher::new(documents);

        let pages = discover_all(
            source_url.clone(),
            &configuration(TraversalMode::OneLevel, None),
            &fetcher,
            &AllowAllPolicy,
        )
        .expect("the direct documentation links should be discovered");

        assert_eq!(
            pages
                .iter()
                .map(|page| page.source_url().clone())
                .collect::<Vec<_>>(),
            vec![url("/a"), url("/b"), source_url]
        );
        assert_eq!(
            fetcher.fetched_urls.into_inner(),
            vec![url("/start"), url("/a"), url("/b")]
        );
    }

    #[test]
    fn limited_traversal_counts_the_source_page_and_stops_at_its_maximum() {
        let source_url = url("/start");
        let mut documents = BTreeMap::new();
        documents.insert(
            source_url.clone(),
            format!(
                "{}<a href=\"/a\">A</a><a href=\"/b\">B</a>",
                document("Start")
            ),
        );
        documents.insert(url("/a"), document("A"));
        documents.insert(url("/b"), document("B"));
        let fetcher = GraphFetcher::new(documents);

        let pages = discover_all(
            source_url.clone(),
            &configuration(TraversalMode::Limited, Some(2)),
            &fetcher,
            &AllowAllPolicy,
        )
        .expect("reaching the maximum should still return discovered pages");

        assert_eq!(
            pages
                .iter()
                .map(|page| page.source_url().clone())
                .collect::<Vec<_>>(),
            vec![url("/a"), source_url]
        );
        assert_eq!(
            fetcher.fetched_urls.into_inner(),
            vec![url("/start"), url("/a")]
        );
    }

    #[test]
    fn limited_traversal_uses_the_default_maximum_of_one_hundred_pages() {
        let source_url = url("/start");
        let mut documents = BTreeMap::new();
        let mut links = String::new();
        for index in 1..=100 {
            let path = format!("/page-{index:03}");
            links.push_str(&format!("<a href=\"{path}\">Page {index}</a>"));
            documents.insert(url(&path), document(&format!("Page {index}")));
        }
        documents.insert(
            source_url.clone(),
            format!("{}{}", document("Start"), links),
        );
        let fetcher = GraphFetcher::new(documents);

        let pages = discover_all(
            source_url.clone(),
            &configuration(TraversalMode::Limited, None),
            &fetcher,
            &AllowAllPolicy,
        )
        .expect("the default maximum should publish the discovered pages");

        assert_eq!(pages.len(), DiscoveryConfiguration::DEFAULT_MAX_PAGES);
        assert_eq!(
            pages.last().map(|page| page.source_url()),
            Some(&source_url)
        );
        assert_eq!(
            fetcher.fetched_urls.into_inner().len(),
            DiscoveryConfiguration::DEFAULT_MAX_PAGES
        );
    }

    #[test]
    fn redirect_response_aborts_discovery_without_returning_partial_pages() {
        let source_url = url("/start");
        let redirect_url = url("/redirect");
        let fetcher = RedirectingFetcher {
            source_url: source_url.clone(),
            redirect_url: redirect_url.clone(),
            fetched_urls: RefCell::new(Vec::new()),
        };

        let result = discover_all(
            source_url.clone(),
            &DiscoveryConfiguration::default(),
            &fetcher,
            &AllowAllPolicy,
        );

        assert!(matches!(result, Err(DiscoveryError::Redirect(url)) if url == redirect_url));
        assert_eq!(
            fetcher.fetched_urls.into_inner(),
            vec![source_url, redirect_url]
        );
    }

    #[test]
    fn forbidden_page_aborts_discovery_without_returning_partial_pages() {
        let source_url = url("/start");
        let forbidden_url = url("/forbidden");
        let mut documents = BTreeMap::new();
        documents.insert(
            source_url.clone(),
            format!("{}<a href=\"/forbidden\">Forbidden</a>", document("Start")),
        );
        documents.insert(forbidden_url.clone(), document("Forbidden"));
        let fetcher = GraphFetcher::new(documents);

        let result = discover_all(
            source_url.clone(),
            &DiscoveryConfiguration::default(),
            &fetcher,
            &DenyUrlPolicy {
                denied_url: forbidden_url.clone(),
            },
        );

        assert!(matches!(result, Err(DiscoveryError::Forbidden(url)) if url == forbidden_url));
        assert_eq!(fetcher.fetched_urls.into_inner(), vec![source_url]);
    }

    #[test]
    fn failed_page_extraction_aborts_discovery_without_returning_partial_pages() {
        let source_url = url("/start");
        let invalid_url = url("/invalid");
        let mut documents = BTreeMap::new();
        documents.insert(
            source_url.clone(),
            format!("{}<a href=\"/invalid\">Invalid</a>", document("Start")),
        );
        documents.insert(
            invalid_url.clone(),
            "<html><body>Invalid</body></html>".to_owned(),
        );
        let fetcher = GraphFetcher::new(documents);

        let result = discover_all(
            source_url.clone(),
            &DiscoveryConfiguration::default(),
            &fetcher,
            &AllowAllPolicy,
        );

        assert!(matches!(
            result,
            Err(DiscoveryError::Extraction(
                DocumentExtractionError::NoDocumentationContent
            ))
        ));
        assert_eq!(
            fetcher.fetched_urls.into_inner(),
            vec![source_url, invalid_url]
        );
    }

    #[test]
    fn source_without_documentation_aborts_discovery_without_any_valid_pages() {
        let source_url = url("/start");
        let mut documents = BTreeMap::new();
        documents.insert(
            source_url.clone(),
            "<html><body>Empty</body></html>".to_owned(),
        );
        let fetcher = GraphFetcher::new(documents);

        let result = discover_all(
            source_url.clone(),
            &DiscoveryConfiguration::default(),
            &fetcher,
            &AllowAllPolicy,
        );

        assert!(matches!(
            result,
            Err(DiscoveryError::Extraction(
                DocumentExtractionError::NoDocumentationContent
            ))
        ));
        assert_eq!(fetcher.fetched_urls.into_inner(), vec![source_url]);
    }
}
