//! Models commands

use anyhow::{Context as _, Result};
use clap::{Args, Subcommand};
use comfy_table::Cell;
use llm_research_sdk::{CreateModelRequest, ListModelsParams};
use serde::Serialize;
use uuid::Uuid;

use crate::context::Context;
use crate::output::{
    format_relative_time, format_uuid_short, print_field, print_optional_field, print_section,
    TableDisplay,
};

/// Model management commands
#[derive(Debug, Args)]
pub struct ModelsCommands {
    #[command(subcommand)]
    pub command: ModelsSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum ModelsSubcommand {
    /// List models
    List {
        /// Maximum number of models to return
        #[arg(short, long, default_value = "20")]
        limit: u32,

        /// Offset for pagination
        #[arg(short, long, default_value = "0")]
        offset: u32,

        /// Filter by provider
        #[arg(short, long)]
        provider: Option<String>,

        /// Filter by name
        #[arg(short, long)]
        name: Option<String>,
    },

    /// Get model details
    Get {
        /// Model ID
        id: Uuid,
    },

    /// Create a new model
    Create {
        /// Model name
        #[arg(short, long)]
        name: String,

        /// Provider name (e.g., openai, anthropic)
        #[arg(short, long)]
        provider: String,

        /// Model identifier (e.g., gpt-4-turbo, claude-3-opus)
        #[arg(short, long)]
        model: String,

        /// Version
        #[arg(short, long)]
        version: Option<String>,

        /// Configuration as JSON
        #[arg(short, long)]
        config: Option<String>,
    },

    /// Update a model
    Update {
        /// Model ID
        id: Uuid,

        /// New name
        #[arg(short, long)]
        name: Option<String>,

        /// New version
        #[arg(short, long)]
        version: Option<String>,

        /// New configuration as JSON
        #[arg(short, long)]
        config: Option<String>,
    },

    /// Delete a model
    Delete {
        /// Model ID
        id: Uuid,

        /// Force deletion without confirmation
        #[arg(short, long)]
        force: bool,
    },

    /// List available model providers
    Providers,
}

/// Execute model commands
pub async fn execute(ctx: &Context, cmd: ModelsCommands) -> Result<()> {
    match cmd.command {
        ModelsSubcommand::List {
            limit,
            offset,
            provider,
            name,
        } => list(ctx, limit, offset, provider, name).await,
        ModelsSubcommand::Get { id } => get(ctx, id).await,
        ModelsSubcommand::Create {
            name,
            provider,
            model,
            version,
            config,
        } => create(ctx, &name, &provider, &model, version, config).await,
        ModelsSubcommand::Update {
            id,
            name,
            version,
            config,
        } => update(ctx, id, name, version, config).await,
        ModelsSubcommand::Delete { id, force } => delete(ctx, id, force).await,
        ModelsSubcommand::Providers => list_providers(ctx).await,
    }
}

/// Displayable model for output
#[derive(Debug, Serialize)]
struct ModelDisplay {
    id: Uuid,
    name: String,
    provider: String,
    model_identifier: String,
    version: Option<String>,
    config: serde_json::Value,
    created_at: String,
    updated_at: String,
}

impl From<llm_research_sdk::Model> for ModelDisplay {
    fn from(m: llm_research_sdk::Model) -> Self {
        Self {
            id: m.id,
            name: m.name,
            provider: m.provider,
            model_identifier: m.model_identifier,
            version: m.version,
            config: m.config,
            created_at: format_relative_time(&m.created_at),
            updated_at: format_relative_time(&m.updated_at),
        }
    }
}

impl TableDisplay for ModelDisplay {
    fn to_row(&self) -> Vec<Cell> {
        vec![
            Cell::new(format_uuid_short(&self.id)),
            Cell::new(&self.name),
            Cell::new(&self.provider),
            Cell::new(&self.model_identifier),
            Cell::new(self.version.as_deref().unwrap_or("-")),
            Cell::new(&self.created_at),
        ]
    }

    fn display_single(&self) {
        print_section("Model");
        print_field("ID", &self.id.to_string());
        print_field("Name", &self.name);
        print_field("Provider", &self.provider);
        print_field("Model", &self.model_identifier);
        print_optional_field("Version", self.version.as_deref());
        print_field("Created", &self.created_at);
        print_field("Updated", &self.updated_at);

        if !self.config.is_null() && self.config != serde_json::json!({}) {
            print_section("Configuration");
            if let Ok(pretty) = serde_json::to_string_pretty(&self.config) {
                for line in pretty.lines() {
                    println!("  {}", line);
                }
            }
        }
    }

