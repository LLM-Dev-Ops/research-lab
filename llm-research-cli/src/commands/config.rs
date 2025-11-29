//! Configuration commands

use anyhow::{Context as _, Result};
use clap::{Args, Subcommand};
use colored::Colorize;

use crate::config::CliConfig;
use crate::context::Context;

/// Configuration management commands
#[derive(Debug, Args)]
pub struct ConfigCommands {
    #[command(subcommand)]
    pub command: ConfigSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum ConfigSubcommand {
    /// Show current configuration
    Show {
        /// Show configuration for a specific profile
        #[arg(short, long)]
        profile: Option<String>,
    },

    /// Set a configuration value
    Set {
        /// Configuration key (e.g., settings.timeout_secs)
        key: String,

        /// Value to set
        value: String,

        /// Profile to configure
        #[arg(short, long)]
        profile: Option<String>,
    },

    /// Get a configuration value
    Get {
        /// Configuration key
        key: String,

        /// Profile to read from
        #[arg(short, long)]
        profile: Option<String>,
    },

    /// List all profiles
    Profiles,

    /// Set the default profile
    UseProfile {
        /// Profile name to use as default
        name: String,
    },

    /// Create a new profile
    CreateProfile {
        /// Profile name
        name: String,

        /// API URL for this profile
        #[arg(long)]
        api_url: Option<String>,

        /// Copy settings from another profile
        #[arg(long)]
        from: Option<String>,
    },

    /// Delete a profile
    DeleteProfile {
        /// Profile name to delete
        name: String,

        /// Force deletion without confirmation
        #[arg(short, long)]
        force: bool,
    },

    /// Show configuration file paths
    Path,

    /// Reset configuration to defaults
    Reset {
        /// Force reset without confirmation
        #[arg(short, long)]
        force: bool,
    },
}

/// Execute configuration commands
pub async fn execute(ctx: &Context, cmd: ConfigCommands) -> Result<()> {
    match cmd.command {
        ConfigSubcommand::Show { profile } => show(ctx, profile.as_deref()).await,
        ConfigSubcommand::Set { key, value, profile } => {
            set(ctx, &key, &value, profile.as_deref()).await
        }
        ConfigSubcommand::Get { key, profile } => get(ctx, &key, profile.as_deref()).await,
        ConfigSubcommand::Profiles => list_profiles(ctx).await,
        ConfigSubcommand::UseProfile { name } => use_profile(ctx, &name).await,
        ConfigSubcommand::CreateProfile { name, api_url, from } => {
            create_profile(ctx, &name, api_url.as_deref(), from.as_deref()).await
        }
        ConfigSubcommand::DeleteProfile { name, force } => {
            delete_profile(ctx, &name, force).await
        }
        ConfigSubcommand::Path => show_paths().await,
        ConfigSubcommand::Reset { force } => reset(ctx, force).await,
    }
}

async fn show(ctx: &Context, profile: Option<&str>) -> Result<()> {
    println!("{}", "Configuration".bold().underline());
    println!();

    // Show global settings
    println!("{}", "Settings:".cyan());
    println!("  output_format: {}", ctx.config.settings.output_format);
    println!("  color: {}", ctx.config.settings.color);
    println!("  verbose: {}", ctx.config.settings.verbose);
    println!("  timeout_secs: {}", ctx.config.settings.timeout_secs);
    println!("  max_retries: {}", ctx.config.settings.max_retries);

    if let Some(default) = &ctx.config.default_profile {
        println!();
        println!("{}: {}", "Default profile".cyan(), default);
    }

    println!();
    println!("{}", "Profiles:".cyan());

    if ctx.config.profiles.is_empty() {
        println!("  No profiles configured");
    } else if let Some(profile_name) = profile {
        // Show specific profile
        if let Some(p) = ctx.config.profiles.get(profile_name) {
            println!("  [{}]", profile_name);
            println!("    api_url: {}", p.api_url());
            println!("    auth: {:?}", p.auth);
            if !p.headers.is_empty() {
                println!("    headers:");
                for (k, v) in &p.headers {
                    println!("      {}: {}", k, v);
                }
            }
        } else {
            println!("  Profile '{}' not found", profile_name);
        }
    } else {
        // Show all profiles
        for (name, p) in &ctx.config.profiles {
            let default_marker = if ctx.config.default_profile.as_deref() == Some(name) {
                " (default)".green().to_string()
            } else {
                String::new()
            };
            println!("  [{}]{}", name, default_marker);
            println!("    api_url: {}", p.api_url());
        }
    }

    Ok(())
}

async fn set(ctx: &Context, key: &str, value: &str, profile: Option<&str>) -> Result<()> {
    let mut config = ctx.config.clone();

    let parts: Vec<&str> = key.split('.').collect();

    match parts.as_slice() {
        ["settings", setting] => {
            match *setting {
                "output_format" => config.settings.output_format = value.to_string(),
                "color" => config.settings.color = value.parse().context("Invalid boolean value")?,
                "verbose" => config.settings.verbose = value.parse().context("Invalid boolean value")?,
                "timeout_secs" => config.settings.timeout_secs = value.parse().context("Invalid number")?,
                "max_retries" => config.settings.max_retries = value.parse().context("Invalid number")?,
                _ => anyhow::bail!("Unknown setting: {}", setting),
            }
        }
        ["profile", pname, field] => {
            let p = config.get_or_create_profile(*pname);
            match *field {
                "api_url" => p.api_url = Some(value.to_string()),
                "output_format" => p.output_format = Some(value.to_string()),
                _ => anyhow::bail!("Unknown profile field: {}", field),
            }
        }
        [field] if profile.is_some() => {
            let profile_name = profile.unwrap();
            let p = config.get_or_create_profile(profile_name);
            match *field {
                "api_url" => p.api_url = Some(value.to_string()),
                "output_format" => p.output_format = Some(value.to_string()),
                _ => anyhow::bail!("Unknown profile field: {}", field),
            }
        }
        _ => anyhow::bail!("Unknown configuration key: {}", key),
    }

    config.save().context("Failed to save configuration")?;
    ctx.output.success(&format!("Set {} = {}", key, value));

    Ok(())
}

async fn get(ctx: &Context, key: &str, profile: Option<&str>) -> Result<()> {
    let parts: Vec<&str> = key.split('.').collect();

    let value = match parts.as_slice() {
        ["settings", setting] => {
            match *setting {
                "output_format" => ctx.config.settings.output_format.clone(),
                "color" => ctx.config.settings.color.to_string(),
                "verbose" => ctx.config.settings.verbose.to_string(),
                "timeout_secs" => ctx.config.settings.timeout_secs.to_string(),
                "max_retries" => ctx.config.settings.max_retries.to_string(),
                _ => anyhow::bail!("Unknown setting: {}", setting),
            }
        }
        ["profile", pname, field] => {
            let p = ctx.config.get_profile(Some(*pname))
                .context(format!("Profile '{}' not found", pname))?;
            match *field {
                "api_url" => p.api_url().to_string(),
                "output_format" => p.output_format.clone().unwrap_or_default(),
                _ => anyhow::bail!("Unknown profile field: {}", field),
            }
        }
        [field] if profile.is_some() => {
            let profile_name = profile.unwrap();
            let p = ctx.config.get_profile(Some(profile_name))
                .context(format!("Profile '{}' not found", profile_name))?;
            match *field {
                "api_url" => p.api_url().to_string(),
                "output_format" => p.output_format.clone().unwrap_or_default(),
                _ => anyhow::bail!("Unknown profile field: {}", field),
            }
        }
        ["default_profile"] => {
            ctx.config.default_profile.clone().unwrap_or_else(|| "not set".to_string())
        }
        _ => anyhow::bail!("Unknown configuration key: {}", key),
    };

    println!("{}", value);
    Ok(())
}

async fn list_profiles(ctx: &Context) -> Result<()> {
    if ctx.config.profiles.is_empty() {
        ctx.output.info("No profiles configured. Run 'llm-research auth login' to create one.");
        return Ok(());
    }

    println!("{}", "Configured profiles:".bold());
    println!();

    for name in ctx.config.list_profiles() {
        let is_default = ctx.config.default_profile.as_deref() == Some(name);
        if is_default {
            println!("  {} {}", "→".green(), name.green().bold());
        } else {
            println!("    {}", name);
        }
    }

    Ok(())
}

async fn use_profile(ctx: &Context, name: &str) -> Result<()> {
    let mut config = ctx.config.clone();

    if !config.profiles.contains_key(name) {
        anyhow::bail!("Profile '{}' not found. Run 'llm-research config profiles' to list available profiles.", name);
    }

    config.set_default_profile(name);
    config.save().context("Failed to save configuration")?;

    ctx.output.success(&format!("Now using profile '{}'", name));
    Ok(())
}

async fn create_profile(
    ctx: &Context,
    name: &str,
    api_url: Option<&str>,
    from: Option<&str>,
) -> Result<()> {
    let mut config = ctx.config.clone();

    if config.profiles.contains_key(name) {
        anyhow::bail!("Profile '{}' already exists", name);
    }

    let mut new_profile = if let Some(source) = from {
        config.get_profile(Some(source))
            .cloned()
            .context(format!("Source profile '{}' not found", source))?
    } else {
        crate::config::Profile::default()
    };

    if let Some(url) = api_url {
        new_profile.api_url = Some(url.to_string());
    }

    config.profiles.insert(name.to_string(), new_profile);
    config.save().context("Failed to save configuration")?;

    ctx.output.success(&format!("Created profile '{}'", name));

    if from.is_some() {
        ctx.output.info(&format!("Copied settings from '{}'", from.unwrap()));
    }

    Ok(())
}

async fn delete_profile(ctx: &Context, name: &str, force: bool) -> Result<()> {
    let mut config = ctx.config.clone();

    if !config.profiles.contains_key(name) {
        anyhow::bail!("Profile '{}' not found", name);
    }

    if !force {
        let confirm = dialoguer::Confirm::new()
            .with_prompt(format!("Delete profile '{}'?", name))
            .default(false)
            .interact()
            .context("Failed to get confirmation")?;

        if !confirm {
            ctx.output.info("Cancelled");
            return Ok(());
        }
    }

    config.remove_profile(name);
    config.save().context("Failed to save configuration")?;

    // Also remove credentials
    let mut credentials = ctx.credentials.clone();
    credentials.remove(name);
    credentials.save().context("Failed to save credentials")?;

    ctx.output.success(&format!("Deleted profile '{}'", name));
    Ok(())
}

async fn show_paths() -> Result<()> {
    println!("{}", "Configuration paths:".bold());
    println!();

    match CliConfig::config_path() {
        Ok(path) => {
            let exists = path.exists();
            let status = if exists { "✓".green() } else { "✗".red() };
            println!("  Config:      {} {}", status, path.display());
        }
        Err(e) => println!("  Config:      Error: {}", e),
    }

    match CliConfig::credentials_path() {
        Ok(path) => {
            let exists = path.exists();
            let status = if exists { "✓".green() } else { "✗".red() };
            println!("  Credentials: {} {}", status, path.display());
        }
        Err(e) => println!("  Credentials: Error: {}", e),
    }

    Ok(())
}

async fn reset(ctx: &Context, force: bool) -> Result<()> {
    if !force {
        let confirm = dialoguer::Confirm::new()
            .with_prompt("Reset all configuration to defaults? This cannot be undone.")
            .default(false)
            .interact()
            .context("Failed to get confirmation")?;

        if !confirm {
            ctx.output.info("Cancelled");
            return Ok(());
        }
    }

    let config = CliConfig::default();
    config.save().context("Failed to save configuration")?;

    let credentials = crate::config::Credentials::default();
    credentials.save().context("Failed to save credentials")?;

    ctx.output.success("Configuration reset to defaults");
    Ok(())
}
