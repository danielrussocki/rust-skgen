use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use rust_skgen::{
    domain::{
        ContentFormat, DiscoveryConfiguration, DiscoveryScope, SiteBoundary, SkillName, SourceUrl,
        TraversalMode,
    },
    metadata::{ManagedSkillMetadata, content_digest},
    storage::{METADATA_FILE_NAME, SKILL_FILE_NAME, TransactionalSkillCreator},
};

struct TemporarySkillsDirectory {
    path: PathBuf,
}

impl TemporarySkillsDirectory {
    fn new() -> Self {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("rust-skgen-create-{unique}"));
        fs::create_dir(&path).unwrap();
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TemporarySkillsDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn metadata_for(content: &str) -> ManagedSkillMetadata {
    let source_url = SourceUrl::parse("https://docs.example.com/guide").unwrap();
    let discovery = DiscoveryConfiguration::new(
        DiscoveryScope::SameSite,
        Some(SiteBoundary::ExactHost),
        Vec::new(),
        TraversalMode::All,
        None,
        ContentFormat::GuideWithReferences,
    )
    .unwrap();

    ManagedSkillMetadata::new(source_url, discovery, content_digest(content))
}

#[test]
fn creates_a_complete_skill_at_the_requested_output_path() {
    let skills = TemporarySkillsDirectory::new();
    let name = SkillName::parse("radix-primitives").unwrap();
    let content = "# Radix Primitives\n\nUse the documented components.\n";
    let metadata = metadata_for(content);

    TransactionalSkillCreator::new(skills.path())
        .create(&name, content, &metadata)
        .unwrap();

    let skill_path = skills.path().join(name.as_str());
    assert_eq!(
        fs::read_to_string(skill_path.join(SKILL_FILE_NAME)).unwrap(),
        content
    );
    assert_eq!(
        fs::read_to_string(skill_path.join(METADATA_FILE_NAME)).unwrap(),
        metadata.to_json().unwrap()
    );
}

#[test]
fn fails_without_modifying_an_existing_destination() {
    let skills = TemporarySkillsDirectory::new();
    let name = SkillName::parse("existing-skill").unwrap();
    let destination = skills.path().join(name.as_str());
    fs::create_dir(&destination).unwrap();
    fs::write(destination.join(SKILL_FILE_NAME), "# Existing skill\n").unwrap();
    let content = "# Replacement skill\n";

    let result = TransactionalSkillCreator::new(skills.path()).create(
        &name,
        content,
        &metadata_for(content),
    );

    assert!(result.is_err());
    assert_eq!(
        fs::read_to_string(destination.join(SKILL_FILE_NAME)).unwrap(),
        "# Existing skill\n"
    );
    assert!(!destination.join(METADATA_FILE_NAME).exists());
}