    fn display_compact(&self) {
        println!(
            "{}\t{}\t{}\t{}\t{}",
            format_uuid_short(&self.id),
            self.name,
            self.provider,
            self.model_identifier,
            self.version.as_deref().unwrap_or("-")
        );
    }
}

async fn list(
    ctx: &Context,
    limit: u32,
    offset: u32,
    provider: Option<String>,
    name: Option<String>,
) -> Result<()> {
    let client = ctx.create_client()?;

    let mut params = ListModelsParams::new().with_limit(limit).with_offset(offset);

    if let Some(p) = provider {
        params = params.with_provider(p);
    }
    if let Some(n) = name {
        params = params.with_name(n);
    }

    let spinner = ctx.output.spinner("Fetching models...");
    let response = client.models().list(Some(params)).await?;

    if let Some(s) = spinner {
        s.finish_and_clear();
    }

    let models: Vec<ModelDisplay> = response.data.into_iter().map(Into::into).collect();
    ctx.output.write_list(
        &models,
        &["ID", "Name", "Provider", "Model", "Version", "Created"],
    )?;

    if response.pagination.has_more {
        ctx.output.info(&format!(
            "Showing {} of {} total models. Use --offset to paginate.",
            models.len(),
            response.pagination.total
        ));
    }

    Ok(())
}

async fn get(ctx: &Context, id: Uuid) -> Result<()> {
    let client = ctx.create_client()?;

    let spinner = ctx.output.spinner("Fetching model...");
    let model = client.models().get(id).await?;

    if let Some(s) = spinner {
        s.finish_and_clear();
    }

    let display: ModelDisplay = model.into();
    ctx.output.write(&display)?;

    Ok(())
}

async fn create(
    ctx: &Context,
    name: &str,
    provider: &str,
    model: &str,
    version: Option<String>,
    config: Option<String>,
) -> Result<()> {
    let client = ctx.create_client()?;

    let mut request = CreateModelRequest::new(name, provider, model);

    if let Some(v) = version {
        request = request.with_version(v);
    }

    if let Some(c) = config {
        let config_json: serde_json::Value =
            serde_json::from_str(&c).context("Invalid JSON for config")?;
        request = request.with_config(config_json);
    }

    let spinner = ctx.output.spinner("Creating model...");
    let model = client.models().create(request).await?;

    if let Some(s) = spinner {
        s.finish_and_clear();
    }

    ctx.output.success(&format!("Created model: {}", model.id));

    let display: ModelDisplay = model.into();
    ctx.output.write(&display)?;

    Ok(())
}

async fn update(
    ctx: &Context,
    id: Uuid,
    name: Option<String>,
    version: Option<String>,
    config: Option<String>,
) -> Result<()> {
    let client = ctx.create_client()?;

    let mut request = llm_research_sdk::UpdateModelRequest::new();

    if let Some(n) = name {
        request = request.with_name(n);
    }
    if let Some(v) = version {
        request = request.with_version(v);
    }
    if let Some(c) = config {
        let config_json: serde_json::Value =
            serde_json::from_str(&c).context("Invalid JSON for config")?;
        request = request.with_config(config_json);
    }

    let spinner = ctx.output.spinner("Updating model...");
    let model = client.models().update(id, request).await?;

    if let Some(s) = spinner {
        s.finish_and_clear();
    }

    ctx.output.success("Model updated");

    let display: ModelDisplay = model.into();
    ctx.output.write(&display)?;

    Ok(())
}

async fn delete(ctx: &Context, id: Uuid, force: bool) -> Result<()> {
    if !force {
        let confirm = dialoguer::Confirm::new()
            .with_prompt(format!("Delete model {}?", id))
            .default(false)
            .interact()
            .context("Failed to get confirmation")?;

        if !confirm {
            ctx.output.info("Cancelled");
            return Ok(());
        }
    }

    let client = ctx.create_client()?;

    let spinner = ctx.output.spinner("Deleting model...");
    client.models().delete(id).await?;

    if let Some(s) = spinner {
        s.finish_and_clear();
    }

    ctx.output.success(&format!("Deleted model: {}", id));

    Ok(())
}

async fn list_providers(ctx: &Context) -> Result<()> {
    let client = ctx.create_client()?;

    let spinner = ctx.output.spinner("Fetching providers...");
    let providers = client.models().list_providers().await?;

    if let Some(s) = spinner {
        s.finish_and_clear();
    }

    if providers.is_empty() {
        ctx.output.info("No providers available");
        return Ok(());
    }

    print_section("Available Providers");

    for provider in providers {
        println!();
        print_field("Name", &provider.name);
        print_field("Display Name", &provider.display_name);
        print_optional_field("Description", provider.description.as_deref());

        if !provider.supported_models.is_empty() {
            println!("  Supported Models:");
            for model in &provider.supported_models {
                println!("    - {}", model);
            }
        }
    }

    Ok(())
}
