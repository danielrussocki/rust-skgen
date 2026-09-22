use std::{cell::RefCell, collections::BTreeMap};

use rust_skgen::{
    fetch::{DocumentFetcher, FetchError, FetchedDocument},
    sitemap::{SitemapDocument, find_sitemap_urls, parse_sitemap},
};
use url::Url;

struct SitemapFetcher {
    documents: BTreeMap<Url, FetchedDocument>,
    requested: RefCell<Vec<Url>>,
}

impl DocumentFetcher for SitemapFetcher {
    fn fetch(&self, url: Url) -> Result<FetchedDocument, FetchError> {
        self.requested.borrow_mut().push(url.clone());
        Ok(self.documents.get(&url).cloned().unwrap_or_else(|| {
            FetchedDocument::new_with_content_type(
                url.clone(),
                404,
                String::new(),
                "text/plain".to_owned(),
            )
        }))
    }
}

#[test]
fn parses_urlset_locations_in_document_order() {
    let sitemap = r#"<?xml version="1.0"?>
        <urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
          <url><loc>https://docs.example.test/first</loc></url>
          <url><loc>https://docs.example.test/second?version=2</loc></url>
        </urlset>"#;

    let parsed = parse_sitemap(sitemap).expect("a valid urlset must parse");

    assert_eq!(
        parsed,
        SitemapDocument::UrlSet(vec![
            "https://docs.example.test/first".to_owned(),
            "https://docs.example.test/second?version=2".to_owned(),
        ])
    );
}

#[test]
fn rejects_xml_that_is_not_a_urlset_sitemap() {
    let result =
        parse_sitemap("<feed><entry><loc>https://docs.example.test/page</loc></entry></feed>");

    assert!(result.is_err());
}

#[test]
fn rejects_urlsets_without_url_locations() {
    let result = parse_sitemap("<urlset><url><lastmod>2026-01-01</lastmod></url></urlset>");

    assert!(result.is_err());
}

#[test]
fn parses_sitemapindex_locations_in_document_order() {
    let sitemap = r#"<sitemapindex xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
        <sitemap><loc>https://docs.example.test/sitemap-a.xml</loc></sitemap>
        <sitemap><loc>https://docs.example.test/sitemap-b.xml</loc></sitemap>
    </sitemapindex>"#;

    let parsed = parse_sitemap(sitemap).expect("a valid sitemap index must parse");

    assert_eq!(
        parsed,
        SitemapDocument::Index(vec![
            "https://docs.example.test/sitemap-a.xml".to_owned(),
            "https://docs.example.test/sitemap-b.xml".to_owned(),
        ])
    );
}

#[test]
fn rejects_sitemap_indexes_without_sitemap_locations() {
    let result = parse_sitemap(
        "<sitemapindex><sitemap><lastmod>2026-01-01</lastmod></sitemap></sitemapindex>",
    );

    assert!(result.is_err());
}

#[test]
fn searches_conventional_sitemap_locations_in_order_until_a_valid_xml_document() {
    let source = Url::parse("https://docs.example.test/guide/start").unwrap();
    let locations = [
        "https://docs.example.test/sitemap.xml",
        "https://docs.example.test/sitemap_index.xml",
        "https://docs.example.test/sitemap-index.xml",
        "https://docs.example.test/sitemap.php",
    ];
    let documents = BTreeMap::from([
        (
            Url::parse(locations[0]).unwrap(),
            FetchedDocument::new(Url::parse(locations[0]).unwrap(), 404, String::new()),
        ),
        (
            Url::parse(locations[1]).unwrap(),
            FetchedDocument::new_with_content_type(
                Url::parse(locations[1]).unwrap(),
                200,
                "<urlset><url><loc>https://docs.example.test/from-sitemap</loc></url></urlset>"
                    .to_owned(),
                "application/xml".to_owned(),
            ),
        ),
    ]);
    let fetcher = SitemapFetcher {
        documents,
        requested: RefCell::new(Vec::new()),
    };

    let urls = find_sitemap_urls(&source, &fetcher);

    assert_eq!(
        urls,
        vec![Url::parse("https://docs.example.test/from-sitemap").unwrap()]
    );
    assert_eq!(
        fetcher.requested.into_inner(),
        locations[..2]
            .iter()
            .map(|location| Url::parse(location).unwrap())
            .collect::<Vec<_>>()
    );
}

#[test]
fn processes_referenced_sitemaps_in_index_order_and_deduplicates_canonical_urls() {
    let source = Url::parse("https://docs.example.test/start").unwrap();
    let index = Url::parse("https://docs.example.test/sitemap.xml").unwrap();
    let first = Url::parse("https://docs.example.test/first.xml").unwrap();
    let second = Url::parse("https://docs.example.test/second.xml").unwrap();
    let documents = BTreeMap::from([
        (
            index.clone(),
            FetchedDocument::new_with_content_type(
                index,
                200,
                format!(
                    "<sitemapindex><sitemap><loc>{first}</loc></sitemap><sitemap><loc>{second}</loc></sitemap></sitemapindex>"
                ),
                "application/xml".to_owned(),
            ),
        ),
        (
            first.clone(),
            FetchedDocument::new_with_content_type(
                first.clone(),
                200,
                "<urlset><url><loc>https://docs.example.test/a</loc></url><url><loc>https://docs.example.test/shared?one</loc></url></urlset>".to_owned(),
                "application/xml".to_owned(),
            ),
        ),
        (
            second.clone(),
            FetchedDocument::new_with_content_type(
                second.clone(),
                200,
                "<urlset><url><loc>https://docs.example.test/shared#two</loc></url><url><loc>https://docs.example.test/b</loc></url></urlset>".to_owned(),
                "application/xml".to_owned(),
            ),
        ),
    ]);
    let fetcher = SitemapFetcher {
        documents,
        requested: RefCell::new(Vec::new()),
    };

    let urls = find_sitemap_urls(&source, &fetcher);

    assert_eq!(
        urls,
        vec![
            Url::parse("https://docs.example.test/a").unwrap(),
            Url::parse("https://docs.example.test/shared").unwrap(),
            Url::parse("https://docs.example.test/b").unwrap(),
        ]
    );
    assert_eq!(
        fetcher.requested.into_inner(),
        vec![
            Url::parse("https://docs.example.test/sitemap.xml").unwrap(),
            first,
            second
        ]
    );
}
