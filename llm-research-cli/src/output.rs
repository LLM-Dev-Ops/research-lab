//! Output formatting for CLI

use anyhow::Result;
use clap::ValueEnum;
use colored::Colorize;
use comfy_table::{modifiers::UTF8_ROUND_CORNERS, presets::UTF8_FULL, Cell, Color, Table};
use serde::Serialize;

/// Output format for CLI commands
#[derive(Debug, Clone, Copy, Default, ValueEnum, PartialEq, Eq)]
pub enum OutputFormat {
    /// Table format (default)
    #[default]
    Table,
    /// JSON format
    Json,
    /// YAML format
    Yaml,
    /// Compact format (single line per item)
    Compact,
}

impl std::fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Table => write!(f, "table"),
            Self::Json => write!(f, "json"),
            Self::Yaml => write!(f, "yaml"),
            Self::Compact => write!(f, "compact"),
        }
    }
}

/// Output writer that handles different formats
pub struct OutputWriter {
    format: OutputFormat,
    no_color: bool,
}

impl OutputWriter {
    /// Create a new output writer
    pub fn new(format: OutputFormat, no_color: bool) -> Self {
        if no_color {
            colored::control::set_override(false);
        }
        Self { format, no_color }
    }

    /// Write a single item
    pub fn write<T: Serialize + TableDisplay>(&self, item: &T) -> Result<()> {
        match self.format {
            OutputFormat::Table => {
                item.display_single();
            }
            OutputFormat::Json => {
                let json = serde_json::to_string_pretty(item)?;
                println!("{}", json);
            }
            OutputFormat::Yaml => {
                let yaml = serde_yaml::to_string(item)?;
                print!("{}", yaml);
            }
            OutputFormat::Compact => {
                item.display_compact();
            }
        }
        Ok(())
    }

    /// Write a list of items
    pub fn write_list<T: Serialize + TableDisplay>(&self, items: &[T], headers: &[&str]) -> Result<()> {
        match self.format {
            OutputFormat::Table => {
                if items.is_empty() {
                    println!("{}", "No items found.".dimmed());
                    return Ok(());
                }

                let mut table = Table::new();
                table.load_preset(UTF8_FULL);
                table.apply_modifier(UTF8_ROUND_CORNERS);

                // Add headers with color
                let header_cells: Vec<Cell> = headers
                    .iter()
                    .map(|h| Cell::new(h).fg(Color::Cyan))
                    .collect();
                table.set_header(header_cells);

                // Add rows
                for item in items {
                    table.add_row(item.to_row());
                }

                println!("{table}");
                println!(
                    "\n{} {} item(s)",
                    "Total:".bold(),
                    items.len().to_string().green()
                );
            }
            OutputFormat::Json => {
                let json = serde_json::to_string_pretty(items)?;
                println!("{}", json);
            }
            OutputFormat::Yaml => {
                let yaml = serde_yaml::to_string(items)?;
                print!("{}", yaml);
            }
            OutputFormat::Compact => {
                for item in items {
                    item.display_compact();
                }
            }
        }
        Ok(())
    }

    /// Write a success message
    pub fn success(&self, message: &str) {
        if self.format == OutputFormat::Table {
            println!("{} {}", "✓".green(), message);
        } else {
            println!("{}", message);
        }
    }

    /// Write an error message
    pub fn error(&self, message: &str) {
        if self.format == OutputFormat::Table {
            eprintln!("{} {}", "✗".red(), message);
        } else {
            eprintln!("Error: {}", message);
        }
    }

    /// Write a warning message
    pub fn warning(&self, message: &str) {
        if self.format == OutputFormat::Table {
            println!("{} {}", "⚠".yellow(), message);
        } else {
            println!("Warning: {}", message);
        }
    }

    /// Write an info message
    pub fn info(&self, message: &str) {
        if self.format == OutputFormat::Table {
            println!("{} {}", "ℹ".blue(), message);
        } else {
            println!("{}", message);
        }
    }

