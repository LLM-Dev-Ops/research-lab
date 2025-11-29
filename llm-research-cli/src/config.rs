//! CLI configuration management

use anyhow::{Context as _, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// Default API URL
pub const DEFAULT_API_URL: &str = "https://api.llm-research.example.com";

/// CLI configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CliConfig {
    /// Default profile to use
    #[serde(default)]
    pub default_profile: Option<String>,

    /// Named profiles
    #[serde(default)]
    pub profiles: HashMap<String, Profile>,

    /// Global settings
    #[serde(default)]
    pub settings: Settings,
}

impl CliConfig {
    /// Load configuration from the default location
    pub fn load() -> Result<Self> {
        let path = Self::config_path()?;
        if path.exists() {
            let content = fs::read_to_string(&path)
                .with_context(|| format!("Failed to read config from {:?}", path))?;
            let config: CliConfig = toml::from_str(&content)
                .with_context(|| format!("Failed to parse config from {:?}", path))?;
            Ok(config)
        } else {
            Ok(Self::default())
        }
    }

    /// Save configuration to the default location
    pub fn save(&self) -> Result<()> {
        let path = Self::config_path()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create config directory {:?}", parent))?;
        }
        let content = toml::to_string_pretty(self)
            .context("Failed to serialize config")?;
        fs::write(&path, content)
            .with_context(|| format!("Failed to write config to {:?}", path))?;
        Ok(())
    }

    /// Get the configuration file path
    pub fn config_path() -> Result<PathBuf> {
        let dirs = ProjectDirs::from("com", "llm-research", "llm-research-cli")
            .context("Could not determine config directory")?;
        Ok(dirs.config_dir().join("config.toml"))
    }

    /// Get the credentials file path
    pub fn credentials_path() -> Result<PathBuf> {
        let dirs = ProjectDirs::from("com", "llm-research", "llm-research-cli")
            .context("Could not determine config directory")?;
        Ok(dirs.config_dir().join("credentials.toml"))
    }

    /// Get a profile by name
    pub fn get_profile(&self, name: Option<&str>) -> Option<&Profile> {
        let profile_name = name.or(self.default_profile.as_deref())?;
        self.profiles.get(profile_name)
    }

    /// Get or create a profile
    pub fn get_or_create_profile(&mut self, name: &str) -> &mut Profile {
        self.profiles.entry(name.to_string()).or_insert_with(Profile::default)
    }

    /// Set the default profile
    pub fn set_default_profile(&mut self, name: &str) {
        self.default_profile = Some(name.to_string());
    }

    /// Remove a profile
    pub fn remove_profile(&mut self, name: &str) -> Option<Profile> {
        if self.default_profile.as_deref() == Some(name) {
            self.default_profile = None;
        }
        self.profiles.remove(name)
    }

    /// List all profile names
    pub fn list_profiles(&self) -> Vec<&str> {
        self.profiles.keys().map(|s| s.as_str()).collect()
    }
}

/// A configuration profile
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Profile {
    /// API base URL
    #[serde(default)]
    pub api_url: Option<String>,

    /// Authentication method
    #[serde(default)]
    pub auth: AuthMethod,

    /// Default output format
    #[serde(default)]
    pub output_format: Option<String>,

    /// Additional headers
    #[serde(default)]
    pub headers: HashMap<String, String>,
}

impl Profile {
    /// Get the API URL, falling back to default
    pub fn api_url(&self) -> &str {
        self.api_url.as_deref().unwrap_or(DEFAULT_API_URL)
    }
}

/// Authentication method configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AuthMethod {
    /// No authentication
    #[default]
    None,
    /// API key authentication
    ApiKey {
        /// The API key (or reference to keyring)
        key: String,
        /// Whether the key is stored in system keyring
        #[serde(default)]
        use_keyring: bool,
    },
    /// Bearer token authentication
    BearerToken {
        /// The token (or reference to keyring)
        token: String,
        /// Whether the token is stored in system keyring
        #[serde(default)]
        use_keyring: bool,
    },
    /// Basic authentication
    Basic {
        /// Username
        username: String,
        /// Password (or reference to keyring)
        password: String,
        /// Whether the password is stored in system keyring
        #[serde(default)]
        use_keyring: bool,
    },
}

