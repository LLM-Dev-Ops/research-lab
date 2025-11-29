//! Prompts commands

use anyhow::{Context as _, Result};
use clap::{Args, Subcommand};
use comfy_table::Cell;
use llm_research_sdk::{CreatePromptRequest, ListPromptsParams, PromptVariable};
use serde::Serialize;
use std::collections::HashMap;
use uuid::Uuid;

use crate::context::Context;
use crate::output::{
    format_relative_time, format_uuid_short, print_field, print_list_field, print_optional_field,
    print_section, TableDisplay,
};

/// Prompt template management commands
#[derive(Debug, Args)]
pub struct PromptsCommands {
    #[command(subcommand)]
    pub command: PromptsSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum PromptsSubcommand {
    /// List prompt templates
    List {
        /// Maximum number of prompts to return
        #[arg(short, long, default_value = "20")]
        limit: u32,

        /// Offset for pagination
        #[arg(short, long, default_value = "0")]
        offset: u32,

        /// Filter by tags (comma-separated)
        #[arg(short, long)]
        tags: Option<String>,

        /// Search by name or description
        #[arg(short, long)]
        search: Option<String>,
    },

    /// Get prompt template details
    Get {
        /// Prompt template ID
        id: Uuid,
    },

    /// Create a new prompt template
    Create {
        /// Template name
        #[arg(short, long)]
        name: String,

        /// Template content (use {{variable}} for variables)
        #[arg(short, long)]
        template: String,

        /// Description
        #[arg(short, long)]
        description: Option<String>,

        /// System prompt
        #[arg(long)]
        system: Option<String>,

        /// Tags (comma-separated)
        #[arg(long)]
        tags: Option<String>,
    },

    /// Update a prompt template
    Update {
        /// Prompt template ID
        id: Uuid,

        /// New name
        #[arg(short, long)]
        name: Option<String>,

        /// New description
        #[arg(short, long)]
        description: Option<String>,

        /// New tags (comma-separated)
        #[arg(short, long)]
        tags: Option<String>,
    },

    /// Delete a prompt template
    Delete {
        /// Prompt template ID
        id: Uuid,

        /// Force deletion without confirmation
        #[arg(short, long)]
        force: bool,
    },

    /// List versions of a prompt template
    Versions {
        /// Prompt template ID
        id: Uuid,

        /// Maximum number of versions to return
        #[arg(short, long, default_value = "10")]
        limit: u32,
    },

    /// Create a new version of a prompt template
    CreateVersion {
        /// Prompt template ID
        id: Uuid,

        /// New template content
        #[arg(short, long)]
        template: String,

        /// New system prompt
        #[arg(long)]
        system: Option<String>,

        /// Changelog
        #[arg(short, long)]
        changelog: Option<String>,
    },

    /// Render a prompt with variables
    Render {
        /// Prompt template ID
        id: Uuid,

        /// Variables as JSON object
        #[arg(short, long)]
        vars: String,

        /// Specific version ID to use
        #[arg(long)]
        version: Option<Uuid>,
    },

    /// Validate a prompt template
    Validate {
        /// Template content to validate
        #[arg(short, long)]
        template: String,
    },
}

/// Execute prompt commands
pub async fn execute(ctx: &Context, cmd: PromptsCommands) -> Result<()> {
    match cmd.command {
        PromptsSubcommand::List {
            limit,
            offset,
            tags,
            search,
        } => list(ctx, limit, offset, tags, search).await,
        PromptsSubcommand::Get { id } => get(ctx, id).await,
        PromptsSubcommand::Create {
            name,
            template,
            description,
            system,
            tags,
        } => create(ctx, &name, &template, description, system, tags).await,
        PromptsSubcommand::Update {
            id,
            name,
            description,
            tags,
        } => update(ctx, id, name, description, tags).await,
        PromptsSubcommand::Delete { id, force } => delete(ctx, id, force).await,
        PromptsSubcommand::Versions { id, limit } => list_versions(ctx, id, limit).await,
        PromptsSubcommand::CreateVersion {
            id,
            template,
            system,
            changelog,
        } => create_version(ctx, id, &template, system, changelog).await,
        PromptsSubcommand::Render { id, vars, version } => render(ctx, id, &vars, version).await,
        PromptsSubcommand::Validate { template } => validate(ctx, &template).await,
    }
}

/// Displayable prompt for output
#[derive(Debug, Serialize)]
struct PromptDisplay {
    id: Uuid,
    name: String,
    description: Option<String>,
    template: String,
    system_prompt: Option<String>,
    variables: Vec<VariableDisplay>,
    tags: Vec<String>,
    version_count: u32,
    created_at: String,
    updated_at: String,
}

#[derive(Debug, Serialize)]
struct VariableDisplay {
    name: String,
    variable_type: String,
    required: bool,
    description: Option<String>,
}

impl From<PromptVariable> for VariableDisplay {
    fn from(v: PromptVariable) -> Self {
        Self {
            name: v.name,
            variable_type: v.variable_type.to_string(),
            required: v.required,
            description: v.description,
        }
    }
}

impl From<llm_research_sdk::PromptTemplate> for PromptDisplay {
    fn from(p: llm_research_sdk::PromptTemplate) -> Self {
        Self {
            id: p.id,
            name: p.name,
            description: p.description,
            template: p.template,
            system_prompt: p.system_prompt,
            variables: p.variables.into_iter().map(Into::into).collect(),
            tags: p.tags,
            version_count: p.version_count,
            created_at: format_relative_time(&p.created_at),
            updated_at: format_relative_time(&p.updated_at),
        }
    }
}

impl TableDisplay for PromptDisplay {
    fn to_row(&self) -> Vec<Cell> {
        vec![
            Cell::new(format_uuid_short(&self.id)),
            Cell::new(&self.name),
            Cell::new(self.variables.len().to_string()),
            Cell::new(self.version_count.to_string()),
            Cell::new(self.tags.join(", ")),
            Cell::new(&self.created_at),
        ]
    }

