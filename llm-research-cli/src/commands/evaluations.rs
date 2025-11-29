//! Evaluations commands

use anyhow::{Context as _, Result};
use clap::{Args, Subcommand};
use comfy_table::Cell;
use llm_research_sdk::{
    CreateEvaluationRequest, EvaluationConfig, EvaluationType, ListEvaluationsParams, MetricConfig,
};
use serde::Serialize;
use uuid::Uuid;

use crate::context::Context;
use crate::output::{
    format_relative_time, format_uuid_short, print_field, print_list_field, print_optional_field,
    print_section, status_badge, TableDisplay,
};

/// Evaluation management commands
#[derive(Debug, Args)]
pub struct EvaluationsCommands {
    #[command(subcommand)]
    pub command: EvaluationsSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum EvaluationsSubcommand {
    /// List evaluations
    List {
        /// Maximum number of evaluations to return
        #[arg(short, long, default_value = "20")]
        limit: u32,

        /// Offset for pagination
        #[arg(short, long, default_value = "0")]
        offset: u32,

        /// Filter by type (automated, human, comparison, llm_judge, custom)
        #[arg(short = 't', long)]
        eval_type: Option<String>,

        /// Filter by tags (comma-separated)
        #[arg(long)]
        tags: Option<String>,
    },

    /// Get evaluation details
    Get {
        /// Evaluation ID
        id: Uuid,
    },

    /// Create a new evaluation
    Create {
        /// Evaluation name
        #[arg(short, long)]
        name: String,

        /// Evaluation type (automated, human, comparison, llm_judge, custom)
        #[arg(short = 't', long)]
        eval_type: String,

        /// Description
        #[arg(short, long)]
        description: Option<String>,

        /// Dataset ID
        #[arg(long)]
        dataset: Option<Uuid>,

        /// Model IDs (comma-separated)
        #[arg(long)]
        models: Option<String>,

        /// Metrics (comma-separated, format: name:type)
        #[arg(short, long)]
        metrics: Option<String>,

        /// Tags (comma-separated)
        #[arg(long)]
        tags: Option<String>,
    },

    /// Update an evaluation
    Update {
        /// Evaluation ID
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

    /// Delete an evaluation
    Delete {
        /// Evaluation ID
        id: Uuid,

        /// Force deletion without confirmation
        #[arg(short, long)]
        force: bool,
    },

    /// Run an evaluation
    Run {
        /// Evaluation ID
        id: Uuid,

        /// Configuration overrides as JSON
        #[arg(long)]
        overrides: Option<String>,

        /// Run asynchronously
        #[arg(long)]
        async_run: bool,
    },

    /// List runs for an evaluation
    Runs {
        /// Evaluation ID
        id: Uuid,

        /// Maximum number of runs to return
        #[arg(short, long, default_value = "10")]
        limit: u32,
    },

    /// Get results for an evaluation run
    Results {
        /// Evaluation ID
        evaluation_id: Uuid,

        /// Run ID
        run_id: Uuid,
    },

    /// Compare multiple evaluation runs
    Compare {
        /// Run IDs to compare (comma-separated)
        #[arg(short, long)]
        runs: String,

        /// Metrics to compare (comma-separated)
        #[arg(short, long)]
        metrics: Option<String>,
    },

    /// List available metric types
    MetricTypes,
}

/// Execute evaluation commands
pub async fn execute(ctx: &Context, cmd: EvaluationsCommands) -> Result<()> {
    match cmd.command {
        EvaluationsSubcommand::List {
            limit,
            offset,
            eval_type,
            tags,
        } => list(ctx, limit, offset, eval_type, tags).await,
        EvaluationsSubcommand::Get { id } => get(ctx, id).await,
        EvaluationsSubcommand::Create {
            name,
            eval_type,
            description,
            dataset,
            models,
            metrics,
            tags,
        } => {
            create(
                ctx, &name, &eval_type, description, dataset, models, metrics, tags,
            )
            .await
        }
        EvaluationsSubcommand::Update {
            id,
            name,
            description,
            tags,
        } => update(ctx, id, name, description, tags).await,
        EvaluationsSubcommand::Delete { id, force } => delete(ctx, id, force).await,
        EvaluationsSubcommand::Run {
            id,
            overrides,
            async_run,
        } => run_evaluation(ctx, id, overrides, async_run).await,
        EvaluationsSubcommand::Runs { id, limit } => list_runs(ctx, id, limit).await,
        EvaluationsSubcommand::Results {
            evaluation_id,
            run_id,
        } => get_results(ctx, evaluation_id, run_id).await,
        EvaluationsSubcommand::Compare { runs, metrics } => compare(ctx, &runs, metrics).await,
        EvaluationsSubcommand::MetricTypes => list_metric_types(ctx).await,
    }
}

/// Displayable evaluation for output
#[derive(Debug, Serialize)]
struct EvaluationDisplay {
    id: Uuid,
    name: String,
    evaluation_type: String,
    description: Option<String>,
    tags: Vec<String>,
    run_count: u32,
    last_run_at: Option<String>,
    created_at: String,
    updated_at: String,
}

impl From<llm_research_sdk::Evaluation> for EvaluationDisplay {
    fn from(e: llm_research_sdk::Evaluation) -> Self {
        Self {
            id: e.id,
            name: e.name,
            evaluation_type: e.evaluation_type.to_string(),
            description: e.description,
            tags: e.tags,
            run_count: e.run_count,
            last_run_at: e.last_run_at.map(|dt| format_relative_time(&dt)),
            created_at: format_relative_time(&e.created_at),
            updated_at: format_relative_time(&e.updated_at),
        }
    }
}

impl TableDisplay for EvaluationDisplay {
    fn to_row(&self) -> Vec<Cell> {
        vec![
            Cell::new(format_uuid_short(&self.id)),
            Cell::new(&self.name),
            Cell::new(&self.evaluation_type),
            Cell::new(self.run_count.to_string()),
            Cell::new(self.last_run_at.as_deref().unwrap_or("-")),
            Cell::new(&self.created_at),
        ]
    }

    fn display_single(&self) {
        print_section("Evaluation");
        print_field("ID", &self.id.to_string());
        print_field("Name", &self.name);
        print_field("Type", &self.evaluation_type);
        print_optional_field("Description", self.description.as_deref());
        print_list_field("Tags", &self.tags);
        print_field("Run Count", &self.run_count.to_string());
        print_optional_field("Last Run", self.last_run_at.as_deref());
        print_field("Created", &self.created_at);
        print_field("Updated", &self.updated_at);
    }

    fn display_compact(&self) {
        println!(
            "{}\t{}\t{}\t{} runs\t{}",
            format_uuid_short(&self.id),
            self.name,
            self.evaluation_type,
            self.run_count,
            self.created_at
        );
    }
}

fn parse_evaluation_type(s: &str) -> Result<EvaluationType> {
    match s.to_lowercase().as_str() {
        "automated" => Ok(EvaluationType::Automated),
        "human" => Ok(EvaluationType::Human),
        "comparison" => Ok(EvaluationType::Comparison),
        "llm_judge" | "llmjudge" | "judge" => Ok(EvaluationType::LlmJudge),
        "custom" => Ok(EvaluationType::Custom),
        _ => anyhow::bail!(
            "Invalid evaluation type: {}. Valid types: automated, human, comparison, llm_judge, custom",
            s
        ),
    }
}

async fn list(
    ctx: &Context,
    limit: u32,
    offset: u32,
    eval_type: Option<String>,
    tags: Option<String>,
) -> Result<()> {
    let client = ctx.create_client()?;

    let mut params = ListEvaluationsParams::new()
        .with_limit(limit)
        .with_offset(offset);

    if let Some(t) = eval_type {
        let eval_type_enum = parse_evaluation_type(&t)?;
        params = params.with_type(eval_type_enum);
    }
    if let Some(t) = tags {
        params = params.with_tags(t.split(',').map(|s| s.trim().to_string()).collect());
    }

    let spinner = ctx.output.spinner("Fetching evaluations...");
    let response = client.evaluations().list(Some(params)).await?;

    if let Some(s) = spinner {
        s.finish_and_clear();
    }

    let evaluations: Vec<EvaluationDisplay> = response.data.into_iter().map(Into::into).collect();
    ctx.output.write_list(
        &evaluations,
        &["ID", "Name", "Type", "Runs", "Last Run", "Created"],
    )?;

    if response.pagination.has_more {
        ctx.output.info(&format!(
            "Showing {} of {} total evaluations. Use --offset to paginate.",
            evaluations.len(),
            response.pagination.total
        ));
    }

    Ok(())
}

async fn get(ctx: &Context, id: Uuid) -> Result<()> {
    let client = ctx.create_client()?;

    let spinner = ctx.output.spinner("Fetching evaluation...");
    let evaluation = client.evaluations().get(id).await?;

    if let Some(s) = spinner {
        s.finish_and_clear();
    }

    let display: EvaluationDisplay = evaluation.into();
    ctx.output.write(&display)?;

    Ok(())
}

async fn create(
    ctx: &Context,
    name: &str,
    eval_type: &str,
    description: Option<String>,
    dataset: Option<Uuid>,
    models: Option<String>,
    metrics: Option<String>,
    tags: Option<String>,
) -> Result<()> {
    let client = ctx.create_client()?;

    let evaluation_type = parse_evaluation_type(eval_type)?;

    // Build config
    let mut config = EvaluationConfig::new();

    if let Some(d) = dataset {
        config = config.with_dataset(d);
    }

    if let Some(m) = models {
        for model_str in m.split(',') {
            let model_id = Uuid::parse_str(model_str.trim())
                .context(format!("Invalid model ID: {}", model_str))?;
            config = config.with_model(model_id);
        }
    }

    if let Some(met) = metrics {
        for metric_str in met.split(',') {
            let parts: Vec<&str> = metric_str.split(':').collect();
            let metric_config = match parts.as_slice() {
                [name, metric_type] => MetricConfig::new(name.trim(), metric_type.trim()),
                [name] => MetricConfig::new(name.trim(), name.trim()),
                _ => anyhow::bail!("Invalid metric format: {}. Use 'name:type'", metric_str),
            };
            config = config.with_metric(metric_config);
        }
    }

    // Build request
    let mut request = CreateEvaluationRequest::new(name, evaluation_type, config);

    if let Some(d) = description {
        request = request.with_description(d);
    }

    if let Some(t) = tags {
        request = request.with_tags(t.split(',').map(|s| s.trim().to_string()).collect());
    }

    let spinner = ctx.output.spinner("Creating evaluation...");
    let evaluation = client.evaluations().create(request).await?;

    if let Some(s) = spinner {
        s.finish_and_clear();
    }

    ctx.output.success(&format!("Created evaluation: {}", evaluation.id));

    let display: EvaluationDisplay = evaluation.into();
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

    let mut request = llm_research_sdk::UpdateEvaluationRequest::new();

    if let Some(n) = name {
        request = request.with_name(n);
    }
    if let Some(d) = description {
        request = request.with_description(d);
    }
    if let Some(t) = tags {
        request = request.with_tags(t.split(',').map(|s| s.trim().to_string()).collect());
    }

    let spinner = ctx.output.spinner("Updating evaluation...");
    let evaluation = client.evaluations().update(id, request).await?;

    if let Some(s) = spinner {
        s.finish_and_clear();
    }

    ctx.output.success("Evaluation updated");

    let display: EvaluationDisplay = evaluation.into();
    ctx.output.write(&display)?;

    Ok(())
}

async fn delete(ctx: &Context, id: Uuid, force: bool) -> Result<()> {
    if !force {
        let confirm = dialoguer::Confirm::new()
            .with_prompt(format!("Delete evaluation {}?", id))
            .default(false)
            .interact()
            .context("Failed to get confirmation")?;

        if !confirm {
            ctx.output.info("Cancelled");
            return Ok(());
        }
    }

    let client = ctx.create_client()?;

    let spinner = ctx.output.spinner("Deleting evaluation...");
    client.evaluations().delete(id).await?;

    if let Some(s) = spinner {
        s.finish_and_clear();
    }

    ctx.output.success(&format!("Deleted evaluation: {}", id));

    Ok(())
}

async fn run_evaluation(
    ctx: &Context,
    id: Uuid,
    overrides: Option<String>,
    async_run: bool,
) -> Result<()> {
    let client = ctx.create_client()?;

    let mut request = llm_research_sdk::RunEvaluationRequest::new();

    if let Some(o) = overrides {
        let overrides_json: serde_json::Value =
            serde_json::from_str(&o).context("Invalid JSON for overrides")?;
        request = request.with_overrides(overrides_json);
    }

    if async_run {
        request = request.async_execution(true);
    }

    let spinner = ctx.output.spinner("Running evaluation...");
    let run = client.evaluations().run(id, request).await?;

    if let Some(s) = spinner {
        s.finish_and_clear();
    }

    ctx.output.success(&format!("Created run: {}", run.id));
    println!("Status: {}", status_badge(&run.status.to_string()));

    if async_run {
        ctx.output.info("Run started asynchronously. Check status with 'llm-research eval runs'");
    }

    Ok(())
}

async fn list_runs(ctx: &Context, id: Uuid, limit: u32) -> Result<()> {
    let client = ctx.create_client()?;

    let pagination = llm_research_sdk::PaginationParams::new().with_limit(limit);

    let spinner = ctx.output.spinner("Fetching runs...");
    let response = client.evaluations().list_runs(id, Some(pagination)).await?;

    if let Some(s) = spinner {
        s.finish_and_clear();
    }

    if response.data.is_empty() {
        ctx.output.info("No runs found for this evaluation");
        return Ok(());
    }

    print_section("Evaluation Runs");

    for run in &response.data {
        println!();
        print_field("ID", &run.id.to_string());
        print_field("Status", &status_badge(&run.status.to_string()));
        print_field("Started", &format_relative_time(&run.started_at));
        if let Some(completed) = &run.completed_at {
            print_field("Completed", &format_relative_time(completed));
        }
        if let Some(error) = &run.error {
            print_field("Error", error);
        }
    }

    Ok(())
}

async fn get_results(ctx: &Context, evaluation_id: Uuid, run_id: Uuid) -> Result<()> {
    let client = ctx.create_client()?;

    let spinner = ctx.output.spinner("Fetching results...");
    let results = client.evaluations().get_results(evaluation_id, run_id).await?;

    if let Some(s) = spinner {
        s.finish_and_clear();
    }

    print_section("Evaluation Results");
    print_field("Evaluation ID", &results.evaluation_id.to_string());
    print_field("Run ID", &results.run_id.to_string());

    print_section("Summary");
    print_field("Total Samples", &results.summary.total_samples.to_string());
    print_field("Passed", &results.summary.passed_samples.to_string());
    print_field("Failed", &results.summary.failed_samples.to_string());
    print_field("Pass Rate", &format!("{:.1}%", results.summary.pass_rate * 100.0));
    if let Some(score) = results.summary.overall_score {
        print_field("Overall Score", &format!("{:.4}", score));
    }

    if !results.metrics.is_empty() {
        print_section("Metrics");
        for (name, metric) in &results.metrics {
            println!("  {}:", name);
            println!("    Value: {:.4}", metric.value);
            if let Some(std_dev) = metric.std_dev {
                println!("    Std Dev: {:.4}", std_dev);
            }
            if let Some(min) = metric.min {
                println!("    Min: {:.4}", min);
            }
            if let Some(max) = metric.max {
                println!("    Max: {:.4}", max);
            }
            if let Some(passed) = metric.passed {
                println!("    Passed: {}", passed);
            }
        }
    }

    Ok(())
}

async fn compare(ctx: &Context, runs: &str, metrics: Option<String>) -> Result<()> {
    let client = ctx.create_client()?;

    let run_ids: Vec<Uuid> = runs
        .split(',')
        .map(|s| Uuid::parse_str(s.trim()))
        .collect::<Result<Vec<_>, _>>()
        .context("Invalid run ID format")?;

    if run_ids.len() < 2 {
        anyhow::bail!("At least 2 run IDs are required for comparison");
    }

    let mut request = llm_research_sdk::CompareEvaluationsRequest::new(run_ids);

    if let Some(m) = metrics {
        request = request.with_metrics(m.split(',').map(|s| s.trim().to_string()).collect());
    }

    let spinner = ctx.output.spinner("Comparing evaluations...");
    let result = client.evaluations().compare(request).await?;

    if let Some(s) = spinner {
        s.finish_and_clear();
    }

    print_section("Comparison Results");

    for (metric_name, comparison) in &result.metrics_comparison {
        println!("\n  {}:", metric_name);
        for (run_id, value) in &comparison.values {
            let marker = if run_id == &comparison.best_run_id.to_string() {
                " (best)"
            } else {
                ""
            };
            println!("    {}: {:.4}{}", format_uuid_short(&Uuid::parse_str(run_id)?), value, marker);
        }
        if let Some(improvement) = comparison.improvement {
            println!("    Improvement: {:.2}%", improvement * 100.0);
        }
    }

    if let Some(winner) = result.winner {
        print_section("Winner");
        print_field("Run ID", &winner.to_string());
    }

    Ok(())
}

async fn list_metric_types(ctx: &Context) -> Result<()> {
    let client = ctx.create_client()?;

    let spinner = ctx.output.spinner("Fetching metric types...");
    let types = client.evaluations().list_metric_types().await?;

    if let Some(s) = spinner {
        s.finish_and_clear();
    }

    if types.is_empty() {
        ctx.output.info("No metric types available");
        return Ok(());
    }

    print_section("Available Metric Types");

    for metric_type in &types {
        println!();
        print_field("Name", &metric_type.name);
        print_field("Display Name", &metric_type.display_name);
        print_field("Category", &metric_type.category);
        println!("  Description: {}", metric_type.description);

        if !metric_type.parameters.is_empty() {
            println!("  Parameters:");
            for param in &metric_type.parameters {
                let required = if param.required { " (required)" } else { "" };
                println!(
                    "    {} [{}]{}",
                    param.name, param.parameter_type, required
                );
                if let Some(ref desc) = param.description {
                    println!("      {}", desc);
                }
            }
        }
    }

    Ok(())
}
