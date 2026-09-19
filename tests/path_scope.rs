use rust_skgen::discover::{is_within_parent_directory, is_within_path_prefix};
use url::Url;

fn parse_url(value: &str) -> Url {
    Url::parse(value).unwrap_or_else(|error| panic!("invalid test URL: {error}"))
}

#[test]
fn path_prefix_accepts_only_the_source_path_and_its_descendants() {
    let source = parse_url("https://docs.example.com/reference/components");
    let source_path = parse_url("https://docs.example.com/reference/components");
    let descendant = parse_url("https://docs.example.com/reference/components/button");
    let sibling = parse_url("https://docs.example.com/reference/hooks");
    let partial_match = parse_url("https://docs.example.com/reference/components-api");

    assert!(is_within_path_prefix(&source, &source_path));
    assert!(is_within_path_prefix(&source, &descendant));
    assert!(!is_within_path_prefix(&source, &sibling));
    assert!(!is_within_path_prefix(&source, &partial_match));
}

#[test]
fn parent_directory_accepts_only_pages_in_the_source_directory() {
    let source = parse_url("https://docs.example.com/reference/components/button");
    let sibling = parse_url("https://docs.example.com/reference/components/dialog");
    let descendant = parse_url("https://docs.example.com/reference/components/button/api");
    let parent = parse_url("https://docs.example.com/reference");
    let neighboring_directory =
        parse_url("https://docs.example.com/reference/components-api/button");

    assert!(is_within_parent_directory(&source, &sibling));
    assert!(is_within_parent_directory(&source, &descendant));
    assert!(!is_within_parent_directory(&source, &parent));
    assert!(!is_within_parent_directory(&source, &neighboring_directory));
}

#[test]
fn root_parent_directory_includes_descendants_of_the_root_directory() {
    let source = parse_url("https://docs.example.com/overview");
    let root_page = parse_url("https://docs.example.com/faq");
    let nested_page = parse_url("https://docs.example.com/guides/installation");

    assert!(is_within_parent_directory(&source, &root_page));
    assert!(is_within_parent_directory(&source, &nested_page));
}

#[test]
fn path_scopes_reject_matching_paths_from_a_different_origin() {
    let source = parse_url("https://docs.example.com:8443/reference/components/button");
    let different_host =
        parse_url("https://other.example.com:8443/reference/components/button/api");
    let different_scheme =
        parse_url("http://docs.example.com:8443/reference/components/button/api");
    let different_port = parse_url("https://docs.example.com/reference/components/button/api");

    for candidate in [different_host, different_scheme, different_port] {
        assert!(!is_within_path_prefix(&source, &candidate));
        assert!(!is_within_parent_directory(&source, &candidate));
    }
}
