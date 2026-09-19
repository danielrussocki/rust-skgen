//! Normalized documentation domain boundary.

use url::Url;

/// Marks a type belonging to the normalized skill model.
pub trait SkillModel {}

/// A validated skill name used for output directory names and selection.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct SkillName(String);

impl SkillName {
    /// Maximum number of ASCII characters permitted in a skill name.
    pub const MAX_LENGTH: usize = 64;

    /// Validates and stores a skill name slug.
    pub fn parse(value: impl Into<String>) -> Result<Self, SkillNameError> {
        let value = value.into();

        if value.is_empty() {
            return Err(SkillNameError::Empty);
        }

        if value.starts_with('-') {
            return Err(SkillNameError::StartsWithHyphen);
        }

        if value.ends_with('-') {
            return Err(SkillNameError::EndsWithHyphen);
        }

        for character in value.chars() {
            if !character.is_ascii_lowercase() && !character.is_ascii_digit() && character != '-' {
                return Err(SkillNameError::InvalidCharacter(character));
            }
        }

        let length = value.len();
        if length > Self::MAX_LENGTH {
            return Err(SkillNameError::TooLong { length });
        }

        Ok(Self(value))
    }

    /// Returns the validated slug.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl SkillModel for SkillName {}

/// Error returned when a skill name does not meet slug requirements.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SkillNameError {
    /// The name is empty.
    Empty,
    /// The name starts with a hyphen.
    StartsWithHyphen,
    /// The name ends with a hyphen.
    EndsWithHyphen,
    /// The name includes a character outside lowercase ASCII letters, digits, and hyphens.
    InvalidCharacter(char),
    /// The name exceeds the permitted length.
    TooLong {
        /// The supplied name length.
        length: usize,
    },
}

impl std::fmt::Display for SkillNameError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Empty => formatter.write_str("skill name cannot be empty"),
            Self::StartsWithHyphen => formatter.write_str("skill name cannot start with a hyphen"),
            Self::EndsWithHyphen => formatter.write_str("skill name cannot end with a hyphen"),
            Self::InvalidCharacter(character) => {
                write!(
                    formatter,
                    "skill name contains an invalid character: {character}"
                )
            }
            Self::TooLong { length } => write!(
                formatter,
                "skill name length {length} exceeds the maximum of {}",
                SkillName::MAX_LENGTH
            ),
        }
    }
}

impl std::error::Error for SkillNameError {}

/// A validated HTTP(S) documentation source URL.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceUrl(Url);

impl SourceUrl {
    /// Parses an HTTP(S) source URL.
    pub fn parse(value: &str) -> Result<Self, SourceUrlError> {
        let url = Url::parse(value).map_err(SourceUrlError::InvalidUrl)?;
        let scheme = url.scheme();

        if scheme != "http" && scheme != "https" {
            return Err(SourceUrlError::UnsupportedScheme {
                scheme: scheme.to_owned(),
            });
        }

        Ok(Self(url))
    }

    /// Returns the parsed source URL.
    pub fn as_url(&self) -> &Url {
        &self.0
    }
}

impl SkillModel for SourceUrl {}

/// Error returned when a source URL is invalid or unsupported.
#[derive(Debug)]
pub enum SourceUrlError {
    /// The URL cannot be parsed.
    InvalidUrl(url::ParseError),
    /// The URL uses a scheme other than HTTP or HTTPS.
    UnsupportedScheme {
        /// The unsupported URL scheme.
        scheme: String,
    },
}

impl std::fmt::Display for SourceUrlError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidUrl(error) => write!(formatter, "invalid source URL: {error}"),
            Self::UnsupportedScheme { scheme } => {
                write!(formatter, "unsupported source URL scheme: {scheme}")
            }
        }
    }
}

impl std::error::Error for SourceUrlError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::InvalidUrl(error) => Some(error),
            Self::UnsupportedScheme { .. } => None,
        }
    }
}

/// Documentation extracted from one source page.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DocumentationPage {
    source_url: Url,
    content: String,
}

impl DocumentationPage {
    /// Creates a page whose content remains attributable to its source URL.
    pub fn new(source_url: Url, content: String) -> Self {
        Self {
            source_url,
            content,
        }
    }

    /// Returns the URL from which the content was extracted.
    pub fn source_url(&self) -> &Url {
        &self.source_url
    }

    /// Returns normalized extracted documentation content.
    pub fn content(&self) -> &str {
        &self.content
    }
}

impl SkillModel for DocumentationPage {}

/// Determines which related documentation URLs may be discovered.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum DiscoveryScope {
    /// Include pages from the configured site boundary.
    #[default]
    SameSite,
    /// Include pages under the source URL path prefix.
    PathPrefix,
    /// Include pages under the source URL parent directory.
    ParentDirectory,
    /// Include links found in navigation elements on the source page.
    DocumentationNavigation,
}

impl SkillModel for DiscoveryScope {}

