use rust_skgen::domain::DocumentationSource;
use url::Url;

#[test]
fn unavailable_related_source_retains_its_canonical_url_without_content() {
    let source = DocumentationSource::unavailable(
        Url::parse("https://docs.example.test/reference/missing?version=1#section")
            .expect("test URL must be valid"),
    );

    assert!(source.is_unavailable());
    assert_eq!(
        source.source_url().as_str(),
        "https://docs.example.test/reference/missing"
    );
    assert_eq!(source.content(), None);
}
