//! Normalized documentation domain boundary.

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
