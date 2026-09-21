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

/// Update request for a managed skill selected by its current name.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UpdateSkillRequest {
    current_name: SkillName,
    changes: UpdateSkillChanges,
}

/// Validated partial configuration changes that can be applied to one managed skill.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct UpdateSkillChanges {
    new_name: Option<SkillName>,
    source_url: Option<SourceUrl>,
    scope: Option<DiscoveryScope>,
    site_boundary: Option<SiteBoundary>,
    authorized_subdomains: Option<Vec<AuthorizedHost>>,
    traversal_mode: Option<TraversalMode>,
    max_pages: Option<usize>,
    content_format: Option<ContentFormat>,
    requires_robots_txt: Option<bool>,
}

impl UpdateSkillRequest {
    /// Starts an update for the managed skill selected by its current name.
    pub fn new(current_name: SkillName) -> Self {
        Self {
            current_name,
            changes: UpdateSkillChanges::default(),
        }
    }
}

impl UpdateSkillChanges {
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

    /// Changes whether robots.txt is required during discovery.
    pub fn with_requires_robots_txt(mut self, requires_robots_txt: bool) -> Self {
        self.requires_robots_txt = Some(requires_robots_txt);
        self
    }

    fn has_changes(&self) -> bool {
        self.new_name.is_some()
            || self.source_url.is_some()
            || self.scope.is_some()
            || self.site_boundary.is_some()
            || self.authorized_subdomains.is_some()
            || self.traversal_mode.is_some()
            || self.max_pages.is_some()
            || self.content_format.is_some()
            || self.requires_robots_txt.is_some()
    }
}

impl UpdateSkillRequest {
    /// Changes the published skill name.
    pub fn with_name(mut self, name: SkillName) -> Self {
        self.changes = self.changes.with_name(name);
        self
    }

    /// Changes the documentation source URL.
    pub fn with_source_url(mut self, source_url: SourceUrl) -> Self {
        self.changes = self.changes.with_source_url(source_url);
        self
    }

    /// Changes the discovery scope.
    pub fn with_scope(mut self, scope: DiscoveryScope) -> Self {
        self.changes = self.changes.with_scope(scope);
        self
    }

    /// Changes the same-site boundary.
    pub fn with_site_boundary(mut self, site_boundary: SiteBoundary) -> Self {
        self.changes = self.changes.with_site_boundary(site_boundary);
        self
    }

    /// Changes whether robots.txt is required during discovery.
    pub fn with_requires_robots_txt(mut self, requires_robots_txt: bool) -> Self {
        self.changes = self.changes.with_requires_robots_txt(requires_robots_txt);
        self
    }

    /// Replaces the explicitly authorized base-domain subdomains.
    pub fn with_authorized_subdomains(
        mut self,
        authorized_subdomains: Vec<AuthorizedHost>,
    ) -> Self {
        self.changes = self
            .changes
            .with_authorized_subdomains(authorized_subdomains);
        self
    }

    /// Changes the traversal mode.
    pub fn with_traversal_mode(mut self, traversal_mode: TraversalMode) -> Self {
        self.changes = self.changes.with_traversal_mode(traversal_mode);
        self
    }

    /// Changes the maximum page count for limited traversal.
    pub fn with_max_pages(mut self, max_pages: usize) -> Self {
        self.changes = self.changes.with_max_pages(max_pages);
        self
    }