    /// Start a spinner for long operations
    pub fn spinner(&self, message: &str) -> Option<indicatif::ProgressBar> {
        if self.format == OutputFormat::Table {
            let pb = indicatif::ProgressBar::new_spinner();
            pb.set_style(
                indicatif::ProgressStyle::default_spinner()
                    .template("{spinner:.green} {msg}")
                    .unwrap(),
            );
            pb.set_message(message.to_string());
            pb.enable_steady_tick(std::time::Duration::from_millis(100));
            Some(pb)
        } else {
            println!("{}", message);
            None
        }
    }

    /// Create a progress bar
    pub fn progress_bar(&self, total: u64, message: &str) -> Option<indicatif::ProgressBar> {
        if self.format == OutputFormat::Table {
            let pb = indicatif::ProgressBar::new(total);
            pb.set_style(
                indicatif::ProgressStyle::default_bar()
                    .template("{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} {msg}")
                    .unwrap()
                    .progress_chars("█▉▊▋▌▍▎▏ "),
            );
            pb.set_message(message.to_string());
            Some(pb)
        } else {
            println!("{} (total: {})", message, total);
            None
        }
    }
}

/// Trait for displaying items in a table
pub trait TableDisplay {
    /// Convert item to a table row
    fn to_row(&self) -> Vec<Cell>;

    /// Display a single item in detail
    fn display_single(&self);

    /// Display in compact format
    fn display_compact(&self);
}

/// Print a key-value pair in detail format
pub fn print_field(key: &str, value: &str) {
    println!("  {}: {}", key.cyan(), value);
}

/// Print an optional key-value pair
pub fn print_optional_field(key: &str, value: Option<&str>) {
    if let Some(v) = value {
        print_field(key, v);
    }
}

/// Print a list field
pub fn print_list_field(key: &str, values: &[String]) {
    if values.is_empty() {
        println!("  {}: {}", key.cyan(), "-".dimmed());
    } else {
        println!("  {}:", key.cyan());
        for v in values {
            println!("    - {}", v);
        }
    }
}

/// Print a section header
pub fn print_section(title: &str) {
    println!("\n{}", title.bold().underline());
}

/// Format a UUID for display (shortened)
pub fn format_uuid_short(uuid: &uuid::Uuid) -> String {
    let s = uuid.to_string();
    format!("{}...", &s[..8])
}

/// Format a timestamp for display
pub fn format_timestamp(dt: &chrono::DateTime<chrono::Utc>) -> String {
    dt.format("%Y-%m-%d %H:%M:%S UTC").to_string()
}

/// Format a relative time
pub fn format_relative_time(dt: &chrono::DateTime<chrono::Utc>) -> String {
    let now = chrono::Utc::now();
    let diff = now.signed_duration_since(*dt);

    if diff.num_seconds() < 60 {
        "just now".to_string()
    } else if diff.num_minutes() < 60 {
        format!("{} minute(s) ago", diff.num_minutes())
    } else if diff.num_hours() < 24 {
        format!("{} hour(s) ago", diff.num_hours())
    } else if diff.num_days() < 30 {
        format!("{} day(s) ago", diff.num_days())
    } else {
        format_timestamp(dt)
    }
}

/// Format bytes to human readable
pub fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

/// Status badge with color
pub fn status_badge(status: &str) -> String {
    match status.to_lowercase().as_str() {
        "running" | "active" | "in_progress" => status.to_string().blue().to_string(),
        "completed" | "success" | "passed" => status.to_string().green().to_string(),
        "failed" | "error" => status.to_string().red().to_string(),
        "pending" | "draft" => status.to_string().yellow().to_string(),
        "cancelled" | "archived" => status.to_string().dimmed().to_string(),
        _ => status.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(500), "500 B");
        assert_eq!(format_bytes(1024), "1.00 KB");
        assert_eq!(format_bytes(1536), "1.50 KB");
        assert_eq!(format_bytes(1048576), "1.00 MB");
        assert_eq!(format_bytes(1073741824), "1.00 GB");
    }

    #[test]
    fn test_format_uuid_short() {
        let uuid = uuid::Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
        assert_eq!(format_uuid_short(&uuid), "550e8400...");
    }

    #[test]
    fn test_output_format_display() {
        assert_eq!(OutputFormat::Table.to_string(), "table");
        assert_eq!(OutputFormat::Json.to_string(), "json");
        assert_eq!(OutputFormat::Yaml.to_string(), "yaml");
    }
}
