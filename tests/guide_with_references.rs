use rust_skgen::domain::DocumentationPage;
use rust_skgen::render::render_guide_with_references;
use url::Url;

fn page(source_url: &str, content: &str) -> DocumentationPage {
    DocumentationPage::new(
        Url::parse(source_url).expect("test source URL must be valid"),
        content.to_owned(),
    )
}

#[test]
fn renders_a_guide_with_an_extracted_objective_instructions_and_references() {
    let installation = page(
        "https://docs.example.com/guides/installation",
        "Install the package before configuring components.",
    );
    let configuration = page(
        "https://docs.example.com/guides/configuration",
        "Configure components with the documented options.",
    );

    let rendered = render_guide_with_references(&[installation, configuration]);

    assert_eq!(
        rendered,
        "# Documentation Guide\n\n## Objective\n\nConfigure components with the documented options.\n\n## Instructions\n\n### Source: https://docs.example.com/guides/configuration\n\nConfigure components with the documented options.\n\n### Source: https://docs.example.com/guides/installation\n\nInstall the package before configuring components.\n\n## References\n\n- https://docs.example.com/guides/configuration\n- https://docs.example.com/guides/installation\n"
    );
}

#[test]
fn orders_pages_by_canonical_source_url_regardless_of_input_order() {
    let later = page(
        "https://docs.example.com/zebra?version=2#usage",
        "Use the Zebra component.",
    );
    let earlier = page(
        "https://docs.example.com/alpha?version=1#overview",
        "Use the Alpha component.",
    );

    let forward = render_guide_with_references(&[later.clone(), earlier.clone()]);
    let reverse = render_guide_with_references(&[earlier, later]);

    assert_eq!(forward, reverse);
    assert!(forward.contains("## Objective\n\nUse the Alpha component."));
    assert!(forward.contains("### Source: https://docs.example.com/alpha?version=1#overview"));
    assert!(forward.contains("### Source: https://docs.example.com/zebra?version=2#usage"));
}
