//! CLI execution context

use anyhow::{Context as _, Result};
use llm_research_sdk::{AuthConfig, LlmResearchClient, SdkConfig};
use std::time::Duration;

use crate::cli::Cli;
use crate::config::{AuthMethod, CliConfig, Credentials, Profile};
use crate::output::{OutputFormat, OutputWriter};

/// Execution context for CLI commands
pub struct Context {
    /// CLI configuration
    pub config: CliConfig,

    /// Credentials storage
    pub credentials: Credentials,

    /// Active profile name
    pub profile_name: Option<String>,

    /// Active profile
    pub profile: Profile,

    /// Output format
    pub output_format: OutputFormat,

    /// Output writer
    pub output: OutputWriter,

    /// Verbose mode
    pub verbose: bool,

    /// API URL override
    pub api_url_override: Option<String>,

    /// API key override
    pub api_key_override: Option<String>,
}

impl Context {
    /// Create a new context from CLI arguments
    pub fn new(cli: &Cli) -> Result<Self> {
        // Load configuration
        let config = CliConfig::load().unwrap_or_default();
        let credentials = Credentials::load().unwrap_or_default();

        // Determine active profile
        let profile_name = cli.profile.clone().or_else(|| config.default_profile.clone());
        let profile = config.get_profile(profile_name.as_deref())
            .cloned()
            .unwrap_or_default();

        // Determine output format
        let output_format = cli.output;
        let output = OutputWriter::new(output_format, cli.no_color);

        Ok(Self {
            config,
            credentials,
            profile_name,
            profile,
            output_format,
            output,
            verbose: cli.verbose,
            api_url_override: cli.api_url.clone(),
            api_key_override: cli.api_key.clone(),
        })
    }

    /// Get the effective API URL
    pub fn api_url(&self) -> &str {
        self.api_url_override.as_deref()
            .or(self.profile.api_url.as_deref())
            .unwrap_or(crate::config::DEFAULT_API_URL)
    }

    /// Get the SDK authentication configuration
    pub fn get_auth_config(&self) -> Result<AuthConfig> {
        // Check for CLI override
        if let Some(ref api_key) = self.api_key_override {
            return Ok(AuthConfig::ApiKey(api_key.clone()));
        }

        // Get credentials from profile
        let profile_name = self.profile_name.as_deref().unwrap_or("default");

        // Check stored credentials first
        if let Some(creds) = self.credentials.get(profile_name) {
            if let Some(ref key) = creds.api_key {
                return Ok(AuthConfig::ApiKey(key.clone()));
            }
            if let Some(ref token) = creds.token {
                return Ok(AuthConfig::BearerToken(token.clone()));
            }
        }

        // Fall back to profile auth configuration
        match &self.profile.auth {
            AuthMethod::None => Ok(AuthConfig::None),
            AuthMethod::ApiKey { key, use_keyring } => {
                if *use_keyring {
                    self.get_keyring_api_key(profile_name)
                } else {
                    Ok(AuthConfig::ApiKey(key.clone()))
                }
            }
            AuthMethod::BearerToken { token, use_keyring } => {
                if *use_keyring {
                    self.get_keyring_token(profile_name)
                } else {
                    Ok(AuthConfig::BearerToken(token.clone()))
                }
            }
            AuthMethod::Basic { username, password, use_keyring } => {
                if *use_keyring {
                    self.get_keyring_basic(profile_name, username)
                } else {
                    Ok(AuthConfig::Basic {
                        username: username.clone(),
                        password: password.clone(),
                    })
                }
            }
        }
    }

    /// Get API key from system keyring
    fn get_keyring_api_key(&self, profile: &str) -> Result<AuthConfig> {
        let entry = keyring::Entry::new("llm-research-cli", &format!("{}-api-key", profile))
            .context("Failed to access keyring")?;
        let key = entry.get_password()
            .context("API key not found in keyring. Run 'llm-research auth login' to set credentials.")?;
        Ok(AuthConfig::ApiKey(key))
    }

    /// Get token from system keyring
    fn get_keyring_token(&self, profile: &str) -> Result<AuthConfig> {
        let entry = keyring::Entry::new("llm-research-cli", &format!("{}-token", profile))
            .context("Failed to access keyring")?;
        let token = entry.get_password()
            .context("Token not found in keyring. Run 'llm-research auth login' to set credentials.")?;
        Ok(AuthConfig::BearerToken(token))
    }

    /// Get basic auth from system keyring
    fn get_keyring_basic(&self, profile: &str, username: &str) -> Result<AuthConfig> {
        let entry = keyring::Entry::new("llm-research-cli", &format!("{}-password", profile))
            .context("Failed to access keyring")?;
        let password = entry.get_password()
            .context("Password not found in keyring. Run 'llm-research auth login' to set credentials.")?;
        Ok(AuthConfig::Basic {
            username: username.to_string(),
            password,
        })
    }

    /// Create an SDK client
    pub fn create_client(&self) -> Result<LlmResearchClient> {
        let auth = self.get_auth_config()?;
        let timeout = Duration::from_secs(self.config.settings.timeout_secs);

        let mut config = SdkConfig::new(self.api_url())
            .with_auth(auth)
            .with_timeout(timeout)
            .with_max_retries(self.config.settings.max_retries);

        if self.verbose {
            config = config.with_logging(true);
        }

        // Add custom headers from profile
        for (name, value) in &self.profile.headers {
            config = config.with_header(name.clone(), value.clone());
        }

        LlmResearchClient::new(config).context("Failed to create API client")
    }

    /// Check if we have valid authentication
    pub fn has_auth(&self) -> bool {
        if self.api_key_override.is_some() {
            return true;
        }

        let profile_name = self.profile_name.as_deref().unwrap_or("default");
        if let Some(creds) = self.credentials.get(profile_name) {
            if creds.api_key.is_some() || creds.token.is_some() {
                return true;
            }
        }

        !matches!(self.profile.auth, AuthMethod::None)
    }

    /// Save the current configuration
    pub fn save_config(&self) -> Result<()> {
        self.config.save()
    }

    /// Save credentials
    pub fn save_credentials(&self) -> Result<()> {
        self.credentials.save()
    }
}
