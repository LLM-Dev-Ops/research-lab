//! Experiments commands

use anyhow::{Context as _, Result};
use clap::{Args, Subcommand};
use comfy_table::Cell;
use llm_research_sdk::{
    CreateExperimentRequest, ExperimentConfig, ExperimentStatus, ListExperimentsParams,
};
use serde::Serialize;
use uuid::Uuid;

use crate::context::Context;
use crate::output::{
    format_relative_time, format_uuid_short, print_field, print_list_field,
    print_optional_field, print_section, status_badge, TableDisplay,
};

/// Experiment management commands
#[derive(Debug, Args)]
pub struct ExperimentsCommands {
    #[command(subcommand)]
    pub command: ExperimentsSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum ExperimentsSubcommand {
    /// List experiments
    List {
        /// Maximum number of experiments to return
        #[arg(short, long, default_value = "20")]
        limit: u32,

        /// Offset for pagination
        #[arg(short, long, default_value = "0")]
        offset: u32,

        /// Filter by status
        #[arg(short, long)]
        status: Option<String>,

        /// Filter by owner ID
        #[arg(long)]
        owner: Option<Uuid>,

        /// Filter by tags (comma-separated)
        #[arg(short, long)]
        tags: Option<String>,
    },

    /// Get experiment details
    Get {
        /// Experiment ID
        id: Uuid,
    },

    /// Create a new experiment
    Create {
        /// Experiment name
        #[arg(short, long)]
        name: String,

        /// Description
        #[arg(short, long)]
        description: Option<String>,

        /// Hypothesis
        #[arg(long)]
        hypothesis: Option<String>,

        /// Owner ID
        #[arg(long)]
        owner: Uuid,

        /// Tags (comma-separated)
        #[arg(short, long)]
        tags: Option<String>,

        /// Model IDs to use (comma-separated)
        #[arg(long)]
        models: Option<String>,

        /// Dataset IDs to use (comma-separated)
        #[arg(long)]
        datasets: Option<String>,

        /// Evaluation metrics (comma-separated)
        #[arg(long)]
        metrics: Option<String>,
    },

    /// Update an experiment
    Update {
        /// Experiment ID
        id: Uuid,

        /// New name
        #[arg(short, long)]
        name: Option<String>,

        /// New description
        #[arg(short, long)]
        description: Option<String>,

        /// New hypothesis
        #[arg(long)]
        hypothesis: Option<String>,

        /// New tags (comma-separated)
        #[arg(short, long)]
        tags: Option<String>,
    },

    /// Delete an experiment
    Delete {
        /// Experiment ID
        id: Uuid,

        /// Force deletion without confirmation
        #[arg(short, long)]
        force: bool,
    },

    /// Start an experiment
    Start {
        /// Experiment ID
        id: Uuid,
    },

    /// List runs for an experiment
    Runs {
        /// Experiment ID
        id: Uuid,

        /// Maximum number of runs to return
        #[arg(short, long, default_value = "20")]
        limit: u32,
    },

    /// Create a new run
    Run {
        /// Experiment ID
        id: Uuid,

        /// Configuration overrides as JSON
        #[arg(long)]
        overrides: Option<String>,
    },

    /// Get experiment metrics
    Metrics {
        /// Experiment ID
        id: Uuid,
    },
}

/// Execute experiment commands
pub async fn execute(ctx: &Context, cmd: ExperimentsCommands) -> Result<()> {
    match cmd.command {
        ExperimentsSubcommand::List {
            limit,
            offset,
            status,
            owner,
            tags,
        } => list(ctx, limit, offset, status, owner, tags).await,
        ExperimentsSubcommand::Get { id } => get(ctx, id).await,
        ExperimentsSubcommand::Create {
            name,
            description,
            hypothesis,
            owner,
            tags,
            models,
            datasets,
            metrics,
        } => {
            create(
                ctx,
                &name,
                description.as_deref(),
                hypothesis.as_deref(),
                owner,
                tags.as_deref(),
                models.as_deref(),
                datasets.as_deref(),
                metrics.as_deref(),
            )
            .await
        }
        ExperimentsSubcommand::Update {
            id,
            name,
            description,
            hypothesis,
            tags,
        } => update(ctx, id, name, description, hypothesis, tags).await,
        ExperimentsSubcommand::Delete { id, force } => delete(ctx, id, force).await,
        ExperimentsSubcommand::Start { id } => start(ctx, id).await,
        ExperimentsSubcommand::Runs { id, limit } => list_runs(ctx, id, limit).await,
        ExperimentsSubcommand::Run { id, overrides } => create_run(ctx, id, overrides).await,
        ExperimentsSubcommand::Metrics { id } => get_metrics(ctx, id).await,
    }
}

/// Displayable experiment for output
#[derive(Debug, Serialize)]
struct ExperimentDisplay {
    id: Uuid,
    name: String,
    status: String,
    owner_id: Uuid,
    tags: Vec<String>,
    created_at: String,
    updated_at: String,
    description: Option<String>,
    hypothesis: Option<String>,
}

impl From<llm_research_sdk::Experiment> for ExperimentDisplay {
    fn from(e: llm_research_sdk::Experiment) -> Self {
        Self {
            id: e.id,
            name: e.name,
            status: e.status.to_string(),
            owner_id: e.owner_id,
            tags: e.tags,
            created_at: format_relative_time(&e.created_at),
            updated_at: format_relative_time(&e.updated_at),
            description: e.description,
            hypothesis: e.hypothesis,
        }
    }
}

impl TableDisplay for ExperimentDisplay {
    fn to_row(&self) -> Vec<Cell> {
        vec![
            Cell::new(format_uuid_short(&self.id)),
            Cell::new(&self.name),
            Cell::new(status_badge(&self.status)),
            Cell::new(self.tags.join(", ")),
            Cell::new(&self.created_at),
        ]
    }

