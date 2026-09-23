//! Related documentation discovery boundary.

use std::collections::BTreeSet;

use crate::{
    domain::{
        AuthorizedHost, DiscoveryConfiguration, DiscoveryScope, DocumentationSource, LinkCandidate,
        SiteBoundary, TraversalMode,
    },
    extract::{DocumentExtractionError, extract_document, extract_link_candidates},
    fetch::{DocumentFetcher, FetchError},
    policy::{CrawlPolicy, RobotsError},
    sitemap::find_sitemap_urls,
};
use scraper::{Html, Selector};
use url::Url;

/// Marks a component that discovers documentation pages.
pub trait SiteDiscoverer {}

const DOCUMENTATION_SIGNALS: [&str; 5] = ["docs", "guide", "api", "reference", "components"];
const PENALIZED_SIGNALS: [&str; 7] = [
    "blog",
    "changelog",
    "releases",
    "supported-browsers",
    "privacy",
    "terms",
    "careers",
];
const IMAGE_EXTENSIONS: [&str; 7] = [".avif", ".gif", ".jpeg", ".jpg", ".png", ".svg", ".webp"];
const STATIC_EXTENSIONS: [&str; 7] = [".css", ".js", ".map", ".pdf", ".woff", ".woff2", ".zip"];
const SOCIAL_HOSTS: [&str; 6] = [
    "facebook.com",
    "instagram.com",
    "linkedin.com",
    "twitter.com",
    "x.com",
    "youtube.com",
];

/// Returns the deterministic score used to prioritize an admissible link candidate.
pub fn candidate_priority(candidate: &LinkCandidate) -> i32 {
    let text_score = [
        candidate.url().path(),
        candidate.anchor_text(),
        candidate.navigation_text(),
    ]
    .into_iter()
    .map(|text| {
        let lowercase = text.to_ascii_lowercase();
        DOCUMENTATION_SIGNALS
            .iter()
            .filter(|signal| lowercase.contains(**signal))
            .count() as i32
            - PENALIZED_SIGNALS
                .iter()
                .filter(|signal| lowercase.contains(**signal))
                .count() as i32
    })
    .sum::<i32>();
    let path = candidate.url().path().to_ascii_lowercase();
    let resource_penalty = IMAGE_EXTENSIONS
        .iter()
        .chain(STATIC_EXTENSIONS.iter())
        .any(|extension| path.ends_with(extension)) as i32;
    let social_penalty = candidate.url().host_str().is_some_and(|host| {
        let host = host.to_ascii_lowercase();
        SOCIAL_HOSTS
            .iter()
            .any(|social_host| host == *social_host || host.ends_with(&format!(".{social_host}")))
    }) as i32;

    text_score - resource_penalty - social_penalty
}

enum FetchOutcome {
    Document(crate::fetch::FetchedDocument),
    Unavailable,
    Skipped,
}

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
    /// A source document did not return HTML suitable for extraction.
    NonHtmlContent { url: Url, content_type: String },
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
            Self::NonHtmlContent { url, content_type } => {
                write!(
                    formatter,
                    "URL returned non-HTML content type {content_type}: {url}"
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
            Self::Forbidden(_)
            | Self::Redirect(_)
            | Self::UnexpectedStatus { .. }
            | Self::NonHtmlContent { .. } => None,
        }
    }
}

