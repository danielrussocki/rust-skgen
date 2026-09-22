use rust_skgen::extract::{extract_link_candidates, extract_links};
use url::Url;

#[test]
fn resolves_relative_links_against_the_source_url() {
    let source_url = Url::parse("https://docs.example.com/guides/getting-started/index.html")
        .expect("test source URL must be valid");
    let html = r#"
        <a href="installation.html">Installation</a>
        <a href="../reference">Reference</a>
        <a href="/overview">Overview</a>
        <a href="https://api.example.com/v1">API</a>
    "#;

    let links = extract_links(&source_url, html);

    assert_eq!(
        links,
        vec![
            Url::parse("https://docs.example.com/guides/getting-started/installation.html")
                .expect("expected URL must be valid"),
            Url::parse("https://docs.example.com/guides/reference")
                .expect("expected URL must be valid"),
            Url::parse("https://docs.example.com/overview").expect("expected URL must be valid"),
            Url::parse("https://api.example.com/v1").expect("expected URL must be valid"),
        ]
    );
}

#[test]
fn discards_links_with_non_http_schemes() {
    let source_url =
        Url::parse("https://docs.example.com/guide/").expect("test source URL must be valid");
    let html = r#"
        <a href="mailto:docs@example.com">Email</a>
        <a href="javascript:alert('ignored')">Script</a>
        <a href="ftp://files.example.com/archive">Archive</a>
        <a href="data:text/plain,ignored">Data</a>
        <a href="https://docs.example.com/next">Next</a>
    "#;

    assert_eq!(
        extract_links(&source_url, html),
        vec![Url::parse("https://docs.example.com/next").expect("expected URL must be valid")]
    );
}

#[test]
fn extracts_http_candidates_with_anchor_text_and_navigation_context() {
    let source_url =
        Url::parse("https://docs.example.com/guide/").expect("test source URL must be valid");
    let candidates = extract_link_candidates(
        &source_url,
        r#"
            <nav><a href="intro"> Introduction </a></nav>
            <a href="/reference">API Reference</a>
            <a href="mailto:docs@example.com">Email</a>
        "#,
    );

    assert_eq!(candidates.len(), 2);
    assert_eq!(
        candidates[0].url().as_str(),
        "https://docs.example.com/guide/intro"
    );
    assert_eq!(candidates[0].anchor_text(), "Introduction");
    assert!(candidates[0].is_navigation());
    assert_eq!(candidates[0].navigation_text(), "Introduction");
    assert_eq!(
        candidates[1].url().as_str(),
        "https://docs.example.com/reference"
    );
    assert_eq!(candidates[1].anchor_text(), "API Reference");
    assert!(!candidates[1].is_navigation());
}
