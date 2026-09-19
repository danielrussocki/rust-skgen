//! Skill filesystem storage boundary.

use std::{
    fs, io,
    path::{Path, PathBuf},
    time::{SystemTime, SystemTimeError, UNIX_EPOCH},
};

use crate::{
    domain::SkillName,
    metadata::{ManagedSkillMetadata, MetadataError},
};

/// File containing metadata that identifies a skill managed by this generator.
pub const METADATA_FILE_NAME: &str = "metadata.json";

/// File containing the rendered managed skill content.
pub const SKILL_FILE_NAME: &str = "SKILL.md";

/// Publishes complete new skills without modifying an existing destination.
pub struct TransactionalSkillCreator {
    skills_root: PathBuf,
}

impl TransactionalSkillCreator {
    /// Creates a publisher rooted at the supplied skills output directory.
    pub fn new(skills_root: impl Into<PathBuf>) -> Self {
        Self {
            skills_root: skills_root.into(),
        }
    }

    /// Writes a complete skill and metadata set, then publishes it as one directory rename.
    pub fn create(
        &self,
        name: &SkillName,
        content: &str,
        metadata: &ManagedSkillMetadata,
    ) -> Result<(), StorageError> {
        let metadata = metadata
            .to_json()
            .map_err(StorageError::SerializeMetadata)?;
        let destination = self.skills_root.join(name.as_str());

        if destination.exists() {
            return Err(StorageError::DestinationExists(destination));
        }

        fs::create_dir_all(&self.skills_root).map_err(StorageError::CreateSkillsRoot)?;
        let pending = self.create_pending_directory(name)?;
        let result = self.write_and_publish(&pending, &destination, content, &metadata);

        if result.is_err() {
            let _ = fs::remove_dir_all(&pending);
        }

        result
    }

    /// Acquires an exclusive lock for a skill operation.
    pub fn lock(&self, name: &SkillName) -> Result<SkillLock, StorageError> {
        fs::create_dir_all(&self.skills_root).map_err(StorageError::CreateSkillsRoot)?;
        let path = self.skills_root.join(format!(".{}.lock", name.as_str()));
        fs::create_dir(&path).map_err(|error| {
            if error.kind() == io::ErrorKind::AlreadyExists {
                StorageError::SkillLocked(name.as_str().to_owned())
            } else {
                StorageError::AcquireLock(error)
            }
        })?;
        Ok(SkillLock { path })
    }

