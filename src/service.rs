//! Application orchestration boundary.

use crate::{
    discover::{DiscoveryError, discover_all},
    domain::{
        AuthorizedHost, ContentFormat, DiscoveryConfiguration, DiscoveryConfigurationError,
        DiscoveryScope, SiteBoundary, SkillName, SourceUrl, TraversalMode,
    },
    fetch::DocumentFetcher,
    metadata::{ManagedSkillMetadata, content_digest},
    policy::CrawlPolicy,
    render::{render_guide_with_references, render_organized_content},
    storage::{ManagedSkillStatus, SkillLocator, StorageError, TransactionalSkillCreator},
};

/// Marks a component that orchestrates skill operations.
pub trait SkillService {}

/// Requests approval before overwriting a skill whose management state must be rebuilt.
pub trait RebuildConfirmation {
    /// Returns approval, rejection, or no response from the caller.
    fn confirm_rebuild(&self) -> Option<bool>;
}

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

/// Validated partial configuration changes for one managed skill.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UpdateSkillRequest {
    current_name: SkillName,
    new_name: Option<SkillName>,
    source_url: Option<SourceUrl>,
    scope: Option<DiscoveryScope>,
    site_boundary: Option<SiteBoundary>,
    authorized_subdomains: Option<Vec<AuthorizedHost>>,
    traversal_mode: Option<TraversalMode>,
    max_pages: Option<usize>,
    content_format: Option<ContentFormat>,
}

impl UpdateSkillRequest {
    /// Starts an update for the managed skill selected by its current name.
    pub fn new(current_name: SkillName) -> Self {
        Self {
            current_name,
            new_name: None,
            source_url: None,
            scope: None,
            site_boundary: None,
            authorized_subdomains: None,
            traversal_mode: None,
            max_pages: None,
            content_format: None,
        }
    }

    /// Changes the published skill name.
    pub fn with_name(mut self, name: SkillName) -> Self {
        self.new_name = Some(name);
        self
    }

    /// Changes the documentation source URL.
    pub fn with_source_url(mut self, source_url: SourceUrl) -> Self {
        self.source_url = Some(source_url);
        self
    }

    /// Changes the discovery scope.
    pub fn with_scope(mut self, scope: DiscoveryScope) -> Self {
        self.scope = Some(scope);
        self
    }

    /// Changes the same-site boundary.
    pub fn with_site_boundary(mut self, site_boundary: SiteBoundary) -> Self {
        self.site_boundary = Some(site_boundary);
        self
    }

    /// Replaces the explicitly authorized base-domain subdomains.
    pub fn with_authorized_subdomains(
        mut self,
        authorized_subdomains: Vec<AuthorizedHost>,
    ) -> Self {
        self.authorized_subdomains = Some(authorized_subdomains);
        self
    }

    /// Changes the traversal mode.
    pub fn with_traversal_mode(mut self, traversal_mode: TraversalMode) -> Self {
        self.traversal_mode = Some(traversal_mode);
        self
    }

    /// Changes the maximum page count for limited traversal.
    pub fn with_max_pages(mut self, max_pages: usize) -> Self {
        self.max_pages = Some(max_pages);
        self
    }

    /// Changes the generated content format.
    pub fn with_content_format(mut self, content_format: ContentFormat) -> Self {
        self.content_format = Some(content_format);
        self
    }
}

/// Successful result of updating one managed skill.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UpdateSkillResult {
    name: SkillName,
}

impl UpdateSkillResult {
    /// Returns the name under which the updated skill was published.
    pub fn name(&self) -> &SkillName {
        &self.name
    }
}

/// Error returned while rebuilding and replacing one managed skill.
#[derive(Debug)]
pub enum UpdateSkillError {
    /// The selected skill directory does not exist.
    SkillNotFound(SkillName),
    /// The selected skill has no valid generator metadata.
    SkillNotManaged(SkillName),
    /// Metadata is absent, invalid, or does not match the managed content.
    RebuildRequired(SkillName),
    /// The user rejected rebuilding the management state or supplied no response.
    RebuildDeclined(SkillName),
    /// The requested changes produce incompatible discovery settings.
    InvalidDiscoveryConfiguration(DiscoveryConfigurationError),
    /// Documentation discovery could not produce a complete skill input.
    Discovery(DiscoveryError),
    /// The complete skill could not be read, locked, or published.
    Storage(StorageError),
}

