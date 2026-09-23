use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use rust_skgen::{
    domain::{
        AuthorizedHost, ContentFormat, DiscoveryConfiguration, DiscoveryScope, SiteBoundary,
        SkillName, SourceUrl, TraversalMode,
    },
    fetch::{DocumentFetcher, FetchError, FetchedDocument},
    metadata::{ManagedSkillMetadata, content_digest},
    policy::{CrawlPolicy, RobotsError},
    service::{
        RebuildConfirmation, UpdateSkillChanges, UpdateSkillError, UpdateSkillRequest,
        UpdateSkillsError, update_skill, update_skill_with_confirmation, update_skills,
        update_skills_with_changes,
    },
    storage::{METADATA_FILE_NAME, SKILL_FILE_NAME, SkillLocator, TransactionalSkillCreator},
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

impl DocumentFetcher for LocalDocumentationFetcher {
    fn fetch(&self, url: Url) -> Result<FetchedDocument, FetchError> {
        let Some(body) = self.documents.get(&url) else {
            return Err(FetchError::RequestLockPoisoned);
        };

        Ok(FetchedDocument::new(url.clone(), 200, body.clone()))
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
    metadata_for("https://docs.example.test/start")
}

fn metadata_for(source: &str) -> ManagedSkillMetadata {
    metadata_for_content(source, "# Previous skill\n")
}

fn metadata_for_content(source: &str, content: &str) -> ManagedSkillMetadata {
    let source_url = SourceUrl::parse(source).unwrap();
    let discovery = DiscoveryConfiguration::default();
    ManagedSkillMetadata::new(source_url, discovery, content_digest(content))
}

#[test]
fn updating_a_skill_publishes_an_unavailable_related_page_notice_after_not_found() {
    let skills = TemporarySkillsDirectory::new();
    let name = SkillName::parse("missing-related-docs").unwrap();
    let source_url = Url::parse("https://docs.example.test/start").unwrap();
    let missing_url = Url::parse("https://docs.example.test/missing").unwrap();
    let creator = TransactionalSkillCreator::new(skills.path());
    creator
        .create(&name, "# Previous skill\n", &initial_metadata())
        .unwrap();
    let fetcher = StatusDocumentationFetcher {
        documents: BTreeMap::from([
            (
                source_url.clone(),
                FetchedDocument::new(
                    source_url.clone(),
                    200,
                    "<main><p>Updated documentation.</p><a href=\"/missing\">Missing</a></main>"
                        .to_owned(),
                ),
            ),
            (
                missing_url.clone(),
                FetchedDocument::new(missing_url.clone(), 404, "missing".to_owned()),
            ),
        ]),
    };

    update_skill(
        UpdateSkillRequest::new(name.clone()),
        &fetcher,
        &AllowAllPolicy,
        &creator,
    )
    .expect("a related HTTP 404 should not prevent an update");

    let content =
        fs::read_to_string(skills.path().join(name.as_str()).join(SKILL_FILE_NAME)).unwrap();
    assert!(content.contains("Updated documentation."));
    assert!(content.contains(&missing_url.to_string()));
    assert!(content.contains("Documentation is unavailable for this source."));
}

#[test]
fn updating_a_skill_skips_a_related_non_html_response() {
    let skills = TemporarySkillsDirectory::new();
    let name = SkillName::parse("related-non-html-docs").unwrap();
    let source_url = Url::parse("https://docs.example.test/start").unwrap();
    let non_html_url = Url::parse("https://docs.example.test/data").unwrap();
    let related_url = Url::parse("https://docs.example.test/related").unwrap();
    let creator = TransactionalSkillCreator::new(skills.path());
    creator
        .create(&name, "# Previous skill\n", &initial_metadata())
        .unwrap();
    let fetcher = StatusDocumentationFetcher {
        documents: BTreeMap::from([
            (
                source_url.clone(),
                FetchedDocument::new(
                    source_url.clone(),
                    200,
                    "<main><p>Updated documentation.</p><a href=\"/data\">Data</a><a href=\"/related\">Related</a></main>".to_owned(),
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

    update_skill(
        UpdateSkillRequest::new(name.clone()),
        &fetcher,
        &AllowAllPolicy,
        &creator,
    )
    .expect("a related non-HTML response should not prevent an update");

    let content =
        fs::read_to_string(skills.path().join(name.as_str()).join(SKILL_FILE_NAME)).unwrap();
    assert!(content.contains("Updated documentation."));
    assert!(content.contains("Related documentation."));
    assert!(!content.contains(&non_html_url.to_string()));
}

#[test]
fn updating_a_skill_skips_a_related_redirect() {
    let skills = TemporarySkillsDirectory::new();
    let name = SkillName::parse("related-redirect-docs").unwrap();
    let source_url = Url::parse("https://docs.example.test/start").unwrap();
    let redirect_url = Url::parse("https://docs.example.test/redirect").unwrap();
    let related_url = Url::parse("https://docs.example.test/related").unwrap();
    let creator = TransactionalSkillCreator::new(skills.path());
    creator
        .create(&name, "# Previous skill\n", &initial_metadata())
        .unwrap();
    let fetcher = StatusDocumentationFetcher {
        documents: BTreeMap::from([
            (
                source_url.clone(),
                FetchedDocument::new(
                    source_url.clone(),
                    200,
                    "<main><p>Updated documentation.</p><a href=\"/redirect\">Redirect</a><a href=\"/related\">Related</a></main>".to_owned(),
                ),
            ),
            (
                redirect_url.clone(),
                FetchedDocument::new(redirect_url.clone(), 302, String::new()),
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

    update_skill(
        UpdateSkillRequest::new(name.clone()),
        &fetcher,
        &AllowAllPolicy,
        &creator,
    )
    .expect("a related redirect should not prevent an update");

    let content =
        fs::read_to_string(skills.path().join(name.as_str()).join(SKILL_FILE_NAME)).unwrap();
    assert!(content.contains("Updated documentation."));
    assert!(content.contains("Related documentation."));
    assert!(!content.contains(&redirect_url.to_string()));
}

#[test]
fn updating_all_managed_skills_reports_an_empty_result_when_none_exist() {
    let skills = TemporarySkillsDirectory::new();

    let result = update_skills(
        None,
        &SkillLocator::new(skills.path()),
        &LocalDocumentationFetcher {
            documents: BTreeMap::new(),
        },
        &AllowAllPolicy,
        &TransactionalSkillCreator::new(skills.path()),
    )
    .unwrap();

    assert!(result.outcomes().is_empty());
}

#[test]
fn updates_a_directed_mixed_selection_and_reports_each_result() {
    let skills = TemporarySkillsDirectory::new();
    let managed = SkillName::parse("managed-skill").unwrap();
    let missing = SkillName::parse("missing-skill").unwrap();
    TransactionalSkillCreator::new(skills.path())
        .create(&managed, "# Previous skill\n", &initial_metadata())
        .unwrap();
    let source_url = SourceUrl::parse("https://docs.example.test/start").unwrap();

    let result = update_skills(
        Some(&[managed.clone(), missing.clone()]),
        &SkillLocator::new(skills.path()),
        &LocalDocumentationFetcher {
            documents: BTreeMap::from([(
                source_url.as_url().clone(),
                "<main><p>Updated documentation.</p></main>".to_owned(),
            )]),
        },
        &AllowAllPolicy,
        &TransactionalSkillCreator::new(skills.path()),
    )
    .unwrap();

    assert_eq!(result.outcomes().len(), 2);
    assert_eq!(result.outcomes()[0].name(), &managed);
    assert!(result.outcomes()[0].result().is_ok());
    assert_eq!(result.outcomes()[1].name(), &missing);
    assert!(matches!(
        result.outcomes()[1].result(),
        Err(UpdateSkillError::SkillNotFound(name)) if name == &missing
    ));
}

#[test]
fn rejects_configuration_changes_for_multiple_selected_skills_without_modifying_them() {
    let skills = TemporarySkillsDirectory::new();
    let first = SkillName::parse("first-skill").unwrap();
    let second = SkillName::parse("second-skill").unwrap();
    let creator = TransactionalSkillCreator::new(skills.path());
    creator
        .create(&first, "# First skill\n", &initial_metadata())
        .unwrap();
    creator
        .create(&second, "# Second skill\n", &initial_metadata())
        .unwrap();

    let result = update_skills_with_changes(
        &[first.clone(), second.clone()],
        UpdateSkillChanges::default().with_content_format(ContentFormat::OrganizedContent),
        &LocalDocumentationFetcher {
            documents: BTreeMap::new(),
        },
        &AllowAllPolicy,
        &creator,
    );

    assert!(matches!(
        result,
        Err(UpdateSkillsError::ConfigurationChangesRequireSingleSelection)
    ));
    assert_eq!(
        fs::read_to_string(skills.path().join(first.as_str()).join(SKILL_FILE_NAME)).unwrap(),
        "# First skill\n"
    );
    assert_eq!(
        fs::read_to_string(skills.path().join(second.as_str()).join(SKILL_FILE_NAME)).unwrap(),
        "# Second skill\n"
    );
}

#[test]
fn continues_after_individual_failures_and_preserves_the_failed_skill() {
    let skills = TemporarySkillsDirectory::new();
    let failing = SkillName::parse("failing-skill").unwrap();
    let succeeding = SkillName::parse("succeeding-skill").unwrap();
    let creator = TransactionalSkillCreator::new(skills.path());
    creator
        .create(
            &failing,
            "# Failing skill\n",
            &metadata_for_content("https://docs.example.test/failing", "# Failing skill\n"),
        )
        .unwrap();
    creator
        .create(
            &succeeding,
            "# Succeeding skill\n",
            &metadata_for_content(
                "https://docs.example.test/succeeding",
                "# Succeeding skill\n",
            ),
        )
        .unwrap();
    let source_url = SourceUrl::parse("https://docs.example.test/succeeding").unwrap();

    let result = update_skills(
        Some(&[failing.clone(), succeeding.clone()]),
        &SkillLocator::new(skills.path()),
        &LocalDocumentationFetcher {
            documents: BTreeMap::from([(
                source_url.as_url().clone(),
                "<main><p>Updated documentation.</p></main>".to_owned(),
            )]),
        },
        &AllowAllPolicy,
        &creator,
    )
    .unwrap();

    assert!(matches!(
        result.outcomes()[0].result(),
        Err(UpdateSkillError::Discovery(_))
    ));
    assert!(result.outcomes()[1].result().is_ok());
    assert_eq!(
        fs::read_to_string(skills.path().join(failing.as_str()).join(SKILL_FILE_NAME)).unwrap(),
        "# Failing skill\n"
    );
    assert_ne!(
        fs::read_to_string(
            skills
                .path()
                .join(succeeding.as_str())
                .join(SKILL_FILE_NAME)
        )
        .unwrap(),
        "# Succeeding skill\n"
    );
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
fn persists_authorized_subdomains_in_an_individual_update() {
    let skills = TemporarySkillsDirectory::new();
    let name = SkillName::parse("managed-docs").unwrap();
    TransactionalSkillCreator::new(skills.path())
        .create(&name, "# Previous skill\n", &initial_metadata())
        .unwrap();
    let source_url = SourceUrl::parse("https://docs.example.test/start").unwrap();
    let authorized = AuthorizedHost::new("api.docs.example.test".to_owned());

    update_skill(
        UpdateSkillRequest::new(name.clone())
            .with_site_boundary(SiteBoundary::BaseDomain)
            .with_authorized_subdomains(vec![authorized.clone()]),
        &LocalDocumentationFetcher {
            documents: BTreeMap::from([(
                source_url.as_url().clone(),
                "<main><p>Updated documentation.</p></main>".to_owned(),
            )]),
        },
        &AllowAllPolicy,
        &TransactionalSkillCreator::new(skills.path()),
    )
    .unwrap();

    let metadata = ManagedSkillMetadata::from_json(
        &fs::read_to_string(skills.path().join(name.as_str()).join(METADATA_FILE_NAME)).unwrap(),
    )
    .unwrap();
    assert_eq!(
        metadata.discovery().site_boundary(),
        SiteBoundary::BaseDomain
    );
    assert_eq!(metadata.discovery().authorized_subdomains(), &[authorized]);
}

#[test]
fn preserves_the_current_skill_when_a_renamed_update_fails_before_publication() {
    let skills = TemporarySkillsDirectory::new();
    let current_name = SkillName::parse("current-docs").unwrap();
    let new_name = SkillName::parse("renamed-docs").unwrap();
    let creator = TransactionalSkillCreator::new(skills.path());
    creator
        .create(&current_name, "# Previous skill\n", &initial_metadata())
        .unwrap();

    let result = update_skill(
        UpdateSkillRequest::new(current_name.clone()).with_name(new_name.clone()),
        &LocalDocumentationFetcher {
            documents: BTreeMap::new(),
        },
        &AllowAllPolicy,
        &creator,
    );

    assert!(matches!(result, Err(UpdateSkillError::Discovery(_))));
    assert_eq!(
        fs::read_to_string(
            skills
                .path()
                .join(current_name.as_str())
                .join(SKILL_FILE_NAME)
        )
        .unwrap(),
        "# Previous skill\n"
    );
    assert!(!skills.path().join(new_name.as_str()).exists());
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
fn preserves_a_skill_when_an_update_receives_a_non_html_response() {
    let skills = TemporarySkillsDirectory::new();
    let name = SkillName::parse("json-update").unwrap();
    let creator = TransactionalSkillCreator::new(skills.path());
    creator
        .create(&name, "# Previous skill\n", &initial_metadata())
        .unwrap();
    let skill_file = skills.path().join(name.as_str()).join(SKILL_FILE_NAME);
    let metadata_file = skills.path().join(name.as_str()).join(METADATA_FILE_NAME);
    let original_metadata = fs::read_to_string(&metadata_file).unwrap();

    let result = update_skill(
        UpdateSkillRequest::new(name),
        &NonHtmlDocumentationFetcher,
        &AllowAllPolicy,
        &creator,
    );

    assert!(result.is_err());
    assert_eq!(
        fs::read_to_string(skill_file).unwrap(),
        "# Previous skill\n"
    );
    assert_eq!(
        fs::read_to_string(metadata_file).unwrap(),
        original_metadata
    );
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
fn mismatched_content_digest_requires_confirmation_before_rebuilding() {
    for confirmation in [Confirmation(Some(false)), Confirmation(None)] {
        let skills = TemporarySkillsDirectory::new();
        let name = SkillName::parse("changed-skill").unwrap();
        let creator = TransactionalSkillCreator::new(skills.path());
        creator
            .create(&name, "# Previous skill\n", &initial_metadata())
            .unwrap();
        let skill_file = skills.path().join(name.as_str()).join(SKILL_FILE_NAME);
        fs::write(&skill_file, "# Manually changed skill\n").unwrap();

        let result = update_skill_with_confirmation(
            UpdateSkillRequest::new(name),
            &confirmation,
            &LocalDocumentationFetcher {
                documents: BTreeMap::new(),
            },
            &AllowAllPolicy,
            &creator,
        );

        assert!(matches!(result, Err(UpdateSkillError::RebuildDeclined(_))));
        assert_eq!(
            fs::read_to_string(skill_file).unwrap(),
            "# Manually changed skill\n"
        );
    }
}

#[test]
fn updates_a_manually_edited_skill_after_rebuild_confirmation() {
    let skills = TemporarySkillsDirectory::new();
    let name = SkillName::parse("manually-edited-skill").unwrap();
    let creator = TransactionalSkillCreator::new(skills.path());
    creator
        .create(&name, "# Previous skill\n", &initial_metadata())
        .unwrap();
    let skill_file = skills.path().join(name.as_str()).join(SKILL_FILE_NAME);
    fs::write(&skill_file, "# Manually changed skill\n").unwrap();
    let source_url = SourceUrl::parse("https://docs.example.test/start").unwrap();

    let result = update_skill_with_confirmation(
        UpdateSkillRequest::new(name.clone()),
        &Confirmation(Some(true)),
        &LocalDocumentationFetcher {
            documents: BTreeMap::from([(
                source_url.as_url().clone(),
                "<main><p>Updated documentation.</p></main>".to_owned(),
            )]),
        },
        &AllowAllPolicy,
        &creator,
    );

    assert_eq!(result.unwrap().name(), &name);
    let content = fs::read_to_string(&skill_file).unwrap();
    assert_ne!(content, "# Manually changed skill\n");
    let metadata = ManagedSkillMetadata::from_json(
        &fs::read_to_string(skills.path().join(name.as_str()).join(METADATA_FILE_NAME)).unwrap(),
    )
    .unwrap();
    assert!(metadata.has_matching_content_digest(&content));
}

#[test]
fn rejects_rebuild_confirmation_without_modifying_the_skill() {
    assert_rebuild_is_not_published(Confirmation(Some(false)), false);
}

#[test]
fn treats_an_absent_rebuild_response_as_rejected_without_modifying_the_skill() {
    assert_rebuild_is_not_published(Confirmation(None), true);
}

#[test]
fn rebuilds_missing_metadata_after_confirmation_using_the_requested_configuration() {
    assert_rebuild_is_published(false);
}

#[test]
fn rebuilds_invalid_metadata_after_confirmation_using_the_requested_configuration() {
    assert_rebuild_is_published(true);
}

fn assert_rebuild_is_published(invalid_metadata: bool) {
    let skills = TemporarySkillsDirectory::new();
    let name = SkillName::parse("rebuildable-skill").unwrap();
    let skill_directory = skills.path().join(name.as_str());
    fs::create_dir(&skill_directory).unwrap();
    fs::write(skill_directory.join(SKILL_FILE_NAME), "# Existing skill\n").unwrap();
    if invalid_metadata {
        fs::write(skill_directory.join(METADATA_FILE_NAME), "not JSON").unwrap();
    }
    let source_url = SourceUrl::parse("https://docs.example.test/rebuilt").unwrap();

    let result = update_skill_with_confirmation(
        UpdateSkillRequest::new(name.clone())
            .with_source_url(source_url.clone())
            .with_content_format(ContentFormat::OrganizedContent),
        &Confirmation(Some(true)),
        &LocalDocumentationFetcher {
            documents: BTreeMap::from([(
                source_url.as_url().clone(),
                "<main><p>Rebuilt documentation.</p></main>".to_owned(),
            )]),
        },
        &AllowAllPolicy,
        &TransactionalSkillCreator::new(skills.path()),
    );

    assert_eq!(result.unwrap().name(), &name);
    let content = fs::read_to_string(skill_directory.join(SKILL_FILE_NAME)).unwrap();
    let metadata = ManagedSkillMetadata::from_json(
        &fs::read_to_string(skill_directory.join(METADATA_FILE_NAME)).unwrap(),
    )
    .unwrap();
    assert_eq!(metadata.source_url(), &source_url);
    assert_eq!(
        metadata.discovery().content_format(),
        ContentFormat::OrganizedContent
    );
    assert!(metadata.has_matching_content_digest(&content));
}

fn assert_rebuild_is_not_published(confirmation: Confirmation, invalid_metadata: bool) {
    let skills = TemporarySkillsDirectory::new();
    let name = SkillName::parse("unmanaged-skill").unwrap();
    let skill_directory = skills.path().join(name.as_str());
    fs::create_dir(&skill_directory).unwrap();
    let skill_file = skills.path().join(name.as_str()).join(SKILL_FILE_NAME);
    let metadata_file = skills.path().join(name.as_str()).join(METADATA_FILE_NAME);
    fs::write(&skill_file, "# Existing skill\n").unwrap();
    if invalid_metadata {
        fs::write(&metadata_file, "not JSON").unwrap();
    }
    let original_metadata = fs::read_to_string(&metadata_file).ok();

    let result = update_skill_with_confirmation(
        UpdateSkillRequest::new(name)
            .with_source_url(SourceUrl::parse("https://docs.example.test/rebuilt").unwrap()),
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
        "# Existing skill\n"
    );
    assert_eq!(fs::read_to_string(metadata_file).ok(), original_metadata);
}
