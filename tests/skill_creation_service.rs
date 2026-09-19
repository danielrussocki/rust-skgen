use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use rust_skgen::{
    domain::{DiscoveryConfiguration, SkillName, SourceUrl},
    fetch::{DocumentFetcher, FetchError, FetchedDocument},
    metadata::ManagedSkillMetadata,
    policy::{AccessPolicy, CombinedCrawlPolicy, CrawlPolicy, RobotsError},
    service::{CreateSkillRequest, create_skill},
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
        let path = std::env::temp_dir().join(format!("rust-skgen-service-create-{unique}"));
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

struct DenyRestrictedAccess;

impl AccessPolicy for DenyRestrictedAccess {
    fn allows(&self, url: &Url) -> bool {
        url.path() != "/restricted"
    }
}

#[test]
fn creates_a_skill_from_local_documentation_with_attributed_sources() {
    let skills = TemporarySkillsDirectory::new();
    let source_url = Url::parse("https://docs.example.test/start").unwrap();
    let related_url = Url::parse("https://docs.example.test/related").unwrap();
    let fetcher = LocalDocumentationFetcher {
        documents: BTreeMap::from([
            (
                source_url.clone(),
                "<main><p>Start documentation.</p><a href=\"/related\">Related</a></main>"
                    .to_owned(),
            ),
            (
                related_url.clone(),
                "<main><p>Related documentation.</p></main>".to_owned(),
            ),
        ]),
    };
    let name = SkillName::parse("local-docs").unwrap();
    let request = CreateSkillRequest::new(
        name.clone(),
        SourceUrl::parse(source_url.as_str()).unwrap(),
        DiscoveryConfiguration::default(),
    );

    let result = create_skill(
        request,
        &fetcher,
        &AllowAllPolicy,
        &TransactionalSkillCreator::new(skills.path()),
    )
    .unwrap();

    assert_eq!(result.name(), &name);
    let skill_path = skills.path().join(name.as_str());
    let content = fs::read_to_string(skill_path.join(SKILL_FILE_NAME)).unwrap();
    assert!(content.contains(&format!("Source: {source_url}")));
    assert!(content.contains(&format!("Source: {related_url}")));
    let metadata = ManagedSkillMetadata::from_json(
        &fs::read_to_string(skill_path.join(METADATA_FILE_NAME)).unwrap(),
    )
    .unwrap();
    assert_eq!(metadata.source_url().as_url(), &source_url);
    assert!(metadata.has_matching_content_digest(&content));
}

#[test]
fn does_not_publish_a_skill_when_access_conditions_forbid_a_related_page() {
    let skills = TemporarySkillsDirectory::new();
    let source_url = Url::parse("https://docs.example.test/start").unwrap();
    let name = SkillName::parse("restricted-docs").unwrap();
    let fetcher = LocalDocumentationFetcher {
        documents: BTreeMap::from([(
            source_url.clone(),
            "<main><p>Start documentation.</p><a href=\"/restricted\">Restricted</a></main>"
                .to_owned(),
        )]),
    };
    let request = CreateSkillRequest::new(
        name.clone(),
        SourceUrl::parse(source_url.as_str()).unwrap(),
        DiscoveryConfiguration::default(),
    );
    let policy = CombinedCrawlPolicy::new(AllowAllPolicy, DenyRestrictedAccess);

    let result = create_skill(
        request,
        &fetcher,
        &policy,
        &TransactionalSkillCreator::new(skills.path()),
    );

    assert!(result.is_err());
    assert!(!skills.path().join(name.as_str()).exists());
}