impl std::fmt::Display for UpdateSkillError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SkillNotFound(name) => {
                write!(formatter, "skill does not exist: {}", name.as_str())
            }
            Self::SkillNotManaged(name) => {
                write!(formatter, "skill is not managed: {}", name.as_str())
            }
            Self::RebuildRequired(name) => {
                write!(
                    formatter,
                    "skill requires metadata reconstruction: {}",
                    name.as_str()
                )
            }
            Self::RebuildDeclined(name) => {
                write!(
                    formatter,
                    "metadata reconstruction was not confirmed: {}",
                    name.as_str()
                )
            }
            Self::InvalidDiscoveryConfiguration(error) => {
                write!(formatter, "invalid update configuration: {error}")
            }
            Self::Discovery(error) => {
                write!(formatter, "failed to discover documentation: {error}")
            }
            Self::Storage(error) => write!(formatter, "failed to replace skill: {error}"),
        }
    }
}

impl std::error::Error for UpdateSkillError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::InvalidDiscoveryConfiguration(error) => Some(error),
            Self::Discovery(error) => Some(error),
            Self::Storage(error) => Some(error),
            Self::SkillNotFound(_)
            | Self::SkillNotManaged(_)
            | Self::RebuildRequired(_)
            | Self::RebuildDeclined(_) => None,
        }
    }
}

/// The per-skill outcome of a batch update.
#[derive(Debug)]
pub struct UpdateSkillsOutcome {
    name: SkillName,
    result: Result<UpdateSkillResult, UpdateSkillError>,
}

impl UpdateSkillsOutcome {
    /// Returns the selected skill name.
    pub fn name(&self) -> &SkillName {
        &self.name
    }

    /// Returns the individual update outcome.
    pub fn result(&self) -> &Result<UpdateSkillResult, UpdateSkillError> {
        &self.result
    }
}

/// Results collected while updating several skills independently.
#[derive(Debug)]
pub struct UpdateSkillsResult {
    outcomes: Vec<UpdateSkillsOutcome>,
}

impl UpdateSkillsResult {
    /// Returns results in selection order, or stable skill-name order when updating all skills.
    pub fn outcomes(&self) -> &[UpdateSkillsOutcome] {
        &self.outcomes
    }
}

/// Updates every managed skill or an explicitly directed selection.
///
/// Each skill is rebuilt independently, so one failure does not prevent later selections.
pub fn update_skills<F: DocumentFetcher, P: CrawlPolicy>(
    selection: Option<&[SkillName]>,
    locator: &SkillLocator,
    fetcher: &F,
    policy: &P,
    publisher: &TransactionalSkillCreator,
) -> Result<UpdateSkillsResult, StorageError> {
    let names = match selection {
        Some(names) => names.to_vec(),
        None => locator
            .locate()?
            .into_iter()
            .filter(|skill| skill.is_managed())
            .map(|skill| skill.name().clone())
            .collect(),
    };
    let outcomes = names
        .into_iter()
        .map(|name| UpdateSkillsOutcome {
            result: update_skill(
                UpdateSkillRequest::new(name.clone()),
                fetcher,
                policy,
                publisher,
            ),
            name,
        })
        .collect();

    Ok(UpdateSkillsResult { outcomes })
}

/// Rebuilds one managed skill using its persisted settings plus the requested changes.
pub fn update_skill<F: DocumentFetcher, P: CrawlPolicy>(
    request: UpdateSkillRequest,
    fetcher: &F,
    policy: &P,
    publisher: &TransactionalSkillCreator,
) -> Result<UpdateSkillResult, UpdateSkillError> {
    update_skill_inner(request, None, fetcher, policy, publisher)
}

/// Rebuilds one managed skill after requesting approval when its management state is unsafe.
pub fn update_skill_with_confirmation<
    F: DocumentFetcher,
    P: CrawlPolicy,
    C: RebuildConfirmation,
>(
    request: UpdateSkillRequest,
    confirmation: &C,
    fetcher: &F,
    policy: &P,
    publisher: &TransactionalSkillCreator,
) -> Result<UpdateSkillResult, UpdateSkillError> {
    update_skill_inner(request, Some(confirmation), fetcher, policy, publisher)
}

