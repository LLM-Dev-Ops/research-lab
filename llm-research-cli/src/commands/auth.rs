//! Authentication commands

use anyhow::{Context as _, Result};
use clap::{Args, Subcommand};
use dialoguer::{Input, Password, Select};

use crate::config::{AuthMethod, Credentials, ProfileCredentials};
use crate::context::Context;

/// Authentication management commands
#[derive(Debug, Args)]
pub struct AuthCommands {
    #[command(subcommand)]
    pub command: AuthSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum AuthSubcommand {
    /// Log in with credentials
    Login {
        /// Profile to configure
        #[arg(short, long)]
        profile: Option<String>,

        /// API key (can also be set interactively)
        #[arg(long)]
        api_key: Option<String>,

        /// Use system keyring for secure storage
        #[arg(long)]
        use_keyring: bool,
    },

    /// Log out and clear credentials
    Logout {
        /// Profile to clear
        #[arg(short, long)]
        profile: Option<String>,

        /// Clear all profiles
        #[arg(long)]
        all: bool,
    },

    /// Show current authentication status
    Status {
        /// Profile to check
        #[arg(short, long)]
        profile: Option<String>,
    },

    /// Test authentication with the API
    Test {
        /// Profile to test
        #[arg(short, long)]
        profile: Option<String>,
    },
}

/// Execute authentication commands
pub async fn execute(ctx: &Context, cmd: AuthCommands) -> Result<()> {
    match cmd.command {
        AuthSubcommand::Login { profile, api_key, use_keyring } => {
            login(ctx, profile.as_deref(), api_key, use_keyring).await
        }
        AuthSubcommand::Logout { profile, all } => {
            logout(ctx, profile.as_deref(), all).await
        }
        AuthSubcommand::Status { profile } => {
            status(ctx, profile.as_deref()).await
        }
        AuthSubcommand::Test { profile } => {
            test_auth(ctx, profile.as_deref()).await
        }
    }
}

async fn login(
    ctx: &Context,
    profile: Option<&str>,
    api_key: Option<String>,
    use_keyring: bool,
) -> Result<()> {
    let profile_name = profile.unwrap_or("default");
    ctx.output.info(&format!("Configuring authentication for profile: {}", profile_name));

    // Determine auth method
    let auth_types = vec!["API Key", "Bearer Token", "Basic Auth"];
    let selection = Select::new()
        .with_prompt("Select authentication method")
        .items(&auth_types)
        .default(0)
        .interact()
        .context("Failed to get user selection")?;

    let mut config = ctx.config.clone();
    let mut credentials = ctx.credentials.clone();

    match selection {
        0 => {
            // API Key
            let key = if let Some(k) = api_key {
                k
            } else {
                Password::new()
                    .with_prompt("Enter your API key")
                    .interact()
                    .context("Failed to get API key")?
            };

            if use_keyring {
                // Store in system keyring
                let entry = keyring::Entry::new("llm-research-cli", &format!("{}-api-key", profile_name))
                    .context("Failed to create keyring entry")?;
                entry.set_password(&key)
                    .context("Failed to store API key in keyring")?;

                // Update config to use keyring
                let profile_config = config.get_or_create_profile(profile_name);
                profile_config.auth = AuthMethod::ApiKey {
                    key: String::new(),
                    use_keyring: true,
                };
                ctx.output.success("API key stored in system keyring");
            } else {
                // Store in credentials file
                credentials.set(profile_name, ProfileCredentials::api_key(key));
                ctx.output.success("API key stored in credentials file");
            }
        }
        1 => {
            // Bearer Token
            let token = Password::new()
                .with_prompt("Enter your bearer token")
                .interact()
                .context("Failed to get token")?;

            if use_keyring {
                let entry = keyring::Entry::new("llm-research-cli", &format!("{}-token", profile_name))
                    .context("Failed to create keyring entry")?;
                entry.set_password(&token)
                    .context("Failed to store token in keyring")?;

                let profile_config = config.get_or_create_profile(profile_name);
                profile_config.auth = AuthMethod::BearerToken {
                    token: String::new(),
                    use_keyring: true,
                };
                ctx.output.success("Token stored in system keyring");
            } else {
                credentials.set(profile_name, ProfileCredentials::token(token));
                ctx.output.success("Token stored in credentials file");
            }
        }
        2 => {
            // Basic Auth
            let username: String = Input::new()
                .with_prompt("Enter username")
                .interact_text()
                .context("Failed to get username")?;

            let password = Password::new()
                .with_prompt("Enter password")
                .interact()
                .context("Failed to get password")?;

            if use_keyring {
                let entry = keyring::Entry::new("llm-research-cli", &format!("{}-password", profile_name))
                    .context("Failed to create keyring entry")?;
                entry.set_password(&password)
                    .context("Failed to store password in keyring")?;

                let profile_config = config.get_or_create_profile(profile_name);
                profile_config.auth = AuthMethod::Basic {
                    username,
                    password: String::new(),
                    use_keyring: true,
                };
                ctx.output.success("Password stored in system keyring");
            } else {
                let profile_config = config.get_or_create_profile(profile_name);
                profile_config.auth = AuthMethod::Basic {
                    username,
                    password,
                    use_keyring: false,
                };
            }
        }
        _ => unreachable!(),
    }

    // Ask for API URL
    let current_url = config.get_profile(Some(profile_name))
        .and_then(|p| p.api_url.as_deref())
        .unwrap_or(crate::config::DEFAULT_API_URL);

    let api_url: String = Input::new()
        .with_prompt("API URL")
        .default(current_url.to_string())
        .interact_text()
        .context("Failed to get API URL")?;

    let profile_config = config.get_or_create_profile(profile_name);
    profile_config.api_url = Some(api_url);

    // Set as default if no default exists
    if config.default_profile.is_none() {
        config.set_default_profile(profile_name);
        ctx.output.info(&format!("Set '{}' as default profile", profile_name));
    }

    // Save configuration
    config.save().context("Failed to save configuration")?;
    credentials.save().context("Failed to save credentials")?;

    ctx.output.success(&format!("Successfully configured profile '{}'", profile_name));
    Ok(())
}

async fn logout(ctx: &Context, profile: Option<&str>, all: bool) -> Result<()> {
    let mut credentials = ctx.credentials.clone();
    let mut config = ctx.config.clone();

    if all {
        // Clear all credentials
        credentials.profiles.clear();

        // Clear keyring entries for all profiles
        for profile_name in config.list_profiles() {
            clear_keyring_entries(profile_name);
        }

        credentials.save().context("Failed to save credentials")?;
        ctx.output.success("Logged out from all profiles");
    } else {
        let profile_name = profile.unwrap_or("default");

        // Remove from credentials
        credentials.remove(profile_name);

        // Clear keyring entries
        clear_keyring_entries(profile_name);

        // Reset auth in config
        if let Some(p) = config.profiles.get_mut(profile_name) {
            p.auth = AuthMethod::None;
        }

        credentials.save().context("Failed to save credentials")?;
        config.save().context("Failed to save configuration")?;

        ctx.output.success(&format!("Logged out from profile '{}'", profile_name));
    }

    Ok(())
}

fn clear_keyring_entries(profile: &str) {
    // Try to clear keyring entries, ignore errors
    let _ = keyring::Entry::new("llm-research-cli", &format!("{}-api-key", profile))
        .and_then(|e| e.delete_credential());
    let _ = keyring::Entry::new("llm-research-cli", &format!("{}-token", profile))
        .and_then(|e| e.delete_credential());
    let _ = keyring::Entry::new("llm-research-cli", &format!("{}-password", profile))
        .and_then(|e| e.delete_credential());
}

async fn status(ctx: &Context, profile: Option<&str>) -> Result<()> {
    let profile_name = profile
        .or(ctx.profile_name.as_deref())
        .unwrap_or("default");

    println!("Profile: {}", profile_name);

    // Check if profile exists in config
    if let Some(p) = ctx.config.profiles.get(profile_name) {
        println!("API URL: {}", p.api_url());

        match &p.auth {
            AuthMethod::None => {
                println!("Auth: Not configured");
            }
            AuthMethod::ApiKey { use_keyring, .. } => {
                let storage = if *use_keyring { "keyring" } else { "file" };
                println!("Auth: API Key (stored in {})", storage);
            }
            AuthMethod::BearerToken { use_keyring, .. } => {
                let storage = if *use_keyring { "keyring" } else { "file" };
                println!("Auth: Bearer Token (stored in {})", storage);
            }
            AuthMethod::Basic { username, use_keyring, .. } => {
                let storage = if *use_keyring { "keyring" } else { "file" };
                println!("Auth: Basic (user: {}, password in {})", username, storage);
            }
        }

        // Check if credentials exist
        if ctx.credentials.get(profile_name).is_some() {
            println!("Credentials: Found in credentials file");
        }

        // Check if this is the default profile
        if ctx.config.default_profile.as_deref() == Some(profile_name) {
            println!("Default: Yes");
        }
    } else {
        println!("Profile not found. Run 'llm-research auth login --profile {}' to configure.", profile_name);
    }

    Ok(())
}

async fn test_auth(ctx: &Context, _profile: Option<&str>) -> Result<()> {
    let spinner = ctx.output.spinner("Testing authentication...");

    match ctx.create_client() {
        Ok(client) => {
            // Try to make a simple API call
            match client.models().list(None).await {
                Ok(_) => {
                    if let Some(s) = spinner {
                        s.finish_and_clear();
                    }
                    ctx.output.success("Authentication successful!");
                    ctx.output.info(&format!("Connected to: {}", ctx.api_url()));
                }
                Err(e) => {
                    if let Some(s) = spinner {
                        s.finish_and_clear();
                    }
                    ctx.output.error(&format!("Authentication failed: {}", e));
                    return Err(e.into());
                }
            }
        }
        Err(e) => {
            if let Some(s) = spinner {
                s.finish_and_clear();
            }
            ctx.output.error(&format!("Failed to create client: {}", e));
            return Err(e);
        }
    }

    Ok(())
}
