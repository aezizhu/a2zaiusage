//! Output formatting utilities

use crate::types::{ProviderResult, ProviderStatus, UsageData};
use colored::Colorize;
use tabled::{
    settings::{object::Columns, Alignment, Modify, Style},
    Table, Tabled,
};

/// Format a number with K/M suffix for readability
pub fn format_number(num: u64) -> String {
    if num == 0 {
        return "-".to_string();
    }

    if num >= 1_000_000 {
        format!("{:.1}M", num as f64 / 1_000_000.0)
    } else if num >= 1_000 {
        format!("{:.1}K", num as f64 / 1_000.0)
    } else {
        num.to_string()
    }
}

/// Format token count with label
pub fn format_tokens(data: &UsageData) -> String {
    let total = data.total_tokens();
    if total == 0 && data.request_count == 0 {
        return "-".to_string();
    }

    if data.request_count > 0 && total == 0 {
        return format!("{} reqs", format_number(data.request_count));
    }

    format!("{} tokens", format_number(total))
}

/// Format cost as USD
#[allow(dead_code)]
pub fn format_cost(cost: f64) -> String {
    if cost == 0.0 {
        return "-".to_string();
    }

    if cost < 0.01 {
        return "<$0.01".to_string();
    }

    format!("${:.2}", cost)
}

/// Get status display string with color
pub fn format_status(status: ProviderStatus) -> String {
    match status {
        ProviderStatus::Active => "Active".green().to_string(),
        ProviderStatus::NotFound => "N/A".dimmed().to_string(),
        ProviderStatus::NoKey => "No Key".yellow().to_string(),
        ProviderStatus::AuthRequired => "Auth".yellow().to_string(),
        ProviderStatus::Error => "Error".red().to_string(),
        ProviderStatus::LinkOnly => "Link".blue().to_string(),
    }
}

/// Get status icon
pub fn status_icon(status: ProviderStatus) -> &'static str {
    match status {
        ProviderStatus::Active => "✓",
        ProviderStatus::NotFound => "○",
        ProviderStatus::NoKey => "✗",
        ProviderStatus::AuthRequired => "⚠",
        ProviderStatus::Error => "✗",
        ProviderStatus::LinkOnly => "→",
    }
}

/// Table row for display
#[derive(Tabled)]
pub struct TableRow {
    #[tabled(rename = "Tool")]
    pub tool: String,
    #[tabled(rename = "Status")]
    pub status: String,
    #[tabled(rename = "Today")]
    pub today: String,
    #[tabled(rename = "This Week")]
    pub this_week: String,
    #[tabled(rename = "This Month")]
    pub this_month: String,
    #[tabled(rename = "Total")]
    pub total: String,
}

/// Format results as a table
pub fn format_table(results: &[ProviderResult]) -> String {
    let mut rows: Vec<TableRow> = Vec::new();

    for result in results {
        let (today, week, month, total) = if let Some(ref usage) = result.usage {
            (
                format_tokens(&usage.today),
                format_tokens(&usage.this_week),
                format_tokens(&usage.this_month),
                format_tokens(&usage.total),
            )
        } else {
            ("-".to_string(), "-".to_string(), "-".to_string(), "-".to_string())
        };

        rows.push(TableRow {
            tool: result.display_name.clone(),
            status: format!("{} {}", status_icon(result.status), format_status(result.status)),
            today,
            this_week: week,
            this_month: month,
            total,
        });
    }

    let table = Table::new(rows)
        .with(Style::rounded())
        .with(Modify::new(Columns::single(0)).with(Alignment::left()))
        .to_string();

    table
}

/// Format results as JSON
pub fn format_json(results: &[ProviderResult]) -> String {
    serde_json::to_string_pretty(results).unwrap_or_else(|_| "[]".to_string())
}

/// Format results as CSV
pub fn format_csv(results: &[ProviderResult]) -> String {
    let mut output = String::from(
        "Tool,Status,Today Input,Today Output,Month Input,Month Output,Total Input,Total Output,Est Cost\n",
    );

    for result in results {
        let (ti, to, mi, mo, toi, too, cost) = if let Some(ref usage) = result.usage {
            (
                usage.today.input_tokens,
                usage.today.output_tokens,
                usage.this_month.input_tokens,
                usage.this_month.output_tokens,
                usage.total.input_tokens,
                usage.total.output_tokens,
                usage.total.estimated_cost,
            )
        } else {
            (0, 0, 0, 0, 0, 0, 0.0)
        };

        output.push_str(&format!(
            "{},{},{},{},{},{},{},{},{:.2}\n",
            result.display_name,
            result.status,
            ti,
            to,
            mi,
            mo,
            toi,
            too,
            cost
        ));
    }

    output
}

/// Print banner
pub fn print_banner() {
    println!();
    println!("{}", "  ccusage - AI Coding Tools Usage Tracker".cyan().bold());
    println!();
}

/// Print doctor results
pub fn print_doctor_results(checks: &[(String, String, bool)]) {
    println!("{}", "\nProvider Path Detection:\n".bold());

    for (name, path, found) in checks {
        let icon = if *found {
            "✓".green()
        } else {
            "✗".red()
        };
        let path_display = if *found {
            path.green()
        } else {
            path.dimmed()
        };
        println!("  {} {}", icon, name);
        println!("    {}\n", path_display);
    }
}
