//! Managed skill metadata serialization and validation.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::domain::{
    AuthorizedHost, ContentFormat, DiscoveryConfiguration, DiscoveryConfigurationError,
    DiscoveryScope, SiteBoundary, SourceUrl, SourceUrlError, TraversalMode,
};

/// The only metadata schema version supported by this generator.
pub const SCHEMA_VERSION: u32 = 1;

/// Identifies metadata produced by this generator.
pub const GENERATOR: &str = "rust-skgen";

/// Calculates the SHA-256 digest stored for generated managed content.
pub fn content_digest(content: &str) -> String {
    format!("sha256:{:x}", Sha256::digest(content.as_bytes()))
}

/// Validated metadata used to identify and recreate a managed skill.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ManagedSkillMetadata {
    source_url: SourceUrl,
    discovery: DiscoveryConfiguration,
    content_digest: String,
}

impl ManagedSkillMetadata {
    /// Creates metadata from validated configuration and a generated content digest.
    pub fn new(
        source_url: SourceUrl,
        discovery: DiscoveryConfiguration,
        content_digest: String,
    ) -> Self {
        Self {
            source_url,
            discovery,
            content_digest,
        }
    }

    /// Serializes metadata into its stable JSON representation.
    pub fn to_json(&self) -> Result<String, MetadataError> {
        let raw = RawMetadata::from(self);
        serde_json::to_string(&raw).map_err(MetadataError::Serialization)
    }

    /// Deserializes and validates metadata from JSON.
    pub fn from_json(value: &str) -> Result<Self, MetadataError> {
        let raw: RawMetadata =
            serde_json::from_str(value).map_err(MetadataError::Deserialization)?;
        Self::try_from(raw)
    }

    /// Returns the source URL used to generate the skill.
    pub fn source_url(&self) -> &SourceUrl {
        &self.source_url
    }

    /// Returns the validated discovery configuration.
    pub fn discovery(&self) -> &DiscoveryConfiguration {
        &self.discovery
    }

    /// Returns the persisted digest of the managed content.
    pub fn content_digest(&self) -> &str {
        &self.content_digest
    }

    /// Returns whether the supplied managed content matches its stored digest.
    pub fn has_matching_content_digest(&self, content: &str) -> bool {
        self.content_digest == content_digest(content)
    }
}

/// Error returned when managed skill metadata cannot be serialized or validated.
#[derive(Debug)]
pub enum MetadataError {
    /// JSON could not be serialized.
    Serialization(serde_json::Error),
    /// JSON could not be parsed into the metadata representation.
    Deserialization(serde_json::Error),
    /// The metadata schema version is unsupported.
    UnsupportedSchemaVersion(u32),
    /// The metadata was not produced by this generator.
    UnsupportedGenerator(String),
    /// The stored source URL is invalid.
    InvalidSourceUrl(SourceUrlError),
    /// The stored discovery settings are incompatible.
    InvalidDiscoveryConfiguration(DiscoveryConfigurationError),
    /// The stored content digest is empty.
    EmptyContentDigest,
    /// The stored content digest is not a SHA-256 digest.
    InvalidContentDigest(String),
}

impl std::fmt::Display for MetadataError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Serialization(error) => {
                write!(formatter, "failed to serialize metadata: {error}")
            }
            Self::Deserialization(error) => write!(formatter, "invalid metadata JSON: {error}"),
            Self::UnsupportedSchemaVersion(version) => {
                write!(formatter, "unsupported metadata schema version: {version}")
            }
            Self::UnsupportedGenerator(generator) => {
                write!(formatter, "unsupported metadata generator: {generator}")
            }
            Self::InvalidSourceUrl(error) => {
                write!(formatter, "invalid metadata source URL: {error}")
            }
            Self::InvalidDiscoveryConfiguration(error) => {
                write!(
                    formatter,
                    "invalid metadata discovery configuration: {error}"
                )
            }
            Self::EmptyContentDigest => {
                formatter.write_str("metadata content digest cannot be empty")
            }
            Self::InvalidContentDigest(digest) => {
                write!(
                    formatter,
                    "metadata content digest is not a SHA-256 digest: {digest}"
                )
            }
        }
    }
}

