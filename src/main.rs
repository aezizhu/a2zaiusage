//! a2zusage - Query usage statistics from all your AI coding tools in one command

mod providers;
mod types;
mod utils;

use clap::{Parser, Subcommand};
use colored::Colorize;
use providers::get_all_providers;
use std::path::Path;
use types::{OutputFormat, ProviderResult};
use utils::format::{format_csv, format_json, format_table, print_banner, print_doctor_results};

#[derive(Parser)]
#[command(name = "a2zusage")]
#[command(author, version, about = "Query usage statistics from all your AI coding tools in one command")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Filter to specific tool (e.g., claude-code, cursor, copilot)
    #[arg(short, long)]
    tool: Option<String>,

    /// Output format
    #[arg(short, long, value_enum, default_value = "table")]
    format: OutputFormat,

    /// Show verbose output
    #[arg(short, long)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Check provider paths and configuration
    Doctor,
    /// List all supported tools
    List,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Doctor) => run_doctor().await,
        Some(Commands::List) => run_list(),
        None => run_usage_query(&cli).await,
    }
}

async fn run_usage_query(cli: &Cli) -> anyhow::Result<()> {
    // Only show banner for table format
    if matches!(cli.format, OutputFormat::Table) {
        print_banner();
    }

    let providers = get_all_providers();

    // Filter by tool if specified
    let providers: Vec<_> = if let Some(ref tool_filter) = cli.tool {
        providers
            .into_iter()
            .filter(|p| {
                p.name().contains(tool_filter) || p.display_name().to_lowercase().contains(&tool_filter.to_lowercase())
            })
            .collect()
    } else {
        providers
    };

    if providers.is_empty() {
        println!("{}", "No matching providers found.".yellow());
        return Ok(());
    }

    // Query all providers in parallel
    let show_progress = matches!(cli.format, OutputFormat::Table);
    if show_progress {
        println!("{}", "Scanning AI tools...".dimmed());
    }

    let results: Vec<ProviderResult> = futures::future::join_all(
        providers.iter().map(|p| async {
            if cli.verbose && show_progress {
                println!("  Checking {}...", p.display_name());
            }
            match p.get_usage(None).await {
                Ok(result) => result,
                Err(e) => ProviderResult::error(p.name(), p.display_name(), &e.to_string()),
            }
        })
    ).await;

    // Clear the "Scanning" line (only for table format)
    if show_progress {
        print!("\x1B[1A\x1B[2K");
    }

    // Format and display output
    let output = match cli.format {
        OutputFormat::Table => format_table(&results),
        OutputFormat::Json => format_json(&results),
        OutputFormat::Csv => format_csv(&results),
    };

    println!("{}", output);

    // Show verbose info if requested
    if cli.verbose {
        println!("\n{}", "Data Sources:".bold());
        for result in &results {
            if let Some(ref source) = result.data_source {
                println!("  {} {}: {}",
                    if result.status == types::ProviderStatus::Active { "✓".green() } else { "○".dimmed() },
                    result.display_name,
                    source.dimmed()
                );
            }
        }
    }

    Ok(())
}

async fn run_doctor() -> anyhow::Result<()> {
    print_banner();
    println!("{}\n", "Running diagnostics...".cyan());

    let providers = get_all_providers();
    let mut checks: Vec<(String, String, bool)> = Vec::new();

    for provider in &providers {
        let paths = provider.get_paths_to_check();
        for path in paths {
            if path.is_empty() {
                continue;
            }

            let exists = if path.starts_with("http") || path.contains("environment variable") {
                // For URLs and env vars, check differently
                if path.contains("environment variable") {
                    false // Can't easily check
                } else {
                    true // URLs are always "found"
                }
            } else {
                Path::new(&path).exists()
            };

            checks.push((provider.display_name().to_string(), path, exists));
        }
    }

    print_doctor_results(&checks);

    // Summary
    let found_count = checks.iter().filter(|(_, _, found)| *found).count();
    let total_count = checks.len();

    println!(
        "\n{}: {} of {} paths found\n",
        "Summary".bold(),
        found_count.to_string().green(),
        total_count
    );

    // Environment variable hints
    println!("{}", "Environment Variables for API Providers:".bold());
    println!("  {} - GitHub Copilot", "GITHUB_TOKEN".cyan());
    println!("  {} - OpenAI Codex", "OPENAI_API_KEY".cyan());
    println!("  {} - AWS credentials for Amazon Q", "AWS_PROFILE".cyan());
    println!();

    Ok(())
}

fn run_list() -> anyhow::Result<()> {
    print_banner();
    println!("{}\n", "Supported AI Coding Tools:".bold());

    let tools = [
        ("claude-code", "Claude Code", "CLI + IDE extension (shared JSONL)"),
        ("cursor", "Cursor", "SQLite database"),
        ("github-copilot", "GitHub Copilot", "API + Local logs"),
        ("cline", "Cline", "VS Code extension storage"),
        ("windsurf", "Windsurf", "Cascade logs"),
        ("warp", "Warp AI", "SQLite database"),
        ("opencode", "OpenCode", "Local JSON files"),
        ("openai-codex", "OpenAI Codex", "OpenAI API"),
        ("gemini-cli", "Gemini CLI", "Local telemetry"),
        ("amazon-q", "Amazon Q", "Local logs"),
        ("tabnine", "Tabnine", "Local logs"),
        ("sourcegraph-cody", "Sourcegraph Cody", "VS Code extension"),
        ("replit", "Replit Ghostwriter", "Web only (link)"),
    ];

    for (id, name, source) in tools {
        println!("  {} {} ({})", "•".cyan(), name.bold(), source.dimmed());
        println!("    ID: {}", id.dimmed());
    }

    println!("\n{}", "Usage:".bold());
    println!("  a2zusage              # Query all tools");
    println!("  a2zusage -t cursor    # Query specific tool");
    println!("  a2zusage -f json      # Output as JSON");
    println!("  a2zusage doctor       # Check configuration");
    println!();

    Ok(())
}
