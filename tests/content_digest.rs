use rust_skgen::domain::{
    ContentFormat, DiscoveryConfiguration, DiscoveryScope, SiteBoundary, SourceUrl, TraversalMode,
};
use rust_skgen::metadata::{ManagedSkillMetadata, content_digest};

fn metadata_for(content: &str) -> ManagedSkillMetadata {
    let source_url = SourceUrl::parse("https://docs.example.com/guide").unwrap();
    let discovery = DiscoveryConfiguration::new(
        DiscoveryScope::SameSite,
        Some(SiteBoundary::ExactHost),
        Vec::new(),
        TraversalMode::All,
        None,
        ContentFormat::GuideWithReferences,
        false,
    )
    .unwrap();

    ManagedSkillMetadata::new(source_url, discovery, content_digest(content))
}

#[test]
fn accepts_content_that_matches_its_managed_digest() {
    let content = "# Generated skill\n\nUse the documented components.\n";
    let metadata = metadata_for(content);

    assert!(metadata.has_matching_content_digest(content));
}

#[test]
fn detects_a_manual_modification_to_managed_content() {
    let content = "# Generated skill\n\nUse the documented components.\n";
    let metadata = metadata_for(content);
    let modified_content = "# Generated skill\n\nUse arbitrary components.\n";

    assert!(!metadata.has_matching_content_digest(modified_content));
}
