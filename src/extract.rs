//! HTML documentation extraction boundary.

use crate::domain::{DocumentationPage, LinkCandidate};
use scraper::{Html, Selector};
use url::Url;

/// Marks a component that extracts normalized documentation content.
pub trait DocumentExtractor {}

/// Extracts normalized documentation content while retaining its source URL.
pub fn extract_document(
    source_url: Url,
    html: &str,
) -> Result<DocumentationPage, DocumentExtractionError> {
    let document = Html::parse_document(html);
    let content_root = ["main", "article", "[role='main']"]
        .iter()
        .find_map(|selector| {
            Selector::parse(selector)
                .ok()
                .and_then(|selector| document.select(&selector).next())
        })
        .ok_or(DocumentExtractionError::NoDocumentationContent)?;
    let selector = Selector::parse("h1, h2, h3, h4, h5, h6, p, li, pre, blockquote, td, th")
        .map_err(|_| DocumentExtractionError::NoDocumentationContent)?;
    let content = content_root
        .select(&selector)
        .map(|element| normalize_text(&element.text().collect::<String>()))
        .filter(|text| !text.is_empty())
        .collect::<Vec<_>>()
        .join("\n\n");

    if content.is_empty() {
        return Err(DocumentExtractionError::NoDocumentationContent);
    }

    Ok(DocumentationPage::new(source_url, content))
}

/// Error returned when an HTML page has no extractable documentation content.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DocumentExtractionError {
    /// The document contains no semantic documentation content.
    NoDocumentationContent,
}

impl std::fmt::Display for DocumentExtractionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoDocumentationContent => formatter.write_str("no documentation content found"),
        }
    }
}

impl std::error::Error for DocumentExtractionError {}

/// Extracts HTTP(S) links from HTML and resolves relative references against the source URL.
pub fn extract_links(source_url: &Url, html: &str) -> Vec<Url> {
    let document = Html::parse_document(html);
    let Ok(selector) = Selector::parse("a[href]") else {
        return Vec::new();
    };

    document
        .select(&selector)
        .filter_map(|element| element.value().attr("href"))
        .filter_map(|href| source_url.join(href).ok())
        .filter(|url| matches!(url.scheme(), "http" | "https"))
        .collect()
}

/// Extracts contextual HTTP(S) link candidates and resolves relative references.
pub fn extract_link_candidates(source_url: &Url, html: &str) -> Vec<LinkCandidate> {
    let document = Html::parse_document(html);
    let Ok(selector) = Selector::parse("a[href]") else {
        return Vec::new();
    };
    let Ok(navigation_selector) = Selector::parse("nav") else {
        return Vec::new();
    };
    let navigation_contexts = document
        .select(&navigation_selector)
        .flat_map(|navigation| {
            let navigation_text = normalize_text(&navigation.text().collect::<String>());
            navigation.select(&selector).filter_map(move |anchor| {
                anchor
                    .value()
                    .attr("href")
                    .map(|href| (href.to_owned(), navigation_text.clone()))
            })
        })
        .collect::<Vec<_>>();

    document
        .select(&selector)
        .filter_map(|element| {
            let href = element.value().attr("href")?;
            let url = source_url.join(href).ok()?;
            matches!(url.scheme(), "http" | "https").then(|| {
                let anchor_text = normalize_text(&element.text().collect::<String>());
                let is_navigation = element.ancestors().any(|ancestor| {
                    ancestor
                        .value()
                        .as_element()
                        .is_some_and(|element| element.name() == "nav")
                });
                let navigation_text = is_navigation.then(|| {
                    navigation_contexts
                        .iter()
                        .find(|(navigation_href, _)| navigation_href == href)
                        .map(|(_, text)| text.clone())
                        .unwrap_or_default()
                });

                match navigation_text {
                    Some(text) => LinkCandidate::with_navigation_text(url, anchor_text, text),
                    None => LinkCandidate::new(url, anchor_text, false),
                }
            })
        })
        .collect()
}

fn normalize_text(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}
