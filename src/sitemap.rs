//! Structured sitemap XML parsing.

use std::collections::BTreeSet;

use quick_xml::{Reader, events::Event};
use url::Url;

use crate::fetch::DocumentFetcher;

/// A sitemap document containing either canonical page URLs or sitemap references.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SitemapDocument {
    /// A `urlset` sitemap containing canonical documentation URLs.
    UrlSet(Vec<String>),
    /// A `sitemapindex` sitemap containing referenced sitemap locations.
    Index(Vec<String>),
}

/// Error returned when XML does not represent a valid sitemap document.
#[derive(Debug)]
pub enum SitemapError {
    /// The XML could not be parsed.
    Xml(quick_xml::Error),
    /// The root element is not a supported sitemap type.
    UnsupportedRoot,
    /// A text value could not be decoded as XML content.
    InvalidText(String),
    /// A sitemap entry does not include a non-empty `loc` value.
    MissingLocation,
}

impl std::fmt::Display for SitemapError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Xml(error) => write!(formatter, "invalid sitemap XML: {error}"),
            Self::UnsupportedRoot => formatter.write_str("unsupported sitemap XML root element"),
            Self::InvalidText(error) => write!(formatter, "invalid sitemap XML text: {error}"),
            Self::MissingLocation => formatter.write_str("sitemap entry is missing a location"),
        }
    }
}

impl std::error::Error for SitemapError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Xml(error) => Some(error),
            Self::UnsupportedRoot | Self::InvalidText(_) | Self::MissingLocation => None,
        }
    }
}

/// Parses a `urlset` or `sitemapindex` document and preserves its location order.
pub fn parse_sitemap(xml: &str) -> Result<SitemapDocument, SitemapError> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);
    let mut root = None;
    let mut entry_name = None;
    let mut locations = Vec::new();
    let mut location = None;
    let mut entry_has_location = false;

    loop {
        match reader.read_event().map_err(SitemapError::Xml)? {
            Event::Start(element) => {
                let name = element.local_name();
                match root.as_deref() {
                    None if name.as_ref() == b"urlset" || name.as_ref() == b"sitemapindex" => {
                        root = Some(String::from_utf8_lossy(name.as_ref()).into_owned());
                    }
                    None => return Err(SitemapError::UnsupportedRoot),
                    Some("urlset") if name.as_ref() == b"url" => {
                        entry_name = Some("url");
                        entry_has_location = false;
                    }
                    Some("sitemapindex") if name.as_ref() == b"sitemap" => {
                        entry_name = Some("sitemap");
                        entry_has_location = false;
                    }
                    _ if entry_name.is_some() && name.as_ref() == b"loc" => {
                        location = Some(String::new())
                    }
                    _ => {}
                }
            }
            Event::Text(text) if location.is_some() => {
                let value = text
                    .unescape()
                    .map_err(|error| SitemapError::InvalidText(error.to_string()))?;
                if let Some(location) = location.as_mut() {
                    location.push_str(&value);
                }
            }
            Event::End(element) => {
                let name = element.local_name();
                if name.as_ref() == b"loc" {
                    let value = location.take().unwrap_or_default();
                    if value.is_empty() {
                        return Err(SitemapError::MissingLocation);
                    }
                    locations.push(value);
                    entry_has_location = true;
                } else if entry_name.is_some_and(|entry| entry.as_bytes() == name.as_ref()) {
                    if !entry_has_location {
                        return Err(SitemapError::MissingLocation);
                    }
                    entry_name = None;
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }

    match root.as_deref() {
        Some("urlset") if !locations.is_empty() => Ok(SitemapDocument::UrlSet(locations)),
        Some("sitemapindex") if !locations.is_empty() => Ok(SitemapDocument::Index(locations)),
        Some(_) => Err(SitemapError::MissingLocation),
        None => Err(SitemapError::UnsupportedRoot),
    }
}

/// Finds canonical URLs from the first valid conventional sitemap document.
pub fn find_sitemap_urls<F: DocumentFetcher>(source_url: &Url, fetcher: &F) -> Vec<Url> {
    const LOCATIONS: [&str; 4] = [
        "/sitemap.xml",
        "/sitemap_index.xml",
        "/sitemap-index.xml",
        "/sitemap.php",
    ];

    for location in LOCATIONS {
        let Ok(sitemap_url) = source_url.join(location) else {
            continue;
        };
        let Ok(document) = fetcher.fetch(sitemap_url) else {
            continue;
        };
        if !(200..300).contains(&document.status()) || !document.is_xml() {
            continue;
        }
        match parse_sitemap(document.body()) {
            Ok(SitemapDocument::UrlSet(locations)) => return canonical_urls(locations),
            Ok(SitemapDocument::Index(references)) => {
                let mut urls = Vec::new();
                let mut seen = BTreeSet::new();
                for reference in references {
                    let Ok(reference) = Url::parse(&reference) else {
                        continue;
                    };
                    let Ok(document) = fetcher.fetch(reference) else {
                        continue;
                    };
                    if !(200..300).contains(&document.status()) || !document.is_xml() {
                        continue;
                    }
                    let Ok(SitemapDocument::UrlSet(locations)) = parse_sitemap(document.body())
                    else {
                        continue;
                    };
                    for url in canonical_urls(locations) {
                        if seen.insert(url.clone()) {
                            urls.push(url);
                        }
                    }
                }
                return urls;
            }
            Err(_) => continue,
        }
    }

    Vec::new()
}

fn canonical_urls(locations: Vec<String>) -> Vec<Url> {
    let mut seen = BTreeSet::new();
    let mut urls = Vec::new();
    for mut url in locations
        .into_iter()
        .filter_map(|location| Url::parse(&location).ok())
    {
        if !matches!(url.scheme(), "http" | "https") {
            continue;
        }
        url.set_query(None);
        url.set_fragment(None);
        if seen.insert(url.clone()) {
            urls.push(url);
        }
    }
    urls
}