    /// Reads the management status for one existing skill without modifying it.
    pub fn management_status(
        &self,
        name: &SkillName,
    ) -> Result<Option<ManagedSkillStatus>, StorageError> {
        let path = self.skills_root.join(name.as_str());
        if !path.is_dir() {
            return Ok(None);
        }

        let metadata_path = path.join(METADATA_FILE_NAME);
        match fs::read_to_string(metadata_path) {
            Ok(value) => match ManagedSkillMetadata::from_json(&value) {
                Ok(metadata) => Ok(Some(ManagedSkillStatus::Managed(metadata))),
                Err(error) => Ok(Some(ManagedSkillStatus::InvalidMetadata(error))),
            },
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                Ok(Some(ManagedSkillStatus::MetadataMissing))
            }
            Err(error) => Err(StorageError::ReadMetadata { path, error }),
        }
    }

    /// Replaces an existing skill, optionally publishing it under a new available name.
    pub fn replace(
        &self,
        current_name: &SkillName,
        new_name: &SkillName,
        content: &str,
        metadata: &ManagedSkillMetadata,
    ) -> Result<(), StorageError> {
        let metadata = metadata
            .to_json()
            .map_err(StorageError::SerializeMetadata)?;
        self.replace_with_writer(current_name, new_name, |pending| {
            fs::write(pending.join(SKILL_FILE_NAME), content)
                .map_err(StorageError::WriteContent)?;
            fs::write(pending.join(METADATA_FILE_NAME), metadata)
                .map_err(StorageError::WriteMetadata)
        })
    }

    fn replace_with_writer<F>(
        &self,
        current_name: &SkillName,
        new_name: &SkillName,
        writer: F,
    ) -> Result<(), StorageError>
    where
        F: FnOnce(&Path) -> Result<(), StorageError>,
    {
        let _locks = self.lock_names(current_name, new_name)?;
        let current = self.skills_root.join(current_name.as_str());
        let destination = self.skills_root.join(new_name.as_str());
        if !current.is_dir() {
            return Err(StorageError::SourceMissing(current));
        }
        if current_name != new_name && destination.exists() {
            return Err(StorageError::DestinationExists(destination));
        }

        let pending = self.create_pending_directory(new_name)?;
        if let Err(error) = writer(&pending) {
            let _ = fs::remove_dir_all(&pending);
            return Err(error);
        }

        let backup = self.backup_path(current_name)?;
        if let Err(error) = fs::rename(&current, &backup) {
            let _ = fs::remove_dir_all(&pending);
            return Err(StorageError::BackupSkill(error));
        }

        match fs::rename(&pending, &destination) {
            Ok(()) => fs::remove_dir_all(&backup).map_err(StorageError::RemoveBackup),
            Err(error) => {
                let _ = fs::remove_dir_all(&pending);
                if let Err(restore_error) = fs::rename(&backup, &current) {
                    return Err(StorageError::RestorePreviousSkill {
                        publish_error: error,
                        restore_error,
                    });
                }
                Err(StorageError::PublishSkill(error))
            }
        }
    }

    fn lock_names(
        &self,
        current_name: &SkillName,
        new_name: &SkillName,
    ) -> Result<Vec<SkillLock>, StorageError> {
        if current_name == new_name {
            return Ok(vec![self.lock(current_name)?]);
        }

        let mut names = [current_name, new_name];
        names.sort_by(|left, right| left.as_str().cmp(right.as_str()));
        names.into_iter().map(|name| self.lock(name)).collect()
    }

    fn backup_path(&self, name: &SkillName) -> Result<PathBuf, StorageError> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(StorageError::ReadSystemClock)?
            .as_nanos();

        for attempt in 0..100 {
            let backup = self
                .skills_root
                .join(format!(".{}.backup-{timestamp}-{attempt}", name.as_str()));
            if !backup.exists() {
                return Ok(backup);
            }
        }

        Err(StorageError::CreatePendingDirectory(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "could not allocate a unique skill backup directory",
        )))
    }

    fn create_pending_directory(&self, name: &SkillName) -> Result<PathBuf, StorageError> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(StorageError::ReadSystemClock)?
            .as_nanos();

        for attempt in 0..100 {
            let pending = self
                .skills_root
                .join(format!(".{}.pending-{timestamp}-{attempt}", name.as_str()));
            match fs::create_dir(&pending) {
                Ok(()) => return Ok(pending),
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(StorageError::CreatePendingDirectory(error)),
            }
        }

        Err(StorageError::CreatePendingDirectory(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "could not allocate a unique pending skill directory",
        )))
    }

    fn write_and_publish(
        &self,
        pending: &Path,
        destination: &Path,
        content: &str,
        metadata: &str,
    ) -> Result<(), StorageError> {
        fs::write(pending.join(SKILL_FILE_NAME), content).map_err(StorageError::WriteContent)?;
        fs::write(pending.join(METADATA_FILE_NAME), metadata)
            .map_err(StorageError::WriteMetadata)?;

        if destination.exists() {
            return Err(StorageError::DestinationExists(destination.to_path_buf()));
        }

        fs::rename(pending, destination).map_err(StorageError::PublishSkill)
    }
}

/// A filesystem-backed exclusive skill lock released when dropped.
pub struct SkillLock {
    path: PathBuf,
}

impl Drop for SkillLock {
    fn drop(&mut self) {
        let _ = fs::remove_dir(&self.path);
    }
}

/// The result of reading metadata for a discovered skill directory.
#[derive(Debug)]
pub enum ManagedSkillStatus {
    /// Metadata is valid and was generated by this CLI.
    Managed(ManagedSkillMetadata),
    /// The skill directory does not contain generator metadata.
    MetadataMissing,
    /// Metadata exists but cannot identify a managed skill safely.
    InvalidMetadata(MetadataError),
}

