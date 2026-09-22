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
        Ok(self.documents.get(&url).map_or_else(
            || {
                FetchedDocument::new_with_content_type(
                    url.clone(),
                    404,
                    String::new(),
                    "text/plain".to_owned(),
                )
            },
            |body| FetchedDocument::new(url.clone(), 200, body.clone()),
        ))
    }
}

struct NonHtmlDocumentationFetcher;

impl DocumentFetcher for NonHtmlDocumentationFetcher {
    fn fetch(&self, url: Url) -> Result<FetchedDocument, FetchError> {
        Ok(FetchedDocument::new_with_content_type(
            url,
            200,
            "{\"message\":\"not documentation\"}".to_owned(),
            "application/json".to_owned(),
        ))
    }
}

struct StatusDocumentationFetcher {
    documents: BTreeMap<Url, FetchedDocument>,
}

impl DocumentFetcher for StatusDocumentationFetcher {
    fn fetch(&self, url: Url) -> Result<FetchedDocument, FetchError> {
        Ok(self.documents.get(&url).cloned().unwrap_or_else(|| {
            FetchedDocument::new_with_content_type(url, 404, String::new(), "text/plain".to_owned())
        }))
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
fn creates_a_complete_skill_when_a_related_page_returns_not_found() {
    let skills = TemporarySkillsDirectory::new();
    let source_url = Url::parse("https://docs.example.test/start").unwrap();
    let missing_url = Url::parse("https://docs.example.test/missing").unwrap();
    let name = SkillName::parse("missing-related-docs").unwrap();
    let fetcher = StatusDocumentationFetcher {
        documents: BTreeMap::from([
            (
                source_url.clone(),
                FetchedDocument::new(
                    source_url.clone(),
                    200,
                    "<main><p>Start documentation.</p><a href=\"/missing\">Missing</a></main>"
                        .to_owned(),
                ),
            ),
            (
                missing_url.clone(),
                FetchedDocument::new(missing_url.clone(), 404, "missing".to_owned()),
            ),
        ]),
    };
    let request = CreateSkillRequest::new(
        name.clone(),
        SourceUrl::parse(source_url.as_str()).unwrap(),
        DiscoveryConfiguration::default(),
    );

    create_skill(
        request,
        &fetcher,
        &AllowAllPolicy,
        &TransactionalSkillCreator::new(skills.path()),
    )
    .expect("a related HTTP 404 should still publish a complete skill");

    let content =
        fs::read_to_string(skills.path().join(name.as_str()).join(SKILL_FILE_NAME)).unwrap();
    assert!(content.contains("Start documentation."));
    assert!(content.contains(&missing_url.to_string()));
    assert!(content.contains("Documentation is unavailable for this source."));
}

#[test]
fn creates_a_skill_while_skipping_a_related_non_html_response() {
    let skills = TemporarySkillsDirectory::new();
    let source_url = Url::parse("https://docs.example.test/start").unwrap();
    let non_html_url = Url::parse("https://docs.example.test/data").unwrap();
    let related_url = Url::parse("https://docs.example.test/related").unwrap();
    let name = SkillName::parse("related-non-html-docs").unwrap();
    let fetcher = StatusDocumentationFetcher {
        documents: BTreeMap::from([
            (
                source_url.clone(),
                FetchedDocument::new(
                    source_url.clone(),
                    200,
                    "<main><p>Start documentation.</p><a href=\"/data\">Data</a><a href=\"/related\">Related</a></main>".to_owned(),
                ),
            ),
            (
                non_html_url.clone(),
                FetchedDocument::new_with_content_type(
                    non_html_url.clone(),
                    200,
                    "{\"version\":1}".to_owned(),
                    "application/json".to_owned(),
                ),
            ),
            (
                related_url.clone(),
                FetchedDocument::new(
                    related_url.clone(),
                    200,
                    "<main><p>Related documentation.</p></main>".to_owned(),
                ),
            ),
        ]),
    };
    let request = CreateSkillRequest::new(
        name.clone(),
        SourceUrl::parse(source_url.as_str()).unwrap(),
        DiscoveryConfiguration::default(),
    );

    create_skill(
        request,
        &fetcher,
        &AllowAllPolicy,
        &TransactionalSkillCreator::new(skills.path()),
    )
    .expect("a related non-HTML response should not prevent skill creation");

    let content =
        fs::read_to_string(skills.path().join(name.as_str()).join(SKILL_FILE_NAME)).unwrap();
    assert!(content.contains("Start documentation."));
    assert!(content.contains("Related documentation."));
    assert!(!content.contains(&non_html_url.to_string()));
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

#[test]
fn does_not_publish_a_skill_from_a_non_html_response() {
    let skills = TemporarySkillsDirectory::new();
    let source_url = SourceUrl::parse("https://docs.example.test/start").unwrap();
    let name = SkillName::parse("json-docs").unwrap();
    let request =
        CreateSkillRequest::new(name.clone(), source_url, DiscoveryConfiguration::default());

    let result = create_skill(
        request,
        &NonHtmlDocumentationFetcher,
        &AllowAllPolicy,
        &TransactionalSkillCreator::new(skills.path()),
    );

    assert!(result.is_err());
    assert!(!skills.path().join(name.as_str()).exists());
}