fn update_skill_inner<F: DocumentFetcher, P: CrawlPolicy>(
    request: UpdateSkillRequest,
    confirmation: Option<&dyn RebuildConfirmation>,
    fetcher: &F,
    policy: &P,
    publisher: &TransactionalSkillCreator,
) -> Result<UpdateSkillResult, UpdateSkillError> {
    let metadata = match publisher
        .management_status(&request.current_name)
        .map_err(UpdateSkillError::Storage)?
    {
        Some(ManagedSkillStatus::Managed(metadata)) => {
            let content = publisher
                .managed_content(&request.current_name)
                .map_err(UpdateSkillError::Storage)?;
            if content
                .as_deref()
                .is_none_or(|content| !metadata.has_matching_content_digest(content))
            {
                require_rebuild_confirmation(&request.current_name, confirmation)?;
            }
            metadata
        }
        Some(ManagedSkillStatus::MetadataMissing | ManagedSkillStatus::InvalidMetadata(_)) => {
            require_rebuild_confirmation(&request.current_name, confirmation)?;
            return Err(UpdateSkillError::SkillNotManaged(request.current_name));
        }
        None => return Err(UpdateSkillError::SkillNotFound(request.current_name)),
    };
    let discovery = updated_discovery_configuration(&metadata, &request)?;
    let source_url = request
        .source_url
        .unwrap_or_else(|| metadata.source_url().clone());
    let new_name = request
        .new_name
        .unwrap_or_else(|| request.current_name.clone());
    let pages = discover_all(source_url.as_url().clone(), &discovery, fetcher, policy)
        .map_err(UpdateSkillError::Discovery)?;
    let content = match discovery.content_format() {
        ContentFormat::GuideWithReferences => render_guide_with_references(&pages),
        ContentFormat::OrganizedContent => render_organized_content(&pages),
    };
    let metadata = ManagedSkillMetadata::new(source_url, discovery, content_digest(&content));
    publisher
        .replace(&request.current_name, &new_name, &content, &metadata)
        .map_err(UpdateSkillError::Storage)?;

    Ok(UpdateSkillResult { name: new_name })
}

fn require_rebuild_confirmation(
    name: &SkillName,
    confirmation: Option<&dyn RebuildConfirmation>,
) -> Result<(), UpdateSkillError> {
    match confirmation {
        Some(confirmation) if confirmation.confirm_rebuild() == Some(true) => Ok(()),
        Some(_) => Err(UpdateSkillError::RebuildDeclined(name.clone())),
        None => Err(UpdateSkillError::RebuildRequired(name.clone())),
    }
}

fn updated_discovery_configuration(
    metadata: &ManagedSkillMetadata,
    request: &UpdateSkillRequest,
) -> Result<DiscoveryConfiguration, UpdateSkillError> {
    let previous = metadata.discovery();
    let scope = request.scope.unwrap_or_else(|| previous.scope());
    let site_boundary = (scope == DiscoveryScope::SameSite).then(|| {
        request.site_boundary.unwrap_or_else(|| {
            if previous.scope() == DiscoveryScope::SameSite {
                previous.site_boundary()
            } else {
                SiteBoundary::default()
            }
        })
    });
    let authorized_subdomains = if site_boundary == Some(SiteBoundary::BaseDomain) {
        request.authorized_subdomains.clone().unwrap_or_else(|| {
            if previous.scope() == DiscoveryScope::SameSite {
                previous.authorized_subdomains().to_vec()
            } else {
                Vec::new()
            }
        })
    } else {
        Vec::new()
    };
    let traversal_mode = request
        .traversal_mode
        .unwrap_or_else(|| previous.traversal_mode());
    let max_pages = if traversal_mode == TraversalMode::Limited {
        request.max_pages.or_else(|| previous.max_pages())
    } else {
        request.max_pages
    };
    let content_format = request
        .content_format
        .unwrap_or_else(|| previous.content_format());

    DiscoveryConfiguration::new(
        scope,
        site_boundary,
        authorized_subdomains,
        traversal_mode,
        max_pages,
        content_format,
    )
    .map_err(UpdateSkillError::InvalidDiscoveryConfiguration)
}
