use std::{cell::RefCell, collections::BTreeMap};

use rust_skgen::{
    discover::discover_all,
    domain::{ContentFormat, DiscoveryConfiguration, DiscoveryScope, TraversalMode},
    fetch::{DocumentFetcher, FetchError, FetchedDocument},
    policy::{CrawlPolicy, RobotsError},
};
use url::Url;

struct LocalFetcher {
    documents: BTreeMap<Url, FetchedDocument>,
    requested: RefCell<Vec<Url>>,
}

impl DocumentFetcher for LocalFetcher {
    fn fetch(&self, url: Url) -> Result<FetchedDocument, FetchError> {
        self.requested.borrow_mut().push(url.clone());
        Ok(self.documents.get(&url).cloned().unwrap_or_else(|| {
            FetchedDocument::new_with_content_type(url, 404, String::new(), "text/plain".to_owned())
        }))
    }
}

struct AllowAll;

impl CrawlPolicy for AllowAll {
    fn allows(&self, _url: &Url) -> Result<bool, RobotsError> {
        Ok(true)
    }
}

struct DenyUrl {
    denied: Url,
}

impl CrawlPolicy for DenyUrl {
    fn allows(&self, url: &Url) -> Result<bool, RobotsError> {
        Ok(url != &self.denied)
    }
}

fn html(url: Url, body: &str) -> FetchedDocument {
    FetchedDocument::new_with_content_type(url, 200, body.to_owned(), "text/html".to_owned())
}

#[test]
fn unusable_sitemaps_fall_back_to_html_link_discovery() {
    let source = Url::parse("https://docs.example.test/start").unwrap();
    let related = Url::parse("https://docs.example.test/related").unwrap();
    let fetcher = LocalFetcher {
        documents: BTreeMap::from([
            (
                source.clone(),
                html(
                    source.clone(),
                    "<main><p>Start</p><a href=\"/related\">Related</a></main>",
                ),
            ),
            (
                related.clone(),
                html(related.clone(), "<main><p>Related</p></main>"),
            ),
            (
                Url::parse("https://docs.example.test/sitemap_index.xml").unwrap(),
                FetchedDocument::new_with_content_type(
                    Url::parse("https://docs.example.test/sitemap_index.xml").unwrap(),
                    200,
                    "not XML".to_owned(),
                    "text/plain".to_owned(),
                ),
            ),
            (
                Url::parse("https://docs.example.test/sitemap-index.xml").unwrap(),
                FetchedDocument::new_with_content_type(
                    Url::parse("https://docs.example.test/sitemap-index.xml").unwrap(),
                    302,
                    String::new(),
                    "application/xml".to_owned(),
                ),
            ),
            (
                Url::parse("https://docs.example.test/sitemap.php").unwrap(),
                FetchedDocument::new_with_content_type(
                    Url::parse("https://docs.example.test/sitemap.php").unwrap(),
                    200,
                    "<not-a-sitemap/>".to_owned(),
                    "application/xml".to_owned(),
                ),
            ),
        ]),
        requested: RefCell::new(Vec::new()),
    };

    let pages = discover_all(
        source.clone(),
        &DiscoveryConfiguration::default(),
        &fetcher,
        &AllowAll,
    )
    .expect("unusable sitemap resources must not abort HTML discovery");

    assert_eq!(
        pages
            .iter()
            .map(|page| page.source_url().clone())
            .collect::<Vec<_>>(),
        vec![related.clone(), source.clone()]
    );
    assert_eq!(
        fetcher.requested.into_inner(),
        vec![
            source,
            Url::parse("https://docs.example.test/sitemap.xml").unwrap(),
            Url::parse("https://docs.example.test/sitemap_index.xml").unwrap(),
            Url::parse("https://docs.example.test/sitemap-index.xml").unwrap(),
            Url::parse("https://docs.example.test/sitemap.php").unwrap(),
            related,
        ]
    );
}

#[test]
fn visits_in_scope_sitemap_candidates_before_html_links_and_includes_later_html_pages() {
    let source = Url::parse("https://docs.example.test/start").unwrap();
    let sitemap_page = Url::parse("https://docs.example.test/z-from-sitemap").unwrap();
    let html_page = Url::parse("https://docs.example.test/a-from-html").unwrap();
    let later_page = Url::parse("https://docs.example.test/later").unwrap();
    let sitemap = Url::parse("https://docs.example.test/sitemap.xml").unwrap();
    let fetcher = LocalFetcher {
        documents: BTreeMap::from([
            (
                source.clone(),
                html(
                    source.clone(),
                    "<main><p>Start</p><a href=\"/a-from-html\">HTML</a></main>",
                ),
            ),
            (
                sitemap.clone(),
                FetchedDocument::new_with_content_type(
                    sitemap.clone(),
                    200,
                    format!("<urlset><url><loc>{sitemap_page}</loc></url></urlset>"),
                    "application/xml".to_owned(),
                ),
            ),
            (
                sitemap_page.clone(),
                html(sitemap_page.clone(), "<main><p>Sitemap</p></main>"),
            ),
            (
                html_page.clone(),
                html(
                    html_page.clone(),
                    "<main><p>HTML</p><a href=\"/later\">Later</a></main>",
                ),
            ),
            (
                later_page.clone(),
                html(later_page.clone(), "<main><p>Later</p></main>"),
            ),
        ]),
        requested: RefCell::new(Vec::new()),
    };

    let pages = discover_all(
        source.clone(),
        &DiscoveryConfiguration::default(),
        &fetcher,
        &AllowAll,
    )
    .expect("sitemap and HTML pages must be discovered");

    assert_eq!(pages.len(), 4);
    let requests = fetcher.requested.into_inner();
    assert_eq!(requests[0], source);
    assert_eq!(requests[1], sitemap);
    assert_eq!(requests[2], sitemap_page);
    assert_eq!(requests[3], html_page);
    assert_eq!(requests[4], later_page);
}