/// A skill directory located under the configured skills root.
#[derive(Debug)]
pub struct LocatedSkill {
    name: SkillName,
    path: PathBuf,
    status: ManagedSkillStatus,
}

impl LocatedSkill {
    /// Returns the validated directory name of the skill.
    pub fn name(&self) -> &SkillName {
        &self.name
    }

    /// Returns the directory containing the skill.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Returns the result of reading this skill's metadata.
    pub fn status(&self) -> &ManagedSkillStatus {
        &self.status
    }

    /// Returns whether this skill has valid metadata from this generator.
    pub fn is_managed(&self) -> bool {
        matches!(self.status, ManagedSkillStatus::Managed(_))
    }
}

/// Locates generated skills and reads their metadata without modifying the filesystem.
pub struct SkillLocator {
    skills_root: PathBuf,
}

impl SkillLocator {
    /// Creates a locator for directories directly below the supplied skills root.
    pub fn new(skills_root: impl Into<PathBuf>) -> Self {
        Self {
            skills_root: skills_root.into(),
        }
    }

    /// Lists candidate skill directories in stable name order with their metadata status.
    pub fn locate(&self) -> Result<Vec<LocatedSkill>, StorageError> {
        let entries = match fs::read_dir(&self.skills_root) {
            Ok(entries) => entries,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => return Err(StorageError::ReadSkillsRoot(error)),
        };

        let mut candidates = Vec::new();
        for entry in entries {
            let entry = entry.map_err(StorageError::ReadSkillsRoot)?;
            if !entry
                .file_type()
                .map_err(StorageError::ReadSkillsRoot)?
                .is_dir()
            {
                continue;
            }

            let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
                continue;
            };
            let Ok(name) = SkillName::parse(name) else {
                continue;
            };
            candidates.push((name, entry.path()));
        }

        candidates.sort_by(|left, right| left.0.as_str().cmp(right.0.as_str()));
        candidates
            .into_iter()
            .map(|(name, path)| self.locate_skill(name, path))
            .collect()
    }

    fn locate_skill(&self, name: SkillName, path: PathBuf) -> Result<LocatedSkill, StorageError> {
        let metadata_path = path.join(METADATA_FILE_NAME);
        let status = match fs::read_to_string(metadata_path) {
            Ok(value) => match ManagedSkillMetadata::from_json(&value) {
                Ok(metadata) => ManagedSkillStatus::Managed(metadata),
                Err(error) => ManagedSkillStatus::InvalidMetadata(error),
            },
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                ManagedSkillStatus::MetadataMissing
            }
            Err(error) => return Err(StorageError::ReadMetadata { path, error }),
        };

        Ok(LocatedSkill { name, path, status })
    }
}

/// Error returned while locating skills or reading metadata from the filesystem.
#[derive(Debug)]
pub enum StorageError {
    /// The skills root could not be created.
    CreateSkillsRoot(io::Error),
    /// The system clock could not provide a pending directory identifier.
    ReadSystemClock(SystemTimeError),
    /// A pending directory could not be created.
    CreatePendingDirectory(io::Error),
    /// Another operation already owns the exclusive lock for this skill.
    SkillLocked(String),
    /// The exclusive lock directory could not be created.
    AcquireLock(io::Error),
    /// The existing skill to replace is missing.
    SourceMissing(PathBuf),
    /// The skill content could not be written to the pending directory.
    WriteContent(io::Error),
    /// The skill metadata could not be serialized.
    SerializeMetadata(MetadataError),
    /// The skill metadata could not be written to the pending directory.
    WriteMetadata(io::Error),
    /// The destination already exists and must not be overwritten during creation.
    DestinationExists(PathBuf),
    /// The complete pending skill could not be published.
    PublishSkill(io::Error),
    /// The existing skill could not be moved aside before publication.
    BackupSkill(io::Error),
    /// The old skill backup could not be removed after successful publication.
    RemoveBackup(io::Error),
    /// Publication failed and the prior skill could not be restored.
    RestorePreviousSkill {
        /// The error that prevented publication.
        publish_error: io::Error,
        /// The error that prevented restoration.
        restore_error: io::Error,
    },
    /// The skills root could not be listed.
    ReadSkillsRoot(io::Error),
    /// Metadata for one skill could not be read.
    ReadMetadata { path: PathBuf, error: io::Error },
}

