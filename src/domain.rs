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
