use rust_skgen::extract::extract_links;
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
