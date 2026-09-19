//! Application orchestration boundary.

use crate::{
    discover::{DiscoveryError, discover_all},
    domain::{ContentFormat, DiscoveryConfiguration, SkillName, SourceUrl},
    fetch::DocumentFetcher,
    metadata::{ManagedSkillMetadata, content_digest},
    policy::CrawlPolicy,
    render::{render_guide_with_references, render_organized_content},
    storage::{StorageError, TransactionalSkillCreator},
};

/// Marks a component that orchestrates skill operations.
pub trait SkillService {}

/// Validated input required to create one managed skill.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CreateSkillRequest {
    name: SkillName,
    source_url: SourceUrl,
    discovery: DiscoveryConfiguration,
}

impl CreateSkillRequest {
    /// Creates a request from configuration validated at the application boundary.
    pub fn new(name: SkillName, source_url: SourceUrl, discovery: DiscoveryConfiguration) -> Self {
        Self {
            name,
            source_url,
            discovery,
        }
    }
}

/// Successful result of creating one managed skill.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CreateSkillResult {
    name: SkillName,
}

impl CreateSkillResult {
    /// Returns the name of the published skill.
    pub fn name(&self) -> &SkillName {
        &self.name
    }
}

/// Error returned while building and publishing a skill.
#[derive(Debug)]
pub enum CreateSkillError {
    /// Documentation discovery could not produce a complete skill input.
    Discovery(DiscoveryError),
    /// The complete skill could not be locked or published.
    Storage(StorageError),
}

impl std::fmt::Display for CreateSkillError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Discovery(error) => {
                write!(formatter, "failed to discover documentation: {error}")
            }
            Self::Storage(error) => write!(formatter, "failed to publish skill: {error}"),
        }
    }
}

impl std::error::Error for CreateSkillError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Discovery(error) => Some(error),
            Self::Storage(error) => Some(error),
        }
    }
}

/// Discovers, renders, and transactionally publishes a new managed skill.
pub fn create_skill<F: DocumentFetcher, P: CrawlPolicy>(
    request: CreateSkillRequest,
    fetcher: &F,
    policy: &P,
    publisher: &TransactionalSkillCreator,
) -> Result<CreateSkillResult, CreateSkillError> {
    let _lock = publisher
        .lock(&request.name)
        .map_err(CreateSkillError::Storage)?;
    let pages = discover_all(
        request.source_url.as_url().clone(),
        &request.discovery,
        fetcher,
        policy,
    )
    .map_err(CreateSkillError::Discovery)?;
    let content = match request.discovery.content_format() {
        ContentFormat::GuideWithReferences => render_guide_with_references(&pages),
        ContentFormat::OrganizedContent => render_organized_content(&pages),
    };
    let metadata = ManagedSkillMetadata::new(
        request.source_url,
        request.discovery,
        content_digest(&content),
    );
    publisher
        .create(&request.name, &content, &metadata)
        .map_err(CreateSkillError::Storage)?;

    Ok(CreateSkillResult { name: request.name })
}
