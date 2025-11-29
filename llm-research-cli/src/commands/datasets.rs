//! Datasets commands

use anyhow::{Context as _, Result};
use clap::{Args, Subcommand};
use comfy_table::Cell;
use llm_research_sdk::{CreateDatasetRequest, DatasetFormat, ListDatasetsParams};
use serde::Serialize;
use uuid::Uuid;

use crate::context::Context;
use crate::output::{
    format_bytes, format_relative_time, format_uuid_short, print_field, print_list_field,
    print_optional_field, print_section, TableDisplay,
};

/// Dataset management commands
#[derive(Debug, Args)]
pub struct DatasetsCommands {
    #[command(subcommand)]
    pub command: DatasetsSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum DatasetsSubcommand {
    /// List datasets
    List {
        /// Maximum number of datasets to return
        #[arg(short, long, default_value = "20")]
        limit: u32,

        /// Offset for pagination
        #[arg(short, long, default_value = "0")]
        offset: u32,

        /// Filter by format
        #[arg(short, long)]
        format: Option<String>,

        /// Filter by tags (comma-separated)
        #[arg(short, long)]
        tags: Option<String>,
    },

    /// Get dataset details
    Get {
        /// Dataset ID
        id: Uuid,
    },

    /// Create a new dataset
    Create {
        /// Dataset name
        #[arg(short, long)]
        name: String,

        /// Format (json, jsonl, csv, parquet, text)
        #[arg(short, long)]
        format: String,

        /// Description
        #[arg(short, long)]
        description: Option<String>,

        /// Schema as JSON
        #[arg(short, long)]
        schema: Option<String>,

        /// Tags (comma-separated)
        #[arg(short, long)]
        tags: Option<String>,
    },

    /// Update a dataset
    Update {
        /// Dataset ID
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

    /// Delete a dataset
    Delete {
        /// Dataset ID
        id: Uuid,

        /// Force deletion without confirmation
        #[arg(short, long)]
        force: bool,
    },

    /// List dataset versions
    Versions {
        /// Dataset ID
        id: Uuid,

        /// Maximum number of versions to return
        #[arg(short, long, default_value = "10")]
        limit: u32,
    },

    /// Create a new dataset version
    CreateVersion {
        /// Dataset ID
        id: Uuid,

        /// Version string
        #[arg(short, long)]
        version: String,

        /// Description
        #[arg(short, long)]
        description: Option<String>,

        /// Changelog
        #[arg(short, long)]
        changelog: Option<String>,
    },

    /// Get an upload URL for a dataset
    Upload {
        /// Dataset ID
        id: Uuid,

        /// Filename
        #[arg(short, long)]
        filename: String,

        /// Content type
        #[arg(short, long, default_value = "application/octet-stream")]
        content_type: String,
    },

    /// Get a download URL for a dataset
    Download {
        /// Dataset ID
        id: Uuid,
    },
}

/// Execute dataset commands
pub async fn execute(ctx: &Context, cmd: DatasetsCommands) -> Result<()> {
    match cmd.command {
        DatasetsSubcommand::List {
            limit,
            offset,
            format,
            tags,
        } => list(ctx, limit, offset, format, tags).await,
        DatasetsSubcommand::Get { id } => get(ctx, id).await,
        DatasetsSubcommand::Create {
            name,
            format,
            description,
            schema,
            tags,
        } => create(ctx, &name, &format, description, schema, tags).await,
        DatasetsSubcommand::Update {
            id,
            name,
            description,
            tags,
        } => update(ctx, id, name, description, tags).await,
        DatasetsSubcommand::Delete { id, force } => delete(ctx, id, force).await,
        DatasetsSubcommand::Versions { id, limit } => list_versions(ctx, id, limit).await,
        DatasetsSubcommand::CreateVersion {
            id,
            version,
            description,
            changelog,
        } => create_version(ctx, id, &version, description, changelog).await,
        DatasetsSubcommand::Upload {
            id,
            filename,
            content_type,
        } => get_upload_url(ctx, id, &filename, &content_type).await,
        DatasetsSubcommand::Download { id } => get_download_url(ctx, id).await,
    }
}

/// Displayable dataset for output
#[derive(Debug, Serialize)]
struct DatasetDisplay {
    id: Uuid,
    name: String,
    format: String,
    description: Option<String>,
    tags: Vec<String>,
    size_bytes: Option<String>,
    row_count: Option<u64>,
    created_at: String,
    updated_at: String,
}

impl From<llm_research_sdk::Dataset> for DatasetDisplay {
    fn from(d: llm_research_sdk::Dataset) -> Self {
        Self {
            id: d.id,
            name: d.name,
            format: d.format.to_string(),
            description: d.description,
            tags: d.tags,
            size_bytes: d.size_bytes.map(format_bytes),
            row_count: d.row_count,
            created_at: format_relative_time(&d.created_at),
            updated_at: format_relative_time(&d.updated_at),
        }
    }
}

impl TableDisplay for DatasetDisplay {
    fn to_row(&self) -> Vec<Cell> {
        vec![
            Cell::new(format_uuid_short(&self.id)),
            Cell::new(&self.name),
            Cell::new(&self.format),
            Cell::new(self.size_bytes.as_deref().unwrap_or("-")),
            Cell::new(
                self.row_count
                    .map(|r| r.to_string())
                    .unwrap_or_else(|| "-".to_string()),
            ),
            Cell::new(&self.created_at),
        ]
    }

    fn display_single(&self) {
        print_section("Dataset");
        print_field("ID", &self.id.to_string());
        print_field("Name", &self.name);
        print_field("Format", &self.format);
        print_optional_field("Description", self.description.as_deref());
        print_list_field("Tags", &self.tags);
        print_optional_field("Size", self.size_bytes.as_deref());
        if let Some(count) = self.row_count {
            print_field("Rows", &count.to_string());
        }
        print_field("Created", &self.created_at);
        print_field("Updated", &self.updated_at);
    }

    fn display_compact(&self) {
        println!(
            "{}\t{}\t{}\t{}\t{}",
            format_uuid_short(&self.id),
            self.name,
            self.format,
            self.size_bytes.as_deref().unwrap_or("-"),
            self.created_at
        );
    }
}

async fn list(
    ctx: &Context,
    limit: u32,
    offset: u32,
    format: Option<String>,
    tags: Option<String>,
) -> Result<()> {
    let client = ctx.create_client()?;

    let mut params = ListDatasetsParams::new()
        .with_limit(limit)
        .with_offset(offset);

    if let Some(f) = format {
        let format_enum: DatasetFormat = f.parse().map_err(|e| anyhow::anyhow!("Invalid format: {}", e))?;
        params = params.with_format(format_enum);
    }
    if let Some(t) = tags {
        params = params.with_tags(t.split(',').map(|s| s.trim().to_string()).collect());
    }

    let spinner = ctx.output.spinner("Fetching datasets...");
    let response = client.datasets().list(Some(params)).await?;

    if let Some(s) = spinner {
        s.finish_and_clear();
    }

    let datasets: Vec<DatasetDisplay> = response.data.into_iter().map(Into::into).collect();
    ctx.output.write_list(
        &datasets,
        &["ID", "Name", "Format", "Size", "Rows", "Created"],
    )?;

    if response.pagination.has_more {
        ctx.output.info(&format!(
            "Showing {} of {} total datasets. Use --offset to paginate.",
            datasets.len(),
            response.pagination.total
        ));
    }

    Ok(())
}

async fn get(ctx: &Context, id: Uuid) -> Result<()> {
    let client = ctx.create_client()?;

    let spinner = ctx.output.spinner("Fetching dataset...");
    let dataset = client.datasets().get(id).await?;

    if let Some(s) = spinner {
        s.finish_and_clear();
    }

    let display: DatasetDisplay = dataset.into();
    ctx.output.write(&display)?;

    Ok(())
}

async fn create(
    ctx: &Context,
    name: &str,
    format: &str,
    description: Option<String>,
    schema: Option<String>,
    tags: Option<String>,
) -> Result<()> {
    let client = ctx.create_client()?;

    let format_enum: DatasetFormat = format.parse().map_err(|e| anyhow::anyhow!("Invalid format: {}", e))?;

    let mut request = CreateDatasetRequest::new(name, format_enum);

    if let Some(d) = description {
        request = request.with_description(d);
    }

    if let Some(s) = schema {
        let schema_json: serde_json::Value =
            serde_json::from_str(&s).context("Invalid JSON for schema")?;
        request = request.with_schema(schema_json);
    }

    if let Some(t) = tags {
        request = request.with_tags(t.split(',').map(|s| s.trim().to_string()).collect());
    }

    let spinner = ctx.output.spinner("Creating dataset...");
    let dataset = client.datasets().create(request).await?;

    if let Some(s) = spinner {
        s.finish_and_clear();
    }

    ctx.output.success(&format!("Created dataset: {}", dataset.id));

    let display: DatasetDisplay = dataset.into();
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

    let mut request = llm_research_sdk::UpdateDatasetRequest::new();

    if let Some(n) = name {
        request = request.with_name(n);
    }
    if let Some(d) = description {
        request = request.with_description(d);
    }
    if let Some(t) = tags {
        request = request.with_tags(t.split(',').map(|s| s.trim().to_string()).collect());
    }

    let spinner = ctx.output.spinner("Updating dataset...");
    let dataset = client.datasets().update(id, request).await?;

    if let Some(s) = spinner {
        s.finish_and_clear();
    }

    ctx.output.success("Dataset updated");

    let display: DatasetDisplay = dataset.into();
    ctx.output.write(&display)?;

    Ok(())
}

async fn delete(ctx: &Context, id: Uuid, force: bool) -> Result<()> {
    if !force {
        let confirm = dialoguer::Confirm::new()
            .with_prompt(format!("Delete dataset {}?", id))
            .default(false)
            .interact()
            .context("Failed to get confirmation")?;

        if !confirm {
            ctx.output.info("Cancelled");
            return Ok(());
        }
    }

    let client = ctx.create_client()?;

    let spinner = ctx.output.spinner("Deleting dataset...");
    client.datasets().delete(id).await?;

    if let Some(s) = spinner {
        s.finish_and_clear();
    }

    ctx.output.success(&format!("Deleted dataset: {}", id));

    Ok(())
}

async fn list_versions(ctx: &Context, id: Uuid, limit: u32) -> Result<()> {
    let client = ctx.create_client()?;

    let pagination = llm_research_sdk::PaginationParams::new().with_limit(limit);

    let spinner = ctx.output.spinner("Fetching versions...");
    let response = client.datasets().list_versions(id, Some(pagination)).await?;

    if let Some(s) = spinner {
        s.finish_and_clear();
    }

    if response.data.is_empty() {
        ctx.output.info("No versions found for this dataset");
        return Ok(());
    }

    print_section("Dataset Versions");

    for version in &response.data {
        println!();
        print_field("ID", &version.id.to_string());
        print_field("Version", &version.version);
        print_optional_field("Description", version.description.as_deref());
        print_optional_field("Changelog", version.changelog.as_deref());
        if let Some(size) = version.size_bytes {
            print_field("Size", &format_bytes(size));
        }
        if let Some(rows) = version.row_count {
            print_field("Rows", &rows.to_string());
        }
        print_field("Created", &format_relative_time(&version.created_at));
    }

    Ok(())
}

async fn create_version(
    ctx: &Context,
    id: Uuid,
    version: &str,
    description: Option<String>,
    changelog: Option<String>,
) -> Result<()> {
    let client = ctx.create_client()?;

    let mut request = llm_research_sdk::CreateDatasetVersionRequest::new(version);

    if let Some(d) = description {
        request = request.with_description(d);
    }
    if let Some(c) = changelog {
        request = request.with_changelog(c);
    }

    let spinner = ctx.output.spinner("Creating version...");
    let version = client.datasets().create_version(id, request).await?;

    if let Some(s) = spinner {
        s.finish_and_clear();
    }

    ctx.output.success(&format!("Created version: {}", version.version));
    print_field("Version ID", &version.id.to_string());

    Ok(())
}

async fn get_upload_url(
    ctx: &Context,
    id: Uuid,
    filename: &str,
    content_type: &str,
) -> Result<()> {
    let client = ctx.create_client()?;

    let request = llm_research_sdk::UploadRequest::new(filename, content_type);

    let spinner = ctx.output.spinner("Getting upload URL...");
    let response = client.datasets().get_upload_url(id, request).await?;

    if let Some(s) = spinner {
        s.finish_and_clear();
    }

    print_section("Upload Information");
    print_field("URL", &response.upload_url);
    print_field("Expires", &format_relative_time(&response.expires_at));

    ctx.output.info("Use the following curl command to upload:");
    println!(
        "  curl -X PUT -H \"Content-Type: {}\" --data-binary @{} \"{}\"",
        content_type, filename, response.upload_url
    );

    Ok(())
}

async fn get_download_url(ctx: &Context, id: Uuid) -> Result<()> {
    let client = ctx.create_client()?;

    let spinner = ctx.output.spinner("Getting download URL...");
    let response = client.datasets().get_download_url(id).await?;

    if let Some(s) = spinner {
        s.finish_and_clear();
    }

    print_section("Download Information");
    print_field("URL", &response.download_url);
    print_field("Expires", &format_relative_time(&response.expires_at));

    ctx.output.info("Use the following curl command to download:");
    println!("  curl -o dataset.bin \"{}\"", response.download_url);

    Ok(())
}