/// Discovers in-scope documentation pages using deterministic URL order.
pub fn discover_all<F: DocumentFetcher, P: CrawlPolicy>(
    source_url: Url,
    configuration: &DiscoveryConfiguration,
    fetcher: &F,
    policy: &P,
) -> Result<Vec<DocumentationSource>, DiscoveryError> {
    let source_canonical = normalize_visit_url(&source_url);
    let FetchOutcome::Document(source_document) = fetch_document(
        source_url.clone(),
        true,
        false,
        configuration,
        fetcher,
        policy,
    )?
    else {
        return Err(DiscoveryError::UnexpectedStatus {
            url: source_url.clone(),
            status: 404,
        });
    };
    let source_page = extract_document(source_url.clone(), source_document.body())
        .map_err(DiscoveryError::Extraction)?;
    let navigation_urls = navigation_urls(&source_url, source_document.body());
    let mut visited = BTreeSet::from([source_canonical]);
    let mut sitemap_queue = find_sitemap_urls(&source_url, fetcher)
        .into_iter()
        .map(|url| LinkCandidate::new(url, String::new(), false))
        .collect::<Vec<_>>();
    let mut link_queue = source_links(&source_url, source_document.body(), configuration.scope());
    let mut pages = vec![DocumentationSource::Extracted(source_page)];

    while let Some((candidate, from_sitemap)) = next_candidate(&mut sitemap_queue)
        .map(|candidate| (candidate, true))
        .or_else(|| next_candidate(&mut link_queue).map(|candidate| (candidate, false)))
    {
        if maximum_reached(configuration, pages.len()) {
            break;
        }

        let candidate = candidate.url().clone();
        if !visited.insert(candidate.clone())
            || !is_in_scope(&source_url, &candidate, configuration, &navigation_urls)
        {
            continue;
        }

        let document = match fetch_document(
            candidate.clone(),
            false,
            from_sitemap,
            configuration,
            fetcher,
            policy,
        )? {
            FetchOutcome::Document(document) => document,
            FetchOutcome::Unavailable => {
                pages.push(DocumentationSource::unavailable(candidate));
                continue;
            }
            FetchOutcome::Skipped => continue,
        };
        let page = extract_document(candidate.clone(), document.body())
            .map_err(DiscoveryError::Extraction)?;
        pages.push(DocumentationSource::Extracted(page));
        if configuration.traversal_mode() != TraversalMode::OneLevel
            && !maximum_reached(configuration, pages.len())
        {
            link_queue.extend(extract_link_candidates(&candidate, document.body()));
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
    is_source: bool,
    skip_if_forbidden: bool,
    configuration: &DiscoveryConfiguration,
    fetcher: &F,
    policy: &P,
) -> Result<FetchOutcome, DiscoveryError> {
    if !policy
        .allows_with_robots_requirement(&url, configuration.requires_robots_txt())
        .map_err(DiscoveryError::Policy)?
    {
        if skip_if_forbidden {
            return Ok(FetchOutcome::Skipped);
        }
        return Err(DiscoveryError::Forbidden(url));
    }

    let document = fetcher.fetch(url.clone()).map_err(DiscoveryError::Fetch)?;
    if (300..400).contains(&document.status()) && !is_source {
        return Ok(FetchOutcome::Skipped);
    }
    if (300..400).contains(&document.status()) {
        return Err(DiscoveryError::Redirect(url));
    }
    if document.status() == 404 && !is_source {
        return Ok(FetchOutcome::Unavailable);
    }
    if !(200..300).contains(&document.status()) {
        return Err(DiscoveryError::UnexpectedStatus {
            url,
            status: document.status(),
        });
    }
    if !document.is_html() {
        if !is_source {
            return Ok(FetchOutcome::Skipped);
        }
        return Err(DiscoveryError::NonHtmlContent {
            url,
            content_type: document.content_type().unwrap_or_default().to_owned(),
        });
    }

    Ok(FetchOutcome::Document(document))
}

fn next_candidate(queue: &mut Vec<LinkCandidate>) -> Option<LinkCandidate> {
    queue.sort_by(|left, right| {
        candidate_priority(right)
            .cmp(&candidate_priority(left))
            .then_with(|| left.url().cmp(right.url()))
    });
    (!queue.is_empty()).then(|| queue.remove(0))
}

fn source_links(source_url: &Url, html: &str, scope: DiscoveryScope) -> Vec<LinkCandidate> {
    match scope {
        DiscoveryScope::DocumentationNavigation => extract_link_candidates(source_url, html)
            .into_iter()
            .filter(|candidate| candidate.is_navigation())
            .collect(),
        DiscoveryScope::SameSite | DiscoveryScope::PathPrefix | DiscoveryScope::ParentDirectory => {
            extract_link_candidates(source_url, html)
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
    has_same_origin(source, candidate) && path_is_within_directory(candidate.path(), source.path())
}

/// Returns whether a candidate URL belongs to the directory containing the source URL.
pub fn is_within_parent_directory(source: &Url, candidate: &Url) -> bool {
    has_same_origin(source, candidate)
        && path_is_within_directory(candidate.path(), parent_directory(source.path()))
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
        SiteBoundary::SameOrigin => has_same_origin(source, candidate),
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

fn has_same_origin(source: &Url, candidate: &Url) -> bool {
    source.scheme() == candidate.scheme() && has_same_host_and_port(source, candidate)
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
        domain::{
            ContentFormat, DiscoveryConfiguration, DiscoveryScope, DocumentationPage,
            DocumentationSource, TraversalMode,
        },
        extract::DocumentExtractionError,
        fetch::{DocumentFetcher, FetchError, FetchedDocument},
        policy::{CrawlPolicy, RobotsError},
    };
    use url::Url;

    struct GraphFetcher {
        documents: BTreeMap<Url, String>,
        fetched_urls: RefCell<Vec<Url>>,
    }

    struct StatusFetcher {
        documents: BTreeMap<Url, FetchedDocument>,
        fetched_urls: RefCell<Vec<Url>>,
    }

    impl DocumentFetcher for StatusFetcher {
        fn fetch(&self, url: Url) -> Result<FetchedDocument, FetchError> {
            if !is_sitemap_location(&url) {
                self.fetched_urls.borrow_mut().push(url.clone());
            }
            Ok(self.documents.get(&url).cloned().unwrap_or_else(|| {
                FetchedDocument::new_with_content_type(
                    url,
                    404,
                    String::new(),
                    "text/plain".to_owned(),
                )
            }))
        }
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
            if !is_sitemap_location(&url) {
                self.fetched_urls.borrow_mut().push(url.clone());
            }
            Ok(self.documents.get(&url).map_or_else(
                || {
                    FetchedDocument::new_with_content_type(
                        url.clone(),
                        404,
                        String::new(),
                        "text/plain".to_owned(),
                    )
                },
                |body| FetchedDocument::new(url.clone(), 200, body.clone()),
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

    struct SourceRedirectingFetcher {
        fetched_urls: RefCell<Vec<Url>>,
    }

    impl DocumentFetcher for SourceRedirectingFetcher {
        fn fetch(&self, url: Url) -> Result<FetchedDocument, FetchError> {
            if !is_sitemap_location(&url) {
                self.fetched_urls.borrow_mut().push(url.clone());
            }
            Ok(FetchedDocument::new(url, 302, String::new()))
        }
    }

    fn url(path: &str) -> Url {
        Url::parse(&format!("https://docs.example.com{path}")).expect("test URL must be valid")
    }

    fn is_sitemap_location(url: &Url) -> bool {
        matches!(
            url.path(),
            "/sitemap.xml" | "/sitemap_index.xml" | "/sitemap-index.xml" | "/sitemap.php"
        )
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
            false,
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
    fn prioritizes_html_candidates_before_fetching_and_breaks_ties_by_canonical_url() {
        let source_url = url("/start");
        let mut documents = BTreeMap::new();
        documents.insert(
            source_url.clone(),
            format!(
                "{}<a href=\"/blog\">Blog</a><a href=\"/guide\">Guide</a><a href=\"/b\">B</a><a href=\"/a\">A</a>",
                document("Start")
            ),
        );
        for path in ["/a", "/b", "/blog", "/guide"] {
            documents.insert(url(path), document(path));
        }
        let fetcher = GraphFetcher::new(documents);

        discover_all(
            source_url.clone(),
            &DiscoveryConfiguration::default(),
            &fetcher,
            &AllowAllPolicy,
        )
        .expect("the local documentation graph should be discovered");

        assert_eq!(
            fetcher.fetched_urls.into_inner(),
            vec![
                source_url,
                url("/guide"),
                url("/a"),
                url("/b"),
                url("/blog"),
            ]
        );
    }

    #[test]
    fn limited_traversal_uses_priority_for_mixed_signals_without_relaxing_scope() {
        let source_url = url("/start");
        let guide_url = url("/guide");
        let mixed_url = url("/docs-blog");
        let outside_url =
            Url::parse("https://outside.example.com/docs").expect("test URL must be valid");
        let mut documents = BTreeMap::new();
        documents.insert(
            source_url.clone(),
            format!(
                "{}<a href=\"/blog\">Blog</a><a href=\"/docs-blog\">Docs blog</a><a href=\"/guide\">Guide</a><a href=\"{outside_url}\">Docs</a>",
                document("Start")
            ),
        );
        documents.insert(guide_url.clone(), document("Guide"));
        documents.insert(mixed_url.clone(), document("Mixed"));
        let fetcher = GraphFetcher::new(documents);

        let pages = discover_all(
            source_url.clone(),
            &configuration(TraversalMode::Limited, Some(3)),
            &fetcher,
            &AllowAllPolicy,
        )
        .expect("priority should determine the limited traversal order");

        assert_eq!(
            pages
                .iter()
                .map(|page| page.source_url().clone())
                .collect::<Vec<_>>(),
            vec![mixed_url.clone(), guide_url.clone(), source_url.clone()]
        );
        assert_eq!(
            fetcher.fetched_urls.into_inner(),
            vec![source_url, guide_url, mixed_url]
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
    fn related_redirect_is_skipped_while_remaining_html_pages_are_discovered() {
        let source_url = url("/start");
        let redirect_url = url("/redirect");
        let related_url = url("/related");
        let fetcher = StatusFetcher {
            documents: BTreeMap::from([
                (
                    source_url.clone(),
                    FetchedDocument::new(
                        source_url.clone(),
                        200,
                        format!(
                            "{}<a href=\"/redirect\">Redirect</a><a href=\"/related\">Related</a>",
                            document("Start")
                        ),
                    ),
                ),
                (
                    redirect_url.clone(),
                    FetchedDocument::new(redirect_url.clone(), 302, String::new()),
                ),
                (
                    related_url.clone(),
                    FetchedDocument::new(related_url.clone(), 200, document("Related")),
                ),
            ]),
            fetched_urls: RefCell::new(Vec::new()),
        };

        let pages = discover_all(
            source_url.clone(),
            &DiscoveryConfiguration::default(),
            &fetcher,
            &AllowAllPolicy,
        )
        .expect("a related redirect should not abort discovery");

        assert_eq!(
            pages
                .iter()
                .map(|page| page.source_url().clone())
                .collect::<Vec<_>>(),
            vec![related_url.clone(), source_url.clone()]
        );
        assert_eq!(
            fetcher.fetched_urls.into_inner(),
            vec![source_url, redirect_url, related_url]
        );
    }

    #[test]
    fn extractable_source_is_published_when_all_related_urls_redirect() {
        let source_url = url("/start");
        let first_redirect_url = url("/first-redirect");
        let second_redirect_url = url("/second-redirect");
        let fetcher = StatusFetcher {
            documents: BTreeMap::from([
                (
                    source_url.clone(),
                    FetchedDocument::new(
                        source_url.clone(),
                        200,
                        format!(
                            "{}<a href=\"/first-redirect\">First</a><a href=\"/second-redirect\">Second</a>",
                            document("Start")
                        ),
                    ),
                ),
                (
                    first_redirect_url.clone(),
                    FetchedDocument::new(first_redirect_url.clone(), 302, String::new()),
                ),
                (
                    second_redirect_url.clone(),
                    FetchedDocument::new(second_redirect_url.clone(), 301, String::new()),
                ),
            ]),
            fetched_urls: RefCell::new(Vec::new()),
        };

        let pages = discover_all(
            source_url.clone(),
            &DiscoveryConfiguration::default(),
            &fetcher,
            &AllowAllPolicy,
        )
        .expect("an extractable source should remain publishable after related redirects");

        assert_eq!(
            pages
                .iter()
                .map(|page| page.source_url().clone())
                .collect::<Vec<_>>(),
            vec![source_url.clone()]
        );
        assert_eq!(
            fetcher.fetched_urls.into_inner(),
            vec![source_url, first_redirect_url, second_redirect_url]
        );
    }

    #[test]
    fn related_redirects_without_any_documentation_page_abort_without_results() {
        let source_url = url("/start");
        let redirect_url = url("/redirect");
        let fetcher = StatusFetcher {
            documents: BTreeMap::from([
                (
                    source_url.clone(),
                    FetchedDocument::new(
                        source_url.clone(),
                        200,
                        "<main><a href=\"/redirect\">Redirect</a></main>".to_owned(),
                    ),
                ),
                (
                    redirect_url,
                    FetchedDocument::new(url("/redirect"), 302, String::new()),
                ),
            ]),
            fetched_urls: RefCell::new(Vec::new()),
        };

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

    #[test]
    fn source_redirect_aborts_discovery_before_related_pages_are_visited() {
        let source_url = url("/start");
        let fetcher = SourceRedirectingFetcher {
            fetched_urls: RefCell::new(Vec::new()),
        };

        let result = discover_all(
            source_url.clone(),
            &DiscoveryConfiguration::default(),
            &fetcher,
            &AllowAllPolicy,
        );

        assert!(matches!(result, Err(DiscoveryError::Redirect(url)) if url == source_url));
        assert_eq!(fetcher.fetched_urls.into_inner(), vec![source_url]);
    }

    #[test]
    fn documentation_navigation_only_visits_initial_navigation_links() {
        let source_url = url("/start");
        let navigation_url = url("/navigation");
        let mut documents = BTreeMap::new();
        documents.insert(
            source_url.clone(),
            format!(
                "{}<nav><a href=\"/navigation\">Navigation</a></nav><a href=\"/outside\">Outside</a>",
                document("Start")
            ),
        );
        documents.insert(
            navigation_url.clone(),
            format!(
                "{}<a href=\"/descendant\">Descendant</a>",
                document("Navigation")
            ),
        );
        let fetcher = GraphFetcher::new(documents);
        let configuration = DiscoveryConfiguration::new(
            DiscoveryScope::DocumentationNavigation,
            None,
            Vec::new(),
            TraversalMode::All,
            None,
            ContentFormat::GuideWithReferences,
            false,
        )
        .expect("navigation configuration must be valid");

        let pages = discover_all(
            source_url.clone(),
            &configuration,
            &fetcher,
            &AllowAllPolicy,
        )
        .expect("the navigation link should be discovered");

        assert_eq!(
            pages
                .iter()
                .map(|page| page.source_url().clone())
                .collect::<Vec<_>>(),
            vec![navigation_url.clone(), source_url.clone()]
        );
        assert_eq!(
            fetcher.fetched_urls.into_inner(),
            vec![source_url, navigation_url]
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

    #[test]
    fn source_http_error_aborts_discovery_before_any_related_url_is_visited() {
        let source_url = url("/start");
        let fetcher = StatusFetcher {
            documents: BTreeMap::from([(
                source_url.clone(),
                FetchedDocument::new(source_url.clone(), 500, "unavailable".to_owned()),
            )]),
            fetched_urls: RefCell::new(Vec::new()),
        };

        let result = discover_all(
            source_url.clone(),
            &DiscoveryConfiguration::default(),
            &fetcher,
            &AllowAllPolicy,
        );

        assert!(matches!(
            result,
            Err(DiscoveryError::UnexpectedStatus { url, status: 500 }) if url == source_url
        ));
        assert_eq!(fetcher.fetched_urls.into_inner(), vec![source_url]);
    }

    #[test]
    fn related_http_not_found_is_retained_as_an_unavailable_source() {
        let source_url = url("/start");
        let missing_url = url("/missing");
        let fetcher = StatusFetcher {
            documents: BTreeMap::from([
                (
                    source_url.clone(),
                    FetchedDocument::new(
                        source_url.clone(),
                        200,
                        format!("{}<a href=\"/missing\">Missing</a>", document("Start")),
                    ),
                ),
                (
                    missing_url.clone(),
                    FetchedDocument::new(missing_url.clone(), 404, "missing".to_owned()),
                ),
            ]),
            fetched_urls: RefCell::new(Vec::new()),
        };

        let pages = discover_all(
            source_url.clone(),
            &DiscoveryConfiguration::default(),
            &fetcher,
            &AllowAllPolicy,
        )
        .expect("a related HTTP 404 should not abort discovery");

        assert_eq!(
            pages,
            vec![
                DocumentationSource::unavailable(missing_url.clone()),
                DocumentationSource::Extracted(DocumentationPage::new(
                    source_url.clone(),
                    "Start".to_owned(),
                )),
            ]
        );
        assert_eq!(
            fetcher.fetched_urls.into_inner(),
            vec![source_url, missing_url]
        );
    }

    #[test]
    fn related_non_html_response_is_skipped_while_remaining_html_pages_are_discovered() {
        let source_url = url("/start");
        let non_html_url = url("/data");
        let related_url = url("/related");
        let fetcher = StatusFetcher {
            documents: BTreeMap::from([
                (
                    source_url.clone(),
                    FetchedDocument::new(
                        source_url.clone(),
                        200,
                        format!(
                            "{}<a href=\"/data\">Data</a><a href=\"/related\">Related</a>",
                            document("Start")
                        ),
                    ),
                ),
                (
                    non_html_url.clone(),
                    FetchedDocument::new_with_content_type(
                        non_html_url.clone(),
                        200,
                        "{\"version\":1}".to_owned(),
                        "application/json".to_owned(),
                    ),
                ),
                (
                    related_url.clone(),
                    FetchedDocument::new(related_url.clone(), 200, document("Related")),
                ),
            ]),
            fetched_urls: RefCell::new(Vec::new()),
        };

        let pages = discover_all(
            source_url.clone(),
            &DiscoveryConfiguration::default(),
            &fetcher,
            &AllowAllPolicy,
        )
        .expect("a related non-HTML response should not abort discovery");

        assert_eq!(
            pages
                .iter()
                .map(|page| page.source_url().clone())
                .collect::<Vec<_>>(),
            vec![related_url.clone(), source_url.clone()]
        );
        assert_eq!(
            fetcher.fetched_urls.into_inner(),
            vec![source_url, non_html_url, related_url]
        );
    }

    #[test]
    fn related_http_error_other_than_not_found_aborts_without_results() {
        let source_url = url("/start");
        let failing_url = url("/failing");
        let fetcher = StatusFetcher {
            documents: BTreeMap::from([
                (
                    source_url.clone(),
                    FetchedDocument::new(
                        source_url.clone(),
                        200,
                        format!("{}<a href=\"/failing\">Failing</a>", document("Start")),
                    ),
                ),
                (
                    failing_url.clone(),
                    FetchedDocument::new(failing_url.clone(), 500, "unavailable".to_owned()),
                ),
            ]),
            fetched_urls: RefCell::new(Vec::new()),
        };

        let result = discover_all(
            source_url.clone(),
            &DiscoveryConfiguration::default(),
            &fetcher,
            &AllowAllPolicy,
        );

        assert!(matches!(
            result,
            Err(DiscoveryError::UnexpectedStatus { url, status: 500 }) if url == failing_url
        ));
        assert_eq!(
            fetcher.fetched_urls.into_inner(),
            vec![source_url, failing_url]
        );
    }
}
