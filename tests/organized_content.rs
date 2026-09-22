use rust_skgen::domain::{DocumentationPage, DocumentationSource};
use rust_skgen::render::render_organized_content;
use url::Url;

fn page(source_url: &str, content: &str) -> DocumentationPage {
    DocumentationPage::new(
        Url::parse(source_url).expect("test source URL must be valid"),
        content.to_owned(),
    )
}

#[test]
fn renders_only_extracted_content_grouped_by_source_page() {
    let installation = page(
        "https://docs.example.com/guides/installation",
        "Install the package before configuring components.",
    );
    let configuration = page(
        "https://docs.example.com/guides/configuration",
        "Configure components with the documented options.",
    );

    let rendered = render_organized_content(&[installation, configuration]);

    assert_eq!(
        rendered,
        "# Organized Documentation\n\n## Source: https://docs.example.com/guides/configuration\n\nConfigure components with the documented options.\n\n## Source: https://docs.example.com/guides/installation\n\nInstall the package before configuring components.\n"
    );
    assert!(!rendered.contains("Objective"));
    assert!(!rendered.contains("Instructions"));
    assert!(!rendered.contains("References"));
}

#[test]
fn orders_content_by_canonical_source_url_regardless_of_input_order() {
    let later = page(
        "https://docs.example.com/zebra?version=2#usage",
        "Use the Zebra component.",
    );
    let earlier = page(
        "https://docs.example.com/alpha?version=1#overview",
        "Use the Alpha component.",
    );

    let forward = render_organized_content(&[later.clone(), earlier.clone()]);
    let reverse = render_organized_content(&[earlier, later]);

    assert_eq!(forward, reverse);
    assert!(forward.contains("## Source: https://docs.example.com/alpha?version=1#overview"));
    assert!(forward.contains("## Source: https://docs.example.com/zebra?version=2#usage"));
}

#[test]
fn renders_an_unavailable_related_source_as_a_notice() {
    let rendered = render_organized_content(&[DocumentationSource::unavailable(
        Url::parse("https://docs.example.com/missing?version=1#overview")
            .expect("test URL must be valid"),
    )]);

    assert!(rendered.contains("https://docs.example.com/missing"));
    assert!(rendered.contains("Documentation is unavailable for this source."));
    assert!(rendered.contains("Search the internet or the source code for current documentation."));
}
