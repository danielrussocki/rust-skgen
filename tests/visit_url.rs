use rust_skgen::discover::normalize_visit_url;
use url::Url;

#[test]
fn removes_query_and_fragment_from_a_visit_url() {
    let url = Url::parse("https://docs.example.com/guide?version=2#installation")
        .unwrap_or_else(|error| panic!("invalid test URL: {error}"));

    let normalized = normalize_visit_url(&url);

    assert_eq!(normalized.as_str(), "https://docs.example.com/guide");
}

#[test]
fn treats_urls_that_differ_only_by_query_or_fragment_as_equal() {
    let first = Url::parse("https://docs.example.com/guide?version=1#overview")
        .unwrap_or_else(|error| panic!("invalid test URL: {error}"));
    let second = Url::parse("https://docs.example.com/guide?version=2#api")
        .unwrap_or_else(|error| panic!("invalid test URL: {error}"));

    assert_eq!(normalize_visit_url(&first), normalize_visit_url(&second));
}

#[test]
fn preserves_scheme_host_port_and_path_when_normalizing() {
    let url = Url::parse("http://docs.example.com:8080/reference/button?theme=dark#props")
        .unwrap_or_else(|error| panic!("invalid test URL: {error}"));

    let normalized = normalize_visit_url(&url);

    assert_eq!(
        normalized.as_str(),
        "http://docs.example.com:8080/reference/button"
    );
}
