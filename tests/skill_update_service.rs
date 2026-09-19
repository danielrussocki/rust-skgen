use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use rust_skgen::{
    domain::{
        ContentFormat, DiscoveryConfiguration, DiscoveryScope, SiteBoundary, SkillName, SourceUrl,
        TraversalMode,
    },
    fetch::{DocumentFetcher, FetchError, FetchedDocument},
    metadata::{ManagedSkillMetadata, content_digest},
    policy::{CrawlPolicy, RobotsError},
    service::{
        RebuildConfirmation, UpdateSkillError, UpdateSkillRequest, update_skill,
        update_skill_with_confirmation,
    },
    storage::{METADATA_FILE_NAME, SKILL_FILE_NAME, TransactionalSkillCreator},
};
use url::Url;

struct TemporarySkillsDirectory {
    path: PathBuf,
}

impl TemporarySkillsDirectory {
    fn new() -> Self {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("rust-skgen-service-update-{unique}"));
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

struct LocalDocumentationFetcher {
    documents: BTreeMap<Url, String>,
}

impl DocumentFetcher for LocalDocumentationFetcher {
    fn fetch(&self, url: Url) -> Result<FetchedDocument, FetchError> {
        Ok(FetchedDocument::new(
            url.clone(),
            200,
            self.documents[&url].clone(),
        ))
    }
}

struct AllowAllPolicy;

impl CrawlPolicy for AllowAllPolicy {
    fn allows(&self, _url: &Url) -> Result<bool, RobotsError> {
        Ok(true)
    }
}

struct Confirmation(Option<bool>);

impl RebuildConfirmation for Confirmation {
    fn confirm_rebuild(&self) -> Option<bool> {
        self.0
    }
}

fn initial_metadata() -> ManagedSkillMetadata {
    let source_url = SourceUrl::parse("https://docs.example.test/start").unwrap();
    let discovery = DiscoveryConfiguration::default();
    ManagedSkillMetadata::new(source_url, discovery, content_digest("# Previous skill\n"))
}

#[test]
fn updates_each_individual_configuration_field_and_persists_it() {
    let skills = TemporarySkillsDirectory::new();
    let current_name = SkillName::parse("old-docs").unwrap();
    let new_name = SkillName::parse("new-docs").unwrap();
    TransactionalSkillCreator::new(skills.path())
        .create(&current_name, "# Previous skill\n", &initial_metadata())
        .unwrap();

    let new_source = SourceUrl::parse("https://docs.example.test/reference/start").unwrap();
    let source_url = new_source.as_url().clone();
    let fetcher = LocalDocumentationFetcher {
        documents: BTreeMap::from([(
            source_url,
            "<main><p>Updated documentation.</p></main>".to_owned(),
        )]),
    };
    let request = UpdateSkillRequest::new(current_name.clone())
        .with_name(new_name.clone())
        .with_source_url(new_source.clone())
        .with_scope(DiscoveryScope::SameSite)
        .with_site_boundary(SiteBoundary::SameOrigin)
        .with_traversal_mode(TraversalMode::Limited)
        .with_max_pages(2)
        .with_content_format(ContentFormat::OrganizedContent);

    let result = update_skill(
        request,
        &fetcher,
        &AllowAllPolicy,
        &TransactionalSkillCreator::new(skills.path()),
    )
    .unwrap();

    assert_eq!(result.name(), &new_name);
    assert!(!skills.path().join(current_name.as_str()).exists());
    let metadata = ManagedSkillMetadata::from_json(
        &fs::read_to_string(
            skills
                .path()
                .join(new_name.as_str())
                .join(METADATA_FILE_NAME),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(metadata.source_url(), &new_source);
    assert_eq!(metadata.discovery().scope(), DiscoveryScope::SameSite);
    assert_eq!(
        metadata.discovery().site_boundary(),
        SiteBoundary::SameOrigin
    );
    assert_eq!(
        metadata.discovery().traversal_mode(),
        TraversalMode::Limited
    );
    assert_eq!(metadata.discovery().max_pages(), Some(2));
    assert_eq!(
        metadata.discovery().content_format(),
        ContentFormat::OrganizedContent
    );
}

#[test]
fn detects_missing_metadata_as_requiring_a_rebuild() {
    let skills = TemporarySkillsDirectory::new();
    let name = SkillName::parse("missing-metadata").unwrap();
    let skill_path = skills.path().join(name.as_str());
    fs::create_dir(&skill_path).unwrap();
    fs::write(skill_path.join(SKILL_FILE_NAME), "# Existing skill\n").unwrap();

    let result = update_skill(
        UpdateSkillRequest::new(name),
        &LocalDocumentationFetcher {
            documents: BTreeMap::new(),
        },
        &AllowAllPolicy,
        &TransactionalSkillCreator::new(skills.path()),
    );

    assert!(matches!(result, Err(UpdateSkillError::RebuildRequired(_))));
}

#[test]
fn detects_invalid_metadata_as_requiring_a_rebuild() {
    let skills = TemporarySkillsDirectory::new();
    let name = SkillName::parse("invalid-metadata").unwrap();
    let skill_path = skills.path().join(name.as_str());
    fs::create_dir(&skill_path).unwrap();
    fs::write(skill_path.join(SKILL_FILE_NAME), "# Existing skill\n").unwrap();
    fs::write(skill_path.join(METADATA_FILE_NAME), "not JSON").unwrap();

    let result = update_skill(
        UpdateSkillRequest::new(name),
        &LocalDocumentationFetcher {
            documents: BTreeMap::new(),
        },
        &AllowAllPolicy,
        &TransactionalSkillCreator::new(skills.path()),
    );

    assert!(matches!(result, Err(UpdateSkillError::RebuildRequired(_))));
}

#[test]
fn accepts_rebuild_confirmation_for_a_mismatched_content_digest() {
    let skills = TemporarySkillsDirectory::new();
    let name = SkillName::parse("changed-skill").unwrap();
    TransactionalSkillCreator::new(skills.path())
        .create(&name, "# Previous skill\n", &initial_metadata())
        .unwrap();
    fs::write(
        skills.path().join(name.as_str()).join(SKILL_FILE_NAME),
        "# Manually changed skill\n",
    )
    .unwrap();
    let source_url = SourceUrl::parse("https://docs.example.test/start").unwrap();
    let fetcher = LocalDocumentationFetcher {
        documents: BTreeMap::from([(
            source_url.as_url().clone(),
            "<main><p>Rebuilt documentation.</p></main>".to_owned(),
        )]),
    };

    let result = update_skill_with_confirmation(
        UpdateSkillRequest::new(name.clone()),
        &Confirmation(Some(true)),
        &fetcher,
        &AllowAllPolicy,
        &TransactionalSkillCreator::new(skills.path()),
    );

    assert_eq!(result.unwrap().name(), &name);
    assert_ne!(
        fs::read_to_string(skills.path().join(name.as_str()).join(SKILL_FILE_NAME)).unwrap(),
        "# Manually changed skill\n"
    );
}

#[test]
fn rejects_rebuild_confirmation_without_modifying_the_skill() {
    assert_rebuild_is_not_published(Confirmation(Some(false)));
}

#[test]
fn treats_an_absent_rebuild_response_as_rejected_without_modifying_the_skill() {
    assert_rebuild_is_not_published(Confirmation(None));
}

fn assert_rebuild_is_not_published(confirmation: Confirmation) {
    let skills = TemporarySkillsDirectory::new();
    let name = SkillName::parse("changed-skill").unwrap();
    TransactionalSkillCreator::new(skills.path())
        .create(&name, "# Previous skill\n", &initial_metadata())
        .unwrap();
    let skill_file = skills.path().join(name.as_str()).join(SKILL_FILE_NAME);
    let metadata_file = skills.path().join(name.as_str()).join(METADATA_FILE_NAME);
    fs::write(&skill_file, "# Manually changed skill\n").unwrap();
    let original_metadata = fs::read_to_string(&metadata_file).unwrap();

    let result = update_skill_with_confirmation(
        UpdateSkillRequest::new(name),
        &confirmation,
        &LocalDocumentationFetcher {
            documents: BTreeMap::new(),
        },
        &AllowAllPolicy,
        &TransactionalSkillCreator::new(skills.path()),
    );

    assert!(matches!(result, Err(UpdateSkillError::RebuildDeclined(_))));
    assert_eq!(
        fs::read_to_string(skill_file).unwrap(),
        "# Manually changed skill\n"
    );
    assert_eq!(
        fs::read_to_string(metadata_file).unwrap(),
        original_metadata
    );
}
