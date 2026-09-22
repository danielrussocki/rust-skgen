use rust_skgen::{discover::candidate_priority, domain::LinkCandidate};
use url::Url;

#[test]
fn preserves_context_while_normalizing_the_candidate_url() {
    let candidate = LinkCandidate::new(
        Url::parse("https://docs.example.test/guide?version=1#install")
            .expect("test URL must be valid"),
        "Installation Guide".to_owned(),
        true,
    );

    assert_eq!(
        candidate.url(),
        &Url::parse("https://docs.example.test/guide").expect("expected URL must be valid")
    );
    assert_eq!(candidate.anchor_text(), "Installation Guide");
    assert!(candidate.is_navigation());
}

#[test]
fn scores_documentation_signals_case_insensitively_from_each_context() {
    let path_candidate = LinkCandidate::new(
        Url::parse("https://docs.example.test/DOCS").expect("test URL must be valid"),
        String::new(),
        false,
    );
    let anchor_candidate = LinkCandidate::new(
        Url::parse("https://docs.example.test/page").expect("test URL must be valid"),
        "Guide API Reference Components".to_owned(),
        false,
    );
    let navigation_candidate = LinkCandidate::with_navigation_text(
        Url::parse("https://docs.example.test/page").expect("test URL must be valid"),
        String::new(),
        "Components documentation".to_owned(),
    );

    assert_eq!(candidate_priority(&path_candidate), 1);
    assert_eq!(candidate_priority(&anchor_candidate), 4);
    assert_eq!(candidate_priority(&navigation_candidate), 1);
}

#[test]
fn penalizes_terms_and_resource_destinations_without_rejecting_candidates() {
    let terms = LinkCandidate::new(
        Url::parse("https://docs.example.test/page").expect("test URL must be valid"),
        "BLOG Changelog releases supported-browsers privacy terms careers".to_owned(),
        false,
    );
    let image = LinkCandidate::new(
        Url::parse("https://docs.example.test/logo.svg").expect("test URL must be valid"),
        String::new(),
        false,
    );
    let static_resource = LinkCandidate::new(
        Url::parse("https://docs.example.test/site.css").expect("test URL must be valid"),
        String::new(),
        false,
    );
    let social = LinkCandidate::new(
        Url::parse("https://twitter.com/example").expect("test URL must be valid"),
        String::new(),
        false,
    );

    assert_eq!(candidate_priority(&terms), -7);
    assert_eq!(candidate_priority(&image), -1);
    assert_eq!(candidate_priority(&static_resource), -1);
    assert_eq!(candidate_priority(&social), -1);
}