impl std::fmt::Display for StorageError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CreateSkillsRoot(error) => {
                write!(formatter, "failed to create skills root: {error}")
            }
            Self::ReadSystemClock(error) => {
                write!(formatter, "failed to read system clock: {error}")
            }
            Self::CreatePendingDirectory(error) => {
                write!(
                    formatter,
                    "failed to create pending skill directory: {error}"
                )
            }
            Self::SkillLocked(name) => {
                write!(formatter, "skill is locked by another operation: {name}")
            }
            Self::AcquireLock(error) => write!(formatter, "failed to acquire skill lock: {error}"),
            Self::SourceMissing(path) => {
                write!(
                    formatter,
                    "skill to replace does not exist: {}",
                    path.display()
                )
            }
            Self::WriteContent(error) => {
                write!(formatter, "failed to write skill content: {error}")
            }
            Self::SerializeMetadata(error) => {
                write!(formatter, "failed to serialize metadata: {error}")
            }
            Self::WriteMetadata(error) => {
                write!(formatter, "failed to write skill metadata: {error}")
            }
            Self::DestinationExists(path) => {
                write!(
                    formatter,
                    "skill destination already exists: {}",
                    path.display()
                )
            }
            Self::PublishSkill(error) => write!(formatter, "failed to publish skill: {error}"),
            Self::BackupSkill(error) => {
                write!(formatter, "failed to back up existing skill: {error}")
            }
            Self::RemoveBackup(error) => {
                write!(formatter, "failed to remove skill backup: {error}")
            }
            Self::RestorePreviousSkill {
                publish_error,
                restore_error,
            } => write!(
                formatter,
                "failed to publish skill ({publish_error}) and restore previous skill ({restore_error})"
            ),
            Self::ReadSkillsRoot(error) => write!(formatter, "failed to read skills root: {error}"),
            Self::ReadMetadata { path, error } => {
                write!(
                    formatter,
                    "failed to read metadata at {}: {error}",
                    path.display()
                )
            }
        }
    }
}

impl std::error::Error for StorageError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::CreateSkillsRoot(error)
            | Self::CreatePendingDirectory(error)
            | Self::AcquireLock(error)
            | Self::WriteContent(error)
            | Self::WriteMetadata(error)
            | Self::PublishSkill(error)
            | Self::BackupSkill(error)
            | Self::RemoveBackup(error)
            | Self::ReadSkillsRoot(error) => Some(error),
            Self::ReadSystemClock(error) => Some(error),
            Self::SerializeMetadata(error) => Some(error),
            Self::RestorePreviousSkill { publish_error, .. } => Some(publish_error),
            Self::DestinationExists(_) | Self::SkillLocked(_) | Self::SourceMissing(_) => None,
            Self::ReadMetadata { error, .. } => Some(error),
        }
    }
}

/// Marks a component that persists generated skills.
pub trait SkillStorage {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn restores_the_previous_skill_when_writing_a_replacement_fails() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("rust-skgen-write-failure-{unique}"));
        fs::create_dir(&root).unwrap();
        let name = SkillName::parse("existing-skill").unwrap();
        let skill = root.join(name.as_str());
        fs::create_dir(&skill).unwrap();
        fs::write(skill.join(SKILL_FILE_NAME), "# Previous skill\n").unwrap();

        let result =
            TransactionalSkillCreator::new(&root).replace_with_writer(&name, &name, |_| {
                Err(StorageError::WriteContent(io::Error::other(
                    "simulated failure",
                )))
            });

        assert!(matches!(result, Err(StorageError::WriteContent(_))));
        assert_eq!(
            fs::read_to_string(skill.join(SKILL_FILE_NAME)).unwrap(),
            "# Previous skill\n"
        );
        fs::remove_dir_all(root).unwrap();
    }
}
