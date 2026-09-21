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
        let path = std::env::temp_dir().join(format!("rust-skgen-replace-{unique}"));
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
        false,
    )
    .unwrap();

    ManagedSkillMetadata::new(source_url, discovery, content_digest(content))
}

fn create_skill(skills: &TemporarySkillsDirectory, name: &SkillName, content: &str) {
    TransactionalSkillCreator::new(skills.path())
        .create(name, content, &metadata_for(content))
        .unwrap();
}

#[test]
fn replaces_an_existing_skill_after_writing_the_complete_replacement() {
    let skills = TemporarySkillsDirectory::new();
    let name = SkillName::parse("radix-primitives").unwrap();
    create_skill(&skills, &name, "# Previous skill\n");
    let replacement = "# Replacement skill\n";

    TransactionalSkillCreator::new(skills.path())
        .replace(&name, &name, replacement, &metadata_for(replacement))
        .unwrap();

    let path = skills.path().join(name.as_str());
    assert_eq!(
        fs::read_to_string(path.join(SKILL_FILE_NAME)).unwrap(),
        replacement
    );
    assert_eq!(
        fs::read_to_string(path.join(METADATA_FILE_NAME)).unwrap(),
        metadata_for(replacement).to_json().unwrap()
    );
}

#[test]
fn renames_an_existing_skill_when_the_new_name_is_available() {
    let skills = TemporarySkillsDirectory::new();
    let old_name = SkillName::parse("radix-primitives").unwrap();
    let new_name = SkillName::parse("radix-docs").unwrap();
    create_skill(&skills, &old_name, "# Previous skill\n");
    let replacement = "# Renamed skill\n";

    TransactionalSkillCreator::new(skills.path())
        .replace(
            &old_name,
            &new_name,
            replacement,
            &metadata_for(replacement),
        )
        .unwrap();

    assert!(!skills.path().join(old_name.as_str()).exists());
    assert_eq!(
        fs::read_to_string(skills.path().join(new_name.as_str()).join(SKILL_FILE_NAME)).unwrap(),
        replacement
    );
}

#[test]
fn rejects_a_rename_to_an_existing_name_without_changing_either_skill() {
    let skills = TemporarySkillsDirectory::new();
    let old_name = SkillName::parse("radix-primitives").unwrap();
    let new_name = SkillName::parse("radix-docs").unwrap();
    create_skill(&skills, &old_name, "# Previous skill\n");
    create_skill(&skills, &new_name, "# Other skill\n");

    let result = TransactionalSkillCreator::new(skills.path()).replace(
        &old_name,
        &new_name,
        "# Replacement skill\n",
        &metadata_for("# Replacement skill\n"),
    );

    assert!(result.is_err());
    assert_eq!(
        fs::read_to_string(skills.path().join(old_name.as_str()).join(SKILL_FILE_NAME)).unwrap(),
        "# Previous skill\n"
    );
    assert_eq!(
        fs::read_to_string(skills.path().join(new_name.as_str()).join(SKILL_FILE_NAME)).unwrap(),
        "# Other skill\n"
    );
}

#[test]
fn rejects_a_second_simultaneous_operation_for_the_same_skill() {
    let skills = TemporarySkillsDirectory::new();
    let name = SkillName::parse("radix-primitives").unwrap();
    create_skill(&skills, &name, "# Previous skill\n");
    let storage = TransactionalSkillCreator::new(skills.path());
    let _lock = storage.lock(&name).unwrap();

    let result = storage.replace(
        &name,
        &name,
        "# Replacement skill\n",
        &metadata_for("# Replacement skill\n"),
    );

    assert!(result.is_err());
    assert_eq!(
        fs::read_to_string(skills.path().join(name.as_str()).join(SKILL_FILE_NAME)).unwrap(),
        "# Previous skill\n"
    );
}