    /// Changes the generated content format.
    pub fn with_content_format(mut self, content_format: ContentFormat) -> Self {
        self.changes = self.changes.with_content_format(content_format);
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
    /// Rebuilding unmanaged metadata requires an explicit source URL.
    RebuildConfigurationRequired(SkillName),
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
            Self::RebuildConfigurationRequired(name) => {
                write!(
                    formatter,
                    "metadata reconstruction requires a source URL: {}",
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
            | Self::RebuildDeclined(_)
            | Self::RebuildConfigurationRequired(_) => None,
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

/// Error returned before a directed batch update can start.
#[derive(Debug)]
pub enum UpdateSkillsError {
    /// Configuration changes are only valid when exactly one skill is selected.
    ConfigurationChangesRequireSingleSelection,
}

impl std::fmt::Display for UpdateSkillsError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ConfigurationChangesRequireSingleSelection => {
                write!(
                    formatter,
                    "configuration changes require a single selected skill"
                )
            }
        }
    }
}

impl std::error::Error for UpdateSkillsError {}

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
    let names = selected_skill_names(selection, locator)?;
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

/// Updates every managed skill or an explicitly directed selection after requesting confirmation
/// for each skill whose metadata must be rebuilt.
pub fn update_skills_with_confirmation<
    F: DocumentFetcher,
    P: CrawlPolicy,
    C: RebuildConfirmation,
>(
    selection: Option<&[SkillName]>,
    locator: &SkillLocator,
    confirmation: &C,
    fetcher: &F,
    policy: &P,
    publisher: &TransactionalSkillCreator,
) -> Result<UpdateSkillsResult, StorageError> {
    let names = selected_skill_names(selection, locator)?;
    let outcomes = names
        .into_iter()
        .map(|name| UpdateSkillsOutcome {
            result: update_skill_with_confirmation(
                UpdateSkillRequest::new(name.clone()),
                confirmation,
                fetcher,
                policy,
                publisher,
            ),
            name,
        })
        .collect();

    Ok(UpdateSkillsResult { outcomes })
}

fn selected_skill_names(
    selection: Option<&[SkillName]>,
    locator: &SkillLocator,
) -> Result<Vec<SkillName>, StorageError> {
    match selection {
        Some(names) => Ok(names.to_vec()),
        None => Ok(locator
            .locate()?
            .into_iter()
            .filter(|skill| skill.is_managed())
            .map(|skill| skill.name().clone())
            .collect()),
    }
}

/// Updates a directed selection, applying configuration changes only when one skill is selected.
///
/// The validation happens before any selected skill is read, fetched, or published.
pub fn update_skills_with_changes<F: DocumentFetcher, P: CrawlPolicy>(
    selection: &[SkillName],
    changes: UpdateSkillChanges,
    fetcher: &F,
    policy: &P,
    publisher: &TransactionalSkillCreator,
) -> Result<UpdateSkillsResult, UpdateSkillsError> {
    if selection.len() > 1 && changes.has_changes() {
        return Err(UpdateSkillsError::ConfigurationChangesRequireSingleSelection);
    }

    let outcomes = selection
        .iter()
        .cloned()
        .map(|name| UpdateSkillsOutcome {
            result: update_skill(
                UpdateSkillRequest {
                    current_name: name.clone(),
                    changes: changes.clone(),
                },
                fetcher,
                policy,
                publisher,
            ),
            name,
        })
        .collect();

    Ok(UpdateSkillsResult { outcomes })
}

/// Updates a directed selection after requesting confirmation for each skill whose metadata must
/// be rebuilt.
pub fn update_skills_with_changes_and_confirmation<
    F: DocumentFetcher,
    P: CrawlPolicy,
    C: RebuildConfirmation,
>(
    selection: &[SkillName],
    changes: UpdateSkillChanges,
    confirmation: &C,
    fetcher: &F,
    policy: &P,
    publisher: &TransactionalSkillCreator,
) -> Result<UpdateSkillsResult, UpdateSkillsError> {
    if selection.len() > 1 && changes.has_changes() {
        return Err(UpdateSkillsError::ConfigurationChangesRequireSingleSelection);
    }

    let outcomes = selection
        .iter()
        .cloned()
        .map(|name| UpdateSkillsOutcome {
            result: update_skill_with_confirmation(
                UpdateSkillRequest {
                    current_name: name.clone(),
                    changes: changes.clone(),
                },
                confirmation,
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
    let (source_url, discovery) = match publisher
        .management_status(&request.current_name)
        .map_err(UpdateSkillError::Storage)?
    {
        Some(ManagedSkillStatus::Managed(metadata)) => (
            request
                .changes
                .source_url
                .clone()
                .unwrap_or_else(|| metadata.source_url().clone()),
            updated_discovery_configuration(metadata.discovery(), &request.changes)?,
        ),
        Some(ManagedSkillStatus::ContentDigestMismatch(metadata)) => {
            require_rebuild_confirmation(&request.current_name, confirmation)?;
            (
                request
                    .changes
                    .source_url
                    .clone()
                    .unwrap_or_else(|| metadata.source_url().clone()),
                updated_discovery_configuration(metadata.discovery(), &request.changes)?,
            )
        }
        Some(ManagedSkillStatus::MetadataMissing | ManagedSkillStatus::InvalidMetadata(_)) => {
            require_rebuild_confirmation(&request.current_name, confirmation)?;
            let source_url = request.changes.source_url.clone().ok_or_else(|| {
                UpdateSkillError::RebuildConfigurationRequired(request.current_name.clone())
            })?;
            let defaults = DiscoveryConfiguration::default();
            (
                source_url,
                updated_discovery_configuration(&defaults, &request.changes)?,
            )
        }
        None => return Err(UpdateSkillError::SkillNotFound(request.current_name)),
    };
    let new_name = request
        .changes
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
    previous: &DiscoveryConfiguration,
    changes: &UpdateSkillChanges,
) -> Result<DiscoveryConfiguration, UpdateSkillError> {
    let scope = changes.scope.unwrap_or_else(|| previous.scope());
    let site_boundary = (scope == DiscoveryScope::SameSite).then(|| {
        changes.site_boundary.unwrap_or_else(|| {
            if previous.scope() == DiscoveryScope::SameSite {
                previous.site_boundary()
            } else {
                SiteBoundary::default()
            }
        })
    });
    let authorized_subdomains = if site_boundary == Some(SiteBoundary::BaseDomain) {
        changes.authorized_subdomains.clone().unwrap_or_else(|| {
            if previous.scope() == DiscoveryScope::SameSite {
                previous.authorized_subdomains().to_vec()
            } else {
                Vec::new()
            }
        })
    } else {
        Vec::new()
    };
    let traversal_mode = changes
        .traversal_mode
        .unwrap_or_else(|| previous.traversal_mode());
    let max_pages = if traversal_mode == TraversalMode::Limited {
        changes.max_pages.or_else(|| previous.max_pages())
    } else {
        changes.max_pages
    };
    let content_format = changes
        .content_format
        .unwrap_or_else(|| previous.content_format());

    DiscoveryConfiguration::new(
        scope,
        site_boundary,
        authorized_subdomains,
        traversal_mode,
        max_pages,
        content_format,
        changes
            .requires_robots_txt
            .unwrap_or_else(|| previous.requires_robots_txt()),
    )
    .map_err(UpdateSkillError::InvalidDiscoveryConfiguration)
}