/// Determines how a same-site discovery boundary is evaluated.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum SiteBoundary {
    /// Require the same host and port while allowing HTTP and HTTPS changes.
    #[default]
    ExactHost,
    /// Permit explicitly authorized and linked subdomains of the base domain.
    BaseDomain,
    /// Require the same scheme, host, and port.
    SameOrigin,
}

impl SkillModel for SiteBoundary {}

/// Determines how deeply related documentation links are traversed.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum TraversalMode {
    /// Traverse until no new in-scope pages remain.
    #[default]
    All,
    /// Include only direct links from the source page.
    OneLevel,
    /// Traverse until a configured maximum page count is reached.
    Limited,
}

impl SkillModel for TraversalMode {}

/// Determines how extracted documentation is rendered into a skill.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ContentFormat {
    /// Render a guide that includes source references.
    #[default]
    GuideWithReferences,
    /// Render only extracted content organized by page or topic.
    OrganizedContent,
}

impl SkillModel for ContentFormat {}

/// A host explicitly authorized for base-domain discovery.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthorizedHost(String);

impl AuthorizedHost {
    /// Stores a host that was authorized at the CLI boundary.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Returns the authorized host.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl SkillModel for AuthorizedHost {}

/// Validated settings that control related documentation discovery.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DiscoveryConfiguration {
    scope: DiscoveryScope,
    site_boundary: Option<SiteBoundary>,
    authorized_subdomains: Vec<AuthorizedHost>,
    traversal_mode: TraversalMode,
    max_pages: Option<usize>,
    content_format: ContentFormat,
}

impl DiscoveryConfiguration {
    /// Default maximum number of pages for limited traversal.
    pub const DEFAULT_MAX_PAGES: usize = 100;

    /// Creates a validated discovery configuration.
    pub fn new(
        scope: DiscoveryScope,
        site_boundary: Option<SiteBoundary>,
        authorized_subdomains: Vec<AuthorizedHost>,
        traversal_mode: TraversalMode,
        max_pages: Option<usize>,
        content_format: ContentFormat,
    ) -> Result<Self, DiscoveryConfigurationError> {
        if scope != DiscoveryScope::SameSite && site_boundary.is_some() {
            return Err(DiscoveryConfigurationError::SiteBoundaryRequiresSameSiteScope);
        }

        if !authorized_subdomains.is_empty() && site_boundary != Some(SiteBoundary::BaseDomain) {
            return Err(DiscoveryConfigurationError::AuthorizedSubdomainsRequireBaseDomain);
        }

        let max_pages = match traversal_mode {
            TraversalMode::Limited => match max_pages {
                Some(0) => return Err(DiscoveryConfigurationError::MaximumPagesMustBePositive),
                Some(max_pages) => Some(max_pages),
                None => Some(Self::DEFAULT_MAX_PAGES),
            },
            TraversalMode::All | TraversalMode::OneLevel => {
                if max_pages.is_some() {
                    return Err(DiscoveryConfigurationError::MaximumPagesRequiresLimitedTraversal);
                }

                None
            }
        };

        Ok(Self {
            scope,
            site_boundary,
            authorized_subdomains,
            traversal_mode,
            max_pages,
            content_format,
        })
    }

    /// Returns the selected discovery scope.
    pub fn scope(&self) -> DiscoveryScope {
        self.scope
    }

    /// Returns the effective site boundary.
    pub fn site_boundary(&self) -> SiteBoundary {
        self.site_boundary.unwrap_or_default()
    }

    /// Returns explicitly authorized subdomains.
    pub fn authorized_subdomains(&self) -> &[AuthorizedHost] {
        &self.authorized_subdomains
    }

    /// Returns the selected traversal mode.
    pub fn traversal_mode(&self) -> TraversalMode {
        self.traversal_mode
    }

    /// Returns the effective maximum page count for limited traversal.
    pub fn max_pages(&self) -> Option<usize> {
        self.max_pages
    }

    /// Returns the selected content format.
    pub fn content_format(&self) -> ContentFormat {
        self.content_format
    }
}

impl SkillModel for DiscoveryConfiguration {}

/// Error returned when discovery settings are incompatible.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DiscoveryConfigurationError {
    /// A site boundary was selected outside same-site scope.
    SiteBoundaryRequiresSameSiteScope,
    /// Authorized subdomains were supplied without the base-domain boundary.
    AuthorizedSubdomainsRequireBaseDomain,
    /// A maximum page count was supplied for non-limited traversal.
    MaximumPagesRequiresLimitedTraversal,
    /// A limited traversal maximum was zero.
    MaximumPagesMustBePositive,
}

impl std::fmt::Display for DiscoveryConfigurationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SiteBoundaryRequiresSameSiteScope => {
                formatter.write_str("site boundary requires same-site scope")
            }
            Self::AuthorizedSubdomainsRequireBaseDomain => {
                formatter.write_str("authorized subdomains require the base-domain boundary")
            }
            Self::MaximumPagesRequiresLimitedTraversal => {
                formatter.write_str("maximum pages requires limited traversal")
            }
            Self::MaximumPagesMustBePositive => {
                formatter.write_str("maximum pages must be positive")
            }
        }
    }
}

impl std::error::Error for DiscoveryConfigurationError {}
