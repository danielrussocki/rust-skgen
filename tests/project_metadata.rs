#[test]
fn package_metadata_declares_the_minimum_supported_rust_version() {
    let manifest = include_str!("../Cargo.toml");

    assert!(manifest.contains("name = \"rust-skgen\""));
    assert!(manifest.contains("edition = \"2024\""));
    assert!(manifest.contains("rust-version = \"1.85\""));
}

#[test]
fn manifest_declares_only_the_dependencies_justified_by_the_specification() {
    let manifest = include_str!("../Cargo.toml");

    for dependency in [
        "reqwest =",
        "url =",
        "scraper =",
        "serde =",
        "serde_json =",
        "sha2 =",
        "clap =",
    ] {
        assert!(
            manifest.contains(dependency),
            "missing dependency: {dependency}"
        );
    }
}