/// Global settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    /// Default output format
    #[serde(default = "default_output_format")]
    pub output_format: String,

    /// Enable colored output
    #[serde(default = "default_true")]
    pub color: bool,

    /// Enable verbose output by default
    #[serde(default)]
    pub verbose: bool,

    /// Request timeout in seconds
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,

    /// Maximum retries for failed requests
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            output_format: default_output_format(),
            color: true,
            verbose: false,
            timeout_secs: default_timeout(),
            max_retries: default_max_retries(),
        }
    }
}

fn default_output_format() -> String {
    "table".to_string()
}

fn default_true() -> bool {
    true
}

fn default_timeout() -> u64 {
    30
}

fn default_max_retries() -> u32 {
    3
}

/// Credential storage
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Credentials {
    /// Stored credentials by profile name
    #[serde(default)]
    pub profiles: HashMap<String, ProfileCredentials>,
}

impl Credentials {
    /// Load credentials from the default location
    pub fn load() -> Result<Self> {
        let path = CliConfig::credentials_path()?;
        if path.exists() {
            let content = fs::read_to_string(&path)
                .with_context(|| format!("Failed to read credentials from {:?}", path))?;
            let creds: Credentials = toml::from_str(&content)
                .with_context(|| format!("Failed to parse credentials from {:?}", path))?;
            Ok(creds)
        } else {
            Ok(Self::default())
        }
    }

    /// Save credentials to the default location
    pub fn save(&self) -> Result<()> {
        let path = CliConfig::credentials_path()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create credentials directory {:?}", parent))?;
        }

        let content = toml::to_string_pretty(self)
            .context("Failed to serialize credentials")?;

        // Set restrictive permissions on the credentials file
        fs::write(&path, content)
            .with_context(|| format!("Failed to write credentials to {:?}", path))?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&path)?.permissions();
            perms.set_mode(0o600);
            fs::set_permissions(&path, perms)?;
        }

        Ok(())
    }

    /// Get credentials for a profile
    pub fn get(&self, profile: &str) -> Option<&ProfileCredentials> {
        self.profiles.get(profile)
    }

    /// Set credentials for a profile
    pub fn set(&mut self, profile: &str, creds: ProfileCredentials) {
        self.profiles.insert(profile.to_string(), creds);
    }

    /// Remove credentials for a profile
    pub fn remove(&mut self, profile: &str) -> Option<ProfileCredentials> {
        self.profiles.remove(profile)
    }
}

/// Credentials for a single profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileCredentials {
    /// API key
    pub api_key: Option<String>,
    /// Bearer token
    pub token: Option<String>,
    /// Basic auth password
    pub password: Option<String>,
}

impl ProfileCredentials {
    /// Create credentials with an API key
    pub fn api_key(key: String) -> Self {
        Self {
            api_key: Some(key),
            token: None,
            password: None,
        }
    }

    /// Create credentials with a bearer token
    pub fn token(token: String) -> Self {
        Self {
            api_key: None,
            token: Some(token),
            password: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = CliConfig::default();
        assert!(config.default_profile.is_none());
        assert!(config.profiles.is_empty());
    }

    #[test]
    fn test_profile_api_url() {
        let profile = Profile::default();
        assert_eq!(profile.api_url(), DEFAULT_API_URL);

        let profile = Profile {
            api_url: Some("https://custom.example.com".to_string()),
            ..Default::default()
        };
        assert_eq!(profile.api_url(), "https://custom.example.com");
    }

    #[test]
    fn test_settings_defaults() {
        let settings = Settings::default();
        assert_eq!(settings.output_format, "table");
        assert!(settings.color);
        assert!(!settings.verbose);
        assert_eq!(settings.timeout_secs, 30);
        assert_eq!(settings.max_retries, 3);
    }
}
