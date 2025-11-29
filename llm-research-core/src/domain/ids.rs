use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;
use sha2::{Sha256, Digest};

/// Newtype pattern for ExperimentId
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(transparent)]
pub struct ExperimentId(pub Uuid);

impl ExperimentId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl Default for ExperimentId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for ExperimentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Uuid> for ExperimentId {
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

impl From<ExperimentId> for Uuid {
    fn from(id: ExperimentId) -> Self {
        id.0
    }
}

/// Newtype pattern for RunId
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(transparent)]
pub struct RunId(pub Uuid);

impl RunId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl Default for RunId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for RunId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Uuid> for RunId {
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

impl From<RunId> for Uuid {
    fn from(id: RunId) -> Self {
        id.0
    }
}

/// Newtype pattern for DatasetId
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(transparent)]
pub struct DatasetId(pub Uuid);

impl DatasetId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl Default for DatasetId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for DatasetId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Uuid> for DatasetId {
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

impl From<DatasetId> for Uuid {
    fn from(id: DatasetId) -> Self {
        id.0
    }
}

/// Newtype pattern for DatasetVersionId
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(transparent)]
pub struct DatasetVersionId(pub Uuid);

impl DatasetVersionId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl Default for DatasetVersionId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for DatasetVersionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Uuid> for DatasetVersionId {
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

impl From<DatasetVersionId> for Uuid {
    fn from(id: DatasetVersionId) -> Self {
        id.0
    }
}

/// Newtype pattern for MetricId
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(transparent)]
pub struct MetricId(pub Uuid);

impl MetricId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl Default for MetricId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for MetricId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Uuid> for MetricId {
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

impl From<MetricId> for Uuid {
    fn from(id: MetricId) -> Self {
        id.0
    }
}

/// Newtype pattern for ArtifactId
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(transparent)]
pub struct ArtifactId(pub Uuid);

impl ArtifactId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl Default for ArtifactId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for ArtifactId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Uuid> for ArtifactId {
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

impl From<ArtifactId> for Uuid {
    fn from(id: ArtifactId) -> Self {
        id.0
    }
}

/// Newtype pattern for UserId
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(transparent)]
pub struct UserId(pub Uuid);

impl UserId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl Default for UserId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for UserId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Uuid> for UserId {
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

impl From<UserId> for Uuid {
    fn from(id: UserId) -> Self {
        id.0
    }
}

/// Newtype pattern for WorkflowId
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(transparent)]
pub struct WorkflowId(pub Uuid);

impl WorkflowId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl Default for WorkflowId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for WorkflowId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Uuid> for WorkflowId {
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

impl From<WorkflowId> for Uuid {
    fn from(id: WorkflowId) -> Self {
        id.0
    }
}

/// Newtype pattern for WorkflowRunId
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(transparent)]
pub struct WorkflowRunId(pub Uuid);

impl WorkflowRunId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl Default for WorkflowRunId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for WorkflowRunId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Uuid> for WorkflowRunId {
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

impl From<WorkflowRunId> for Uuid {
    fn from(id: WorkflowRunId) -> Self {
        id.0
    }
}

/// Content hash for content-addressable storage
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(transparent)]
pub struct ContentHash(pub String);

impl ContentHash {
    /// Create a new content hash from bytes
    pub fn from_bytes(data: &[u8]) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(data);
        let result = hasher.finalize();
        Self(hex::encode(result))
    }

    /// Create a new content hash from a string
    pub fn from_str(data: &str) -> Self {
        Self::from_bytes(data.as_bytes())
    }

    /// Get the hash as a string
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ContentHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for ContentHash {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<ContentHash> for String {
    fn from(hash: ContentHash) -> Self {
        hash.0
    }
}

/// Semantic version following semver specification
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct SemanticVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
    pub pre_release: Option<String>,
    pub build_metadata: Option<String>,
}