impl std::error::Error for MetadataError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Serialization(error) | Self::Deserialization(error) => Some(error),
            Self::InvalidSourceUrl(error) => Some(error),
            Self::InvalidDiscoveryConfiguration(error) => Some(error),
            Self::UnsupportedSchemaVersion(_)
            | Self::UnsupportedGenerator(_)
            | Self::EmptyContentDigest
            | Self::InvalidContentDigest(_) => None,
        }
    }
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct RawMetadata {
    schema_version: u32,
    generator: String,
    source_url: String,
    discovery: RawDiscoveryConfiguration,
    content_format: RawContentFormat,
    content_digest: String,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct RawDiscoveryConfiguration {
    scope: RawDiscoveryScope,
    site_boundary: Option<RawSiteBoundary>,
    allowed_subdomains: Vec<String>,
    traversal: RawTraversal,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct RawTraversal {
    mode: RawTraversalMode,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_pages: Option<usize>,
}

#[derive(Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
enum RawDiscoveryScope {
    SameSite,
    PathPrefix,
    ParentDirectory,
    DocumentationNavigation,
}

#[derive(Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
enum RawSiteBoundary {
    ExactHost,
    BaseDomain,
    SameOrigin,
}

#[derive(Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
enum RawTraversalMode {
    All,
    OneLevel,
    Limited,
}

#[derive(Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
enum RawContentFormat {
    GuideWithReferences,
    OrganizedContent,
}

impl From<&ManagedSkillMetadata> for RawMetadata {
    fn from(metadata: &ManagedSkillMetadata) -> Self {
        let discovery = metadata.discovery();
        Self {
            schema_version: SCHEMA_VERSION,
            generator: GENERATOR.to_owned(),
            source_url: metadata.source_url().as_url().as_str().to_owned(),
            discovery: RawDiscoveryConfiguration {
                scope: discovery.scope().into(),
                site_boundary: (discovery.scope() == DiscoveryScope::SameSite)
                    .then(|| discovery.site_boundary().into()),
                allowed_subdomains: discovery
                    .authorized_subdomains()
                    .iter()
                    .map(|host| host.as_str().to_owned())
                    .collect(),
                traversal: RawTraversal {
                    mode: discovery.traversal_mode().into(),
                    max_pages: discovery.max_pages(),
                },
            },
            content_format: discovery.content_format().into(),
            content_digest: metadata.content_digest().to_owned(),
        }
    }
}

impl TryFrom<RawMetadata> for ManagedSkillMetadata {
    type Error = MetadataError;

    fn try_from(raw: RawMetadata) -> Result<Self, Self::Error> {
        if raw.schema_version != SCHEMA_VERSION {
            return Err(MetadataError::UnsupportedSchemaVersion(raw.schema_version));
        }
        if raw.generator != GENERATOR {
            return Err(MetadataError::UnsupportedGenerator(raw.generator));
        }
        if raw.content_digest.is_empty() {
            return Err(MetadataError::EmptyContentDigest);
        }
        if !is_sha256_digest(&raw.content_digest) {
            return Err(MetadataError::InvalidContentDigest(raw.content_digest));
        }

        let scope: DiscoveryScope = raw.discovery.scope.into();
        let site_boundary = match scope {
            DiscoveryScope::SameSite => raw.discovery.site_boundary.map(Into::into),
            DiscoveryScope::PathPrefix
            | DiscoveryScope::ParentDirectory
            | DiscoveryScope::DocumentationNavigation => {
                if raw.discovery.site_boundary.is_some() {
                    return Err(MetadataError::InvalidDiscoveryConfiguration(
                        DiscoveryConfigurationError::SiteBoundaryRequiresSameSiteScope,
                    ));
                }
                None
            }
        };
        let authorized_subdomains = raw
            .discovery
            .allowed_subdomains
            .into_iter()
            .map(AuthorizedHost::new)
            .collect();
        let discovery = DiscoveryConfiguration::new(
            scope,
            site_boundary,
            authorized_subdomains,
            raw.discovery.traversal.mode.into(),
            raw.discovery.traversal.max_pages,
            raw.content_format.into(),
        )
        .map_err(MetadataError::InvalidDiscoveryConfiguration)?;

        let source_url =
            SourceUrl::parse(&raw.source_url).map_err(MetadataError::InvalidSourceUrl)?;
        Ok(Self::new(source_url, discovery, raw.content_digest))
    }
}

fn is_sha256_digest(value: &str) -> bool {
    value.strip_prefix("sha256:").is_some_and(|digest| {
        digest.len() == 64 && digest.bytes().all(|byte| byte.is_ascii_hexdigit())
    })
}

impl From<DiscoveryScope> for RawDiscoveryScope {
    fn from(value: DiscoveryScope) -> Self {
        match value {
            DiscoveryScope::SameSite => Self::SameSite,
            DiscoveryScope::PathPrefix => Self::PathPrefix,
            DiscoveryScope::ParentDirectory => Self::ParentDirectory,
            DiscoveryScope::DocumentationNavigation => Self::DocumentationNavigation,
        }
    }
}

impl From<RawDiscoveryScope> for DiscoveryScope {
    fn from(value: RawDiscoveryScope) -> Self {
        match value {
            RawDiscoveryScope::SameSite => Self::SameSite,
            RawDiscoveryScope::PathPrefix => Self::PathPrefix,
            RawDiscoveryScope::ParentDirectory => Self::ParentDirectory,
            RawDiscoveryScope::DocumentationNavigation => Self::DocumentationNavigation,
        }
    }
}

impl From<SiteBoundary> for RawSiteBoundary {
    fn from(value: SiteBoundary) -> Self {
        match value {
            SiteBoundary::ExactHost => Self::ExactHost,
            SiteBoundary::BaseDomain => Self::BaseDomain,
            SiteBoundary::SameOrigin => Self::SameOrigin,
        }
    }
}

impl From<RawSiteBoundary> for SiteBoundary {
    fn from(value: RawSiteBoundary) -> Self {
        match value {
            RawSiteBoundary::ExactHost => Self::ExactHost,
            RawSiteBoundary::BaseDomain => Self::BaseDomain,
            RawSiteBoundary::SameOrigin => Self::SameOrigin,
        }
    }
}

impl From<TraversalMode> for RawTraversalMode {
    fn from(value: TraversalMode) -> Self {
        match value {
            TraversalMode::All => Self::All,
            TraversalMode::OneLevel => Self::OneLevel,
            TraversalMode::Limited => Self::Limited,
        }
    }
}

impl From<RawTraversalMode> for TraversalMode {
    fn from(value: RawTraversalMode) -> Self {
        match value {
            RawTraversalMode::All => Self::All,
            RawTraversalMode::OneLevel => Self::OneLevel,
            RawTraversalMode::Limited => Self::Limited,
        }
    }
}

impl From<ContentFormat> for RawContentFormat {
    fn from(value: ContentFormat) -> Self {
        match value {
            ContentFormat::GuideWithReferences => Self::GuideWithReferences,
            ContentFormat::OrganizedContent => Self::OrganizedContent,
        }
    }
}

impl From<RawContentFormat> for ContentFormat {
    fn from(value: RawContentFormat) -> Self {
        match value {
            RawContentFormat::GuideWithReferences => Self::GuideWithReferences,
            RawContentFormat::OrganizedContent => Self::OrganizedContent,
        }
    }
}

/// Marks a component that reads and writes managed skill metadata.
pub trait MetadataStore {}