#[test]
fn sitemap_candidates_obey_scope_deduplication_and_traversal_limits() {
    let source = Url::parse("https://docs.example.test/docs/guide/start").unwrap();
    let sitemap = Url::parse("https://docs.example.test/sitemap.xml").unwrap();
    let included = Url::parse("https://docs.example.test/docs/guide/start/included").unwrap();
    let descendant = Url::parse("https://docs.example.test/docs/guide/start/descendant").unwrap();
    let outside = Url::parse("https://docs.example.test/outside").unwrap();
    let fetcher = LocalFetcher {
        documents: BTreeMap::from([
            (
                source.clone(),
                html(source.clone(), "<main><p>Start</p></main>"),
            ),
            (
                sitemap.clone(),
                FetchedDocument::new_with_content_type(
                    sitemap,
                    200,
                    format!(
                        "<urlset><url><loc>{outside}</loc></url><url><loc>{included}?one</loc></url><url><loc>{included}#two</loc></url></urlset>"
                    ),
                    "application/xml".to_owned(),
                ),
            ),
            (
                included.clone(),
                html(
                    included.clone(),
                    "<main><p>Included</p><a href=\"/docs/guide/start/descendant\">Descendant</a></main>",
                ),
            ),
            (
                descendant.clone(),
                html(descendant.clone(), "<main><p>Descendant</p></main>"),
            ),
        ]),
        requested: RefCell::new(Vec::new()),
    };
    let one_level = DiscoveryConfiguration::new(
        DiscoveryScope::PathPrefix,
        None,
        Vec::new(),
        TraversalMode::OneLevel,
        None,
        ContentFormat::GuideWithReferences,
        false,
    )
    .unwrap();

    let pages = discover_all(source.clone(), &one_level, &fetcher, &AllowAll)
        .expect("admissible sitemap candidates must be discovered");

    assert_eq!(
        pages
            .iter()
            .map(|page| page.source_url().clone())
            .collect::<Vec<_>>(),
        vec![source, included.clone()]
    );
    let requests = fetcher.requested.into_inner();
    assert!(!requests.contains(&outside));
    assert!(!requests.contains(&descendant));
}

#[test]
fn limited_traversal_counts_sitemap_candidates_toward_maximum_pages() {
    let source = Url::parse("https://docs.example.test/start").unwrap();
    let sitemap = Url::parse("https://docs.example.test/sitemap.xml").unwrap();
    let first = Url::parse("https://docs.example.test/first").unwrap();
    let second = Url::parse("https://docs.example.test/second").unwrap();
    let fetcher = LocalFetcher {
        documents: BTreeMap::from([
            (
                source.clone(),
                html(source.clone(), "<main><p>Start</p></main>"),
            ),
            (
                sitemap.clone(),
                FetchedDocument::new_with_content_type(
                    sitemap,
                    200,
                    format!(
                        "<urlset><url><loc>{first}</loc></url><url><loc>{second}</loc></url></urlset>"
                    ),
                    "application/xml".to_owned(),
                ),
            ),
            (
                first.clone(),
                html(first.clone(), "<main><p>First</p></main>"),
            ),
            (
                second.clone(),
                html(second.clone(), "<main><p>Second</p></main>"),
            ),
        ]),
        requested: RefCell::new(Vec::new()),
    };
    let limited = DiscoveryConfiguration::new(
        DiscoveryScope::SameSite,
        None,
        Vec::new(),
        TraversalMode::Limited,
        Some(2),
        ContentFormat::GuideWithReferences,
        false,
    )
    .unwrap();

    let pages = discover_all(source.clone(), &limited, &fetcher, &AllowAll)
        .expect("reaching the maximum through sitemap candidates must succeed");

    assert_eq!(pages.len(), 2);
    assert_eq!(
        fetcher.requested.into_inner(),
        vec![
            source,
            Url::parse("https://docs.example.test/sitemap.xml").unwrap(),
            first
        ]
    );
}

#[test]
fn skips_sitemap_candidates_forbidden_by_crawl_policy_and_discovers_remaining_candidates() {
    let source = Url::parse("https://docs.example.test/start").unwrap();
    let sitemap = Url::parse("https://docs.example.test/sitemap.xml").unwrap();
    let forbidden = Url::parse("https://docs.example.test/forbidden").unwrap();
    let allowed = Url::parse("https://docs.example.test/allowed").unwrap();
    let fetcher = LocalFetcher {
        documents: BTreeMap::from([
            (
                source.clone(),
                html(source.clone(), "<main><p>Start</p></main>"),
            ),
            (
                sitemap.clone(),
                FetchedDocument::new_with_content_type(
                    sitemap,
                    200,
                    format!(
                        "<urlset><url><loc>{forbidden}</loc></url><url><loc>{allowed}</loc></url></urlset>"
                    ),
                    "application/xml".to_owned(),
                ),
            ),
            (
                allowed.clone(),
                html(allowed.clone(), "<main><p>Allowed</p></main>"),
            ),
        ]),
        requested: RefCell::new(Vec::new()),
    };

    let pages = discover_all(
        source.clone(),
        &DiscoveryConfiguration::default(),
        &fetcher,
        &DenyUrl {
            denied: forbidden.clone(),
        },
    )
    .expect("a forbidden sitemap candidate must not abort remaining discovery");

    assert_eq!(
        pages
            .iter()
            .map(|page| page.source_url().clone())
            .collect::<Vec<_>>(),
        vec![allowed.clone(), source]
    );
    assert!(!fetcher.requested.into_inner().contains(&forbidden));
}