impl SemanticVersion {
    pub fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self {
            major,
            minor,
            patch,
            pre_release: None,
            build_metadata: None,
        }
    }

    pub fn with_pre_release(mut self, pre_release: String) -> Self {
        self.pre_release = Some(pre_release);
        self
    }

    pub fn with_build_metadata(mut self, build_metadata: String) -> Self {
        self.build_metadata = Some(build_metadata);
        self
    }

    /// Parse a semantic version from a string
    pub fn parse(s: &str) -> Result<Self, String> {
        let s = s.trim();

        // Split on '+' to separate build metadata
        let (version_part, build_metadata) = if let Some(pos) = s.find('+') {
            let (v, b) = s.split_at(pos);
            (v, Some(b[1..].to_string()))
        } else {
            (s, None)
        };

        // Split on '-' to separate pre-release
        let (core_part, pre_release) = if let Some(pos) = version_part.find('-') {
            let (c, p) = version_part.split_at(pos);
            (c, Some(p[1..].to_string()))
        } else {
            (version_part, None)
        };

        // Parse major.minor.patch
        let parts: Vec<&str> = core_part.split('.').collect();
        if parts.len() != 3 {
            return Err(format!("Invalid version format: {}", s));
        }

        let major = parts[0].parse::<u32>()
            .map_err(|_| format!("Invalid major version: {}", parts[0]))?;
        let minor = parts[1].parse::<u32>()
            .map_err(|_| format!("Invalid minor version: {}", parts[1]))?;
        let patch = parts[2].parse::<u32>()
            .map_err(|_| format!("Invalid patch version: {}", parts[2]))?;

        Ok(Self {
            major,
            minor,
            patch,
            pre_release,
            build_metadata,
        })
    }
}

impl Default for SemanticVersion {
    fn default() -> Self {
        Self::new(0, 1, 0)
    }
}

impl fmt::Display for SemanticVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)?;
        if let Some(ref pre) = self.pre_release {
            write!(f, "-{}", pre)?;
        }
        if let Some(ref build) = self.build_metadata {
            write!(f, "+{}", build)?;
        }
        Ok(())
    }
}

impl PartialOrd for SemanticVersion {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SemanticVersion {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match self.major.cmp(&other.major) {
            std::cmp::Ordering::Equal => {}
            ord => return ord,
        }
        match self.minor.cmp(&other.minor) {
            std::cmp::Ordering::Equal => {}
            ord => return ord,
        }
        match self.patch.cmp(&other.patch) {
            std::cmp::Ordering::Equal => {}
            ord => return ord,
        }

        // Pre-release version has lower precedence than normal version
        match (&self.pre_release, &other.pre_release) {
            (None, None) => std::cmp::Ordering::Equal,
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (Some(a), Some(b)) => a.cmp(b),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_semantic_version_display() {
        let v = SemanticVersion::new(1, 2, 3);
        assert_eq!(v.to_string(), "1.2.3");

        let v = SemanticVersion::new(1, 2, 3)
            .with_pre_release("alpha.1".to_string());
        assert_eq!(v.to_string(), "1.2.3-alpha.1");

        let v = SemanticVersion::new(1, 2, 3)
            .with_pre_release("alpha.1".to_string())
            .with_build_metadata("build.123".to_string());
        assert_eq!(v.to_string(), "1.2.3-alpha.1+build.123");
    }

    #[test]
    fn test_semantic_version_parse() {
        let v = SemanticVersion::parse("1.2.3").unwrap();
        assert_eq!(v.major, 1);
        assert_eq!(v.minor, 2);
        assert_eq!(v.patch, 3);
        assert_eq!(v.pre_release, None);
        assert_eq!(v.build_metadata, None);

        let v = SemanticVersion::parse("1.2.3-alpha.1").unwrap();
        assert_eq!(v.pre_release, Some("alpha.1".to_string()));

        let v = SemanticVersion::parse("1.2.3-alpha.1+build.123").unwrap();
        assert_eq!(v.pre_release, Some("alpha.1".to_string()));
        assert_eq!(v.build_metadata, Some("build.123".to_string()));
    }

    #[test]
    fn test_content_hash() {
        let hash1 = ContentHash::from_str("hello world");
        let hash2 = ContentHash::from_str("hello world");
        let hash3 = ContentHash::from_str("hello world!");

        assert_eq!(hash1, hash2);
        assert_ne!(hash1, hash3);
    }
}