    fn display_single(&self) {
        print_section("Prompt Template");
        print_field("ID", &self.id.to_string());
        print_field("Name", &self.name);
        print_optional_field("Description", self.description.as_deref());
        print_list_field("Tags", &self.tags);
        print_field("Versions", &self.version_count.to_string());
        print_field("Created", &self.created_at);
        print_field("Updated", &self.updated_at);

        print_section("Template");
        for line in self.template.lines() {
            println!("  {}", line);
        }

        if let Some(ref system) = self.system_prompt {
            print_section("System Prompt");
            for line in system.lines() {
                println!("  {}", line);
            }
        }

        if !self.variables.is_empty() {
            print_section("Variables");
            for var in &self.variables {
                let required = if var.required { " (required)" } else { "" };
                println!(
                    "  {} [{}]{}",
                    var.name, var.variable_type, required
                );
                if let Some(ref desc) = var.description {
                    println!("    {}", desc);
                }
            }
        }
    }

    fn display_compact(&self) {
        println!(
            "{}\t{}\t{} vars\t{}",
            format_uuid_short(&self.id),
            self.name,
            self.variables.len(),
            self.created_at
        );
    }
}

async fn list(
    ctx: &Context,
    limit: u32,
    offset: u32,
    tags: Option<String>,
    search: Option<String>,
) -> Result<()> {
    let client = ctx.create_client()?;

    let mut params = ListPromptsParams::new()
        .with_limit(limit)
        .with_offset(offset);

    if let Some(t) = tags {
        params = params.with_tags(t.split(',').map(|s| s.trim().to_string()).collect());
    }
    if let Some(s) = search {
        params = params.with_search(s);
    }

    let spinner = ctx.output.spinner("Fetching prompts...");
    let response = client.prompts().list(Some(params)).await?;

    if let Some(s) = spinner {
        s.finish_and_clear();
    }

    let prompts: Vec<PromptDisplay> = response.data.into_iter().map(Into::into).collect();
    ctx.output.write_list(
        &prompts,
        &["ID", "Name", "Variables", "Versions", "Tags", "Created"],
    )?;

    if response.pagination.has_more {
        ctx.output.info(&format!(
            "Showing {} of {} total prompts. Use --offset to paginate.",
            prompts.len(),
            response.pagination.total
        ));
    }

    Ok(())
}

async fn get(ctx: &Context, id: Uuid) -> Result<()> {
    let client = ctx.create_client()?;

    let spinner = ctx.output.spinner("Fetching prompt...");
    let prompt = client.prompts().get(id).await?;

    if let Some(s) = spinner {
        s.finish_and_clear();
    }

    let display: PromptDisplay = prompt.into();
    ctx.output.write(&display)?;

    Ok(())
}

async fn create(
    ctx: &Context,
    name: &str,
    template: &str,
    description: Option<String>,
    system: Option<String>,
    tags: Option<String>,
) -> Result<()> {
    let client = ctx.create_client()?;

    let mut request = CreatePromptRequest::new(name, template);

    if let Some(d) = description {
        request = request.with_description(d);
    }
    if let Some(s) = system {
        request = request.with_system_prompt(s);
    }
    if let Some(t) = tags {
        request = request.with_tags(t.split(',').map(|s| s.trim().to_string()).collect());
    }

    let spinner = ctx.output.spinner("Creating prompt...");
    let prompt = client.prompts().create(request).await?;

    if let Some(s) = spinner {
        s.finish_and_clear();
    }

    ctx.output.success(&format!("Created prompt: {}", prompt.id));

    let display: PromptDisplay = prompt.into();
    ctx.output.write(&display)?;

    Ok(())
}

async fn update(
    ctx: &Context,
    id: Uuid,
    name: Option<String>,
    description: Option<String>,
    tags: Option<String>,
) -> Result<()> {
    let client = ctx.create_client()?;

    let mut request = llm_research_sdk::UpdatePromptRequest::new();

    if let Some(n) = name {
        request = request.with_name(n);
    }
    if let Some(d) = description {
        request = request.with_description(d);
    }
    if let Some(t) = tags {
        request = request.with_tags(t.split(',').map(|s| s.trim().to_string()).collect());
    }

    let spinner = ctx.output.spinner("Updating prompt...");
    let prompt = client.prompts().update(id, request).await?;

    if let Some(s) = spinner {
        s.finish_and_clear();
    }

    ctx.output.success("Prompt updated");

    let display: PromptDisplay = prompt.into();
    ctx.output.write(&display)?;

    Ok(())
}

async fn delete(ctx: &Context, id: Uuid, force: bool) -> Result<()> {
    if !force {
        let confirm = dialoguer::Confirm::new()
            .with_prompt(format!("Delete prompt template {}?", id))
            .default(false)
            .interact()
            .context("Failed to get confirmation")?;

        if !confirm {
            ctx.output.info("Cancelled");
            return Ok(());
        }
    }

    let client = ctx.create_client()?;

    let spinner = ctx.output.spinner("Deleting prompt...");
    client.prompts().delete(id).await?;

    if let Some(s) = spinner {
        s.finish_and_clear();
    }

    ctx.output.success(&format!("Deleted prompt: {}", id));

    Ok(())
}

async fn list_versions(ctx: &Context, id: Uuid, limit: u32) -> Result<()> {
    let client = ctx.create_client()?;

    let pagination = llm_research_sdk::PaginationParams::new().with_limit(limit);

    let spinner = ctx.output.spinner("Fetching versions...");
    let response = client.prompts().list_versions(id, Some(pagination)).await?;

    if let Some(s) = spinner {
        s.finish_and_clear();
    }

    if response.data.is_empty() {
        ctx.output.info("No versions found for this prompt");
        return Ok(());
    }

    print_section("Prompt Versions");

    for version in &response.data {
        println!();
        print_field("ID", &version.id.to_string());
        print_field("Version", &version.version_number.to_string());
        print_optional_field("Changelog", version.changelog.as_deref());
        print_field("Created", &format_relative_time(&version.created_at));
    }

    Ok(())
}

async fn create_version(
    ctx: &Context,
    id: Uuid,
    template: &str,
    system: Option<String>,
    changelog: Option<String>,
) -> Result<()> {
    let client = ctx.create_client()?;

    let mut request = llm_research_sdk::CreatePromptVersionRequest::new(template);

    if let Some(s) = system {
        request = request.with_system_prompt(s);
    }
    if let Some(c) = changelog {
        request = request.with_changelog(c);
    }

    let spinner = ctx.output.spinner("Creating version...");
    let version = client.prompts().create_version(id, request).await?;

    if let Some(s) = spinner {
        s.finish_and_clear();
    }

    ctx.output.success(&format!("Created version: {}", version.version_number));
    print_field("Version ID", &version.id.to_string());

    Ok(())
}

async fn render(ctx: &Context, id: Uuid, vars: &str, version: Option<Uuid>) -> Result<()> {
    let client = ctx.create_client()?;

    let variables: HashMap<String, serde_json::Value> =
        serde_json::from_str(vars).context("Invalid JSON for variables")?;

    let mut request = llm_research_sdk::RenderPromptRequest::new(variables);

    if let Some(v) = version {
        request = request.with_version(v);
    }

    let spinner = ctx.output.spinner("Rendering prompt...");
    let response = client.prompts().render(id, request).await?;

    if let Some(s) = spinner {
        s.finish_and_clear();
    }

    print_section("Rendered Prompt");

    if let Some(ref system) = response.rendered_system_prompt {
        println!("System:");
        for line in system.lines() {
            println!("  {}", line);
        }
        println!();
    }

    println!("Template:");
    for line in response.rendered_template.lines() {
        println!("  {}", line);
    }

    if let Some(tokens) = response.token_count {
        println!();
        print_field("Estimated tokens", &tokens.to_string());
    }

    Ok(())
}

async fn validate(ctx: &Context, template: &str) -> Result<()> {
    let client = ctx.create_client()?;

    let request = llm_research_sdk::ValidatePromptRequest::new(template);

    let spinner = ctx.output.spinner("Validating prompt...");
    let response = client.prompts().validate(request).await?;

    if let Some(s) = spinner {
        s.finish_and_clear();
    }

    if response.valid {
        ctx.output.success("Template is valid");
    } else {
        ctx.output.error("Template has errors");
    }

    if !response.errors.is_empty() {
        print_section("Errors");
        for error in &response.errors {
            let location = match (error.line, error.column) {
                (Some(line), Some(col)) => format!(" (line {}, column {})", line, col),
                (Some(line), None) => format!(" (line {})", line),
                _ => String::new(),
            };
            println!("  ✗ {}{}", error.message, location);
        }
    }

    if !response.warnings.is_empty() {
        print_section("Warnings");
        for warning in &response.warnings {
            let location = match (warning.line, warning.column) {
                (Some(line), Some(col)) => format!(" (line {}, column {})", line, col),
                (Some(line), None) => format!(" (line {})", line),
                _ => String::new(),
            };
            println!("  ⚠ {}{}", warning.message, location);
        }
    }

    if !response.detected_variables.is_empty() {
        print_section("Detected Variables");
        for var in &response.detected_variables {
            println!("  - {}", var);
        }
    }

    Ok(())
}
