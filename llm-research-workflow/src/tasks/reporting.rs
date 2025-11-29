use async_trait::async_trait;
use llm_research_core::Result;
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::{Task, TaskContext, TaskResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReportFormat {
    Json,
    Html,
    Markdown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportingConfig {
    pub formats: Vec<ReportFormat>,
    pub include_charts: bool,
    pub storage_path: Option<String>,
}

impl Default for ReportingConfig {
    fn default() -> Self {
        Self {
            formats: vec![ReportFormat::Json, ReportFormat::Html],
            include_charts: true,
            storage_path: None,
        }
    }
}

pub struct ReportingTask {
    config: ReportingConfig,
}

impl ReportingTask {
    pub fn new(config: ReportingConfig) -> Self {
        Self { config }
    }

    /// Generate JSON report
    fn generate_json_report(&self, experiment_id: &uuid::Uuid, data: &serde_json::Value) -> String {
        let report = json!({
            "experiment_id": experiment_id,
            "generated_at": chrono::Utc::now(),
            "format": "json",
            "data": data,
        });

        serde_json::to_string_pretty(&report).unwrap_or_default()
    }

    /// Generate HTML report
    fn generate_html_report(&self, experiment_id: &uuid::Uuid, data: &serde_json::Value) -> String {
        let mut html = String::from("<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n");
        html.push_str("  <meta charset=\"UTF-8\">\n");
        html.push_str("  <meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">\n");
        html.push_str(&format!("  <title>Experiment Report - {}</title>\n", experiment_id));
        html.push_str("  <style>\n");
        html.push_str("    body { font-family: Arial, sans-serif; margin: 40px; background: #f5f5f5; }\n");
        html.push_str("    .container { max-width: 1200px; margin: 0 auto; background: white; padding: 30px; border-radius: 8px; box-shadow: 0 2px 4px rgba(0,0,0,0.1); }\n");
        html.push_str("    h1 { color: #333; border-bottom: 2px solid #007bff; padding-bottom: 10px; }\n");
        html.push_str("    h2 { color: #555; margin-top: 30px; }\n");
        html.push_str("    .metric { background: #f8f9fa; padding: 15px; margin: 10px 0; border-left: 4px solid #007bff; border-radius: 4px; }\n");
        html.push_str("    .metric-name { font-weight: bold; color: #007bff; }\n");
        html.push_str("    .metric-value { font-size: 1.2em; margin: 5px 0; }\n");
        html.push_str("    table { width: 100%; border-collapse: collapse; margin: 20px 0; }\n");
        html.push_str("    th, td { padding: 12px; text-align: left; border-bottom: 1px solid #ddd; }\n");
        html.push_str("    th { background-color: #007bff; color: white; }\n");
        html.push_str("    tr:hover { background-color: #f5f5f5; }\n");
        html.push_str("    .timestamp { color: #666; font-size: 0.9em; }\n");
        html.push_str("  </style>\n");
        html.push_str("</head>\n<body>\n");
        html.push_str("  <div class=\"container\">\n");
        html.push_str(&format!("    <h1>Experiment Report</h1>\n"));
        html.push_str(&format!("    <p class=\"timestamp\">Experiment ID: {}</p>\n", experiment_id));
        html.push_str(&format!("    <p class=\"timestamp\">Generated: {}</p>\n", chrono::Utc::now().to_rfc3339()));

        // Extract and display metrics
        if let Some(metrics) = data.get("metrics") {
            html.push_str("    <h2>Metrics Summary</h2>\n");

            if let Some(obj) = metrics.as_object() {
                for (metric_name, metric_data) in obj {
                    html.push_str("    <div class=\"metric\">\n");
                    html.push_str(&format!("      <div class=\"metric-name\">{}</div>\n", metric_name));

                    if let Some(metric_obj) = metric_data.as_object() {
                        for (key, value) in metric_obj {
                            html.push_str(&format!("      <div class=\"metric-value\">{}: {}</div>\n", key, value));
                        }
                    }

                    html.push_str("    </div>\n");
                }
            }
        }

        // Display raw data in a table
        html.push_str("    <h2>Raw Data</h2>\n");
        html.push_str("    <pre style=\"background: #f8f9fa; padding: 15px; border-radius: 4px; overflow-x: auto;\">");
        html.push_str(&serde_json::to_string_pretty(data).unwrap_or_default());
        html.push_str("</pre>\n");

        html.push_str("  </div>\n");
        html.push_str("</body>\n</html>");

        html
    }

    /// Generate Markdown report
    fn generate_markdown_report(&self, experiment_id: &uuid::Uuid, data: &serde_json::Value) -> String {
        let mut md = String::new();

        md.push_str(&format!("# Experiment Report\n\n"));
        md.push_str(&format!("**Experiment ID:** {}\n\n", experiment_id));
        md.push_str(&format!("**Generated:** {}\n\n", chrono::Utc::now().to_rfc3339()));

        // Extract and display metrics
        if let Some(metrics) = data.get("metrics") {
            md.push_str("## Metrics Summary\n\n");

            if let Some(obj) = metrics.as_object() {
                for (metric_name, metric_data) in obj {
                    md.push_str(&format!("### {}\n\n", metric_name));

                    if let Some(metric_obj) = metric_data.as_object() {
                        for (key, value) in metric_obj {
                            md.push_str(&format!("- **{}**: {}\n", key, value));
                        }
                        md.push_str("\n");
                    }
                }
            }
        }

        // Display raw data
        md.push_str("## Raw Data\n\n");
        md.push_str("```json\n");
        md.push_str(&serde_json::to_string_pretty(data).unwrap_or_default());
        md.push_str("\n```\n");

        md
    }

    /// Save report to storage
    async fn save_report(&self, format: &ReportFormat, content: &str, experiment_id: &uuid::Uuid) -> Result<String> {
        let extension = match format {
            ReportFormat::Json => "json",
            ReportFormat::Html => "html",
            ReportFormat::Markdown => "md",
        };

        let storage_path = self.config.storage_path
            .clone()
            .unwrap_or_else(|| "s3://reports".to_string());

        let file_path = format!("{}/experiment-{}.{}", storage_path, experiment_id, extension);

        // In real implementation, would save to S3, filesystem, etc.
        tracing::info!("Saving {} report to {}", format!("{:?}", format), file_path);
        tracing::debug!("Report content length: {} bytes", content.len());

        Ok(file_path)
    }
}

#[async_trait]
impl Task for ReportingTask {
    async fn execute(&self, context: TaskContext) -> Result<TaskResult> {
        tracing::info!(
            "Generating report for experiment: {}",
            context.experiment_id
        );

        // Mock data - in real system, this would come from previous tasks
        let mock_data = json!({
            "metrics": {
                "accuracy": {
                    "mean": 0.87,
                    "median": 0.89,
                    "std_dev": 0.12,
                    "min": 0.45,
                    "max": 1.0
                },
                "bleu": {
                    "mean": 0.65,
                    "median": 0.67,
                    "std_dev": 0.15
                },
                "rouge_l": {
                    "mean": 0.72,
                    "median": 0.74,
                    "std_dev": 0.11
                }
            },
            "total_samples": 1000,
            "model": "gpt-4",
            "provider": "OpenAI"
        });

        let mut generated_reports = Vec::new();

        for format in &self.config.formats {
            let content = match format {
                ReportFormat::Json => self.generate_json_report(&context.experiment_id, &mock_data),
                ReportFormat::Html => self.generate_html_report(&context.experiment_id, &mock_data),
                ReportFormat::Markdown => self.generate_markdown_report(&context.experiment_id, &mock_data),
            };

            let storage_location = self.save_report(format, &content, &context.experiment_id).await?;

            generated_reports.push(json!({
                "format": format!("{:?}", format),
                "location": storage_location,
                "size_bytes": content.len(),
            }));
        }

        let output = json!({
            "report_generated": true,
            "formats": self.config.formats.iter().map(|f| format!("{:?}", f)).collect::<Vec<_>>(),
            "reports": generated_reports,
            "include_charts": self.config.include_charts,
        });

        Ok(TaskResult::success(output))
    }

    fn name(&self) -> &str {
        "reporting"
    }
}
