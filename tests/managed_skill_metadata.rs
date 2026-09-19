use rust_skgen::domain::{
    AuthorizedHost, ContentFormat, DiscoveryConfiguration, DiscoveryScope, SiteBoundary, SourceUrl,
    TraversalMode,
};
use rust_skgen::metadata::ManagedSkillMetadata;

fn metadata() -> ManagedSkillMetadata {
    let source_url = SourceUrl::parse("https://docs.example.com/guide").unwrap();
    let discovery = DiscoveryConfiguration::new(
        DiscoveryScope::SameSite,
        Some(SiteBoundary::BaseDomain),
        vec![AuthorizedHost::new("api.docs.example.com")],
        TraversalMode::Limited,
        Some(25),
        ContentFormat::OrganizedContent,
    )
    .unwrap();

    ManagedSkillMetadata::new(
        source_url,
        discovery,
        "sha256:7b50fcd0d5f3a4c8b3e53b7a8585a35e42f2cfc0cb37360e6f4ecaa7f2e7166e".to_owned(),
    )
}

#[test]
fn serializes_every_required_configuration_field() {
    let json = metadata().to_json().unwrap();
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();

    assert_eq!(value["schema_version"], 1);
    assert_eq!(value["generator"], "rust-skgen");
    assert_eq!(value["source_url"], "https://docs.example.com/guide");
    assert_eq!(value["discovery"]["scope"], "same-site");
    assert_eq!(value["discovery"]["site_boundary"], "base-domain");
    assert_eq!(
        value["discovery"]["allowed_subdomains"],
        serde_json::json!(["api.docs.example.com"])
    );
    assert_eq!(value["discovery"]["traversal"]["mode"], "limited");
    assert_eq!(value["discovery"]["traversal"]["max_pages"], 25);
    assert_eq!(value["content_format"], "organized-content");
    assert_eq!(
        value["content_digest"],
        "sha256:7b50fcd0d5f3a4c8b3e53b7a8585a35e42f2cfc0cb37360e6f4ecaa7f2e7166e"
    );
}

#[test]
fn deserializes_valid_metadata_into_validated_domain_types() {
    let restored = ManagedSkillMetadata::from_json(&metadata().to_json().unwrap()).unwrap();

    assert_eq!(
        restored.source_url().as_url().as_str(),
        "https://docs.example.com/guide"
    );
    assert_eq!(restored.discovery().scope(), DiscoveryScope::SameSite);
    assert_eq!(
        restored.discovery().site_boundary(),
        SiteBoundary::BaseDomain
    );
    assert_eq!(
        restored.discovery().authorized_subdomains()[0].as_str(),
        "api.docs.example.com"
    );
    assert_eq!(
        restored.discovery().traversal_mode(),
        TraversalMode::Limited
    );
    assert_eq!(restored.discovery().max_pages(), Some(25));
    assert_eq!(
        restored.discovery().content_format(),
        ContentFormat::OrganizedContent
    );
}

#[test]
fn rejects_an_unsupported_schema_version() {
    let invalid = metadata()
        .to_json()
        .unwrap()
        .replace("\"schema_version\":1", "\"schema_version\":2");

    assert!(ManagedSkillMetadata::from_json(&invalid).is_err());
}

#[test]
fn rejects_invalid_configuration_values() {
    let invalid = metadata()
        .to_json()
        .unwrap()
        .replace("\"mode\":\"limited\"", "\"mode\":\"recursive\"");

    assert!(ManagedSkillMetadata::from_json(&invalid).is_err());
}

#[test]
fn rejects_metadata_missing_an_essential_field() {
    let invalid = r#"{
        "schema_version": 1,
        "generator": "rust-skgen",
        "discovery": {
            "scope": "same-site",
            "site_boundary": "exact-host",
            "allowed_subdomains": [],
            "traversal": { "mode": "all" }
        },
        "content_format": "guide-with-references",
        "content_digest": "sha256:abc"
    }"#;

    assert!(ManagedSkillMetadata::from_json(invalid).is_err());
}

#[test]
fn rejects_empty_or_malformed_content_digests() {
    let valid_digest = "sha256:7b50fcd0d5f3a4c8b3e53b7a8585a35e42f2cfc0cb37360e6f4ecaa7f2e7166e";

    for digest in [
        "",
        "sha512:abc",
        "sha256:abc",
        "sha256:zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz",
    ] {
        let invalid = metadata().to_json().unwrap().replace(valid_digest, digest);

        assert!(ManagedSkillMetadata::from_json(&invalid).is_err());
    }
}
