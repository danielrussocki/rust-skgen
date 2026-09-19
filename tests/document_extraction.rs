use rust_skgen::extract::{DocumentExtractionError, extract_document};
use url::Url;

#[test]
fn extracts_normalized_documentation_with_its_source_url() {
    let source_url = Url::parse("https://docs.example.com/guides/installation")
        .expect("test source URL must be valid");
    let html = r#"
        <html>
          <body>
            <nav><a href="/">Home</a></nav>
            <main>
              <h1>Installation</h1>
              <p>Install the package with your preferred package manager.</p>
            </main>
          </body>
        </html>
    "#;

    let page = extract_document(source_url.clone(), html).expect("document should be extracted");

    assert_eq!(page.source_url(), &source_url);
    assert_eq!(
        page.content(),
        "Installation\n\nInstall the package with your preferred package manager."
    );
}

#[test]
fn rejects_html_without_documentation_content() {
    let source_url =
        Url::parse("https://docs.example.com/").expect("test source URL must be valid");
    let html = r#"
        <html>
          <body>
            <nav><a href="/guides">Guides</a></nav>
            <footer>Copyright Example</footer>
          </body>
        </html>
    "#;

    assert_eq!(
        extract_document(source_url, html),
        Err(DocumentExtractionError::NoDocumentationContent)
    );
}