    fn display_single(&self) {
        print_section("Experiment");
        print_field("ID", &self.id.to_string());
        print_field("Name", &self.name);
        print_field("Status", &status_badge(&self.status));
        print_optional_field("Description", self.description.as_deref());
        print_optional_field("Hypothesis", self.hypothesis.as_deref());
        print_field("Owner", &self.owner_id.to_string());
        print_list_field("Tags", &self.tags);
        print_field("Created", &self.created_at);
        print_field("Updated", &self.updated_at);
    }

    fn display_compact(&self) {
        println!(
            "{}\t{}\t{}\t{}",
            format_uuid_short(&self.id),
            self.name,
            self.status,
            self.created_at
        );
    }
}

async fn list(
    ctx: &Context,
    limit: u32,
    offset: u32,
    status: Option<String>,
    owner: Option<Uuid>,
    tags: Option<String>,
) -> Result<()> {
    let client = ctx.create_client()?;

    let mut params = ListExperimentsParams::new()
        .with_limit(limit)
        .with_offset(offset);

    if let Some(s) = status {
        params = params.with_status(s);
    }
    if let Some(o) = owner {
        params = params.with_owner(o);
    }
    if let Some(t) = tags {
        params = params.with_tags(t.split(',').map(|s| s.trim().to_string()).collect());
    }

    let spinner = ctx.output.spinner("Fetching experiments...");
    let response = client.experiments().list(Some(params)).await?;

    if let Some(s) = spinner {
        s.finish_and_clear();
    }

    let experiments: Vec<ExperimentDisplay> = response.data.into_iter().map(Into::into).collect();
    ctx.output.write_list(
        &experiments,
        &["ID", "Name", "Status", "Tags", "Created"],
    )?;

    if response.pagination.has_more {
        ctx.output.info(&format!(
            "Showing {} of {} total experiments. Use --offset to paginate.",
            experiments.len(),
            response.pagination.total
        ));
    }

    Ok(())
}

async fn get(ctx: &Context, id: Uuid) -> Result<()> {
    let client = ctx.create_client()?;

    let spinner = ctx.output.spinner("Fetching experiment...");
    let experiment = client.experiments().get(id).await?;

    if let Some(s) = spinner {
        s.finish_and_clear();
    }

    let display: ExperimentDisplay = experiment.into();
    ctx.output.write(&display)?;

    Ok(())
}

async fn create(
    ctx: &Context,
    name: &str,
    description: Option<&str>,
    hypothesis: Option<&str>,
    owner: Uuid,
    tags: Option<&str>,
    models: Option<&str>,
    datasets: Option<&str>,
    metrics: Option<&str>,
) -> Result<()> {
    let client = ctx.create_client()?;

    // Build experiment config
    let mut config = ExperimentConfig::new();

    if let Some(m) = models {
        for model_str in m.split(',') {
            let model_id = Uuid::parse_str(model_str.trim())
                .context(format!("Invalid model ID: {}", model_str))?;
            config = config.with_model(model_id);
        }
    }

    if let Some(d) = datasets {
        for dataset_str in d.split(',') {
            let dataset_id = Uuid::parse_str(dataset_str.trim())
                .context(format!("Invalid dataset ID: {}", dataset_str))?;
            config = config.with_dataset(dataset_id);
        }
    }

    if let Some(met) = metrics {
        for metric in met.split(',') {
            config = config.with_metric(metric.trim());
        }
    }

    // Build request
    let mut request = CreateExperimentRequest::new(name, owner, config);

    if let Some(d) = description {
        request = request.with_description(d);
    }

    if let Some(h) = hypothesis {
        request = request.with_hypothesis(h);
    }

    if let Some(t) = tags {
        request = request.with_tags(t.split(',').map(|s| s.trim().to_string()).collect());
    }

    let spinner = ctx.output.spinner("Creating experiment...");
    let experiment = client.experiments().create(request).await?;

    if let Some(s) = spinner {
        s.finish_and_clear();
    }

    ctx.output.success(&format!("Created experiment: {}", experiment.id));

    let display: ExperimentDisplay = experiment.into();
    ctx.output.write(&display)?;

    Ok(())
}

async fn update(
    ctx: &Context,
    id: Uuid,
    name: Option<String>,
    description: Option<String>,
    hypothesis: Option<String>,
    tags: Option<String>,
) -> Result<()> {
    let client = ctx.create_client()?;

    let mut request = llm_research_sdk::UpdateExperimentRequest::new();

    if let Some(n) = name {
        request = request.with_name(n);
    }
    if let Some(d) = description {
        request = request.with_description(d);
    }
    if let Some(h) = hypothesis {
        request = request.with_hypothesis(h);
    }
    if let Some(t) = tags {
        request = request.with_tags(t.split(',').map(|s| s.trim().to_string()).collect());
    }

    let spinner = ctx.output.spinner("Updating experiment...");
    let experiment = client.experiments().update(id, request).await?;

    if let Some(s) = spinner {
        s.finish_and_clear();
    }

    ctx.output.success("Experiment updated");

    let display: ExperimentDisplay = experiment.into();
    ctx.output.write(&display)?;

    Ok(())
}

async fn delete(ctx: &Context, id: Uuid, force: bool) -> Result<()> {
    if !force {
        let confirm = dialoguer::Confirm::new()
            .with_prompt(format!("Delete experiment {}?", id))
            .default(false)
            .interact()
            .context("Failed to get confirmation")?;

        if !confirm {
            ctx.output.info("Cancelled");
            return Ok(());
        }
    }

    let client = ctx.create_client()?;

    let spinner = ctx.output.spinner("Deleting experiment...");
    client.experiments().delete(id).await?;

    if let Some(s) = spinner {
        s.finish_and_clear();
    }

    ctx.output.success(&format!("Deleted experiment: {}", id));

    Ok(())
}

async fn start(ctx: &Context, id: Uuid) -> Result<()> {
    let client = ctx.create_client()?;

    let spinner = ctx.output.spinner("Starting experiment...");
    let experiment = client.experiments().start(id).await?;

    if let Some(s) = spinner {
        s.finish_and_clear();
    }

    ctx.output.success(&format!(
        "Experiment started. Status: {}",
        status_badge(&experiment.status.to_string())
    ));

    Ok(())
}

async fn list_runs(ctx: &Context, id: Uuid, limit: u32) -> Result<()> {
    let client = ctx.create_client()?;

    let pagination = llm_research_sdk::PaginationParams::new().with_limit(limit);

    let spinner = ctx.output.spinner("Fetching runs...");
    let response = client.experiments().list_runs(id, Some(pagination)).await?;

    if let Some(s) = spinner {
        s.finish_and_clear();
    }

    if response.data.is_empty() {
        ctx.output.info("No runs found for this experiment");
        return Ok(());
    }

    // Display runs
    for run in &response.data {
        println!(
            "{}\t{}\t{}\t{}",
            format_uuid_short(&run.id),
            status_badge(&run.status),
            format_relative_time(&run.started_at),
            run.completed_at
                .map(|dt| format_relative_time(&dt))
                .unwrap_or_else(|| "-".to_string())
        );
    }

    Ok(())
}

async fn create_run(ctx: &Context, id: Uuid, overrides: Option<String>) -> Result<()> {
    let client = ctx.create_client()?;

    let mut request = llm_research_sdk::CreateRunRequest::new();

    if let Some(o) = overrides {
        let overrides_json: serde_json::Value =
            serde_json::from_str(&o).context("Invalid JSON for overrides")?;
        request = request.with_overrides(overrides_json);
    }

    let spinner = ctx.output.spinner("Creating run...");
    let run = client.experiments().create_run(id, request).await?;

    if let Some(s) = spinner {
        s.finish_and_clear();
    }

    ctx.output.success(&format!("Created run: {}", run.id));
    println!("Status: {}", status_badge(&run.status));

    Ok(())
}

async fn get_metrics(ctx: &Context, id: Uuid) -> Result<()> {
    let client = ctx.create_client()?;

    let spinner = ctx.output.spinner("Fetching metrics...");
    let metrics = client.experiments().get_metrics(id).await?;

    if let Some(s) = spinner {
        s.finish_and_clear();
    }

    print_section("Experiment Metrics");
    print_field("Experiment ID", &metrics.experiment_id.to_string());

    if metrics.aggregated_metrics.is_empty() {
        ctx.output.info("No metrics recorded yet");
    } else {
        print_section("Aggregated Metrics");
        for (name, summary) in &metrics.aggregated_metrics {
            println!("  {}:", name);
            println!("    Mean: {:.4}", summary.mean);
            println!("    Std:  {:.4}", summary.std);
            println!("    Min:  {:.4}", summary.min);
            println!("    Max:  {:.4}", summary.max);
            println!("    N:    {}", summary.count);
        }
    }

    if !metrics.runs.is_empty() {
        print_section("Per-Run Metrics");
        for run in &metrics.runs {
            println!("  Run {}:", format_uuid_short(&run.run_id));
            for (name, value) in &run.metrics {
                println!("    {}: {:.4}", name, value);
            }
        }
    }

    Ok(())
}
